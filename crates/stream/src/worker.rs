use super::client::BoardConfig;
use rchan_api::{client::Client, error::Error};
use rchan_types::{
    board::Board,
    post::{Post, ThreadPage},
};
use std::{collections::HashMap, sync::Arc};
use tracing::{debug, error, info};

#[derive(Debug, Clone)]
pub struct ThreadCache {
    no: i32,
    last_modified: i64,
    prev_last_modified: i64,
}

impl ThreadCache {
    pub fn new(no: i32, last_modified: i64) -> ThreadCache {
        ThreadCache {
            no,
            last_modified,
            prev_last_modified: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BoardCache {
    last_update_sec: i64,
    threads: HashMap<i32, ThreadCache>,
}

impl BoardCache {
    pub fn new(thread_limit: usize) -> BoardCache {
        BoardCache {
            last_update_sec: 0,
            threads: HashMap::with_capacity(thread_limit),
        }
    }
}

pub struct BoardWorker {
    api: Arc<Client>,
    cfg: BoardConfig,
    board: Board,
    cache: BoardCache,
    new_posts_chan: tokio::sync::mpsc::Sender<Post>,
    kill: Option<tokio::sync::oneshot::Receiver<()>>,
}

impl BoardWorker {
    pub fn new(
        api: Arc<Client>,
        cfg: BoardConfig,
        board: Board,
        new_posts_tx: tokio::sync::mpsc::Sender<Post>,
        kill: Option<tokio::sync::oneshot::Receiver<()>>,
    ) -> BoardWorker {
        let cache = BoardCache::new(board.thread_limit() as usize);
        BoardWorker {
            api,
            cfg,
            board,
            cache,
            new_posts_chan: new_posts_tx,
            kill,
        }
    }

    pub async fn new_and_run(
        api: Arc<Client>,
        cfg: BoardConfig,
        board: Board,
        new_posts_tx: tokio::sync::mpsc::Sender<Post>,
        kill: Option<tokio::sync::oneshot::Receiver<()>>,
    ) -> Result<(), Error> {
        let mut worker = BoardWorker::new(api, cfg, board, new_posts_tx, kill);
        worker.run().await
    }

    pub async fn init(&mut self) -> Result<(), Error> {
        self.api.get_threads(self.board.name()).await.map(|pages| {
            self.update_cache(&pages);
        })
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        self.init().await?;
        loop {
            if let Some(ref mut rx) = self.kill {
                match rx.try_recv() {
                    Ok(_) => {
                        info!(
                            "Received kill signal, stopping worker: {}",
                            self.board.name()
                        );
                        return Ok(());
                    }
                    Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                        error!(
                            "Kill channel closed, stopping worker: {}",
                            self.board.name()
                        );
                        return Ok(());
                    }
                    _ => {}
                }
            }
            self.update_board()
                .await
                .map_err(|e| error!("{:?}", e))
                .unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(
                self.cfg.refresh_rate_ms as u64,
            ))
            .await;
        }
    }

    /// A full board update cycle
    /// 1. Fetch all threads and update the local cache, returning new and modified threads
    /// 2. Fetch each modified thread, entirely, in parallel
    /// 3. Filter new posts and send them to the main thread
    /// 4. Send new cache object to the main thread for storage
    async fn update_board(&mut self) -> Result<(), Error> {
        debug!("Performing full board update: {}", self.board.name());
        let now = chrono::Utc::now().timestamp();
        let last_update_sec = self.cache.last_update_sec;
        let mut rxs = vec![];
        for modified_thread in self
            .api
            .get_threads(self.board.name())
            .await
            .map(|pages| self.update_cache(&pages))?
        {
            let api = self.api.clone();
            let board = self.board.name().to_string();
            let new_posts_chan = self.new_posts_chan.clone();
            let cache = self
                .cache
                .threads
                .get(&modified_thread.no)
                .unwrap_or(&ThreadCache::new(modified_thread.no, last_update_sec))
                .clone();
            let (tx, rx) = tokio::sync::oneshot::channel();
            rxs.push(rx);
            tokio::spawn(async move {
                match api.get_thread(&board, cache.no).await {
                    Ok(thread) => {
                        for new_post in thread
                            .posts
                            .iter()
                            .filter(|post| {
                                post.time.map_or(false, |t| t > cache.prev_last_modified)
                            })
                            .collect::<Vec<_>>()
                        {
                            if let Err(e) = new_posts_chan.send(new_post.clone()).await {
                                error!("Error sending new post: {:?}", e);
                            }
                        }
                        if let Err(e) = tx.send(None) {
                            error!("Error sending thread no: {:?}", e);
                        }
                    }
                    Err(e) => {
                        error!("Error fetching thread: {:?}", e);
                        if let Err(e) = tx.send(Some(cache.no)) {
                            error!("Error sending thread no: {:?}", e);
                        }
                    }
                }
            });
        }
        debug!(
            "Waiting for tasks to complete, board: {}",
            self.board.name()
        );
        futures::future::join_all(rxs)
            .await
            .into_iter()
            .for_each(|res| match res {
                Ok(Some(thread_no)) if thread_no >= 0 => {
                    if let Some(entry) = self.cache.threads.get_mut(&thread_no) {
                        info!("Reverting thread: {}, trying again later", thread_no);
                        entry.last_modified = entry.prev_last_modified;
                    }
                }
                _ => {}
            });
        self.cache.last_update_sec = now;
        Ok(())
    }

    /// Update the local cache with new and modified threads
    /// 1. Remove deleted threads
    /// 2. Add new threads
    /// 3. Update modified threads
    /// 4. Sort modified threads by last_modified
    /// 5. Return modified threads
    fn update_cache(&mut self, pages: &[ThreadPage]) -> Vec<Post> {
        let n_threads: usize = pages.iter().map(|page| page.threads.len()).sum();
        debug!(
            "Updating cache for board: {}, threads: {}",
            self.board.name(),
            n_threads,
        );
        self.cache.threads.retain(|k, _| {
            pages
                .iter()
                .any(|page| page.threads.iter().any(|t| t.no == *k))
        });
        let mut modified_threads = vec![];
        for page in pages {
            for thread in &page.threads {
                let cache = self
                    .cache
                    .threads
                    .entry(thread.no)
                    .or_insert(ThreadCache::new(thread.no, 0));
                let thread_last_modified = thread.last_modified.unwrap_or(0);
                if cache.last_modified < thread_last_modified {
                    modified_threads.push(thread.clone());
                    cache.prev_last_modified = cache.last_modified;
                    cache.last_modified = thread_last_modified;
                }
            }
        }
        modified_threads.sort_by(|a, b| a.last_modified.cmp(&b.last_modified));
        modified_threads
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rchan_api::client::Client;
    use rchan_types::board::Cooldowns;
    use std::sync::Arc;

    #[tracing_test::traced_test]
    #[tokio::test]
    async fn test_run() {
        let client = Arc::new(Client::new());
        let cfg = BoardConfig {
            name: "pol".to_string(),
            refresh_rate_ms: 10000,
        };
        let board = Board {
            board: "g".to_string(),
            title: "Technology".to_string(),
            ws_board: 1,
            per_page: 15,
            pages: 10,
            max_filesize: 4194304,
            max_webm_filesize: 3145728,
            max_comment_chars: 2000,
            max_webm_duration: 120,
            bump_limit: 500,
            image_limit: 250,
            cooldowns: Cooldowns {
                threads: 600,
                replies: 60,
                images: 60,
            },
            meta_description: "Technology".to_string(),
            spoilers: None,
            custom_spoilers: None,
            is_archived: None,
            forced_anon: None,
            board_flags: None,
        };

        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        tokio::spawn(async move {
            if let Err(e) = BoardWorker::new_and_run(client, cfg, board, tx, None).await {
                error!("Error in worker: {:?}", e);
            }
        });
        let n_posts_to_receive = 10;
        let mut n_posts_received = 0;
        while let Some(post) = rx.recv().await {
            info!("Received post: {:?}", post);
            n_posts_received += 1;
            if n_posts_received >= n_posts_to_receive {
                break;
            }
        }
    }

    #[tracing_test::traced_test]
    #[tokio::test]
    async fn test_kill() {
        let client = Arc::new(Client::new());
        let cfg = BoardConfig {
            name: "pol".to_string(),
            refresh_rate_ms: 10000,
        };
        let board = Board {
            board: "g".to_string(),
            title: "Technology".to_string(),
            ws_board: 1,
            per_page: 15,
            pages: 10,
            max_filesize: 4194304,
            max_webm_filesize: 3145728,
            max_comment_chars: 2000,
            max_webm_duration: 120,
            bump_limit: 500,
            image_limit: 250,
            cooldowns: Cooldowns {
                threads: 600,
                replies: 60,
                images: 60,
            },
            meta_description: "Technology".to_string(),
            spoilers: None,
            custom_spoilers: None,
            is_archived: None,
            forced_anon: None,
            board_flags: None,
        };

        let (tx, _rx) = tokio::sync::mpsc::channel(100);
        let (kill_tx, kill_rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            if let Err(e) = BoardWorker::new_and_run(client, cfg, board, tx, Some(kill_rx)).await {
                error!("Error in worker: {:?}", e);
            }
        });
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        kill_tx.send(()).unwrap();
    }
}

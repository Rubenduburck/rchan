use super::stream::BoardConfig;
use crate::{
    api::{client::Client, endpoint::Endpoint, error::Error, response::ClientResponse},
    types::post::{Post, Thread},
};
use std::sync::Arc;
use tracing::info;

pub struct BoardCache {
    last_update_sec: i64,
}

impl BoardCache {
    pub fn new() -> BoardCache {
        BoardCache { last_update_sec: 0 }
    }

    pub fn new_at(time: i64) -> BoardCache {
        BoardCache {
            last_update_sec: time,
        }
    }
}

impl Default for BoardCache {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BoardWorker {
    http: Arc<Client>,
    board: BoardConfig,
    cache: BoardCache,
    new_posts_chan: tokio::sync::mpsc::Sender<Post>,
}

impl BoardWorker {
    pub fn new(
        http: Arc<Client>,
        board: BoardConfig,
        new_posts: tokio::sync::mpsc::Sender<Post>,
    ) -> BoardWorker {
        BoardWorker {
            http,
            board,
            cache: BoardCache::new_at(chrono::Utc::now().timestamp()),
            new_posts_chan: new_posts,
        }
    }

    pub async fn new_and_run(
        http: Arc<Client>,
        board: BoardConfig,
        new_posts_chan: tokio::sync::mpsc::Sender<Post>,
    ) {
        let mut worker = BoardWorker::new(http, board, new_posts_chan);
        worker.run().await;
    }

    pub async fn run(&mut self) {
        loop {
            self.update_board()
                .await
                .map_err(|e| println!("{:?}", e))
                .unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(
                self.board.refresh_rate_ms as u64,
            ))
            .await;
        }
    }

    /// A full board update cycle
    /// 1. Get thread numbers for threads that have been updated since the last fetch
    /// 2. Fetch each thread
    async fn update_board(&mut self) -> Result<(), Error> {
        println!("Fetching board {}", self.board.name);
        let now = chrono::Utc::now().timestamp();
        let last_update_sec = self.cache.last_update_sec;
        let modified_threads = self.fetch_modified_threads().await?;
        println!("Modified threads: {:?}", modified_threads.len());
        for thread_no in modified_threads {
            let http = self.http.clone();
            let board = self.board.name.clone();
            let new_posts_chan = self.new_posts_chan.clone();
            tokio::spawn(async move {
                match Self::fetch_thread(http, board, thread_no).await {
                    Ok(thread) => {
                        let new_posts = thread
                            .posts
                            .iter()
                            .filter(|post| post.time.map_or(false, |t| t > last_update_sec));
                        for post in new_posts {
                            match new_posts_chan.send(post.clone()).await {
                                Ok(_) => {}
                                Err(e) => {
                                    println!("Error sending new post: {:?}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("Error fetching thread: {:?}", e);
                    }
                }
            });
        }
        self.cache.last_update_sec = now;
        Ok(())
    }

    /// Fetches all threads from the board
    /// Returns a list of thread numbers for threads that have been updated since the last fetch
    async fn fetch_modified_threads(&self) -> Result<Vec<i32>, Error> {
        println!("Fetching modified threads");
        let endpoint = Endpoint::Threads(self.board.name.clone());
        match *(self.http.get(&endpoint, false).await?) {
            ClientResponse::Threads(ref pages) => Ok(pages
                .iter()
                .flat_map(|page| {
                    page.threads.iter().filter_map(|thread| {
                        thread.last_modified.and_then(|lm| {
                            if lm > self.cache.last_update_sec {
                                Some(thread.no)
                            } else {
                                None
                            }
                        })
                    })
                })
                .collect::<Vec<_>>()),
            _ => Err(Error::Generic("Invalid response".to_string())),
        }
    }

    async fn fetch_thread(
        http: Arc<Client>,
        board: String,
        thread_no: i32,
    ) -> Result<Thread, Error> {
        println!("Fetching thread {}", thread_no);
        let endpoint = Endpoint::Thread(board, thread_no);
        match *(http.get(&endpoint, false).await?) {
            ClientResponse::Thread(ref thread) => Ok(thread.clone()),
            _ => Err(Error::Generic("Invalid response".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::client::Client;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_run() {
        let client = Arc::new(Client::new());
        let board = BoardConfig {
            name: "pol".to_string(),
            refresh_rate_ms: 10000,
        };
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        tokio::spawn(async move {
            BoardWorker::new_and_run(client, board, tx).await;
        });
        while let Some(post) = rx.recv().await {
            println!("Received post: {:?}", post);
        }
    }
}

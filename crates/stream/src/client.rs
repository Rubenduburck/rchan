use super::error::Error;
use rchan_api::client::Client;
use rchan_types::board::Board;
use std::{collections::HashMap, sync::Arc};
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct Subscription {
    pub board_name: String,
    pub refresh_rate_ms: i64,
}

impl Subscription {
    const DEFAULT_REFRESH_RATE_MS: i64 = 10000;
    pub fn new(name: String, refresh_rate_ms: Option<i64>) -> Subscription {
        Subscription {
            board_name: name,
            refresh_rate_ms: refresh_rate_ms.unwrap_or(Self::DEFAULT_REFRESH_RATE_MS),
        }
    }
}

pub struct Stream {
    boards: Arc<Vec<Board>>,
    api: Arc<Client>,
    events_tx: tokio::sync::mpsc::Sender<crate::worker::Event>,

    workers: HashMap<String, tokio::sync::oneshot::Sender<()>>,
}

impl Stream {
    pub fn new(
        client: Option<Arc<Client>>,
        events_tx: tokio::sync::mpsc::Sender<crate::worker::Event>,
    ) -> Self {
        Stream {
            api: client.unwrap_or_else(|| Arc::new(Client::default())),
            boards: Arc::new(Vec::new()),
            events_tx,
            workers: HashMap::new(),
        }
    }

    pub async fn subscribe(&mut self, sub: Subscription) -> Result<(), Error> {
        if self.workers.contains_key(&sub.board_name) {
            return Err(Error::AlreadySubscribed(
                "Board already subscribed".to_string(),
            ));
        }
        let (events_tx, mut events_rx) = tokio::sync::mpsc::channel(100);
        let board_name = sub.board_name.clone();
        let board_data = self.get_board_data(&board_name).await?;
        let kill_worker = Self::start_worker(self.api.clone(), sub, board_data, events_tx);
        let events_tx = self.events_tx.clone();
        self.workers.insert(board_name, kill_worker);
        tokio::spawn(async move {
            while let Some(new_event) = events_rx.recv().await {
                events_tx.send(new_event).await.unwrap();
            }
        });
        Ok(())
    }

    pub fn unsubscribe(&mut self, board: &str) {
        self.kill_worker(board);
    }

    async fn get_board_data(&mut self, board: &str) -> Result<Board, Error> {
        if self.boards.is_empty() {
            self.boards = self.api.get_boards().await?;
        }
        if let Some(board) = self.boards.iter().find(|b| b.board == board) {
            Ok(board.clone())
        } else {
            Err(Error::BoardNotFound("Board not found".to_string()))
        }
    }

    pub fn kill_worker(&mut self, board: &str) {
        if let Some(worker) = self.workers.remove(board) {
            let _ = worker.send(());
        }
    }

    fn start_worker(
        api: Arc<Client>,
        cfg: Subscription,
        board: Board,
        new_posts_tx: tokio::sync::mpsc::Sender<crate::worker::Event>,
    ) -> tokio::sync::oneshot::Sender<()> {
        info!("Starting worker for board {}", board.board.clone());
        let (kill_tx, kill_rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            if let Err(e) = crate::worker::BoardWorker::new_and_run(
                api.clone(),
                cfg,
                board,
                new_posts_tx,
                Some(kill_rx),
            )
            .await
            {
                error!("Error in worker: {:?}", e);
            }
        });
        kill_tx
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use tracing::debug;

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_subscribe() {
        let (events_tx, mut events_rx) = tokio::sync::mpsc::channel(100);

        let mut stream = Stream::new(None, events_tx);
        stream
            .subscribe(Subscription::new("g".to_string(), None))
            .await
            .unwrap();
        stream
            .subscribe(Subscription::new("v".to_string(), None))
            .await
            .unwrap();

        let mut counts = HashMap::new();
        let counts_needed = 5;
        while let Some(event) = events_rx.recv().await {
            match event {
                crate::worker::Event::NewPost(event) => {
                    debug!(
                        "New post on {}:\n{}",
                        event.board,
                        event.post.clean_comment().unwrap_or_default()
                    );
                    counts
                        .entry(event.board)
                        .and_modify(|e| *e += 1)
                        .or_insert(1);
                }
                crate::worker::Event::NewThread(event) => {
                    debug!(
                        "New thread on {}:\n{}",
                        event.board,
                        event.post.clean_comment().unwrap_or_default()
                    );
                    counts
                        .entry(event.board)
                        .and_modify(|e| *e += 1)
                        .or_insert(1);
                }
            }
            if counts.values().all(|v| *v >= counts_needed) {
                break;
            }
        }
    }
}

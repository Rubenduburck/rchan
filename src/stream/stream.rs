use tracing::info;

use crate::{api::client::Client, stream::worker, types::post::Post};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct BoardConfig {
    pub name: String,
    pub refresh_rate_ms: i64,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub boards: Vec<BoardConfig>,
}

pub struct Stream {
    cfg: Config,
    http: Arc<Client>,
}

impl Stream {
    pub fn new(http: Arc<Client>, cfg: Config) -> Stream {
        Stream { http, cfg }
    }

    pub async fn run(&self) {
        let (new_posts_tx, mut new_posts_rx) = tokio::sync::mpsc::channel(100);
        for board in self.cfg.boards.clone() {
            Self::start_worker(self.http.clone(), board, new_posts_tx.clone());
        }
        while let Some(post) = new_posts_rx.recv().await {
            info!("Received new post: {:?}", post);
        }
    }

    fn start_worker(http: Arc<Client>, board: BoardConfig, new_posts_tx: tokio::sync::mpsc::Sender<Post>) {
        info!("Starting worker for board {}", board.name);
        tokio::spawn(async move {
            worker::BoardWorker::new_and_run(http.clone(), board, new_posts_tx).await;
        });
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
        let boards = vec![
            BoardConfig {
                name: "test1".to_string(),
                refresh_rate_ms: 1000,
            },
            BoardConfig {
                name: "test2".to_string(),
                refresh_rate_ms: 1000,
            },
        ];
        let config = Config { boards };
        let stream = Stream::new(client, config.clone());
        stream.run().await;
    }
}

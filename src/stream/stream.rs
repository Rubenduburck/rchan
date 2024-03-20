use tracing::info;

use crate::{api::client::Client, stream::worker, types::post::Post};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct BoardConfig {
    pub name: String,
    pub refresh_rate_ms: i64,
}

impl BoardConfig {
    const DEFAULT_REFRESH_RATE_MS: i64 = 10000;
    pub fn new(name: String, refresh_rate_ms: Option<i64>) -> BoardConfig {
        BoardConfig {
            name,
            refresh_rate_ms: refresh_rate_ms.unwrap_or(Self::DEFAULT_REFRESH_RATE_MS),
        }
    }
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

    fn start_worker(
        http: Arc<Client>,
        board: BoardConfig,
        new_posts_tx: tokio::sync::mpsc::Sender<Post>,
    ) {
        info!("Starting worker for board {}", board.name);
        tokio::spawn(async move {
            worker::BoardWorker::new_and_run(http.clone(), board, new_posts_tx).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use tracing_test::traced_test;

    use super::*;
    use crate::api::client::Client;
    use std::sync::Arc;

    #[tokio::test]
    #[traced_test]
    async fn test_run() {
        let client = Arc::new(Client::new());
        let boards = vec![
            BoardConfig {
                name: "g".to_string(),
                refresh_rate_ms: 10000,
            },
            BoardConfig {
                name: "v".to_string(),
                refresh_rate_ms: 10000,
            },
        ];
        let config = Config { boards };
        let stream = Stream::new(client, config.clone());
        stream.run().await;
    }
}

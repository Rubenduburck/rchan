use crate::{
    api::{client::Client, endpoint::Endpoint, error::Error, response::ClientResponse},
    stream::worker,
    types::{board::Board, post::Post},
};
use std::sync::Arc;
use tracing::{info, error};

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
    boards: Vec<Board>,
    http: Arc<Client>,
}

impl Stream {
    pub fn new(http: Arc<Client>, cfg: Config) -> Stream {
        Stream {
            http,
            cfg,
            boards: vec![],
        }
    }

    fn get_config_for_board(&self, board: &Board) -> Option<BoardConfig> {
        self.cfg
            .boards
            .iter()
            .find(|c| c.name == board.board)
            .cloned()
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        self.init().await?;
        let (new_posts_tx, mut new_posts_rx) = tokio::sync::mpsc::channel(100);
        for board in self.boards.clone() {
            let cfg = self.get_config_for_board(&board).unwrap();
            Self::start_worker(self.http.clone(), cfg, board, new_posts_tx.clone());
        }
        self.handle_new_posts(&mut new_posts_rx).await
    }

    async fn init(&mut self) -> Result<(), Error> {
        self.boards = self
            .get_available_boards()
            .await?
            .into_iter()
            .filter(|b| self.cfg.boards.iter().any(|c| c.name == b.board))
            .collect();
        Ok(())
    }

    async fn handle_new_posts(
        &self,
        new_posts_rx: &mut tokio::sync::mpsc::Receiver<Post>,
    ) -> Result<(), Error> {
        while let Some(post) = new_posts_rx.recv().await {
            info!("New post: {:?}", post);
        }
        Ok(())
    }

    async fn get_available_boards(&self) -> Result<Vec<Board>, Error> {
        let endpoint = Endpoint::Boards;
        match *(self.http.get(&endpoint, false).await?) {
            ClientResponse::Boards(ref resp) => Ok(resp.boards.clone()),
            _ => Err(Error::Generic("Invalid response".to_string())),
        }
    }

    fn start_worker(
        http: Arc<Client>,
        cfg: BoardConfig,
        board: Board,
        new_posts_tx: tokio::sync::mpsc::Sender<Post>,
    ) {
        info!("Starting worker for board {}", board.board.clone());
        tokio::spawn(async move {
            if let Err(e) = worker::BoardWorker::new_and_run(http.clone(), cfg, board, new_posts_tx).await {
                error!("Error in worker: {:?}", e);
            }
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
        let mut stream = Stream::new(client, config.clone());
        let _ = stream.run().await;
    }
}

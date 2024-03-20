use rchan_api::{client::Client, endpoint::Endpoint, error::Error, response::ClientResponse};
use rchan_types::{board::Board, post::Post};
use std::sync::Arc;
use tracing::{error, info};

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

#[derive(Debug)]
pub enum Event {
    NewPost(Post),
    NewThread(Post),
}

impl From<Post> for Event {
    fn from(post: Post) -> Self {
        if post.is_op() {
            Event::NewThread(post)
        } else {
            Event::NewPost(post)
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
    events_tx: tokio::sync::mpsc::Sender<Event>,
}

impl Stream {
    pub fn new(
        http: Arc<Client>,
        cfg: Config,
        events_tx: tokio::sync::mpsc::Sender<Event>,
    ) -> Self {
        Stream {
            http,
            cfg,
            boards: vec![],
            events_tx,
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
        loop {
            tokio::select! {
                Some(post) = new_posts_rx.recv() => {
                    self.handle_new_posts(post).await?;
                }
            }
        }
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

    async fn handle_new_posts(&self, new_post: Post) -> Result<(), Error> {
        self.events_tx
            .send(new_post.into())
            .await
            .map_err(|e| Error::Generic(e.to_string()))
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
            if let Err(e) =
                crate::worker::BoardWorker::new_and_run(http.clone(), cfg, board, new_posts_tx)
                    .await
            {
                error!("Error in worker: {:?}", e);
            }
        });
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use rchan_api::client::Client;
    use std::sync::Arc;

    #[tokio::test]
    #[tracing_test::traced_test]
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
        let (events_tx, mut events_rx) = tokio::sync::mpsc::channel(100);
        let mut stream = Stream::new(client, config.clone(), events_tx);
        tokio::spawn(async move {
            let _ = stream.run().await;
        });
        while let Some(event) = events_rx.recv().await {
            info!("Received event: {:?}", event);
        }
    }
}

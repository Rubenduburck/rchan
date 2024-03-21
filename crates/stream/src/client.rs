use rchan_api::{client::Client, endpoint::Endpoint, response::ClientResponse};
use rchan_types::{board::Board, post::Post};
use std::{collections::HashMap, sync::Arc};
use tracing::{error, info};
use super::error::Error;

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

pub struct Stream {
    boards: Vec<Board>,
    http: Arc<Client>,
    events_tx: tokio::sync::mpsc::Sender<Event>,

    kill_switches: HashMap<String, tokio::sync::oneshot::Sender<()>>,
}

impl Stream {
    pub fn new(
        http: Arc<Client>,
        events_tx: tokio::sync::mpsc::Sender<Event>,
    ) -> Self {
        Stream {
            http,
            boards: vec![],
            events_tx,
            kill_switches: HashMap::new(),
        }
    }

    pub async fn subscribe(&mut self, cfg: BoardConfig) -> Result<(), Error> {
        if self.kill_switches.contains_key(&cfg.name) {
            return Err(Error::AlreadySubscribed("Board already subscribed".to_string()));
        }
        let (new_posts_tx, mut new_posts_rx) = tokio::sync::mpsc::channel(100);
        let board_name = cfg.name.clone();
        let board_data = self.get_board_data(&board_name).await?;
        let kill_worker = Self::start_worker(self.http.clone(), cfg, board_data, new_posts_tx);
        let events_tx = self.events_tx.clone();
        self.kill_switches.insert(board_name, kill_worker);
        tokio::spawn(async move {
            while let Some(new_post) = new_posts_rx.recv().await {
                let event = Event::from(new_post);
                if let Err(e) = events_tx.send(event).await {
                    error!("Error sending event: {:?}", e);
                }
            }
        });
        Ok(())
    }

    pub fn unsubscribe(&mut self, board: &str) {
        self.kill_worker(board);
    }

    async fn get_board_data(&mut self, board: &str) -> Result<Board, Error> {
        if self.boards.is_empty() {
            self.boards = self.get_available_boards().await?;
        }
        if let Some(board) = self.boards.iter().find(|b| b.board == board) {
            Ok(board.clone())
        } else {
            Err(Error::BoardNotFound("Board not found".to_string()))
        }
    }

    async fn get_available_boards(&self) -> Result<Vec<Board>, Error> {
        let endpoint = Endpoint::Boards;
        match *(self.http.get(&endpoint, false).await?) {
            ClientResponse::Boards(ref resp) => Ok(resp.boards.clone()),
            _ => Err(Error::InvalidResponse),
        }
    }

    pub fn kill_worker(&mut self, board: &str) {
        if let Some(kill_switch) = self.kill_switches.remove(board) {
            let _ = kill_switch.send(());
        }
    }

    fn start_worker(
        http: Arc<Client>,
        cfg: BoardConfig,
        board: Board,
        new_posts_tx: tokio::sync::mpsc::Sender<Post>,
    ) -> tokio::sync::oneshot::Sender<()> {
        info!("Starting worker for board {}", board.board.clone());
        let (kill_tx, kill_rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            if let Err(e) = crate::worker::BoardWorker::new_and_run(
                http.clone(),
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
    use rchan_api::client::Client;
    use std::sync::Arc;

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_subscribe() {
        let client = Arc::new(Client::new());
        let cfg = BoardConfig::new("g".to_string(), Some(10000));
        let (events_tx, mut events_rx) = tokio::sync::mpsc::channel(100);
        let mut stream = Stream::new(client, events_tx);
        stream.subscribe(cfg).await.unwrap();
        while let Some(event) = events_rx.recv().await {
            info!("Received event: {:?}", event);
        }
    }

}

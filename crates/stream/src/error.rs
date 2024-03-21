#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Rchan API error: {0}")]
    RchanApi(#[from] rchan_api::error::Error),

    #[error("Already subscribed to board: {0}")]
    AlreadySubscribed(String),

    #[error("Board not found: {0}")]
    BoardNotFound(String),

    #[error("Invalid Response")]
    InvalidResponse,
}

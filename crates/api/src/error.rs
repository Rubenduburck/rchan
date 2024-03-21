#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Max retries exceeded: {0}")]
    MaxRetriesExceeded(String),

    #[error("Status code: {0}")]
    StatusCode(String),

    #[error("Stream error: {0}")]
    Stream(String),

    #[error("Invalid response")]
    InvalidResponse,

    #[error("No cached response")]
    NoCachedResponse,

}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Generic error: {0}")]
    Generic(String),

}

use crate::types::{board::BoardsResponse, post::{ThreadPage, Thread}, catalog::CatalogPage, index::Index};

use super::endpoint::Endpoint;

#[derive(Debug)]
pub enum ClientResponse {
    Boards(BoardsResponse),
    Threads(Vec<ThreadPage>),
    Catalog(Vec<CatalogPage>),
    Archive(Vec<i32>),
    Index(Index),
    Thread(Thread),
    NotModified,
}

impl ClientResponse {
    pub async fn parse(endpoint: &Endpoint, resp: reqwest::Response) -> Result<Self, reqwest::Error> {
        match endpoint {
            Endpoint::Boards => Ok(Self::Boards(resp.json().await?)),
            Endpoint::Threads(_) => Ok(Self::Threads(resp.json().await?)),
            Endpoint::Catalog(_) => Ok(Self::Catalog(resp.json().await?)),
            Endpoint::Archive(_) => Ok(Self::Archive(resp.json().await?)),
            Endpoint::Index(_, _) => Ok(Self::Index(resp.json().await?)),
            Endpoint::Thread(_, _) => Ok(Self::Thread(resp.json().await?)),
        }
    }
}

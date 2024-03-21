use std::sync::Arc;

use super::endpoint::Endpoint;
use rchan_types::{
    board::{Board, BoardsResponse},
    catalog::CatalogPage,
    index::Index,
    post::{Thread, ThreadPage},
};

#[derive(Debug, Clone)]
pub enum ClientResponse {
    Boards(Arc<Vec<Board>>),
    Threads(Arc<Vec<ThreadPage>>),
    Catalog(Arc<Vec<CatalogPage>>),
    Archive(Arc<Vec<i32>>),
    Index(Arc<Index>),
    Thread(Arc<Thread>),
    NotModified,
}

impl ClientResponse {
    pub async fn parse(
        endpoint: &Endpoint,
        resp: reqwest::Response,
    ) -> Result<Self, reqwest::Error> {
        match endpoint {
            Endpoint::Boards => Ok(ClientResponse::Boards(Arc::new(resp.json::<BoardsResponse>().await?.boards))),
            Endpoint::Threads(_) => Ok(ClientResponse::Threads(Arc::new(resp.json().await?))),
            Endpoint::Catalog(_) => Ok(ClientResponse::Catalog(Arc::new(resp.json().await?))),
            Endpoint::Archive(_) => Ok(ClientResponse::Archive(Arc::new(resp.json().await?))),
            Endpoint::Index(_, _) => Ok(ClientResponse::Index(Arc::new(resp.json().await?))),
            Endpoint::Thread(_, _) => Ok(ClientResponse::Thread(Arc::new(resp.json().await?))),
        }
    }
}

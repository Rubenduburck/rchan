use std::{collections::HashMap, sync::Arc};

use tokio::sync::mpsc::Sender;
use tracing::debug;

use super::{endpoint::Endpoint, response::ClientResponse};

pub enum CacheRequest {
    LastCalled(Endpoint, Sender<CacheResponse>),
    LastResponse(Endpoint, Sender<CacheResponse>),
    Update(Endpoint, Arc<ClientResponse>),
}

#[derive(Debug, Clone)]
pub enum CacheResponse {
    LastCalled(chrono::DateTime<chrono::Utc>),
    LastResponse(Arc<ClientResponse>),
    None,
}

#[derive(Debug, Clone)]
pub struct ClientCache {
    pub receiver: Sender<CacheRequest>,
}

pub struct CacheInner {
    last_called: HashMap<Endpoint, chrono::DateTime<chrono::Utc>>,
    last_response: HashMap<Endpoint, Arc<ClientResponse>>,
}

impl ClientCache {
    pub fn new() -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<CacheRequest>(100);
        tokio::spawn(async move {
            let mut inner = CacheInner::new();
            loop {
                if let Some(request) = rx.recv().await {
                    inner.handle_request(request).await;
                }
            }
        });
        Self { receiver: tx }
    }

    pub async fn last_called(&self, endpoint: Endpoint) -> Option<chrono::DateTime<chrono::Utc>> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        self.receiver
            .send(CacheRequest::LastCalled(endpoint, tx))
            .await
            .unwrap();
        match rx.recv().await.unwrap() {
            CacheResponse::LastCalled(time) => Some(time),
            _ => None,
        }
    }

    pub async fn update(&self, endpoint: Endpoint, response: Arc<ClientResponse>) {
        self.receiver
            .send(CacheRequest::Update(endpoint, response))
            .await
            .unwrap();
    }

    pub async fn last_response(&self, endpoint: Endpoint) -> Option<Arc<ClientResponse>> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        self.receiver
            .send(CacheRequest::LastResponse(endpoint, tx))
            .await
            .unwrap();
        match rx.recv().await.unwrap() {
            CacheResponse::LastResponse(response) => Some(response),
            _ => None,
        }
    }
}

impl Default for ClientCache {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheInner {
    pub fn new() -> Self {
        Self {
            last_called: HashMap::new(),
            last_response: HashMap::new(),
        }
    }
    pub async fn handle_request(&mut self, request: CacheRequest) {
        match request {
            CacheRequest::LastCalled(endpoint, tx) => {
                self.handle_last_called(&endpoint, tx).await;
            }
            CacheRequest::Update(endpoint, resp) => {
                self.handle_update(&endpoint, resp);
            }
            CacheRequest::LastResponse(endpoint, tx) => {
                self.handle_last_response(&endpoint, tx).await;
            }
        }
    }

    pub async fn handle_last_called(&self, endpoint: &Endpoint, tx: Sender<CacheResponse>) {
        match self.last_called.get(endpoint) {
            Some(time) => {
                debug!("Found last called time for {}", endpoint);
                tx.send(CacheResponse::LastCalled(*time))
                    .await
                    .unwrap();
            }
            None => {
                debug!("No last called time for {}", endpoint);
                tx.send(CacheResponse::None).await.unwrap();
            }
        }
    }

    pub async fn handle_last_response(&self, endpoint: &Endpoint, tx: Sender<CacheResponse>) {
        match self.last_response.get(endpoint) {
            Some(response) => {
                debug!("Found cached response for {}", endpoint);
                tx.send(CacheResponse::LastResponse(response.clone()))
                    .await
                    .unwrap();
            }
            None => {
                debug!("No cached response for {}", endpoint);
                tx.send(CacheResponse::None).await.unwrap();
            }
        }
    }

    pub fn handle_update(&mut self, endpoint: &Endpoint, response: Arc<ClientResponse>) {
        debug!("Updating cache for {}", endpoint);
        self.last_called
            .insert(endpoint.clone(), chrono::Utc::now());
        self.last_response
            .insert(endpoint.clone(), response);
    }
}

impl Default for CacheInner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use rchan_types::board::BoardsResponse;

    use tokio::sync::mpsc::channel;

    #[tokio::test]
    async fn test_cache() {
        let endpoint = Endpoint::Boards;
        let cache = ClientCache::new();
        let (tx, mut rx) = channel(1);
        cache
            .receiver
            .send(CacheRequest::LastCalled(endpoint.clone(), tx))
            .await
            .unwrap();
        let response = rx.recv().await.unwrap();
        assert!(matches!(response, CacheResponse::None));

        let resp = Arc::new(ClientResponse::Boards(BoardsResponse{
            boards: vec![]
        }));

        let update_request = CacheRequest::Update(endpoint.clone(), resp.clone());
        cache.receiver.send(update_request).await.unwrap();

        let (tx, mut rx) = channel(1);
        let last_called_request = CacheRequest::LastCalled(endpoint.clone(), tx);
        cache.receiver.send(last_called_request).await.unwrap();
        let response = rx.recv().await.unwrap();
        assert!(matches!(response, CacheResponse::LastCalled(_)));
        println!("{:?}", response);

        let (tx, mut rx) = channel(1);
        let last_response_request = CacheRequest::LastResponse(endpoint.clone(), tx);
        cache.receiver.send(last_response_request).await.unwrap();
        let response = rx.recv().await.unwrap();
        assert!(matches!(response, CacheResponse::LastResponse(_)));
        println!("{:?}", response);
    }
}

use std::collections::HashMap;

use tokio::sync::mpsc::Sender;

use super::endpoint::Endpoint;

pub enum CacheRequest {
    LastCalled(Endpoint, Sender<CacheResponse>),
    UpdateLastCalled(Endpoint),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CacheResponse {
    LastCalled(Option<chrono::DateTime<chrono::Utc>>),
}

#[derive(Debug, Clone)]
pub struct ClientCache {
    pub receiver: Sender<CacheRequest>,
}

pub struct CacheInner {
    last_called: HashMap<Endpoint, chrono::DateTime<chrono::Utc>>,
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
            CacheResponse::LastCalled(time) => time,
        }
    }

    pub async fn update_last_called(&self, endpoint: Endpoint) {
        self.receiver
            .send(CacheRequest::UpdateLastCalled(endpoint))
            .await
            .unwrap();
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
        }
    }
    pub async fn handle_request(&mut self, request: CacheRequest) {
        match request {
            CacheRequest::LastCalled(endpoint, tx) => {
                self.handle_last_called(&endpoint, tx).await;
            }
            CacheRequest::UpdateLastCalled(endpoint) => {
                self.handle_update_last_called(&endpoint);
            }
        }
    }

    pub async fn handle_last_called(&self, endpoint: &Endpoint, tx: Sender<CacheResponse>) {
        tx.send(CacheResponse::LastCalled(
            self.last_called.get(endpoint).cloned(),
        ))
        .await
        .unwrap();
    }

    pub fn handle_update_last_called(&mut self, endpoint: &Endpoint) {
        self.last_called
            .insert(endpoint.clone(), chrono::Utc::now());
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
    use tokio::sync::mpsc::channel;

    #[tokio::test]
    async fn test_cache() {
        let cache = ClientCache::new();
        let (tx, mut rx) = channel(1);
        cache
            .receiver
            .send(CacheRequest::LastCalled(Endpoint::Boards, tx))
            .await
            .unwrap();
        let response = rx.recv().await.unwrap();
        assert_eq!(response, CacheResponse::LastCalled(None));

        let update_request = CacheRequest::UpdateLastCalled(Endpoint::Boards);
        cache.receiver.send(update_request).await.unwrap();

        let (tx, mut rx) = channel(1);
        let last_called_request = CacheRequest::LastCalled(Endpoint::Boards, tx);
        cache.receiver.send(last_called_request).await.unwrap();
        let response = rx.recv().await.unwrap();
        assert!(matches!(response, CacheResponse::LastCalled(Some(_))));
        println!("{:?}", response);
    }
}

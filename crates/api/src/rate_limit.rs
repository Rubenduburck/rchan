use tokio::sync::mpsc::Sender;
use tracing::{debug, error};

#[derive(Debug, Clone)]
pub struct RateLimiter {
    rate_limit_per_interval: usize,
    interval_duration_ms: u128,
    timestamps: Vec<u128>,
}

impl RateLimiter {
    pub fn new(rate_limit_per_interval: usize, interval_duration_ms: u128) -> Self {
        assert!(rate_limit_per_interval > 0);
        Self {
            rate_limit_per_interval,
            interval_duration_ms,
            timestamps: vec![],
        }
    }

    fn now() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    }

    async fn rate_limit(&mut self) {
        let now = Self::now();
        self.timestamps = self
            .timestamps
            .iter()
            .filter(|&&t| t > now)
            .copied()
            .collect::<Vec<_>>();
        if self.timestamps.len() >= self.rate_limit_per_interval {
            let sleep_duration = self.timestamps.first().unwrap() - now;
            debug!("Rate limiting: sleeping for {} ms", sleep_duration);
            self.timestamps
                .push(now + sleep_duration + self.interval_duration_ms);
            tokio::time::sleep(std::time::Duration::from_millis(sleep_duration as u64)).await;
        } else {
            self.timestamps.push(now + self.interval_duration_ms);
        }
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitedClient {
    receiver: Sender<ClientRequest>,
}

enum ClientRequest {
    Get(String, Sender<reqwest::Response>),
    Execute(reqwest::Request, Sender<reqwest::Response>),
}

impl RateLimitedClient {
    pub fn new(rate_limit_per_interval: usize, interval_duration_ms: u128) -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        tokio::spawn(async move {
            let mut rl = RateLimiter::new(rate_limit_per_interval, interval_duration_ms);
            let client = reqwest::Client::new();
            loop {
                if let Some(req) = rx.recv().await {
                    rl.rate_limit().await;
                    let client = client.clone();
                    match req {
                        ClientRequest::Get(url, tx) => {
                            Self::handle_get(client, tx, url).await;
                        }
                        ClientRequest::Execute(req, tx) => {
                            Self::handle_execute(client, tx, req).await;
                        }
                    }
                }
            }
        });
        Self { receiver: tx }
    }

    async fn handle_get(client: reqwest::Client, tx: Sender<reqwest::Response>, url: String) {
        tokio::spawn(async move {
            let response = client.get(&url).send().await.unwrap();
            tx.send(response).await.unwrap();
        });
    }

    async fn handle_execute(
        client: reqwest::Client,
        tx: Sender<reqwest::Response>,
        request: reqwest::Request,
    ) {
        tokio::spawn(async move {
            let response = client.execute(request).await.unwrap();
            tx.send(response).await.unwrap();
        });
    }

    pub async fn get(&self, url: &str) -> Result<reqwest::Response, reqwest::Error> {
        debug!("getting {}", url);
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        self.receiver
            .send(ClientRequest::Get(url.to_string(), tx))
            .await
            .map_err(|e| error!("{:?}", e))
            .unwrap();
        Ok(rx.recv().await.unwrap())
    }

    pub async fn execute(
        &self,
        request: reqwest::Request,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        self.receiver
            .send(ClientRequest::Execute(request, tx))
            .await
            .map_err(|e| error!("{:?}", e))
            .unwrap();
        Ok(rx.recv().await.unwrap())
    }
}

impl Default for RateLimitedClient {
    fn default() -> Self {
        Self::new(1, std::time::Duration::from_secs(1).as_millis())
    }
}

// tokio tests
#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::*;

    #[tokio::test]
    async fn test_client() {
        let client = RateLimitedClient::default();
        let response = client.get("https://google.com").await.unwrap();
        assert!(response.status().is_success());
    }

    #[tracing_test::traced_test]
    #[tokio::test]
    async fn test_rate_limiter() {
        let rate_limit_per_interval = 1;
        let interval_duration_ms = 1000;
        let n = 3;

        let mut rate_limiter = RateLimiter::new(rate_limit_per_interval, interval_duration_ms);
        let now = SystemTime::now();
        for _ in 0..n {
            rate_limiter.rate_limit().await;
        }
        debug!("elapsed: {}", now.elapsed().unwrap().as_millis());
        assert!(
            now.elapsed().unwrap().as_millis()
                >= (n - 1) / rate_limit_per_interval as u128 * interval_duration_ms
        );
    }

    #[tracing_test::traced_test]
    #[tokio::test]
    async fn test_rate_limited_http_client() {
        let rate_limit_per_interval = 1;
        let interval_duration_ms = 1000;
        let n = 10;

        let client = RateLimitedClient::new(rate_limit_per_interval, interval_duration_ms);
        let url = "http://google.com".to_string();
        let now = SystemTime::now();
        let mut set = tokio::task::JoinSet::new();
        for _ in 0..n {
            let client = client.clone();
            let url = url.clone();
            set.spawn(async move {
                let response = client.get(&url).await.unwrap();
                let status = response.status();
                debug!("status: {}", status);
                assert!(status.is_success());
            });
        }
        while (set.join_next().await).is_some() {}
        assert!(
            now.elapsed().unwrap().as_millis()
                >= (n - 1) / rate_limit_per_interval as u128 * interval_duration_ms
        );
    }
}

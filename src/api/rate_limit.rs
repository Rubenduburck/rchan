use tokio::sync::mpsc::Sender;
use tracing::debug;

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

    fn rate_limit(&mut self) {
        let mut now = Self::now();
        self.timestamps = self
            .timestamps
            .iter()
            .filter(|&&t| t > now)
            .copied()
            .collect::<Vec<_>>();
        if self.timestamps.len() >= self.rate_limit_per_interval {
            let sleep_duration = self.timestamps.first().unwrap() - now;
            debug!("Rate limiting: sleeping for {} ms", sleep_duration);
            tokio::time::sleep(std::time::Duration::from_millis(sleep_duration as u64));
            now = Self::now();
        }
        self.timestamps.push(now + self.interval_duration_ms);
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitedClient {
    receiver: Sender<(reqwest::Request, Sender<reqwest::Response>)>,
}

impl RateLimitedClient {
    pub fn new(rate_limit_per_interval: usize, interval_duration_ms: u128) -> Self {
        let (tx, mut rx) =
            tokio::sync::mpsc::channel::<(reqwest::Request, Sender<reqwest::Response>)>(100);
        tokio::spawn(async move {
            let mut rl = RateLimiter::new(rate_limit_per_interval, interval_duration_ms);
            let client = reqwest::Client::new();
            loop {
                if let Some((req, tx)) = rx.recv().await {
                    rl.rate_limit();
                    let response = client.execute(req).await.unwrap();
                    tx.send(response)
                        .await
                        .map_err(|e| println!("{:?}", e))
                        .unwrap();
                }
            }
        });
        Self { receiver: tx }
    }

    pub async fn get(&self, url: &str) -> Result<reqwest::Response, reqwest::Error> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        self.receiver
            .send((
                reqwest::Request::new(reqwest::Method::GET, url.parse().unwrap()),
                tx,
            ))
            .await
            .map_err(|e| println!("{:?}", e))
            .unwrap();
        Ok(rx.recv().await.unwrap())
    }

    pub async fn execute(
        &self,
        request: reqwest::Request,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        self.receiver
            .send((request, tx))
            .await
            .map_err(|e| println!("{:?}", e))
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

    #[tokio::test]
    async fn test_rate_limiter() {
        let rate_limit_per_interval = 1;
        let interval_duration_ms = 1000;
        let n = 3;

        let mut rate_limiter = RateLimiter::new(rate_limit_per_interval, interval_duration_ms);
        let now = SystemTime::now();
        (0..n).for_each(|_| {
            rate_limiter.rate_limit();
        });
        println!("elapsed: {}", now.elapsed().unwrap().as_millis());
        assert!(
            now.elapsed().unwrap().as_millis()
                >= (n - 1) / rate_limit_per_interval as u128 * interval_duration_ms
        );
    }

    #[tokio::test]
    async fn test_rate_limited_http_client() {
        let rate_limit_per_interval = 1;
        let interval_duration_ms = 1000;
        let n = 3;

        let client = RateLimitedClient::new(rate_limit_per_interval, interval_duration_ms);
        let url = "http://google.com".to_string();
        let now = SystemTime::now();
        for _ in 0..n {
            let response = client.get(&url).await.unwrap();
            let status = response.status();
            println!("status: {}", status);
            assert!(status.is_success());
        }
        assert!(
            now.elapsed().unwrap().as_millis()
                >= (n - 1) / rate_limit_per_interval as u128 * interval_duration_ms
        );
    }
}

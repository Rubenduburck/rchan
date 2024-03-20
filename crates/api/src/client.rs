use tracing::{debug, error};

use super::{
    cache::ClientCache, endpoint::Endpoint, error::Error, rate_limit::RateLimitedClient,
    response::ClientResponse,
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Client {
    http: Arc<RateLimitedClient>,
    cache: Arc<ClientCache>,
}

/// A client for interacting with the 4chan API.
/// This client conforms to the API rules specified on
/// https://github.com/4chan/4chan-API/blob/master/README.md
/// 1. No more than one request per second.
/// 2. Thread updating should be set to a minimum of 10 seconds, preferably higher.
/// 3. Use If-Modified-Since when doing your requests.
/// 4. Make API requests using the same protocol as the app. Only use SSL when a user is accessing
///    your app over HTTPS.
impl Client {
    pub fn new() -> Self {
        Self {
            http: Arc::new(RateLimitedClient::default()),
            cache: Arc::new(ClientCache::new()),
        }
    }

    async fn new_request(&self, endpoint: &Endpoint, https: bool) -> reqwest::Request {
        let mut request =
            reqwest::Request::new(reqwest::Method::GET, endpoint.url(https).parse().unwrap());
        if let Some(time) = self.cache.last_called(endpoint.clone()).await {
            request.headers_mut().insert(
                reqwest::header::IF_MODIFIED_SINCE,
                reqwest::header::HeaderValue::from_str(&time.to_rfc2822()).unwrap(),
            );
        }
        request
    }

    pub async fn get(
        &self,
        endpoint: &Endpoint,
        https: bool,
    ) -> Result<Arc<ClientResponse>, Error> {
        debug!("Sending request to {}", endpoint.url(https));
        self.handle_response(
            endpoint,
            self.http
                .execute(self.new_request(endpoint, https).await)
                .await?,
        )
        .await
    }

    pub async fn handle_response(
        &self,
        endpoint: &Endpoint,
        resp: reqwest::Response,
    ) -> Result<Arc<ClientResponse>, Error> {
        match resp.status() {
            reqwest::StatusCode::OK => {
                debug!("request: {} status: OK", endpoint);
                let parsed = Arc::new(self.parse_response(endpoint, resp).await?);
                self.cache.update(endpoint.clone(), parsed.clone()).await;
                Ok(parsed)
            }
            reqwest::StatusCode::NOT_MODIFIED => {
                debug!("request: {} status: NOT_MODIFIED", endpoint);
                Ok(self.cache.last_response(endpoint.clone()).await.unwrap())
            }
            _ => {
                error!("request {} status: {}", endpoint, resp.status());
                Err(Error::Generic(format!(
                    "Received status code: {}",
                    resp.status()
                )))
            }
        }
    }

    pub async fn parse_response(
        &self,
        endpoint: &Endpoint,
        resp: reqwest::Response,
    ) -> Result<ClientResponse, Error> {
        match endpoint {
            Endpoint::Boards => Ok(ClientResponse::Boards(resp.json().await?)),
            Endpoint::Threads(_) => Ok(ClientResponse::Threads(resp.json().await?)),
            Endpoint::Catalog(_) => Ok(ClientResponse::Catalog(resp.json().await?)),
            Endpoint::Archive(_) => Ok(ClientResponse::Archive(resp.json().await?)),
            Endpoint::Index(_, _) => Ok(ClientResponse::Index(resp.json().await?)),
            Endpoint::Thread(_, _) => Ok(ClientResponse::Thread(resp.json().await?)),
        }
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::*;

    #[test]
    fn dummy_test() {
        assert_eq!(1, 1);
    }

    #[tracing_test::traced_test]
    #[tokio::test]
    async fn test_client_rate_limit() {
        let endpoints = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "k"]
            .iter()
            .map(|x| Endpoint::Threads(x.to_string()));
        let client_0 = Client::new();
        let client_1 = client_0.clone();
        let client_2 = client_0.clone();
        let clients = vec![client_0, client_1, client_2];
        let now = SystemTime::now();
        for (i, endpoint) in endpoints.enumerate() {
            debug!("Sending request to {}", endpoint.url(false));
            let resp = clients[i % 3].get(&endpoint, false).await.unwrap();
            assert!(matches!(*resp, ClientResponse::Threads(_)));
        }
        let elapsed = now.elapsed().unwrap().as_millis();
        assert!(elapsed >= 9000);
    }

    #[tracing_test::traced_test]
	#[tokio::test]
    async fn test_if_modified_since() {
        let endpoint = Endpoint::Boards;
        let client = Client::new();
        let resp = client.get(&endpoint, false).await.unwrap();
        assert!(matches!(*resp, ClientResponse::Boards(_)));
        let resp = client.get(&endpoint, false).await.unwrap();
        assert!(matches!(*resp, ClientResponse::Boards(_)));
    }

    #[tracing_test::traced_test]
	#[tokio::test]
    async fn test_get_boards() {
        let client = Client::default();
        let endpoint = Endpoint::Boards;
        let resp = client.get(&endpoint, false).await.unwrap();
        debug!("{:?}", resp);
    }

    #[tracing_test::traced_test]
	#[tokio::test]
    async fn test_get_threads() {
        let client = Client::default();
        let endpoint = Endpoint::Threads("g".to_string());
        let resp = client.get(&endpoint, false).await.unwrap();
        debug!("{:?}", resp);
    }

    #[tracing_test::traced_test]
	#[tokio::test]
    async fn test_get_catalog() {
        let client = Client::default();
        let endpoint = Endpoint::Catalog("g".to_string());
        let resp = client.get(&endpoint, false).await.unwrap();
        debug!("{:?}", resp);
    }

    #[tracing_test::traced_test]
	#[tokio::test]
    async fn test_get_archive() {
        let client = Client::default();
        let endpoint = Endpoint::Archive("g".to_string());
        let resp = client.get(&endpoint, false).await.unwrap();
        debug!("{:?}", resp);
    }

    #[tracing_test::traced_test]
	#[tokio::test]
    async fn test_get_index() {
        let client = Client::default();
        let endpoint = Endpoint::Index("g".to_string(), 1);
        let resp = client.get(&endpoint, false).await.unwrap();
        debug!("{:?}", resp);
    }

    #[tracing_test::traced_test]
	#[tokio::test]
    async fn test_get_thread() {
        let client = Client::default();
        let endpoint = Endpoint::Thread("g".to_string(), 99566851);
        let resp = client.get(&endpoint, false).await.unwrap();
        debug!("{:?}", resp);
    }
}

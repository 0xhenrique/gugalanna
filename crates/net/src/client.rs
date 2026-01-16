//! HTTP client implementation

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use log::{debug, info};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT, ACCEPT_ENCODING, USER_AGENT};
use url::Url;

use crate::error::{NetError, NetResult};
use crate::response::Response;

/// Default user agent string
const DEFAULT_USER_AGENT: &str = concat!("Gugalanna/", env!("CARGO_PKG_VERSION"));

/// Default timeout in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Maximum number of redirects to follow
const MAX_REDIRECTS: usize = 10;

/// A tracked network request for DevTools
#[derive(Debug, Clone)]
pub struct NetworkRequest {
    /// Unique request ID
    pub id: usize,
    /// HTTP method
    pub method: String,
    /// Request URL
    pub url: String,
    /// Response status (None if pending)
    pub status: Option<u16>,
    /// Response body size in bytes
    pub response_size: Option<usize>,
    /// Request duration
    pub duration: Option<Duration>,
    /// When the request started
    pub started_at: Instant,
    /// Request headers
    pub request_headers: Vec<(String, String)>,
    /// Response headers
    pub response_headers: Vec<(String, String)>,
}

/// Shared network request storage for DevTools
pub type NetworkRequests = Arc<Mutex<Vec<NetworkRequest>>>;

/// Create a new network request storage
pub fn new_network_requests() -> NetworkRequests {
    Arc::new(Mutex::new(Vec::new()))
}

/// HTTP client for fetching resources
#[derive(Clone)]
pub struct HttpClient {
    client: reqwest::Client,
    /// Optional request tracking for DevTools
    requests: Option<NetworkRequests>,
    /// Counter for request IDs
    next_id: Arc<AtomicUsize>,
}

impl HttpClient {
    /// Create a new HTTP client with default settings
    pub fn new() -> NetResult<Self> {
        Self::with_config(ClientConfig::default())
    }

    /// Create a new HTTP client with custom configuration
    pub fn with_config(config: ClientConfig) -> NetResult<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(DEFAULT_USER_AGENT));
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"),
        );
        headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(config.timeout_secs))
            .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
            .build()
            .map_err(|e| NetError::RequestFailed(e.to_string()))?;

        Ok(Self {
            client,
            requests: None,
            next_id: Arc::new(AtomicUsize::new(0)),
        })
    }

    /// Create a new HTTP client with request tracking for DevTools
    pub fn with_tracking(requests: NetworkRequests) -> NetResult<Self> {
        let mut client = Self::new()?;
        client.requests = Some(requests);
        Ok(client)
    }

    /// Get the next request ID
    fn next_request_id(&self) -> usize {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Track start of a request
    fn track_request_start(&self, method: &str, url: &str, headers: &[(String, String)]) -> Option<usize> {
        if let Some(ref requests) = self.requests {
            let id = self.next_request_id();
            if let Ok(mut reqs) = requests.lock() {
                reqs.push(NetworkRequest {
                    id,
                    method: method.to_string(),
                    url: url.to_string(),
                    status: None,
                    response_size: None,
                    duration: None,
                    started_at: Instant::now(),
                    request_headers: headers.to_vec(),
                    response_headers: vec![],
                });
            }
            Some(id)
        } else {
            None
        }
    }

    /// Track completion of a request
    fn track_request_complete(
        &self,
        id: usize,
        status: u16,
        response_size: usize,
        response_headers: Vec<(String, String)>,
    ) {
        if let Some(ref requests) = self.requests {
            if let Ok(mut reqs) = requests.lock() {
                if let Some(req) = reqs.iter_mut().find(|r| r.id == id) {
                    req.status = Some(status);
                    req.response_size = Some(response_size);
                    req.duration = Some(req.started_at.elapsed());
                    req.response_headers = response_headers;
                }
            }
        }
    }

    /// Fetch a URL using GET
    pub async fn get(&self, url: &Url) -> NetResult<Response> {
        self.get_with_headers(url, HashMap::new()).await
    }

    /// Fetch a URL with custom headers
    pub async fn get_with_headers(
        &self,
        url: &Url,
        extra_headers: HashMap<String, String>,
    ) -> NetResult<Response> {
        info!("Fetching: {}", url);

        // Track request start
        let req_headers: Vec<(String, String)> = extra_headers
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let request_id = self.track_request_start("GET", url.as_str(), &req_headers);

        let mut request = self.client.get(url.clone());

        // Add extra headers
        for (key, value) in extra_headers {
            if let (Ok(name), Ok(val)) = (
                HeaderName::try_from(key.as_str()),
                HeaderValue::try_from(value.as_str()),
            ) {
                request = request.header(name, val);
            }
        }

        let response = request.send().await?;

        let final_url = response.url().clone();
        let status = response.status().as_u16();

        debug!("Response status: {}", status);

        // Convert headers
        let headers: HashMap<String, String> = response
            .headers()
            .iter()
            .filter_map(|(k, v)| {
                v.to_str()
                    .ok()
                    .map(|val| (k.as_str().to_lowercase(), val.to_string()))
            })
            .collect();

        let body = response.bytes().await?.to_vec();

        debug!("Received {} bytes", body.len());

        // Track request completion
        if let Some(id) = request_id {
            let resp_headers: Vec<(String, String)> = headers
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            self.track_request_complete(id, status, body.len(), resp_headers);
        }

        Ok(Response::new(final_url, status, headers, body))
    }

    /// Send a POST request with form data
    pub async fn post_form(&self, url: &Url, form_data: &str) -> NetResult<Response> {
        info!("POST to: {} with data: {}", url, form_data);

        // Track request start
        let req_headers = vec![
            ("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string()),
        ];
        let request_id = self.track_request_start("POST", url.as_str(), &req_headers);

        let response = self
            .client
            .post(url.clone())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form_data.to_string())
            .send()
            .await?;

        let final_url = response.url().clone();
        let status = response.status().as_u16();

        debug!("Response status: {}", status);

        // Convert headers
        let headers: HashMap<String, String> = response
            .headers()
            .iter()
            .filter_map(|(k, v)| {
                v.to_str()
                    .ok()
                    .map(|val| (k.as_str().to_lowercase(), val.to_string()))
            })
            .collect();

        let body = response.bytes().await?.to_vec();

        debug!("Received {} bytes", body.len());

        // Track request completion
        if let Some(id) = request_id {
            let resp_headers: Vec<(String, String)> = headers
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            self.track_request_complete(id, status, body.len(), resp_headers);
        }

        Ok(Response::new(final_url, status, headers, body))
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default HTTP client")
    }
}

/// HTTP client configuration
pub struct ClientConfig {
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            timeout_secs: DEFAULT_TIMEOUT_SECS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_example() {
        let client = HttpClient::new().unwrap();
        let url = Url::parse("https://example.com").unwrap();
        let response = client.get(&url).await.unwrap();

        assert!(response.is_success());
        assert!(response.text_lossy().contains("Example Domain"));
    }
}

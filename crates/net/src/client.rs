//! HTTP client implementation

use std::collections::HashMap;
use std::time::Duration;

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

/// HTTP client for fetching resources
#[derive(Clone)]
pub struct HttpClient {
    client: reqwest::Client,
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

        Ok(Self { client })
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

        Ok(Response::new(final_url, status, headers, body))
    }

    /// Send a POST request with form data
    pub async fn post_form(&self, url: &Url, form_data: &str) -> NetResult<Response> {
        info!("POST to: {} with data: {}", url, form_data);

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

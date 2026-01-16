//! HTTP response representation

use std::collections::HashMap;
use url::Url;

/// HTTP response
#[derive(Debug)]
pub struct Response {
    /// Final URL after redirects
    pub url: Url,
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body as bytes
    pub body: Vec<u8>,
}

impl Response {
    /// Create a new response
    pub fn new(url: Url, status: u16, headers: HashMap<String, String>, body: Vec<u8>) -> Self {
        Self {
            url,
            status,
            headers,
            body,
        }
    }

    /// Check if the response was successful (2xx)
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// Get the Content-Type header
    pub fn content_type(&self) -> Option<&str> {
        self.headers
            .get("content-type")
            .or_else(|| self.headers.get("Content-Type"))
            .map(|s| s.as_str())
    }

    /// Get the body as a UTF-8 string
    pub fn text(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.body.clone())
    }

    /// Get the body as a UTF-8 string, replacing invalid characters
    pub fn text_lossy(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

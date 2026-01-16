//! Resource loading abstraction

use std::collections::HashMap;

use url::Url;

use crate::client::HttpClient;
use crate::error::NetResult;
use crate::response::Response;

/// Type of resource being loaded
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    /// HTML document
    Document,
    /// CSS stylesheet
    Stylesheet,
    /// JavaScript
    Script,
    /// Image
    Image,
    /// Font
    Font,
    /// Other/unknown
    Other,
}

impl ResourceType {
    /// Detect resource type from Content-Type header
    pub fn from_content_type(content_type: &str) -> Self {
        let ct = content_type.to_lowercase();
        if ct.contains("text/html") || ct.contains("application/xhtml+xml") {
            ResourceType::Document
        } else if ct.contains("text/css") {
            ResourceType::Stylesheet
        } else if ct.contains("javascript") || ct.contains("application/json") {
            ResourceType::Script
        } else if ct.contains("image/") {
            ResourceType::Image
        } else if ct.contains("font/") || ct.contains("application/font") {
            ResourceType::Font
        } else {
            ResourceType::Other
        }
    }

    /// Detect resource type from URL extension
    pub fn from_url(url: &Url) -> Self {
        let path = url.path().to_lowercase();
        if path.ends_with(".html") || path.ends_with(".htm") || path.ends_with("/") {
            ResourceType::Document
        } else if path.ends_with(".css") {
            ResourceType::Stylesheet
        } else if path.ends_with(".js") || path.ends_with(".mjs") {
            ResourceType::Script
        } else if path.ends_with(".png")
            || path.ends_with(".jpg")
            || path.ends_with(".jpeg")
            || path.ends_with(".gif")
            || path.ends_with(".webp")
            || path.ends_with(".svg")
        {
            ResourceType::Image
        } else if path.ends_with(".woff")
            || path.ends_with(".woff2")
            || path.ends_with(".ttf")
            || path.ends_with(".otf")
        {
            ResourceType::Font
        } else {
            ResourceType::Other
        }
    }
}

/// Abstraction for loading resources
pub struct ResourceLoader {
    client: HttpClient,
    /// Simple in-memory cache (URL -> Response)
    cache: HashMap<String, Response>,
}

impl ResourceLoader {
    /// Create a new resource loader
    pub fn new() -> NetResult<Self> {
        Ok(Self {
            client: HttpClient::new()?,
            cache: HashMap::new(),
        })
    }

    /// Load a resource from a URL
    pub async fn load(&mut self, url: &Url) -> NetResult<&Response> {
        let key = url.to_string();

        // Check cache first
        if self.cache.contains_key(&key) {
            log::debug!("Cache hit: {}", url);
            return Ok(self.cache.get(&key).unwrap());
        }

        // Fetch from network
        let response = self.client.get(url).await?;
        self.cache.insert(key.clone(), response);
        Ok(self.cache.get(&key).unwrap())
    }

    /// Load a resource without caching
    pub async fn load_uncached(&self, url: &Url) -> NetResult<Response> {
        self.client.get(url).await
    }

    /// Resolve a relative URL against a base URL
    pub fn resolve_url(&self, base: &Url, relative: &str) -> NetResult<Url> {
        base.join(relative).map_err(|e| e.into())
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

impl Default for ResourceLoader {
    fn default() -> Self {
        Self::new().expect("Failed to create default ResourceLoader")
    }
}

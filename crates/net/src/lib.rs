//! Gugalanna Network Layer
//!
//! Provides HTTP/HTTPS fetching capabilities for the browser.

mod client;
mod error;
mod loader;
mod response;

pub use client::{HttpClient, NetworkRequest, NetworkRequests, new_network_requests};
pub use error::{NetError, NetResult};
pub use loader::{ResourceLoader, ResourceType};
pub use response::Response;

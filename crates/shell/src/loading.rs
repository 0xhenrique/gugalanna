//! Loading State Management
//!
//! Types for tracking navigation state and errors.

use url::Url;

/// Loading state for the browser
#[derive(Debug, Clone, Default)]
pub enum LoadingState {
    /// No navigation in progress
    #[default]
    Idle,
    /// Page is being fetched
    Loading {
        /// URL being loaded
        url: Url,
    },
    /// Navigation failed
    Failed {
        /// URL that failed
        url: Url,
        /// Error information
        error: NavigationError,
    },
}

/// Navigation error types
#[derive(Debug, Clone)]
pub enum NavigationError {
    /// HTTP error (404, 500, etc.)
    HttpError { status: u16 },
    /// Network unreachable / connection refused
    NetworkError(String),
    /// Request timed out
    Timeout,
    /// Navigation was cancelled
    Cancelled,
}

impl NavigationError {
    /// Human-readable title for error page
    pub fn title(&self) -> &'static str {
        match self {
            Self::HttpError { status } if *status == 404 => "Page Not Found",
            Self::HttpError { status } if *status >= 500 => "Server Error",
            Self::HttpError { .. } => "HTTP Error",
            Self::NetworkError(_) => "Network Error",
            Self::Timeout => "Connection Timed Out",
            Self::Cancelled => "Navigation Cancelled",
        }
    }

    /// Detailed description for error page
    pub fn details(&self) -> String {
        match self {
            Self::HttpError { status } => format!("The server returned status code {}", status),
            Self::NetworkError(msg) => msg.clone(),
            Self::Timeout => "The connection took too long to respond.".into(),
            Self::Cancelled => "Navigation was cancelled.".into(),
        }
    }
}

/// Result from async navigation task
pub enum NavigationResult {
    /// Successfully fetched page
    Success {
        /// Final URL (may differ from requested due to redirects)
        url: Url,
        /// HTML content
        html: String,
    },
    /// Navigation failed
    Failed {
        /// URL that failed
        url: Url,
        /// Error information
        error: NavigationError,
    },
}

//! Network error types

use thiserror::Error;

/// Network operation result type
pub type NetResult<T> = Result<T, NetError>;

/// Network errors
#[derive(Debug, Error)]
pub enum NetError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Request failed: {0}")]
    RequestFailed(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Timeout")]
    Timeout,

    #[error("TLS error: {0}")]
    TlsError(String),

    #[error("Too many redirects")]
    TooManyRedirects,

    #[error("HTTP error: {status}")]
    HttpError { status: u16 },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<reqwest::Error> for NetError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            NetError::Timeout
        } else if err.is_connect() {
            NetError::ConnectionError(err.to_string())
        } else if err.is_redirect() {
            NetError::TooManyRedirects
        } else {
            NetError::RequestFailed(err.to_string())
        }
    }
}

impl From<url::ParseError> for NetError {
    fn from(err: url::ParseError) -> Self {
        NetError::InvalidUrl(err.to_string())
    }
}

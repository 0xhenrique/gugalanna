//! HTML parsing error types

use thiserror::Error;

/// HTML parsing result type
pub type HtmlResult<T> = Result<T, HtmlError>;

/// HTML parsing errors
#[derive(Debug, Error)]
pub enum HtmlError {
    #[error("Unexpected character: {0}")]
    UnexpectedChar(char),

    #[error("Unexpected end of file")]
    UnexpectedEof,

    #[error("Invalid tag name: {0}")]
    InvalidTagName(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

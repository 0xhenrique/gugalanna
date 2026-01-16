//! DOM error types

use thiserror::Error;

/// DOM operation result type
pub type DomResult<T> = Result<T, DomError>;

/// DOM errors
#[derive(Debug, Error)]
pub enum DomError {
    #[error("Node not found: {0}")]
    NodeNotFound(u32),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Invalid node type for operation")]
    InvalidNodeType,
}

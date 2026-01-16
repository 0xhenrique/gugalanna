//! JavaScript error types

use thiserror::Error;

/// JavaScript runtime error
#[derive(Debug, Error)]
pub enum JsError {
    #[error("JavaScript error: {message}")]
    Runtime {
        message: String,
        stack: Option<String>,
    },

    #[error("QuickJS error: {0}")]
    QuickJs(String),

    #[error("Type error: {0}")]
    Type(String),

    #[error("Reference error: {0}")]
    Reference(String),

    #[error("Syntax error: {0}")]
    Syntax(String),
}

impl JsError {
    pub fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime {
            message: message.into(),
            stack: None,
        }
    }

    pub fn with_stack(message: impl Into<String>, stack: impl Into<String>) -> Self {
        Self::Runtime {
            message: message.into(),
            stack: Some(stack.into()),
        }
    }
}

impl From<rquickjs::Error> for JsError {
    fn from(err: rquickjs::Error) -> Self {
        Self::QuickJs(err.to_string())
    }
}

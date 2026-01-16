//! Gugalanna JavaScript Engine
//!
//! JavaScript execution via QuickJS with DOM bindings.

// TODO: Epic 7 - JavaScript Integration
// - QuickJS integration
// - Window object
// - Document object
// - Element API
// - Event system

/// JavaScript runtime wrapper
pub struct JsRuntime {
    // Will hold QuickJS runtime
    _placeholder: (),
}

impl JsRuntime {
    /// Create a new JavaScript runtime
    pub fn new() -> Self {
        Self { _placeholder: () }
    }

    /// Evaluate JavaScript code
    pub fn eval(&mut self, _code: &str) -> Result<JsValue, JsError> {
        // TODO: Implement with QuickJS
        Ok(JsValue::Undefined)
    }
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// JavaScript value
#[derive(Debug)]
pub enum JsValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Object,
    Array,
    Function,
}

/// JavaScript error
#[derive(Debug)]
pub struct JsError {
    pub message: String,
    pub stack: Option<String>,
}

impl std::fmt::Display for JsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JsError: {}", self.message)
    }
}

impl std::error::Error for JsError {}

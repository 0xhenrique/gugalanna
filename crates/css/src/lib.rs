//! Gugalanna CSS Parser
//!
//! CSS tokenizer and stylesheet parser.
//!
//! # Example
//!
//! ```
//! use gugalanna_css::{Stylesheet, Selector};
//!
//! // Parse a stylesheet
//! let css = "p { color: red; font-size: 16px; }";
//! let stylesheet = Stylesheet::parse(css).unwrap();
//!
//! // Parse a selector
//! let selector = Selector::parse("div.container > p").unwrap();
//! ```

mod error;
mod tokenizer;
mod value;
mod selector;
mod parser;

// Re-export main types
pub use error::{CssError, CssResult, SourceLocation};
pub use tokenizer::{Token, Tokenizer, HashType};
pub use value::{CssValue, Color, LengthUnit, TimeUnit, ValueParser};
pub use selector::{Selector, SelectorPart, Combinator, AttributeOp, Specificity};
pub use parser::{
    Stylesheet, Rule, StyleRule, Declaration,
    ImportRule, MediaRule, FontFaceRule, KeyframesRule, Keyframe,
    CssParser,
};

/// Parse inline style declarations from a style attribute value.
///
/// # Example
/// ```
/// use gugalanna_css::parse_inline_style;
///
/// let declarations = parse_inline_style("color: red; font-size: 16px;").unwrap();
/// assert_eq!(declarations.len(), 2);
/// ```
pub fn parse_inline_style(style: &str) -> CssResult<Vec<Declaration>> {
    let mut parser = CssParser::new(style);
    parser.parse_inline_style()
}

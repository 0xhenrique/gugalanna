//! Gugalanna HTML Parser
//!
//! HTML5 tokenizer and tree construction.

mod tokenizer;
mod tree_builder;
mod error;
mod entities;

pub use tokenizer::{Tokenizer, Token};
pub use tree_builder::HtmlParser;
pub use error::{HtmlError, HtmlResult};

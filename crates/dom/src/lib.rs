//! Gugalanna DOM - Document Object Model
//!
//! Provides the DOM tree structure for representing HTML documents.

mod node;
mod tree;
mod error;
mod query;

pub use node::{Node, NodeId, NodeType, ElementData};
pub use tree::DomTree;
pub use error::{DomError, DomResult};
pub use query::Queryable;

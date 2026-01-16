//! DOM Node representation

use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use std::fmt;

/// Unique identifier for a node in the DOM tree
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

impl NodeId {
    /// Create a new node ID
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId({})", self.0)
    }
}

/// Type of DOM node
#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    /// Document root node
    Document,
    /// DOCTYPE declaration
    Doctype {
        name: String,
        public_id: String,
        system_id: String,
    },
    /// Element node (HTML tag)
    Element(ElementData),
    /// Text content
    Text(String),
    /// HTML comment
    Comment(String),
}

/// Element-specific data
#[derive(Debug, Clone, PartialEq)]
pub struct ElementData {
    /// Tag name (lowercase)
    pub tag_name: String,
    /// Element attributes
    pub attributes: FxHashMap<String, String>,
}

impl ElementData {
    /// Create a new element with the given tag name
    pub fn new(tag_name: impl Into<String>) -> Self {
        Self {
            tag_name: tag_name.into().to_ascii_lowercase(),
            attributes: FxHashMap::default(),
        }
    }

    /// Get an attribute value
    pub fn get_attribute(&self, name: &str) -> Option<&str> {
        self.attributes.get(&name.to_ascii_lowercase()).map(|s| s.as_str())
    }

    /// Set an attribute value
    pub fn set_attribute(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.attributes.insert(name.into().to_ascii_lowercase(), value.into());
    }

    /// Remove an attribute
    pub fn remove_attribute(&mut self, name: &str) -> Option<String> {
        self.attributes.remove(&name.to_ascii_lowercase())
    }

    /// Check if the element has a class
    pub fn has_class(&self, class: &str) -> bool {
        self.get_attribute("class")
            .map(|classes| classes.split_whitespace().any(|c| c == class))
            .unwrap_or(false)
    }

    /// Get the element's ID
    pub fn id(&self) -> Option<&str> {
        self.get_attribute("id")
    }

    /// Get all classes as a vector
    pub fn classes(&self) -> Vec<&str> {
        self.get_attribute("class")
            .map(|c| c.split_whitespace().collect())
            .unwrap_or_default()
    }
}

/// A node in the DOM tree
#[derive(Debug, Clone)]
pub struct Node {
    /// Unique identifier
    pub id: NodeId,
    /// Node type and associated data
    pub node_type: NodeType,
    /// Parent node ID (None for root)
    pub parent: Option<NodeId>,
    /// Child node IDs
    pub children: SmallVec<[NodeId; 8]>,
    /// Previous sibling
    pub prev_sibling: Option<NodeId>,
    /// Next sibling
    pub next_sibling: Option<NodeId>,
}

impl Node {
    /// Create a new node
    pub fn new(id: NodeId, node_type: NodeType) -> Self {
        Self {
            id,
            node_type,
            parent: None,
            children: SmallVec::new(),
            prev_sibling: None,
            next_sibling: None,
        }
    }

    /// Check if this is a document node
    pub fn is_document(&self) -> bool {
        matches!(self.node_type, NodeType::Document)
    }

    /// Check if this is an element node
    pub fn is_element(&self) -> bool {
        matches!(self.node_type, NodeType::Element(_))
    }

    /// Check if this is a text node
    pub fn is_text(&self) -> bool {
        matches!(self.node_type, NodeType::Text(_))
    }

    /// Check if this is a comment node
    pub fn is_comment(&self) -> bool {
        matches!(self.node_type, NodeType::Comment(_))
    }

    /// Get element data if this is an element
    pub fn as_element(&self) -> Option<&ElementData> {
        match &self.node_type {
            NodeType::Element(data) => Some(data),
            _ => None,
        }
    }

    /// Get mutable element data if this is an element
    pub fn as_element_mut(&mut self) -> Option<&mut ElementData> {
        match &mut self.node_type {
            NodeType::Element(data) => Some(data),
            _ => None,
        }
    }

    /// Get text content if this is a text node
    pub fn as_text(&self) -> Option<&str> {
        match &self.node_type {
            NodeType::Text(text) => Some(text),
            _ => None,
        }
    }

    /// Get the tag name if this is an element
    pub fn tag_name(&self) -> Option<&str> {
        self.as_element().map(|e| e.tag_name.as_str())
    }
}

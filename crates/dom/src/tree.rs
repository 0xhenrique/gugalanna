//! DOM Tree structure

use rustc_hash::FxHashMap;
use std::fmt;

use crate::error::{DomError, DomResult};
use crate::node::{ElementData, Node, NodeId, NodeType};

/// DOM tree that owns all nodes
pub struct DomTree {
    /// All nodes in the tree
    nodes: FxHashMap<NodeId, Node>,
    /// Next available node ID
    next_id: u32,
    /// Root document node
    document_id: NodeId,
}

impl DomTree {
    /// Create a new empty DOM tree
    pub fn new() -> Self {
        let document_id = NodeId::new(0);
        let document = Node::new(document_id, NodeType::Document);

        let mut nodes = FxHashMap::default();
        nodes.insert(document_id, document);

        Self {
            nodes,
            next_id: 1,
            document_id,
        }
    }

    /// Get the document (root) node ID
    pub fn document_id(&self) -> NodeId {
        self.document_id
    }

    /// Get a node by ID
    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    /// Get a mutable node by ID
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(&id)
    }

    /// Create a new element node
    pub fn create_element(&mut self, tag_name: impl Into<String>) -> NodeId {
        let id = NodeId::new(self.next_id);
        self.next_id += 1;

        let element = ElementData::new(tag_name);
        let node = Node::new(id, NodeType::Element(element));
        self.nodes.insert(id, node);

        id
    }

    /// Create a new text node
    pub fn create_text(&mut self, content: impl Into<String>) -> NodeId {
        let id = NodeId::new(self.next_id);
        self.next_id += 1;

        let node = Node::new(id, NodeType::Text(content.into()));
        self.nodes.insert(id, node);

        id
    }

    /// Create a new comment node
    pub fn create_comment(&mut self, content: impl Into<String>) -> NodeId {
        let id = NodeId::new(self.next_id);
        self.next_id += 1;

        let node = Node::new(id, NodeType::Comment(content.into()));
        self.nodes.insert(id, node);

        id
    }

    /// Create a DOCTYPE node
    pub fn create_doctype(
        &mut self,
        name: impl Into<String>,
        public_id: impl Into<String>,
        system_id: impl Into<String>,
    ) -> NodeId {
        let id = NodeId::new(self.next_id);
        self.next_id += 1;

        let node = Node::new(
            id,
            NodeType::Doctype {
                name: name.into(),
                public_id: public_id.into(),
                system_id: system_id.into(),
            },
        );
        self.nodes.insert(id, node);

        id
    }

    /// Append a child node to a parent
    pub fn append_child(&mut self, parent_id: NodeId, child_id: NodeId) -> DomResult<()> {
        // Get the last child of parent to set sibling links
        let last_child = {
            let parent = self.get(parent_id).ok_or(DomError::NodeNotFound(parent_id.0))?;
            parent.children.last().copied()
        };

        // Update the new child's prev_sibling and parent
        {
            let child = self.get_mut(child_id).ok_or(DomError::NodeNotFound(child_id.0))?;
            child.parent = Some(parent_id);
            child.prev_sibling = last_child;
            child.next_sibling = None;
        }

        // Update the previous last child's next_sibling
        if let Some(last_child_id) = last_child {
            if let Some(last) = self.get_mut(last_child_id) {
                last.next_sibling = Some(child_id);
            }
        }

        // Add child to parent's children list
        {
            let parent = self.get_mut(parent_id).ok_or(DomError::NodeNotFound(parent_id.0))?;
            parent.children.push(child_id);
        }

        Ok(())
    }

    /// Remove a node from its parent
    pub fn remove_child(&mut self, parent_id: NodeId, child_id: NodeId) -> DomResult<()> {
        let (prev_sibling, next_sibling) = {
            let child = self.get(child_id).ok_or(DomError::NodeNotFound(child_id.0))?;
            (child.prev_sibling, child.next_sibling)
        };

        // Update siblings
        if let Some(prev_id) = prev_sibling {
            if let Some(prev) = self.get_mut(prev_id) {
                prev.next_sibling = next_sibling;
            }
        }
        if let Some(next_id) = next_sibling {
            if let Some(next) = self.get_mut(next_id) {
                next.prev_sibling = prev_sibling;
            }
        }

        // Remove from parent's children
        {
            let parent = self.get_mut(parent_id).ok_or(DomError::NodeNotFound(parent_id.0))?;
            parent.children.retain(|id| *id != child_id);
        }

        // Clear child's parent and sibling links
        {
            let child = self.get_mut(child_id).ok_or(DomError::NodeNotFound(child_id.0))?;
            child.parent = None;
            child.prev_sibling = None;
            child.next_sibling = None;
        }

        Ok(())
    }

    /// Get all children of a node
    pub fn children(&self, id: NodeId) -> Vec<NodeId> {
        self.get(id)
            .map(|n| n.children.to_vec())
            .unwrap_or_default()
    }

    /// Iterate over all descendants of a node (depth-first)
    pub fn descendants(&self, id: NodeId) -> Vec<NodeId> {
        let mut result = Vec::new();
        self.collect_descendants(id, &mut result);
        result
    }

    fn collect_descendants(&self, id: NodeId, result: &mut Vec<NodeId>) {
        if let Some(node) = self.get(id) {
            for &child_id in &node.children {
                result.push(child_id);
                self.collect_descendants(child_id, result);
            }
        }
    }

    /// Get the text content of a node and all its descendants
    pub fn text_content(&self, id: NodeId) -> String {
        let mut result = String::new();
        self.collect_text(id, &mut result);
        result
    }

    fn collect_text(&self, id: NodeId, result: &mut String) {
        if let Some(node) = self.get(id) {
            match &node.node_type {
                NodeType::Text(text) => result.push_str(text),
                _ => {
                    for &child_id in &node.children {
                        self.collect_text(child_id, result);
                    }
                }
            }
        }
    }

    /// Get the number of nodes in the tree
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if the tree is empty (only has document node)
    pub fn is_empty(&self) -> bool {
        self.nodes.len() <= 1
    }

    /// Pretty print the tree for debugging
    pub fn pretty_print(&self) -> String {
        let mut output = String::new();
        self.print_node(self.document_id, 0, &mut output);
        output
    }

    fn print_node(&self, id: NodeId, depth: usize, output: &mut String) {
        let indent = "  ".repeat(depth);

        if let Some(node) = self.get(id) {
            match &node.node_type {
                NodeType::Document => {
                    output.push_str("#document\n");
                }
                NodeType::Doctype { name, .. } => {
                    output.push_str(&format!("{}<!DOCTYPE {}>\n", indent, name));
                }
                NodeType::Element(elem) => {
                    let attrs: Vec<String> = elem
                        .attributes
                        .iter()
                        .map(|(k, v)| format!("{}=\"{}\"", k, v))
                        .collect();
                    let attrs_str = if attrs.is_empty() {
                        String::new()
                    } else {
                        format!(" {}", attrs.join(" "))
                    };
                    output.push_str(&format!("{}<{}{}>\n", indent, elem.tag_name, attrs_str));
                }
                NodeType::Text(text) => {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        output.push_str(&format!("{}#text: {:?}\n", indent, trimmed));
                    }
                }
                NodeType::Comment(text) => {
                    output.push_str(&format!("{}<!-- {} -->\n", indent, text));
                }
            }

            for &child_id in &node.children {
                self.print_node(child_id, depth + 1, output);
            }
        }
    }
}

impl Default for DomTree {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for DomTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.pretty_print())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_elements() {
        let mut tree = DomTree::new();
        let html = tree.create_element("html");
        let body = tree.create_element("body");
        let text = tree.create_text("Hello, World!");

        tree.append_child(tree.document_id(), html).unwrap();
        tree.append_child(html, body).unwrap();
        tree.append_child(body, text).unwrap();

        assert_eq!(tree.len(), 4); // document + html + body + text
        assert_eq!(tree.text_content(body), "Hello, World!");
    }
}

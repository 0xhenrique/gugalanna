//! HTML Tree Builder
//!
//! Constructs a DOM tree from HTML tokens.

use gugalanna_dom::{DomTree, NodeId, NodeType};

use crate::tokenizer::{Token, Tokenizer};
use crate::error::HtmlResult;

/// Stack of open elements
type OpenElements = Vec<NodeId>;

/// HTML parser that builds a DOM tree
pub struct HtmlParser {
    tree: DomTree,
    open_elements: OpenElements,
    head_element: Option<NodeId>,
    form_element: Option<NodeId>,
}

impl HtmlParser {
    /// Create a new HTML parser
    pub fn new() -> Self {
        Self {
            tree: DomTree::new(),
            open_elements: Vec::new(),
            head_element: None,
            form_element: None,
        }
    }

    /// Parse HTML string into a DOM tree
    pub fn parse(mut self, html: &str) -> HtmlResult<DomTree> {
        let mut tokenizer = Tokenizer::new(html);

        loop {
            let token = tokenizer.next_token()?;
            if token == Token::Eof {
                break;
            }
            self.process_token(token)?;
        }

        Ok(self.tree)
    }

    /// Process a single token
    fn process_token(&mut self, token: Token) -> HtmlResult<()> {
        match token {
            Token::Doctype { name, public_id, system_id, .. } => {
                let doctype = self.tree.create_doctype(
                    name,
                    public_id.unwrap_or_default(),
                    system_id.unwrap_or_default(),
                );
                self.tree.append_child(self.tree.document_id(), doctype).ok();
            }

            Token::StartTag { name, attributes, self_closing } => {
                self.handle_start_tag(&name, attributes, self_closing)?;
            }

            Token::EndTag { name } => {
                self.handle_end_tag(&name)?;
            }

            Token::Character(c) => {
                self.handle_character(c)?;
            }

            Token::Comment(text) => {
                let comment = self.tree.create_comment(text);
                let parent = self.current_node();
                self.tree.append_child(parent, comment).ok();
            }

            Token::Eof => {}
        }
        Ok(())
    }

    /// Handle a start tag
    fn handle_start_tag(
        &mut self,
        name: &str,
        attributes: smallvec::SmallVec<[(String, String); 4]>,
        self_closing: bool,
    ) -> HtmlResult<()> {
        // Create the element
        let element_id = self.tree.create_element(name);

        // Set attributes
        if let Some(elem) = self.tree.get_mut(element_id).and_then(|n| n.as_element_mut()) {
            for (key, value) in attributes {
                elem.set_attribute(key, value);
            }
        }

        // Handle implicit elements
        self.ensure_implicit_elements(name);

        // Append to current parent
        let parent = self.current_node();
        self.tree.append_child(parent, element_id).ok();

        // Track special elements
        match name {
            "head" => self.head_element = Some(element_id),
            "form" => self.form_element = Some(element_id),
            _ => {}
        }

        // Push to open elements (unless self-closing or void element)
        if !self_closing && !is_void_element(name) {
            self.open_elements.push(element_id);
        }

        Ok(())
    }

    /// Handle an end tag
    fn handle_end_tag(&mut self, name: &str) -> HtmlResult<()> {
        // Find matching open element
        for i in (0..self.open_elements.len()).rev() {
            let element_id = self.open_elements[i];
            if let Some(node) = self.tree.get(element_id) {
                if let Some(elem) = node.as_element() {
                    if elem.tag_name == name {
                        // Pop all elements up to and including this one
                        self.open_elements.truncate(i);
                        return Ok(());
                    }
                }
            }
        }

        // No matching element found, ignore
        Ok(())
    }

    /// Handle a character token
    fn handle_character(&mut self, c: char) -> HtmlResult<()> {
        let parent = self.current_node();

        // Try to append to existing text node
        if let Some(&last_child_id) = self.tree.get(parent)
            .and_then(|n| n.children.last())
        {
            if let Some(last_child) = self.tree.get_mut(last_child_id) {
                if let NodeType::Text(ref mut text) = last_child.node_type {
                    text.push(c);
                    return Ok(());
                }
            }
        }

        // Create new text node
        let text_id = self.tree.create_text(c.to_string());
        self.tree.append_child(parent, text_id).ok();
        Ok(())
    }

    /// Get the current node (top of stack or document)
    fn current_node(&self) -> NodeId {
        self.open_elements.last().copied().unwrap_or(self.tree.document_id())
    }

    /// Ensure implicit html/head/body elements exist
    fn ensure_implicit_elements(&mut self, incoming_tag: &str) {
        // If no elements and not html, create html first
        if self.open_elements.is_empty() && incoming_tag != "html" {
            let html = self.tree.create_element("html");
            self.tree.append_child(self.tree.document_id(), html).ok();
            self.open_elements.push(html);
        }

        // If only html and head tag not present, handle body-start tags
        if self.open_elements.len() == 1 {
            let top_tag = self.tree.get(self.open_elements[0])
                .and_then(|n| n.as_element())
                .map(|e| e.tag_name.as_str());

            if top_tag == Some("html") {
                if incoming_tag == "body" || is_body_content(incoming_tag) {
                    // Create implicit body if needed
                    if incoming_tag != "body" {
                        let body = self.tree.create_element("body");
                        let html = self.open_elements[0];
                        self.tree.append_child(html, body).ok();
                        self.open_elements.push(body);
                    }
                } else if incoming_tag != "head" && incoming_tag != "html" {
                    // Create implicit head
                    let head = self.tree.create_element("head");
                    let html = self.open_elements[0];
                    self.tree.append_child(html, head).ok();
                    self.head_element = Some(head);
                    // Don't push head to stack - it auto-closes
                }
            }
        }
    }
}

impl Default for HtmlParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if an element is a void element (self-closing)
fn is_void_element(name: &str) -> bool {
    matches!(
        name,
        "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input"
        | "link" | "meta" | "param" | "source" | "track" | "wbr"
    )
}

/// Check if a tag belongs in body (not head)
fn is_body_content(name: &str) -> bool {
    !matches!(
        name,
        "base" | "basefont" | "bgsound" | "link" | "meta" | "noframes"
        | "script" | "style" | "template" | "title"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use gugalanna_dom::Queryable;

    #[test]
    fn test_parse_simple() {
        let html = r#"<!DOCTYPE html>
<html>
<head><title>Test</title></head>
<body><p>Hello</p></body>
</html>"#;

        let parser = HtmlParser::new();
        let tree = parser.parse(html).unwrap();

        // Should have html element
        let html_nodes = tree.get_elements_by_tag_name("html");
        assert_eq!(html_nodes.len(), 1);

        // Should have body with p
        let p_nodes = tree.get_elements_by_tag_name("p");
        assert_eq!(p_nodes.len(), 1);
    }

    #[test]
    fn test_parse_with_attributes() {
        let html = r#"<div id="main" class="container">Content</div>"#;

        let parser = HtmlParser::new();
        let tree = parser.parse(html).unwrap();

        let main = tree.get_element_by_id("main");
        assert!(main.is_some());

        let containers = tree.get_elements_by_class_name("container");
        assert_eq!(containers.len(), 1);
    }
}

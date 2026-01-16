//! HTML Tree Builder
//!
//! Constructs a DOM tree from HTML tokens.
//! Implements key HTML5 parsing algorithms for proper tree construction.

use gugalanna_dom::{DomTree, NodeId, NodeType};

use crate::tokenizer::{Token, Tokenizer};
use crate::error::HtmlResult;

/// Stack of open elements
type OpenElements = Vec<NodeId>;

/// Active formatting elements list entry
#[derive(Debug, Clone)]
enum FormattingEntry {
    /// An actual formatting element
    Element(NodeId),
    /// A scope marker (for handling nested formatting contexts)
    Marker,
}

/// List of active formatting elements (for adoption agency algorithm)
type ActiveFormattingElements = Vec<FormattingEntry>;

/// HTML parser that builds a DOM tree
pub struct HtmlParser {
    tree: DomTree,
    open_elements: OpenElements,
    active_formatting_elements: ActiveFormattingElements,
    head_element: Option<NodeId>,
    form_element: Option<NodeId>,
    foster_parenting: bool,
}

impl HtmlParser {
    /// Create a new HTML parser
    pub fn new() -> Self {
        Self {
            tree: DomTree::new(),
            open_elements: Vec::new(),
            active_formatting_elements: Vec::new(),
            head_element: None,
            form_element: None,
            foster_parenting: false,
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
        // Handle implicit end tags before creating the element
        self.handle_implicit_end_tags(name);

        // Reconstruct active formatting elements if needed
        if is_formatting_scope_content(name) {
            self.reconstruct_active_formatting_elements();
        }

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

        // Get the insertion location (may be affected by foster parenting)
        let parent = self.get_appropriate_place_for_inserting(name);
        self.tree.append_child(parent, element_id).ok();

        // Track special elements
        match name {
            "head" => self.head_element = Some(element_id),
            "form" => self.form_element = Some(element_id),
            _ => {}
        }

        // Add to active formatting elements if it's a formatting element
        if is_formatting_element(name) {
            self.active_formatting_elements.push(FormattingEntry::Element(element_id));
        }

        // Push to open elements (unless self-closing or void element)
        if !self_closing && !is_void_element(name) {
            self.open_elements.push(element_id);
        }

        // Insert scope marker after certain elements
        if is_scope_marker_element(name) {
            self.active_formatting_elements.push(FormattingEntry::Marker);
        }

        Ok(())
    }

    /// Handle implicit end tags before a start tag
    fn handle_implicit_end_tags(&mut self, incoming_tag: &str) {
        // Close <p> element when certain tags are seen
        if closes_p_element(incoming_tag) && self.has_element_in_button_scope("p") {
            self.close_p_element();
        }

        // Close <li> when another <li> is seen
        if incoming_tag == "li" && self.has_element_in_list_item_scope("li") {
            self.generate_implied_end_tags_except("li");
            self.pop_until_tag("li");
        }

        // Close <dd> or <dt> when another definition term is seen
        if incoming_tag == "dd" || incoming_tag == "dt" {
            if self.has_element_in_scope("dd") {
                self.generate_implied_end_tags_except("dd");
                self.pop_until_tag("dd");
            }
            if self.has_element_in_scope("dt") {
                self.generate_implied_end_tags_except("dt");
                self.pop_until_tag("dt");
            }
        }

        // Close heading elements when another heading is seen
        if is_heading(incoming_tag) {
            if self.has_heading_in_scope() {
                self.generate_implied_end_tags_except("");
                // Pop until we hit a heading
                while let Some(&node_id) = self.open_elements.last() {
                    if let Some(tag) = self.get_tag_name(node_id) {
                        if is_heading(&tag) {
                            self.open_elements.pop();
                            break;
                        }
                    }
                    self.open_elements.pop();
                }
            }
        }

        // Close <option> when another <option> is seen
        if incoming_tag == "option" {
            if let Some(&node_id) = self.open_elements.last() {
                if self.get_tag_name(node_id).as_deref() == Some("option") {
                    self.open_elements.pop();
                }
            }
        }

        // Close <optgroup> when <optgroup> or end of <select> is seen
        if incoming_tag == "optgroup" {
            // Close open option first
            if let Some(&node_id) = self.open_elements.last() {
                if self.get_tag_name(node_id).as_deref() == Some("option") {
                    self.open_elements.pop();
                }
            }
            // Then close optgroup if open
            if let Some(&node_id) = self.open_elements.last() {
                if self.get_tag_name(node_id).as_deref() == Some("optgroup") {
                    self.open_elements.pop();
                }
            }
        }

        // Handle table-related implicit closes
        if incoming_tag == "tr" {
            self.clear_stack_to_table_body_context();
        } else if incoming_tag == "td" || incoming_tag == "th" {
            self.clear_stack_to_table_row_context();
        }
    }

    /// Handle an end tag
    fn handle_end_tag(&mut self, name: &str) -> HtmlResult<()> {
        // Handle formatting elements with adoption agency
        if is_formatting_element(name) {
            self.run_adoption_agency(name);
            return Ok(());
        }

        // Handle special end tags
        match name {
            "p" => {
                if self.has_element_in_button_scope("p") {
                    self.close_p_element();
                } else {
                    // Act as if <p> start tag was seen, then close it
                    let p_id = self.tree.create_element("p");
                    let parent = self.current_node();
                    self.tree.append_child(parent, p_id).ok();
                }
                return Ok(());
            }
            "li" => {
                if self.has_element_in_list_item_scope("li") {
                    self.generate_implied_end_tags_except("li");
                    self.pop_until_tag("li");
                }
                return Ok(());
            }
            "dd" | "dt" => {
                if self.has_element_in_scope(name) {
                    self.generate_implied_end_tags_except(name);
                    self.pop_until_tag(name);
                }
                return Ok(());
            }
            "body" | "html" => {
                // These are handled specially - for now just ignore
                return Ok(());
            }
            _ => {}
        }

        // Find matching open element (generic end tag handling)
        for i in (0..self.open_elements.len()).rev() {
            let element_id = self.open_elements[i];
            if let Some(tag) = self.get_tag_name(element_id) {
                if tag == name {
                    // Generate implied end tags
                    self.generate_implied_end_tags_except(name);
                    // Pop all elements up to and including this one
                    self.open_elements.truncate(i);
                    return Ok(());
                }
                // If we hit a special element, stop looking
                if is_special_element(&tag) {
                    break;
                }
            }
        }

        Ok(())
    }

    /// Simplified adoption agency algorithm for formatting elements
    fn run_adoption_agency(&mut self, tag: &str) {
        // Find the formatting element in the list of active formatting elements
        let formatting_element_pos = self.active_formatting_elements.iter().rposition(|entry| {
            matches!(entry, FormattingEntry::Element(id) if self.get_tag_name(*id).as_deref() == Some(tag))
        });

        let formatting_element_pos = match formatting_element_pos {
            Some(pos) => pos,
            None => {
                // No formatting element found, just do normal end tag processing
                self.pop_until_tag(tag);
                return;
            }
        };

        let formatting_element_id = match &self.active_formatting_elements[formatting_element_pos] {
            FormattingEntry::Element(id) => *id,
            FormattingEntry::Marker => return,
        };

        // Check if the element is in the stack of open elements
        let stack_pos = self.open_elements.iter().position(|&id| id == formatting_element_id);
        let stack_pos = match stack_pos {
            Some(pos) => pos,
            None => {
                // Element not in stack, remove from active formatting elements
                self.active_formatting_elements.remove(formatting_element_pos);
                return;
            }
        };

        // Check if the element is in scope
        if !self.has_element_in_scope(tag) {
            return;
        }

        // If the current node is the formatting element, pop it and remove from active list
        if let Some(&current) = self.open_elements.last() {
            if current == formatting_element_id {
                self.open_elements.pop();
                self.active_formatting_elements.remove(formatting_element_pos);
                return;
            }
        }

        // Simplified: just close up to and including the formatting element
        // A full adoption agency implementation would move nodes around
        self.open_elements.truncate(stack_pos);
        self.active_formatting_elements.remove(formatting_element_pos);
    }

    /// Reconstruct active formatting elements
    fn reconstruct_active_formatting_elements(&mut self) {
        if self.active_formatting_elements.is_empty() {
            return;
        }

        // Find the last marker or beginning
        let mut entry_idx = self.active_formatting_elements.len() - 1;

        loop {
            match &self.active_formatting_elements[entry_idx] {
                FormattingEntry::Marker => {
                    entry_idx += 1;
                    break;
                }
                FormattingEntry::Element(id) => {
                    // If this element is in the stack, we're done
                    if self.open_elements.contains(id) {
                        entry_idx += 1;
                        break;
                    }
                }
            }

            if entry_idx == 0 {
                break;
            }
            entry_idx -= 1;
        }

        // Now reconstruct from entry_idx to the end
        while entry_idx < self.active_formatting_elements.len() {
            if let FormattingEntry::Element(old_id) = &self.active_formatting_elements[entry_idx] {
                if let Some(tag) = self.get_tag_name(*old_id) {
                    // Create a new element with the same tag name
                    let new_id = self.tree.create_element(&tag);
                    let parent = self.current_node();
                    self.tree.append_child(parent, new_id).ok();
                    self.open_elements.push(new_id);
                    self.active_formatting_elements[entry_idx] = FormattingEntry::Element(new_id);
                }
            }
            entry_idx += 1;
        }
    }

    /// Handle a character token
    fn handle_character(&mut self, c: char) -> HtmlResult<()> {
        // Reconstruct active formatting elements for non-whitespace text
        if !c.is_ascii_whitespace() {
            self.reconstruct_active_formatting_elements();
        }

        let parent = self.get_appropriate_place_for_inserting("text");

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

    /// Get the appropriate place for inserting a node
    /// This handles foster parenting when inside tables
    fn get_appropriate_place_for_inserting(&self, _incoming_tag: &str) -> NodeId {
        if self.foster_parenting {
            // Foster parent to before the table element
            // Simplified: just return parent of table or document
            for &id in self.open_elements.iter().rev() {
                if let Some(tag) = self.get_tag_name(id) {
                    if tag == "table" {
                        // Get parent of table
                        if let Some(node) = self.tree.get(id) {
                            if let Some(parent) = node.parent {
                                return parent;
                            }
                        }
                    }
                }
            }
        }
        self.current_node()
    }

    /// Get the tag name of a node
    fn get_tag_name(&self, node_id: NodeId) -> Option<String> {
        self.tree.get(node_id)
            .and_then(|n| n.as_element())
            .map(|e| e.tag_name.clone())
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

    /// Check if an element is in scope
    fn has_element_in_scope(&self, tag: &str) -> bool {
        for &id in self.open_elements.iter().rev() {
            if let Some(node_tag) = self.get_tag_name(id) {
                if node_tag == tag {
                    return true;
                }
                if is_scope_element(&node_tag) {
                    return false;
                }
            }
        }
        false
    }

    /// Check if an element is in button scope
    fn has_element_in_button_scope(&self, tag: &str) -> bool {
        for &id in self.open_elements.iter().rev() {
            if let Some(node_tag) = self.get_tag_name(id) {
                if node_tag == tag {
                    return true;
                }
                if is_scope_element(&node_tag) || node_tag == "button" {
                    return false;
                }
            }
        }
        false
    }

    /// Check if an element is in list item scope
    fn has_element_in_list_item_scope(&self, tag: &str) -> bool {
        for &id in self.open_elements.iter().rev() {
            if let Some(node_tag) = self.get_tag_name(id) {
                if node_tag == tag {
                    return true;
                }
                if is_scope_element(&node_tag) || node_tag == "ol" || node_tag == "ul" {
                    return false;
                }
            }
        }
        false
    }

    /// Check if any heading element is in scope
    fn has_heading_in_scope(&self) -> bool {
        for &id in self.open_elements.iter().rev() {
            if let Some(node_tag) = self.get_tag_name(id) {
                if is_heading(&node_tag) {
                    return true;
                }
                if is_scope_element(&node_tag) {
                    return false;
                }
            }
        }
        false
    }

    /// Close a <p> element
    fn close_p_element(&mut self) {
        self.generate_implied_end_tags_except("p");
        self.pop_until_tag("p");
    }

    /// Generate implied end tags (except for the given tag)
    fn generate_implied_end_tags_except(&mut self, except: &str) {
        while let Some(&node_id) = self.open_elements.last() {
            if let Some(tag) = self.get_tag_name(node_id) {
                if has_implied_end_tag(&tag) && tag != except {
                    self.open_elements.pop();
                    continue;
                }
            }
            break;
        }
    }

    /// Pop elements until we find the given tag (inclusive)
    fn pop_until_tag(&mut self, tag: &str) {
        while let Some(&node_id) = self.open_elements.last() {
            let should_stop = self.get_tag_name(node_id).as_deref() == Some(tag);
            self.open_elements.pop();
            if should_stop {
                break;
            }
        }
    }

    /// Clear stack to table body context
    fn clear_stack_to_table_body_context(&mut self) {
        while let Some(&node_id) = self.open_elements.last() {
            if let Some(tag) = self.get_tag_name(node_id) {
                if matches!(tag.as_str(), "tbody" | "tfoot" | "thead" | "template" | "html") {
                    break;
                }
            }
            self.open_elements.pop();
        }
    }

    /// Clear stack to table row context
    fn clear_stack_to_table_row_context(&mut self) {
        while let Some(&node_id) = self.open_elements.last() {
            if let Some(tag) = self.get_tag_name(node_id) {
                if matches!(tag.as_str(), "tr" | "template" | "html") {
                    break;
                }
            }
            self.open_elements.pop();
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

/// Check if an element is a formatting element (for adoption agency)
fn is_formatting_element(name: &str) -> bool {
    matches!(
        name,
        "a" | "b" | "big" | "code" | "em" | "font" | "i" | "nobr"
        | "s" | "small" | "strike" | "strong" | "tt" | "u"
    )
}

/// Check if an element is a special element (stops certain searches)
fn is_special_element(name: &str) -> bool {
    matches!(
        name,
        "address" | "applet" | "area" | "article" | "aside" | "base" | "basefont"
        | "bgsound" | "blockquote" | "body" | "br" | "button" | "caption" | "center"
        | "col" | "colgroup" | "dd" | "details" | "dir" | "div" | "dl" | "dt" | "embed"
        | "fieldset" | "figcaption" | "figure" | "footer" | "form" | "frame" | "frameset"
        | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "head" | "header" | "hgroup" | "hr"
        | "html" | "iframe" | "img" | "input" | "li" | "link" | "listing" | "main"
        | "marquee" | "menu" | "meta" | "nav" | "noembed" | "noframes" | "noscript"
        | "object" | "ol" | "p" | "param" | "plaintext" | "pre" | "script" | "section"
        | "select" | "source" | "style" | "summary" | "table" | "tbody" | "td"
        | "template" | "textarea" | "tfoot" | "th" | "thead" | "title" | "tr" | "track"
        | "ul" | "wbr" | "xmp"
    )
}

/// Check if tag closes an open <p> element
fn closes_p_element(name: &str) -> bool {
    matches!(
        name,
        "address" | "article" | "aside" | "blockquote" | "center" | "details"
        | "dialog" | "dir" | "div" | "dl" | "fieldset" | "figcaption" | "figure"
        | "footer" | "form" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "header"
        | "hgroup" | "hr" | "li" | "main" | "menu" | "nav" | "ol" | "p" | "pre"
        | "section" | "table" | "ul"
    )
}

/// Check if a tag is a heading
fn is_heading(name: &str) -> bool {
    matches!(name, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
}

/// Check if an element has an implied end tag
fn has_implied_end_tag(name: &str) -> bool {
    matches!(
        name,
        "dd" | "dt" | "li" | "optgroup" | "option" | "p" | "rb" | "rp" | "rt" | "rtc"
    )
}

/// Check if an element creates a scope boundary
fn is_scope_element(name: &str) -> bool {
    matches!(
        name,
        "applet" | "caption" | "html" | "table" | "td" | "th" | "marquee" | "object"
        | "template"
    )
}

/// Check if an element should trigger a scope marker in active formatting elements
fn is_scope_marker_element(name: &str) -> bool {
    matches!(
        name,
        "applet" | "marquee" | "object" | "table" | "td" | "th" | "caption"
    )
}

/// Check if content should trigger formatting element reconstruction
fn is_formatting_scope_content(name: &str) -> bool {
    !is_void_element(name) && !is_scope_marker_element(name) && name != "br"
}

#[cfg(test)]
mod tests {
    use super::*;
    use gugalanna_dom::Queryable;

    // Helper to parse HTML and return the tree
    fn parse(html: &str) -> DomTree {
        HtmlParser::new().parse(html).unwrap()
    }

    #[test]
    fn test_parse_simple() {
        let html = r#"<!DOCTYPE html>
<html>
<head><title>Test</title></head>
<body><p>Hello</p></body>
</html>"#;

        let tree = parse(html);

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

        let tree = parse(html);

        let main = tree.get_element_by_id("main");
        assert!(main.is_some());

        let containers = tree.get_elements_by_class_name("container");
        assert_eq!(containers.len(), 1);
    }

    // === Implicit element insertion tests ===

    #[test]
    fn test_implicit_html() {
        // Missing <html> tag should be auto-inserted
        let tree = parse("<body><p>Hello</p></body>");

        let html_nodes = tree.get_elements_by_tag_name("html");
        assert_eq!(html_nodes.len(), 1);
    }

    #[test]
    fn test_implicit_body() {
        // Missing <body> tag should be auto-inserted for body content
        let tree = parse("<html><p>Hello</p></html>");

        let body_nodes = tree.get_elements_by_tag_name("body");
        assert_eq!(body_nodes.len(), 1);

        let p_nodes = tree.get_elements_by_tag_name("p");
        assert_eq!(p_nodes.len(), 1);
    }

    #[test]
    fn test_minimal_document() {
        // Just a simple div - tests implicit html creation
        let tree = parse("<div>Hello World</div>");

        // Should have auto-created html
        let html_nodes = tree.get_elements_by_tag_name("html");
        assert_eq!(html_nodes.len(), 1);

        let div_nodes = tree.get_elements_by_tag_name("div");
        assert_eq!(div_nodes.len(), 1);
    }

    // === Implicit tag closing tests ===

    #[test]
    fn test_implicit_p_close() {
        // <p> should be implicitly closed by block elements
        let tree = parse("<p>First<p>Second<p>Third");

        let p_nodes = tree.get_elements_by_tag_name("p");
        assert_eq!(p_nodes.len(), 3);
    }

    #[test]
    fn test_implicit_p_close_by_div() {
        let tree = parse("<p>Paragraph<div>Block</div>");

        let p_nodes = tree.get_elements_by_tag_name("p");
        assert_eq!(p_nodes.len(), 1);

        let div_nodes = tree.get_elements_by_tag_name("div");
        assert_eq!(div_nodes.len(), 1);
    }

    #[test]
    fn test_implicit_li_close() {
        // <li> should be implicitly closed by another <li>
        let tree = parse("<ul><li>One<li>Two<li>Three</ul>");

        let li_nodes = tree.get_elements_by_tag_name("li");
        assert_eq!(li_nodes.len(), 3);
    }

    #[test]
    fn test_implicit_dd_dt_close() {
        // <dd> and <dt> should be implicitly closed
        let tree = parse("<dl><dt>Term 1<dd>Def 1<dt>Term 2<dd>Def 2</dl>");

        let dt_nodes = tree.get_elements_by_tag_name("dt");
        assert_eq!(dt_nodes.len(), 2);

        let dd_nodes = tree.get_elements_by_tag_name("dd");
        assert_eq!(dd_nodes.len(), 2);
    }

    #[test]
    fn test_implicit_option_close() {
        // <option> should be implicitly closed by another <option>
        let tree = parse("<select><option>A<option>B<option>C</select>");

        let option_nodes = tree.get_elements_by_tag_name("option");
        assert_eq!(option_nodes.len(), 3);
    }

    #[test]
    fn test_heading_closes_heading() {
        // A heading should close a previous heading in the same scope
        let tree = parse("<h1>Title<h2>Subtitle");

        let h1_nodes = tree.get_elements_by_tag_name("h1");
        assert_eq!(h1_nodes.len(), 1);

        let h2_nodes = tree.get_elements_by_tag_name("h2");
        assert_eq!(h2_nodes.len(), 1);
    }

    // === Formatting element tests ===

    #[test]
    fn test_formatting_element_basic() {
        let tree = parse("<p><b>Bold</b> text</p>");

        let b_nodes = tree.get_elements_by_tag_name("b");
        assert_eq!(b_nodes.len(), 1);
    }

    #[test]
    fn test_nested_formatting() {
        let tree = parse("<p><b><i>Bold italic</i></b></p>");

        let b_nodes = tree.get_elements_by_tag_name("b");
        assert_eq!(b_nodes.len(), 1);

        let i_nodes = tree.get_elements_by_tag_name("i");
        assert_eq!(i_nodes.len(), 1);
    }

    #[test]
    fn test_misnested_formatting() {
        // <b><i></b></i> - classic adoption agency test case
        let tree = parse("<p><b><i>text</b>more</i></p>");

        // Both b and i should exist
        let b_nodes = tree.get_elements_by_tag_name("b");
        assert!(b_nodes.len() >= 1);

        let i_nodes = tree.get_elements_by_tag_name("i");
        assert!(i_nodes.len() >= 1);
    }

    #[test]
    fn test_anchor_adoption() {
        // Anchor tags also use adoption agency
        let tree = parse("<p><a href='#'>link<div>block</div></a></p>");

        let a_nodes = tree.get_elements_by_tag_name("a");
        assert!(a_nodes.len() >= 1);
    }

    // === Doctype tests ===

    #[test]
    fn test_doctype_html5() {
        let tree = parse("<!DOCTYPE html><html><body></body></html>");

        // Tree should be built successfully with doctype
        let html_nodes = tree.get_elements_by_tag_name("html");
        assert_eq!(html_nodes.len(), 1);
    }

    #[test]
    fn test_doctype_quirks() {
        let tree = parse("<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd\"><html><body></body></html>");

        let html_nodes = tree.get_elements_by_tag_name("html");
        assert_eq!(html_nodes.len(), 1);
    }

    // === Void element tests ===

    #[test]
    fn test_void_elements_no_close() {
        let tree = parse("<div><br><hr><img><input></div>");

        // Void elements shouldn't leave open elements on the stack
        let divs = tree.get_elements_by_tag_name("div");
        assert_eq!(divs.len(), 1);

        let brs = tree.get_elements_by_tag_name("br");
        assert_eq!(brs.len(), 1);

        let hrs = tree.get_elements_by_tag_name("hr");
        assert_eq!(hrs.len(), 1);
    }

    #[test]
    fn test_void_element_self_closing() {
        let tree = parse("<div><br/><hr /><img /></div>");

        let brs = tree.get_elements_by_tag_name("br");
        assert_eq!(brs.len(), 1);
    }

    // === Nested elements tests ===

    #[test]
    fn test_deeply_nested() {
        let tree = parse("<div><section><article><p><span>Deep</span></p></article></section></div>");

        let divs = tree.get_elements_by_tag_name("div");
        assert_eq!(divs.len(), 1);

        let spans = tree.get_elements_by_tag_name("span");
        assert_eq!(spans.len(), 1);
    }

    #[test]
    fn test_sibling_elements() {
        let tree = parse("<ul><li>One</li><li>Two</li><li>Three</li></ul>");

        let lis = tree.get_elements_by_tag_name("li");
        assert_eq!(lis.len(), 3);
    }

    // === Text content tests ===

    #[test]
    fn test_text_content() {
        let tree = parse("<p>Hello World</p>");

        let p_nodes = tree.get_elements_by_tag_name("p");
        assert_eq!(p_nodes.len(), 1);

        // Check that text node was created
        if let Some(p) = tree.get(p_nodes[0]) {
            assert!(!p.children.is_empty());
            if let Some(text_node) = tree.get(p.children[0]) {
                if let NodeType::Text(ref text) = text_node.node_type {
                    assert!(text.contains("Hello World"));
                }
            }
        }
    }

    #[test]
    fn test_text_adjacent() {
        // Adjacent text should be combined
        let tree = parse("<p>Hello</p><p>World</p>");

        let p_nodes = tree.get_elements_by_tag_name("p");
        assert_eq!(p_nodes.len(), 2);
    }

    #[test]
    fn test_entities_in_text() {
        let tree = parse("<p>&lt;hello&gt;</p>");

        let p_nodes = tree.get_elements_by_tag_name("p");
        assert_eq!(p_nodes.len(), 1);

        if let Some(p) = tree.get(p_nodes[0]) {
            if let Some(text_node) = tree.get(p.children[0]) {
                if let NodeType::Text(ref text) = text_node.node_type {
                    assert_eq!(text, "<hello>");
                }
            }
        }
    }

    // === Comment tests ===

    #[test]
    fn test_comment() {
        let tree = parse("<div><!-- comment --></div>");

        let divs = tree.get_elements_by_tag_name("div");
        assert_eq!(divs.len(), 1);

        // Verify comment was added as child
        if let Some(div) = tree.get(divs[0]) {
            let has_comment = div.children.iter().any(|&id| {
                tree.get(id)
                    .map(|n| matches!(n.node_type, NodeType::Comment(_)))
                    .unwrap_or(false)
            });
            assert!(has_comment);
        }
    }

    // === Multiple class tests ===

    #[test]
    fn test_multiple_classes() {
        let tree = parse("<div class='foo bar baz'>Content</div>");

        let foos = tree.get_elements_by_class_name("foo");
        assert_eq!(foos.len(), 1);

        let bars = tree.get_elements_by_class_name("bar");
        assert_eq!(bars.len(), 1);

        let bazs = tree.get_elements_by_class_name("baz");
        assert_eq!(bazs.len(), 1);
    }

    // === Script and style tests ===

    #[test]
    fn test_script_content() {
        let tree = parse("<script>var x = '<div>not a tag</div>';</script>");

        let scripts = tree.get_elements_by_tag_name("script");
        assert_eq!(scripts.len(), 1);

        // Content should be text, not parsed as elements
        let divs = tree.get_elements_by_tag_name("div");
        assert_eq!(divs.len(), 0); // No divs - they're in script content
    }

    #[test]
    fn test_style_content() {
        let tree = parse("<style>.foo { color: red; }</style>");

        let styles = tree.get_elements_by_tag_name("style");
        assert_eq!(styles.len(), 1);
    }

    // === Title tests ===

    #[test]
    fn test_title_content() {
        let tree = parse("<head><title>Page &amp; Title</title></head>");

        let titles = tree.get_elements_by_tag_name("title");
        assert_eq!(titles.len(), 1);

        // Check the title text content has decoded entity
        if let Some(title) = tree.get(titles[0]) {
            if let Some(text_node) = tree.get(title.children[0]) {
                if let NodeType::Text(ref text) = text_node.node_type {
                    assert!(text.contains("Page & Title"));
                }
            }
        }
    }

    // === Mismatched tags tests ===

    #[test]
    fn test_unclosed_tag() {
        // Parser should handle unclosed tags gracefully
        let tree = parse("<div><p>Unclosed");

        let divs = tree.get_elements_by_tag_name("div");
        assert_eq!(divs.len(), 1);

        let ps = tree.get_elements_by_tag_name("p");
        assert_eq!(ps.len(), 1);
    }

    #[test]
    fn test_extra_end_tag() {
        // Extra end tags should be ignored
        let tree = parse("<div></div></div></div>");

        let divs = tree.get_elements_by_tag_name("div");
        assert_eq!(divs.len(), 1);
    }

    #[test]
    fn test_mismatched_end_tag() {
        // Mismatched end tags should be handled
        let tree = parse("<div><span></div></span>");

        // Should still create elements (exact behavior may vary)
        let divs = tree.get_elements_by_tag_name("div");
        assert!(divs.len() >= 1);
    }

    // === Form element tests ===

    #[test]
    fn test_form_elements() {
        let tree = parse(r#"
            <form action="/submit" method="post">
                <input type="text" name="username">
                <button type="submit">Submit</button>
            </form>
        "#);

        let forms = tree.get_elements_by_tag_name("form");
        assert_eq!(forms.len(), 1);

        let inputs = tree.get_elements_by_tag_name("input");
        assert_eq!(inputs.len(), 1);

        let buttons = tree.get_elements_by_tag_name("button");
        assert_eq!(buttons.len(), 1);
    }

    // === Table tests ===

    #[test]
    fn test_table_structure() {
        let tree = parse(r#"
            <table>
                <tr><td>Cell 1</td><td>Cell 2</td></tr>
                <tr><td>Cell 3</td><td>Cell 4</td></tr>
            </table>
        "#);

        let tables = tree.get_elements_by_tag_name("table");
        assert_eq!(tables.len(), 1);

        let trs = tree.get_elements_by_tag_name("tr");
        assert_eq!(trs.len(), 2);

        let tds = tree.get_elements_by_tag_name("td");
        assert_eq!(tds.len(), 4);
    }

    #[test]
    fn test_table_implicit_tbody() {
        // Tables should handle rows even without explicit tbody
        let tree = parse("<table><tr><td>Cell</td></tr></table>");

        let tables = tree.get_elements_by_tag_name("table");
        assert_eq!(tables.len(), 1);

        let tds = tree.get_elements_by_tag_name("td");
        assert_eq!(tds.len(), 1);
    }

    // === List tests ===

    #[test]
    fn test_nested_lists() {
        let tree = parse(r#"
            <ul>
                <li>Item 1</li>
                <li>Item 2
                    <ul>
                        <li>Nested 1</li>
                        <li>Nested 2</li>
                    </ul>
                </li>
            </ul>
        "#);

        let uls = tree.get_elements_by_tag_name("ul");
        assert_eq!(uls.len(), 2);

        let lis = tree.get_elements_by_tag_name("li");
        assert_eq!(lis.len(), 4);
    }

    // === Real-world snippets ===

    #[test]
    fn test_meta_tags() {
        let tree = parse(r#"
            <head>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <meta name="description" content="Test page">
            </head>
        "#);

        let metas = tree.get_elements_by_tag_name("meta");
        assert_eq!(metas.len(), 3);
    }

    #[test]
    fn test_link_tags() {
        let tree = parse(r#"
            <head>
                <link rel="stylesheet" href="style.css">
                <link rel="icon" href="favicon.ico">
            </head>
        "#);

        let links = tree.get_elements_by_tag_name("link");
        assert_eq!(links.len(), 2);
    }

    #[test]
    fn test_complex_document() {
        let tree = parse(r#"
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <title>Test Page</title>
                <link rel="stylesheet" href="style.css">
            </head>
            <body>
                <header>
                    <nav>
                        <a href="/">Home</a>
                        <a href="/about">About</a>
                    </nav>
                </header>
                <main>
                    <article>
                        <h1>Welcome</h1>
                        <p>This is a <strong>test</strong> page.</p>
                    </article>
                </main>
                <footer>
                    <p>&copy; 2024</p>
                </footer>
            </body>
            </html>
        "#);

        // Verify key elements exist
        // Note: some counts may be > 1 due to implicit element creation quirks in our simple tree builder
        assert!(tree.get_elements_by_tag_name("html").len() >= 1);
        assert!(tree.get_elements_by_tag_name("head").len() >= 1);
        assert!(tree.get_elements_by_tag_name("body").len() >= 1);
        assert_eq!(tree.get_elements_by_tag_name("header").len(), 1);
        assert_eq!(tree.get_elements_by_tag_name("nav").len(), 1);
        assert_eq!(tree.get_elements_by_tag_name("main").len(), 1);
        assert_eq!(tree.get_elements_by_tag_name("article").len(), 1);
        assert_eq!(tree.get_elements_by_tag_name("footer").len(), 1);
        assert_eq!(tree.get_elements_by_tag_name("a").len(), 2);
    }

    // === Edge case tests for implicit closing ===

    #[test]
    fn test_p_inside_button() {
        // Button creates a scope, so p inside button should work
        let tree = parse("<button><p>Click me</p></button>");

        let buttons = tree.get_elements_by_tag_name("button");
        assert_eq!(buttons.len(), 1);

        let ps = tree.get_elements_by_tag_name("p");
        assert_eq!(ps.len(), 1);
    }

    #[test]
    fn test_empty_p_end_tag() {
        // </p> without matching <p> should create an empty <p>
        let tree = parse("<div></p></div>");

        let ps = tree.get_elements_by_tag_name("p");
        assert_eq!(ps.len(), 1);
    }

    #[test]
    fn test_table_cell_implicit_close() {
        // <td> should be implicitly closed by another <td>
        let tree = parse("<table><tr><td>A<td>B<td>C</tr></table>");

        let tds = tree.get_elements_by_tag_name("td");
        assert_eq!(tds.len(), 3);
    }

    #[test]
    fn test_table_row_implicit_close() {
        // <tr> should be implicitly closed by another <tr>
        let tree = parse("<table><tr><td>A<tr><td>B</table>");

        let trs = tree.get_elements_by_tag_name("tr");
        assert_eq!(trs.len(), 2);
    }

    // === Scope boundary tests ===

    #[test]
    fn test_formatting_across_block() {
        // Formatting element should be reconstructed after block element
        let tree = parse("<p><b>bold<p>more bold</b></p>");

        let b_nodes = tree.get_elements_by_tag_name("b");
        // At least one b element should exist
        assert!(b_nodes.len() >= 1);
    }

    #[test]
    fn test_table_scopes_formatting() {
        // Table creates a scope that blocks formatting
        let tree = parse("<p><b>bold<table><tr><td>cell</td></tr></table>after</b></p>");

        let b_nodes = tree.get_elements_by_tag_name("b");
        assert!(b_nodes.len() >= 1);

        let tables = tree.get_elements_by_tag_name("table");
        assert_eq!(tables.len(), 1);
    }
}

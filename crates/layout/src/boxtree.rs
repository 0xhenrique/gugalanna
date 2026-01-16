//! Box Tree Generation
//!
//! Builds a layout box tree from the style tree.

use gugalanna_dom::{DomTree, NodeId};
use gugalanna_style::{ComputedStyle, Display, StyleTree};

use crate::{Dimensions, EdgeSizes};

/// A layout box in the box tree
#[derive(Debug)]
pub struct LayoutBox<'a> {
    /// Box dimensions (computed during layout)
    pub dimensions: Dimensions,
    /// Type of box
    pub box_type: BoxType<'a>,
    /// Child boxes
    pub children: Vec<LayoutBox<'a>>,
}

/// Type of form input element for layout purposes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputType {
    /// Text input field
    Text,
    /// Password input field (displays dots)
    Password,
    /// Checkbox
    Checkbox,
    /// Radio button
    Radio,
    /// Submit button
    Submit,
    /// Generic button
    Button,
    /// Hidden input (no visual representation)
    Hidden,
}

impl InputType {
    /// Parse input type from HTML type attribute
    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "password" => InputType::Password,
            "checkbox" => InputType::Checkbox,
            "radio" => InputType::Radio,
            "submit" => InputType::Submit,
            "button" => InputType::Button,
            "hidden" => InputType::Hidden,
            _ => InputType::Text, // Default to text
        }
    }

    /// Check if this input type is visually rendered
    pub fn is_visible(&self) -> bool {
        !matches!(self, InputType::Hidden)
    }
}

/// Data for an image element
#[derive(Debug, Clone)]
pub struct ImageData {
    /// Image source URL
    pub src: String,
    /// Intrinsic width from decoded image or HTML attribute
    pub intrinsic_width: Option<f32>,
    /// Intrinsic height from decoded image or HTML attribute
    pub intrinsic_height: Option<f32>,
    /// Alt text for accessibility and placeholder display
    pub alt: String,
    /// Decoded RGBA pixel data (None if not yet loaded or failed)
    pub pixels: Option<ImagePixels>,
}

/// Decoded image pixel data
#[derive(Debug, Clone)]
pub struct ImagePixels {
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// RGBA pixel data, 4 bytes per pixel
    pub data: Vec<u8>,
}

/// Type of layout box
#[derive(Debug)]
pub enum BoxType<'a> {
    /// Block-level box with associated style
    Block(NodeId, &'a ComputedStyle),
    /// Inline-level box with associated style
    Inline(NodeId, &'a ComputedStyle),
    /// Text content
    Text(NodeId, String, &'a ComputedStyle),
    /// Anonymous block box (wraps inline content in block context)
    AnonymousBlock,
    /// Anonymous inline box
    AnonymousInline,
    /// Form input element (replaced element with intrinsic size)
    Input(NodeId, InputType, &'a ComputedStyle),
    /// Button element
    Button(NodeId, String, &'a ComputedStyle),
    /// Image element (replaced element with intrinsic size)
    Image(NodeId, ImageData, &'a ComputedStyle),
}

impl<'a> LayoutBox<'a> {
    /// Create a new block box
    pub fn new_block(node_id: NodeId, style: &'a ComputedStyle) -> Self {
        Self {
            dimensions: Dimensions::default(),
            box_type: BoxType::Block(node_id, style),
            children: Vec::new(),
        }
    }

    /// Create a new inline box
    pub fn new_inline(node_id: NodeId, style: &'a ComputedStyle) -> Self {
        Self {
            dimensions: Dimensions::default(),
            box_type: BoxType::Inline(node_id, style),
            children: Vec::new(),
        }
    }

    /// Create a new text box
    pub fn new_text(node_id: NodeId, text: String, style: &'a ComputedStyle) -> Self {
        Self {
            dimensions: Dimensions::default(),
            box_type: BoxType::Text(node_id, text, style),
            children: Vec::new(),
        }
    }

    /// Create a new input box (for form inputs)
    pub fn new_input(node_id: NodeId, input_type: InputType, style: &'a ComputedStyle) -> Self {
        Self {
            dimensions: Dimensions::default(),
            box_type: BoxType::Input(node_id, input_type, style),
            children: Vec::new(),
        }
    }

    /// Create a new button box
    pub fn new_button(node_id: NodeId, label: String, style: &'a ComputedStyle) -> Self {
        Self {
            dimensions: Dimensions::default(),
            box_type: BoxType::Button(node_id, label, style),
            children: Vec::new(),
        }
    }

    /// Create a new image box
    pub fn new_image(node_id: NodeId, image_data: ImageData, style: &'a ComputedStyle) -> Self {
        Self {
            dimensions: Dimensions::default(),
            box_type: BoxType::Image(node_id, image_data, style),
            children: Vec::new(),
        }
    }

    /// Create an anonymous block box
    pub fn new_anonymous_block() -> Self {
        Self {
            dimensions: Dimensions::default(),
            box_type: BoxType::AnonymousBlock,
            children: Vec::new(),
        }
    }

    /// Get the style if this box has one
    pub fn style(&self) -> Option<&'a ComputedStyle> {
        match &self.box_type {
            BoxType::Block(_, style) => Some(style),
            BoxType::Inline(_, style) => Some(style),
            BoxType::Text(_, _, style) => Some(style),
            BoxType::Input(_, _, style) => Some(style),
            BoxType::Button(_, _, style) => Some(style),
            BoxType::Image(_, _, style) => Some(style),
            BoxType::AnonymousBlock | BoxType::AnonymousInline => None,
        }
    }

    /// Get the node ID if this box has one
    pub fn node_id(&self) -> Option<NodeId> {
        match &self.box_type {
            BoxType::Block(id, _) => Some(*id),
            BoxType::Inline(id, _) => Some(*id),
            BoxType::Text(id, _, _) => Some(*id),
            BoxType::Input(id, _, _) => Some(*id),
            BoxType::Button(id, _, _) => Some(*id),
            BoxType::Image(id, _, _) => Some(*id),
            BoxType::AnonymousBlock | BoxType::AnonymousInline => None,
        }
    }

    /// Check if this is a block-level box
    pub fn is_block(&self) -> bool {
        matches!(self.box_type, BoxType::Block(_, _) | BoxType::AnonymousBlock)
    }

    /// Check if this is an inline-level box
    pub fn is_inline(&self) -> bool {
        matches!(
            self.box_type,
            BoxType::Inline(_, _) | BoxType::Text(_, _, _) | BoxType::AnonymousInline
                | BoxType::Input(_, _, _) | BoxType::Button(_, _, _) | BoxType::Image(_, _, _)
        )
    }

    /// Get or create an anonymous block for inline children
    fn get_inline_container(&mut self) -> &mut LayoutBox<'a> {
        // If the last child is an anonymous block, use it
        let dominated_by_blocks = self.children.iter().any(|c| c.is_block());

        if dominated_by_blocks {
            // Need anonymous blocks to wrap inline content
            match self.children.last() {
                Some(child) if matches!(child.box_type, BoxType::AnonymousBlock) => {}
                _ => self.children.push(LayoutBox::new_anonymous_block()),
            }
            self.children.last_mut().unwrap()
        } else {
            // All inline, no wrapper needed - return self
            self
        }
    }

    /// Copy edge sizes from computed style
    pub fn apply_style_edges(&mut self) {
        if let Some(style) = self.style() {
            self.dimensions.margin = EdgeSizes {
                top: style.margin_top,
                right: style.margin_right,
                bottom: style.margin_bottom,
                left: style.margin_left,
            };
            self.dimensions.padding = EdgeSizes {
                top: style.padding_top,
                right: style.padding_right,
                bottom: style.padding_bottom,
                left: style.padding_left,
            };
            self.dimensions.border = EdgeSizes {
                top: style.border_top_width,
                right: style.border_right_width,
                bottom: style.border_bottom_width,
                left: style.border_left_width,
            };
        }
    }
}

/// Collapse whitespace in text according to CSS rules
/// - Multiple whitespace characters become a single space
/// - Preserves a single space at start/end if there was any whitespace
fn collapse_whitespace(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }

    // Check for leading/trailing whitespace
    let has_leading_space = text.starts_with(char::is_whitespace);
    let has_trailing_space = text.ends_with(char::is_whitespace);

    // Split on whitespace and rejoin with single spaces
    let words: Vec<&str> = text.split_whitespace().collect();

    if words.is_empty() {
        // All whitespace - collapse to single space
        return " ".to_string();
    }

    let mut result = String::new();

    if has_leading_space {
        result.push(' ');
    }

    result.push_str(&words.join(" "));

    if has_trailing_space {
        result.push(' ');
    }

    result
}

/// Build a layout tree from DOM and style tree
pub fn build_layout_tree<'a>(
    dom: &DomTree,
    style_tree: &'a StyleTree,
    root_id: NodeId,
) -> Option<LayoutBox<'a>> {
    let style = style_tree.get_style(root_id)?;

    // Skip elements with display: none
    if style.display == Display::None {
        return None;
    }

    let mut root = match style.display {
        Display::Block | Display::Flex => LayoutBox::new_block(root_id, style),
        Display::Inline | Display::InlineBlock => LayoutBox::new_inline(root_id, style),
        Display::None => return None,
    };

    // Process children
    build_children(dom, style_tree, root_id, &mut root);

    Some(root)
}

/// Build child boxes for a parent
fn build_children<'a>(
    dom: &DomTree,
    style_tree: &'a StyleTree,
    parent_id: NodeId,
    parent_box: &mut LayoutBox<'a>,
) {
    for child_id in dom.children(parent_id) {
        let node = match dom.get(child_id) {
            Some(n) => n,
            None => continue,
        };

        if node.is_element() {
            // Element node - get its style
            if let Some(child_style) = style_tree.get_style(child_id) {
                if child_style.display == Display::None {
                    continue;
                }

                // Check for form elements first
                if let Some(elem) = node.as_element() {
                    match elem.tag_name.as_str() {
                        "input" => {
                            let input_type_str = elem.get_attribute("type").unwrap_or("text");
                            let input_type = InputType::from_str(input_type_str);

                            // Skip hidden inputs
                            if !input_type.is_visible() {
                                continue;
                            }

                            let child_box = LayoutBox::new_input(child_id, input_type, child_style);
                            let container = parent_box.get_inline_container();
                            container.children.push(child_box);
                            continue;
                        }
                        "button" => {
                            // Get button label from text content
                            let label = get_text_content(dom, child_id);
                            let label = if label.is_empty() { "Button".to_string() } else { label };

                            let child_box = LayoutBox::new_button(child_id, label, child_style);
                            let container = parent_box.get_inline_container();
                            container.children.push(child_box);
                            continue;
                        }
                        "img" => {
                            // Get image attributes
                            let src = elem.get_attribute("src").unwrap_or("").to_string();
                            let alt = elem.get_attribute("alt").unwrap_or("").to_string();
                            let attr_width = elem.get_attribute("width")
                                .and_then(|s| s.parse::<f32>().ok());
                            let attr_height = elem.get_attribute("height")
                                .and_then(|s| s.parse::<f32>().ok());

                            let image_data = ImageData {
                                src,
                                intrinsic_width: attr_width,
                                intrinsic_height: attr_height,
                                alt,
                                pixels: None,
                            };

                            let child_box = LayoutBox::new_image(child_id, image_data, child_style);
                            let container = parent_box.get_inline_container();
                            container.children.push(child_box);
                            continue;
                        }
                        _ => {}
                    }
                }

                let child_box = match child_style.display {
                    Display::Block | Display::Flex => {
                        let mut b = LayoutBox::new_block(child_id, child_style);
                        build_children(dom, style_tree, child_id, &mut b);
                        b
                    }
                    Display::Inline | Display::InlineBlock => {
                        let mut b = LayoutBox::new_inline(child_id, child_style);
                        build_children(dom, style_tree, child_id, &mut b);
                        b
                    }
                    Display::None => continue,
                };

                if child_box.is_block() {
                    parent_box.children.push(child_box);
                } else {
                    // Inline content may need wrapping
                    let container = parent_box.get_inline_container();
                    container.children.push(child_box);
                }
            }
        } else if node.is_text() {
            // Text node - create text box
            if let Some(text) = node.as_text() {
                // Collapse whitespace according to CSS rules:
                // - Multiple whitespace → single space
                // - Preserve leading/trailing space if present (important for inline flow)
                let collapsed = collapse_whitespace(text);
                if !collapsed.is_empty() {
                    // Inherit style from parent element
                    // Walk up to find nearest element with style
                    if let Some(parent_style) = find_parent_style(dom, style_tree, parent_id) {
                        let text_box = LayoutBox::new_text(
                            child_id,
                            collapsed,
                            parent_style,
                        );
                        let container = parent_box.get_inline_container();
                        container.children.push(text_box);
                    }
                }
            }
        }
    }
}

/// Find the style of the nearest ancestor element
fn find_parent_style<'a>(
    dom: &DomTree,
    style_tree: &'a StyleTree,
    node_id: NodeId,
) -> Option<&'a ComputedStyle> {
    // First try the node itself
    if let Some(style) = style_tree.get_style(node_id) {
        return Some(style);
    }

    // Walk up to parent
    if let Some(node) = dom.get(node_id) {
        if let Some(parent_id) = node.parent {
            return find_parent_style(dom, style_tree, parent_id);
        }
    }

    None
}

/// Extract text content from an element and its descendants
fn get_text_content(dom: &DomTree, node_id: NodeId) -> String {
    let mut text = String::new();
    get_text_content_recursive(dom, node_id, &mut text);
    collapse_whitespace(&text)
}

fn get_text_content_recursive(dom: &DomTree, node_id: NodeId, text: &mut String) {
    for child_id in dom.children(node_id) {
        if let Some(node) = dom.get(child_id) {
            if let Some(t) = node.as_text() {
                text.push_str(t);
            } else if node.is_element() {
                get_text_content_recursive(dom, child_id, text);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gugalanna_css::Stylesheet;
    use gugalanna_html::HtmlParser;
    use gugalanna_style::Cascade;
    use gugalanna_dom::Queryable;

    fn setup(html: &str, css: &str) -> (DomTree, StyleTree) {
        let dom = HtmlParser::new().parse(html).unwrap();
        let mut cascade = Cascade::new();
        if !css.is_empty() {
            cascade.add_author_stylesheet(Stylesheet::parse(css).unwrap());
        }
        let style_tree = StyleTree::build(&dom, &cascade, 1024.0, 768.0);
        (dom, style_tree)
    }

    #[test]
    fn test_build_simple_block() {
        let (dom, style_tree) = setup(
            "<div>Hello</div>",
            "div { display: block; }",
        );
        let div_id = dom.get_elements_by_tag_name("div")[0];
        let layout = build_layout_tree(&dom, &style_tree, div_id).unwrap();

        assert!(layout.is_block());
    }

    #[test]
    fn test_build_with_text() {
        let (dom, style_tree) = setup(
            "<p>Hello World</p>",
            "p { display: block; }",
        );
        let p_id = dom.get_elements_by_tag_name("p")[0];
        let layout = build_layout_tree(&dom, &style_tree, p_id).unwrap();

        assert!(layout.is_block());
        // Text should be in an anonymous block since p is block
        assert!(!layout.children.is_empty());
    }

    #[test]
    fn test_display_none_skipped() {
        let (dom, style_tree) = setup(
            "<div><span>visible</span><span class='hidden'>hidden</span></div>",
            "div { display: block; } span { display: inline; } .hidden { display: none; }",
        );
        let div_id = dom.get_elements_by_tag_name("div")[0];
        let layout = build_layout_tree(&dom, &style_tree, div_id).unwrap();

        // Only one span should be included
        assert_eq!(layout.children.len(), 1);
    }

    #[test]
    fn test_nested_blocks() {
        let (dom, style_tree) = setup(
            "<div><p>Para 1</p><p>Para 2</p></div>",
            "div, p { display: block; }",
        );
        let div_id = dom.get_elements_by_tag_name("div")[0];
        let layout = build_layout_tree(&dom, &style_tree, div_id).unwrap();

        // Should have 2 block children (p elements) plus potential anonymous blocks for text
        assert!(layout.children.len() >= 2);
    }

    #[test]
    fn test_inline_in_block() {
        let (dom, style_tree) = setup(
            "<div><span>inline</span></div>",
            "div { display: block; } span { display: inline; }",
        );
        let div_id = dom.get_elements_by_tag_name("div")[0];
        let layout = build_layout_tree(&dom, &style_tree, div_id).unwrap();

        assert!(layout.is_block());
        // Inline content should be wrapped or direct child
        assert!(!layout.children.is_empty());
    }

    #[test]
    fn test_collapse_whitespace_basic() {
        assert_eq!(collapse_whitespace("hello"), "hello");
        assert_eq!(collapse_whitespace("hello world"), "hello world");
    }

    #[test]
    fn test_collapse_whitespace_multiple_spaces() {
        assert_eq!(collapse_whitespace("hello    world"), "hello world");
        assert_eq!(collapse_whitespace("a   b   c"), "a b c");
    }

    #[test]
    fn test_collapse_whitespace_preserves_leading_trailing() {
        // Leading space preserved
        assert_eq!(collapse_whitespace(" hello"), " hello");
        // Trailing space preserved
        assert_eq!(collapse_whitespace("hello "), "hello ");
        // Both preserved
        assert_eq!(collapse_whitespace(" hello "), " hello ");
    }

    #[test]
    fn test_collapse_whitespace_only_whitespace() {
        assert_eq!(collapse_whitespace("   "), " ");
        assert_eq!(collapse_whitespace("\t\n"), " ");
    }

    #[test]
    fn test_collapse_whitespace_empty() {
        assert_eq!(collapse_whitespace(""), "");
    }
}

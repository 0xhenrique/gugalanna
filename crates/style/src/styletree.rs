//! Style Tree
//!
//! Builds a tree of computed styles from DOM and stylesheets.

use std::collections::HashMap;

use gugalanna_css::{CssValue, Declaration};
use gugalanna_dom::{DomTree, NodeId};

use crate::cascade::Cascade;
use crate::resolver::{ResolveContext, StyleResolver};
use crate::ComputedStyle;

/// A tree of computed styles, parallel to the DOM tree
pub struct StyleTree {
    /// Map from node ID to computed style
    styles: HashMap<NodeId, ComputedStyle>,
    /// Root element ID
    root: Option<NodeId>,
}

impl StyleTree {
    /// Create a new empty style tree
    pub fn new() -> Self {
        Self {
            styles: HashMap::new(),
            root: None,
        }
    }

    /// Build a style tree from DOM and cascade
    pub fn build(tree: &DomTree, cascade: &Cascade) -> Self {
        let mut style_tree = Self::new();
        let mut context = ResolveContext::default();

        let root_id = tree.document_id();
        style_tree.root = Some(root_id);
        style_tree.compute_styles_recursive(tree, cascade, root_id, &mut context);

        style_tree
    }

    /// Get the computed style for a node
    pub fn get_style(&self, node_id: NodeId) -> Option<&ComputedStyle> {
        self.styles.get(&node_id)
    }

    /// Compute styles recursively for the tree
    fn compute_styles_recursive(
        &mut self,
        tree: &DomTree,
        cascade: &Cascade,
        node_id: NodeId,
        context: &mut ResolveContext,
    ) {
        let node = match tree.get(node_id) {
            Some(n) => n,
            None => return,
        };

        // Only compute styles for element nodes
        if node.is_element() {
            let style = self.compute_style(tree, cascade, node_id, context);

            // Update context for children with this element's style
            let old_parent = context.parent_style.take();
            context.parent_style = Some(style.clone());

            self.styles.insert(node_id, style);

            // Process children
            for child_id in tree.children(node_id) {
                self.compute_styles_recursive(tree, cascade, child_id, context);
            }

            // Restore parent context
            context.parent_style = old_parent;
        } else {
            // For non-element nodes, just process children with same context
            for child_id in tree.children(node_id) {
                self.compute_styles_recursive(tree, cascade, child_id, context);
            }
        }
    }

    /// Compute the style for a single element
    fn compute_style(
        &self,
        tree: &DomTree,
        cascade: &Cascade,
        node_id: NodeId,
        context: &ResolveContext,
    ) -> ComputedStyle {
        // Start with default style
        let mut style = ComputedStyle::default();

        // Get declarations from cascade, sorted by priority
        let declarations = cascade.get_matching_declarations(tree, node_id);

        // Group declarations by property (later declarations override earlier ones)
        let mut property_values: HashMap<String, &Declaration> = HashMap::new();
        for matched in &declarations {
            property_values.insert(
                matched.declaration.property.clone(),
                &matched.declaration,
            );
        }

        // Apply each property value
        for (property, decl) in &property_values {
            self.apply_property(&mut style, property, &decl.value, context);
        }

        // Apply inheritance for unset inherited properties
        if let Some(parent) = &context.parent_style {
            self.apply_inheritance(&mut style, parent, &property_values);
        }

        style
    }

    /// Apply a property value to the computed style
    fn apply_property(
        &self,
        style: &mut ComputedStyle,
        property: &str,
        value: &CssValue,
        context: &ResolveContext,
    ) {
        // Handle inherit/initial/unset keywords
        let value = match StyleResolver::resolve_keyword_value(property, value, context) {
            Some(v) => v,
            None => return, // initial value - use default
        };

        match property {
            // Display
            "display" => {
                if let Some(d) = StyleResolver::resolve_display(&value) {
                    style.display = d;
                }
            }

            // Position
            "position" => {
                if let Some(p) = StyleResolver::resolve_position(&value) {
                    style.position = p;
                }
            }

            // Box positioning
            "top" => {
                style.top = StyleResolver::resolve_length(&value, context);
            }
            "right" => {
                style.right = StyleResolver::resolve_length(&value, context);
            }
            "bottom" => {
                style.bottom = StyleResolver::resolve_length(&value, context);
            }
            "left" => {
                style.left = StyleResolver::resolve_length(&value, context);
            }

            // Dimensions
            "width" => {
                style.width = StyleResolver::resolve_length(&value, context);
            }
            "height" => {
                style.height = StyleResolver::resolve_length(&value, context);
            }

            // Margins
            "margin-top" => {
                if let Some(v) = StyleResolver::resolve_length(&value, context) {
                    style.margin_top = v;
                }
            }
            "margin-right" => {
                if let Some(v) = StyleResolver::resolve_length(&value, context) {
                    style.margin_right = v;
                }
            }
            "margin-bottom" => {
                if let Some(v) = StyleResolver::resolve_length(&value, context) {
                    style.margin_bottom = v;
                }
            }
            "margin-left" => {
                if let Some(v) = StyleResolver::resolve_length(&value, context) {
                    style.margin_left = v;
                }
            }

            // Padding
            "padding-top" => {
                if let Some(v) = StyleResolver::resolve_length(&value, context) {
                    style.padding_top = v;
                }
            }
            "padding-right" => {
                if let Some(v) = StyleResolver::resolve_length(&value, context) {
                    style.padding_right = v;
                }
            }
            "padding-bottom" => {
                if let Some(v) = StyleResolver::resolve_length(&value, context) {
                    style.padding_bottom = v;
                }
            }
            "padding-left" => {
                if let Some(v) = StyleResolver::resolve_length(&value, context) {
                    style.padding_left = v;
                }
            }

            // Border widths
            "border-top-width" => {
                if let Some(v) = StyleResolver::resolve_length(&value, context) {
                    style.border_top_width = v;
                }
            }
            "border-right-width" => {
                if let Some(v) = StyleResolver::resolve_length(&value, context) {
                    style.border_right_width = v;
                }
            }
            "border-bottom-width" => {
                if let Some(v) = StyleResolver::resolve_length(&value, context) {
                    style.border_bottom_width = v;
                }
            }
            "border-left-width" => {
                if let Some(v) = StyleResolver::resolve_length(&value, context) {
                    style.border_left_width = v;
                }
            }

            // Colors
            "color" => {
                if let Some(c) = StyleResolver::resolve_color(&value, context) {
                    style.color = c;
                }
            }
            "background-color" => {
                if let Some(c) = StyleResolver::resolve_color(&value, context) {
                    style.background_color = c;
                }
            }
            "border-color" => {
                if let Some(c) = StyleResolver::resolve_color(&value, context) {
                    style.border_color = c;
                }
            }

            // Text
            "font-size" => {
                if let Some(v) = StyleResolver::resolve_font_size(&value, context) {
                    style.font_size = v;
                }
            }
            "font-weight" => {
                if let Some(w) = StyleResolver::resolve_font_weight(&value) {
                    style.font_weight = w;
                }
            }
            "font-family" => {
                if let CssValue::Keyword(f) = &value {
                    style.font_family = f.clone();
                } else if let CssValue::String(f) = &value {
                    style.font_family = f.clone();
                }
            }
            "line-height" => {
                if let Some(v) = StyleResolver::resolve_line_height(&value, context) {
                    style.line_height = v;
                }
            }
            "text-align" => {
                if let Some(a) = StyleResolver::resolve_text_align(&value) {
                    style.text_align = a;
                }
            }

            _ => {}
        }
    }

    /// Apply inheritance for properties that weren't explicitly set
    fn apply_inheritance(
        &self,
        style: &mut ComputedStyle,
        parent: &ComputedStyle,
        set_properties: &HashMap<String, &Declaration>,
    ) {
        // Inherited properties that should be copied from parent if not set
        if !set_properties.contains_key("color") {
            style.color = parent.color;
        }
        if !set_properties.contains_key("font-size") {
            style.font_size = parent.font_size;
        }
        if !set_properties.contains_key("font-family") {
            style.font_family = parent.font_family.clone();
        }
        if !set_properties.contains_key("font-weight") {
            style.font_weight = parent.font_weight;
        }
        if !set_properties.contains_key("line-height") {
            style.line_height = parent.line_height;
        }
        if !set_properties.contains_key("text-align") {
            style.text_align = parent.text_align;
        }
    }
}

impl Default for StyleTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Display;
    use gugalanna_css::Stylesheet;
    use gugalanna_dom::Queryable;
    use gugalanna_html::HtmlParser;

    fn parse_html(html: &str) -> DomTree {
        HtmlParser::new().parse(html).unwrap()
    }

    #[test]
    fn test_style_tree_basic() {
        let tree = parse_html("<div style='color: red;'>Hello</div>");
        let div_id = tree.get_elements_by_tag_name("div")[0];

        let mut cascade = Cascade::new();
        cascade.add_author_stylesheet(
            Stylesheet::parse("div { display: block; color: blue; }").unwrap()
        );

        let style_tree = StyleTree::build(&tree, &cascade);
        let style = style_tree.get_style(div_id).unwrap();

        assert_eq!(style.display, Display::Block);
    }

    #[test]
    fn test_style_tree_inheritance() {
        let tree = parse_html("<div><span>Hello</span></div>");
        let div_id = tree.get_elements_by_tag_name("div")[0];
        let span_id = tree.get_elements_by_tag_name("span")[0];

        let mut cascade = Cascade::new();
        cascade.add_author_stylesheet(
            Stylesheet::parse("div { color: red; font-size: 20px; }").unwrap()
        );

        let style_tree = StyleTree::build(&tree, &cascade);

        // Div should have the explicit color
        let div_style = style_tree.get_style(div_id).unwrap();
        assert_eq!(div_style.color.r, 255);
        assert_eq!(div_style.font_size, 20.0);

        // Span should inherit color and font-size
        let span_style = style_tree.get_style(span_id).unwrap();
        assert_eq!(span_style.color.r, 255);
        assert_eq!(span_style.font_size, 20.0);
    }

    #[test]
    fn test_style_tree_non_inherited() {
        let tree = parse_html("<div><p>Hello</p></div>");
        let div_id = tree.get_elements_by_tag_name("div")[0];
        let p_id = tree.get_elements_by_tag_name("p")[0];

        let mut cascade = Cascade::new();
        cascade.add_author_stylesheet(
            Stylesheet::parse("div { margin-left: 50px; }").unwrap()
        );

        let style_tree = StyleTree::build(&tree, &cascade);

        // Div should have the margin
        let div_style = style_tree.get_style(div_id).unwrap();
        assert_eq!(div_style.margin_left, 50.0);

        // P should NOT inherit margin (margin is not inherited)
        let p_style = style_tree.get_style(p_id).unwrap();
        assert_eq!(p_style.margin_left, 0.0);
    }

    #[test]
    fn test_style_tree_em_units() {
        let tree = parse_html("<div><span>Hello</span></div>");
        let div_id = tree.get_elements_by_tag_name("div")[0];
        let span_id = tree.get_elements_by_tag_name("span")[0];

        let mut cascade = Cascade::new();
        cascade.add_author_stylesheet(
            Stylesheet::parse("div { font-size: 20px; } span { font-size: 2em; }").unwrap()
        );

        let style_tree = StyleTree::build(&tree, &cascade);

        let div_style = style_tree.get_style(div_id).unwrap();
        assert_eq!(div_style.font_size, 20.0);

        // Span's 2em should be 2 * 20 = 40px
        let span_style = style_tree.get_style(span_id).unwrap();
        assert_eq!(span_style.font_size, 40.0);
    }
}

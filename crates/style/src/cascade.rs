//! CSS Cascade
//!
//! Implements the CSS cascade algorithm for determining
//! which declarations apply to an element.

use gugalanna_css::{Stylesheet, Rule, StyleRule, Declaration, Specificity, parse_inline_style};
use gugalanna_dom::{DomTree, NodeId};

use crate::matching::{matches_selector_with_context, MatchingContext};

/// Origin of a stylesheet
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Origin {
    /// User agent stylesheet (browser defaults)
    UserAgent,
    /// User stylesheet
    User,
    /// Author stylesheet (website CSS)
    Author,
}

/// A matched declaration with its cascade priority
#[derive(Debug, Clone)]
pub struct MatchedDeclaration {
    /// The declaration
    pub declaration: Declaration,
    /// Origin of the stylesheet
    pub origin: Origin,
    /// Specificity of the selector that matched
    pub specificity: Specificity,
    /// Order in which this rule appeared (higher = later)
    pub source_order: u32,
}

impl MatchedDeclaration {
    /// Compare two declarations by cascade priority
    /// Returns true if self should override other
    pub fn overrides(&self, other: &Self) -> bool {
        // CSS Cascade order (highest to lowest priority):
        // 1. !important declarations from user agent
        // 2. !important declarations from user
        // 3. !important declarations from author
        // 4. Normal declarations from author
        // 5. Normal declarations from user
        // 6. Normal declarations from user agent

        let self_important = self.declaration.important;
        let other_important = other.declaration.important;

        // Important beats non-important
        if self_important && !other_important {
            return true;
        }
        if !self_important && other_important {
            return false;
        }

        // Both important or both normal
        if self_important {
            // For important, user agent > user > author (reversed)
            if self.origin != other.origin {
                return self.origin < other.origin;
            }
        } else {
            // For normal, author > user > user agent
            if self.origin != other.origin {
                return self.origin > other.origin;
            }
        }

        // Same origin and importance - compare specificity
        if self.specificity != other.specificity {
            return self.specificity > other.specificity;
        }

        // Same specificity - later declaration wins
        self.source_order > other.source_order
    }
}

/// Cascade context for style computation
#[derive(Clone)]
pub struct Cascade {
    /// User agent stylesheets
    ua_stylesheets: Vec<Stylesheet>,
    /// User stylesheets
    user_stylesheets: Vec<Stylesheet>,
    /// Author stylesheets
    author_stylesheets: Vec<Stylesheet>,
}

impl Cascade {
    /// Create a new cascade with the default UA stylesheet
    pub fn new() -> Self {
        Self {
            ua_stylesheets: vec![default_ua_stylesheet()],
            user_stylesheets: Vec::new(),
            author_stylesheets: Vec::new(),
        }
    }

    /// Add a user agent stylesheet
    pub fn add_ua_stylesheet(&mut self, stylesheet: Stylesheet) {
        self.ua_stylesheets.push(stylesheet);
    }

    /// Add a user stylesheet
    pub fn add_user_stylesheet(&mut self, stylesheet: Stylesheet) {
        self.user_stylesheets.push(stylesheet);
    }

    /// Add an author stylesheet
    pub fn add_author_stylesheet(&mut self, stylesheet: Stylesheet) {
        self.author_stylesheets.push(stylesheet);
    }

    /// Get all matching declarations for an element, sorted by cascade priority
    pub fn get_matching_declarations(
        &self,
        tree: &DomTree,
        element_id: NodeId,
    ) -> Vec<MatchedDeclaration> {
        self.get_matching_declarations_with_context(tree, element_id, &MatchingContext::new())
    }

    /// Get all matching declarations with dynamic pseudo-class context (hover, focus, etc.)
    pub fn get_matching_declarations_with_context(
        &self,
        tree: &DomTree,
        element_id: NodeId,
        context: &MatchingContext,
    ) -> Vec<MatchedDeclaration> {
        let mut declarations = Vec::new();
        let mut source_order = 0u32;

        // Collect from all stylesheets in cascade order
        for stylesheet in &self.ua_stylesheets {
            self.collect_matching_declarations(
                tree,
                element_id,
                stylesheet,
                Origin::UserAgent,
                &mut source_order,
                &mut declarations,
                context,
            );
        }

        for stylesheet in &self.user_stylesheets {
            self.collect_matching_declarations(
                tree,
                element_id,
                stylesheet,
                Origin::User,
                &mut source_order,
                &mut declarations,
                context,
            );
        }

        for stylesheet in &self.author_stylesheets {
            self.collect_matching_declarations(
                tree,
                element_id,
                stylesheet,
                Origin::Author,
                &mut source_order,
                &mut declarations,
                context,
            );
        }

        // Collect inline styles from the element's style attribute
        // Inline styles have specificity (1,0,0,0) - higher than any selector
        if let Some(node) = tree.get(element_id) {
            if let Some(element) = node.as_element() {
                if let Some(style_attr) = element.get_attribute("style") {
                    self.collect_inline_style_declarations(
                        style_attr,
                        &mut source_order,
                        &mut declarations,
                    );
                }
            }
        }

        // Sort by cascade priority (stable sort preserves source order for equal priority)
        declarations.sort_by(|a, b| {
            if a.overrides(b) {
                std::cmp::Ordering::Greater
            } else if b.overrides(a) {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        });

        declarations
    }

    /// Collect matching declarations from a stylesheet
    fn collect_matching_declarations(
        &self,
        tree: &DomTree,
        element_id: NodeId,
        stylesheet: &Stylesheet,
        origin: Origin,
        source_order: &mut u32,
        declarations: &mut Vec<MatchedDeclaration>,
        context: &MatchingContext,
    ) {
        for rule in &stylesheet.rules {
            match rule {
                Rule::Style(style_rule) => {
                    self.collect_from_style_rule(
                        tree,
                        element_id,
                        style_rule,
                        origin,
                        source_order,
                        declarations,
                        context,
                    );
                }
                Rule::Media(media_rule) => {
                    // TODO: Evaluate media query
                    // For now, assume all media rules match
                    for nested_rule in &media_rule.rules {
                        if let Rule::Style(style_rule) = nested_rule {
                            self.collect_from_style_rule(
                                tree,
                                element_id,
                                style_rule,
                                origin,
                                source_order,
                                declarations,
                                context,
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Collect matching declarations from a style rule
    fn collect_from_style_rule(
        &self,
        tree: &DomTree,
        element_id: NodeId,
        rule: &StyleRule,
        origin: Origin,
        source_order: &mut u32,
        declarations: &mut Vec<MatchedDeclaration>,
        context: &MatchingContext,
    ) {
        // Find the highest specificity selector that matches
        let mut best_specificity: Option<Specificity> = None;

        for selector in &rule.selectors {
            if matches_selector_with_context(tree, element_id, selector, context) {
                match &best_specificity {
                    Some(spec) if selector.specificity <= *spec => {}
                    _ => best_specificity = Some(selector.specificity),
                }
            }
        }

        // If any selector matched, add all declarations
        if let Some(specificity) = best_specificity {
            for decl in &rule.declarations {
                declarations.push(MatchedDeclaration {
                    declaration: decl.clone(),
                    origin,
                    specificity,
                    source_order: *source_order,
                });
                *source_order += 1;
            }
        }
    }

    /// Collect declarations from inline style attribute
    fn collect_inline_style_declarations(
        &self,
        style_attr: &str,
        source_order: &mut u32,
        declarations: &mut Vec<MatchedDeclaration>,
    ) {
        // Parse the inline style
        if let Ok(decls) = parse_inline_style(style_attr) {
            // Inline styles have highest specificity - use 1000 for 'a' component
            // (regular selectors have a max of ~100 or so for deeply nested IDs)
            let inline_specificity = Specificity::new(1000, 0, 0);

            for decl in decls {
                declarations.push(MatchedDeclaration {
                    declaration: decl,
                    origin: Origin::Author,
                    specificity: inline_specificity,
                    source_order: *source_order,
                });
                *source_order += 1;
            }
        }
    }

    /// Get the cascaded value for a specific property
    pub fn get_cascaded_value(
        &self,
        tree: &DomTree,
        element_id: NodeId,
        property: &str,
    ) -> Option<Declaration> {
        let declarations = self.get_matching_declarations(tree, element_id);

        // Return the highest priority declaration for this property
        declarations
            .into_iter()
            .filter(|d| d.declaration.property == property)
            .max_by(|a, b| {
                if a.overrides(b) {
                    std::cmp::Ordering::Greater
                } else if b.overrides(a) {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .map(|d| d.declaration)
    }
}

impl Default for Cascade {
    fn default() -> Self {
        Self::new()
    }
}

/// Default user agent styles for common HTML elements
pub fn default_ua_stylesheet() -> Stylesheet {
    let css = r#"
        /* Block elements */
        html, address, blockquote, body, dd, div, dl, dt, fieldset, form,
        frame, frameset, h1, h2, h3, h4, h5, h6, noframes, ol, p, ul, center,
        dir, hr, menu, pre, article, aside, footer, header, main, nav, section,
        figure, figcaption { display: block; }

        li { display: list-item; }

        /* Hidden elements */
        head, script, style, title, meta, link, noscript, template { display: none; }

        /* Table elements */
        table { display: table; }
        tr { display: table-row; }
        thead { display: table-header-group; }
        tbody { display: table-row-group; }
        tfoot { display: table-footer-group; }
        td, th { display: table-cell; }
        caption { display: table-caption; }
        colgroup { display: table-column-group; }
        col { display: table-column; }

        /* Inline elements */
        a, abbr, acronym, b, bdo, big, br, cite, code, dfn, em, i, img, kbd,
        label, q, s, samp, small, span, strong, sub, sup, tt, u, var { display: inline; }

        /* Headings */
        h1 { font-size: 2em; margin-top: 0.67em; margin-bottom: 0.67em; font-weight: bold; }
        h2 { font-size: 1.5em; margin-top: 0.83em; margin-bottom: 0.83em; font-weight: bold; }
        h3 { font-size: 1.17em; margin-top: 1em; margin-bottom: 1em; font-weight: bold; }
        h4 { margin-top: 1.33em; margin-bottom: 1.33em; font-weight: bold; }
        h5 { font-size: 0.83em; margin-top: 1.67em; margin-bottom: 1.67em; font-weight: bold; }
        h6 { font-size: 0.67em; margin-top: 2.33em; margin-bottom: 2.33em; font-weight: bold; }

        /* Paragraphs and lists */
        p { margin-top: 1em; margin-bottom: 1em; }
        ul, ol { margin-top: 1em; margin-bottom: 1em; padding-left: 40px; }
        li { margin-top: 0; margin-bottom: 0; }

        /* Links */
        a { color: blue; }
        a:visited { color: purple; }

        /* Text formatting */
        strong, b { font-weight: bold; }
        em, i { font-style: italic; }
        u { text-decoration: underline; }
        s, strike, del { text-decoration: line-through; }

        /* Monospace */
        pre, code, tt, kbd, samp { font-family: monospace; }

        /* Form elements - inline-block so they flow with text but have box properties */
        button, input, select, textarea { display: inline-block; }

        /* Horizontal rule */
        hr { border: 1px solid gray; margin-top: 0.5em; margin-bottom: 0.5em; }

        /* Blockquote */
        blockquote { margin-left: 40px; margin-right: 40px; margin-top: 1em; margin-bottom: 1em; }
    "#;

    Stylesheet::parse(css).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gugalanna_dom::Queryable;
    use gugalanna_html::HtmlParser;
    use gugalanna_css::CssValue;

    fn parse_html(html: &str) -> DomTree {
        HtmlParser::new().parse(html).unwrap()
    }

    #[test]
    fn test_cascade_basic() {
        let tree = parse_html("<p>Hello</p>");
        let p_nodes = tree.get_elements_by_tag_name("p");

        let mut cascade = Cascade::new();
        cascade.add_author_stylesheet(Stylesheet::parse("p { color: red; }").unwrap());

        let decl = cascade.get_cascaded_value(&tree, p_nodes[0], "color");
        assert!(decl.is_some());
        assert!(matches!(decl.unwrap().value, CssValue::Color(_)));
    }

    #[test]
    fn test_cascade_specificity() {
        let tree = parse_html("<p class='intro'>Hello</p>");
        let p_nodes = tree.get_elements_by_tag_name("p");

        let mut cascade = Cascade::new();
        cascade.add_author_stylesheet(
            Stylesheet::parse("p { color: red; } .intro { color: blue; }").unwrap()
        );

        let decl = cascade.get_cascaded_value(&tree, p_nodes[0], "color");
        assert!(decl.is_some());
        // .intro has higher specificity than p
        if let CssValue::Color(color) = decl.unwrap().value {
            assert_eq!(color.b, 255); // blue
        }
    }

    #[test]
    fn test_cascade_important() {
        let tree = parse_html("<p class='intro'>Hello</p>");
        let p_nodes = tree.get_elements_by_tag_name("p");

        let mut cascade = Cascade::new();
        cascade.add_author_stylesheet(
            Stylesheet::parse("p { color: red !important; } .intro { color: blue; }").unwrap()
        );

        let decl = cascade.get_cascaded_value(&tree, p_nodes[0], "color");
        assert!(decl.is_some());
        // !important beats specificity
        if let CssValue::Color(color) = decl.unwrap().value {
            assert_eq!(color.r, 255); // red
        }
    }

    #[test]
    fn test_cascade_source_order() {
        let tree = parse_html("<p>Hello</p>");
        let p_nodes = tree.get_elements_by_tag_name("p");

        let mut cascade = Cascade::new();
        cascade.add_author_stylesheet(
            Stylesheet::parse("p { color: red; } p { color: blue; }").unwrap()
        );

        let decl = cascade.get_cascaded_value(&tree, p_nodes[0], "color");
        assert!(decl.is_some());
        // Later declaration wins
        if let CssValue::Color(color) = decl.unwrap().value {
            assert_eq!(color.b, 255); // blue
        }
    }

    #[test]
    fn test_cascade_origin() {
        let tree = parse_html("<p>Hello</p>");
        let p_nodes = tree.get_elements_by_tag_name("p");

        let mut cascade = Cascade::new();
        cascade.add_ua_stylesheet(Stylesheet::parse("p { color: gray; }").unwrap());
        cascade.add_author_stylesheet(Stylesheet::parse("p { color: red; }").unwrap());

        let decl = cascade.get_cascaded_value(&tree, p_nodes[0], "color");
        assert!(decl.is_some());
        // Author beats UA
        if let CssValue::Color(color) = decl.unwrap().value {
            assert_eq!(color.r, 255); // red
        }
    }

    #[test]
    fn test_default_ua_stylesheet() {
        let ua = default_ua_stylesheet();
        assert!(!ua.rules.is_empty());
    }
}

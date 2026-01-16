//! Selector Matching
//!
//! Matches CSS selectors against DOM elements.

use std::collections::HashSet;
use gugalanna_css::{Selector, SelectorPart, Combinator, AttributeOp};
use gugalanna_dom::{DomTree, NodeId, ElementData};

/// Context for dynamic pseudo-class matching (hover, focus, etc.)
#[derive(Debug, Clone, Default)]
pub struct MatchingContext {
    /// Elements currently being hovered
    pub hovered: HashSet<NodeId>,
    /// Element currently focused
    pub focused: Option<NodeId>,
}

impl MatchingContext {
    /// Create a new empty matching context
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a context with a hovered element and its ancestors
    pub fn with_hover(tree: &DomTree, element_id: NodeId) -> Self {
        let mut ctx = Self::new();
        // Add the element and all its ancestors to the hover set
        // (because :hover applies to parent elements too)
        let mut current = Some(element_id);
        while let Some(id) = current {
            ctx.hovered.insert(id);
            current = tree.get(id).and_then(|n| n.parent);
        }
        ctx
    }

    /// Check if an element is hovered
    pub fn is_hovered(&self, element_id: NodeId) -> bool {
        self.hovered.contains(&element_id)
    }

    /// Check if an element is focused
    pub fn is_focused(&self, element_id: NodeId) -> bool {
        self.focused == Some(element_id)
    }
}

/// Check if a selector matches a specific element in the DOM tree
pub fn matches_selector(tree: &DomTree, element_id: NodeId, selector: &Selector) -> bool {
    matches_selector_with_context(tree, element_id, selector, &MatchingContext::new())
}

/// Check if a selector matches with dynamic pseudo-class context (hover, focus, etc.)
pub fn matches_selector_with_context(
    tree: &DomTree,
    element_id: NodeId,
    selector: &Selector,
    context: &MatchingContext,
) -> bool {
    // Start matching from the rightmost part of the selector
    // and work backwards through combinators
    let parts = &selector.parts;

    if parts.is_empty() {
        return false;
    }

    // Find the last non-combinator part (the subject of the selector)
    let mut current_element = element_id;
    let mut part_index = parts.len() - 1;

    loop {
        // Skip trailing combinators (shouldn't happen, but be safe)
        while part_index > 0 && matches!(parts[part_index], SelectorPart::Combinator(_)) {
            part_index -= 1;
        }

        // Match compound selector (consecutive non-combinator parts)
        let compound_end = part_index;
        let mut compound_start = part_index;

        // Find the start of the compound selector
        while compound_start > 0 && !matches!(parts[compound_start - 1], SelectorPart::Combinator(_)) {
            compound_start -= 1;
        }

        // Match all parts in the compound selector against current element
        if !matches_compound(tree, current_element, &parts[compound_start..=compound_end], context) {
            return false;
        }

        // If we've matched everything, success!
        if compound_start == 0 {
            return true;
        }

        // Get the combinator before this compound selector
        let combinator = match &parts[compound_start - 1] {
            SelectorPart::Combinator(c) => *c,
            _ => return false, // Shouldn't happen
        };

        // Move to the next part
        part_index = if compound_start > 1 { compound_start - 2 } else { return true };

        // Find the next element based on the combinator
        current_element = match find_matching_element(tree, current_element, combinator, &parts[..=part_index], context) {
            Some(id) => id,
            None => return false,
        };
    }
}

/// Match a compound selector (consecutive simple selectors) against an element
fn matches_compound(
    tree: &DomTree,
    element_id: NodeId,
    parts: &[SelectorPart],
    context: &MatchingContext,
) -> bool {
    let node = match tree.get(element_id) {
        Some(n) => n,
        None => return false,
    };

    let element = match node.as_element() {
        Some(e) => e,
        None => return false,
    };

    for part in parts {
        if !matches_simple_selector(tree, element_id, element, part, context) {
            return false;
        }
    }

    true
}

/// Match a single simple selector against an element
fn matches_simple_selector(
    tree: &DomTree,
    element_id: NodeId,
    element: &ElementData,
    part: &SelectorPart,
    context: &MatchingContext,
) -> bool {
    match part {
        SelectorPart::Universal => true,

        SelectorPart::Type(tag) => element.tag_name.eq_ignore_ascii_case(tag),

        SelectorPart::Class(class) => element.has_class(class),

        SelectorPart::Id(id) => element.id() == Some(id.as_str()),

        SelectorPart::Attribute { name, op, value, case_insensitive } => {
            matches_attribute(element, name, op.as_ref(), value.as_deref(), *case_insensitive)
        }

        SelectorPart::PseudoClass { name, args } => {
            matches_pseudo_class(tree, element_id, element, name, args.as_deref(), context)
        }

        SelectorPart::PseudoElement { .. } => {
            // Pseudo-elements don't affect matching, they create additional boxes
            true
        }

        SelectorPart::Combinator(_) => {
            // Combinators are handled separately
            true
        }
    }
}

/// Match an attribute selector
fn matches_attribute(
    element: &ElementData,
    name: &str,
    op: Option<&AttributeOp>,
    expected_value: Option<&str>,
    case_insensitive: bool,
) -> bool {
    let attr_value = match element.get_attribute(name) {
        Some(v) => v,
        None => return false,
    };

    let op = match op {
        Some(o) => o,
        None => return true, // [attr] just checks existence
    };

    let expected = match expected_value {
        Some(v) => v,
        None => return false,
    };

    let (attr_value, expected) = if case_insensitive {
        (attr_value.to_ascii_lowercase(), expected.to_ascii_lowercase())
    } else {
        (attr_value.to_string(), expected.to_string())
    };

    match op {
        AttributeOp::Equals => attr_value == expected,
        AttributeOp::Includes => attr_value.split_whitespace().any(|w| w == expected),
        AttributeOp::DashMatch => {
            attr_value == expected || attr_value.starts_with(&format!("{}-", expected))
        }
        AttributeOp::PrefixMatch => attr_value.starts_with(&expected),
        AttributeOp::SuffixMatch => attr_value.ends_with(&expected),
        AttributeOp::SubstringMatch => attr_value.contains(&expected),
    }
}

/// Match a pseudo-class
fn matches_pseudo_class(
    tree: &DomTree,
    element_id: NodeId,
    element: &ElementData,
    name: &str,
    args: Option<&str>,
    context: &MatchingContext,
) -> bool {
    match name {
        "first-child" => is_first_child(tree, element_id),
        "last-child" => is_last_child(tree, element_id),
        "only-child" => is_first_child(tree, element_id) && is_last_child(tree, element_id),
        "first-of-type" => is_first_of_type(tree, element_id, &element.tag_name),
        "last-of-type" => is_last_of_type(tree, element_id, &element.tag_name),
        "only-of-type" => {
            is_first_of_type(tree, element_id, &element.tag_name)
            && is_last_of_type(tree, element_id, &element.tag_name)
        }
        "empty" => is_empty(tree, element_id),
        "root" => is_root(tree, element_id),
        "nth-child" => {
            if let Some(args) = args {
                matches_nth_child(tree, element_id, args, false)
            } else {
                false
            }
        }
        "nth-last-child" => {
            if let Some(args) = args {
                matches_nth_child(tree, element_id, args, true)
            } else {
                false
            }
        }
        "nth-of-type" => {
            if let Some(args) = args {
                matches_nth_of_type(tree, element_id, &element.tag_name, args, false)
            } else {
                false
            }
        }
        "nth-last-of-type" => {
            if let Some(args) = args {
                matches_nth_of_type(tree, element_id, &element.tag_name, args, true)
            } else {
                false
            }
        }
        "not" => {
            if let Some(args) = args {
                // Parse the selector argument and check if it doesn't match
                if let Ok(sel) = Selector::parse(args) {
                    !matches_selector_with_context(tree, element_id, &sel, context)
                } else {
                    true
                }
            } else {
                true
            }
        }
        "link" => element.tag_name == "a" && element.get_attribute("href").is_some(),
        "enabled" => !matches_disabled(element),
        "disabled" => matches_disabled(element),
        "checked" => element.get_attribute("checked").is_some(),
        "required" => element.get_attribute("required").is_some(),
        "optional" => element.get_attribute("required").is_none(),
        "read-only" => element.get_attribute("readonly").is_some(),
        "read-write" => element.get_attribute("readonly").is_none(),

        // Dynamic pseudo-classes - now using context
        "hover" => context.is_hovered(element_id),
        "focus" => context.is_focused(element_id),

        // Not yet implemented dynamic pseudo-classes
        "active" | "focus-within" | "focus-visible" | "visited" | "target" => false,

        _ => false,
    }
}

/// Check if element is disabled
fn matches_disabled(element: &ElementData) -> bool {
    if element.get_attribute("disabled").is_some() {
        return true;
    }
    // Could add fieldset disabled logic here
    false
}

/// Check if element is first child
fn is_first_child(tree: &DomTree, element_id: NodeId) -> bool {
    let node = match tree.get(element_id) {
        Some(n) => n,
        None => return false,
    };

    let parent_id = match node.parent {
        Some(id) => id,
        None => return false,
    };

    let parent = match tree.get(parent_id) {
        Some(n) => n,
        None => return false,
    };

    // Find first element child
    for &child_id in &parent.children {
        if let Some(child) = tree.get(child_id) {
            if child.is_element() {
                return child_id == element_id;
            }
        }
    }

    false
}

/// Check if element is last child
fn is_last_child(tree: &DomTree, element_id: NodeId) -> bool {
    let node = match tree.get(element_id) {
        Some(n) => n,
        None => return false,
    };

    let parent_id = match node.parent {
        Some(id) => id,
        None => return false,
    };

    let parent = match tree.get(parent_id) {
        Some(n) => n,
        None => return false,
    };

    // Find last element child
    for &child_id in parent.children.iter().rev() {
        if let Some(child) = tree.get(child_id) {
            if child.is_element() {
                return child_id == element_id;
            }
        }
    }

    false
}

/// Check if element is first of its type among siblings
fn is_first_of_type(tree: &DomTree, element_id: NodeId, tag_name: &str) -> bool {
    let node = match tree.get(element_id) {
        Some(n) => n,
        None => return false,
    };

    let parent_id = match node.parent {
        Some(id) => id,
        None => return false,
    };

    let parent = match tree.get(parent_id) {
        Some(n) => n,
        None => return false,
    };

    for &child_id in &parent.children {
        if let Some(child) = tree.get(child_id) {
            if let Some(elem) = child.as_element() {
                if elem.tag_name == tag_name {
                    return child_id == element_id;
                }
            }
        }
    }

    false
}

/// Check if element is last of its type among siblings
fn is_last_of_type(tree: &DomTree, element_id: NodeId, tag_name: &str) -> bool {
    let node = match tree.get(element_id) {
        Some(n) => n,
        None => return false,
    };

    let parent_id = match node.parent {
        Some(id) => id,
        None => return false,
    };

    let parent = match tree.get(parent_id) {
        Some(n) => n,
        None => return false,
    };

    for &child_id in parent.children.iter().rev() {
        if let Some(child) = tree.get(child_id) {
            if let Some(elem) = child.as_element() {
                if elem.tag_name == tag_name {
                    return child_id == element_id;
                }
            }
        }
    }

    false
}

/// Check if element has no children (or only whitespace text)
fn is_empty(tree: &DomTree, element_id: NodeId) -> bool {
    let node = match tree.get(element_id) {
        Some(n) => n,
        None => return false,
    };

    for &child_id in &node.children {
        if let Some(child) = tree.get(child_id) {
            match &child.node_type {
                gugalanna_dom::NodeType::Element(_) => return false,
                gugalanna_dom::NodeType::Text(text) => {
                    if !text.trim().is_empty() {
                        return false;
                    }
                }
                _ => {}
            }
        }
    }

    true
}

/// Check if element is the root element (html)
fn is_root(tree: &DomTree, element_id: NodeId) -> bool {
    let node = match tree.get(element_id) {
        Some(n) => n,
        None => return false,
    };

    // Root element's parent should be the document
    node.parent == Some(tree.document_id())
}

/// Match :nth-child() pseudo-class
fn matches_nth_child(tree: &DomTree, element_id: NodeId, args: &str, from_end: bool) -> bool {
    let (a, b) = parse_nth_args(args);
    let index = get_element_index(tree, element_id, from_end);

    match index {
        Some(n) => matches_an_plus_b(n as i32, a, b),
        None => false,
    }
}

/// Match :nth-of-type() pseudo-class
fn matches_nth_of_type(
    tree: &DomTree,
    element_id: NodeId,
    tag_name: &str,
    args: &str,
    from_end: bool,
) -> bool {
    let (a, b) = parse_nth_args(args);
    let index = get_type_index(tree, element_id, tag_name, from_end);

    match index {
        Some(n) => matches_an_plus_b(n as i32, a, b),
        None => false,
    }
}

/// Parse an+b expression (e.g., "2n+1", "odd", "even", "3")
fn parse_nth_args(args: &str) -> (i32, i32) {
    let args = args.trim().to_ascii_lowercase();

    match args.as_str() {
        "odd" => (2, 1),
        "even" => (2, 0),
        _ => {
            // Try to parse an+b format
            if let Some(n_pos) = args.find('n') {
                let a_part = &args[..n_pos].trim();
                let a = if a_part.is_empty() || *a_part == "+" {
                    1
                } else if *a_part == "-" {
                    -1
                } else {
                    a_part.parse().unwrap_or(1)
                };

                let b_part = args[n_pos + 1..].trim();
                let b = if b_part.is_empty() {
                    0
                } else {
                    // Remove leading + sign if present
                    let b_str = b_part.trim_start_matches('+');
                    b_str.parse().unwrap_or(0)
                };

                (a, b)
            } else {
                // Just a number (b only)
                (0, args.parse().unwrap_or(0))
            }
        }
    }
}

/// Check if index matches an+b formula
fn matches_an_plus_b(index: i32, a: i32, b: i32) -> bool {
    if a == 0 {
        return index == b;
    }

    let diff = index - b;
    if a > 0 {
        diff >= 0 && diff % a == 0
    } else {
        diff <= 0 && diff % a == 0
    }
}

/// Get the 1-based index of an element among its siblings
fn get_element_index(tree: &DomTree, element_id: NodeId, from_end: bool) -> Option<usize> {
    let node = tree.get(element_id)?;
    let parent_id = node.parent?;
    let parent = tree.get(parent_id)?;

    let elements: Vec<NodeId> = parent.children.iter()
        .filter(|&&id| tree.get(id).map(|n| n.is_element()).unwrap_or(false))
        .copied()
        .collect();

    let index = if from_end {
        elements.iter().rev().position(|&id| id == element_id)
    } else {
        elements.iter().position(|&id| id == element_id)
    };

    index.map(|i| i + 1) // Convert to 1-based
}

/// Get the 1-based index of an element among siblings of the same type
fn get_type_index(
    tree: &DomTree,
    element_id: NodeId,
    tag_name: &str,
    from_end: bool,
) -> Option<usize> {
    let node = tree.get(element_id)?;
    let parent_id = node.parent?;
    let parent = tree.get(parent_id)?;

    let elements: Vec<NodeId> = parent.children.iter()
        .filter(|&&id| {
            tree.get(id)
                .and_then(|n| n.as_element())
                .map(|e| e.tag_name == tag_name)
                .unwrap_or(false)
        })
        .copied()
        .collect();

    let index = if from_end {
        elements.iter().rev().position(|&id| id == element_id)
    } else {
        elements.iter().position(|&id| id == element_id)
    };

    index.map(|i| i + 1) // Convert to 1-based
}

/// Find an element matching the remaining selector parts based on combinator
fn find_matching_element(
    tree: &DomTree,
    start_element: NodeId,
    combinator: Combinator,
    remaining_parts: &[SelectorPart],
    context: &MatchingContext,
) -> Option<NodeId> {
    // Find compound selector bounds in remaining_parts
    let compound_end = remaining_parts.len() - 1;
    let mut compound_start = compound_end;
    while compound_start > 0 && !matches!(remaining_parts[compound_start - 1], SelectorPart::Combinator(_)) {
        compound_start -= 1;
    }

    let compound = &remaining_parts[compound_start..=compound_end];

    match combinator {
        Combinator::Descendant => {
            // Check all ancestors
            let mut current = tree.get(start_element)?.parent;
            while let Some(parent_id) = current {
                if matches_compound(tree, parent_id, compound, context) {
                    return Some(parent_id);
                }
                current = tree.get(parent_id)?.parent;
            }
            None
        }
        Combinator::Child => {
            // Check immediate parent only
            let parent_id = tree.get(start_element)?.parent?;
            if matches_compound(tree, parent_id, compound, context) {
                Some(parent_id)
            } else {
                None
            }
        }
        Combinator::NextSibling => {
            // Check previous sibling only
            let prev_id = tree.get(start_element)?.prev_sibling?;
            // Find previous element sibling (skip text/comments)
            let mut current = Some(prev_id);
            while let Some(id) = current {
                if let Some(node) = tree.get(id) {
                    if node.is_element() {
                        if matches_compound(tree, id, compound, context) {
                            return Some(id);
                        }
                        return None;
                    }
                    current = node.prev_sibling;
                } else {
                    break;
                }
            }
            None
        }
        Combinator::SubsequentSibling => {
            // Check all previous siblings
            let mut current = tree.get(start_element)?.prev_sibling;
            while let Some(id) = current {
                if let Some(node) = tree.get(id) {
                    if node.is_element() && matches_compound(tree, id, compound, context) {
                        return Some(id);
                    }
                    current = node.prev_sibling;
                } else {
                    break;
                }
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gugalanna_dom::Queryable;
    use gugalanna_html::HtmlParser;

    fn parse_html(html: &str) -> DomTree {
        HtmlParser::new().parse(html).unwrap()
    }

    #[test]
    fn test_type_selector() {
        let tree = parse_html("<div><p>Hello</p></div>");
        let p_nodes = tree.get_elements_by_tag_name("p");
        let sel = Selector::parse("p").unwrap();

        assert!(matches_selector(&tree, p_nodes[0], &sel));
    }

    #[test]
    fn test_class_selector() {
        let tree = parse_html("<div class='container'>Hello</div>");
        let divs = tree.get_elements_by_tag_name("div");

        let sel = Selector::parse(".container").unwrap();
        assert!(matches_selector(&tree, divs[0], &sel));

        let sel2 = Selector::parse(".other").unwrap();
        assert!(!matches_selector(&tree, divs[0], &sel2));
    }

    #[test]
    fn test_id_selector() {
        let tree = parse_html("<div id='main'>Hello</div>");
        let div = tree.get_element_by_id("main").unwrap();

        let sel = Selector::parse("#main").unwrap();
        assert!(matches_selector(&tree, div, &sel));
    }

    #[test]
    fn test_compound_selector() {
        let tree = parse_html("<div id='main' class='container'>Hello</div>");
        let div = tree.get_element_by_id("main").unwrap();

        let sel = Selector::parse("div.container#main").unwrap();
        assert!(matches_selector(&tree, div, &sel));
    }

    #[test]
    fn test_descendant_combinator() {
        let tree = parse_html("<div><section><p>Hello</p></section></div>");
        let p_nodes = tree.get_elements_by_tag_name("p");

        let sel = Selector::parse("div p").unwrap();
        assert!(matches_selector(&tree, p_nodes[0], &sel));

        let sel2 = Selector::parse("section p").unwrap();
        assert!(matches_selector(&tree, p_nodes[0], &sel2));
    }

    #[test]
    fn test_child_combinator() {
        let tree = parse_html("<div><p>Direct</p><section><p>Nested</p></section></div>");
        let p_nodes = tree.get_elements_by_tag_name("p");

        let sel = Selector::parse("div > p").unwrap();
        // First p is direct child of div
        assert!(matches_selector(&tree, p_nodes[0], &sel));
        // Second p is child of section, not direct child of div
        assert!(!matches_selector(&tree, p_nodes[1], &sel));
    }

    #[test]
    fn test_attribute_selector() {
        let tree = parse_html("<input type='text'><input type='password'>");
        let inputs = tree.get_elements_by_tag_name("input");

        let sel = Selector::parse("[type='text']").unwrap();
        assert!(matches_selector(&tree, inputs[0], &sel));
        assert!(!matches_selector(&tree, inputs[1], &sel));
    }

    #[test]
    fn test_first_child() {
        let tree = parse_html("<ul><li>First</li><li>Second</li><li>Third</li></ul>");
        let lis = tree.get_elements_by_tag_name("li");

        let sel = Selector::parse("li:first-child").unwrap();
        assert!(matches_selector(&tree, lis[0], &sel));
        assert!(!matches_selector(&tree, lis[1], &sel));
        assert!(!matches_selector(&tree, lis[2], &sel));
    }

    #[test]
    fn test_last_child() {
        let tree = parse_html("<ul><li>First</li><li>Second</li><li>Third</li></ul>");
        let lis = tree.get_elements_by_tag_name("li");

        let sel = Selector::parse("li:last-child").unwrap();
        assert!(!matches_selector(&tree, lis[0], &sel));
        assert!(!matches_selector(&tree, lis[1], &sel));
        assert!(matches_selector(&tree, lis[2], &sel));
    }

    #[test]
    fn test_nth_child() {
        let tree = parse_html("<ul><li>1</li><li>2</li><li>3</li><li>4</li><li>5</li></ul>");
        let lis = tree.get_elements_by_tag_name("li");

        // :nth-child(2)
        let sel = Selector::parse("li:nth-child(2)").unwrap();
        assert!(!matches_selector(&tree, lis[0], &sel));
        assert!(matches_selector(&tree, lis[1], &sel));
        assert!(!matches_selector(&tree, lis[2], &sel));

        // :nth-child(odd)
        let sel_odd = Selector::parse("li:nth-child(odd)").unwrap();
        assert!(matches_selector(&tree, lis[0], &sel_odd)); // 1
        assert!(!matches_selector(&tree, lis[1], &sel_odd)); // 2
        assert!(matches_selector(&tree, lis[2], &sel_odd)); // 3

        // :nth-child(even)
        let sel_even = Selector::parse("li:nth-child(even)").unwrap();
        assert!(!matches_selector(&tree, lis[0], &sel_even)); // 1
        assert!(matches_selector(&tree, lis[1], &sel_even)); // 2
        assert!(!matches_selector(&tree, lis[2], &sel_even)); // 3
    }

    #[test]
    fn test_nth_child_formula() {
        let tree = parse_html("<ul><li>1</li><li>2</li><li>3</li><li>4</li><li>5</li><li>6</li></ul>");
        let lis = tree.get_elements_by_tag_name("li");

        // :nth-child(2n) = 2, 4, 6
        let sel = Selector::parse("li:nth-child(2n)").unwrap();
        assert!(!matches_selector(&tree, lis[0], &sel)); // 1
        assert!(matches_selector(&tree, lis[1], &sel)); // 2
        assert!(!matches_selector(&tree, lis[2], &sel)); // 3
        assert!(matches_selector(&tree, lis[3], &sel)); // 4

        // :nth-child(2n+1) = 1, 3, 5
        let sel2 = Selector::parse("li:nth-child(2n+1)").unwrap();
        assert!(matches_selector(&tree, lis[0], &sel2)); // 1
        assert!(!matches_selector(&tree, lis[1], &sel2)); // 2
        assert!(matches_selector(&tree, lis[2], &sel2)); // 3
    }

    #[test]
    fn test_not_selector() {
        let tree = parse_html("<ul><li class='active'>A</li><li>B</li></ul>");
        let lis = tree.get_elements_by_tag_name("li");

        let sel = Selector::parse("li:not(.active)").unwrap();
        assert!(!matches_selector(&tree, lis[0], &sel));
        assert!(matches_selector(&tree, lis[1], &sel));
    }

    #[test]
    fn test_empty_selector() {
        let tree = parse_html("<div></div><div>Not empty</div>");
        let divs = tree.get_elements_by_tag_name("div");

        let sel = Selector::parse("div:empty").unwrap();
        assert!(matches_selector(&tree, divs[0], &sel));
        assert!(!matches_selector(&tree, divs[1], &sel));
    }

    #[test]
    fn test_universal_selector() {
        let tree = parse_html("<div><p>Hello</p></div>");
        let p_nodes = tree.get_elements_by_tag_name("p");

        let sel = Selector::parse("*").unwrap();
        assert!(matches_selector(&tree, p_nodes[0], &sel));
    }

    #[test]
    fn test_sibling_combinator() {
        let tree = parse_html("<div><h1>Title</h1><p>First</p><p>Second</p></div>");
        let p_nodes = tree.get_elements_by_tag_name("p");

        // h1 + p should match first p
        let sel = Selector::parse("h1 + p").unwrap();
        assert!(matches_selector(&tree, p_nodes[0], &sel));
        assert!(!matches_selector(&tree, p_nodes[1], &sel));
    }

    #[test]
    fn test_subsequent_sibling() {
        let tree = parse_html("<div><h1>Title</h1><span>Span</span><p>Para</p></div>");
        let p_nodes = tree.get_elements_by_tag_name("p");

        // h1 ~ p should match p (any sibling after h1)
        let sel = Selector::parse("h1 ~ p").unwrap();
        assert!(matches_selector(&tree, p_nodes[0], &sel));
    }
}

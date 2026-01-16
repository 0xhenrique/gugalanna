//! Block Layout
//!
//! Implements the CSS block formatting context layout algorithm.

use crate::boxtree::LayoutBox;
use crate::flex::layout_flex;
use crate::inline::layout_inline_children;
use crate::ContainingBlock;
use gugalanna_style::{Display, Position};

/// Layout a block-level element and its descendants
pub fn layout_block(
    layout_box: &mut LayoutBox,
    containing_block: ContainingBlock,
) {
    // Calculate width first (depends on containing block)
    calculate_block_width(layout_box, containing_block);

    // Calculate position within containing block
    calculate_block_position(layout_box, containing_block);

    // Layout children and calculate height
    layout_block_children(layout_box);

    // Height calculation (may be auto)
    calculate_block_height(layout_box);
}

/// Calculate the width of a block element
fn calculate_block_width(layout_box: &mut LayoutBox, containing_block: ContainingBlock) {
    let style = match layout_box.style() {
        Some(s) => s,
        None => {
            // Anonymous block - take full width
            layout_box.dimensions.content.width = containing_block.width;
            return;
        }
    };

    // Copy edge sizes from style
    layout_box.apply_style_edges();

    // Get the specified width or auto
    let width = style.width;

    let d = &mut layout_box.dimensions;

    // Total horizontal space used by margins, padding, borders
    let total_horizontal = d.margin.horizontal()
        + d.padding.horizontal()
        + d.border.horizontal();

    // Calculate content width
    let content_width = match width {
        Some(w) => w,
        None => {
            // Auto width - fill available space
            (containing_block.width - total_horizontal).max(0.0)
        }
    };

    d.content.width = content_width;

    // Handle auto margins for centering
    let underflow = containing_block.width - content_width - total_horizontal;
    if underflow > 0.0 && width.is_some() {
        // Check if both margins are auto (for centering)
        if style.margin_left == 0.0 && style.margin_right == 0.0 {
            // Could implement auto margin centering here
            // For now, just add underflow to right margin
            d.margin.right += underflow;
        }
    }
}

/// Calculate the position of a block element
fn calculate_block_position(layout_box: &mut LayoutBox, _containing_block: ContainingBlock) {
    let d = &mut layout_box.dimensions;

    // Vertical position: margin-top + border-top + padding-top
    // X position starts at containing block x + margin-left + border-left + padding-left
    d.content.x = d.margin.left + d.border.left + d.padding.left;

    // Y position will be set by parent during child layout
    // For now, just account for top edges
    d.content.y = d.margin.top + d.border.top + d.padding.top;

    // Apply relative positioning offset
    if let Some(style) = layout_box.style() {
        if style.position == Position::Relative {
            // Apply left/right offset (left takes precedence)
            if let Some(left) = style.left {
                layout_box.dimensions.content.x += left;
            } else if let Some(right) = style.right {
                layout_box.dimensions.content.x -= right;
            }

            // Apply top/bottom offset (top takes precedence)
            if let Some(top) = style.top {
                layout_box.dimensions.content.y += top;
            } else if let Some(bottom) = style.bottom {
                layout_box.dimensions.content.y -= bottom;
            }
        }
    }
}

/// Layout all children of a block element
fn layout_block_children(layout_box: &mut LayoutBox) {
    // Check if this is a flex container
    if let Some(style) = layout_box.style() {
        if style.display == Display::Flex {
            // Use flex layout
            let containing = ContainingBlock::new(
                layout_box.dimensions.content.width,
                layout_box.style().and_then(|s| s.height).unwrap_or(0.0),
            );
            layout_flex(layout_box, containing);
            return;
        }
    }

    // Separate block and inline children
    let has_block_children = layout_box.children.iter().any(|c| c.is_block());

    if has_block_children {
        // Block formatting context
        layout_block_children_as_blocks(layout_box);
    } else {
        // All inline - create inline formatting context
        layout_inline_children(layout_box);
    }
}

/// Layout children in block formatting context
fn layout_block_children_as_blocks(layout_box: &mut LayoutBox) {
    let content_width = layout_box.dimensions.content.width;
    let containing = ContainingBlock::new(content_width, 0.0);

    let mut cursor_y = 0.0;

    for child in &mut layout_box.children {
        if child.is_block() {
            // Layout this block child
            layout_block(child, containing);

            // Position it vertically
            child.dimensions.content.y += cursor_y;

            // Move cursor down
            cursor_y += child.dimensions.margin_box_height();
        } else {
            // Inline content in block context - should be wrapped in anonymous block
            // Just lay it out as inline
            layout_inline_children(child);
            child.dimensions.content.y = cursor_y;
            cursor_y += child.dimensions.margin_box_height();
        }
    }
}

/// Calculate the height of a block element
fn calculate_block_height(layout_box: &mut LayoutBox) {
    // Check for explicit height
    if let Some(style) = layout_box.style() {
        if let Some(h) = style.height {
            layout_box.dimensions.content.height = h;
            return;
        }
    }

    // Auto height - sum of children's margin boxes
    let children_height: f32 = layout_box
        .children
        .iter()
        .map(|c| c.dimensions.margin_box_height())
        .sum();

    layout_box.dimensions.content.height = children_height;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxtree::build_layout_tree;
    use gugalanna_css::Stylesheet;
    use gugalanna_html::HtmlParser;
    use gugalanna_style::{Cascade, StyleTree};
    use gugalanna_dom::Queryable;

    fn setup_and_layout(html: &str, css: &str, width: f32) -> LayoutBox<'static> {
        // We need to leak memory for tests because LayoutBox has lifetime tied to StyleTree
        let dom = Box::leak(Box::new(HtmlParser::new().parse(html).unwrap()));
        let mut cascade = Cascade::new();
        if !css.is_empty() {
            cascade.add_author_stylesheet(Stylesheet::parse(css).unwrap());
        }
        let style_tree = Box::leak(Box::new(StyleTree::build(dom, &cascade, 1024.0, 768.0)));

        // Find the first div element (our test target)
        let div_ids = dom.get_elements_by_tag_name("div");
        let root_id = if !div_ids.is_empty() {
            div_ids[0]
        } else {
            // Fallback to body
            dom.get_elements_by_tag_name("body")[0]
        };

        let mut layout = build_layout_tree(dom, style_tree, root_id).unwrap();
        layout_block(&mut layout, ContainingBlock::new(width, 600.0));
        layout
    }

    #[test]
    fn test_block_width_fill() {
        let layout = setup_and_layout(
            "<div>test</div>",
            "div { display: block; }",
            800.0,
        );

        // Block should fill container width
        assert_eq!(layout.dimensions.content.width, 800.0);
    }

    #[test]
    fn test_block_explicit_width() {
        let layout = setup_and_layout(
            "<div>test</div>",
            "div { display: block; width: 400px; }",
            800.0,
        );

        assert_eq!(layout.dimensions.content.width, 400.0);
    }

    #[test]
    fn test_block_with_padding() {
        let layout = setup_and_layout(
            "<div>test</div>",
            "div { display: block; padding-top: 10px; padding-right: 10px; padding-bottom: 10px; padding-left: 10px; }",
            800.0,
        );

        assert_eq!(layout.dimensions.padding.top, 10.0);
        assert_eq!(layout.dimensions.padding.left, 10.0);
        // Content width should be reduced by padding
        assert_eq!(layout.dimensions.content.width, 780.0);
    }

    #[test]
    fn test_nested_blocks_height() {
        let layout = setup_and_layout(
            "<div><p>Line 1</p><p>Line 2</p></div>",
            "div, p { display: block; }",
            800.0,
        );

        // Height should be sum of children
        assert!(layout.dimensions.content.height > 0.0);
    }

    #[test]
    fn test_block_with_margin() {
        let layout = setup_and_layout(
            "<div>test</div>",
            "div { display: block; margin-top: 20px; margin-right: 20px; margin-bottom: 20px; margin-left: 20px; }",
            800.0,
        );

        assert_eq!(layout.dimensions.margin.top, 20.0);
        assert_eq!(layout.dimensions.margin.left, 20.0);
        // Content width reduced by margins
        assert_eq!(layout.dimensions.content.width, 760.0);
    }
}

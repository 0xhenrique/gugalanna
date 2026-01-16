//! Inline Layout
//!
//! Implements inline formatting context and line box layout.

use crate::boxtree::{LayoutBox, BoxType, InputType};
use crate::text::measure_text;
use crate::Rect;

/// A line box containing inline content
#[derive(Debug)]
pub struct LineBox {
    /// Baseline position (y coordinate)
    pub baseline: f32,
    /// Line height
    pub height: f32,
    /// Starting x position
    pub x: f32,
    /// Width of content on this line
    pub width: f32,
}

impl LineBox {
    pub fn new(x: f32, baseline: f32, height: f32) -> Self {
        Self {
            baseline,
            height,
            x,
            width: 0.0,
        }
    }
}

/// An inline box fragment (part of an inline element on one line)
#[derive(Debug)]
pub struct InlineBox {
    /// Position and dimensions
    pub rect: Rect,
    /// Associated text (if text box)
    pub text: Option<String>,
}

impl InlineBox {
    pub fn new(rect: Rect, text: Option<String>) -> Self {
        Self { rect, text }
    }
}

/// Layout inline children of a block element
pub fn layout_inline_children(parent: &mut LayoutBox) {
    let available_width = parent.dimensions.content.width;

    // Track current position
    let mut cursor_x = 0.0;
    let mut cursor_y = 0.0;
    let mut line_height = 0.0_f32;
    let mut max_width = 0.0_f32;

    for child in &mut parent.children {
        let (child_width, child_height) = layout_inline_box(child, available_width - cursor_x);

        // Check if we need to wrap to next line
        if cursor_x + child_width > available_width && cursor_x > 0.0 {
            // Start new line
            cursor_y += line_height;
            cursor_x = 0.0;
            line_height = 0.0;
        }

        // Position this inline box
        child.dimensions.content.x = cursor_x;
        child.dimensions.content.y = cursor_y;

        // Update cursor
        cursor_x += child_width;
        max_width = max_width.max(cursor_x);
        line_height = line_height.max(child_height);
    }

    // Final line
    cursor_y += line_height;

    // Set parent dimensions based on inline content
    // For inline elements (which set width to f32::MAX), shrink-wrap to content
    if parent.dimensions.content.width == f32::MAX || parent.dimensions.content.width == 0.0 {
        parent.dimensions.content.width = max_width;
    }
    if parent.dimensions.content.height == 0.0 {
        parent.dimensions.content.height = cursor_y;
    }
}

/// Layout a single inline box, returns (width, height)
fn layout_inline_box(layout_box: &mut LayoutBox, _available_width: f32) -> (f32, f32) {
    match &layout_box.box_type {
        BoxType::Text(_, text, style) => {
            // Measure text
            let metrics = measure_text(text, style);
            layout_box.dimensions.content.width = metrics.width;
            layout_box.dimensions.content.height = metrics.height;
            (metrics.width, metrics.height)
        }
        BoxType::Inline(_, _style) => {
            // Apply style edges
            layout_box.apply_style_edges();

            // For inline elements, set a large available width so children don't wrap
            // The inline element will shrink-wrap to its content
            layout_box.dimensions.content.width = f32::MAX;

            // Layout children
            layout_inline_children(layout_box);

            let width = layout_box.dimensions.margin_box_width();
            let height = layout_box.dimensions.margin_box_height();
            (width, height)
        }
        BoxType::AnonymousInline | BoxType::AnonymousBlock => {
            // Layout children
            layout_inline_children(layout_box);

            let width = layout_box.dimensions.content.width;
            let height = layout_box.dimensions.content.height;
            (width, height)
        }
        BoxType::Block(_, _) => {
            // Block inside inline - treat as inline-block
            // This shouldn't happen in well-formed content
            (0.0, 0.0)
        }
        BoxType::Input(_, input_type, _) => {
            // Form input elements have intrinsic dimensions
            // Copy input_type before mutable borrow
            let input_type = *input_type;
            layout_box.apply_style_edges();

            let (width, height) = input_intrinsic_size(input_type);
            layout_box.dimensions.content.width = width;
            layout_box.dimensions.content.height = height;

            (
                layout_box.dimensions.margin_box_width(),
                layout_box.dimensions.margin_box_height(),
            )
        }
        BoxType::Button(_, label, _) => {
            // Button size based on label text - get style from layout_box
            // Clone label before mutable borrow
            let label = label.clone();
            layout_box.apply_style_edges();

            let style = layout_box.style().unwrap();
            let metrics = measure_text(&label, style);
            // Add padding for button appearance
            let width = metrics.width + 16.0; // 8px padding on each side
            let height = metrics.height.max(24.0); // Minimum height of 24px

            layout_box.dimensions.content.width = width;
            layout_box.dimensions.content.height = height;

            (
                layout_box.dimensions.margin_box_width(),
                layout_box.dimensions.margin_box_height(),
            )
        }
    }
}

/// Get intrinsic size for a form input based on type
fn input_intrinsic_size(input_type: InputType) -> (f32, f32) {
    match input_type {
        InputType::Text | InputType::Password => {
            // Default text input size
            (200.0, 24.0)
        }
        InputType::Checkbox | InputType::Radio => {
            // Small square/circle for checkboxes and radios
            (16.0, 16.0)
        }
        InputType::Submit | InputType::Button => {
            // Button with default size
            (80.0, 24.0)
        }
        InputType::Hidden => {
            // Hidden inputs have no size
            (0.0, 0.0)
        }
    }
}

/// Split text into words for line breaking
pub fn split_into_words(text: &str) -> Vec<&str> {
    text.split_whitespace().collect()
}

/// Calculate line box height from inline content
pub fn calculate_line_height(boxes: &[LayoutBox]) -> f32 {
    boxes
        .iter()
        .map(|b| {
            if let Some(style) = b.style() {
                style.line_height
            } else {
                b.dimensions.content.height
            }
        })
        .fold(0.0_f32, |a, b| a.max(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_words() {
        let words = split_into_words("hello world foo");
        assert_eq!(words, vec!["hello", "world", "foo"]);
    }

    #[test]
    fn test_split_words_multiple_spaces() {
        let words = split_into_words("hello    world");
        assert_eq!(words, vec!["hello", "world"]);
    }

    #[test]
    fn test_line_box_creation() {
        let line = LineBox::new(0.0, 12.0, 16.0);
        assert_eq!(line.x, 0.0);
        assert_eq!(line.baseline, 12.0);
        assert_eq!(line.height, 16.0);
    }
}

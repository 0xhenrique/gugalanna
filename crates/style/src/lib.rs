//! Gugalanna Style Engine
//!
//! Style computation, cascade, and selector matching.

// TODO: Epic 4 - Style Computation
// - Selector matching
// - Specificity calculation
// - Cascade algorithm
// - Inheritance
// - Computed values

use gugalanna_css::Color;

/// Computed style for an element
#[derive(Debug, Clone)]
pub struct ComputedStyle {
    // Display
    pub display: Display,

    // Box model
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub margin_top: f32,
    pub margin_right: f32,
    pub margin_bottom: f32,
    pub margin_left: f32,
    pub padding_top: f32,
    pub padding_right: f32,
    pub padding_bottom: f32,
    pub padding_left: f32,
    pub border_top_width: f32,
    pub border_right_width: f32,
    pub border_bottom_width: f32,
    pub border_left_width: f32,

    // Colors
    pub color: Color,
    pub background_color: Color,
    pub border_color: Color,

    // Text
    pub font_size: f32,
    pub font_family: String,
    pub font_weight: u16,
    pub line_height: f32,
    pub text_align: TextAlign,

    // Position
    pub position: Position,
    pub top: Option<f32>,
    pub right: Option<f32>,
    pub bottom: Option<f32>,
    pub left: Option<f32>,
}

/// Display property values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    None,
    Block,
    Inline,
    InlineBlock,
    Flex,
}

/// Position property values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    Static,
    Relative,
    Absolute,
    Fixed,
}

/// Text alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Right,
    Center,
    Justify,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            display: Display::Inline,
            width: None,
            height: None,
            margin_top: 0.0,
            margin_right: 0.0,
            margin_bottom: 0.0,
            margin_left: 0.0,
            padding_top: 0.0,
            padding_right: 0.0,
            padding_bottom: 0.0,
            padding_left: 0.0,
            border_top_width: 0.0,
            border_right_width: 0.0,
            border_bottom_width: 0.0,
            border_left_width: 0.0,
            color: Color::black(),
            background_color: Color::transparent(),
            border_color: Color::black(),
            font_size: 16.0,
            font_family: String::from("sans-serif"),
            font_weight: 400,
            line_height: 1.2,
            text_align: TextAlign::Left,
            position: Position::Static,
            top: None,
            right: None,
            bottom: None,
            left: None,
        }
    }
}

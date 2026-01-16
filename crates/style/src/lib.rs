//! Gugalanna Style Engine
//!
//! Style computation, cascade, and selector matching.

pub mod matching;
pub mod cascade;
pub mod properties;
pub mod resolver;
pub mod styletree;

use gugalanna_css::Color;

pub use matching::matches_selector;
pub use cascade::{Cascade, Origin, MatchedDeclaration, default_ua_stylesheet};
pub use properties::{Inheritance, is_inherited, get_inheritance};
pub use resolver::{ResolveContext, StyleResolver};
pub use styletree::StyleTree;

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

    // Colors and background
    pub color: Color,
    pub background: Background,
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

    // Stacking and overflow
    pub z_index: i32,
    pub overflow: Overflow,
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,

    // Visual effects
    pub opacity: f32,
    pub box_shadow: Option<BoxShadow>,
    pub border_radius: BorderRadius,

    // Flex container properties
    pub flex_direction: FlexDirection,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,

    // Flex item properties
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Option<f32>,  // None = auto
    pub align_self: AlignSelf,
    pub order: i32,

    // Transitions
    pub transitions: Vec<TransitionDef>,
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

/// Overflow property values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Scroll,
    Auto,
}

/// Flex direction property
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexDirection {
    #[default]
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

/// Justify content (main axis alignment)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JustifyContent {
    #[default]
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

/// Align items (cross axis alignment)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignItems {
    FlexStart,
    FlexEnd,
    Center,
    #[default]
    Stretch,
    Baseline,
}

/// Align self (per-item cross axis override)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignSelf {
    #[default]
    Auto,
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
    Baseline,
}

/// Easing function for CSS transitions
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TimingFunction {
    #[default]
    Ease,
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    CubicBezier(f32, f32, f32, f32),
}

/// A single transition definition
#[derive(Debug, Clone, Default)]
pub struct TransitionDef {
    /// Property to transition ("all" or specific property name)
    pub property: String,
    /// Duration in milliseconds
    pub duration_ms: f32,
    /// Delay before starting in milliseconds
    pub delay_ms: f32,
    /// Easing function
    pub timing_function: TimingFunction,
}

/// Box shadow effect
#[derive(Debug, Clone, Default)]
pub struct BoxShadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur_radius: f32,
    pub spread_radius: f32,
    pub color: Color,
    pub inset: bool,
}

/// Border radius for rounded corners
#[derive(Debug, Clone, Copy, Default)]
pub struct BorderRadius {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl BorderRadius {
    /// Check if any corner has a radius
    pub fn has_radius(&self) -> bool {
        self.top_left > 0.0
            || self.top_right > 0.0
            || self.bottom_right > 0.0
            || self.bottom_left > 0.0
    }
}

/// A color stop in a gradient
#[derive(Debug, Clone)]
pub struct ColorStop {
    pub color: Color,
    pub position: Option<f32>, // 0.0 to 1.0, None = auto-distribute
}

/// Linear gradient direction
#[derive(Debug, Clone)]
pub enum GradientDirection {
    Angle(f32),         // Degrees (0 = to top, 90 = to right)
    ToTop,
    ToBottom,
    ToLeft,
    ToRight,
    ToTopLeft,
    ToTopRight,
    ToBottomLeft,
    ToBottomRight,
}

impl Default for GradientDirection {
    fn default() -> Self {
        GradientDirection::ToBottom // CSS default
    }
}

/// Radial gradient shape
#[derive(Debug, Clone, Copy, Default)]
pub enum RadialShape {
    #[default]
    Ellipse,
    Circle,
}

/// Radial gradient size
#[derive(Debug, Clone, Copy, Default)]
pub enum RadialSize {
    #[default]
    FarthestCorner,
    ClosestSide,
    ClosestCorner,
    FarthestSide,
}

/// A CSS gradient
#[derive(Debug, Clone)]
pub enum Gradient {
    Linear {
        direction: GradientDirection,
        stops: Vec<ColorStop>,
    },
    Radial {
        shape: RadialShape,
        size: RadialSize,
        center_x: f32, // 0.0 to 1.0, default 0.5
        center_y: f32, // 0.0 to 1.0, default 0.5
        stops: Vec<ColorStop>,
    },
}

/// Background can be a solid color or gradient
#[derive(Debug, Clone)]
pub enum Background {
    Color(Color),
    Gradient(Gradient),
}

impl Default for Background {
    fn default() -> Self {
        Background::Color(Color::transparent())
    }
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
            background: Background::default(),
            border_color: Color::black(),
            font_size: 16.0,
            font_family: String::from("sans-serif"),
            font_weight: 400,
            line_height: 19.2, // 16.0 * 1.2
            text_align: TextAlign::Left,
            position: Position::Static,
            top: None,
            right: None,
            bottom: None,
            left: None,
            z_index: 0,
            overflow: Overflow::Visible,
            overflow_x: Overflow::Visible,
            overflow_y: Overflow::Visible,
            opacity: 1.0,
            box_shadow: None,
            border_radius: BorderRadius::default(),

            // Flex container defaults
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Stretch,

            // Flex item defaults
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: None,
            align_self: AlignSelf::Auto,
            order: 0,

            // Transition defaults
            transitions: Vec::new(),
        }
    }
}

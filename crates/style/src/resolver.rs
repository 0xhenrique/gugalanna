//! Style Resolver
//!
//! Resolves CSS values to computed values, handling inheritance,
//! relative units, and keyword values.

use gugalanna_css::{CssValue, Color, LengthUnit};

use crate::properties::is_inherited;
use crate::{
    AlignItems, AlignSelf, Background, BorderRadius, BoxShadow, ColorStop, ComputedStyle,
    Display, FlexDirection, Gradient, GradientDirection, JustifyContent, Overflow, Position,
    RadialShape, RadialSize, TextAlign, TimingFunction, TransitionDef,
};

/// Context for resolving styles
#[derive(Debug, Clone)]
pub struct ResolveContext {
    /// Parent's computed style (for inheritance)
    pub parent_style: Option<ComputedStyle>,
    /// Root font size (for rem units)
    pub root_font_size: f32,
    /// Viewport width (for vw units)
    pub viewport_width: f32,
    /// Viewport height (for vh units)
    pub viewport_height: f32,
}

impl Default for ResolveContext {
    fn default() -> Self {
        Self {
            parent_style: None,
            root_font_size: 16.0,
            viewport_width: 1024.0,
            viewport_height: 768.0,
        }
    }
}

impl ResolveContext {
    /// Create a new resolve context
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the parent style
    pub fn with_parent(mut self, parent: ComputedStyle) -> Self {
        self.parent_style = Some(parent);
        self
    }

    /// Set viewport dimensions
    pub fn with_viewport(mut self, width: f32, height: f32) -> Self {
        self.viewport_width = width;
        self.viewport_height = height;
        self
    }

    /// Set root font size
    pub fn with_root_font_size(mut self, size: f32) -> Self {
        self.root_font_size = size;
        self
    }

    /// Get the current font size (from parent or default)
    pub fn font_size(&self) -> f32 {
        self.parent_style.as_ref()
            .map(|s| s.font_size)
            .unwrap_or(16.0)
    }
}

/// Style value resolver
pub struct StyleResolver;

impl StyleResolver {
    /// Resolve a CSS value to a computed length in pixels
    pub fn resolve_length(
        value: &CssValue,
        context: &ResolveContext,
    ) -> Option<f32> {
        match value {
            CssValue::Length(n, unit) => {
                let font_size = context.font_size();
                Some(unit.to_px(
                    *n,
                    font_size,
                    context.root_font_size,
                    context.viewport_width,
                    context.viewport_height,
                ))
            }
            CssValue::Number(n) => {
                // Bare numbers are treated as px for length properties
                Some(*n)
            }
            CssValue::Percentage(_) => {
                // Percentage of containing block - caller needs to handle this
                // For now, return None to indicate it needs special handling
                None
            }
            CssValue::Keyword(k) if k == "0" => Some(0.0),
            CssValue::Keyword(k) if k == "auto" => None,
            _ => None,
        }
    }

    /// Resolve a CSS color value
    pub fn resolve_color(
        value: &CssValue,
        context: &ResolveContext,
    ) -> Option<Color> {
        match value {
            CssValue::Color(c) => Some(*c),
            CssValue::Keyword(k) if k == "currentColor" || k == "currentcolor" => {
                // Use the computed color from parent
                context.parent_style.as_ref()
                    .map(|s| s.color)
                    .or_else(|| Some(Color::black()))
            }
            CssValue::Keyword(k) if k == "inherit" => {
                context.parent_style.as_ref()
                    .map(|s| s.color)
            }
            CssValue::Keyword(k) if k == "transparent" => {
                Some(Color::transparent())
            }
            CssValue::Keyword(k) => {
                Color::from_name(k)
            }
            _ => None,
        }
    }

    /// Resolve display value
    pub fn resolve_display(value: &CssValue) -> Option<Display> {
        match value {
            CssValue::Keyword(k) => match k.to_ascii_lowercase().as_str() {
                "none" => Some(Display::None),
                "block" => Some(Display::Block),
                "inline" => Some(Display::Inline),
                "inline-block" => Some(Display::InlineBlock),
                "flex" => Some(Display::Flex),
                "list-item" => Some(Display::Block), // Simplified
                "table" | "table-row" | "table-cell" |
                "table-row-group" | "table-header-group" |
                "table-footer-group" | "table-column" |
                "table-column-group" | "table-caption" => Some(Display::Block), // Simplified
                _ => None,
            },
            _ => None,
        }
    }

    /// Resolve position value
    pub fn resolve_position(value: &CssValue) -> Option<Position> {
        match value {
            CssValue::Keyword(k) => match k.to_ascii_lowercase().as_str() {
                "static" => Some(Position::Static),
                "relative" => Some(Position::Relative),
                "absolute" => Some(Position::Absolute),
                "fixed" => Some(Position::Fixed),
                _ => None,
            },
            _ => None,
        }
    }

    /// Resolve text-align value
    pub fn resolve_text_align(value: &CssValue) -> Option<TextAlign> {
        match value {
            CssValue::Keyword(k) => match k.to_ascii_lowercase().as_str() {
                "left" => Some(TextAlign::Left),
                "right" => Some(TextAlign::Right),
                "center" => Some(TextAlign::Center),
                "justify" => Some(TextAlign::Justify),
                _ => None,
            },
            _ => None,
        }
    }

    /// Resolve z-index value
    pub fn resolve_z_index(value: &CssValue) -> Option<i32> {
        match value {
            CssValue::Number(n) => Some(*n as i32),
            CssValue::Keyword(k) if k == "auto" => Some(0),
            _ => None,
        }
    }

    /// Resolve overflow value
    pub fn resolve_overflow(value: &CssValue) -> Option<Overflow> {
        match value {
            CssValue::Keyword(k) => match k.to_ascii_lowercase().as_str() {
                "visible" => Some(Overflow::Visible),
                "hidden" => Some(Overflow::Hidden),
                "scroll" => Some(Overflow::Scroll),
                "auto" => Some(Overflow::Auto),
                _ => None,
            },
            _ => None,
        }
    }

    /// Resolve flex-direction value
    pub fn resolve_flex_direction(value: &CssValue) -> Option<FlexDirection> {
        match value {
            CssValue::Keyword(k) => match k.to_ascii_lowercase().as_str() {
                "row" => Some(FlexDirection::Row),
                "row-reverse" => Some(FlexDirection::RowReverse),
                "column" => Some(FlexDirection::Column),
                "column-reverse" => Some(FlexDirection::ColumnReverse),
                _ => None,
            },
            _ => None,
        }
    }

    /// Resolve justify-content value
    pub fn resolve_justify_content(value: &CssValue) -> Option<JustifyContent> {
        match value {
            CssValue::Keyword(k) => match k.to_ascii_lowercase().as_str() {
                "flex-start" | "start" => Some(JustifyContent::FlexStart),
                "flex-end" | "end" => Some(JustifyContent::FlexEnd),
                "center" => Some(JustifyContent::Center),
                "space-between" => Some(JustifyContent::SpaceBetween),
                "space-around" => Some(JustifyContent::SpaceAround),
                "space-evenly" => Some(JustifyContent::SpaceEvenly),
                _ => None,
            },
            _ => None,
        }
    }

    /// Resolve align-items value
    pub fn resolve_align_items(value: &CssValue) -> Option<AlignItems> {
        match value {
            CssValue::Keyword(k) => match k.to_ascii_lowercase().as_str() {
                "flex-start" | "start" => Some(AlignItems::FlexStart),
                "flex-end" | "end" => Some(AlignItems::FlexEnd),
                "center" => Some(AlignItems::Center),
                "stretch" => Some(AlignItems::Stretch),
                "baseline" => Some(AlignItems::Baseline),
                _ => None,
            },
            _ => None,
        }
    }

    /// Resolve align-self value
    pub fn resolve_align_self(value: &CssValue) -> Option<AlignSelf> {
        match value {
            CssValue::Keyword(k) => match k.to_ascii_lowercase().as_str() {
                "auto" => Some(AlignSelf::Auto),
                "flex-start" | "start" => Some(AlignSelf::FlexStart),
                "flex-end" | "end" => Some(AlignSelf::FlexEnd),
                "center" => Some(AlignSelf::Center),
                "stretch" => Some(AlignSelf::Stretch),
                "baseline" => Some(AlignSelf::Baseline),
                _ => None,
            },
            _ => None,
        }
    }

    /// Resolve flex-grow value (non-negative number)
    pub fn resolve_flex_grow(value: &CssValue) -> Option<f32> {
        match value {
            CssValue::Number(n) if *n >= 0.0 => Some(*n),
            _ => None,
        }
    }

    /// Resolve flex-shrink value (non-negative number)
    pub fn resolve_flex_shrink(value: &CssValue) -> Option<f32> {
        match value {
            CssValue::Number(n) if *n >= 0.0 => Some(*n),
            _ => None,
        }
    }

    /// Resolve flex-basis value (length, percentage, or auto)
    pub fn resolve_flex_basis(value: &CssValue, context: &ResolveContext) -> Option<Option<f32>> {
        match value {
            CssValue::Keyword(k) => {
                let lower = k.to_ascii_lowercase();
                if lower == "auto" || lower == "content" {
                    Some(None) // auto
                } else {
                    None
                }
            }
            _ => Self::resolve_length(value, context).map(Some),
        }
    }

    /// Resolve order value (integer)
    pub fn resolve_order(value: &CssValue) -> Option<i32> {
        match value {
            CssValue::Number(n) => Some(*n as i32),
            _ => None,
        }
    }

    /// Resolve a time value to milliseconds
    pub fn resolve_time_ms(value: &CssValue) -> Option<f32> {
        match value {
            CssValue::Time(n, unit) => Some(unit.to_ms(*n)),
            CssValue::Number(n) if *n == 0.0 => Some(0.0), // 0 without unit is valid
            _ => None,
        }
    }

    /// Resolve timing-function value
    pub fn resolve_timing_function(value: &CssValue) -> Option<TimingFunction> {
        match value {
            CssValue::Keyword(k) => match k.to_ascii_lowercase().as_str() {
                "linear" => Some(TimingFunction::Linear),
                "ease" => Some(TimingFunction::Ease),
                "ease-in" => Some(TimingFunction::EaseIn),
                "ease-out" => Some(TimingFunction::EaseOut),
                "ease-in-out" => Some(TimingFunction::EaseInOut),
                _ => None,
            },
            CssValue::Function(name, args) if name == "cubic-bezier" => {
                // Parse 4 number arguments
                let nums: Vec<f32> = args
                    .iter()
                    .filter_map(|v| {
                        if let CssValue::Number(n) = v {
                            Some(*n)
                        } else {
                            None
                        }
                    })
                    .collect();
                if nums.len() == 4 {
                    Some(TimingFunction::CubicBezier(nums[0], nums[1], nums[2], nums[3]))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Resolve transition shorthand value
    /// Format: property duration [timing-function] [delay]
    /// Example: "width 0.3s ease 0.1s" or "all 300ms linear"
    pub fn resolve_transition(value: &CssValue) -> Option<Vec<TransitionDef>> {
        // Handle comma-separated multiple transitions
        let transition_lists = match value {
            CssValue::CommaSeparated(items) => items.clone(),
            _ => vec![value.clone()],
        };

        let mut transitions = Vec::new();

        for item in transition_lists {
            if let Some(def) = Self::resolve_single_transition(&item) {
                transitions.push(def);
            }
        }

        if transitions.is_empty() {
            None
        } else {
            Some(transitions)
        }
    }

    /// Resolve a single transition definition
    fn resolve_single_transition(value: &CssValue) -> Option<TransitionDef> {
        let values = match value {
            CssValue::List(v) => v.clone(),
            _ => vec![value.clone()],
        };

        let mut def = TransitionDef::default();
        def.property = "all".to_string(); // Default
        let mut found_duration = false;

        for v in values {
            // Try as time value
            if let Some(time_ms) = Self::resolve_time_ms(&v) {
                if !found_duration {
                    def.duration_ms = time_ms;
                    found_duration = true;
                } else {
                    // Second time value is delay
                    def.delay_ms = time_ms;
                }
                continue;
            }

            // Try as timing function
            if let Some(timing) = Self::resolve_timing_function(&v) {
                def.timing_function = timing;
                continue;
            }

            // Try as property name (keyword)
            if let CssValue::Keyword(k) = &v {
                let lower = k.to_ascii_lowercase();
                // Check if it's not a timing function keyword
                if !matches!(
                    lower.as_str(),
                    "linear" | "ease" | "ease-in" | "ease-out" | "ease-in-out"
                ) {
                    def.property = lower;
                }
            }
        }

        // Need at least a duration for a valid transition
        if found_duration {
            Some(def)
        } else {
            None
        }
    }

    /// Resolve opacity value (0.0 to 1.0)
    pub fn resolve_opacity(value: &CssValue) -> Option<f32> {
        match value {
            CssValue::Number(n) => Some(n.clamp(0.0, 1.0)),
            CssValue::Percentage(p) => Some((p / 100.0).clamp(0.0, 1.0)),
            _ => None,
        }
    }

    /// Resolve box-shadow value
    /// Format: [inset] offset-x offset-y [blur-radius] [spread-radius] [color]
    pub fn resolve_box_shadow(value: &CssValue, context: &ResolveContext) -> Option<BoxShadow> {
        let values = match value {
            CssValue::List(v) => v.clone(),
            _ => vec![value.clone()],
        };

        let mut shadow = BoxShadow::default();
        shadow.color = Color::rgba(0, 0, 0, 128); // Default: semi-transparent black
        let mut length_idx = 0;

        for v in &values {
            // Check for 'inset' keyword
            if let CssValue::Keyword(k) = v {
                if k.to_ascii_lowercase() == "inset" {
                    shadow.inset = true;
                    continue;
                }
                // Check if it's a color name
                if let Some(color) = Color::from_name(k) {
                    shadow.color = color;
                    continue;
                }
            }

            // Check for color value
            if let CssValue::Color(c) = v {
                shadow.color = *c;
                continue;
            }

            // Check for function (like rgba())
            if let CssValue::Function(_, _) = v {
                if let Some(color) = Self::resolve_color(v, context) {
                    shadow.color = color;
                    continue;
                }
            }

            // Try to parse as length
            if let Some(len) = Self::resolve_length(v, context) {
                match length_idx {
                    0 => shadow.offset_x = len,
                    1 => shadow.offset_y = len,
                    2 => shadow.blur_radius = len.max(0.0),
                    3 => shadow.spread_radius = len,
                    _ => {}
                }
                length_idx += 1;
            }
        }

        // Need at least offset-x and offset-y
        if length_idx >= 2 {
            Some(shadow)
        } else {
            None
        }
    }

    /// Resolve border-radius value
    /// 1 value: all corners
    /// 2 values: (top-left/bottom-right, top-right/bottom-left)
    /// 3 values: (top-left, top-right/bottom-left, bottom-right)
    /// 4 values: (top-left, top-right, bottom-right, bottom-left)
    pub fn resolve_border_radius(value: &CssValue, context: &ResolveContext) -> Option<BorderRadius> {
        let lengths: Vec<f32> = match value {
            CssValue::List(values) => {
                values.iter().filter_map(|v| Self::resolve_length(v, context)).collect()
            }
            _ => {
                if let Some(len) = Self::resolve_length(value, context) {
                    vec![len]
                } else {
                    return None;
                }
            }
        };

        let radius = match lengths.len() {
            1 => BorderRadius {
                top_left: lengths[0],
                top_right: lengths[0],
                bottom_right: lengths[0],
                bottom_left: lengths[0],
            },
            2 => BorderRadius {
                top_left: lengths[0],
                top_right: lengths[1],
                bottom_right: lengths[0],
                bottom_left: lengths[1],
            },
            3 => BorderRadius {
                top_left: lengths[0],
                top_right: lengths[1],
                bottom_right: lengths[2],
                bottom_left: lengths[1],
            },
            4 => BorderRadius {
                top_left: lengths[0],
                top_right: lengths[1],
                bottom_right: lengths[2],
                bottom_left: lengths[3],
            },
            _ => return None,
        };

        Some(radius)
    }

    /// Resolve a background value (color or gradient)
    pub fn resolve_background(value: &CssValue, context: &ResolveContext) -> Option<Background> {
        // Try as gradient first
        if let Some(gradient) = Self::resolve_gradient(value, context) {
            return Some(Background::Gradient(gradient));
        }

        // Fall back to color
        if let Some(color) = Self::resolve_color(value, context) {
            return Some(Background::Color(color));
        }

        None
    }

    /// Resolve a gradient from a CssValue::Function
    pub fn resolve_gradient(value: &CssValue, context: &ResolveContext) -> Option<Gradient> {
        match value {
            CssValue::Function(name, args) => {
                match name.to_ascii_lowercase().as_str() {
                    "linear-gradient" => Self::parse_linear_gradient(args, context),
                    "radial-gradient" => Self::parse_radial_gradient(args, context),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Parse a linear-gradient() function
    /// Syntax: linear-gradient([angle | to direction], color-stop, color-stop, ...)
    fn parse_linear_gradient(args: &[CssValue], context: &ResolveContext) -> Option<Gradient> {
        if args.is_empty() {
            return None;
        }

        let mut direction = GradientDirection::ToBottom; // CSS default
        let mut start_idx = 0;

        // Check if first arg is a direction
        if let Some(first) = args.first() {
            if let Some(dir) = Self::parse_gradient_direction(first) {
                direction = dir;
                start_idx = 1;
            }
        }

        // Parse color stops
        let mut stops = Vec::new();
        for arg in &args[start_idx..] {
            if let Some(stop) = Self::parse_color_stop(arg, context) {
                stops.push(stop);
            }
        }

        // Need at least 2 color stops
        if stops.len() < 2 {
            return None;
        }

        Some(Gradient::Linear { direction, stops })
    }

    /// Parse a radial-gradient() function
    /// Syntax: radial-gradient([shape] [size] [at position], color-stop, color-stop, ...)
    fn parse_radial_gradient(args: &[CssValue], context: &ResolveContext) -> Option<Gradient> {
        if args.is_empty() {
            return None;
        }

        let mut shape = RadialShape::Ellipse;
        let mut size = RadialSize::FarthestCorner;
        let center_x = 0.5;
        let center_y = 0.5;
        let mut start_idx = 0;

        // Check for shape/size keywords in first argument
        if let Some(first) = args.first() {
            if let CssValue::Keyword(k) = first {
                match k.to_ascii_lowercase().as_str() {
                    "circle" => {
                        shape = RadialShape::Circle;
                        start_idx = 1;
                    }
                    "ellipse" => {
                        shape = RadialShape::Ellipse;
                        start_idx = 1;
                    }
                    "closest-side" => {
                        size = RadialSize::ClosestSide;
                        start_idx = 1;
                    }
                    "closest-corner" => {
                        size = RadialSize::ClosestCorner;
                        start_idx = 1;
                    }
                    "farthest-side" => {
                        size = RadialSize::FarthestSide;
                        start_idx = 1;
                    }
                    "farthest-corner" => {
                        size = RadialSize::FarthestCorner;
                        start_idx = 1;
                    }
                    _ => {}
                }
            }
        }

        // Parse color stops
        let mut stops = Vec::new();
        for arg in &args[start_idx..] {
            if let Some(stop) = Self::parse_color_stop(arg, context) {
                stops.push(stop);
            }
        }

        // Need at least 2 color stops
        if stops.len() < 2 {
            return None;
        }

        Some(Gradient::Radial {
            shape,
            size,
            center_x,
            center_y,
            stops,
        })
    }

    /// Parse gradient direction from keyword or angle
    fn parse_gradient_direction(value: &CssValue) -> Option<GradientDirection> {
        match value {
            CssValue::Keyword(k) => {
                let lower = k.to_ascii_lowercase();
                // Check for angle (e.g., "45deg", "90deg")
                if lower.ends_with("deg") {
                    if let Ok(degrees) = lower.trim_end_matches("deg").parse::<f32>() {
                        return Some(GradientDirection::Angle(degrees));
                    }
                }
                // Check for directional keywords
                match lower.as_str() {
                    "to top" => Some(GradientDirection::ToTop),
                    "to bottom" => Some(GradientDirection::ToBottom),
                    "to left" => Some(GradientDirection::ToLeft),
                    "to right" => Some(GradientDirection::ToRight),
                    "to top left" | "to left top" => Some(GradientDirection::ToTopLeft),
                    "to top right" | "to right top" => Some(GradientDirection::ToTopRight),
                    "to bottom left" | "to left bottom" => Some(GradientDirection::ToBottomLeft),
                    "to bottom right" | "to right bottom" => Some(GradientDirection::ToBottomRight),
                    _ => None,
                }
            }
            CssValue::Number(n) => {
                // Bare number treated as degrees
                Some(GradientDirection::Angle(*n))
            }
            _ => None,
        }
    }

    /// Parse a color stop (color with optional position)
    fn parse_color_stop(value: &CssValue, context: &ResolveContext) -> Option<ColorStop> {
        match value {
            // Simple color
            CssValue::Color(c) => Some(ColorStop {
                color: *c,
                position: None,
            }),
            CssValue::Keyword(k) => {
                // Could be a color name
                Color::from_name(k).map(|color| ColorStop {
                    color,
                    position: None,
                })
            }
            // Color function (rgb, rgba, etc)
            CssValue::Function(_, _) => {
                Self::resolve_color(value, context).map(|color| ColorStop {
                    color,
                    position: None,
                })
            }
            // List: color followed by position
            CssValue::List(items) => {
                if items.is_empty() {
                    return None;
                }
                let color = Self::resolve_color(&items[0], context)?;
                let position = if items.len() > 1 {
                    Self::parse_stop_position(&items[1])
                } else {
                    None
                };
                Some(ColorStop { color, position })
            }
            _ => None,
        }
    }

    /// Parse a stop position (percentage or length as fraction)
    fn parse_stop_position(value: &CssValue) -> Option<f32> {
        match value {
            CssValue::Percentage(p) => Some(*p / 100.0),
            CssValue::Length(_n, LengthUnit::Px) => {
                // For px, we'd need the gradient length - for now, skip
                None
            }
            CssValue::Number(n) => {
                // Bare number interpreted as percentage if 0-1 range
                if *n >= 0.0 && *n <= 1.0 {
                    Some(*n)
                } else {
                    Some(*n / 100.0)
                }
            }
            _ => None,
        }
    }

    /// Resolve font-weight value
    pub fn resolve_font_weight(value: &CssValue) -> Option<u16> {
        match value {
            CssValue::Number(n) => Some(*n as u16),
            CssValue::Keyword(k) => match k.to_ascii_lowercase().as_str() {
                "normal" => Some(400),
                "bold" => Some(700),
                "lighter" => Some(100), // Simplified
                "bolder" => Some(900),  // Simplified
                _ => k.parse().ok(),
            },
            _ => None,
        }
    }

    /// Resolve font-size value (returns pixels)
    pub fn resolve_font_size(
        value: &CssValue,
        context: &ResolveContext,
    ) -> Option<f32> {
        match value {
            CssValue::Length(n, unit) => {
                let parent_font_size = context.font_size();
                Some(unit.to_px(
                    *n,
                    parent_font_size,
                    context.root_font_size,
                    context.viewport_width,
                    context.viewport_height,
                ))
            }
            CssValue::Percentage(p) => {
                let parent_font_size = context.font_size();
                Some(parent_font_size * p / 100.0)
            }
            CssValue::Number(n) => Some(*n),
            CssValue::Keyword(k) => {
                let base = 16.0; // Base font size
                match k.to_ascii_lowercase().as_str() {
                    "xx-small" => Some(base * 0.5),
                    "x-small" => Some(base * 0.625),
                    "small" => Some(base * 0.833),
                    "medium" => Some(base),
                    "large" => Some(base * 1.2),
                    "x-large" => Some(base * 1.5),
                    "xx-large" => Some(base * 2.0),
                    "smaller" => {
                        let parent = context.font_size();
                        Some(parent * 0.833)
                    }
                    "larger" => {
                        let parent = context.font_size();
                        Some(parent * 1.2)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Resolve line-height value
    pub fn resolve_line_height(
        value: &CssValue,
        context: &ResolveContext,
    ) -> Option<f32> {
        match value {
            CssValue::Number(n) => {
                // Unitless number is a multiplier of font-size
                Some(context.font_size() * n)
            }
            CssValue::Length(n, unit) => {
                let font_size = context.font_size();
                Some(unit.to_px(
                    *n,
                    font_size,
                    context.root_font_size,
                    context.viewport_width,
                    context.viewport_height,
                ))
            }
            CssValue::Percentage(p) => {
                Some(context.font_size() * p / 100.0)
            }
            CssValue::Keyword(k) if k == "normal" => {
                Some(context.font_size() * 1.2)
            }
            _ => None,
        }
    }

    /// Get the inherited value for a property
    pub fn get_inherited_value(
        property: &str,
        context: &ResolveContext,
    ) -> Option<CssValue> {
        let parent = context.parent_style.as_ref()?;

        match property.to_ascii_lowercase().as_str() {
            "color" => Some(CssValue::Color(parent.color)),
            "font-size" => Some(CssValue::Length(parent.font_size, LengthUnit::Px)),
            "font-weight" => Some(CssValue::Number(parent.font_weight as f32)),
            "line-height" => Some(CssValue::Length(parent.line_height, LengthUnit::Px)),
            "font-family" => Some(CssValue::Keyword(parent.font_family.clone())),
            "text-align" => {
                let value = match parent.text_align {
                    TextAlign::Left => "left",
                    TextAlign::Right => "right",
                    TextAlign::Center => "center",
                    TextAlign::Justify => "justify",
                };
                Some(CssValue::Keyword(value.to_string()))
            }
            _ => None,
        }
    }

    /// Check if a value is 'inherit'
    pub fn is_inherit(value: &CssValue) -> bool {
        matches!(value, CssValue::Keyword(k) if k == "inherit")
    }

    /// Check if a value is 'initial'
    pub fn is_initial(value: &CssValue) -> bool {
        matches!(value, CssValue::Keyword(k) if k == "initial")
    }

    /// Check if a value is 'unset'
    pub fn is_unset(value: &CssValue) -> bool {
        matches!(value, CssValue::Keyword(k) if k == "unset")
    }

    /// Resolve a value considering inherit/initial/unset keywords
    pub fn resolve_keyword_value(
        property: &str,
        value: &CssValue,
        context: &ResolveContext,
    ) -> Option<CssValue> {
        if Self::is_inherit(value) {
            return Self::get_inherited_value(property, context);
        }

        if Self::is_initial(value) {
            // Return None to indicate initial value should be used
            return None;
        }

        if Self::is_unset(value) {
            // For inherited properties, acts like inherit
            // For non-inherited properties, acts like initial
            if is_inherited(property) {
                return Self::get_inherited_value(property, context);
            } else {
                return None;
            }
        }

        Some(value.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_length_px() {
        let ctx = ResolveContext::default();
        let value = CssValue::Length(100.0, LengthUnit::Px);
        assert_eq!(StyleResolver::resolve_length(&value, &ctx), Some(100.0));
    }

    #[test]
    fn test_resolve_length_em() {
        let mut parent = ComputedStyle::default();
        parent.font_size = 20.0;
        let ctx = ResolveContext::default().with_parent(parent);

        let value = CssValue::Length(2.0, LengthUnit::Em);
        assert_eq!(StyleResolver::resolve_length(&value, &ctx), Some(40.0));
    }

    #[test]
    fn test_resolve_length_rem() {
        let ctx = ResolveContext::default().with_root_font_size(20.0);
        let value = CssValue::Length(2.0, LengthUnit::Rem);
        assert_eq!(StyleResolver::resolve_length(&value, &ctx), Some(40.0));
    }

    #[test]
    fn test_resolve_color() {
        let ctx = ResolveContext::default();
        let value = CssValue::Color(Color::rgb(255, 0, 0));
        assert_eq!(StyleResolver::resolve_color(&value, &ctx), Some(Color::rgb(255, 0, 0)));
    }

    #[test]
    fn test_resolve_color_keyword() {
        let ctx = ResolveContext::default();
        let value = CssValue::Keyword("red".to_string());
        assert_eq!(StyleResolver::resolve_color(&value, &ctx), Some(Color::rgb(255, 0, 0)));
    }

    #[test]
    fn test_resolve_display() {
        assert_eq!(
            StyleResolver::resolve_display(&CssValue::Keyword("block".to_string())),
            Some(Display::Block)
        );
        assert_eq!(
            StyleResolver::resolve_display(&CssValue::Keyword("none".to_string())),
            Some(Display::None)
        );
    }

    #[test]
    fn test_resolve_font_weight() {
        assert_eq!(
            StyleResolver::resolve_font_weight(&CssValue::Keyword("bold".to_string())),
            Some(700)
        );
        assert_eq!(
            StyleResolver::resolve_font_weight(&CssValue::Number(600.0)),
            Some(600)
        );
    }

    #[test]
    fn test_resolve_font_size() {
        let ctx = ResolveContext::default();

        // Absolute keyword
        let value = CssValue::Keyword("large".to_string());
        let result = StyleResolver::resolve_font_size(&value, &ctx);
        assert!(result.is_some());
        assert!((result.unwrap() - 19.2).abs() < 0.1); // 16 * 1.2
    }

    #[test]
    fn test_inherit_keyword() {
        let mut parent = ComputedStyle::default();
        parent.color = Color::rgb(255, 0, 0);
        let ctx = ResolveContext::default().with_parent(parent);

        let value = CssValue::Keyword("inherit".to_string());
        let resolved = StyleResolver::resolve_keyword_value("color", &value, &ctx);
        assert!(resolved.is_some());
        if let Some(CssValue::Color(c)) = resolved {
            assert_eq!(c, Color::rgb(255, 0, 0));
        } else {
            panic!("Expected Color value");
        }
    }
}

//! Style Resolver
//!
//! Resolves CSS values to computed values, handling inheritance,
//! relative units, and keyword values.

use gugalanna_css::{CssValue, Color, LengthUnit};

use crate::properties::is_inherited;
use crate::{ComputedStyle, Display, Position, TextAlign};

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

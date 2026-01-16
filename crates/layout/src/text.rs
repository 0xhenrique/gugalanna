//! Text Measurement
//!
//! Interface for measuring text dimensions.

use gugalanna_style::ComputedStyle;

/// Text metrics for layout
#[derive(Debug, Clone, Copy)]
pub struct TextMetrics {
    /// Width of the text
    pub width: f32,
    /// Height of the text (typically line height)
    pub height: f32,
    /// Ascent (distance from baseline to top)
    pub ascent: f32,
    /// Descent (distance from baseline to bottom)
    pub descent: f32,
}

impl TextMetrics {
    /// Create new text metrics
    pub fn new(width: f32, height: f32, ascent: f32, descent: f32) -> Self {
        Self { width, height, ascent, descent }
    }
}

/// Trait for measuring text
pub trait TextMeasurer {
    /// Measure the dimensions of a text string with the given style
    fn measure(&self, text: &str, style: &ComputedStyle) -> TextMetrics;
}

/// Simple text measurer using fixed-width estimation
///
/// This is a placeholder until we have real font rendering.
/// It uses a simple heuristic based on font size.
#[derive(Debug, Default)]
pub struct SimpleTextMeasurer;

impl SimpleTextMeasurer {
    pub fn new() -> Self {
        Self
    }
}

impl TextMeasurer for SimpleTextMeasurer {
    fn measure(&self, text: &str, style: &ComputedStyle) -> TextMetrics {
        // Simple heuristic: average character width is ~0.6 * font size
        // This is a rough approximation for proportional fonts
        let char_width = style.font_size * 0.6;
        let width = text.chars().count() as f32 * char_width;

        // Line height from style
        let height = style.line_height;

        // Approximate ascent/descent
        let ascent = style.font_size * 0.8;
        let descent = style.font_size * 0.2;

        TextMetrics { width, height, ascent, descent }
    }
}

/// Measure text width using the simple measurer
pub fn measure_text_width(text: &str, style: &ComputedStyle) -> f32 {
    let measurer = SimpleTextMeasurer::new();
    measurer.measure(text, style).width
}

/// Measure full text metrics using the simple measurer
pub fn measure_text(text: &str, style: &ComputedStyle) -> TextMetrics {
    let measurer = SimpleTextMeasurer::new();
    measurer.measure(text, style)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_measurement() {
        let mut style = ComputedStyle::default();
        style.font_size = 16.0;
        style.line_height = 20.0;

        let metrics = measure_text("Hello", &style);

        // 5 chars * 16 * 0.6 = 48
        assert!((metrics.width - 48.0).abs() < 0.1);
        assert_eq!(metrics.height, 20.0);
    }

    #[test]
    fn test_empty_text() {
        let style = ComputedStyle::default();
        let metrics = measure_text("", &style);

        assert_eq!(metrics.width, 0.0);
    }
}

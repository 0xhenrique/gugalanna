//! Paint primitives
//!
//! Basic types for rendering.

use gugalanna_css::Color;

/// Color for rendering (RGBA)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl RenderColor {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 255)
    }

    pub fn black() -> Self {
        Self::new(0, 0, 0, 255)
    }

    pub fn white() -> Self {
        Self::new(255, 255, 255, 255)
    }

    pub fn transparent() -> Self {
        Self::new(0, 0, 0, 0)
    }

    /// Check if color is fully transparent
    pub fn is_transparent(&self) -> bool {
        self.a == 0
    }
}

impl From<Color> for RenderColor {
    fn from(c: Color) -> Self {
        Self::new(c.r, c.g, c.b, c.a)
    }
}

impl Default for RenderColor {
    fn default() -> Self {
        Self::black()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_color() {
        let c = RenderColor::rgb(255, 0, 0);
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 0);
        assert_eq!(c.a, 255);
    }

    #[test]
    fn test_from_css_color() {
        let css = Color::rgb(100, 150, 200);
        let render: RenderColor = css.into();
        assert_eq!(render.r, 100);
        assert_eq!(render.g, 150);
        assert_eq!(render.b, 200);
    }

    #[test]
    fn test_transparent() {
        let t = RenderColor::transparent();
        assert!(t.is_transparent());

        let o = RenderColor::black();
        assert!(!o.is_transparent());
    }
}

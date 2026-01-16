//! Font rendering
//!
//! Text rendering using fontdue.

use fontdue::{Font, FontSettings};
use std::collections::HashMap;

/// Default embedded font (DejaVu Sans Mono subset or similar)
/// For now, we'll use a built-in font from the system or embed one.
const DEFAULT_FONT_DATA: &[u8] = include_bytes!("fonts/DejaVuSans.ttf");

/// Cache for rendered glyphs
pub struct FontCache {
    font: Font,
    glyph_cache: HashMap<GlyphKey, GlyphData>,
}

/// Key for cached glyphs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GlyphKey {
    character: char,
    size_tenths: u32, // Font size * 10 to avoid float hashing
}

/// Cached glyph bitmap data
#[derive(Debug, Clone)]
pub struct GlyphData {
    pub width: u32,
    pub height: u32,
    pub bitmap: Vec<u8>,
    pub advance_width: f32,
    pub offset_x: i32,
    pub offset_y: i32,
}

impl FontCache {
    /// Create a new font cache with the default font
    pub fn new() -> Self {
        let font = Font::from_bytes(DEFAULT_FONT_DATA, FontSettings::default())
            .expect("Failed to load default font");

        Self {
            font,
            glyph_cache: HashMap::new(),
        }
    }

    /// Create a font cache from font data
    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        let font = Font::from_bytes(data, FontSettings::default())
            .map_err(|_| "Failed to parse font data")?;

        Ok(Self {
            font,
            glyph_cache: HashMap::new(),
        })
    }

    /// Rasterize a character at a given size
    pub fn rasterize(&mut self, c: char, size: f32) -> &GlyphData {
        let key = GlyphKey {
            character: c,
            size_tenths: (size * 10.0) as u32,
        };

        if !self.glyph_cache.contains_key(&key) {
            let (metrics, bitmap) = self.font.rasterize(c, size);

            let glyph = GlyphData {
                width: metrics.width as u32,
                height: metrics.height as u32,
                bitmap,
                advance_width: metrics.advance_width,
                offset_x: metrics.xmin,
                offset_y: metrics.ymin,
            };

            self.glyph_cache.insert(key, glyph);
        }

        self.glyph_cache.get(&key).unwrap()
    }

    /// Measure the width of a string
    pub fn measure_text(&mut self, text: &str, size: f32) -> f32 {
        text.chars()
            .map(|c| self.rasterize(c, size).advance_width)
            .sum()
    }

    /// Get line metrics for a font size
    pub fn line_height(&self, size: f32) -> f32 {
        let metrics = self.font.horizontal_line_metrics(size);
        match metrics {
            Some(m) => m.new_line_size,
            None => size * 1.2,
        }
    }

    /// Get the ascent for a font size
    pub fn ascent(&self, size: f32) -> f32 {
        let metrics = self.font.horizontal_line_metrics(size);
        match metrics {
            Some(m) => m.ascent,
            None => size * 0.8,
        }
    }
}

impl Default for FontCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_cache_creation() {
        let cache = FontCache::new();
        assert!(cache.glyph_cache.is_empty());
    }

    #[test]
    fn test_rasterize_char() {
        let mut cache = FontCache::new();
        let glyph = cache.rasterize('A', 16.0);
        assert!(glyph.width > 0);
        assert!(glyph.height > 0);
        assert!(!glyph.bitmap.is_empty());
    }

    #[test]
    fn test_measure_text() {
        let mut cache = FontCache::new();
        let width = cache.measure_text("Hello", 16.0);
        assert!(width > 0.0);
    }

    #[test]
    fn test_glyph_caching() {
        let mut cache = FontCache::new();

        // First call should populate cache
        cache.rasterize('X', 20.0);
        assert_eq!(cache.glyph_cache.len(), 1);

        // Second call should use cache
        cache.rasterize('X', 20.0);
        assert_eq!(cache.glyph_cache.len(), 1);

        // Different size should add new entry
        cache.rasterize('X', 24.0);
        assert_eq!(cache.glyph_cache.len(), 2);
    }
}

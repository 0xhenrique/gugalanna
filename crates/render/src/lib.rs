//! Gugalanna Render Engine
//!
//! Painting and display list generation.

mod display_list;
mod paint;
mod sdl_backend;
mod font;

pub use display_list::{DisplayList, PaintCommand, BorderWidths, build_display_list};
pub use paint::RenderColor;
pub use sdl_backend::{SdlBackend, CursorType};
pub use font::{FontCache, GlyphData};

/// Trait for render backends
pub trait RenderBackend {
    /// Clear the screen with a color
    fn clear(&mut self, color: RenderColor);

    /// Execute a display list
    fn render(&mut self, display_list: &DisplayList);

    /// Present the rendered frame
    fn present(&mut self);

    /// Get the window width
    fn width(&self) -> u32;

    /// Get the window height
    fn height(&self) -> u32;
}

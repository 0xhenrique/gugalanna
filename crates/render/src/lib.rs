//! Gugalanna Render Engine
//!
//! Painting and display list generation.

// TODO: Epic 6 - Rendering
// - Paint commands
// - Display list
// - SDL backend
// - Text rendering

use gugalanna_layout::Rect;

/// A display list of paint commands
#[derive(Debug, Default)]
pub struct DisplayList {
    pub commands: Vec<PaintCommand>,
}

/// A paint command
#[derive(Debug)]
pub enum PaintCommand {
    /// Fill a rectangle with a solid color
    FillRect {
        rect: Rect,
        color: RenderColor,
    },
    /// Draw text
    DrawText {
        text: String,
        x: f32,
        y: f32,
        color: RenderColor,
        font_size: f32,
    },
    /// Draw a border
    DrawBorder {
        rect: Rect,
        width: f32,
        color: RenderColor,
    },
    /// Draw an image
    DrawImage {
        rect: Rect,
        image_data: Vec<u8>,
    },
}

/// Color for rendering (RGBA)
#[derive(Debug, Clone, Copy)]
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

    pub fn black() -> Self {
        Self::new(0, 0, 0, 255)
    }

    pub fn white() -> Self {
        Self::new(255, 255, 255, 255)
    }
}

/// Trait for render backends
pub trait RenderBackend {
    /// Clear the screen with a color
    fn clear(&mut self, color: RenderColor);

    /// Execute a display list
    fn render(&mut self, display_list: &DisplayList);

    /// Present the rendered frame
    fn present(&mut self);
}

impl DisplayList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, command: PaintCommand) {
        self.commands.push(command);
    }
}

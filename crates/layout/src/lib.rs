//! Gugalanna Layout Engine
//!
//! Box model and layout algorithms.

mod boxtree;
mod block;
mod inline;
mod text;

pub use boxtree::{LayoutBox, BoxType, InputType, build_layout_tree};
pub use block::layout_block;
pub use inline::{LineBox, InlineBox};
pub use text::TextMetrics;

/// Box dimensions
#[derive(Debug, Clone, Copy, Default)]
pub struct Dimensions {
    /// Content area
    pub content: Rect,
    /// Padding
    pub padding: EdgeSizes,
    /// Border
    pub border: EdgeSizes,
    /// Margin
    pub margin: EdgeSizes,
}

/// A rectangle
#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Edge sizes (top, right, bottom, left)
#[derive(Debug, Clone, Copy, Default)]
pub struct EdgeSizes {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Dimensions {
    /// Get the total width including padding, border, and margin
    pub fn margin_box_width(&self) -> f32 {
        self.content.width
            + self.padding.left + self.padding.right
            + self.border.left + self.border.right
            + self.margin.left + self.margin.right
    }

    /// Get the total height including padding, border, and margin
    pub fn margin_box_height(&self) -> f32 {
        self.content.height
            + self.padding.top + self.padding.bottom
            + self.border.top + self.border.bottom
            + self.margin.top + self.margin.bottom
    }

    /// Get the padding box rectangle
    pub fn padding_box(&self) -> Rect {
        Rect {
            x: self.content.x - self.padding.left,
            y: self.content.y - self.padding.top,
            width: self.content.width + self.padding.left + self.padding.right,
            height: self.content.height + self.padding.top + self.padding.bottom,
        }
    }

    /// Get the border box rectangle
    pub fn border_box(&self) -> Rect {
        let padding = self.padding_box();
        Rect {
            x: padding.x - self.border.left,
            y: padding.y - self.border.top,
            width: padding.width + self.border.left + self.border.right,
            height: padding.height + self.border.top + self.border.bottom,
        }
    }

    /// Get the margin box rectangle
    pub fn margin_box(&self) -> Rect {
        let border = self.border_box();
        Rect {
            x: border.x - self.margin.left,
            y: border.y - self.margin.top,
            width: border.width + self.margin.left + self.margin.right,
            height: border.height + self.margin.top + self.margin.bottom,
        }
    }
}

impl Rect {
    /// Create a new rectangle
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    /// Check if a point is inside the rectangle
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    /// Get the right edge
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    /// Get the bottom edge
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }
}

impl EdgeSizes {
    /// Create edge sizes with all sides equal
    pub fn all(size: f32) -> Self {
        Self {
            top: size,
            right: size,
            bottom: size,
            left: size,
        }
    }

    /// Total horizontal size
    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    /// Total vertical size
    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }
}

/// Containing block for layout
#[derive(Debug, Clone, Copy)]
pub struct ContainingBlock {
    pub width: f32,
    pub height: f32,
}

impl ContainingBlock {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

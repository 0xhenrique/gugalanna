//! Gugalanna Layout Engine
//!
//! Box model and layout algorithms.

// TODO: Epic 5 - Layout Engine
// - Box model
// - Block layout
// - Inline layout
// - Text measurement

/// A layout box
#[derive(Debug)]
pub struct LayoutBox {
    /// Box dimensions
    pub dimensions: Dimensions,
    /// Box type
    pub box_type: BoxType,
    /// Child boxes
    pub children: Vec<LayoutBox>,
}

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

/// Type of layout box
#[derive(Debug)]
pub enum BoxType {
    Block,
    Inline,
    Anonymous,
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
}

impl Rect {
    /// Check if a point is inside the rectangle
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}

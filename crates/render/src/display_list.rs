//! Display List
//!
//! Converts layout tree to paint commands.

use gugalanna_dom::NodeId;
use gugalanna_layout::{LayoutBox, BoxType, InputType, ImagePixels, Rect};
use gugalanna_style::{Background, BorderRadius, BoxShadow, ColorStop, Gradient, GradientDirection, Overflow, RadialShape, RadialSize};

use crate::paint::RenderColor;

/// A display list of paint commands
#[derive(Debug, Default, Clone)]
pub struct DisplayList {
    pub commands: Vec<PaintCommand>,
}

/// A paint command
#[derive(Debug, Clone)]
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
    /// Draw a border (outline of rectangle)
    DrawBorder {
        rect: Rect,
        widths: BorderWidths,
        color: RenderColor,
    },
    /// Draw a text input field
    DrawTextInput {
        node_id: NodeId,
        rect: Rect,
        text: String,
        cursor_pos: Option<usize>,
        is_password: bool,
        is_focused: bool,
    },
    /// Draw a checkbox
    DrawCheckbox {
        node_id: NodeId,
        rect: Rect,
        checked: bool,
        is_focused: bool,
    },
    /// Draw a radio button
    DrawRadio {
        node_id: NodeId,
        rect: Rect,
        checked: bool,
        is_focused: bool,
    },
    /// Draw a button
    DrawButton {
        node_id: NodeId,
        rect: Rect,
        text: String,
        is_pressed: bool,
    },
    /// Draw an image
    DrawImage {
        rect: Rect,
        /// Decoded RGBA pixel data (None if not loaded or failed)
        pixels: Option<ImagePixels>,
        /// Alt text for placeholder display
        alt: String,
    },
    /// Set clipping rectangle (for overflow: hidden)
    SetClipRect(Rect),
    /// Clear clipping rectangle
    ClearClipRect,
    /// Push an opacity modifier (affects all subsequent commands until PopOpacity)
    PushOpacity(f32),
    /// Pop the current opacity modifier
    PopOpacity,
    /// Draw a box shadow
    DrawBoxShadow {
        rect: Rect,
        shadow: BoxShadow,
    },
    /// Fill a rounded rectangle
    FillRoundedRect {
        rect: Rect,
        radius: BorderRadius,
        color: RenderColor,
    },
    /// Draw a rounded border
    DrawRoundedBorder {
        rect: Rect,
        radius: BorderRadius,
        widths: BorderWidths,
        color: RenderColor,
    },
    /// Fill a rectangle with a linear gradient
    FillLinearGradient {
        rect: Rect,
        direction: GradientDirection,
        stops: Vec<ColorStop>,
        radius: Option<BorderRadius>,
    },
    /// Fill a rectangle with a radial gradient
    FillRadialGradient {
        rect: Rect,
        shape: RadialShape,
        size: RadialSize,
        center_x: f32,
        center_y: f32,
        stops: Vec<ColorStop>,
        radius: Option<BorderRadius>,
    },
}

/// Border widths for all four sides
#[derive(Debug, Clone, Copy, Default)]
pub struct BorderWidths {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl DisplayList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, command: PaintCommand) {
        self.commands.push(command);
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

/// Build a display list from a layout box tree
pub fn build_display_list(layout_root: &LayoutBox) -> DisplayList {
    let mut list = DisplayList::new();
    render_layout_box(&mut list, layout_root, 0.0, 0.0);
    list
}

/// Recursively render a layout box and its children
/// offset_x and offset_y are the absolute position of the parent's content area
fn render_layout_box(list: &mut DisplayList, layout_box: &LayoutBox, offset_x: f32, offset_y: f32) {
    let d = &layout_box.dimensions;

    // Calculate absolute position of this box's content area
    let abs_x = offset_x + d.content.x;
    let abs_y = offset_y + d.content.y;

    // Check if we need to apply opacity
    let needs_opacity = layout_box.style().map_or(false, |s| s.opacity < 1.0);
    if needs_opacity {
        let opacity = layout_box.style().map(|s| s.opacity).unwrap_or(1.0);
        list.push(PaintCommand::PushOpacity(opacity));
    }

    // Render box-shadow first (behind everything)
    render_box_shadow(list, layout_box, offset_x, offset_y);

    // Render this box's background and borders
    render_background(list, layout_box, offset_x, offset_y);
    render_borders(list, layout_box, offset_x, offset_y);

    // Render content (text)
    render_content(list, layout_box, abs_x, abs_y);

    // Check if we need to clip overflow
    let needs_clip = layout_box.style().map_or(false, |s| {
        s.overflow != Overflow::Visible ||
        s.overflow_x != Overflow::Visible ||
        s.overflow_y != Overflow::Visible
    });

    if needs_clip {
        // Set clip rect to the content area of this box
        let clip_rect = Rect::new(abs_x, abs_y, d.content.width, d.content.height);
        list.push(PaintCommand::SetClipRect(clip_rect));
    }

    // Sort children by z-index before rendering
    let mut children_sorted: Vec<_> = layout_box.children.iter().collect();
    children_sorted.sort_by_key(|child| {
        child.style().map(|s| s.z_index).unwrap_or(0)
    });

    // Render children - they are positioned relative to this box's content area
    for child in children_sorted {
        render_layout_box(list, child, abs_x, abs_y);
    }

    if needs_clip {
        list.push(PaintCommand::ClearClipRect);
    }

    if needs_opacity {
        list.push(PaintCommand::PopOpacity);
    }
}

/// Render box shadow for a layout box
fn render_box_shadow(list: &mut DisplayList, layout_box: &LayoutBox, offset_x: f32, offset_y: f32) {
    let style = match layout_box.style() {
        Some(s) => s,
        None => return,
    };

    let shadow = match &style.box_shadow {
        Some(s) => s,
        None => return,
    };

    // Skip inset shadows for now (more complex to render)
    if shadow.inset {
        return;
    }

    let d = &layout_box.dimensions;
    let border_box = d.border_box();

    let rect = Rect::new(
        offset_x + border_box.x,
        offset_y + border_box.y,
        border_box.width,
        border_box.height,
    );

    list.push(PaintCommand::DrawBoxShadow {
        rect,
        shadow: shadow.clone(),
    });
}

/// Render the background of a layout box
fn render_background(list: &mut DisplayList, layout_box: &LayoutBox, offset_x: f32, offset_y: f32) {
    let style = match layout_box.style() {
        Some(s) => s,
        None => return,
    };

    let d = &layout_box.dimensions;
    let border_box = d.border_box();

    // Adjust to absolute position
    let rect = Rect::new(
        offset_x + border_box.x,
        offset_y + border_box.y,
        border_box.width,
        border_box.height,
    );

    let has_radius = style.border_radius.has_radius();
    let radius = if has_radius {
        Some(style.border_radius)
    } else {
        None
    };

    match &style.background {
        Background::Color(color) => {
            let render_color: RenderColor = (*color).into();

            // Skip transparent backgrounds
            if render_color.is_transparent() {
                return;
            }

            // Check if we have border-radius
            if has_radius {
                list.push(PaintCommand::FillRoundedRect {
                    rect,
                    radius: style.border_radius,
                    color: render_color,
                });
            } else {
                list.push(PaintCommand::FillRect { rect, color: render_color });
            }
        }
        Background::Gradient(gradient) => {
            match gradient {
                Gradient::Linear { direction, stops } => {
                    list.push(PaintCommand::FillLinearGradient {
                        rect,
                        direction: direction.clone(),
                        stops: stops.clone(),
                        radius,
                    });
                }
                Gradient::Radial {
                    shape,
                    size,
                    center_x,
                    center_y,
                    stops,
                } => {
                    list.push(PaintCommand::FillRadialGradient {
                        rect,
                        shape: *shape,
                        size: *size,
                        center_x: *center_x,
                        center_y: *center_y,
                        stops: stops.clone(),
                        radius,
                    });
                }
            }
        }
    }
}

/// Render the borders of a layout box
fn render_borders(list: &mut DisplayList, layout_box: &LayoutBox, offset_x: f32, offset_y: f32) {
    let style = match layout_box.style() {
        Some(s) => s,
        None => return,
    };

    let d = &layout_box.dimensions;

    // Skip if no borders
    if d.border.top == 0.0
        && d.border.right == 0.0
        && d.border.bottom == 0.0
        && d.border.left == 0.0
    {
        return;
    }

    let color: RenderColor = style.border_color.into();
    let border_box = d.border_box();

    // Adjust to absolute position
    let rect = Rect::new(
        offset_x + border_box.x,
        offset_y + border_box.y,
        border_box.width,
        border_box.height,
    );

    let widths = BorderWidths {
        top: d.border.top,
        right: d.border.right,
        bottom: d.border.bottom,
        left: d.border.left,
    };

    // Check if we have border-radius
    if style.border_radius.has_radius() {
        list.push(PaintCommand::DrawRoundedBorder {
            rect,
            radius: style.border_radius,
            widths,
            color,
        });
    } else {
        list.push(PaintCommand::DrawBorder {
            rect,
            widths,
            color,
        });
    }
}

/// Render text content and form elements
fn render_content(list: &mut DisplayList, layout_box: &LayoutBox, abs_x: f32, abs_y: f32) {
    match &layout_box.box_type {
        BoxType::Text(_, text, style) => {
            let color: RenderColor = style.color.into();

            list.push(PaintCommand::DrawText {
                text: text.clone(),
                x: abs_x,
                y: abs_y,
                color,
                font_size: style.font_size,
            });
        }
        BoxType::Input(node_id, input_type, _) => {
            let d = &layout_box.dimensions;
            let rect = Rect::new(abs_x, abs_y, d.content.width, d.content.height);
            let node_id = *node_id;

            match input_type {
                InputType::Text | InputType::Password => {
                    // Text input - rendered with placeholder text and no cursor initially
                    list.push(PaintCommand::DrawTextInput {
                        node_id,
                        rect,
                        text: String::new(),
                        cursor_pos: None,
                        is_password: matches!(input_type, InputType::Password),
                        is_focused: false,
                    });
                }
                InputType::Checkbox => {
                    list.push(PaintCommand::DrawCheckbox {
                        node_id,
                        rect,
                        checked: false,
                        is_focused: false,
                    });
                }
                InputType::Radio => {
                    list.push(PaintCommand::DrawRadio {
                        node_id,
                        rect,
                        checked: false,
                        is_focused: false,
                    });
                }
                InputType::Submit | InputType::Button => {
                    // Submit/button input rendered as button
                    list.push(PaintCommand::DrawButton {
                        node_id,
                        rect,
                        text: "Submit".to_string(),
                        is_pressed: false,
                    });
                }
                InputType::Hidden => {
                    // Hidden inputs don't render anything
                }
            }
        }
        BoxType::Button(node_id, label, _) => {
            let d = &layout_box.dimensions;
            let rect = Rect::new(abs_x, abs_y, d.content.width, d.content.height);

            list.push(PaintCommand::DrawButton {
                node_id: *node_id,
                rect,
                text: label.clone(),
                is_pressed: false,
            });
        }
        BoxType::Image(_, ref image_data, _) => {
            let d = &layout_box.dimensions;
            let rect = Rect::new(abs_x, abs_y, d.content.width, d.content.height);

            list.push(PaintCommand::DrawImage {
                rect,
                pixels: image_data.pixels.clone(),
                alt: image_data.alt.clone(),
            });
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_list_new() {
        let list = DisplayList::new();
        assert!(list.is_empty());
    }

    #[test]
    fn test_display_list_push() {
        let mut list = DisplayList::new();
        list.push(PaintCommand::FillRect {
            rect: Rect::new(0.0, 0.0, 100.0, 100.0),
            color: RenderColor::black(),
        });
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn test_border_widths() {
        let bw = BorderWidths {
            top: 1.0,
            right: 2.0,
            bottom: 3.0,
            left: 4.0,
        };
        assert_eq!(bw.top, 1.0);
        assert_eq!(bw.right, 2.0);
    }
}

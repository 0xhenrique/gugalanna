//! SDL2 Render Backend
//!
//! Implements rendering using SDL2.

use sdl2::mouse::{Cursor, SystemCursor};
use sdl2::pixels::{Color as SdlColor, PixelFormatEnum};
use sdl2::rect::Rect as SdlRect;
use sdl2::render::{BlendMode, Canvas, TextureCreator};
use sdl2::video::{Window, WindowContext};
use sdl2::Sdl;

use gugalanna_layout::Rect;
use gugalanna_style::{BorderRadius, BoxShadow, ColorStop, GradientDirection, RadialShape, RadialSize};

use crate::display_list::{BorderWidths, DisplayList, PaintCommand};
use crate::font::FontCache;
use crate::paint::RenderColor;
use crate::RenderBackend;

/// Cursor type for link hover
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorType {
    Arrow,
    Hand,
}

/// SDL2-based render backend
pub struct SdlBackend {
    sdl_context: Sdl,
    canvas: Canvas<Window>,
    texture_creator: TextureCreator<WindowContext>,
    font_cache: FontCache,
    width: u32,
    height: u32,
    cursor_arrow: Cursor,
    cursor_hand: Cursor,
    /// Stack of opacity modifiers (multiplied together)
    opacity_stack: Vec<f32>,
}

impl SdlBackend {
    /// Create a new SDL backend with a window
    pub fn new(title: &str, width: u32, height: u32) -> Result<Self, String> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;

        let window = video_subsystem
            .window(title, width, height)
            .position_centered()
            .resizable()
            .build()
            .map_err(|e| e.to_string())?;

        let canvas = window
            .into_canvas()
            .accelerated()
            .present_vsync()
            .build()
            .map_err(|e| e.to_string())?;

        let texture_creator = canvas.texture_creator();
        let font_cache = FontCache::new();

        // Create cursors for hover states
        let cursor_arrow = Cursor::from_system(SystemCursor::Arrow)
            .map_err(|e| e.to_string())?;
        let cursor_hand = Cursor::from_system(SystemCursor::Hand)
            .map_err(|e| e.to_string())?;

        Ok(Self {
            sdl_context,
            canvas,
            texture_creator,
            font_cache,
            width,
            height,
            cursor_arrow,
            cursor_hand,
            opacity_stack: Vec::new(),
        })
    }

    /// Set the mouse cursor type
    pub fn set_cursor(&self, cursor_type: CursorType) {
        match cursor_type {
            CursorType::Arrow => self.cursor_arrow.set(),
            CursorType::Hand => self.cursor_hand.set(),
        }
    }

    /// Get the SDL context for event handling
    pub fn sdl_context(&self) -> &Sdl {
        &self.sdl_context
    }

    /// Get mutable access to font cache
    pub fn font_cache_mut(&mut self) -> &mut FontCache {
        &mut self.font_cache
    }

    /// Draw a filled rectangle
    fn draw_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color: RenderColor) {
        self.canvas.set_draw_color(SdlColor::RGBA(color.r, color.g, color.b, color.a));
        let rect = SdlRect::new(x, y, w, h);
        let _ = self.canvas.fill_rect(rect);
    }

    /// Draw text at a position
    fn draw_text(&mut self, text: &str, x: f32, y: f32, color: RenderColor, font_size: f32) {
        let mut cursor_x = x as i32;
        let baseline_y = (y as i32).saturating_add(self.font_cache.ascent(font_size) as i32);

        // Pre-rasterize all glyphs and collect their data
        let glyphs: Vec<_> = text.chars().map(|c| {
            let glyph = self.font_cache.rasterize(c, font_size);
            (
                glyph.width,
                glyph.height,
                glyph.bitmap.clone(),
                glyph.advance_width,
                glyph.offset_x,
                glyph.offset_y,
            )
        }).collect();

        // Now draw them
        for (width, height, bitmap, advance_width, offset_x, offset_y) in glyphs {
            if width > 0 && height > 0 {
                let glyph_x = cursor_x.saturating_add(offset_x);
                let glyph_y = baseline_y.saturating_sub(offset_y).saturating_sub(height as i32);

                self.draw_glyph_bitmap(
                    &bitmap,
                    width,
                    height,
                    glyph_x,
                    glyph_y,
                    color,
                );
            }

            cursor_x = cursor_x.saturating_add(advance_width as i32);
        }
    }

    /// Draw a glyph bitmap at a position using texture blitting
    fn draw_glyph_bitmap(
        &mut self,
        bitmap: &[u8],
        width: u32,
        height: u32,
        x: i32,
        y: i32,
        color: RenderColor,
    ) {
        if width == 0 || height == 0 || bitmap.is_empty() {
            return;
        }

        // Create RGBA pixel data from the alpha-only bitmap
        let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);
        for &alpha in bitmap.iter().take((width * height) as usize) {
            // Pre-multiply color with alpha for proper blending
            let blended_alpha = ((alpha as u32 * color.a as u32) / 255) as u8;
            rgba_data.push(color.r);
            rgba_data.push(color.g);
            rgba_data.push(color.b);
            rgba_data.push(blended_alpha);
        }

        // Create a streaming texture for this glyph
        let mut texture = match self.texture_creator.create_texture_streaming(
            PixelFormatEnum::RGBA32,
            width,
            height,
        ) {
            Ok(t) => t,
            Err(_) => return,
        };

        // Set blend mode for alpha transparency
        texture.set_blend_mode(BlendMode::Blend);

        // Update texture with pixel data
        let pitch = (width * 4) as usize;
        if texture.update(None, &rgba_data, pitch).is_err() {
            return;
        }

        // Blit the texture to the canvas
        let dst_rect = SdlRect::new(x, y, width, height);
        let _ = self.canvas.copy(&texture, None, dst_rect);
    }

    /// Draw a border (four rectangles)
    fn draw_border(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        top: f32,
        right: f32,
        bottom: f32,
        left: f32,
        color: RenderColor,
    ) {
        let x = x as i32;
        let y = y as i32;
        let w = w as u32;
        let h = h as u32;

        // Top border
        if top > 0.0 {
            self.draw_rect(x, y, w, top as u32, color);
        }

        // Bottom border
        if bottom > 0.0 {
            self.draw_rect(x, y + h as i32 - bottom as i32, w, bottom as u32, color);
        }

        // Left border
        if left > 0.0 {
            self.draw_rect(x, y, left as u32, h, color);
        }

        // Right border
        if right > 0.0 {
            self.draw_rect(x + w as i32 - right as i32, y, right as u32, h, color);
        }
    }

    /// Draw a text input field
    fn draw_text_input(
        &mut self,
        rect: &gugalanna_layout::Rect,
        text: &str,
        cursor_pos: Option<usize>,
        is_password: bool,
        is_focused: bool,
    ) {
        let x = rect.x as i32;
        let y = rect.y as i32;
        let w = rect.width as u32;
        let h = rect.height as u32;

        // Background
        let bg_color = if is_focused {
            RenderColor::rgb(255, 255, 255)
        } else {
            RenderColor::rgb(250, 250, 250)
        };
        self.draw_rect(x, y, w, h, bg_color);

        // Border
        let border_color = if is_focused {
            RenderColor::rgb(0, 120, 212)
        } else {
            RenderColor::rgb(180, 180, 180)
        };
        self.draw_border(rect.x, rect.y, rect.width, rect.height, 1.0, 1.0, 1.0, 1.0, border_color);

        // Text (or dots for password)
        if !text.is_empty() {
            let display_text = if is_password {
                "\u{2022}".repeat(text.chars().count())
            } else {
                text.to_string()
            };
            self.draw_text(&display_text, rect.x + 4.0, rect.y + 4.0, RenderColor::black(), 14.0);
        }

        // Cursor
        if let Some(pos) = cursor_pos {
            let cursor_x = rect.x + 4.0 + (pos as f32 * 8.0);
            self.draw_rect(
                cursor_x as i32,
                y + 2,
                1,
                h.saturating_sub(4),
                RenderColor::black(),
            );
        }
    }

    /// Draw a checkbox
    fn draw_checkbox(&mut self, rect: &gugalanna_layout::Rect, checked: bool, is_focused: bool) {
        let x = rect.x as i32;
        let y = rect.y as i32;
        let size = rect.width.min(rect.height) as u32;

        // Background
        self.draw_rect(x, y, size, size, RenderColor::rgb(255, 255, 255));

        // Border
        let border_color = if is_focused {
            RenderColor::rgb(0, 120, 212)
        } else {
            RenderColor::rgb(128, 128, 128)
        };
        self.draw_border(rect.x, rect.y, size as f32, size as f32, 1.0, 1.0, 1.0, 1.0, border_color);

        // Checkmark
        if checked {
            // Draw a simple checkmark using two diagonal lines
            let inset = 3;
            let inner_size = size.saturating_sub(inset * 2);
            let check_color = RenderColor::rgb(0, 120, 212);

            // Simple checkmark: draw a small filled rectangle in center
            self.draw_rect(
                x + inset as i32 + 2,
                y + inset as i32 + 2,
                inner_size.saturating_sub(4),
                inner_size.saturating_sub(4),
                check_color,
            );
        }
    }

    /// Draw a radio button
    fn draw_radio(&mut self, rect: &gugalanna_layout::Rect, checked: bool, is_focused: bool) {
        let x = rect.x as i32;
        let y = rect.y as i32;
        let size = rect.width.min(rect.height) as u32;

        // Background (circular approximated with filled rect)
        self.draw_rect(x, y, size, size, RenderColor::rgb(255, 255, 255));

        // Border
        let border_color = if is_focused {
            RenderColor::rgb(0, 120, 212)
        } else {
            RenderColor::rgb(128, 128, 128)
        };
        self.draw_border(rect.x, rect.y, size as f32, size as f32, 1.0, 1.0, 1.0, 1.0, border_color);

        // Inner dot when checked
        if checked {
            let inset = 4;
            let inner_size = size.saturating_sub(inset * 2);
            self.draw_rect(
                x + inset as i32,
                y + inset as i32,
                inner_size,
                inner_size,
                RenderColor::rgb(0, 120, 212),
            );
        }
    }

    /// Draw a button
    fn draw_button(&mut self, rect: &gugalanna_layout::Rect, text: &str, is_pressed: bool) {
        let x = rect.x as i32;
        let y = rect.y as i32;
        let w = rect.width as u32;
        let h = rect.height as u32;

        // Background
        let bg_color = if is_pressed {
            RenderColor::rgb(200, 200, 200)
        } else {
            RenderColor::rgb(240, 240, 240)
        };
        self.draw_rect(x, y, w, h, bg_color);

        // Border
        self.draw_border(rect.x, rect.y, rect.width, rect.height, 1.0, 1.0, 1.0, 1.0, RenderColor::rgb(128, 128, 128));

        // Centered text
        // Calculate approximate text width (8px per character at 14px font)
        let text_width = text.len() as f32 * 8.0;
        let text_x = rect.x + (rect.width - text_width) / 2.0;
        let text_y = rect.y + (rect.height - 14.0) / 2.0;
        self.draw_text(text, text_x, text_y, RenderColor::black(), 14.0);
    }

    /// Draw an image
    fn draw_image(
        &mut self,
        rect: &gugalanna_layout::Rect,
        pixels: Option<&gugalanna_layout::ImagePixels>,
        alt: &str,
    ) {
        // Check if we have valid image data first
        let img = match pixels {
            Some(img) if img.width > 0 && img.height > 0 && !img.data.is_empty() => img,
            _ => {
                // No valid image data - draw placeholder
                self.draw_image_placeholder(rect, alt);
                return;
            }
        };

        let x = rect.x as i32;
        let y = rect.y as i32;
        let w = rect.width as u32;
        let h = rect.height as u32;

        // Try to render the image, track if we need to show placeholder
        let render_success = self.try_render_image_texture(img, x, y, w, h);

        if !render_success {
            self.draw_image_placeholder(rect, alt);
        }
    }

    /// Try to render image pixels as a texture, returns true on success
    fn try_render_image_texture(
        &mut self,
        img: &gugalanna_layout::ImagePixels,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
    ) -> bool {
        // Create texture from pixel data
        let mut texture = match self.texture_creator.create_texture_streaming(
            PixelFormatEnum::RGBA32,
            img.width,
            img.height,
        ) {
            Ok(t) => t,
            Err(_) => return false,
        };

        // Enable alpha blending
        texture.set_blend_mode(BlendMode::Blend);

        // Update texture with pixel data
        let pitch = (img.width * 4) as usize;
        if texture.update(None, &img.data, pitch).is_err() {
            return false;
        }

        // Copy texture to canvas, scaling to fit the layout rect
        let dst_rect = SdlRect::new(x, y, w, h);
        self.canvas.copy(&texture, None, dst_rect).is_ok()
    }

    /// Draw a placeholder for failed/loading images
    fn draw_image_placeholder(&mut self, rect: &gugalanna_layout::Rect, alt: &str) {
        let x = rect.x as i32;
        let y = rect.y as i32;
        let w = rect.width as u32;
        let h = rect.height as u32;

        // Light gray background
        self.draw_rect(x, y, w, h, RenderColor::rgb(240, 240, 240));

        // Border
        self.draw_border(
            rect.x, rect.y, rect.width, rect.height,
            1.0, 1.0, 1.0, 1.0,
            RenderColor::rgb(200, 200, 200),
        );

        // Alt text (truncated if too long)
        if !alt.is_empty() {
            let text = if alt.len() > 30 {
                format!("{}...", &alt[..27])
            } else {
                alt.to_string()
            };

            // Center the text
            let text_width = text.len() as f32 * 7.0;
            let text_x = rect.x + (rect.width - text_width).max(0.0) / 2.0;
            let text_y = rect.y + (rect.height - 14.0).max(0.0) / 2.0;

            self.draw_text(&text, text_x.max(rect.x + 4.0), text_y.max(rect.y + 4.0), RenderColor::rgb(128, 128, 128), 14.0);
        }
    }

    /// Get the current opacity (product of all stacked opacities)
    fn current_opacity(&self) -> f32 {
        self.opacity_stack.iter().fold(1.0, |acc, &o| acc * o)
    }

    /// Apply current opacity to a color
    fn apply_opacity(&self, color: RenderColor) -> RenderColor {
        let opacity = self.current_opacity();
        if opacity >= 1.0 {
            return color;
        }
        RenderColor {
            r: color.r,
            g: color.g,
            b: color.b,
            a: (color.a as f32 * opacity) as u8,
        }
    }

    /// Draw a box shadow using layered rectangles
    fn draw_box_shadow(&mut self, rect: &gugalanna_layout::Rect, shadow: &BoxShadow) {
        // Calculate shadow position
        let base_x = rect.x + shadow.offset_x;
        let base_y = rect.y + shadow.offset_y;

        // Expand rect by spread radius
        let shadow_rect = gugalanna_layout::Rect::new(
            base_x - shadow.spread_radius,
            base_y - shadow.spread_radius,
            rect.width + 2.0 * shadow.spread_radius,
            rect.height + 2.0 * shadow.spread_radius,
        );

        // Convert shadow color to RenderColor
        let shadow_color = RenderColor {
            r: shadow.color.r,
            g: shadow.color.g,
            b: shadow.color.b,
            a: shadow.color.a,
        };

        // If no blur, just draw a solid shadow
        if shadow.blur_radius <= 0.0 {
            let color = self.apply_opacity(shadow_color);
            self.draw_rect(
                shadow_rect.x as i32,
                shadow_rect.y as i32,
                shadow_rect.width as u32,
                shadow_rect.height as u32,
                color,
            );
            return;
        }

        // Draw blur layers (outer to inner, each smaller with more alpha)
        let layers = (shadow.blur_radius / 2.0).max(1.0).min(20.0) as i32;
        for i in (0..layers).rev() {
            let t = i as f32 / layers as f32;
            let expansion = t * shadow.blur_radius;
            // Alpha decreases from outside to inside
            let alpha = (shadow_color.a as f32 * (1.0 - t * 0.7)) as u8;

            let layer_rect = gugalanna_layout::Rect::new(
                shadow_rect.x - expansion,
                shadow_rect.y - expansion,
                shadow_rect.width + 2.0 * expansion,
                shadow_rect.height + 2.0 * expansion,
            );

            let color = self.apply_opacity(RenderColor {
                r: shadow_color.r,
                g: shadow_color.g,
                b: shadow_color.b,
                a: alpha / layers as u8, // Divide by layers for softer effect
            });

            self.draw_rect(
                layer_rect.x as i32,
                layer_rect.y as i32,
                layer_rect.width as u32,
                layer_rect.height as u32,
                color,
            );
        }
    }

    /// Draw a filled rounded rectangle
    fn draw_rounded_rect(
        &mut self,
        rect: &gugalanna_layout::Rect,
        radius: &BorderRadius,
        color: RenderColor,
    ) {
        let x = rect.x as i32;
        let y = rect.y as i32;
        let w = rect.width;
        let h = rect.height;

        // Clamp radii to half the dimensions
        let max_radius = (w / 2.0).min(h / 2.0);
        let tl = radius.top_left.min(max_radius);
        let tr = radius.top_right.min(max_radius);
        let br = radius.bottom_right.min(max_radius);
        let bl = radius.bottom_left.min(max_radius);

        let color = self.apply_opacity(color);

        // If all radii are 0, just draw a regular rect
        if tl <= 0.0 && tr <= 0.0 && br <= 0.0 && bl <= 0.0 {
            self.draw_rect(x, y, w as u32, h as u32, color);
            return;
        }

        // Draw the main rectangle body (center region)
        let max_top = tl.max(tr);
        let max_bottom = bl.max(br);
        let max_left = tl.max(bl);
        let max_right = tr.max(br);

        // Center horizontal bar
        self.draw_rect(
            x + max_left as i32,
            y,
            (w - max_left - max_right) as u32,
            h as u32,
            color,
        );

        // Left bar
        self.draw_rect(
            x,
            y + max_top as i32,
            max_left as u32,
            (h - max_top - max_bottom) as u32,
            color,
        );

        // Right bar
        self.draw_rect(
            x + (w - max_right) as i32,
            y + max_top as i32,
            max_right as u32,
            (h - max_top - max_bottom) as u32,
            color,
        );

        // Draw corner quarters using filled circles (scanlines)
        // Top-left corner
        if tl > 0.0 {
            self.fill_quarter_circle(x + tl as i32, y + tl as i32, tl, 0, color);
        }
        // Top-right corner
        if tr > 0.0 {
            self.fill_quarter_circle(x + (w - tr) as i32, y + tr as i32, tr, 1, color);
        }
        // Bottom-right corner
        if br > 0.0 {
            self.fill_quarter_circle(x + (w - br) as i32, y + (h - br) as i32, br, 2, color);
        }
        // Bottom-left corner
        if bl > 0.0 {
            self.fill_quarter_circle(x + bl as i32, y + (h - bl) as i32, bl, 3, color);
        }
    }

    /// Fill a quarter circle using horizontal scanlines
    /// quadrant: 0=top-left, 1=top-right, 2=bottom-right, 3=bottom-left
    fn fill_quarter_circle(&mut self, cx: i32, cy: i32, r: f32, quadrant: u8, color: RenderColor) {
        let r_int = r as i32;
        let r_sq = r * r;

        for dy in 0..=r_int {
            let dx = ((r_sq - (dy as f32 * dy as f32)).sqrt()) as i32;

            let (line_x, line_y, line_w) = match quadrant {
                0 => (cx - dx, cy - dy, dx as u32),           // top-left
                1 => (cx, cy - dy, dx as u32),                 // top-right
                2 => (cx, cy + dy, dx as u32),                 // bottom-right
                3 => (cx - dx, cy + dy, dx as u32),           // bottom-left
                _ => continue,
            };

            if line_w > 0 {
                self.draw_rect(line_x, line_y, line_w, 1, color);
            }
        }
    }

    /// Draw a rounded border
    fn draw_rounded_border(
        &mut self,
        rect: &gugalanna_layout::Rect,
        radius: &BorderRadius,
        widths: &BorderWidths,
        color: RenderColor,
    ) {
        // For now, draw outer rounded rect minus inner rounded rect
        // This is a simplified approach - proper rounded borders are complex

        let color = self.apply_opacity(color);

        // Draw the border sides (simplified - not truly rounded at corners)
        // Top border
        if widths.top > 0.0 {
            self.draw_rect(
                rect.x as i32 + radius.top_left as i32,
                rect.y as i32,
                (rect.width - radius.top_left - radius.top_right) as u32,
                widths.top as u32,
                color,
            );
        }

        // Bottom border
        if widths.bottom > 0.0 {
            self.draw_rect(
                rect.x as i32 + radius.bottom_left as i32,
                (rect.y + rect.height - widths.bottom) as i32,
                (rect.width - radius.bottom_left - radius.bottom_right) as u32,
                widths.bottom as u32,
                color,
            );
        }

        // Left border
        if widths.left > 0.0 {
            self.draw_rect(
                rect.x as i32,
                rect.y as i32 + radius.top_left as i32,
                widths.left as u32,
                (rect.height - radius.top_left - radius.bottom_left) as u32,
                color,
            );
        }

        // Right border
        if widths.right > 0.0 {
            self.draw_rect(
                (rect.x + rect.width - widths.right) as i32,
                rect.y as i32 + radius.top_right as i32,
                widths.right as u32,
                (rect.height - radius.top_right - radius.bottom_right) as u32,
                color,
            );
        }

        // Draw corner arcs (simplified as quarter rings using multiple circles)
        let border_width = widths.top.max(widths.right).max(widths.bottom).max(widths.left);
        if border_width > 0.0 {
            // Top-left arc
            if radius.top_left > 0.0 {
                self.draw_quarter_arc(
                    rect.x as i32 + radius.top_left as i32,
                    rect.y as i32 + radius.top_left as i32,
                    radius.top_left,
                    radius.top_left - border_width,
                    0,
                    color,
                );
            }
            // Top-right arc
            if radius.top_right > 0.0 {
                self.draw_quarter_arc(
                    (rect.x + rect.width - radius.top_right) as i32,
                    rect.y as i32 + radius.top_right as i32,
                    radius.top_right,
                    radius.top_right - border_width,
                    1,
                    color,
                );
            }
            // Bottom-right arc
            if radius.bottom_right > 0.0 {
                self.draw_quarter_arc(
                    (rect.x + rect.width - radius.bottom_right) as i32,
                    (rect.y + rect.height - radius.bottom_right) as i32,
                    radius.bottom_right,
                    radius.bottom_right - border_width,
                    2,
                    color,
                );
            }
            // Bottom-left arc
            if radius.bottom_left > 0.0 {
                self.draw_quarter_arc(
                    rect.x as i32 + radius.bottom_left as i32,
                    (rect.y + rect.height - radius.bottom_left) as i32,
                    radius.bottom_left,
                    radius.bottom_left - border_width,
                    3,
                    color,
                );
            }
        }
    }

    /// Draw a quarter arc (ring segment) using horizontal scanlines
    fn draw_quarter_arc(
        &mut self,
        cx: i32,
        cy: i32,
        outer_r: f32,
        inner_r: f32,
        quadrant: u8,
        color: RenderColor,
    ) {
        let outer_r_int = outer_r as i32;
        let outer_r_sq = outer_r * outer_r;
        let inner_r_sq = inner_r.max(0.0) * inner_r.max(0.0);

        for dy in 0..=outer_r_int {
            let dy_sq = (dy as f32) * (dy as f32);
            let outer_dx = ((outer_r_sq - dy_sq).max(0.0).sqrt()) as i32;
            let inner_dx = if inner_r > 0.0 {
                ((inner_r_sq - dy_sq).max(0.0).sqrt()) as i32
            } else {
                0
            };

            let line_width = (outer_dx - inner_dx) as u32;
            if line_width == 0 {
                continue;
            }

            let (line_x, line_y) = match quadrant {
                0 => (cx - outer_dx, cy - dy),       // top-left
                1 => (cx + inner_dx, cy - dy),        // top-right
                2 => (cx + inner_dx, cy + dy),        // bottom-right
                3 => (cx - outer_dx, cy + dy),       // bottom-left
                _ => continue,
            };

            self.draw_rect(line_x, line_y, line_width, 1, color);
        }
    }

    /// Draw a linear gradient
    fn draw_linear_gradient(
        &mut self,
        rect: &Rect,
        direction: &GradientDirection,
        stops: &[ColorStop],
        _radius: Option<&BorderRadius>,
    ) {
        if stops.len() < 2 {
            return;
        }

        let x = rect.x as i32;
        let y = rect.y as i32;
        let w = rect.width as i32;
        let h = rect.height as i32;

        // Normalize color stops (distribute auto positions)
        let normalized = Self::normalize_color_stops(stops);

        // Calculate gradient direction vector
        let (is_vertical, is_horizontal) = match direction {
            GradientDirection::ToBottom | GradientDirection::ToTop => (true, false),
            GradientDirection::ToRight | GradientDirection::ToLeft => (false, true),
            GradientDirection::Angle(deg) => {
                // For angled gradients, we'll approximate with scanlines
                let rad = deg.to_radians();
                let dy = -rad.cos();
                let dx = rad.sin();
                // If mostly vertical or horizontal, use that
                if dy.abs() > dx.abs() {
                    (true, false)
                } else {
                    (false, true)
                }
            }
            _ => (true, false), // Default to vertical for diagonal directions
        };

        let reverse = matches!(direction, GradientDirection::ToTop | GradientDirection::ToLeft);

        if is_vertical {
            // Vertical gradient - draw horizontal lines
            for row in 0..h {
                let t = if h > 1 {
                    row as f32 / (h - 1) as f32
                } else {
                    0.5
                };
                let t = if reverse { 1.0 - t } else { t };
                let color = Self::interpolate_color(&normalized, t);
                let final_color = self.apply_opacity(color);
                self.canvas.set_draw_color(SdlColor::RGBA(
                    final_color.r,
                    final_color.g,
                    final_color.b,
                    final_color.a,
                ));
                let _ = self.canvas.fill_rect(SdlRect::new(x, y + row, w as u32, 1));
            }
        } else if is_horizontal {
            // Horizontal gradient - draw vertical lines
            for col in 0..w {
                let t = if w > 1 {
                    col as f32 / (w - 1) as f32
                } else {
                    0.5
                };
                let t = if reverse { 1.0 - t } else { t };
                let color = Self::interpolate_color(&normalized, t);
                let final_color = self.apply_opacity(color);
                self.canvas.set_draw_color(SdlColor::RGBA(
                    final_color.r,
                    final_color.g,
                    final_color.b,
                    final_color.a,
                ));
                let _ = self.canvas.fill_rect(SdlRect::new(x + col, y, 1, h as u32));
            }
        }

        // Note: border-radius not applied for gradients in this basic implementation
    }

    /// Draw a radial gradient
    fn draw_radial_gradient(
        &mut self,
        rect: &Rect,
        _shape: &RadialShape,
        _size: &RadialSize,
        center_x: f32,
        center_y: f32,
        stops: &[ColorStop],
        _radius: Option<&BorderRadius>,
    ) {
        if stops.len() < 2 {
            return;
        }

        let x = rect.x as i32;
        let y = rect.y as i32;
        let w = rect.width as i32;
        let h = rect.height as i32;

        // Center point in absolute pixels
        let cx = rect.x + rect.width * center_x;
        let cy = rect.y + rect.height * center_y;

        // Maximum radius (distance to farthest corner)
        let corners = [
            (rect.x, rect.y),
            (rect.x + rect.width, rect.y),
            (rect.x, rect.y + rect.height),
            (rect.x + rect.width, rect.y + rect.height),
        ];
        let max_radius = corners.iter()
            .map(|(px, py)| {
                let dx = px - cx;
                let dy = py - cy;
                (dx * dx + dy * dy).sqrt()
            })
            .fold(0.0_f32, f32::max);

        // Normalize color stops
        let normalized = Self::normalize_color_stops(stops);

        // Draw pixel by pixel (simple but slow approach)
        for row in 0..h {
            for col in 0..w {
                let px = x + col;
                let py = y + row;

                // Distance from center
                let dx = px as f32 - cx;
                let dy = py as f32 - cy;
                let distance = (dx * dx + dy * dy).sqrt();

                // Normalize to 0..1
                let t = if max_radius > 0.0 {
                    (distance / max_radius).min(1.0)
                } else {
                    0.0
                };

                let color = Self::interpolate_color(&normalized, t);
                let final_color = self.apply_opacity(color);
                self.canvas.set_draw_color(SdlColor::RGBA(
                    final_color.r,
                    final_color.g,
                    final_color.b,
                    final_color.a,
                ));
                let _ = self.canvas.draw_point((px, py));
            }
        }
    }

    /// Normalize color stops by distributing auto positions
    fn normalize_color_stops(stops: &[ColorStop]) -> Vec<(f32, RenderColor)> {
        let mut result = Vec::with_capacity(stops.len());

        // First pass: collect known positions
        let mut positions: Vec<Option<f32>> = stops.iter()
            .map(|s| s.position)
            .collect();

        // Ensure first and last have positions
        if positions.first().map(|p| p.is_none()).unwrap_or(true) {
            positions[0] = Some(0.0);
        }
        if positions.last().map(|p| p.is_none()).unwrap_or(true) {
            let last = positions.len() - 1;
            positions[last] = Some(1.0);
        }

        // Interpolate missing positions
        let mut i = 0;
        while i < positions.len() {
            if positions[i].is_none() {
                // Find next known position
                let start_idx = i - 1;
                let start_pos = positions[start_idx].unwrap();

                let mut end_idx = i + 1;
                while end_idx < positions.len() && positions[end_idx].is_none() {
                    end_idx += 1;
                }
                let end_pos = positions[end_idx].unwrap();

                // Distribute positions evenly
                let count = end_idx - start_idx;
                for j in i..end_idx {
                    let frac = (j - start_idx) as f32 / count as f32;
                    positions[j] = Some(start_pos + (end_pos - start_pos) * frac);
                }
                i = end_idx;
            } else {
                i += 1;
            }
        }

        // Build result
        for (stop, pos) in stops.iter().zip(positions.iter()) {
            let color: RenderColor = stop.color.into();
            result.push((pos.unwrap_or(0.0), color));
        }

        result
    }

    /// Interpolate between color stops at position t (0.0 to 1.0)
    fn interpolate_color(stops: &[(f32, RenderColor)], t: f32) -> RenderColor {
        if stops.is_empty() {
            return RenderColor::black();
        }
        if stops.len() == 1 {
            return stops[0].1;
        }

        let t = t.clamp(0.0, 1.0);

        // Find surrounding stops
        let mut prev = &stops[0];
        let mut next = &stops[stops.len() - 1];

        for i in 0..stops.len() - 1 {
            if stops[i].0 <= t && t <= stops[i + 1].0 {
                prev = &stops[i];
                next = &stops[i + 1];
                break;
            }
        }

        // Interpolate between stops
        let range = next.0 - prev.0;
        let local_t = if range > 0.0 { (t - prev.0) / range } else { 0.0 };

        RenderColor {
            r: Self::lerp_u8(prev.1.r, next.1.r, local_t),
            g: Self::lerp_u8(prev.1.g, next.1.g, local_t),
            b: Self::lerp_u8(prev.1.b, next.1.b, local_t),
            a: Self::lerp_u8(prev.1.a, next.1.a, local_t),
        }
    }

    /// Linear interpolation for u8 values
    fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
        let result = a as f32 + (b as f32 - a as f32) * t;
        result.round().clamp(0.0, 255.0) as u8
    }
}

impl RenderBackend for SdlBackend {
    fn clear(&mut self, color: RenderColor) {
        self.canvas.set_draw_color(SdlColor::RGBA(color.r, color.g, color.b, color.a));
        self.canvas.clear();
    }

    fn render(&mut self, display_list: &DisplayList) {
        for command in &display_list.commands {
            match command {
                PaintCommand::FillRect { rect, color } => {
                    self.draw_rect(
                        rect.x as i32,
                        rect.y as i32,
                        rect.width as u32,
                        rect.height as u32,
                        *color,
                    );
                }
                PaintCommand::DrawText { text, x, y, color, font_size } => {
                    self.draw_text(text, *x, *y, *color, *font_size);
                }
                PaintCommand::DrawBorder { rect, widths, color } => {
                    self.draw_border(
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        widths.top,
                        widths.right,
                        widths.bottom,
                        widths.left,
                        *color,
                    );
                }
                PaintCommand::DrawTextInput { rect, text, cursor_pos, is_password, is_focused, .. } => {
                    self.draw_text_input(rect, text, *cursor_pos, *is_password, *is_focused);
                }
                PaintCommand::DrawCheckbox { rect, checked, is_focused, .. } => {
                    self.draw_checkbox(rect, *checked, *is_focused);
                }
                PaintCommand::DrawRadio { rect, checked, is_focused, .. } => {
                    self.draw_radio(rect, *checked, *is_focused);
                }
                PaintCommand::DrawButton { rect, text, is_pressed, .. } => {
                    self.draw_button(rect, text, *is_pressed);
                }
                PaintCommand::DrawImage { rect, pixels, alt } => {
                    self.draw_image(rect, pixels.as_ref(), alt);
                }
                PaintCommand::SetClipRect(rect) => {
                    let sdl_rect = SdlRect::new(
                        rect.x as i32,
                        rect.y as i32,
                        rect.width as u32,
                        rect.height as u32,
                    );
                    self.canvas.set_clip_rect(Some(sdl_rect));
                }
                PaintCommand::ClearClipRect => {
                    self.canvas.set_clip_rect(None);
                }
                PaintCommand::PushOpacity(opacity) => {
                    self.opacity_stack.push(*opacity);
                }
                PaintCommand::PopOpacity => {
                    self.opacity_stack.pop();
                }
                PaintCommand::DrawBoxShadow { rect, shadow } => {
                    self.draw_box_shadow(rect, shadow);
                }
                PaintCommand::FillRoundedRect { rect, radius, color } => {
                    self.draw_rounded_rect(rect, radius, *color);
                }
                PaintCommand::DrawRoundedBorder { rect, radius, widths, color } => {
                    self.draw_rounded_border(rect, radius, widths, *color);
                }
                PaintCommand::FillLinearGradient { rect, direction, stops, radius } => {
                    self.draw_linear_gradient(rect, direction, stops, radius.as_ref());
                }
                PaintCommand::FillRadialGradient { rect, shape, size, center_x, center_y, stops, radius } => {
                    self.draw_radial_gradient(rect, shape, size, *center_x, *center_y, stops, radius.as_ref());
                }
            }
        }
    }

    fn present(&mut self) {
        self.canvas.present();
    }

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }
}

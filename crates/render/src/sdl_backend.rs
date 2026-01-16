//! SDL2 Render Backend
//!
//! Implements rendering using SDL2.

use sdl2::mouse::{Cursor, SystemCursor};
use sdl2::pixels::{Color as SdlColor, PixelFormatEnum};
use sdl2::rect::Rect as SdlRect;
use sdl2::render::{BlendMode, Canvas, TextureCreator};
use sdl2::video::{Window, WindowContext};
use sdl2::Sdl;

use crate::display_list::{DisplayList, PaintCommand};
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
        let baseline_y = y as i32 + self.font_cache.ascent(font_size) as i32;

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
                let glyph_x = cursor_x + offset_x;
                let glyph_y = baseline_y - offset_y - height as i32;

                self.draw_glyph_bitmap(
                    &bitmap,
                    width,
                    height,
                    glyph_x,
                    glyph_y,
                    color,
                );
            }

            cursor_x += advance_width as i32;
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

//! Browser chrome UI components
//!
//! Address bar, navigation buttons, and browser chrome rendering.

use gugalanna_layout::Rect;
use gugalanna_render::{DisplayList, PaintCommand, RenderColor};

/// Browser chrome height in pixels
pub const CHROME_HEIGHT: f32 = 48.0;

/// Padding around chrome elements
const PADDING: f32 = 8.0;

/// Button width
const BUTTON_WIDTH: f32 = 32.0;

/// Button height
const BUTTON_HEIGHT: f32 = 32.0;

/// Browser chrome UI state
#[derive(Debug)]
pub struct Chrome {
    /// Total chrome height
    pub height: f32,
    /// Window width
    width: f32,
    /// Back button
    pub back_button: Button,
    /// Forward button
    pub forward_button: Button,
    /// Address bar
    pub address_bar: AddressBar,
    /// Go button
    pub go_button: Button,
    /// Whether a page is currently loading
    pub is_loading: bool,
    /// Loading animation frame counter
    loading_frame: u8,
}

/// A clickable button
#[derive(Debug, Clone)]
pub struct Button {
    /// Button bounds
    pub rect: Rect,
    /// Button label
    pub label: &'static str,
    /// Whether button is enabled
    pub enabled: bool,
}

/// Address bar state
#[derive(Debug, Clone)]
pub struct AddressBar {
    /// Address bar bounds
    pub rect: Rect,
    /// Current text content
    pub text: String,
    /// Cursor position (byte index)
    pub cursor_pos: usize,
    /// Whether address bar is focused
    pub is_focused: bool,
}

/// Result of hit testing the chrome
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeHit {
    BackButton,
    ForwardButton,
    GoButton,
    AddressBar,
}

impl Chrome {
    /// Create a new chrome instance
    pub fn new(window_width: f32) -> Self {
        let y_center = CHROME_HEIGHT / 2.0 - BUTTON_HEIGHT / 2.0;

        // Back button: [<]
        let back_button = Button {
            rect: Rect {
                x: PADDING,
                y: y_center,
                width: BUTTON_WIDTH,
                height: BUTTON_HEIGHT,
            },
            label: "<",
            enabled: false,
        };

        // Forward button: [>]
        let forward_button = Button {
            rect: Rect {
                x: PADDING + BUTTON_WIDTH + PADDING,
                y: y_center,
                width: BUTTON_WIDTH,
                height: BUTTON_HEIGHT,
            },
            label: ">",
            enabled: false,
        };

        // Go button at the right
        let go_button = Button {
            rect: Rect {
                x: window_width - PADDING - BUTTON_WIDTH,
                y: y_center,
                width: BUTTON_WIDTH,
                height: BUTTON_HEIGHT,
            },
            label: "Go",
            enabled: true,
        };

        // Address bar between forward button and go button
        let address_bar_x = forward_button.rect.x + forward_button.rect.width + PADDING;
        let address_bar_width = go_button.rect.x - address_bar_x - PADDING;

        let address_bar = AddressBar {
            rect: Rect {
                x: address_bar_x,
                y: y_center,
                width: address_bar_width,
                height: BUTTON_HEIGHT,
            },
            text: String::new(),
            cursor_pos: 0,
            is_focused: false,
        };

        Self {
            height: CHROME_HEIGHT,
            width: window_width,
            back_button,
            forward_button,
            address_bar,
            go_button,
            is_loading: false,
            loading_frame: 0,
        }
    }

    /// Update loading animation (call each frame when loading)
    pub fn tick_loading(&mut self) {
        if self.is_loading {
            self.loading_frame = self.loading_frame.wrapping_add(1);
        } else {
            self.loading_frame = 0;
        }
    }

    /// Build a display list for the chrome
    pub fn build_display_list(&self) -> DisplayList {
        let mut commands = Vec::new();

        // Chrome background
        commands.push(PaintCommand::FillRect {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: self.width,
                height: self.height,
            },
            color: RenderColor::new(240, 240, 240, 255), // Light gray
        });

        // Bottom border
        commands.push(PaintCommand::FillRect {
            rect: Rect {
                x: 0.0,
                y: self.height - 1.0,
                width: self.width,
                height: 1.0,
            },
            color: RenderColor::new(200, 200, 200, 255), // Darker gray
        });

        // Loading indicator (animated progress bar at bottom of chrome)
        if self.is_loading {
            // Oscillating progress bar using sine wave
            let progress = (self.loading_frame as f32 / 30.0 * std::f32::consts::PI).sin();
            let bar_width = self.width * (0.3 + 0.3 * progress.abs());
            let bar_x = (self.width - bar_width) * ((progress + 1.0) / 2.0);

            commands.push(PaintCommand::FillRect {
                rect: Rect {
                    x: bar_x,
                    y: self.height - 3.0,
                    width: bar_width,
                    height: 3.0,
                },
                color: RenderColor::new(66, 133, 244, 255), // Google blue
            });
        }

        // Back button
        self.render_button(&self.back_button, &mut commands);

        // Forward button
        self.render_button(&self.forward_button, &mut commands);

        // Address bar
        self.render_address_bar(&mut commands);

        // Go button
        self.render_button(&self.go_button, &mut commands);

        DisplayList { commands }
    }

    /// Render a button
    fn render_button(&self, button: &Button, commands: &mut Vec<PaintCommand>) {
        let bg_color = if button.enabled {
            RenderColor::new(255, 255, 255, 255) // White
        } else {
            RenderColor::new(220, 220, 220, 255) // Disabled gray
        };

        let text_color = if button.enabled {
            RenderColor::new(0, 0, 0, 255) // Black
        } else {
            RenderColor::new(150, 150, 150, 255) // Gray
        };

        // Button background
        commands.push(PaintCommand::FillRect {
            rect: button.rect,
            color: bg_color,
        });

        // Button border
        commands.push(PaintCommand::DrawBorder {
            rect: button.rect,
            widths: gugalanna_render::BorderWidths {
                top: 1.0,
                right: 1.0,
                bottom: 1.0,
                left: 1.0,
            },
            color: RenderColor::new(180, 180, 180, 255),
        });

        // Button label (centered)
        let text_x = button.rect.x + button.rect.width / 2.0 - 6.0;
        let text_y = button.rect.y + button.rect.height / 2.0 - 6.0;

        commands.push(PaintCommand::DrawText {
            text: button.label.to_string(),
            x: text_x,
            y: text_y,
            color: text_color,
            font_size: 14.0,
        });
    }

    /// Render the address bar
    fn render_address_bar(&self, commands: &mut Vec<PaintCommand>) {
        let border_color = if self.address_bar.is_focused {
            RenderColor::new(66, 133, 244, 255) // Blue when focused
        } else {
            RenderColor::new(180, 180, 180, 255) // Gray
        };

        // Address bar background
        commands.push(PaintCommand::FillRect {
            rect: self.address_bar.rect,
            color: RenderColor::new(255, 255, 255, 255),
        });

        // Address bar border
        let border_width = if self.address_bar.is_focused { 2.0 } else { 1.0 };
        commands.push(PaintCommand::DrawBorder {
            rect: self.address_bar.rect,
            widths: gugalanna_render::BorderWidths {
                top: border_width,
                right: border_width,
                bottom: border_width,
                left: border_width,
            },
            color: border_color,
        });

        // Address bar text
        let text_x = self.address_bar.rect.x + 8.0;
        let text_y = self.address_bar.rect.y + self.address_bar.rect.height / 2.0 - 6.0;

        if !self.address_bar.text.is_empty() {
            commands.push(PaintCommand::DrawText {
                text: self.address_bar.text.clone(),
                x: text_x,
                y: text_y,
                color: RenderColor::new(0, 0, 0, 255),
                font_size: 14.0,
            });
        }

        // Cursor when focused
        if self.address_bar.is_focused {
            // Simple cursor at end of text (approximate position)
            let cursor_x = text_x + (self.address_bar.cursor_pos as f32 * 8.0);
            commands.push(PaintCommand::FillRect {
                rect: Rect {
                    x: cursor_x,
                    y: self.address_bar.rect.y + 6.0,
                    width: 1.0,
                    height: self.address_bar.rect.height - 12.0,
                },
                color: RenderColor::new(0, 0, 0, 255),
            });
        }
    }

    /// Hit test the chrome
    ///
    /// Returns which element was hit, if any.
    pub fn hit_test(&self, x: f32, y: f32) -> Option<ChromeHit> {
        // Only check if within chrome height
        if y >= self.height {
            return None;
        }

        if self.back_button.contains(x, y) {
            return Some(ChromeHit::BackButton);
        }

        if self.forward_button.contains(x, y) {
            return Some(ChromeHit::ForwardButton);
        }

        if self.go_button.contains(x, y) {
            return Some(ChromeHit::GoButton);
        }

        if self.address_bar.contains(x, y) {
            return Some(ChromeHit::AddressBar);
        }

        None
    }

    /// Update button states based on navigation
    pub fn update_navigation_state(&mut self, can_back: bool, can_forward: bool) {
        self.back_button.enabled = can_back;
        self.forward_button.enabled = can_forward;
    }

    /// Update window width (for resize)
    pub fn update_width(&mut self, width: f32) {
        self.width = width;

        // Recalculate go button position
        self.go_button.rect.x = width - PADDING - BUTTON_WIDTH;

        // Recalculate address bar width
        let address_bar_x = self.forward_button.rect.x + self.forward_button.rect.width + PADDING;
        self.address_bar.rect.x = address_bar_x;
        self.address_bar.rect.width = self.go_button.rect.x - address_bar_x - PADDING;
    }
}

impl Button {
    /// Check if a point is within the button bounds
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.rect.x
            && x <= self.rect.x + self.rect.width
            && y >= self.rect.y
            && y <= self.rect.y + self.rect.height
    }
}

impl AddressBar {
    /// Check if a point is within the address bar bounds
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.rect.x
            && x <= self.rect.x + self.rect.width
            && y >= self.rect.y
            && y <= self.rect.y + self.rect.height
    }

    /// Insert a character at the cursor position
    pub fn insert_char(&mut self, c: char) {
        self.text.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    /// Delete the character before the cursor
    pub fn delete_char(&mut self) {
        if self.cursor_pos > 0 {
            // Find the previous character boundary
            let prev = self.text[..self.cursor_pos]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);

            self.text.remove(prev);
            self.cursor_pos = prev;
        }
    }

    /// Clear the address bar
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor_pos = 0;
    }

    /// Set text and move cursor to end
    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.cursor_pos = self.text.len();
    }

    /// Move cursor to end
    pub fn move_cursor_to_end(&mut self) {
        self.cursor_pos = self.text.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chrome_layout() {
        let chrome = Chrome::new(800.0);

        assert_eq!(chrome.height, CHROME_HEIGHT);
        assert!(chrome.back_button.rect.x < chrome.forward_button.rect.x);
        assert!(chrome.forward_button.rect.x < chrome.address_bar.rect.x);
        assert!(chrome.address_bar.rect.x + chrome.address_bar.rect.width < chrome.go_button.rect.x);
    }

    #[test]
    fn test_hit_test_back_button() {
        let chrome = Chrome::new(800.0);
        let center_x = chrome.back_button.rect.x + chrome.back_button.rect.width / 2.0;
        let center_y = chrome.back_button.rect.y + chrome.back_button.rect.height / 2.0;

        assert_eq!(chrome.hit_test(center_x, center_y), Some(ChromeHit::BackButton));
    }

    #[test]
    fn test_hit_test_address_bar() {
        let chrome = Chrome::new(800.0);
        let center_x = chrome.address_bar.rect.x + chrome.address_bar.rect.width / 2.0;
        let center_y = chrome.address_bar.rect.y + chrome.address_bar.rect.height / 2.0;

        assert_eq!(chrome.hit_test(center_x, center_y), Some(ChromeHit::AddressBar));
    }

    #[test]
    fn test_hit_test_below_chrome() {
        let chrome = Chrome::new(800.0);
        assert_eq!(chrome.hit_test(400.0, CHROME_HEIGHT + 10.0), None);
    }

    #[test]
    fn test_address_bar_insert() {
        let mut bar = AddressBar {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 32.0,
            },
            text: String::new(),
            cursor_pos: 0,
            is_focused: true,
        };

        bar.insert_char('h');
        bar.insert_char('i');

        assert_eq!(bar.text, "hi");
        assert_eq!(bar.cursor_pos, 2);
    }

    #[test]
    fn test_address_bar_backspace() {
        let mut bar = AddressBar {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 32.0,
            },
            text: String::from("hello"),
            cursor_pos: 5,
            is_focused: true,
        };

        bar.delete_char();
        assert_eq!(bar.text, "hell");
        assert_eq!(bar.cursor_pos, 4);

        bar.delete_char();
        assert_eq!(bar.text, "hel");
        assert_eq!(bar.cursor_pos, 3);
    }

    #[test]
    fn test_address_bar_set_text() {
        let mut bar = AddressBar {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 32.0,
            },
            text: String::new(),
            cursor_pos: 0,
            is_focused: false,
        };

        bar.set_text("https://example.com");
        assert_eq!(bar.text, "https://example.com");
        assert_eq!(bar.cursor_pos, 19);
    }

    #[test]
    fn test_navigation_state_update() {
        let mut chrome = Chrome::new(800.0);

        assert!(!chrome.back_button.enabled);
        assert!(!chrome.forward_button.enabled);

        chrome.update_navigation_state(true, false);
        assert!(chrome.back_button.enabled);
        assert!(!chrome.forward_button.enabled);

        chrome.update_navigation_state(true, true);
        assert!(chrome.back_button.enabled);
        assert!(chrome.forward_button.enabled);
    }
}

//! SDL event handling
//!
//! Polls SDL events and converts them to browser events.

/// Browser event types
#[derive(Debug, Clone)]
pub enum BrowserEvent {
    /// Quit the browser
    Quit,
    /// Mouse button pressed
    MouseDown { x: f32, y: f32, button: MouseButton },
    /// Mouse moved
    MouseMove { x: f32, y: f32 },
    /// Mouse wheel scrolled
    MouseWheel { x: i32, y: i32 },
    /// Key pressed
    KeyDown { scancode: u32 },
    /// Text input (for address bar)
    TextInput { text: String },
    /// Window resize
    WindowResize { width: u32, height: u32 },
}

/// Mouse button types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    Other(u8),
}

// SDL event type constants
const SDL_QUIT: u32 = 0x100;
const SDL_KEYDOWN: u32 = 0x300;
const SDL_TEXTINPUT: u32 = 0x303;
const SDL_MOUSEMOTION: u32 = 0x400;
const SDL_MOUSEBUTTONDOWN: u32 = 0x401;
const SDL_MOUSEWHEEL: u32 = 0x403;
const SDL_WINDOWEVENT: u32 = 0x200;

// SDL scancode constants
pub const SCANCODE_ESCAPE: u32 = 41;
pub const SCANCODE_Q: u32 = 20;
pub const SCANCODE_BACKSPACE: u32 = 42;
pub const SCANCODE_RETURN: u32 = 40;

// Scroll-related scancodes
pub const SCANCODE_UP: u32 = 82;
pub const SCANCODE_DOWN: u32 = 81;
pub const SCANCODE_PAGEUP: u32 = 75;
pub const SCANCODE_PAGEDOWN: u32 = 78;
pub const SCANCODE_HOME: u32 = 74;
pub const SCANCODE_END: u32 = 77;

// SDL window event subtypes
const SDL_WINDOWEVENT_CLOSE: u8 = 14;
const SDL_WINDOWEVENT_SIZE_CHANGED: u8 = 6;

/// Poll all pending SDL events
///
/// # Safety
/// This function uses raw SDL2 calls.
pub fn poll_events() -> Vec<BrowserEvent> {
    let mut events = Vec::new();

    unsafe {
        let mut raw_event: sdl2::sys::SDL_Event = std::mem::zeroed();

        while sdl2::sys::SDL_PollEvent(&mut raw_event) != 0 {
            let event_type = raw_event.type_;

            match event_type {
                SDL_QUIT => {
                    events.push(BrowserEvent::Quit);
                }

                SDL_KEYDOWN => {
                    let key_event = raw_event.key;
                    let scancode = key_event.keysym.scancode as u32;
                    events.push(BrowserEvent::KeyDown { scancode });
                }

                SDL_TEXTINPUT => {
                    let text_event = raw_event.text;
                    // Convert C string to Rust string
                    let c_str = std::ffi::CStr::from_ptr(text_event.text.as_ptr());
                    if let Ok(text) = c_str.to_str() {
                        if !text.is_empty() {
                            events.push(BrowserEvent::TextInput {
                                text: text.to_string(),
                            });
                        }
                    }
                }

                SDL_MOUSEMOTION => {
                    let motion_event = raw_event.motion;
                    events.push(BrowserEvent::MouseMove {
                        x: motion_event.x as f32,
                        y: motion_event.y as f32,
                    });
                }

                SDL_MOUSEBUTTONDOWN => {
                    let button_event = raw_event.button;
                    let button = match button_event.button {
                        1 => MouseButton::Left,
                        2 => MouseButton::Middle,
                        3 => MouseButton::Right,
                        b => MouseButton::Other(b),
                    };
                    events.push(BrowserEvent::MouseDown {
                        x: button_event.x as f32,
                        y: button_event.y as f32,
                        button,
                    });
                }

                SDL_MOUSEWHEEL => {
                    let wheel_event = raw_event.wheel;
                    events.push(BrowserEvent::MouseWheel {
                        x: wheel_event.x,
                        y: wheel_event.y,
                    });
                }

                SDL_WINDOWEVENT => {
                    let window_event = raw_event.window;
                    match window_event.event {
                        SDL_WINDOWEVENT_CLOSE => {
                            events.push(BrowserEvent::Quit);
                        }
                        SDL_WINDOWEVENT_SIZE_CHANGED => {
                            events.push(BrowserEvent::WindowResize {
                                width: window_event.data1 as u32,
                                height: window_event.data2 as u32,
                            });
                        }
                        _ => {}
                    }
                }

                _ => {
                    // Ignore unknown events
                }
            }
        }
    }

    events
}

/// Enable SDL text input mode
///
/// Must be called when the address bar gains focus.
pub fn start_text_input() {
    unsafe {
        sdl2::sys::SDL_StartTextInput();
    }
}

/// Disable SDL text input mode
///
/// Must be called when the address bar loses focus.
pub fn stop_text_input() {
    unsafe {
        sdl2::sys::SDL_StopTextInput();
    }
}

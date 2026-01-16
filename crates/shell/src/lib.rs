//! Gugalanna Browser Shell
//!
//! Browser window, event handling, and UI.

// TODO: Epic 6 - Browser Shell
// - SDL window creation
// - Event loop
// - Input handling
// - Scrolling

/// Browser configuration
#[derive(Debug, Clone)]
pub struct BrowserConfig {
    pub width: u32,
    pub height: u32,
    pub title: String,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            width: 1024,
            height: 768,
            title: String::from("Gugalanna"),
        }
    }
}

/// Browser window state
pub struct Browser {
    pub config: BrowserConfig,
    // Will hold SDL window, DOM tree, etc.
}

impl Browser {
    /// Create a new browser with the given configuration
    pub fn new(config: BrowserConfig) -> Self {
        Self { config }
    }

    /// Navigate to a URL
    pub fn navigate(&mut self, _url: &str) {
        // TODO: Implement
    }

    /// Run the browser event loop
    pub fn run(&mut self) {
        // TODO: Implement with SDL
    }
}

//! Gugalanna Browser Shell
//!
//! Browser window, event handling, and UI.

mod chrome;
mod event;
mod navigation;

pub use chrome::{Chrome, ChromeHit, CHROME_HEIGHT};
pub use navigation::NavigationState;

use std::cell::RefCell;
use std::rc::Rc;

use url::Url;

use gugalanna_css::Stylesheet;
use gugalanna_dom::{DomTree, Queryable};
use gugalanna_html::HtmlParser;
use gugalanna_js::JsRuntime;
use gugalanna_layout::{build_layout_tree, layout_block, BoxType, ContainingBlock, LayoutBox};
use gugalanna_net::HttpClient;
use gugalanna_render::{build_display_list, DisplayList, RenderBackend, RenderColor, SdlBackend};
use gugalanna_style::{Cascade, StyleTree};

use crate::event::{poll_events, start_text_input, stop_text_input, BrowserEvent, MouseButton};

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

/// Input focus target
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusTarget {
    None,
    AddressBar,
    Page,
}

/// Scroll constants
const SCROLL_LINE_HEIGHT: f32 = 40.0; // Arrow keys scroll amount
const SCROLL_PAGE_FACTOR: f32 = 0.9; // Page Up/Down scrolls 90% of viewport
const SCROLL_WHEEL_MULTIPLIER: f32 = 40.0; // Mouse wheel multiplier

/// Page state (rendered content)
struct PageState {
    /// Current URL
    url: Url,
    /// Display list for rendering
    display_list: DisplayList,
    /// JavaScript runtime (for event handling)
    js_runtime: Option<JsRuntime>,
    /// Layout tree for hit testing (stored as display list node IDs)
    hit_regions: Vec<HitRegion>,
    /// Current vertical scroll offset (0 = top)
    scroll_y: f32,
    /// Total content height
    content_height: f32,
    /// Visible viewport height (window height - chrome height)
    viewport_height: f32,
    /// DOM tree (for re-layout on resize)
    dom: Rc<RefCell<DomTree>>,
    /// CSS cascade (for re-layout on resize)
    cascade: Cascade,
}

/// Hit region for click handling
struct HitRegion {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    node_id: u32,
}

/// Browser window state
pub struct Browser {
    pub config: BrowserConfig,
    backend: SdlBackend,
    chrome: Chrome,
    navigation: NavigationState,
    page: Option<PageState>,
    focus: FocusTarget,
    http_client: HttpClient,
}

impl Browser {
    /// Create a new browser with the given configuration
    pub fn new(config: BrowserConfig) -> Result<Self, String> {
        let backend =
            SdlBackend::new(&config.title, config.width, config.height).map_err(|e| e.to_string())?;

        let chrome = Chrome::new(config.width as f32);

        let http_client = HttpClient::new().map_err(|e| e.to_string())?;

        Ok(Self {
            config,
            backend,
            chrome,
            navigation: NavigationState::new(),
            page: None,
            focus: FocusTarget::None,
            http_client,
        })
    }

    /// Navigate to a URL
    pub fn navigate(&mut self, url_str: &str) -> Result<(), String> {
        // Parse URL
        let url = if url_str.contains("://") {
            Url::parse(url_str).map_err(|e| e.to_string())?
        } else {
            Url::parse(&format!("https://{}", url_str)).map_err(|e| e.to_string())?
        };

        log::info!("Navigating to: {}", url);

        // Update address bar
        self.chrome.address_bar.set_text(url.as_str());

        // Fetch the page - use block_in_place to allow blocking in async context
        let response = self.fetch_url(&url)?;

        if !response.is_success() {
            return Err(format!("HTTP error: {}", response.status));
        }

        let html = response.text_lossy();
        log::info!("Received {} bytes", html.len());

        // Load the page
        self.load_page(url, &html)?;

        Ok(())
    }

    /// Load HTML content directly (for demos and local content)
    pub fn load_html(&mut self, html: &str, css: &str) -> Result<(), String> {
        // Use about:blank as the URL
        let url = Url::parse("about:blank").map_err(|e| e.to_string())?;

        // Update address bar
        self.chrome.address_bar.set_text("about:blank");

        // Load with custom CSS
        self.load_page_with_css(url, html, css)
    }

    /// Load HTML content into a page
    fn load_page(&mut self, url: Url, html: &str) -> Result<(), String> {
        let default_css = r#"
            body { background-color: white; color: black; font-size: 16px; }
            h1, h2, h3, h4, h5, h6, p, div { display: block; }
            h1 { font-size: 32px; margin-top: 20px; margin-bottom: 10px; }
            h2 { font-size: 24px; margin-top: 18px; margin-bottom: 8px; }
            h3 { font-size: 18px; margin-top: 16px; margin-bottom: 6px; }
            p { margin-top: 10px; margin-bottom: 10px; }
        "#;
        self.load_page_with_css(url, html, default_css)
    }

    /// Load HTML content with custom CSS
    fn load_page_with_css(&mut self, url: Url, html: &str, css: &str) -> Result<(), String> {
        // Parse HTML
        let dom = HtmlParser::new().parse(html).map_err(|e| e.to_string())?;

        // Create JS runtime with DOM bindings
        let js_runtime = JsRuntime::with_dom(dom).ok();

        // Get DOM reference
        let shared_dom = match js_runtime.as_ref().and_then(|rt| rt.dom()) {
            Some(dom) => dom.clone(),
            None => {
                // Fallback: create DOM without JS
                let dom = HtmlParser::new().parse(html).map_err(|e| e.to_string())?;
                Rc::new(RefCell::new(dom))
            }
        };

        // Execute scripts
        if let Some(ref rt) = js_runtime {
            if let Err(e) = rt.execute_scripts() {
                log::warn!("Script execution error: {}", e);
            }
        }

        // Parse CSS and build cascade
        let mut cascade = Cascade::new();
        if let Ok(stylesheet) = Stylesheet::parse(css) {
            cascade.add_author_stylesheet(stylesheet);
        }

        // Calculate viewport (below chrome)
        let viewport_width = self.config.width as f32;
        let viewport_height = self.config.height as f32 - CHROME_HEIGHT;

        // Build style and layout trees
        let dom_ref = shared_dom.borrow();
        let style_tree = StyleTree::build(&*dom_ref, &cascade, viewport_width, viewport_height);

        let body_ids = dom_ref.get_elements_by_tag_name("body");
        let root_id = if !body_ids.is_empty() {
            body_ids[0]
        } else {
            dom_ref.document_id()
        };

        let mut layout_tree = match build_layout_tree(&*dom_ref, &style_tree, root_id) {
            Some(tree) => tree,
            None => return Err("Failed to build layout tree".into()),
        };

        // Perform layout
        layout_block(
            &mut layout_tree,
            ContainingBlock::new(viewport_width, viewport_height),
        );

        // Get content height for scrolling
        let content_height = layout_tree.dimensions.margin_box_height();

        // Build display list
        let display_list = build_display_list(&layout_tree);

        // Build hit regions
        let hit_regions = build_hit_regions(&layout_tree);

        // Drop DOM borrow
        drop(dom_ref);

        // Update navigation
        self.navigation.navigate_to(url.clone());
        self.chrome.update_navigation_state(
            self.navigation.can_go_back(),
            self.navigation.can_go_forward(),
        );

        // Store page state
        self.page = Some(PageState {
            url,
            display_list,
            js_runtime,
            hit_regions,
            scroll_y: 0.0,
            content_height,
            viewport_height,
            dom: shared_dom.clone(),
            cascade,
        });

        log::info!(
            "Page loaded with {} paint commands",
            self.page.as_ref().map(|p| p.display_list.commands.len()).unwrap_or(0)
        );

        Ok(())
    }

    /// Go back in history
    pub fn go_back(&mut self) -> Result<(), String> {
        if let Some(url) = self.navigation.go_back().cloned() {
            self.chrome.address_bar.set_text(url.as_str());
            self.reload_url(url)?;
        }
        Ok(())
    }

    /// Go forward in history
    pub fn go_forward(&mut self) -> Result<(), String> {
        if let Some(url) = self.navigation.go_forward().cloned() {
            self.chrome.address_bar.set_text(url.as_str());
            self.reload_url(url)?;
        }
        Ok(())
    }

    /// Reload a URL (for back/forward)
    fn reload_url(&mut self, url: Url) -> Result<(), String> {
        let response = self.fetch_url(&url)?;

        if !response.is_success() {
            return Err(format!("HTTP error: {}", response.status));
        }

        let html = response.text_lossy();
        self.load_page_without_history(url, &html)
    }

    /// Fetch a URL, handling both sync and async contexts
    fn fetch_url(&self, url: &Url) -> Result<gugalanna_net::Response, String> {
        use tokio::runtime::Handle;

        // Check if we're already in a tokio runtime
        if let Ok(handle) = Handle::try_current() {
            // We're in an async context - use block_in_place
            tokio::task::block_in_place(|| {
                handle.block_on(self.http_client.get(url))
            })
            .map_err(|e| e.to_string())
        } else {
            // No runtime - create one
            let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
            rt.block_on(self.http_client.get(url))
                .map_err(|e| e.to_string())
        }
    }

    /// Load page without adding to history (for back/forward)
    fn load_page_without_history(&mut self, url: Url, html: &str) -> Result<(), String> {
        // Similar to load_page but doesn't update navigation
        let dom = HtmlParser::new().parse(html).map_err(|e| e.to_string())?;
        let js_runtime = JsRuntime::with_dom(dom).ok();

        let shared_dom = match js_runtime.as_ref().and_then(|rt| rt.dom()) {
            Some(dom) => dom.clone(),
            None => {
                let dom = HtmlParser::new().parse(html).map_err(|e| e.to_string())?;
                Rc::new(RefCell::new(dom))
            }
        };

        if let Some(ref rt) = js_runtime {
            let _ = rt.execute_scripts();
        }

        let mut cascade = Cascade::new();
        let css = "body { background-color: white; color: black; font-size: 16px; }";
        if let Ok(stylesheet) = Stylesheet::parse(css) {
            cascade.add_author_stylesheet(stylesheet);
        }

        // Calculate viewport (below chrome)
        let viewport_width = self.config.width as f32;
        let viewport_height = self.config.height as f32 - CHROME_HEIGHT;

        let dom_ref = shared_dom.borrow();
        let style_tree = StyleTree::build(&*dom_ref, &cascade, viewport_width, viewport_height);

        let body_ids = dom_ref.get_elements_by_tag_name("body");
        let root_id = if !body_ids.is_empty() {
            body_ids[0]
        } else {
            dom_ref.document_id()
        };

        let mut layout_tree = match build_layout_tree(&*dom_ref, &style_tree, root_id) {
            Some(tree) => tree,
            None => return Err("Failed to build layout tree".into()),
        };

        layout_block(
            &mut layout_tree,
            ContainingBlock::new(viewport_width, viewport_height),
        );

        // Get content height for scrolling
        let content_height = layout_tree.dimensions.margin_box_height();

        let display_list = build_display_list(&layout_tree);
        let hit_regions = build_hit_regions(&layout_tree);
        drop(dom_ref);

        // Update chrome buttons
        self.chrome.update_navigation_state(
            self.navigation.can_go_back(),
            self.navigation.can_go_forward(),
        );

        self.page = Some(PageState {
            url,
            display_list,
            js_runtime,
            hit_regions,
            scroll_y: 0.0,
            content_height,
            viewport_height,
            dom: shared_dom.clone(),
            cascade,
        });

        Ok(())
    }

    /// Run the browser event loop
    pub fn run(&mut self) -> Result<(), String> {
        'running: loop {
            // Poll events
            let events = poll_events();

            for event in events {
                match event {
                    BrowserEvent::Quit => {
                        break 'running;
                    }

                    BrowserEvent::KeyDown { scancode } => {
                        if self.handle_key(scancode) {
                            break 'running;
                        }
                    }

                    BrowserEvent::TextInput { text } => {
                        self.handle_text_input(&text);
                    }

                    BrowserEvent::MouseDown { x, y, button } => {
                        if button == MouseButton::Left {
                            self.handle_click(x, y);
                        }
                    }

                    BrowserEvent::MouseWheel { y, .. } => {
                        // Scroll page (y > 0 = scroll up, y < 0 = scroll down)
                        let delta = y as f32 * SCROLL_WHEEL_MULTIPLIER;
                        self.handle_scroll(delta);
                    }

                    BrowserEvent::WindowResize { width, height } => {
                        self.config.width = width;
                        self.config.height = height;
                        self.chrome.update_width(width as f32);
                        self.relayout_page();
                    }
                }
            }

            // Render
            self.render();

            // Small sleep to avoid busy-waiting (~60 FPS)
            std::thread::sleep(std::time::Duration::from_millis(16));
        }

        Ok(())
    }

    /// Handle a key press
    ///
    /// Returns true if the browser should quit.
    fn handle_key(&mut self, scancode: u32) -> bool {
        use crate::event::{
            SCANCODE_BACKSPACE, SCANCODE_DOWN, SCANCODE_END, SCANCODE_ESCAPE, SCANCODE_HOME,
            SCANCODE_PAGEDOWN, SCANCODE_PAGEUP, SCANCODE_Q, SCANCODE_RETURN, SCANCODE_UP,
        };

        match scancode {
            SCANCODE_ESCAPE | SCANCODE_Q if self.focus != FocusTarget::AddressBar => {
                return true;
            }

            SCANCODE_BACKSPACE if self.focus == FocusTarget::AddressBar => {
                self.chrome.address_bar.delete_char();
            }

            SCANCODE_RETURN if self.focus == FocusTarget::AddressBar => {
                // Navigate to URL in address bar
                let url = self.chrome.address_bar.text.clone();
                if !url.is_empty() {
                    if let Err(e) = self.navigate(&url) {
                        log::error!("Navigation failed: {}", e);
                    }
                }
                self.blur_address_bar();
            }

            SCANCODE_ESCAPE if self.focus == FocusTarget::AddressBar => {
                self.blur_address_bar();
            }

            // Scroll keys (only when not editing address bar)
            SCANCODE_UP if self.focus != FocusTarget::AddressBar => {
                self.handle_scroll(SCROLL_LINE_HEIGHT);
            }

            SCANCODE_DOWN if self.focus != FocusTarget::AddressBar => {
                self.handle_scroll(-SCROLL_LINE_HEIGHT);
            }

            SCANCODE_PAGEUP if self.focus != FocusTarget::AddressBar => {
                if let Some(ref page) = self.page {
                    let delta = page.viewport_height * SCROLL_PAGE_FACTOR;
                    self.handle_scroll(delta);
                }
            }

            SCANCODE_PAGEDOWN if self.focus != FocusTarget::AddressBar => {
                if let Some(ref page) = self.page {
                    let delta = page.viewport_height * SCROLL_PAGE_FACTOR;
                    self.handle_scroll(-delta);
                }
            }

            SCANCODE_HOME if self.focus != FocusTarget::AddressBar => {
                self.scroll_to_top();
            }

            SCANCODE_END if self.focus != FocusTarget::AddressBar => {
                self.scroll_to_bottom();
            }

            _ => {}
        }

        false
    }

    /// Handle text input (for address bar)
    fn handle_text_input(&mut self, text: &str) {
        if self.focus == FocusTarget::AddressBar {
            for c in text.chars() {
                self.chrome.address_bar.insert_char(c);
            }
        }
    }

    /// Handle scroll by delta (positive = scroll up/show content above, negative = scroll down)
    fn handle_scroll(&mut self, delta: f32) {
        if let Some(ref mut page) = self.page {
            let max_scroll = (page.content_height - page.viewport_height).max(0.0);
            page.scroll_y = (page.scroll_y - delta).clamp(0.0, max_scroll);
        }
    }

    /// Scroll to the top of the page
    fn scroll_to_top(&mut self) {
        if let Some(ref mut page) = self.page {
            page.scroll_y = 0.0;
        }
    }

    /// Scroll to the bottom of the page
    fn scroll_to_bottom(&mut self) {
        if let Some(ref mut page) = self.page {
            let max_scroll = (page.content_height - page.viewport_height).max(0.0);
            page.scroll_y = max_scroll;
        }
    }

    /// Re-layout the page with new viewport dimensions
    fn relayout_page(&mut self) {
        if let Some(ref mut page) = self.page {
            let viewport_width = self.config.width as f32;
            let viewport_height = self.config.height as f32 - CHROME_HEIGHT;

            let dom_ref = page.dom.borrow();

            // Rebuild style tree with new viewport dimensions
            let style_tree = StyleTree::build(&*dom_ref, &page.cascade, viewport_width, viewport_height);

            // Get root element
            let body_ids = dom_ref.get_elements_by_tag_name("body");
            let root_id = if !body_ids.is_empty() {
                body_ids[0]
            } else {
                dom_ref.document_id()
            };

            // Build and perform layout
            if let Some(mut layout_tree) = build_layout_tree(&*dom_ref, &style_tree, root_id) {
                layout_block(
                    &mut layout_tree,
                    ContainingBlock::new(viewport_width, viewport_height),
                );

                // Update content height
                let content_height = layout_tree.dimensions.margin_box_height();

                // Rebuild display list and hit regions
                let display_list = build_display_list(&layout_tree);
                let hit_regions = build_hit_regions(&layout_tree);

                // Update page state
                page.display_list = display_list;
                page.hit_regions = hit_regions;
                page.content_height = content_height;
                page.viewport_height = viewport_height;

                // Clamp scroll position to new content bounds
                let max_scroll = (content_height - viewport_height).max(0.0);
                page.scroll_y = page.scroll_y.clamp(0.0, max_scroll);
            }
        }
    }

    /// Handle a mouse click
    fn handle_click(&mut self, x: f32, y: f32) {
        // Check chrome first
        if let Some(hit) = self.chrome.hit_test(x, y) {
            match hit {
                ChromeHit::BackButton => {
                    if self.chrome.back_button.enabled {
                        if let Err(e) = self.go_back() {
                            log::error!("Go back failed: {}", e);
                        }
                    }
                }
                ChromeHit::ForwardButton => {
                    if self.chrome.forward_button.enabled {
                        if let Err(e) = self.go_forward() {
                            log::error!("Go forward failed: {}", e);
                        }
                    }
                }
                ChromeHit::GoButton => {
                    let url = self.chrome.address_bar.text.clone();
                    if !url.is_empty() {
                        if let Err(e) = self.navigate(&url) {
                            log::error!("Navigation failed: {}", e);
                        }
                    }
                }
                ChromeHit::AddressBar => {
                    self.focus_address_bar();
                }
            }
            return;
        }

        // Blur address bar if clicking outside
        if self.focus == FocusTarget::AddressBar {
            self.blur_address_bar();
        }

        // Check page content
        let page_y = y - CHROME_HEIGHT;
        if page_y >= 0.0 {
            if let Some(ref mut page) = self.page {
                // Adjust for scroll: click at page_y corresponds to content at page_y + scroll_y
                let content_y = page_y + page.scroll_y;
                // Hit test page
                if let Some(node_id) = hit_test_regions(&page.hit_regions, x, content_y) {
                    log::debug!("Page click on node {}", node_id);
                    // Dispatch click to JS
                    if let Some(ref rt) = page.js_runtime {
                        if let Err(e) = rt.dispatch_click(node_id) {
                            log::warn!("Click dispatch failed: {}", e);
                        }
                    }
                }
            }
        }
    }

    /// Focus the address bar
    fn focus_address_bar(&mut self) {
        self.focus = FocusTarget::AddressBar;
        self.chrome.address_bar.is_focused = true;
        self.chrome.address_bar.move_cursor_to_end();
        start_text_input();
    }

    /// Blur the address bar
    fn blur_address_bar(&mut self) {
        self.focus = FocusTarget::None;
        self.chrome.address_bar.is_focused = false;
        stop_text_input();
    }

    /// Render the browser
    fn render(&mut self) {
        // Clear with white
        self.backend.clear(RenderColor::white());

        // Render chrome
        let chrome_display_list = self.chrome.build_display_list();
        self.backend.render(&chrome_display_list);

        // Render page content (offset by chrome height and scroll)
        // Clone the display list and scroll_y to avoid borrow issues
        let page_data = self.page.as_ref().map(|p| (p.display_list.clone(), p.scroll_y));
        if let Some((display_list, scroll_y)) = page_data {
            self.render_page(&display_list, scroll_y);
        }

        // Present
        self.backend.present();
    }

    /// Render page content with Y offset (chrome height) and scroll offset
    fn render_page(&mut self, display_list: &DisplayList, scroll_y: f32) {
        use gugalanna_layout::Rect;
        use gugalanna_render::PaintCommand;

        // Combined offset: chrome pushes content down, scroll moves it up
        let y_offset = CHROME_HEIGHT - scroll_y;
        let viewport_bottom = self.config.height as f32;

        // Offset all commands by combined offset
        let mut offset_commands = Vec::with_capacity(display_list.commands.len());

        for cmd in &display_list.commands {
            match cmd {
                PaintCommand::FillRect { rect, color } => {
                    let new_y = rect.y + y_offset;
                    // Skip if completely off-screen (above chrome or below viewport)
                    if new_y + rect.height < CHROME_HEIGHT || new_y > viewport_bottom {
                        continue;
                    }
                    offset_commands.push(PaintCommand::FillRect {
                        rect: Rect {
                            x: rect.x,
                            y: new_y,
                            width: rect.width,
                            height: rect.height,
                        },
                        color: *color,
                    });
                }
                PaintCommand::DrawText {
                    text,
                    x,
                    y,
                    color,
                    font_size,
                } => {
                    let new_y = *y + y_offset;
                    // Skip if text is off-screen (approximate with font_size as height)
                    if new_y + *font_size < CHROME_HEIGHT || new_y > viewport_bottom {
                        continue;
                    }
                    offset_commands.push(PaintCommand::DrawText {
                        text: text.clone(),
                        x: *x,
                        y: new_y,
                        color: *color,
                        font_size: *font_size,
                    });
                }
                PaintCommand::DrawBorder {
                    rect,
                    widths,
                    color,
                } => {
                    let new_y = rect.y + y_offset;
                    // Skip if completely off-screen
                    if new_y + rect.height < CHROME_HEIGHT || new_y > viewport_bottom {
                        continue;
                    }
                    offset_commands.push(PaintCommand::DrawBorder {
                        rect: Rect {
                            x: rect.x,
                            y: new_y,
                            width: rect.width,
                            height: rect.height,
                        },
                        widths: *widths,
                        color: *color,
                    });
                }
            }
        }

        let offset_list = DisplayList {
            commands: offset_commands,
        };
        self.backend.render(&offset_list);
    }
}

/// Build hit regions from layout tree
fn build_hit_regions(layout: &LayoutBox) -> Vec<HitRegion> {
    let mut regions = Vec::new();
    build_hit_regions_recursive(layout, &mut regions);
    regions
}

fn build_hit_regions_recursive(layout: &LayoutBox, regions: &mut Vec<HitRegion>) {
    let rect = &layout.dimensions.content;

    // Get node ID from box type
    let node_id = match &layout.box_type {
        BoxType::Block(id, _) => Some(id.0),
        BoxType::Inline(id, _) => Some(id.0),
        BoxType::Text(id, _, _) => Some(id.0),
        BoxType::AnonymousBlock | BoxType::AnonymousInline => None,
    };

    if let Some(id) = node_id {
        if rect.width > 0.0 && rect.height > 0.0 {
            regions.push(HitRegion {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
                node_id: id,
            });
        }
    }

    // Process children
    for child in &layout.children {
        build_hit_regions_recursive(child, regions);
    }
}

/// Hit test hit regions
fn hit_test_regions(regions: &[HitRegion], x: f32, y: f32) -> Option<u32> {
    // Test in reverse order (later elements are on top)
    for region in regions.iter().rev() {
        if x >= region.x
            && x <= region.x + region.width
            && y >= region.y
            && y <= region.y + region.height
        {
            return Some(region.node_id);
        }
    }
    None
}

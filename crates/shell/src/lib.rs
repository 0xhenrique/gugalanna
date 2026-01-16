//! Gugalanna Browser Shell
//!
//! Browser window, event handling, and UI.

mod chrome;
mod event;
mod form;
mod image_loader;
mod loading;
mod navigation;
mod transition;

pub use chrome::{Chrome, ChromeHit, CHROME_HEIGHT};
pub use loading::{LoadingState, NavigationError, NavigationResult};
pub use navigation::NavigationState;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use url::Url;

use crate::transition::TransitionManager;

use gugalanna_css::Stylesheet;
use gugalanna_dom::{DomTree, NodeId, Queryable};
use gugalanna_html::HtmlParser;
use gugalanna_js::JsRuntime;
use gugalanna_layout::{build_layout_tree, layout_block, BoxType, ContainingBlock, LayoutBox};
use gugalanna_net::HttpClient;
use gugalanna_render::{build_display_list, CursorType, DisplayList, RenderBackend, RenderColor, SdlBackend};
use gugalanna_style::{Cascade, StyleTree};

use crate::event::{poll_events, start_text_input, stop_text_input, BrowserEvent, Modifiers, MouseButton};
use crate::form::FormState;

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
    /// A form input element is focused
    FormInput(NodeId),
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

/// Unique tab identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TabId(pub u32);

/// State for a single browser tab
pub struct TabState {
    /// Unique tab identifier
    pub id: TabId,
    /// Navigation history for this tab
    pub navigation: NavigationState,
    /// Current page state (rendered content)
    pub page: Option<PageState>,
    /// Current loading state for this tab
    pub loading_state: LoadingState,
    /// Receiver for navigation results from async task
    pub nav_receiver: Option<tokio::sync::mpsc::Receiver<NavigationResult>>,
    /// Cancellation token for current navigation
    pub nav_cancel: Option<tokio_util::sync::CancellationToken>,
    /// Form state for this tab
    pub form_state: FormState,
}

impl TabState {
    /// Create a new empty tab
    pub fn new(id: TabId) -> Self {
        Self {
            id,
            navigation: NavigationState::new(),
            page: None,
            loading_state: LoadingState::default(),
            nav_receiver: None,
            nav_cancel: None,
            form_state: FormState::new(),
        }
    }

    /// Get the tab's title (URL host or "New Tab")
    pub fn title(&self) -> String {
        if let Some(url) = self.navigation.current_url() {
            url.host_str()
                .map(|h| h.to_string())
                .unwrap_or_else(|| {
                    // Fall back to path or full URL
                    let path = url.path();
                    if path.len() > 1 {
                        path.to_string()
                    } else {
                        url.as_str().to_string()
                    }
                })
        } else {
            "New Tab".to_string()
        }
    }

    /// Check if this tab is loading
    pub fn is_loading(&self) -> bool {
        matches!(self.loading_state, LoadingState::Loading { .. })
    }

    /// Get current URL as string (for address bar display)
    pub fn current_url_string(&self) -> String {
        self.navigation
            .current_url()
            .map(|u| u.as_str().to_string())
            .unwrap_or_default()
    }
}

/// Browser window state
pub struct Browser {
    pub config: BrowserConfig,
    backend: SdlBackend,
    chrome: Chrome,
    /// All open tabs
    tabs: Vec<TabState>,
    /// Active tab ID
    active_tab_id: TabId,
    /// Next tab ID to assign
    next_tab_id: u32,
    /// Input focus target
    focus: FocusTarget,
    /// HTTP client (shared across all tabs)
    http_client: HttpClient,
    /// Current cursor type
    current_cursor: CursorType,
    /// Transition manager for CSS transitions
    transition_manager: TransitionManager,
    /// Last frame timestamp for delta time calculation
    last_frame: Instant,
}

impl Browser {
    /// Create a new browser with the given configuration
    pub fn new(config: BrowserConfig) -> Result<Self, String> {
        let backend =
            SdlBackend::new(&config.title, config.width, config.height).map_err(|e| e.to_string())?;

        let mut chrome = Chrome::new(config.width as f32);

        let http_client = HttpClient::new().map_err(|e| e.to_string())?;

        // Create initial tab
        let initial_tab_id = TabId(0);
        let initial_tab = TabState::new(initial_tab_id);

        // Initialize tab bar with first tab
        let tab_infos = vec![(initial_tab_id, initial_tab.title(), false, true)];
        chrome.layout_tabs(&tab_infos, initial_tab_id);

        Ok(Self {
            config,
            backend,
            chrome,
            tabs: vec![initial_tab],
            active_tab_id: initial_tab_id,
            next_tab_id: 1,
            focus: FocusTarget::None,
            http_client,
            current_cursor: CursorType::Arrow,
            transition_manager: TransitionManager::new(),
            last_frame: Instant::now(),
        })
    }

    // ==================== Tab Helper Methods ====================

    /// Get the active tab (immutable)
    fn active_tab(&self) -> Option<&TabState> {
        self.tabs.iter().find(|t| t.id == self.active_tab_id)
    }

    /// Get the active tab (mutable)
    fn active_tab_mut(&mut self) -> Option<&mut TabState> {
        let active_id = self.active_tab_id;
        self.tabs.iter_mut().find(|t| t.id == active_id)
    }

    /// Get a tab by ID (mutable)
    fn tab_mut(&mut self, id: TabId) -> Option<&mut TabState> {
        self.tabs.iter_mut().find(|t| t.id == id)
    }

    /// Get index of tab by ID
    fn tab_index(&self, id: TabId) -> Option<usize> {
        self.tabs.iter().position(|t| t.id == id)
    }

    /// Sync Chrome UI with current tab state
    fn sync_chrome_with_tabs(&mut self) {
        // Collect tab info for layout
        let tab_infos: Vec<(TabId, String, bool, bool)> = self
            .tabs
            .iter()
            .map(|tab| {
                (
                    tab.id,
                    tab.title(),
                    tab.is_loading(),
                    tab.id == self.active_tab_id,
                )
            })
            .collect();

        let active_id = self.active_tab_id;

        // Update tab bar layout
        self.chrome.layout_tabs(&tab_infos, active_id);

        // Extract values for active tab
        let tab_data = self.active_tab().map(|tab| {
            (
                tab.current_url_string(),
                tab.navigation.can_go_back(),
                tab.navigation.can_go_forward(),
                tab.is_loading(),
            )
        });

        // Update chrome UI
        if let Some((url, can_back, can_forward, is_loading)) = tab_data {
            self.chrome.address_bar.set_text(&url);
            self.chrome.update_navigation_state(can_back, can_forward);
            self.chrome.is_loading = is_loading;
        }
    }

    // ==================== Tab Management ====================

    /// Create a new tab and switch to it
    pub fn new_tab(&mut self) -> TabId {
        let id = TabId(self.next_tab_id);
        self.next_tab_id += 1;

        let tab = TabState::new(id);
        self.tabs.push(tab);

        self.switch_to_tab(id);

        log::info!("Created new tab {}", id.0);
        id
    }

    /// Close a tab by ID
    /// Returns true if the browser should quit (last tab closed)
    pub fn close_tab(&mut self, id: TabId) -> bool {
        // Don't allow closing the last tab directly - quit instead
        if self.tabs.len() <= 1 {
            log::info!("Closing last tab - quitting browser");
            return true;
        }

        let index = match self.tab_index(id) {
            Some(i) => i,
            None => return false,
        };

        // Cancel any in-progress navigation for this tab
        if let Some(tab) = self.tab_mut(id) {
            if let Some(cancel) = tab.nav_cancel.take() {
                cancel.cancel();
            }
        }

        // Remove the tab
        self.tabs.remove(index);

        // If we closed the active tab, switch to an adjacent one
        if id == self.active_tab_id {
            let new_index = index.min(self.tabs.len() - 1);
            let new_active_id = self.tabs[new_index].id;
            self.switch_to_tab(new_active_id);
        } else {
            self.sync_chrome_with_tabs();
        }

        log::info!("Closed tab {}", id.0);
        false
    }

    /// Switch to a tab by ID
    pub fn switch_to_tab(&mut self, id: TabId) {
        if self.tabs.iter().any(|t| t.id == id) {
            self.active_tab_id = id;
            self.sync_chrome_with_tabs();
            log::debug!("Switched to tab {}", id.0);
        }
    }

    /// Switch to next tab (wraps around)
    pub fn next_tab(&mut self) {
        if let Some(current_index) = self.tab_index(self.active_tab_id) {
            let next_index = (current_index + 1) % self.tabs.len();
            let next_id = self.tabs[next_index].id;
            self.switch_to_tab(next_id);
        }
    }

    /// Switch to previous tab (wraps around)
    pub fn prev_tab(&mut self) {
        if let Some(current_index) = self.tab_index(self.active_tab_id) {
            let prev_index = if current_index == 0 {
                self.tabs.len() - 1
            } else {
                current_index - 1
            };
            let prev_id = self.tabs[prev_index].id;
            self.switch_to_tab(prev_id);
        }
    }

    // ==================== Navigation ====================

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

    /// Navigate via POST form submission
    pub fn navigate_post(&mut self, url: &Url, form_data: &str) -> Result<(), String> {
        log::info!("POST navigating to: {} with data: {}", url, form_data);

        // Update address bar
        self.chrome.address_bar.set_text(url.as_str());

        // POST the form data
        let response = self.fetch_url_post(url, form_data)?;

        if !response.is_success() {
            return Err(format!("HTTP error: {}", response.status));
        }

        let html = response.text_lossy();
        log::info!("Received {} bytes", html.len());

        // Load the page (use final URL from response in case of redirects)
        self.load_page(response.url, &html)?;

        Ok(())
    }

    /// Navigate to a URL asynchronously (non-blocking)
    ///
    /// This method starts the navigation and returns immediately.
    /// The event loop will poll for completion via poll_navigation().
    pub fn navigate_async(&mut self, url_str: &str) -> Result<(), String> {
        // Get active tab and cancel any in-progress navigation
        let active_id = self.active_tab_id;
        if let Some(tab) = self.tab_mut(active_id) {
            if let Some(cancel) = tab.nav_cancel.take() {
                cancel.cancel();
            }
            tab.nav_receiver = None;
        }

        // Parse URL
        let url = if url_str.contains("://") {
            Url::parse(url_str).map_err(|e| e.to_string())?
        } else {
            Url::parse(&format!("https://{}", url_str)).map_err(|e| e.to_string())?
        };

        log::info!("Starting async navigation to: {}", url);

        // Update UI immediately
        self.chrome.address_bar.set_text(url.as_str());
        self.chrome.is_loading = true;

        // Create channel and cancellation token
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let cancel_token = tokio_util::sync::CancellationToken::new();

        // Store in active tab
        if let Some(tab) = self.tab_mut(active_id) {
            tab.loading_state = LoadingState::Loading { url: url.clone() };
            tab.nav_receiver = Some(rx);
            tab.nav_cancel = Some(cancel_token.clone());
        }

        // Clone what we need for the async task
        let client = self.http_client.clone();
        let url_clone = url.clone();

        // Spawn async fetch task
        tokio::spawn(async move {
            let result = tokio::select! {
                _ = cancel_token.cancelled() => {
                    NavigationResult::Failed {
                        url: url_clone,
                        error: NavigationError::Cancelled,
                    }
                }
                fetch_result = client.get(&url_clone) => {
                    match fetch_result {
                        Ok(response) if response.is_success() => {
                            let html = response.text_lossy();
                            NavigationResult::Success {
                                url: response.url,
                                html,
                            }
                        }
                        Ok(response) => {
                            NavigationResult::Failed {
                                url: url_clone,
                                error: NavigationError::HttpError { status: response.status },
                            }
                        }
                        Err(e) => {
                            let error = if e.to_string().contains("timed out") {
                                NavigationError::Timeout
                            } else {
                                NavigationError::NetworkError(e.to_string())
                            };
                            NavigationResult::Failed {
                                url: url_clone,
                                error,
                            }
                        }
                    }
                }
            };

            let _ = tx.send(result).await;
        });

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

    /// Load HTML content from a local file path
    pub fn load_html_from_file(&mut self, path: &std::path::Path, html: &str, css: &str) -> Result<(), String> {
        // Convert path to absolute and create file:// URL
        let abs_path = path.canonicalize().map_err(|e| format!("Invalid path: {}", e))?;
        let url = Url::from_file_path(&abs_path)
            .map_err(|_| format!("Cannot create URL from path: {}", abs_path.display()))?;

        // Update address bar
        self.chrome.address_bar.set_text(url.as_str());

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

        // Extract and add CSS from <style> tags in the document
        {
            let dom_ref = shared_dom.borrow();
            let style_elements = dom_ref.get_elements_by_tag_name("style");
            for style_id in style_elements {
                // Get the text content of the style element
                if let Some(style_css) = extract_style_content(&dom_ref, style_id) {
                    if let Ok(stylesheet) = Stylesheet::parse(&style_css) {
                        cascade.add_author_stylesheet(stylesheet);
                    }
                }
            }
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

        // Load images (before layout so intrinsic dimensions are available)
        image_loader::load_images_in_tree(&mut layout_tree, &self.http_client, &url);

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

        // Store page state in active tab
        let active_id = self.active_tab_id;
        let paint_commands = display_list.commands.len();

        if let Some(tab) = self.tab_mut(active_id) {
            // Update navigation
            tab.navigation.navigate_to(url.clone());

            // Store page state
            tab.page = Some(PageState {
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
        }

        // Update chrome UI
        self.sync_chrome_with_tabs();

        log::info!("Page loaded with {} paint commands", paint_commands);

        Ok(())
    }

    /// Go back in history
    pub fn go_back(&mut self) -> Result<(), String> {
        let active_id = self.active_tab_id;
        let url = if let Some(tab) = self.tab_mut(active_id) {
            tab.navigation.go_back().cloned()
        } else {
            None
        };

        if let Some(url) = url {
            self.chrome.address_bar.set_text(url.as_str());
            self.reload_url(url)?;
        }
        Ok(())
    }

    /// Go forward in history
    pub fn go_forward(&mut self) -> Result<(), String> {
        let active_id = self.active_tab_id;
        let url = if let Some(tab) = self.tab_mut(active_id) {
            tab.navigation.go_forward().cloned()
        } else {
            None
        };

        if let Some(url) = url {
            self.chrome.address_bar.set_text(url.as_str());
            self.reload_url(url)?;
        }
        Ok(())
    }

    /// Reload the current page
    pub fn reload_page(&mut self) {
        // Get the current URL from active tab's navigation history or address bar
        let url = self
            .active_tab()
            .and_then(|tab| tab.navigation.current_url())
            .map(|u| u.as_str().to_string())
            .or_else(|| {
                let text = &self.chrome.address_bar.text;
                if !text.is_empty() {
                    Some(text.clone())
                } else {
                    None
                }
            });

        if let Some(url) = url {
            log::info!("Reloading page: {}", url);
            if let Err(e) = self.navigate_async(&url) {
                log::error!("Reload failed: {}", e);
            }
        }
    }

    /// Stop any in-progress navigation
    pub fn stop_loading(&mut self) {
        let active_id = self.active_tab_id;
        if let Some(tab) = self.tab_mut(active_id) {
            if let Some(cancel) = tab.nav_cancel.take() {
                log::info!("Cancelling navigation");
                cancel.cancel();
            }
            tab.loading_state = LoadingState::Idle;
            tab.nav_receiver = None;
        }
        self.chrome.is_loading = false;
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

    /// POST form data to a URL
    fn fetch_url_post(&self, url: &Url, form_data: &str) -> Result<gugalanna_net::Response, String> {
        use tokio::runtime::Handle;

        // Check if we're already in a tokio runtime
        if let Ok(handle) = Handle::try_current() {
            // We're in an async context - use block_in_place
            tokio::task::block_in_place(|| {
                handle.block_on(self.http_client.post_form(url, form_data))
            })
            .map_err(|e| e.to_string())
        } else {
            // No runtime - create one
            let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
            rt.block_on(self.http_client.post_form(url, form_data))
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

        // Extract and add CSS from <style> tags in the document
        {
            let dom_ref = shared_dom.borrow();
            let style_elements = dom_ref.get_elements_by_tag_name("style");
            for style_id in style_elements {
                if let Some(style_css) = extract_style_content(&dom_ref, style_id) {
                    if let Ok(stylesheet) = Stylesheet::parse(&style_css) {
                        cascade.add_author_stylesheet(stylesheet);
                    }
                }
            }
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

        // Store page state in active tab (without updating navigation history)
        let active_id = self.active_tab_id;
        if let Some(tab) = self.tab_mut(active_id) {
            tab.page = Some(PageState {
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
        }

        // Update chrome UI
        self.sync_chrome_with_tabs();

        Ok(())
    }

    /// Run the browser event loop
    pub fn run(&mut self) -> Result<(), String> {
        self.last_frame = Instant::now();

        'running: loop {
            // Calculate delta time for animations
            let now = Instant::now();
            let delta_ms = now.duration_since(self.last_frame).as_secs_f32() * 1000.0;
            self.last_frame = now;

            // Poll for navigation completion
            self.poll_navigation();

            // Poll events
            let events = poll_events();

            for event in events {
                match event {
                    BrowserEvent::Quit => {
                        break 'running;
                    }

                    BrowserEvent::KeyDown { scancode, modifiers } => {
                        if self.handle_key(scancode, modifiers) {
                            break 'running;
                        }
                    }

                    BrowserEvent::TextInput { text } => {
                        self.handle_text_input(&text);
                    }

                    BrowserEvent::MouseDown { x, y, button } => {
                        if button == MouseButton::Left {
                            if self.handle_click(x, y) {
                                break 'running;
                            }
                        }
                    }

                    BrowserEvent::MouseWheel { y, .. } => {
                        // Scroll page (y > 0 = scroll up, y < 0 = scroll down)
                        let delta = y as f32 * SCROLL_WHEEL_MULTIPLIER;
                        self.handle_scroll(delta);
                    }

                    BrowserEvent::MouseMove { x, y } => {
                        log::trace!("MouseMove: x={}, y={}", x, y);
                        self.handle_mouse_move(x, y);
                    }

                    BrowserEvent::WindowResize { width, height } => {
                        self.config.width = width;
                        self.config.height = height;
                        self.chrome.update_width(width as f32);
                        self.relayout_page();
                    }
                }
            }

            // Tick CSS transitions
            let _transitions_active = self.transition_manager.tick(delta_ms);

            // Update loading animation
            self.chrome.tick_loading();

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
    fn handle_key(&mut self, scancode: u32, modifiers: Modifiers) -> bool {
        use crate::event::{
            SCANCODE_BACKSPACE, SCANCODE_DOWN, SCANCODE_END, SCANCODE_ESCAPE, SCANCODE_F5,
            SCANCODE_HOME, SCANCODE_L, SCANCODE_LEFT, SCANCODE_PAGEDOWN, SCANCODE_PAGEUP,
            SCANCODE_Q, SCANCODE_R, SCANCODE_RETURN, SCANCODE_RIGHT, SCANCODE_T, SCANCODE_TAB,
            SCANCODE_UP, SCANCODE_W,
        };

        // Handle keyboard shortcuts with modifiers first
        match (scancode, modifiers.ctrl, modifiers.alt, modifiers.shift) {
            // Ctrl+Q: Quit browser
            (SCANCODE_Q, true, false, _) => {
                log::info!("Ctrl+Q pressed - quitting browser");
                return true;
            }

            // Ctrl+T: New tab
            (SCANCODE_T, true, false, false) => {
                self.new_tab();
                self.focus_address_bar();
                return false;
            }

            // Ctrl+W: Close current tab
            (SCANCODE_W, true, false, false) => {
                let active_id = self.active_tab_id;
                if self.close_tab(active_id) {
                    // Last tab closed - quit
                    return true;
                }
                return false;
            }

            // Ctrl+Tab: Next tab
            (SCANCODE_TAB, true, false, false) => {
                self.next_tab();
                return false;
            }

            // Ctrl+Shift+Tab: Previous tab
            (SCANCODE_TAB, true, false, true) => {
                self.prev_tab();
                return false;
            }

            // Ctrl+L: Focus address bar
            (SCANCODE_L, true, false, _) => {
                self.focus_address_bar();
                // Select all text in address bar
                self.chrome.address_bar.move_cursor_to_end();
                return false;
            }

            // Ctrl+R: Reload page
            (SCANCODE_R, true, false, _) => {
                self.reload_page();
                return false;
            }

            // Alt+Left: Go back
            (SCANCODE_LEFT, false, true, _) => {
                if self.chrome.back_button.enabled {
                    if let Err(e) = self.go_back() {
                        log::error!("Go back failed: {}", e);
                    }
                }
                return false;
            }

            // Alt+Right: Go forward
            (SCANCODE_RIGHT, false, true, _) => {
                if self.chrome.forward_button.enabled {
                    if let Err(e) = self.go_forward() {
                        log::error!("Go forward failed: {}", e);
                    }
                }
                return false;
            }

            _ => {}
        }

        // Handle non-modifier keys
        match scancode {
            // F5: Reload page
            SCANCODE_F5 => {
                self.reload_page();
            }

            // Escape: Stop loading or blur address bar (no longer quits)
            SCANCODE_ESCAPE => {
                if self.chrome.is_loading {
                    self.stop_loading();
                } else if self.focus == FocusTarget::AddressBar {
                    self.blur_address_bar();
                }
                // No longer quits - use Ctrl+Q to quit
            }

            SCANCODE_BACKSPACE if self.focus == FocusTarget::AddressBar => {
                self.chrome.address_bar.delete_char();
            }

            SCANCODE_RETURN if self.focus == FocusTarget::AddressBar => {
                // Navigate to URL in address bar
                let url = self.chrome.address_bar.text.clone();
                if !url.is_empty() {
                    if let Err(e) = self.navigate_async(&url) {
                        log::error!("Navigation failed: {}", e);
                    }
                }
                self.blur_address_bar();
            }

            // Form input keyboard handling
            SCANCODE_BACKSPACE if matches!(self.focus, FocusTarget::FormInput(_)) => {
                if let FocusTarget::FormInput(node_id) = self.focus {
                    if let Some(tab) = self.tab_mut(self.active_tab_id) {
                        if let Some(state) = tab.form_state.get_text_mut(node_id) {
                            state.delete_char_before();
                        }
                    }
                }
            }

            SCANCODE_LEFT if matches!(self.focus, FocusTarget::FormInput(_)) => {
                if let FocusTarget::FormInput(node_id) = self.focus {
                    if let Some(tab) = self.tab_mut(self.active_tab_id) {
                        if let Some(state) = tab.form_state.get_text_mut(node_id) {
                            state.move_cursor_left();
                        }
                    }
                }
            }

            SCANCODE_RIGHT if matches!(self.focus, FocusTarget::FormInput(_)) => {
                if let FocusTarget::FormInput(node_id) = self.focus {
                    if let Some(tab) = self.tab_mut(self.active_tab_id) {
                        if let Some(state) = tab.form_state.get_text_mut(node_id) {
                            state.move_cursor_right();
                        }
                    }
                }
            }

            SCANCODE_RETURN if matches!(self.focus, FocusTarget::FormInput(_)) => {
                // TODO: Submit form or move to next input
                self.blur_form_input();
            }

            // Scroll keys (only when not editing address bar or form input)
            SCANCODE_UP if self.focus != FocusTarget::AddressBar => {
                self.handle_scroll(SCROLL_LINE_HEIGHT);
            }

            SCANCODE_DOWN if self.focus != FocusTarget::AddressBar => {
                self.handle_scroll(-SCROLL_LINE_HEIGHT);
            }

            SCANCODE_PAGEUP if self.focus != FocusTarget::AddressBar => {
                let viewport_height = self
                    .active_tab()
                    .and_then(|t| t.page.as_ref())
                    .map(|p| p.viewport_height)
                    .unwrap_or(600.0);
                let delta = viewport_height * SCROLL_PAGE_FACTOR;
                self.handle_scroll(delta);
            }

            SCANCODE_PAGEDOWN if self.focus != FocusTarget::AddressBar => {
                let viewport_height = self
                    .active_tab()
                    .and_then(|t| t.page.as_ref())
                    .map(|p| p.viewport_height)
                    .unwrap_or(600.0);
                let delta = viewport_height * SCROLL_PAGE_FACTOR;
                self.handle_scroll(-delta);
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
        match self.focus {
            FocusTarget::AddressBar => {
                for c in text.chars() {
                    self.chrome.address_bar.insert_char(c);
                }
            }
            FocusTarget::FormInput(node_id) => {
                // Insert text into the form input
                if let Some(tab) = self.tab_mut(self.active_tab_id) {
                    if let Some(state) = tab.form_state.get_text_mut(node_id) {
                        state.insert_text(text);
                    }
                }
            }
            _ => {}
        }
    }

    /// Handle scroll by delta (positive = scroll up/show content above, negative = scroll down)
    fn handle_scroll(&mut self, delta: f32) {
        let active_id = self.active_tab_id;
        if let Some(tab) = self.tab_mut(active_id) {
            if let Some(ref mut page) = tab.page {
                let max_scroll = (page.content_height - page.viewport_height).max(0.0);
                page.scroll_y = (page.scroll_y - delta).clamp(0.0, max_scroll);
            }
        }
    }

    /// Scroll to the top of the page
    fn scroll_to_top(&mut self) {
        let active_id = self.active_tab_id;
        if let Some(tab) = self.tab_mut(active_id) {
            if let Some(ref mut page) = tab.page {
                page.scroll_y = 0.0;
            }
        }
    }

    /// Scroll to the bottom of the page
    fn scroll_to_bottom(&mut self) {
        let active_id = self.active_tab_id;
        if let Some(tab) = self.tab_mut(active_id) {
            if let Some(ref mut page) = tab.page {
                let max_scroll = (page.content_height - page.viewport_height).max(0.0);
                page.scroll_y = max_scroll;
            }
        }
    }

    /// Scroll to an element with the given ID (fragment)
    fn scroll_to_fragment(&mut self, fragment: &str) {
        if fragment.is_empty() {
            return;
        }

        let active_id = self.active_tab_id;
        if let Some(tab) = self.tab_mut(active_id) {
            if let Some(ref mut page) = tab.page {
                let dom_ref = page.dom.borrow();

                // Find element by ID
                if let Some(element_id) = dom_ref.get_element_by_id(fragment) {
                    // Find hit region for this element to get Y position
                    for region in &page.hit_regions {
                        if region.node_id == element_id.0 {
                            // Scroll to put element at top of viewport
                            let max_scroll = (page.content_height - page.viewport_height).max(0.0);
                            page.scroll_y = region.y.clamp(0.0, max_scroll);
                            log::debug!("Scrolling to fragment '{}' at y={}", fragment, region.y);
                            break;
                        }
                    }
                } else {
                    log::debug!("Fragment '{}' not found in document", fragment);
                }
            }
        }
    }

    /// Poll for navigation completion (called each frame)
    fn poll_navigation(&mut self) {
        // Poll all tabs for navigation results
        // We need to collect results first to avoid borrow issues
        let mut results: Vec<(TabId, NavigationResult)> = Vec::new();

        for tab in &mut self.tabs {
            if let Some(ref mut rx) = tab.nav_receiver {
                if let Ok(result) = rx.try_recv() {
                    results.push((tab.id, result));
                }
            }
        }

        // Process results
        for (tab_id, result) in results {
            // Clear loading state for this tab
            if let Some(tab) = self.tab_mut(tab_id) {
                tab.nav_receiver = None;
                tab.nav_cancel = None;
                tab.loading_state = LoadingState::Idle;
            }

            // Update chrome if this is the active tab
            if tab_id == self.active_tab_id {
                self.chrome.is_loading = false;
            }

            match result {
                NavigationResult::Success { url, html } => {
                    log::info!("Navigation complete for tab {}: {}", tab_id.0, url);

                    // Load the page into the specific tab
                    if tab_id == self.active_tab_id {
                        // Active tab - use normal load
                        if let Err(e) = self.load_page(url, &html) {
                            log::error!("Failed to load page: {}", e);
                        }
                    } else {
                        // Background tab - load directly into tab
                        if let Err(e) = self.load_page_into_tab(tab_id, url, &html) {
                            log::error!("Failed to load page into tab {}: {}", tab_id.0, e);
                        }
                    }
                }
                NavigationResult::Failed { url, error } => {
                    log::error!("Navigation failed for tab {} to {}: {:?}", tab_id.0, url, error);

                    if let Some(tab) = self.tab_mut(tab_id) {
                        tab.loading_state = LoadingState::Failed {
                            url: url.clone(),
                            error: error.clone(),
                        };
                    }

                    // Show error page if this is the active tab
                    if tab_id == self.active_tab_id {
                        self.show_error_page(&url, &error);
                    }
                }
            }
        }
    }

    /// Load a page into a specific tab (for background tab loading)
    fn load_page_into_tab(&mut self, tab_id: TabId, url: Url, html: &str) -> Result<(), String> {
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
        let css = r#"
            body { background-color: white; color: black; font-size: 16px; }
            h1, h2, h3, h4, h5, h6, p, div { display: block; }
            h1 { font-size: 32px; margin-top: 20px; margin-bottom: 10px; }
            h2 { font-size: 24px; margin-top: 18px; margin-bottom: 8px; }
            h3 { font-size: 18px; margin-top: 16px; margin-bottom: 6px; }
            p { margin-top: 10px; margin-bottom: 10px; }
        "#;
        if let Ok(stylesheet) = Stylesheet::parse(css) {
            cascade.add_author_stylesheet(stylesheet);
        }

        // Extract and add CSS from <style> tags
        {
            let dom_ref = shared_dom.borrow();
            let style_elements = dom_ref.get_elements_by_tag_name("style");
            for style_id in style_elements {
                if let Some(style_css) = extract_style_content(&dom_ref, style_id) {
                    if let Ok(stylesheet) = Stylesheet::parse(&style_css) {
                        cascade.add_author_stylesheet(stylesheet);
                    }
                }
            }
        }

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

        let content_height = layout_tree.dimensions.margin_box_height();
        let display_list = build_display_list(&layout_tree);
        let hit_regions = build_hit_regions(&layout_tree);
        drop(dom_ref);

        // Store in the specific tab
        if let Some(tab) = self.tab_mut(tab_id) {
            tab.navigation.navigate_to(url.clone());
            tab.page = Some(PageState {
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
        }

        Ok(())
    }

    /// Display an error page for navigation failures
    fn show_error_page(&mut self, url: &Url, error: &NavigationError) {
        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>{title}</title>
    <style>
        body {{
            font-family: sans-serif;
            background-color: #f5f5f5;
            color: #333;
            padding: 40px;
            text-align: center;
        }}
        h1 {{
            color: #d93025;
            font-size: 28px;
            margin-bottom: 10px;
        }}
        .url {{
            color: #666;
            word-break: break-all;
            margin: 20px 0;
        }}
        .details {{
            color: #888;
            font-size: 14px;
        }}
    </style>
</head>
<body>
    <h1>{title}</h1>
    <p class="url">{url}</p>
    <p class="details">{details}</p>
</body>
</html>"#,
            title = error.title(),
            url = url.as_str(),
            details = error.details(),
        );

        // Load as error page (don't add to history)
        let error_url = Url::parse("about:error").unwrap();
        let _ = self.load_page_without_history(error_url, &html);
    }

    /// Re-layout the page with new viewport dimensions
    fn relayout_page(&mut self) {
        let active_id = self.active_tab_id;
        let viewport_width = self.config.width as f32;
        let viewport_height = self.config.height as f32 - CHROME_HEIGHT;

        if let Some(tab) = self.tab_mut(active_id) {
            if let Some(ref mut page) = tab.page {
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
    }

    /// Handle a mouse click
    /// Returns true if the browser should quit (last tab closed)
    fn handle_click(&mut self, x: f32, y: f32) -> bool {
        // Check chrome first
        if let Some(hit) = self.chrome.hit_test(x, y) {
            match hit {
                ChromeHit::Tab(id) => {
                    self.switch_to_tab(id);
                }
                ChromeHit::TabClose(id) => {
                    if self.close_tab(id) {
                        // Last tab closed - quit
                        return true;
                    }
                }
                ChromeHit::NewTab => {
                    self.new_tab();
                    self.focus_address_bar();
                }
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
                        if let Err(e) = self.navigate_async(&url) {
                            log::error!("Navigation failed: {}", e);
                        }
                    }
                }
                ChromeHit::AddressBar => {
                    self.focus_address_bar();
                }
            }
            return false;
        }

        // Blur address bar if clicking outside
        if self.focus == FocusTarget::AddressBar {
            self.blur_address_bar();
        }

        // Blur form input if clicking outside chrome
        if let FocusTarget::FormInput(_) = self.focus {
            self.blur_form_input();
        }

        // Check page content
        let page_y = y - CHROME_HEIGHT;
        log::debug!("Click at x={}, y={}, page_y={}", x, y, page_y);
        if page_y >= 0.0 {
            let active_id = self.active_tab_id;

            // First check for form elements without mutable borrow
            let form_info = if let Some(tab) = self.tabs.iter().find(|t| t.id == active_id) {
                if let Some(ref page) = tab.page {
                    let content_y = page_y + page.scroll_y;

                    if let Some(node_id) = hit_test_regions(&page.hit_regions, x, content_y) {
                        let dom_ref = page.dom.borrow();
                        find_form_element(&dom_ref, NodeId(node_id))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            // Handle form element click
            if let Some(ref form_elem) = form_info {
                log::info!("Form element clicked: {:?}", form_elem);
                match form_elem {
                    FormElementInfo::TextInput { node_id, .. } => {
                        self.focus_form_input(*node_id);
                        return false;
                    }
                    FormElementInfo::Checkbox { node_id } => {
                        self.toggle_checkbox(*node_id);
                        return false;
                    }
                    FormElementInfo::Radio { node_id, name } => {
                        self.select_radio(*node_id, name);
                        return false;
                    }
                    FormElementInfo::Submit { node_id } => {
                        log::info!("Submit button clicked (node {})", node_id.0);
                        self.submit_form(*node_id);
                        return false;
                    }
                    FormElementInfo::Button { node_id } => {
                        log::info!("Button clicked (node {})", node_id.0);
                        // Dispatch to JS if available
                        return false;
                    }
                }
            }

            // Check for link
            let link_info = if let Some(tab) = self.tabs.iter().find(|t| t.id == active_id) {
                if let Some(ref page) = tab.page {
                    let content_y = page_y + page.scroll_y;
                    log::debug!("Content y={}, hit_regions count={}", content_y, page.hit_regions.len());

                    if let Some(node_id) = hit_test_regions(&page.hit_regions, x, content_y) {
                        log::debug!("Page click on node {}", node_id);
                        let dom_ref = page.dom.borrow();
                        find_anchor_href(&dom_ref, gugalanna_dom::NodeId(node_id))
                            .map(|(href, _)| (href, page.url.clone(), node_id))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            if let Some((href, base_url, _node_id)) = link_info {
                log::info!("Link clicked: {}", href);

                // Handle fragment-only links (same page scroll)
                if href.starts_with('#') {
                    self.scroll_to_fragment(&href[1..]);
                    return false;
                }

                // Resolve the URL and navigate
                match resolve_link_url(&base_url, &href) {
                    Ok(target_url) => {
                        if let Err(e) = self.navigate_async(target_url.as_str()) {
                            log::error!("Link navigation failed: {}", e);
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to resolve URL '{}': {}", href, e);
                    }
                }
                return false;
            }

            // Not a link - dispatch click to JS
            if let Some(tab) = self.tab_mut(active_id) {
                if let Some(ref mut page) = tab.page {
                    let content_y = page_y + page.scroll_y;
                    if let Some(node_id) = hit_test_regions(&page.hit_regions, x, content_y) {
                        if let Some(ref rt) = page.js_runtime {
                            if let Err(e) = rt.dispatch_click(node_id) {
                                log::warn!("Click dispatch failed: {}", e);
                            }
                        }
                    }
                }
            }
        }
        false
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

    /// Focus a form text input
    fn focus_form_input(&mut self, node_id: NodeId) {
        self.focus = FocusTarget::FormInput(node_id);
        start_text_input();

        // Ensure the input has state
        if let Some(tab) = self.tab_mut(self.active_tab_id) {
            tab.form_state.ensure_text(node_id);
        }
    }

    /// Blur a form text input
    fn blur_form_input(&mut self) {
        self.focus = FocusTarget::None;
        stop_text_input();
    }

    /// Toggle a checkbox
    fn toggle_checkbox(&mut self, node_id: NodeId) {
        if let Some(tab) = self.tab_mut(self.active_tab_id) {
            tab.form_state.toggle_checked(node_id);
        }
    }

    /// Select a radio button (and deselect others in the same group)
    fn select_radio(&mut self, node_id: NodeId, group_name: &str) {
        let active_id = self.active_tab_id;

        // First, find all radio buttons in the same group and deselect them
        let radios_to_deselect: Vec<NodeId> = if let Some(tab) = self.tabs.iter().find(|t| t.id == active_id) {
            if let Some(ref page) = tab.page {
                let dom = page.dom.borrow();
                find_radio_buttons_in_group(&dom, group_name)
                    .into_iter()
                    .filter(|id| *id != node_id)
                    .collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        // Now update the form state
        if let Some(tab) = self.tab_mut(active_id) {
            for id in radios_to_deselect {
                tab.form_state.set_checked(id, false);
            }
            tab.form_state.set_checked(node_id, true);
        }
    }

    /// Submit a form
    fn submit_form(&mut self, submit_button_id: NodeId) {
        let active_id = self.active_tab_id;

        // Collect form data
        let submit_data = if let Some(tab) = self.tabs.iter().find(|t| t.id == active_id) {
            if let Some(ref page) = tab.page {
                let dom = page.dom.borrow();

                // Find the parent form
                let form_id = match find_parent_form(&dom, submit_button_id) {
                    Some(id) => id,
                    None => {
                        log::warn!("Submit button has no parent form");
                        return;
                    }
                };

                // Get form attributes
                let (action, method) = if let Some(node) = dom.get(form_id) {
                    if let Some(elem) = node.as_element() {
                        (
                            elem.get_attribute("action").unwrap_or("").to_string(),
                            elem.get_attribute("method").unwrap_or("get").to_lowercase(),
                        )
                    } else {
                        ("".to_string(), "get".to_string())
                    }
                } else {
                    ("".to_string(), "get".to_string())
                };

                // Collect form data
                let fields = collect_form_data(&dom, form_id, &tab.form_state);
                let query_string = build_form_data_string(&fields);

                // Get base URL for resolving action
                let base_url = page.url.clone();

                Some((action, method, query_string, base_url))
            } else {
                None
            }
        } else {
            None
        };

        // Now perform navigation (outside the borrow)
        if let Some((action, method, query_string, base_url)) = submit_data {
            log::info!("Form submit: action={}, method={}, data={}", action, method, query_string);

            if method == "get" {
                // Build URL with query string
                let target_url = if action.is_empty() {
                    // Submit to current URL
                    let mut url = base_url.clone();
                    url.set_query(if query_string.is_empty() { None } else { Some(&query_string) });
                    url.to_string()
                } else {
                    // Resolve action URL
                    match resolve_link_url(&base_url, &action) {
                        Ok(mut url) => {
                            url.set_query(if query_string.is_empty() { None } else { Some(&query_string) });
                            url.to_string()
                        }
                        Err(e) => {
                            log::error!("Failed to resolve form action URL: {}", e);
                            return;
                        }
                    }
                };

                // Navigate to the form submission URL
                if let Err(e) = self.navigate(&target_url) {
                    log::error!("Form submission failed: {}", e);
                }
            } else {
                // POST submission
                let target_url = if action.is_empty() {
                    base_url.clone()
                } else {
                    match resolve_link_url(&base_url, &action) {
                        Ok(url) => url,
                        Err(e) => {
                            log::error!("Failed to resolve form action URL: {}", e);
                            return;
                        }
                    }
                };

                if let Err(e) = self.navigate_post(&target_url, &query_string) {
                    log::error!("Form POST submission failed: {}", e);
                }
            }
        }
    }

    /// Handle mouse movement (for cursor changes on link hover)
    fn handle_mouse_move(&mut self, x: f32, y: f32) {
        let is_over_link = self.is_over_link(x, y);

        let desired_cursor = if is_over_link {
            CursorType::Hand
        } else {
            CursorType::Arrow
        };

        if desired_cursor != self.current_cursor {
            self.current_cursor = desired_cursor;
            self.backend.set_cursor(desired_cursor);
        }
    }

    /// Check if mouse position is over a link
    fn is_over_link(&self, x: f32, y: f32) -> bool {
        // Skip if in chrome area
        if y < CHROME_HEIGHT {
            return false;
        }

        if let Some(tab) = self.active_tab() {
            if let Some(ref page) = tab.page {
                let content_y = (y - CHROME_HEIGHT) + page.scroll_y;

                if let Some(node_id) = hit_test_regions(&page.hit_regions, x, content_y) {
                    let dom_ref = page.dom.borrow();
                    let result = find_anchor_href(&dom_ref, gugalanna_dom::NodeId(node_id));
                    if result.is_some() {
                        log::debug!("Over link! node_id={}, href={:?}", node_id, result.as_ref().map(|(h, _)| h));
                    }
                    return result.is_some();
                }
            }
        }
        false
    }

    /// Render the browser
    fn render(&mut self) {
        // Clear with white
        self.backend.clear(RenderColor::white());

        // Render chrome
        let chrome_display_list = self.chrome.build_display_list();
        self.backend.render(&chrome_display_list);

        // Render page content (offset by chrome height and scroll)
        // Clone the display list, scroll_y, and form state to avoid borrow issues
        let page_data = self
            .active_tab()
            .map(|t| {
                let display_list = t.page.as_ref().map(|p| p.display_list.clone());
                let scroll_y = t.page.as_ref().map(|p| p.scroll_y).unwrap_or(0.0);
                let form_state = t.form_state.clone();
                (display_list, scroll_y, form_state)
            });

        // Get the focused form node_id if any
        let focused_form_node = match self.focus {
            FocusTarget::FormInput(node_id) => Some(node_id),
            _ => None,
        };

        if let Some((Some(display_list), scroll_y, form_state)) = page_data {
            self.render_page(&display_list, scroll_y, &form_state, focused_form_node);
        }

        // Present
        self.backend.present();
    }

    /// Render page content with Y offset (chrome height) and scroll offset
    fn render_page(
        &mut self,
        display_list: &DisplayList,
        scroll_y: f32,
        form_state: &crate::form::FormState,
        focused_form_node: Option<NodeId>,
    ) {
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
                    let mut new_y = rect.y + y_offset;
                    let mut height = rect.height;

                    // Skip if completely off-screen
                    if new_y + height < CHROME_HEIGHT || new_y > viewport_bottom {
                        continue;
                    }

                    // Clip to chrome area (don't render above chrome)
                    if new_y < CHROME_HEIGHT {
                        let clip_amount = CHROME_HEIGHT - new_y;
                        new_y = CHROME_HEIGHT;
                        height -= clip_amount;
                        if height <= 0.0 {
                            continue;
                        }
                    }

                    offset_commands.push(PaintCommand::FillRect {
                        rect: Rect {
                            x: rect.x,
                            y: new_y,
                            width: rect.width,
                            height,
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
                    // Skip if text is off-screen or in chrome area
                    if new_y + *font_size < CHROME_HEIGHT || new_y > viewport_bottom || new_y < CHROME_HEIGHT {
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
                    // Skip if off-screen or in chrome area
                    if new_y + rect.height < CHROME_HEIGHT || new_y > viewport_bottom || new_y < CHROME_HEIGHT {
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
                PaintCommand::DrawTextInput {
                    node_id,
                    rect,
                    text: _,
                    cursor_pos: _,
                    is_password,
                    is_focused: _,
                } => {
                    let new_y = rect.y + y_offset;
                    // Skip if off-screen or in chrome area
                    if new_y + rect.height < CHROME_HEIGHT || new_y > viewport_bottom || new_y < CHROME_HEIGHT {
                        continue;
                    }
                    // Get actual text value and cursor position from form state
                    let is_focused = focused_form_node == Some(*node_id);
                    let (text, cursor_pos) = if let Some(state) = form_state.get_text(*node_id) {
                        (state.value.clone(), if is_focused { Some(state.cursor_pos) } else { None })
                    } else {
                        (String::new(), None)
                    };
                    offset_commands.push(PaintCommand::DrawTextInput {
                        node_id: *node_id,
                        rect: Rect {
                            x: rect.x,
                            y: new_y,
                            width: rect.width,
                            height: rect.height,
                        },
                        text,
                        cursor_pos,
                        is_password: *is_password,
                        is_focused,
                    });
                }
                PaintCommand::DrawCheckbox {
                    node_id,
                    rect,
                    checked: _,
                    is_focused: _,
                } => {
                    let new_y = rect.y + y_offset;
                    // Skip if off-screen or in chrome area
                    if new_y + rect.height < CHROME_HEIGHT || new_y > viewport_bottom || new_y < CHROME_HEIGHT {
                        continue;
                    }
                    // Get actual checked state from form state
                    let checked = form_state.is_checked(*node_id);
                    let is_focused = focused_form_node == Some(*node_id);
                    offset_commands.push(PaintCommand::DrawCheckbox {
                        node_id: *node_id,
                        rect: Rect {
                            x: rect.x,
                            y: new_y,
                            width: rect.width,
                            height: rect.height,
                        },
                        checked,
                        is_focused,
                    });
                }
                PaintCommand::DrawRadio {
                    node_id,
                    rect,
                    checked: _,
                    is_focused: _,
                } => {
                    let new_y = rect.y + y_offset;
                    // Skip if off-screen or in chrome area
                    if new_y + rect.height < CHROME_HEIGHT || new_y > viewport_bottom || new_y < CHROME_HEIGHT {
                        continue;
                    }
                    // Get actual checked state from form state
                    let checked = form_state.is_checked(*node_id);
                    let is_focused = focused_form_node == Some(*node_id);
                    offset_commands.push(PaintCommand::DrawRadio {
                        node_id: *node_id,
                        rect: Rect {
                            x: rect.x,
                            y: new_y,
                            width: rect.width,
                            height: rect.height,
                        },
                        checked,
                        is_focused,
                    });
                }
                PaintCommand::DrawButton {
                    node_id,
                    rect,
                    text,
                    is_pressed,
                } => {
                    let new_y = rect.y + y_offset;
                    // Skip if off-screen or in chrome area
                    if new_y + rect.height < CHROME_HEIGHT || new_y > viewport_bottom || new_y < CHROME_HEIGHT {
                        continue;
                    }
                    offset_commands.push(PaintCommand::DrawButton {
                        node_id: *node_id,
                        rect: Rect {
                            x: rect.x,
                            y: new_y,
                            width: rect.width,
                            height: rect.height,
                        },
                        text: text.clone(),
                        is_pressed: *is_pressed,
                    });
                }
                PaintCommand::DrawImage { rect, pixels, alt } => {
                    let new_y = rect.y + y_offset;
                    // Skip if off-screen or in chrome area
                    if new_y + rect.height < CHROME_HEIGHT || new_y > viewport_bottom {
                        continue;
                    }

                    // Clip to chrome area
                    let mut clipped_y = new_y;
                    let mut clipped_height = rect.height;
                    if clipped_y < CHROME_HEIGHT {
                        let clip_amount = CHROME_HEIGHT - clipped_y;
                        clipped_y = CHROME_HEIGHT;
                        clipped_height -= clip_amount;
                        if clipped_height <= 0.0 {
                            continue;
                        }
                    }

                    offset_commands.push(PaintCommand::DrawImage {
                        rect: Rect {
                            x: rect.x,
                            y: clipped_y,
                            width: rect.width,
                            height: clipped_height,
                        },
                        pixels: pixels.clone(),
                        alt: alt.clone(),
                    });
                }
                PaintCommand::SetClipRect(rect) => {
                    // Offset the clip rect for scroll position
                    let new_y = rect.y + y_offset;
                    offset_commands.push(PaintCommand::SetClipRect(Rect {
                        x: rect.x,
                        y: new_y,
                        width: rect.width,
                        height: rect.height,
                    }));
                }
                PaintCommand::ClearClipRect => {
                    offset_commands.push(PaintCommand::ClearClipRect);
                }
                PaintCommand::PushOpacity(opacity) => {
                    offset_commands.push(PaintCommand::PushOpacity(*opacity));
                }
                PaintCommand::PopOpacity => {
                    offset_commands.push(PaintCommand::PopOpacity);
                }
                PaintCommand::DrawBoxShadow { rect, shadow } => {
                    let new_y = rect.y + y_offset;
                    // Skip if off-screen
                    if new_y + rect.height + shadow.blur_radius + shadow.spread_radius < CHROME_HEIGHT
                        || new_y - shadow.blur_radius - shadow.spread_radius > viewport_bottom
                    {
                        continue;
                    }
                    offset_commands.push(PaintCommand::DrawBoxShadow {
                        rect: Rect {
                            x: rect.x,
                            y: new_y,
                            width: rect.width,
                            height: rect.height,
                        },
                        shadow: shadow.clone(),
                    });
                }
                PaintCommand::FillRoundedRect { rect, radius, color } => {
                    let new_y = rect.y + y_offset;
                    // Skip if off-screen or in chrome area
                    if new_y + rect.height < CHROME_HEIGHT || new_y > viewport_bottom {
                        continue;
                    }
                    offset_commands.push(PaintCommand::FillRoundedRect {
                        rect: Rect {
                            x: rect.x,
                            y: new_y,
                            width: rect.width,
                            height: rect.height,
                        },
                        radius: *radius,
                        color: *color,
                    });
                }
                PaintCommand::DrawRoundedBorder { rect, radius, widths, color } => {
                    let new_y = rect.y + y_offset;
                    // Skip if off-screen or in chrome area
                    if new_y + rect.height < CHROME_HEIGHT || new_y > viewport_bottom {
                        continue;
                    }
                    offset_commands.push(PaintCommand::DrawRoundedBorder {
                        rect: Rect {
                            x: rect.x,
                            y: new_y,
                            width: rect.width,
                            height: rect.height,
                        },
                        radius: *radius,
                        widths: *widths,
                        color: *color,
                    });
                }
                PaintCommand::FillLinearGradient { rect, direction, stops, radius } => {
                    let new_y = rect.y + y_offset;
                    // Skip if off-screen or in chrome area
                    if new_y + rect.height < CHROME_HEIGHT || new_y > viewport_bottom {
                        continue;
                    }
                    offset_commands.push(PaintCommand::FillLinearGradient {
                        rect: Rect {
                            x: rect.x,
                            y: new_y,
                            width: rect.width,
                            height: rect.height,
                        },
                        direction: direction.clone(),
                        stops: stops.clone(),
                        radius: *radius,
                    });
                }
                PaintCommand::FillRadialGradient { rect, shape, size, center_x, center_y, stops, radius } => {
                    let new_y = rect.y + y_offset;
                    // Skip if off-screen or in chrome area
                    if new_y + rect.height < CHROME_HEIGHT || new_y > viewport_bottom {
                        continue;
                    }
                    offset_commands.push(PaintCommand::FillRadialGradient {
                        rect: Rect {
                            x: rect.x,
                            y: new_y,
                            width: rect.width,
                            height: rect.height,
                        },
                        shape: *shape,
                        size: *size,
                        center_x: *center_x,
                        center_y: *center_y,
                        stops: stops.clone(),
                        radius: *radius,
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
    build_hit_regions_recursive(layout, &mut regions, 0.0, 0.0);
    regions
}

fn build_hit_regions_recursive(layout: &LayoutBox, regions: &mut Vec<HitRegion>, offset_x: f32, offset_y: f32) {
    let d = &layout.dimensions;

    // Calculate absolute position of this box's content area
    let abs_x = offset_x + d.content.x;
    let abs_y = offset_y + d.content.y;

    // Get node ID from box type
    let node_id = match &layout.box_type {
        BoxType::Block(id, _) => Some(id.0),
        BoxType::Inline(id, _) => Some(id.0),
        BoxType::Text(id, _, _) => Some(id.0),
        BoxType::Input(id, _, _) => Some(id.0),
        BoxType::Button(id, _, _) => Some(id.0),
        BoxType::Image(id, _, _) => Some(id.0),
        BoxType::AnonymousBlock | BoxType::AnonymousInline => None,
    };

    if let Some(id) = node_id {
        if d.content.width > 0.0 && d.content.height > 0.0 {
            regions.push(HitRegion {
                x: abs_x,
                y: abs_y,
                width: d.content.width,
                height: d.content.height,
                node_id: id,
            });
        }
    }

    // Process children - they are positioned relative to this box's content area
    for child in &layout.children {
        build_hit_regions_recursive(child, regions, abs_x, abs_y);
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

/// Extract text content from a <style> element
fn extract_style_content(dom: &DomTree, style_id: gugalanna_dom::NodeId) -> Option<String> {
    // Get all text children of the style element and concatenate them
    let mut css_content = String::new();
    for child_id in dom.children(style_id) {
        if let Some(node) = dom.get(child_id) {
            if let Some(text) = node.as_text() {
                css_content.push_str(text);
            }
        }
    }
    if css_content.is_empty() {
        None
    } else {
        Some(css_content)
    }
}

/// Walk up the DOM tree to find an anchor element with href attribute
fn find_anchor_href(dom: &DomTree, start_id: gugalanna_dom::NodeId) -> Option<(String, gugalanna_dom::NodeId)> {
    let mut current_id = Some(start_id);
    let mut depth = 0;

    while let Some(id) = current_id {
        if let Some(node) = dom.get(id) {
            if let Some(elem) = node.as_element() {
                log::trace!("find_anchor_href: depth={} id={} tag={}", depth, id.0, elem.tag_name);
                if elem.tag_name == "a" {
                    if let Some(href) = elem.get_attribute("href") {
                        log::debug!("Found anchor with href='{}' at depth {}", href, depth);
                        return Some((href.to_string(), id));
                    } else {
                        log::debug!("Found anchor without href at depth {}", depth);
                    }
                }
            } else if let Some(_text) = node.as_text() {
                log::trace!("find_anchor_href: depth={} id={} text node", depth, id.0);
            }
            current_id = node.parent;
            depth += 1;
        } else {
            break;
        }
    }
    log::trace!("find_anchor_href: no anchor found after {} levels", depth);
    None
}

/// Information about a form element that was clicked
#[derive(Debug, Clone)]
enum FormElementInfo {
    /// Text or password input
    TextInput { node_id: NodeId, is_password: bool },
    /// Checkbox
    Checkbox { node_id: NodeId },
    /// Radio button
    Radio { node_id: NodeId, name: String },
    /// Submit button
    Submit { node_id: NodeId },
    /// Regular button
    Button { node_id: NodeId },
}

/// Find form element info for a clicked node
fn find_form_element(dom: &DomTree, start_id: NodeId) -> Option<FormElementInfo> {
    let mut current_id = Some(start_id);

    while let Some(id) = current_id {
        if let Some(node) = dom.get(id) {
            if let Some(elem) = node.as_element() {
                match elem.tag_name.as_str() {
                    "input" => {
                        let input_type = elem.get_attribute("type").unwrap_or("text");
                        match input_type {
                            "text" => return Some(FormElementInfo::TextInput { node_id: id, is_password: false }),
                            "password" => return Some(FormElementInfo::TextInput { node_id: id, is_password: true }),
                            "checkbox" => return Some(FormElementInfo::Checkbox { node_id: id }),
                            "radio" => {
                                let name = elem.get_attribute("name").unwrap_or("").to_string();
                                return Some(FormElementInfo::Radio { node_id: id, name });
                            }
                            "submit" => return Some(FormElementInfo::Submit { node_id: id }),
                            "button" => return Some(FormElementInfo::Button { node_id: id }),
                            _ => {}
                        }
                    }
                    "button" => {
                        let btn_type = elem.get_attribute("type").unwrap_or("submit");
                        if btn_type == "submit" {
                            return Some(FormElementInfo::Submit { node_id: id });
                        } else {
                            return Some(FormElementInfo::Button { node_id: id });
                        }
                    }
                    _ => {}
                }
            }
            current_id = node.parent;
        } else {
            break;
        }
    }
    None
}

/// Find all radio buttons with a given name attribute in the DOM
fn find_radio_buttons_in_group(dom: &DomTree, group_name: &str) -> Vec<NodeId> {
    let mut result = Vec::new();

    // Get all elements and find inputs with type="radio" and matching name
    for element in dom.get_elements_by_tag_name("input") {
        if let Some(node) = dom.get(element) {
            if let Some(elem) = node.as_element() {
                if elem.get_attribute("type") == Some("radio") {
                    if elem.get_attribute("name") == Some(group_name) {
                        result.push(element);
                    }
                }
            }
        }
    }

    result
}

/// Resolve a link href against the current page URL
fn resolve_link_url(base_url: &Url, href: &str) -> Result<Url, String> {
    // Handle empty href (link to self)
    if href.is_empty() {
        return Ok(base_url.clone());
    }

    // Fragment-only link (same page scroll)
    if href.starts_with('#') {
        let mut url = base_url.clone();
        url.set_fragment(Some(&href[1..]));
        return Ok(url);
    }

    // If href is an absolute URL, parse it directly
    if href.contains("://") {
        return Url::parse(href).map_err(|e| e.to_string());
    }

    // Check if base URL can be used as a base (about:blank, data:, etc. cannot)
    if base_url.cannot_be_a_base() {
        return Err(format!(
            "Cannot resolve relative path '{}' against '{}'. \
             The current page URL doesn't support relative path resolution. \
             Use an absolute URL instead.",
            href, base_url.scheme()
        ));
    }

    // Handle file:// URLs specially since they can't use join() for relative paths
    if base_url.scheme() == "file" {
        // For file:// URLs, we can't properly resolve server-relative paths like "/search"
        // because there's no server. Return an error with a helpful message.
        if href.starts_with('/') {
            return Err(format!(
                "Cannot resolve server-relative path '{}' for local file. \
                 Form submission from file:// URLs requires an absolute action URL.",
                href
            ));
        }

        // For relative paths, try to resolve against the file's directory
        if let Some(path) = base_url.to_file_path().ok() {
            if let Some(parent) = path.parent() {
                let resolved = parent.join(href);
                return Url::from_file_path(resolved)
                    .map_err(|_| format!("Invalid file path: {}", href));
            }
        }

        return Err("Cannot resolve relative URL for file:// base".to_string());
    }

    // Use url crate's join() for relative resolution (for http/https URLs)
    base_url.join(href).map_err(|e| e.to_string())
}

/// Find the parent form element for a given node
fn find_parent_form(dom: &DomTree, start_id: NodeId) -> Option<NodeId> {
    let mut current_id = Some(start_id);

    while let Some(id) = current_id {
        if let Some(node) = dom.get(id) {
            if let Some(elem) = node.as_element() {
                if elem.tag_name == "form" {
                    return Some(id);
                }
            }
            current_id = node.parent;
        } else {
            break;
        }
    }
    None
}

/// Form field data for submission
#[derive(Debug)]
struct FormField {
    name: String,
    value: String,
}

/// Collect all form fields from a form element
fn collect_form_data(
    dom: &DomTree,
    form_id: NodeId,
    form_state: &crate::form::FormState,
) -> Vec<FormField> {
    let mut fields = Vec::new();

    // Get all input elements under the form
    for input_id in dom.get_elements_by_tag_name("input") {
        // Check if this input is a descendant of the form
        if !is_descendant_of(dom, input_id, form_id) {
            continue;
        }

        if let Some(node) = dom.get(input_id) {
            if let Some(elem) = node.as_element() {
                let name = match elem.get_attribute("name") {
                    Some(n) if !n.is_empty() => n.to_string(),
                    _ => continue, // Skip inputs without a name
                };

                let input_type = elem.get_attribute("type").unwrap_or("text");
                match input_type {
                    "text" | "password" | "hidden" => {
                        // Get value from form state or attribute
                        let value = form_state
                            .get_value(input_id)
                            .map(|v| v.to_string())
                            .or_else(|| elem.get_attribute("value").map(|v| v.to_string()))
                            .unwrap_or_default();
                        fields.push(FormField { name, value });
                    }
                    "checkbox" => {
                        if form_state.is_checked(input_id) {
                            let value = elem.get_attribute("value").unwrap_or("on").to_string();
                            fields.push(FormField { name, value });
                        }
                    }
                    "radio" => {
                        if form_state.is_checked(input_id) {
                            let value = elem.get_attribute("value").unwrap_or("on").to_string();
                            fields.push(FormField { name, value });
                        }
                    }
                    "submit" | "button" => {
                        // Submit buttons are not included in form data unless clicked
                    }
                    _ => {}
                }
            }
        }
    }

    fields
}

/// Check if a node is a descendant of another node
fn is_descendant_of(dom: &DomTree, node_id: NodeId, ancestor_id: NodeId) -> bool {
    let mut current_id = Some(node_id);
    while let Some(id) = current_id {
        if id == ancestor_id {
            return true;
        }
        if let Some(node) = dom.get(id) {
            current_id = node.parent;
        } else {
            break;
        }
    }
    false
}

/// URL-encode a string for form submission
fn url_encode(s: &str) -> String {
    let mut encoded = String::new();
    for c in s.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                encoded.push(c);
            }
            ' ' => encoded.push('+'),
            _ => {
                for b in c.to_string().as_bytes() {
                    encoded.push_str(&format!("%{:02X}", b));
                }
            }
        }
    }
    encoded
}

/// Build URL-encoded form data string
fn build_form_data_string(fields: &[FormField]) -> String {
    fields
        .iter()
        .map(|f| format!("{}={}", url_encode(&f.name), url_encode(&f.value)))
        .collect::<Vec<_>>()
        .join("&")
}

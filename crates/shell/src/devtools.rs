//! Developer Tools Panel
//!
//! Provides Console, DOM Inspector, Network, and Element Selector panels.

use std::collections::HashSet;

use gugalanna_dom::{DomTree, NodeId, NodeType};
use gugalanna_js::{ConsoleMessage, LogLevel};
use gugalanna_layout::Rect;
use gugalanna_net::NetworkRequest;
use gugalanna_render::{DisplayList, PaintCommand, RenderColor};

/// Height of the DevTools panel when open
pub const DEVTOOLS_HEIGHT: f32 = 250.0;

/// Height of the tab bar within DevTools
const DEVTOOLS_TAB_HEIGHT: f32 = 28.0;

/// DevTools tab type
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DevToolsTab {
    #[default]
    Console,
    Elements,
    Network,
}

/// Hit test result for DevTools
#[derive(Debug, Clone)]
pub enum DevToolsHit {
    /// A tab was clicked
    Tab(DevToolsTab),
    /// Element selector button was clicked
    ElementSelector,
    /// Content area was clicked
    Content { local_x: f32, local_y: f32 },
    /// DOM tree node was clicked
    DomNode(NodeId),
}

/// DevTools state
#[derive(Debug)]
pub struct DevTools {
    /// Whether DevTools panel is open
    pub open: bool,
    /// Current active tab
    pub active_tab: DevToolsTab,
    /// Element selector mode active
    pub element_selector_active: bool,
    /// Currently selected element in DOM inspector
    pub selected_element: Option<NodeId>,
    /// Scroll position for console
    pub console_scroll: f32,
    /// Scroll position for DOM tree
    pub dom_scroll: f32,
    /// Scroll position for network
    pub network_scroll: f32,
    /// Expanded nodes in DOM tree
    pub expanded_nodes: HashSet<NodeId>,
    /// Window width
    width: f32,
}

impl Default for DevTools {
    fn default() -> Self {
        Self::new(800.0)
    }
}

impl DevTools {
    /// Create a new DevTools instance
    pub fn new(window_width: f32) -> Self {
        Self {
            open: false,
            active_tab: DevToolsTab::Console,
            element_selector_active: false,
            selected_element: None,
            console_scroll: 0.0,
            dom_scroll: 0.0,
            network_scroll: 0.0,
            expanded_nodes: HashSet::new(),
            width: window_width,
        }
    }

    /// Toggle DevTools visibility
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// Toggle element selector mode
    pub fn toggle_element_selector(&mut self) {
        self.element_selector_active = !self.element_selector_active;
    }

    /// Update window width
    pub fn update_width(&mut self, width: f32) {
        self.width = width;
    }

    /// Expand path to a node in the DOM tree
    pub fn expand_path_to_node(&mut self, tree: &DomTree, node_id: NodeId) {
        // Walk up the tree and expand all ancestors
        let mut current = node_id;
        while let Some(node) = tree.get(current) {
            self.expanded_nodes.insert(current);
            if let Some(parent) = node.parent {
                current = parent;
            } else {
                break;
            }
        }
    }

    /// Toggle expansion of a DOM node
    pub fn toggle_node_expansion(&mut self, node_id: NodeId) {
        if self.expanded_nodes.contains(&node_id) {
            self.expanded_nodes.remove(&node_id);
        } else {
            self.expanded_nodes.insert(node_id);
        }
    }

    /// Build display list for DevTools panel
    pub fn build_display_list(
        &self,
        viewport_height: f32,
        console_messages: &[ConsoleMessage],
        dom_tree: Option<&DomTree>,
        network_requests: &[NetworkRequest],
    ) -> DisplayList {
        if !self.open {
            return DisplayList { commands: vec![] };
        }

        let mut commands = Vec::new();
        let panel_y = viewport_height - DEVTOOLS_HEIGHT;

        // Background
        commands.push(PaintCommand::FillRect {
            rect: Rect {
                x: 0.0,
                y: panel_y,
                width: self.width,
                height: DEVTOOLS_HEIGHT,
            },
            color: RenderColor::new(36, 36, 36, 255), // Dark background
        });

        // Top border
        commands.push(PaintCommand::FillRect {
            rect: Rect {
                x: 0.0,
                y: panel_y,
                width: self.width,
                height: 1.0,
            },
            color: RenderColor::new(60, 60, 60, 255),
        });

        // Tab bar
        self.build_tab_bar(&mut commands, panel_y);

        // Content area
        let content_y = panel_y + DEVTOOLS_TAB_HEIGHT;
        let content_height = DEVTOOLS_HEIGHT - DEVTOOLS_TAB_HEIGHT;

        match self.active_tab {
            DevToolsTab::Console => {
                self.build_console_panel(&mut commands, content_y, content_height, console_messages);
            }
            DevToolsTab::Elements => {
                if let Some(tree) = dom_tree {
                    self.build_elements_panel(&mut commands, content_y, content_height, tree);
                }
            }
            DevToolsTab::Network => {
                self.build_network_panel(&mut commands, content_y, content_height, network_requests);
            }
        }

        DisplayList { commands }
    }

    fn build_tab_bar(&self, commands: &mut Vec<PaintCommand>, y: f32) {
        // Tab bar background
        commands.push(PaintCommand::FillRect {
            rect: Rect {
                x: 0.0,
                y,
                width: self.width,
                height: DEVTOOLS_TAB_HEIGHT,
            },
            color: RenderColor::new(45, 45, 45, 255),
        });

        let tabs = [
            (DevToolsTab::Console, "Console"),
            (DevToolsTab::Elements, "Elements"),
            (DevToolsTab::Network, "Network"),
        ];

        let mut x = 10.0;
        let tab_width = 80.0;
        for (tab, label) in tabs {
            let is_active = self.active_tab == tab;

            // Tab background
            commands.push(PaintCommand::FillRect {
                rect: Rect {
                    x,
                    y: y + 2.0,
                    width: tab_width,
                    height: DEVTOOLS_TAB_HEIGHT - 2.0,
                },
                color: if is_active {
                    RenderColor::new(50, 50, 50, 255)
                } else {
                    RenderColor::new(45, 45, 45, 255)
                },
            });

            // Tab text
            commands.push(PaintCommand::DrawText {
                x: x + 10.0,
                y: y + 8.0,
                text: label.to_string(),
                color: if is_active {
                    RenderColor::new(255, 255, 255, 255)
                } else {
                    RenderColor::new(160, 160, 160, 255)
                },
                font_size: 12.0,
            });

            x += tab_width + 4.0;
        }

        // Element selector button
        let selector_x = self.width - 100.0;
        commands.push(PaintCommand::FillRect {
            rect: Rect {
                x: selector_x,
                y: y + 4.0,
                width: 90.0,
                height: DEVTOOLS_TAB_HEIGHT - 8.0,
            },
            color: if self.element_selector_active {
                RenderColor::new(74, 144, 226, 255) // Blue when active
            } else {
                RenderColor::new(60, 60, 60, 255)
            },
        });
        commands.push(PaintCommand::DrawText {
            x: selector_x + 8.0,
            y: y + 8.0,
            text: "Select".to_string(),
            color: RenderColor::new(255, 255, 255, 255),
            font_size: 12.0,
        });
    }

    fn build_console_panel(
        &self,
        commands: &mut Vec<PaintCommand>,
        y: f32,
        height: f32,
        messages: &[ConsoleMessage],
    ) {
        let line_height = 18.0;
        let mut line_y = y + 8.0 - self.console_scroll;

        for msg in messages {
            if line_y > y - line_height && line_y < y + height {
                // Level indicator
                let (indicator, color) = match msg.level {
                    LogLevel::Log => ("", RenderColor::new(200, 200, 200, 255)),
                    LogLevel::Info => ("i", RenderColor::new(100, 180, 255, 255)),
                    LogLevel::Warn => ("!", RenderColor::new(255, 200, 100, 255)),
                    LogLevel::Error => ("X", RenderColor::new(255, 100, 100, 255)),
                    LogLevel::Debug => ("D", RenderColor::new(150, 150, 150, 255)),
                };

                if !indicator.is_empty() {
                    commands.push(PaintCommand::DrawText {
                        x: 10.0,
                        y: line_y,
                        text: indicator.to_string(),
                        color,
                        font_size: 12.0,
                    });
                }

                commands.push(PaintCommand::DrawText {
                    x: 30.0,
                    y: line_y,
                    text: msg.message.clone(),
                    color,
                    font_size: 12.0,
                });
            }
            line_y += line_height;
        }

        // Empty state
        if messages.is_empty() {
            commands.push(PaintCommand::DrawText {
                x: 10.0,
                y: y + 8.0,
                text: "No console messages".to_string(),
                color: RenderColor::new(120, 120, 120, 255),
                font_size: 12.0,
            });
        }
    }

    fn build_elements_panel(
        &self,
        commands: &mut Vec<PaintCommand>,
        y: f32,
        height: f32,
        tree: &DomTree,
    ) {
        let line_height = 18.0;
        let mut line_y = y + 8.0 - self.dom_scroll;

        // Render DOM tree starting from document root
        let root_id = tree.document_id();
        self.render_dom_node(commands, tree, root_id, 0, &mut line_y, y, height);
    }

    fn render_dom_node(
        &self,
        commands: &mut Vec<PaintCommand>,
        tree: &DomTree,
        node_id: NodeId,
        depth: i32,
        line_y: &mut f32,
        panel_y: f32,
        panel_height: f32,
    ) {
        let indent = 10.0 + depth as f32 * 16.0;
        let line_height = 18.0;

        if let Some(node) = tree.get(node_id) {
            // Only render if visible
            if *line_y > panel_y - line_height && *line_y < panel_y + panel_height {
                let is_selected = self.selected_element == Some(node_id);

                // Selection highlight
                if is_selected {
                    commands.push(PaintCommand::FillRect {
                        rect: Rect {
                            x: 0.0,
                            y: *line_y - 2.0,
                            width: self.width,
                            height: line_height,
                        },
                        color: RenderColor::new(74, 144, 226, 100),
                    });
                }

                // Node text
                let text = match &node.node_type {
                    NodeType::Element(elem) => {
                        let tag = &elem.tag_name;
                        let id = elem
                            .get_attribute("id")
                            .map(|i| format!("#{}", i))
                            .unwrap_or_default();
                        let class = elem
                            .get_attribute("class")
                            .map(|c| {
                                format!(
                                    ".{}",
                                    c.split_whitespace().collect::<Vec<_>>().join(".")
                                )
                            })
                            .unwrap_or_default();
                        format!("<{}{}{}>", tag, id, class)
                    }
                    NodeType::Text(text) => {
                        let trimmed = text.trim();
                        if trimmed.is_empty() {
                            // Skip empty text nodes
                            *line_y += line_height;
                            return;
                        }
                        let truncated = if trimmed.len() > 40 {
                            format!("\"{}...\"", &trimmed[..40])
                        } else {
                            format!("\"{}\"", trimmed)
                        };
                        truncated
                    }
                    NodeType::Comment(_) => "<!-- comment -->".to_string(),
                    NodeType::Document => "#document".to_string(),
                    NodeType::Doctype { .. } => "<!DOCTYPE>".to_string(),
                };

                let color = match &node.node_type {
                    NodeType::Element(_) => RenderColor::new(136, 180, 255, 255),
                    NodeType::Text(_) => RenderColor::new(200, 200, 200, 255),
                    _ => RenderColor::new(120, 120, 120, 255),
                };

                // Expand/collapse indicator for elements with children
                let has_expandable_children = !node.children.is_empty();
                if matches!(&node.node_type, NodeType::Element(_) | NodeType::Document)
                    && has_expandable_children
                {
                    let expanded = self.expanded_nodes.contains(&node_id) || depth < 2;
                    commands.push(PaintCommand::DrawText {
                        x: indent - 12.0,
                        y: *line_y,
                        text: if expanded { "v" } else { ">" }.to_string(),
                        color: RenderColor::new(150, 150, 150, 255),
                        font_size: 10.0,
                    });
                }

                commands.push(PaintCommand::DrawText {
                    x: indent,
                    y: *line_y,
                    text,
                    color,
                    font_size: 12.0,
                });
            }

            *line_y += line_height;

            // Render children if expanded
            let expanded = self.expanded_nodes.contains(&node_id) || depth < 2;
            if expanded {
                for child_id in tree.children(node_id) {
                    self.render_dom_node(
                        commands,
                        tree,
                        child_id,
                        depth + 1,
                        line_y,
                        panel_y,
                        panel_height,
                    );
                }
            }
        }
    }

    fn build_network_panel(
        &self,
        commands: &mut Vec<PaintCommand>,
        y: f32,
        height: f32,
        requests: &[NetworkRequest],
    ) {
        let line_height = 20.0;

        // Header
        commands.push(PaintCommand::FillRect {
            rect: Rect {
                x: 0.0,
                y,
                width: self.width,
                height: line_height,
            },
            color: RenderColor::new(45, 45, 45, 255),
        });

        let headers = ["Status", "Method", "URL", "Size", "Time"];
        let cols = [10.0, 70.0, 130.0, 500.0, 580.0];
        for (i, header) in headers.iter().enumerate() {
            commands.push(PaintCommand::DrawText {
                x: cols[i],
                y: y + 4.0,
                text: header.to_string(),
                color: RenderColor::new(160, 160, 160, 255),
                font_size: 11.0,
            });
        }

        // Requests
        let mut line_y = y + line_height + 4.0 - self.network_scroll;

        for req in requests {
            if line_y > y && line_y < y + height {
                // Status
                let status_color = match req.status {
                    Some(s) if s >= 200 && s < 300 => RenderColor::new(100, 200, 100, 255),
                    Some(s) if s >= 400 => RenderColor::new(255, 100, 100, 255),
                    Some(_) => RenderColor::new(200, 200, 100, 255),
                    None => RenderColor::new(150, 150, 150, 255),
                };
                commands.push(PaintCommand::DrawText {
                    x: cols[0],
                    y: line_y,
                    text: req.status.map(|s| s.to_string()).unwrap_or("...".to_string()),
                    color: status_color,
                    font_size: 11.0,
                });

                // Method
                commands.push(PaintCommand::DrawText {
                    x: cols[1],
                    y: line_y,
                    text: req.method.clone(),
                    color: RenderColor::new(200, 200, 200, 255),
                    font_size: 11.0,
                });

                // URL (truncated)
                let url = if req.url.len() > 50 {
                    format!("{}...", &req.url[..50])
                } else {
                    req.url.clone()
                };
                commands.push(PaintCommand::DrawText {
                    x: cols[2],
                    y: line_y,
                    text: url,
                    color: RenderColor::new(180, 180, 180, 255),
                    font_size: 11.0,
                });

                // Size
                if let Some(size) = req.response_size {
                    let size_str = if size > 1024 * 1024 {
                        format!("{:.1} MB", size as f64 / 1024.0 / 1024.0)
                    } else if size > 1024 {
                        format!("{:.1} KB", size as f64 / 1024.0)
                    } else {
                        format!("{} B", size)
                    };
                    commands.push(PaintCommand::DrawText {
                        x: cols[3],
                        y: line_y,
                        text: size_str,
                        color: RenderColor::new(150, 150, 150, 255),
                        font_size: 11.0,
                    });
                }

                // Time
                if let Some(duration) = req.duration {
                    let time_str = if duration.as_millis() > 1000 {
                        format!("{:.2}s", duration.as_secs_f64())
                    } else {
                        format!("{}ms", duration.as_millis())
                    };
                    commands.push(PaintCommand::DrawText {
                        x: cols[4],
                        y: line_y,
                        text: time_str,
                        color: RenderColor::new(150, 150, 150, 255),
                        font_size: 11.0,
                    });
                }
            }
            line_y += line_height;
        }

        // Empty state
        if requests.is_empty() {
            commands.push(PaintCommand::DrawText {
                x: 10.0,
                y: y + line_height + 8.0,
                text: "No network requests".to_string(),
                color: RenderColor::new(120, 120, 120, 255),
                font_size: 12.0,
            });
        }
    }

    /// Hit test within DevTools panel
    pub fn hit_test(&self, x: f32, y: f32, viewport_height: f32) -> Option<DevToolsHit> {
        if !self.open {
            return None;
        }

        let panel_y = viewport_height - DEVTOOLS_HEIGHT;
        if y < panel_y {
            return None;
        }

        // Tab bar hit test
        if y < panel_y + DEVTOOLS_TAB_HEIGHT {
            // Check tabs
            let tabs = [DevToolsTab::Console, DevToolsTab::Elements, DevToolsTab::Network];
            let mut tab_x = 10.0;
            for tab in tabs {
                if x >= tab_x && x < tab_x + 80.0 {
                    return Some(DevToolsHit::Tab(tab));
                }
                tab_x += 84.0;
            }

            // Check element selector button
            let selector_x = self.width - 100.0;
            if x >= selector_x && x < selector_x + 90.0 {
                return Some(DevToolsHit::ElementSelector);
            }
        }

        // Content area hit test
        Some(DevToolsHit::Content {
            local_x: x,
            local_y: y - panel_y - DEVTOOLS_TAB_HEIGHT,
        })
    }

    /// Handle scroll in the active panel
    pub fn scroll(&mut self, delta: f32) {
        match self.active_tab {
            DevToolsTab::Console => {
                self.console_scroll = (self.console_scroll + delta).max(0.0);
            }
            DevToolsTab::Elements => {
                self.dom_scroll = (self.dom_scroll + delta).max(0.0);
            }
            DevToolsTab::Network => {
                self.network_scroll = (self.network_scroll + delta).max(0.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_devtools_toggle() {
        let mut devtools = DevTools::new(800.0);
        assert!(!devtools.open);

        devtools.toggle();
        assert!(devtools.open);

        devtools.toggle();
        assert!(!devtools.open);
    }

    #[test]
    fn test_devtools_element_selector_toggle() {
        let mut devtools = DevTools::new(800.0);
        assert!(!devtools.element_selector_active);

        devtools.toggle_element_selector();
        assert!(devtools.element_selector_active);

        devtools.toggle_element_selector();
        assert!(!devtools.element_selector_active);
    }

    #[test]
    fn test_devtools_hit_test_closed() {
        let devtools = DevTools::new(800.0);
        assert!(devtools.hit_test(400.0, 500.0, 600.0).is_none());
    }

    #[test]
    fn test_devtools_hit_test_tab() {
        let mut devtools = DevTools::new(800.0);
        devtools.open = true;

        // Hit test on Console tab (at x=10-90)
        let hit = devtools.hit_test(50.0, 600.0 - DEVTOOLS_HEIGHT + 10.0, 600.0);
        assert!(matches!(hit, Some(DevToolsHit::Tab(DevToolsTab::Console))));

        // Hit test on Elements tab (at x=94-174)
        let hit = devtools.hit_test(130.0, 600.0 - DEVTOOLS_HEIGHT + 10.0, 600.0);
        assert!(matches!(hit, Some(DevToolsHit::Tab(DevToolsTab::Elements))));
    }

    #[test]
    fn test_devtools_hit_test_selector() {
        let mut devtools = DevTools::new(800.0);
        devtools.open = true;

        // Hit test on element selector button
        let hit = devtools.hit_test(750.0, 600.0 - DEVTOOLS_HEIGHT + 10.0, 600.0);
        assert!(matches!(hit, Some(DevToolsHit::ElementSelector)));
    }

    #[test]
    fn test_devtools_scroll() {
        let mut devtools = DevTools::new(800.0);

        devtools.active_tab = DevToolsTab::Console;
        devtools.scroll(50.0);
        assert_eq!(devtools.console_scroll, 50.0);

        devtools.scroll(-100.0); // Should clamp to 0
        assert_eq!(devtools.console_scroll, 0.0);
    }
}

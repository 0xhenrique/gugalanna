//! Form State Management
//!
//! Tracks runtime state for form elements separate from the DOM.
//! This allows user input to be tracked without constantly modifying DOM attributes.

use gugalanna_dom::NodeId;
use rustc_hash::FxHashMap;

/// Tracks runtime state for form elements
#[derive(Debug, Default, Clone)]
pub struct FormState {
    /// Text values for text/password inputs (keyed by node ID)
    text_values: FxHashMap<NodeId, TextInputState>,
    /// Checked state for checkboxes/radios (keyed by node ID)
    checked: FxHashMap<NodeId, bool>,
}

/// State for a text input element
#[derive(Debug, Clone)]
pub struct TextInputState {
    /// Current text value
    pub value: String,
    /// Cursor position (byte offset)
    pub cursor_pos: usize,
}

impl TextInputState {
    /// Create a new text input state with the given initial value
    pub fn new(value: String) -> Self {
        let cursor_pos = value.len();
        Self { value, cursor_pos }
    }

    /// Insert text at the current cursor position
    pub fn insert_text(&mut self, text: &str) {
        self.value.insert_str(self.cursor_pos, text);
        self.cursor_pos += text.len();
    }

    /// Insert a single character at the current cursor position
    pub fn insert_char(&mut self, c: char) {
        self.value.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    /// Delete the character before the cursor (backspace)
    pub fn delete_char_before(&mut self) {
        if self.cursor_pos > 0 {
            // Find the start of the previous character
            let prev_char_boundary = self.value[..self.cursor_pos]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.value.remove(prev_char_boundary);
            self.cursor_pos = prev_char_boundary;
        }
    }

    /// Delete the character after the cursor (delete key)
    pub fn delete_char_after(&mut self) {
        if self.cursor_pos < self.value.len() {
            self.value.remove(self.cursor_pos);
        }
    }

    /// Move cursor left by one character
    pub fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos = self.value[..self.cursor_pos]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    /// Move cursor right by one character
    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.value.len() {
            self.cursor_pos = self.value[self.cursor_pos..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor_pos + i)
                .unwrap_or(self.value.len());
        }
    }

    /// Move cursor to the start
    pub fn move_cursor_to_start(&mut self) {
        self.cursor_pos = 0;
    }

    /// Move cursor to the end
    pub fn move_cursor_to_end(&mut self) {
        self.cursor_pos = self.value.len();
    }

    /// Set the value and move cursor to end
    pub fn set_value(&mut self, value: String) {
        self.value = value;
        self.cursor_pos = self.value.len();
    }

    /// Clear the input
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor_pos = 0;
    }
}

impl FormState {
    /// Create a new empty form state
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the text state for an input (immutable)
    pub fn get_text(&self, node_id: NodeId) -> Option<&TextInputState> {
        self.text_values.get(&node_id)
    }

    /// Get the text state for an input (mutable)
    pub fn get_text_mut(&mut self, node_id: NodeId) -> Option<&mut TextInputState> {
        self.text_values.get_mut(&node_id)
    }

    /// Set or create text state for an input
    pub fn set_text(&mut self, node_id: NodeId, value: String) {
        self.text_values.insert(node_id, TextInputState::new(value));
    }

    /// Ensure text state exists for an input, creating empty state if needed
    pub fn ensure_text(&mut self, node_id: NodeId) -> &mut TextInputState {
        self.text_values
            .entry(node_id)
            .or_insert_with(|| TextInputState::new(String::new()))
    }

    /// Check if a checkbox/radio is checked
    pub fn is_checked(&self, node_id: NodeId) -> bool {
        self.checked.get(&node_id).copied().unwrap_or(false)
    }

    /// Set the checked state for a checkbox/radio
    pub fn set_checked(&mut self, node_id: NodeId, checked: bool) {
        self.checked.insert(node_id, checked);
    }

    /// Toggle the checked state for a checkbox
    pub fn toggle_checked(&mut self, node_id: NodeId) {
        let current = self.is_checked(node_id);
        self.checked.insert(node_id, !current);
    }

    /// Clear all form state
    pub fn clear(&mut self) {
        self.text_values.clear();
        self.checked.clear();
    }

    /// Get the text value for an input (convenience method)
    pub fn get_value(&self, node_id: NodeId) -> Option<&str> {
        self.text_values.get(&node_id).map(|s| s.value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_input_state_basic() {
        let mut state = TextInputState::new("hello".to_string());
        assert_eq!(state.value, "hello");
        assert_eq!(state.cursor_pos, 5);
    }

    #[test]
    fn test_text_input_insert() {
        let mut state = TextInputState::new(String::new());
        state.insert_char('h');
        state.insert_char('i');
        assert_eq!(state.value, "hi");
        assert_eq!(state.cursor_pos, 2);
    }

    #[test]
    fn test_text_input_delete() {
        let mut state = TextInputState::new("hello".to_string());
        state.delete_char_before();
        assert_eq!(state.value, "hell");
        assert_eq!(state.cursor_pos, 4);
    }

    #[test]
    fn test_text_input_cursor_movement() {
        let mut state = TextInputState::new("hello".to_string());
        state.move_cursor_left();
        assert_eq!(state.cursor_pos, 4);
        state.move_cursor_to_start();
        assert_eq!(state.cursor_pos, 0);
        state.move_cursor_right();
        assert_eq!(state.cursor_pos, 1);
    }

    #[test]
    fn test_form_state_text() {
        let mut form = FormState::new();
        let node_id = NodeId::new(1);

        form.set_text(node_id, "test".to_string());
        assert_eq!(form.get_value(node_id), Some("test"));
    }

    #[test]
    fn test_form_state_checkbox() {
        let mut form = FormState::new();
        let node_id = NodeId::new(1);

        assert!(!form.is_checked(node_id));
        form.toggle_checked(node_id);
        assert!(form.is_checked(node_id));
        form.toggle_checked(node_id);
        assert!(!form.is_checked(node_id));
    }
}

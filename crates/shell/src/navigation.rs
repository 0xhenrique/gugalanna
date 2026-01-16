//! Navigation state and history management
//!
//! Handles back/forward navigation with a history stack.

use url::Url;

/// Navigation state with history stack
#[derive(Debug)]
pub struct NavigationState {
    /// History stack (all visited URLs)
    history: Vec<Url>,
    /// Current position in history (0-indexed, -1 if empty)
    current_index: i32,
}

impl NavigationState {
    /// Create a new empty navigation state
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            current_index: -1,
        }
    }

    /// Get the current URL, if any
    pub fn current_url(&self) -> Option<&Url> {
        if self.current_index >= 0 && (self.current_index as usize) < self.history.len() {
            Some(&self.history[self.current_index as usize])
        } else {
            None
        }
    }

    /// Check if we can go back
    pub fn can_go_back(&self) -> bool {
        self.current_index > 0
    }

    /// Check if we can go forward
    pub fn can_go_forward(&self) -> bool {
        self.current_index >= 0 && (self.current_index as usize) < self.history.len() - 1
    }

    /// Navigate to a new URL
    ///
    /// This clears any forward history (pages we went back from).
    pub fn navigate_to(&mut self, url: Url) {
        // If we're not at the end of history, truncate forward history
        if self.current_index >= 0 {
            let new_len = (self.current_index + 1) as usize;
            self.history.truncate(new_len);
        }

        // Add new URL to history
        self.history.push(url);
        self.current_index = (self.history.len() - 1) as i32;
    }

    /// Go back in history
    ///
    /// Returns the URL to navigate to, or None if at the beginning.
    pub fn go_back(&mut self) -> Option<&Url> {
        if self.can_go_back() {
            self.current_index -= 1;
            self.current_url()
        } else {
            None
        }
    }

    /// Go forward in history
    ///
    /// Returns the URL to navigate to, or None if at the end.
    pub fn go_forward(&mut self) -> Option<&Url> {
        if self.can_go_forward() {
            self.current_index += 1;
            self.current_url()
        } else {
            None
        }
    }

    /// Get the number of entries in history
    pub fn len(&self) -> usize {
        self.history.len()
    }

    /// Check if history is empty
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }
}

impl Default for NavigationState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn test_empty_history() {
        let nav = NavigationState::new();
        assert!(nav.is_empty());
        assert!(nav.current_url().is_none());
        assert!(!nav.can_go_back());
        assert!(!nav.can_go_forward());
    }

    #[test]
    fn test_navigate_to() {
        let mut nav = NavigationState::new();

        nav.navigate_to(url("https://example.com"));
        assert_eq!(nav.len(), 1);
        assert_eq!(nav.current_url().unwrap().as_str(), "https://example.com/");
        assert!(!nav.can_go_back());
        assert!(!nav.can_go_forward());
    }

    #[test]
    fn test_multiple_navigations() {
        let mut nav = NavigationState::new();

        nav.navigate_to(url("https://page1.com"));
        nav.navigate_to(url("https://page2.com"));
        nav.navigate_to(url("https://page3.com"));

        assert_eq!(nav.len(), 3);
        assert_eq!(nav.current_url().unwrap().as_str(), "https://page3.com/");
        assert!(nav.can_go_back());
        assert!(!nav.can_go_forward());
    }

    #[test]
    fn test_go_back() {
        let mut nav = NavigationState::new();

        nav.navigate_to(url("https://page1.com"));
        nav.navigate_to(url("https://page2.com"));

        let back_url = nav.go_back().unwrap();
        assert_eq!(back_url.as_str(), "https://page1.com/");
        assert!(!nav.can_go_back());
        assert!(nav.can_go_forward());
    }

    #[test]
    fn test_go_forward() {
        let mut nav = NavigationState::new();

        nav.navigate_to(url("https://page1.com"));
        nav.navigate_to(url("https://page2.com"));
        nav.go_back();

        let forward_url = nav.go_forward().unwrap();
        assert_eq!(forward_url.as_str(), "https://page2.com/");
        assert!(nav.can_go_back());
        assert!(!nav.can_go_forward());
    }

    #[test]
    fn test_navigate_clears_forward_history() {
        let mut nav = NavigationState::new();

        nav.navigate_to(url("https://page1.com"));
        nav.navigate_to(url("https://page2.com"));
        nav.navigate_to(url("https://page3.com"));

        // Go back twice
        nav.go_back();
        nav.go_back();
        assert_eq!(nav.current_url().unwrap().as_str(), "https://page1.com/");

        // Navigate to new page - should clear forward history
        nav.navigate_to(url("https://page4.com"));

        assert_eq!(nav.len(), 2);
        assert_eq!(nav.current_url().unwrap().as_str(), "https://page4.com/");
        assert!(nav.can_go_back());
        assert!(!nav.can_go_forward());
    }

    #[test]
    fn test_go_back_at_start_returns_none() {
        let mut nav = NavigationState::new();
        nav.navigate_to(url("https://page1.com"));

        assert!(nav.go_back().is_none());
        assert_eq!(nav.current_url().unwrap().as_str(), "https://page1.com/");
    }

    #[test]
    fn test_go_forward_at_end_returns_none() {
        let mut nav = NavigationState::new();
        nav.navigate_to(url("https://page1.com"));

        assert!(nav.go_forward().is_none());
        assert_eq!(nav.current_url().unwrap().as_str(), "https://page1.com/");
    }
}

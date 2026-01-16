//! CSS Transition Animation System
//!
//! Tracks active transitions and interpolates property values over time.

use gugalanna_style::TimingFunction;
use std::collections::HashMap;

/// Active transition for a single property
#[derive(Debug, Clone)]
pub struct ActiveTransition {
    /// Property being animated
    pub property: String,
    /// Starting value
    pub start_value: f32,
    /// Target value
    pub end_value: f32,
    /// Duration in milliseconds
    pub duration_ms: f32,
    /// Delay before starting in milliseconds
    pub delay_ms: f32,
    /// Elapsed time in milliseconds
    pub elapsed_ms: f32,
    /// Easing function
    pub timing_function: TimingFunction,
}

impl ActiveTransition {
    /// Create a new active transition
    pub fn new(
        property: String,
        start: f32,
        end: f32,
        duration_ms: f32,
        delay_ms: f32,
        timing_function: TimingFunction,
    ) -> Self {
        Self {
            property,
            start_value: start,
            end_value: end,
            duration_ms,
            delay_ms,
            elapsed_ms: 0.0,
            timing_function,
        }
    }

    /// Calculate current interpolated value
    pub fn current_value(&self) -> f32 {
        // Account for delay
        let active_elapsed = (self.elapsed_ms - self.delay_ms).max(0.0);

        // Handle zero duration (instant transition)
        if self.duration_ms <= 0.0 {
            return self.end_value;
        }

        // Calculate progress (0.0 to 1.0)
        let progress = (active_elapsed / self.duration_ms).clamp(0.0, 1.0);

        // Apply easing function
        let eased = apply_easing(progress, self.timing_function);

        // Interpolate between start and end
        self.start_value + (self.end_value - self.start_value) * eased
    }

    /// Check if transition is still in delay phase
    pub fn is_delayed(&self) -> bool {
        self.elapsed_ms < self.delay_ms
    }

    /// Check if transition is complete
    pub fn is_complete(&self) -> bool {
        self.elapsed_ms >= self.delay_ms + self.duration_ms
    }

    /// Get the progress (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        let active_elapsed = (self.elapsed_ms - self.delay_ms).max(0.0);
        if self.duration_ms <= 0.0 {
            1.0
        } else {
            (active_elapsed / self.duration_ms).clamp(0.0, 1.0)
        }
    }
}

/// Apply easing function to a progress value (0.0 to 1.0)
pub fn apply_easing(t: f32, timing: TimingFunction) -> f32 {
    match timing {
        TimingFunction::Linear => t,
        TimingFunction::Ease => cubic_bezier(t, 0.25, 0.1, 0.25, 1.0),
        TimingFunction::EaseIn => cubic_bezier(t, 0.42, 0.0, 1.0, 1.0),
        TimingFunction::EaseOut => cubic_bezier(t, 0.0, 0.0, 0.58, 1.0),
        TimingFunction::EaseInOut => cubic_bezier(t, 0.42, 0.0, 0.58, 1.0),
        TimingFunction::CubicBezier(x1, y1, x2, y2) => cubic_bezier(t, x1, y1, x2, y2),
    }
}

/// Calculate cubic bezier curve value at time t
/// Uses binary search to find the x parameter, then evaluates y
fn cubic_bezier(t: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    // For t = 0 or t = 1, return exact values
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }

    // Binary search to find the bezier parameter that gives us our x value
    let mut low = 0.0_f32;
    let mut high = 1.0_f32;

    // 16 iterations gives us good precision
    for _ in 0..16 {
        let mid = (low + high) / 2.0;
        let x = bezier_point(mid, x1, x2);
        if x < t {
            low = mid;
        } else {
            high = mid;
        }
    }

    // Evaluate y at the found parameter
    let param = (low + high) / 2.0;
    bezier_point(param, y1, y2)
}

/// Evaluate a cubic bezier curve at parameter t
/// B(t) = 3(1-t)^2*t*p1 + 3(1-t)*t^2*p2 + t^3
fn bezier_point(t: f32, p1: f32, p2: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    3.0 * mt2 * t * p1 + 3.0 * mt * t2 * p2 + t3
}

/// Transition manager for tracking active transitions
#[derive(Debug, Default)]
pub struct TransitionManager {
    /// Active transitions by element ID
    active: HashMap<usize, Vec<ActiveTransition>>,
}

impl TransitionManager {
    /// Create a new transition manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Update all transitions by delta time
    /// Returns true if any transitions are still active
    pub fn tick(&mut self, delta_ms: f32) -> bool {
        let mut any_active = false;

        for transitions in self.active.values_mut() {
            // Update elapsed time and remove completed transitions
            transitions.retain_mut(|t| {
                t.elapsed_ms += delta_ms;
                !t.is_complete()
            });
            if !transitions.is_empty() {
                any_active = true;
            }
        }

        // Clean up empty entries
        self.active.retain(|_, v| !v.is_empty());
        any_active
    }

    /// Start a transition for a property change
    pub fn start_transition(
        &mut self,
        element_id: usize,
        property: String,
        start: f32,
        end: f32,
        duration_ms: f32,
        delay_ms: f32,
        timing: TimingFunction,
    ) {
        // Remove any existing transition for this property
        if let Some(transitions) = self.active.get_mut(&element_id) {
            transitions.retain(|t| t.property != property);
        }

        // Create new transition
        let transition = ActiveTransition::new(property, start, end, duration_ms, delay_ms, timing);

        // Add to element's transitions
        self.active.entry(element_id).or_default().push(transition);
    }

    /// Get current animated value for a property
    /// Returns None if no transition is active for this property
    pub fn get_animated_value(&self, element_id: usize, property: &str) -> Option<f32> {
        self.active
            .get(&element_id)?
            .iter()
            .find(|t| t.property == property)
            .map(|t| t.current_value())
    }

    /// Check if any transitions are active
    pub fn has_active_transitions(&self) -> bool {
        !self.active.is_empty()
    }

    /// Iterate over all active transitions (element_id, transitions)
    pub fn iter_active(&self) -> impl Iterator<Item = (usize, &Vec<ActiveTransition>)> {
        self.active.iter().map(|(k, v)| (*k, v))
    }

    /// Get all active transitions for an element
    pub fn get_transitions(&self, element_id: usize) -> Option<&Vec<ActiveTransition>> {
        self.active.get(&element_id)
    }

    /// Clear all transitions for an element
    pub fn clear_element(&mut self, element_id: usize) {
        self.active.remove(&element_id);
    }

    /// Clear all transitions
    pub fn clear_all(&mut self) {
        self.active.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_easing() {
        assert_eq!(apply_easing(0.0, TimingFunction::Linear), 0.0);
        assert_eq!(apply_easing(0.5, TimingFunction::Linear), 0.5);
        assert_eq!(apply_easing(1.0, TimingFunction::Linear), 1.0);
    }

    #[test]
    fn test_ease_easing() {
        let start = apply_easing(0.0, TimingFunction::Ease);
        let mid = apply_easing(0.5, TimingFunction::Ease);
        let end = apply_easing(1.0, TimingFunction::Ease);

        assert!(start.abs() < 0.001);
        assert!((end - 1.0).abs() < 0.001);
        // Ease should be slower at start and faster in middle
        assert!(mid > 0.5);
    }

    #[test]
    fn test_ease_in_easing() {
        let mid = apply_easing(0.5, TimingFunction::EaseIn);
        // Ease-in should be slower at the start
        assert!(mid < 0.5);
    }

    #[test]
    fn test_ease_out_easing() {
        let mid = apply_easing(0.5, TimingFunction::EaseOut);
        // Ease-out should be faster at the start
        assert!(mid > 0.5);
    }

    #[test]
    fn test_transition_interpolation() {
        let mut transition = ActiveTransition::new(
            "width".to_string(),
            100.0,
            200.0,
            1000.0, // 1 second
            0.0,
            TimingFunction::Linear,
        );

        // At start
        assert_eq!(transition.current_value(), 100.0);

        // At 50%
        transition.elapsed_ms = 500.0;
        assert_eq!(transition.current_value(), 150.0);

        // At 100%
        transition.elapsed_ms = 1000.0;
        assert_eq!(transition.current_value(), 200.0);
    }

    #[test]
    fn test_transition_with_delay() {
        let mut transition = ActiveTransition::new(
            "opacity".to_string(),
            0.0,
            1.0,
            500.0,
            200.0, // 200ms delay
            TimingFunction::Linear,
        );

        // During delay, should stay at start value
        transition.elapsed_ms = 100.0;
        assert_eq!(transition.current_value(), 0.0);
        assert!(transition.is_delayed());

        // After delay starts
        transition.elapsed_ms = 450.0; // 200ms delay + 250ms into animation
        assert!(!transition.is_delayed());
        assert_eq!(transition.current_value(), 0.5);
    }

    #[test]
    fn test_transition_manager() {
        let mut manager = TransitionManager::new();

        manager.start_transition(
            1,
            "width".to_string(),
            100.0,
            200.0,
            1000.0,
            0.0,
            TimingFunction::Linear,
        );

        assert!(manager.has_active_transitions());

        // Tick forward
        manager.tick(500.0);
        let value = manager.get_animated_value(1, "width");
        assert_eq!(value, Some(150.0));

        // Complete the transition
        manager.tick(600.0);
        assert!(!manager.has_active_transitions());
    }

    #[test]
    fn test_cubic_bezier_custom() {
        // Test custom cubic-bezier that should behave like linear
        let mid = apply_easing(
            0.5,
            TimingFunction::CubicBezier(0.333, 0.333, 0.666, 0.666),
        );
        assert!((mid - 0.5).abs() < 0.05); // Should be close to linear
    }
}

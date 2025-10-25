//! Selection state machine and logic

use std::time::Instant;
use crate::constants::CLICK_TIMEOUT_MS;

/// Selection State Machine
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionState {
    /// No selection active
    Idle,
    /// Mouse button pressed, waiting to see if it's a click or drag
    Pressed { start: (usize, usize), timestamp: Instant },
    /// Actively dragging to extend selection
    Dragging { start: (usize, usize), current: (usize, usize) },
    /// Selection is complete and visible
    Complete { start: (usize, usize), end: (usize, usize) },
}

#[derive(Debug, Clone)]
pub struct Selection {
    state: SelectionState,
}

impl Default for Selection {
    fn default() -> Self {
        Self::new()
    }
}

impl Selection {
    pub fn new() -> Self {
        Self {
            state: SelectionState::Idle,
        }
    }

    pub fn is_active(&self) -> bool {
        !matches!(self.state, SelectionState::Idle)
    }

    pub fn get_bounds(&self) -> Option<((usize, usize), (usize, usize))> {
        match self.state {
            SelectionState::Pressed { start, .. } => Some((start, start)),
            SelectionState::Dragging { start, current } => Some((start, current)),
            SelectionState::Complete { start, end } => Some((start, end)),
            SelectionState::Idle => None,
        }
    }

    pub fn get_normalized_bounds(&self) -> Option<((usize, usize), (usize, usize))> {
        let (start, end) = self.get_bounds()?;

        // Normalize rows
        let (min_row, max_row) = if start.0 <= end.0 {
            (start.0, end.0)
        } else {
            (end.0, start.0)
        };

        // Normalize columns
        let (min_col, max_col) = if start.0 == end.0 {
            // Same row - order by column
            if start.1 <= end.1 {
                (start.1, end.1)
            } else {
                (end.1, start.1)
            }
        } else {
            // Different rows - find actual min/max columns across all rows
            if start.1 <= end.1 {
                (start.1, end.1)
            } else {
                (end.1, start.1)
            }
        };

        Some(((min_row, min_col), (max_row, max_col)))
    }

    pub fn is_position_selected(&self, row: usize, col: usize) -> bool {
        let Some(((min_row, min_col), (max_row, max_col))) = self.get_normalized_bounds() else {
            return false;
        };

        if row < min_row || row > max_row {
            return false;
        }

        if row == min_row && row == max_row {
            // Single row selection
            col >= min_col && col <= max_col
        } else if row == min_row {
            // First row - from start column to end
            col >= min_col
        } else if row == max_row {
            // Last row - from start to end column
            col <= max_col
        } else {
            // Middle rows - entire row selected
            true
        }
    }

    // State machine transitions
    pub fn clear(&mut self) {
        self.state = SelectionState::Idle;
    }

    pub fn start(&mut self, row: usize, col: usize, timestamp: Instant) {
        self.state = SelectionState::Pressed { 
            start: (row, col), 
            timestamp 
        };
    }

    pub fn update(&mut self, row: usize, col: usize) {
        self.state = match self.state {
            SelectionState::Pressed { start, .. } | SelectionState::Dragging { start, .. } => {
                // If we start moving, transition to Dragging state
                SelectionState::Dragging { start, current: (row, col) }
            }
            other => other, // Ignore if not in a draggable state
        };
    }

    pub fn complete(&mut self, row: usize, col: usize, timestamp: Instant) -> bool {
        match self.state {
            SelectionState::Pressed { start, timestamp: press_time } => {
                // Quick click (less than CLICK_TIMEOUT_MS) - clear selection, don't create single-cell selection
                if timestamp.duration_since(press_time).as_millis() < CLICK_TIMEOUT_MS {
                    self.state = SelectionState::Idle;
                    false // No selection was created
                } else {
                    // Long press without movement - create single-cell selection
                    self.state = SelectionState::Complete { start, end: start };
                    true // Selection was created
                }
            }
            SelectionState::Dragging { start, .. } => {
                // Drag operation - complete with current position
                self.state = SelectionState::Complete { start, end: (row, col) };
                true // Selection was created
            }
            _ => false, // No state change
        }
    }

    // Query methods
    pub fn is_pressed(&self) -> bool {
        matches!(self.state, SelectionState::Pressed { .. })
    }

    pub fn is_dragging(&self) -> bool {
        matches!(self.state, SelectionState::Dragging { .. })
    }

    pub fn is_selecting(&self) -> bool {
        matches!(self.state, SelectionState::Pressed { .. } | SelectionState::Dragging { .. })
    }

    pub fn has_selection(&self) -> bool {
        matches!(self.state, SelectionState::Complete { .. })
    }

    /// Directly create a selection (bypassing the press/drag/click logic)
    /// Useful for programmatic selections like word/line selection
    pub fn create_selection(&mut self, start_row: usize, start_col: usize, end_row: usize, end_col: usize) {
        self.state = SelectionState::Complete {
            start: (start_row, start_col),
            end: (end_row, end_col),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_selection_creation() {
        let selection = Selection::new();
        assert_eq!(selection.state, SelectionState::Idle);
        assert!(!selection.is_active());
        assert!(selection.get_bounds().is_none());
        assert_eq!(selection.get_normalized_bounds(), None);
    }

    #[test]
    fn test_selection_state_transitions() {
        let mut selection = Selection::new();
        let timestamp = Instant::now();

        // Start selection
        selection.start(1, 2, timestamp);
        match selection.state {
            SelectionState::Pressed { start, .. } if start == (1, 2) => {},
            _ => panic!("Expected Pressed state with start position (1,2)"),
        }
        assert!(selection.is_active());
        assert!(selection.is_pressed());
        assert!(!selection.is_dragging());
        assert!(!selection.has_selection());

        // Update to dragging
        selection.update(3, 4);
        match selection.state {
            SelectionState::Dragging { start, current } if start == (1, 2) && current == (3, 4) => {},
            _ => panic!("Expected Dragging state with correct positions"),
        }
        assert!(selection.is_dragging());
        assert!(selection.is_selecting());

        // Complete selection
        let completed = selection.complete(5, 6, timestamp + Duration::from_millis(1000));
        assert!(completed);
        match selection.state {
            SelectionState::Complete { start, end } if start == (1, 2) && end == (5, 6) => {},
            _ => panic!("Expected Complete state with correct positions"),
        }
        assert!(selection.has_selection());
        assert!(!selection.is_selecting());
    }

    #[test]
    fn test_quick_click_clears_selection() {
        let mut selection = Selection::new();
        let timestamp = Instant::now();

        // Start selection
        selection.start(1, 2, timestamp);

        // Quick click should clear selection
        let completed = selection.complete(1, 2, timestamp + Duration::from_millis(50));
        assert!(!completed);
        assert_eq!(selection.state, SelectionState::Idle);
        assert!(!selection.has_selection());
    }

    #[test]
    fn test_long_press_creates_selection() {
        let mut selection = Selection::new();
        let timestamp = Instant::now();

        // Start selection
        selection.start(2, 3, timestamp);

        // Long press should create selection
        let completed = selection.complete(2, 3, timestamp + Duration::from_millis(300)); // Longer than CLICK_TIMEOUT_MS
        assert!(completed);
        match selection.state {
            SelectionState::Complete { start, end } if start == (2, 3) && end == (2, 3) => {},
            _ => panic!("Expected Complete state with single cell at (2,3)"),
        }
    }

    #[test]
    fn test_selection_bounds_calculation() {
        let mut selection = Selection::new();
        let timestamp = Instant::now();

        // Create a selection (bottom-right to top-left)
        selection.start(5, 7, timestamp);
        selection.update(2, 3);
        selection.complete(2, 3, timestamp + Duration::from_millis(1000));

        // Test get_bounds returns raw bounds
        let bounds = selection.get_bounds().unwrap();
        assert_eq!(bounds, ((5, 7), (2, 3))); // start, end as recorded

        // Test get_normalized_bounds normalizes properly
        let normalized = selection.get_normalized_bounds().unwrap();
        assert_eq!(normalized, ((2, 3), (5, 7))); // min_row, max_row, min_col, max_col
    }

    #[test]
    fn test_single_row_selection() {
        let mut selection = Selection::new();
        let timestamp = Instant::now();

        // Create single row selection
        selection.start(1, 2, timestamp);
        selection.update(1, 5);
        selection.complete(1, 5, timestamp + Duration::from_millis(1000));

        // Test normalized bounds
        let normalized = selection.get_normalized_bounds().unwrap();
        assert_eq!(normalized, ((1, 2), (1, 5))); // Same row, ordered columns

        // Test position selection
        assert!(selection.is_position_selected(1, 2)); // start
        assert!(selection.is_position_selected(1, 3)); // middle
        assert!(selection.is_position_selected(1, 5)); // end
        assert!(!selection.is_position_selected(1, 1)); // before start
        assert!(!selection.is_position_selected(1, 6)); // after end
        assert!(!selection.is_position_selected(0, 3)); // wrong row
    }

    #[test]
    fn test_multi_row_selection() {
        let mut selection = Selection::new();
        let timestamp = Instant::now();

        // Create multi-row selection
        selection.start(1, 3, timestamp);
        selection.update(4, 7);
        selection.complete(4, 7, timestamp + Duration::from_millis(1000));

        // Test normalized bounds
        let normalized = selection.get_normalized_bounds().unwrap();
        assert_eq!(normalized, ((1, 3), (4, 7))); // row 1-4, start col 3, end col 7

        // Test position selection
        // First row: from start col to end
        assert!(selection.is_position_selected(1, 3));
        assert!(selection.is_position_selected(1, 5));
        assert!(!selection.is_position_selected(1, 2)); // before start col on first row

        // Last row: from start to end col
        assert!(selection.is_position_selected(4, 7));
        assert!(selection.is_position_selected(4, 5));
        assert!(!selection.is_position_selected(4, 8)); // after end col on last row

        // Middle rows: entire row selected
        assert!(selection.is_position_selected(2, 0));
        assert!(selection.is_position_selected(2, 10));
        assert!(selection.is_position_selected(3, 0));
        assert!(selection.is_position_selected(3, 100));

        // Outside bounds
        assert!(!selection.is_position_selected(0, 3)); // before start row
        assert!(!selection.is_position_selected(5, 3)); // after end row
    }

    #[test]
    fn test_selection_clearing() {
        let mut selection = Selection::new();
        let timestamp = Instant::now();

        // Create selection
        selection.start(1, 2, timestamp);
        selection.update(3, 4);
        selection.complete(3, 4, timestamp + Duration::from_millis(1000));

        assert!(selection.has_selection());

        // Clear selection
        selection.clear();
        assert_eq!(selection.state, SelectionState::Idle);
        assert!(!selection.is_active());
        assert!(!selection.has_selection());
        assert!(selection.get_bounds().is_none());
    }

    #[test]
    fn test_reverse_direction_selection() {
        let mut selection = Selection::new();
        let timestamp = Instant::now();

        // Start from top-left, drag to bottom-right (normal)
        selection.start(0, 0, timestamp);
        selection.update(5, 8);
        selection.complete(5, 8, timestamp + Duration::from_millis(1000));

        let bounds = selection.get_bounds().unwrap();
        assert_eq!(bounds, ((0, 0), (5, 8)));

        selection.clear();

        // Start from bottom-right, drag to top-left (reverse)
        selection.start(5, 8, timestamp);
        selection.update(0, 0);
        selection.complete(0, 0, timestamp + Duration::from_millis(1000));

        let bounds = selection.get_bounds().unwrap();
        assert_eq!(bounds, ((5, 8), (0, 0))); // Note: raw bounds preserve direction

        let normalized = selection.get_normalized_bounds().unwrap();
        assert_eq!(normalized, ((0, 0), (5, 8))); // min_row, max_row, min_col, max_col
    }

    #[test]
    fn test_idle_state_ignores_updates() {
        let mut selection = Selection::new();

        // Should be safe to call update on idle state
        selection.update(1, 2);
        assert_eq!(selection.state, SelectionState::Idle);
        assert!(!selection.is_active());
    }

    #[test]
    fn test_complete_on_idle_returns_false() {
        let mut selection = Selection::new();

        // Complete on idle state should do nothing
        let completed = selection.complete(1, 2, Instant::now());
        assert!(!completed);
        assert_eq!(selection.state, SelectionState::Idle);
    }

    #[test]
    fn test_state_query_methods() {
        let mut selection = Selection::new();
        let timestamp = Instant::now();

        // Initially idle
        assert!(!selection.is_pressed());
        assert!(!selection.is_dragging());
        assert!(!selection.is_selecting());
        assert!(!selection.has_selection());

        // Pressed state
        selection.start(1, 2, timestamp);
        assert!(selection.is_pressed());
        assert!(!selection.is_dragging());
        assert!(selection.is_selecting());
        assert!(!selection.has_selection());

        // Dragging state
        selection.update(3, 4);
        assert!(!selection.is_pressed());
        assert!(selection.is_dragging());
        assert!(selection.is_selecting());
        assert!(!selection.has_selection());

        // Complete state
        selection.complete(3, 4, timestamp + Duration::from_millis(1000));
        assert!(!selection.is_pressed());
        assert!(!selection.is_dragging());
        assert!(!selection.is_selecting());
        assert!(selection.has_selection());
    }
}

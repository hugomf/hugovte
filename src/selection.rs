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
        
        let (min_row, max_row) = if start.0 <= end.0 {
            (start.0, end.0)
        } else {
            (end.0, start.0)
        };
        
        let (min_col, max_col) = if start.0 == end.0 {
            // Same row - order by column
            if start.1 <= end.1 {
                (start.1, end.1)
            } else {
                (end.1, start.1)
            }
        } else if start.0 < end.0 {
            // Different rows - start gets its column, end gets its column
            (start.1, end.1)
        } else {
            (end.1, start.1)
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
}
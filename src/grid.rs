// src/grid.rs
use crate::ansi::{AnsiGrid, Cell, Color};
use crate::selection::Selection;
use std::time::Instant;

/// Terminal grid - manages cell storage and cursor state
pub struct Grid {
    pub cols: usize,
    pub rows: usize,
    pub cells: Vec<Cell>, // Flat storage for better cache locality
    pub scrollback: Vec<Cell>, // Also flat storage
    pub scroll_offset: usize,
    pub col: usize,
    pub row: usize,
    pub fg: Color,
    pub bg: Color,
    bold: bool,
    italic: bool,
    underline: bool,
    dim: bool,
    // Selection state
    pub selection: Selection,
    // Cursor blink state
    cursor_visible: bool,
}

impl Grid {
    fn default_cell() -> Cell {
        Cell {
            ch: '\0',
            fg: crate::constants::DEFAULT_FG,
            bg: crate::constants::DEFAULT_BG,
            bold: false,
            italic: false,
            underline: false,
            dim: false,
        }
    }

    pub fn new(cols: usize, rows: usize) -> Self {
        let total_cells = cols * rows;
        let cells = vec![Self::default_cell(); total_cells];
        Self {
            cols,
            rows,
            cells,
            scrollback: Vec::new(),
            scroll_offset: 0,
            col: 0,
            row: 0,
            fg: crate::constants::DEFAULT_FG,
            bg: crate::constants::DEFAULT_BG,
            bold: false,
            italic: false,
            underline: false,
            dim: false,
            selection: Selection::new(),
            cursor_visible: true,
        }
    }

    // Flat array accessors
    pub fn get_cell(&self, row: usize, col: usize) -> &Cell {
        &self.cells[row * self.cols + col]
    }

    pub fn get_cell_mut(&mut self, row: usize, col: usize) -> &mut Cell {
        &mut self.cells[row * self.cols + col]
    }

    pub fn clear(&mut self) {
        self.cells.fill(Self::default_cell());
        self.col = 0;
        self.row = 0;
        self.scrollback.clear();
        self.scroll_offset = 0;
        self.selection.clear();
    }

    pub fn resize(&mut self, new_cols: usize, new_rows: usize) {
        let new_total = new_cols * new_rows;
        let mut new_cells = vec![Self::default_cell(); new_total];

        // Copy existing content
        for r in 0..self.rows.min(new_rows) {
            for c in 0..self.cols.min(new_cols) {
                let old_idx = r * self.cols + c;
                let new_idx = r * new_cols + c;
                new_cells[new_idx] = self.cells[old_idx];
            }
        }

        self.cells = new_cells;
        self.cols = new_cols;
        self.rows = new_rows;
        self.col = self.col.min(new_cols.saturating_sub(1));
        self.row = self.row.min(new_rows.saturating_sub(1));
        self.selection.clear();
    }

    // Selection delegation
    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    pub fn start_selection(&mut self, row: usize, col: usize) {
        self.selection.start(row, col, Instant::now());
    }

    pub fn update_selection(&mut self, row: usize, col: usize) {
        self.selection.update(row, col);
    }

    pub fn complete_selection(&mut self, row: usize, col: usize) -> bool {
        self.selection.complete(row, col, Instant::now())
    }

    pub fn toggle_cursor(&mut self) {
        self.cursor_visible = !self.cursor_visible;
    }

    pub fn is_cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    pub fn is_pressed(&self) -> bool {
        self.selection.is_pressed()
    }

    pub fn is_dragging(&self) -> bool {
        self.selection.is_dragging()
    }

    pub fn is_selecting(&self) -> bool {
        self.selection.is_selecting()
    }

    pub fn has_selection(&self) -> bool {
        self.selection.has_selection()
    }

    pub fn is_selected(&self, row: usize, col: usize) -> bool {
        self.selection.is_position_selected(row, col)
    }

    pub fn get_selected_text(&self) -> String {
        let Some(((start_row, start_col), (end_row, end_col))) = self.selection.get_normalized_bounds() else {
            return String::new();
        };

        let total_rows = self.scrollback.len() / self.cols + self.rows;
        
        if start_row >= total_rows || end_row >= total_rows {
            return String::new();
        }

        let mut result = String::new();

        for row in start_row..=end_row {
            let line = if row < self.scrollback.len() / self.cols {
                // Scrollback row
                let start_idx = row * self.cols;
                let end_idx = start_idx + self.cols;
                &self.scrollback[start_idx..end_idx]
            } else {
                // Grid row
                let grid_row = row - self.scrollback.len() / self.cols;
                if grid_row < self.rows {
                    let start_idx = grid_row * self.cols;
                    let end_idx = start_idx + self.cols;
                    &self.cells[start_idx..end_idx]
                } else {
                    continue;
                }
            };

            let start_c = if row == start_row { start_col.min(self.cols.saturating_sub(1)) } else { 0 };
            let end_c = if row == end_row { end_col.min(self.cols.saturating_sub(1)) } else { self.cols.saturating_sub(1) };

            for col in start_c..=end_c {
                let ch = line.get(col).map_or(' ', |cell| if cell.ch == '\0' { ' ' } else { cell.ch });
                result.push(ch);
            }

            if row < end_row {
                result.push('\n');
            }
        }

        result
    }
}

impl AnsiGrid for Grid {
    fn put(&mut self, ch: char) {
        if self.col < self.cols && self.row < self.rows {
            // Store attributes before borrowing self mutably
            let fg = self.fg;
            let bg = self.bg;
            let bold = self.bold;
            let italic = self.italic;
            let underline = self.underline;
            let dim = self.dim;
            
            let cell = self.get_cell_mut(self.row, self.col);
            *cell = Cell {
                ch,
                fg,
                bg,
                bold,
                italic,
                underline,
                dim,
            };
        }
    }

    fn advance(&mut self) {
        self.col += 1;
        if self.col >= self.cols {
            self.newline();
        }
    }

    fn left(&mut self, n: usize) {
        self.col = self.col.saturating_sub(n);
    }
    
    fn right(&mut self, n: usize) {
        self.col = (self.col + n).min(self.cols - 1);
    }
    
    fn up(&mut self, n: usize) {
        self.row = self.row.saturating_sub(n);
    }
    
    fn down(&mut self, n: usize) {
        self.row = (self.row + n).min(self.rows - 1);
    }

    fn newline(&mut self) {
        self.col = 0;
        self.row += 1;
        if self.row >= self.rows {
            // Move top row to scrollback
            let start_idx = 0;
            let end_idx = self.cols;
            let top_row: Vec<Cell> = self.cells[start_idx..end_idx].to_vec();
            self.scrollback.extend(top_row);
            
            // Scroll up
            self.cells.copy_within(self.cols.., 0);
            
            // Clear new bottom row
            let bottom_start = (self.rows - 1) * self.cols;
            for i in 0..self.cols {
                self.cells[bottom_start + i] = Self::default_cell();
            }
            
            self.row = self.rows - 1;
            
            // Limit scrollback
            if self.scrollback.len() > crate::constants::SCROLLBACK_LIMIT * self.cols {
                self.scrollback.drain(0..self.cols);
            }
        }
    }

    fn carriage_return(&mut self) {
        self.col = 0;
    }
    
    fn backspace(&mut self) {
        if self.col > 0 {
            self.col -= 1;
            // Clear the character at the new cursor position
            let cell = self.get_cell_mut(self.row, self.col);
            *cell = Self::default_cell();  // This erases the character!
        }
    }

    fn move_rel(&mut self, dx: i32, dy: i32) {
        let new_col = (self.col as i32 + dx).max(0) as usize;
        let new_row = (self.row as i32 + dy).max(0) as usize;
        self.col = new_col.min(self.cols - 1);
        self.row = new_row.min(self.rows - 1);
    }

    fn move_abs(&mut self, row: usize, col: usize) {
        self.col = col.min(self.cols.saturating_sub(1));
        self.row = row.min(self.rows.saturating_sub(1));
    }

    fn clear_screen(&mut self) {
        self.clear();
    }

    fn clear_line(&mut self) {
        let default = Self::default_cell();
        let start_idx = self.row * self.cols;
        for i in 0..self.cols {
            self.cells[start_idx + i] = default;
        }
    }

    fn reset_attrs(&mut self) {
        self.fg = crate::constants::DEFAULT_FG;
        self.bg = crate::constants::DEFAULT_BG;
        self.bold = false;
        self.italic = false;
        self.underline = false;
        self.dim = false;
    }

    fn set_bold(&mut self, bold: bool) {
        self.bold = bold;
    }
    
    fn set_italic(&mut self, italic: bool) {
        self.italic = italic;
    }
    
    fn set_underline(&mut self, underline: bool) {
        self.underline = underline;
    }
    
    fn set_dim(&mut self, dim: bool) {
        self.dim = dim;
    }
    
    fn set_fg(&mut self, color: Color) {
        self.fg = color;
    }
    
    fn set_bg(&mut self, color: Color) {
        self.bg = color;
    }

    fn get_fg(&self) -> Color {
        self.fg
    }
    
    fn get_bg(&self) -> Color {
        self.bg
    }
}
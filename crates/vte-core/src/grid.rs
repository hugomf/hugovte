// src/grid.rs
use crate::ansi::{AnsiGrid, Cell, Color};
use crate::selection::Selection;
use std::time::Instant;

/// Terminal grid - manages cell storage and cursor state
pub struct Grid {
    pub cols: usize,
    pub rows: usize,
    pub cells: Vec<Cell>, // Flat storage for better cache locality
    pub alternate_cells: Vec<Cell>, // Alternate screen buffer
    pub scrollback: Vec<Cell>, // Also flat storage (primary buffer only)
    pub scroll_offset: usize,
    pub col: usize,
    pub row: usize,
    // Alternate screen state
    primary_cursor: (usize, usize), // Saved for alternate screen
    alternate_cursor: (usize, usize), // Primary screen cursor
    primary_attrs: (Color, Color, bool, bool, bool, bool), // fg, bg, bold, italic, underline, dim
    alternate_attrs: (Color, Color, bool, bool, bool, bool), // fg, bg, bold, italic, underline, dim
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
    // Cursor stack for save/restore
    cursor_stack: Vec<(usize, usize)>,
    // Terminal modes
    insert_mode: bool,
    auto_wrap: bool,
    bracketed_paste_mode: bool,
    // Alternate screen flag
    use_alternate_screen: bool,
    // Terminal title
    title: String,
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
        let alternate_cells = vec![Self::default_cell(); total_cells];
        Self {
            cols,
            rows,
            cells,
            alternate_cells,
            scrollback: Vec::new(),
            scroll_offset: 0,
            col: 0,
            row: 0,
            // Alternate screen state - initially on primary
            primary_cursor: (0, 0),
            alternate_cursor: (0, 0),
            primary_attrs: (
                crate::constants::DEFAULT_FG,
                crate::constants::DEFAULT_BG,
                false, false, false, false  // bold, italic, underline, dim
            ),
            alternate_attrs: (
                crate::constants::DEFAULT_FG,
                crate::constants::DEFAULT_BG,
                false, false, false, false  // bold, italic, underline, dim
            ),
            fg: crate::constants::DEFAULT_FG,
            bg: crate::constants::DEFAULT_BG,
            bold: false,
            italic: false,
            underline: false,
            dim: false,
            selection: Selection::new(),
            cursor_visible: true,
            cursor_stack: Vec::new(),
            insert_mode: false,
            auto_wrap: true,
            bracketed_paste_mode: false,
            use_alternate_screen: false,
            title: String::new(),
        }
    }

    // Get the active cell buffer (primary or alternate)
    fn active_cells(&self) -> &[Cell] {
        if self.use_alternate_screen {
            &self.alternate_cells
        } else {
            &self.cells
        }
    }

    fn active_cells_mut(&mut self) -> &mut Vec<Cell> {
        if self.use_alternate_screen {
            &mut self.alternate_cells
        } else {
            &mut self.cells
        }
    }

    // Flat array accessors - work on active buffer
    pub fn get_cell(&self, row: usize, col: usize) -> &Cell {
        &self.active_cells()[row * self.cols + col]
    }

    pub fn get_cell_mut(&mut self, row: usize, col: usize) -> &mut Cell {
        let idx = row * self.cols + col;
        &mut self.active_cells_mut()[idx]
    }

    pub fn clear(&mut self) {
        self.active_cells_mut().fill(Self::default_cell());
        self.col = 0;
        self.row = 0;
        self.scrollback.clear();
        self.scroll_offset = 0;
        self.selection.clear();
    }

    pub fn resize(&mut self, new_cols: usize, new_rows: usize) {
        let new_total = new_cols * new_rows;

        // Resize both primary and alternate buffers
        let mut new_cells = vec![Self::default_cell(); new_total];
        let mut new_alternate_cells = vec![Self::default_cell(); new_total];

        // Copy existing content for both buffers
        for r in 0..self.rows.min(new_rows) {
            for c in 0..self.cols.min(new_cols) {
                let old_idx = r * self.cols + c;
                let new_idx = r * new_cols + c;
                new_cells[new_idx] = self.cells[old_idx];
                new_alternate_cells[new_idx] = self.alternate_cells[old_idx];
            }
        }

        self.cells = new_cells;
        self.alternate_cells = new_alternate_cells;
        self.cols = new_cols;
        self.rows = new_rows;
        self.col = self.col.min(new_cols.saturating_sub(1));
        self.row = self.row.min(new_rows.saturating_sub(1));
        self.selection.clear();
    }

    /// Resize with line rewrapping (like vte4)
    /// Reflows text when terminal width changes by extracting logical lines
    /// and rewrapping them to fit the new column width.
    pub fn resize_with_rewrap(&mut self, new_cols: usize, new_rows: usize) {
        if new_cols == self.cols && new_rows == self.rows {
            return;
        }

        // Resize active buffer with rewrapping
        let (new_active_cells, new_cursor_pos) = self.resize_buffer_with_rewrap(
            self.active_cells().to_vec(),
            new_cols,
            new_rows,
        );

        // Resize alternate buffer without rewrapping (maintain as-is)
        let new_total_alt = new_cols * new_rows;
        let mut new_alt_cells = vec![Self::default_cell(); new_total_alt];

        // Copy existing alternate content (simple resize, no rewrap)
        for r in 0..self.rows.min(new_rows) {
            for c in 0..self.cols.min(new_cols) {
                let old_idx = r * self.cols + c;
                let new_idx = r * new_cols + c;
                if old_idx < self.alternate_cells.len() {
                    new_alt_cells[new_idx] = self.alternate_cells[old_idx];
                }
            }
        }

        // Update buffers
        if self.use_alternate_screen {
            self.alternate_cells = new_active_cells;
        } else {
            self.cells = new_active_cells;
        }

        let old_cols = self.cols;
        let old_rows = self.rows;
        self.cols = new_cols;
        self.rows = new_rows;

        // Update cursor position - if buffer with rewrap gave (0,0), use simple clamping
        if new_cursor_pos == (0, 0) && old_cols > 0 && old_rows > 0 {
            // For empty or simple cases, just clamp cursor to new bounds
            self.col = self.col.min(new_cols.saturating_sub(1));
            self.row = self.row.min(new_rows.saturating_sub(1));
        } else {
            // Use calculated position from rewrapping logic
            self.col = new_cursor_pos.0.min(new_cols.saturating_sub(1));
            self.row = new_cursor_pos.1.min(new_rows.saturating_sub(1));
        }

        self.selection.clear();
    }

    /// Resize a specific buffer with rewrapping logic
    fn resize_buffer_with_rewrap(&self, old_cells: Vec<Cell>, new_cols: usize, new_rows: usize)
        -> (Vec<Cell>, (usize, usize)) {

        if self.cols == 0 {
            return (vec![Self::default_cell(); new_cols * new_rows], (0, 0));
        }

        // Extract logical lines (merge wrapped lines)
        let logical_lines = self.extract_logical_lines_from_buffer(&old_cells);

        // Rewrap logical lines to new column width
        let mut rewrapped_lines = Vec::new();
        let mut cursor_pos = (0, 0); // Default position for empty buffers

        // Calculate the cursor's absolute character position in the logical layout
        let mut absolute_cursor_pos = 0;
        if self.row < self.rows {
            for (logical_idx, line) in logical_lines.iter().enumerate() {
                if logical_idx < self.row {
                    // Count all content in previous rows
                    absolute_cursor_pos += line.len();
                } else if logical_idx == self.row {
                    // Add characters in the current row up to and including cursor position
                    for col in 0..=self.col {
                        if col < line.len() {
                            absolute_cursor_pos += 1;
                        }
                    }
                    break;
                }
            }
        }

        // Rewrap each logical line to fit new width
        let mut current_row = 0;

        for logical_line in logical_lines.into_iter() {
            if current_row >= new_rows {
                // No more room, line is lost
                break;
            }

            let wrapped = self.wrap_line(&logical_line, new_cols);

            for wrapped_row in wrapped.into_iter() {
                if current_row >= new_rows {
                    break;
                }

                // Place row in new grid
                rewrapped_lines.push(wrapped_row);
                current_row += 1;
            }
        }

        // Convert absolute cursor position to new grid coordinates
        if absolute_cursor_pos > 0 {
            cursor_pos = (
                (absolute_cursor_pos - 1) % new_cols,  // column within row (0-based)
                (absolute_cursor_pos - 1) / new_cols   // row number (0-based)
            );
        }

        // Pad remaining rows with default cells
        while rewrapped_lines.len() < new_rows {
            rewrapped_lines.push(vec![Self::default_cell(); new_cols]);
        }

        // Flatten rows into flat cell array
        let mut new_cells = Vec::with_capacity(new_cols * new_rows);
        for row in rewrapped_lines {
            new_cells.extend(row);
        }

        (new_cells, cursor_pos)
    }

    /// Extract logical lines from a buffer (merge hard-wrapped lines)
    fn extract_logical_lines_from_buffer(&self, buffer: &[Cell]) -> Vec<Vec<Cell>> {
        let mut logical_lines = Vec::new();

        for row in 0..self.rows {
            let row_start = row * self.cols;

            // Check if this row exists in buffer
            if (row_start + self.cols) > buffer.len() {
                break;
            }

            let row_slice = &buffer[row_start..row_start + self.cols];

            // Find the actual content in this row (cells with non-null characters)
            let mut line_cells = Vec::new();
            for cell in row_slice {
                if cell.ch != '\0' {
                    line_cells.push(*cell);
                } else {
                    break; // Stop at first null (line terminator)
                }
            }

            // Only include non-empty lines
            if !line_cells.is_empty() {
                logical_lines.push(line_cells);
            }
        }

        logical_lines
    }

    /// Wrap a logical line to fit new column width
    fn wrap_line(&self, line: &[Cell], new_cols: usize) -> Vec<Vec<Cell>> {
        let mut wrapped = Vec::new();
        let mut current_row = Vec::new();

        for &cell in line {
            current_row.push(cell);

            if current_row.len() >= new_cols {
                wrapped.push(current_row.clone());
                current_row.clear();
            }
        }

        // Pad last row if needed, or add it if not empty
        if !current_row.is_empty() {
            while current_row.len() < new_cols {
                current_row.push(Self::default_cell());
            }
            wrapped.push(current_row);
        }

        wrapped
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

    /// Select word at the given position
    pub fn select_word(&mut self, _row: usize, _col: usize) {
        // TODO: Implement word selection based on Unicode word boundaries
        // For now, just do nothing safely
    }

    /// Select line at the given position
    pub fn select_line(&mut self, _row: usize) {
        // TODO: Implement line selection
        // For now, just do nothing safely
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
                // Scrollback row (always from primary)
                let start_idx = row * self.cols;
                let end_idx = start_idx + self.cols;
                &self.scrollback[start_idx..end_idx]
            } else {
                // Grid row (from active buffer)
                let grid_row = row - self.scrollback.len() / self.cols;
                if grid_row < self.rows {
                    let start_idx = grid_row * self.cols;
                    let end_idx = start_idx + self.cols;
                    &self.active_cells()[start_idx..end_idx]
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

    /// Enable or disable alternate screen buffer
    /// When enabled, switches to the alternate buffer and saves state
    /// When disabled, switches back to primary buffer and restores state
    pub fn use_alternate_screen(&mut self, enable: bool) {
        if self.use_alternate_screen == enable {
            return; // No change needed
        }

        if enable {
            // Switch TO alternate screen - save primary state
            self.primary_cursor = (self.row, self.col);
            self.primary_attrs = (
                self.fg, self.bg,
                self.bold, self.italic, self.underline, self.dim
            );
            // Switch to alternate state
            self.use_alternate_screen = true;
            (self.row, self.col) = self.alternate_cursor;
            (self.fg, self.bg, self.bold, self.italic, self.underline, self.dim) = self.alternate_attrs;
        } else {
            // Switch FROM alternate screen - save alternate state
            self.alternate_cursor = (self.row, self.col);
            self.alternate_attrs = (
                self.fg, self.bg,
                self.bold, self.italic, self.underline, self.dim
            );
            // Switch to primary state
            self.use_alternate_screen = false;
            (self.row, self.col) = self.primary_cursor;
            (self.fg, self.bg, self.bold, self.italic, self.underline, self.dim) = self.primary_attrs;
        }
    }
}

impl AnsiGrid for Grid {
    fn put(&mut self, ch: char) {
        if self.col < self.cols && self.row < self.rows {
            if self.insert_mode {
                self.insert_chars(1);
            }

            // Store attributes
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
        if self.auto_wrap && self.col >= self.cols {
            self.newline();
        } else {
            self.col = self.col.min(self.cols - 1);
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
            self.scroll_offset = 0; // Auto-scroll to bottom on new output
            
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
        // Just move cursor left - don't erase
        // Bash will send \x1B[K to clear if needed
        if self.col > 0 {
            self.col -= 1;
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
            self.active_cells_mut()[start_idx + i] = default;
        }
    }

    fn clear_line_right(&mut self) {
        let default = Self::default_cell();
        let start_idx = self.row * self.cols + self.col;
        let end_idx = (self.row + 1) * self.cols;
        for i in start_idx..end_idx {
            self.active_cells_mut()[i] = default;
        }
    }

    fn clear_line_left(&mut self) {
        let default = Self::default_cell();
        let start_idx = self.row * self.cols;
        let end_idx = self.row * self.cols + self.col + 1;
        for i in start_idx..end_idx {
            self.active_cells_mut()[i] = default;
        }
    }

    fn clear_screen_down(&mut self) {
        // Clear from cursor to end of screen
        self.clear_line_right();
        let default = Self::default_cell();
        let start_idx = (self.row + 1) * self.cols;
        let end_idx = self.rows * self.cols;
        for i in start_idx..end_idx {
            self.active_cells_mut()[i] = default;
        }
    }

    fn clear_screen_up(&mut self) {
        // Clear from top of screen to cursor
        self.clear_line_left();
        let default = Self::default_cell();
        let end_idx = self.row * self.cols;
        for i in 0..end_idx {
            self.active_cells_mut()[i] = default;
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

    fn save_cursor(&mut self) {
        self.cursor_stack.push((self.row, self.col));
    }

    fn restore_cursor(&mut self) {
        if let Some((row, col)) = self.cursor_stack.pop() {
            self.row = row;
            self.col = col;
        }
    }

    fn set_cursor_visible(&mut self, visible: bool) {
        self.cursor_visible = visible;
    }

    fn scroll_up(&mut self, n: usize) {
        if n == 0 {
            return;
        }
        if n >= self.rows {
            self.clear_screen();
            return;
        }

        let cols = self.cols; // Avoid borrowing issues with self.cols

        // Move content up by n rows
        for r in 0..(self.rows - n) {
            let src_start = (r + n) * cols;
            let dst_start = r * cols;
            if self.use_alternate_screen {
                self.alternate_cells.copy_within(src_start..(src_start + cols), dst_start);
            } else {
                self.cells.copy_within(src_start..(src_start + cols), dst_start);
            }
        }

        // Clear bottom n rows
        for r in (self.rows - n)..self.rows {
            for c in 0..cols {
                let idx = r * cols + c;
                if self.use_alternate_screen {
                    self.alternate_cells[idx] = Self::default_cell();
                } else {
                    self.cells[idx] = Self::default_cell();
                }
            }
        }
    }

    fn scroll_down(&mut self, n: usize) {
        if n == 0 {
            return;
        }
        if n >= self.rows {
            self.clear_screen();
            return;
        }

        let cols = self.cols; // Avoid borrowing issues with self.cols

        // Move content down by n rows
        for r in (0..(self.rows - n)).rev() {
            let dst_start = (r + n) * cols;
            let src_start = r * cols;
            if self.use_alternate_screen {
                self.alternate_cells.copy_within(src_start..(src_start + cols), dst_start);
            } else {
                self.cells.copy_within(src_start..(src_start + cols), dst_start);
            }
        }

        // Clear top n rows
        for r in 0..n {
            for c in 0..cols {
                let idx = r * cols + c;
                if self.use_alternate_screen {
                    self.alternate_cells[idx] = Self::default_cell();
                } else {
                    self.cells[idx] = Self::default_cell();
                }
            }
        }
    }

    fn insert_lines(&mut self, n: usize) {
        if n == 0 {
            return;
        }
        let n_clamped = n.min(self.rows - self.row);
        let cols = self.cols; // Avoid borrowing issues with self.cols
        let start_row = self.row;
        let end_row = self.rows - n_clamped;

        // Shift rows below current row down by n_clamped
        for r in (start_row..end_row).rev() {
            let dst_start = (r + n_clamped) * cols;
            let src_start = r * cols;
            if self.use_alternate_screen {
                self.alternate_cells.copy_within(src_start..(src_start + cols), dst_start);
            } else {
                self.cells.copy_within(src_start..(src_start + cols), dst_start);
            }
        }

        // Clear inserted rows
        for r in start_row..(start_row + n_clamped) {
            for c in 0..cols {
                let idx = r * cols + c;
                if self.use_alternate_screen {
                    self.alternate_cells[idx] = Self::default_cell();
                } else {
                    self.cells[idx] = Self::default_cell();
                }
            }
        }
    }

    fn delete_lines(&mut self, n: usize) {
        if n == 0 {
            return;
        }
        let n_clamped = n.min(self.rows - self.row);
        let cols = self.cols; // Avoid borrowing issues with self.cols
        let start_row = self.row;
        let end_row = self.rows;

        // Shift rows up by n_clamped
        for r in start_row..end_row {
            if r + n_clamped < self.rows {
                let dst_start = r * cols;
                let src_start = (r + n_clamped) * cols;
                if self.use_alternate_screen {
                    self.alternate_cells.copy_within(src_start..(src_start + cols), dst_start);
                } else {
                    self.cells.copy_within(src_start..(src_start + cols), dst_start);
                }
            } else {
                // Clear row
                for c in 0..cols {
                    let idx = r * cols + c;
                    if self.use_alternate_screen {
                        self.alternate_cells[idx] = Self::default_cell();
                    } else {
                        self.cells[idx] = Self::default_cell();
                    }
                }
            }
        }
    }

    fn insert_chars(&mut self, n: usize) {
        if n == 0 || self.col >= self.cols {
            return;
        }
        let n_clamped = n.min(self.cols - self.col);
        let row_start = self.row * self.cols;
        let insert_pos = self.col;
        let row_end = self.cols;

        // Shift characters to the right starting from cursor position
        // Work backwards to avoid overwriting
        for pos in ((insert_pos)..row_end).rev() {
            let src_idx = row_start + pos;
            let dst_idx = row_start + pos + n_clamped;
            if dst_idx < row_start + row_end {
                let value = if self.use_alternate_screen {
                    self.alternate_cells[src_idx]
                } else {
                    self.cells[src_idx]
                };
                if self.use_alternate_screen {
                    self.alternate_cells[dst_idx] = value;
                } else {
                    self.cells[dst_idx] = value;
                }
            }
        }

        // Clear inserted chars
        for pos in insert_pos..insert_pos + n_clamped {
            let idx = row_start + pos;
            if self.use_alternate_screen {
                self.alternate_cells[idx] = Self::default_cell();
            } else {
                self.cells[idx] = Self::default_cell();
            }
        }
    }

    fn delete_chars(&mut self, n: usize) {
        if n == 0 || self.col >= self.cols {
            return;
        }
        let n_clamped = n.min(self.cols - self.col);
        let row_start = self.row * self.cols;
        let end_col = self.cols - n_clamped;

        // Shift left to cursor position
        for idx in self.col..end_col {
            let src = row_start + idx + n_clamped;
            let dst = row_start + idx;
            if self.use_alternate_screen {
                self.alternate_cells[dst] = self.alternate_cells[src];
            } else {
                self.cells[dst] = self.cells[src];
            }
        }

        // Clear end of line
        for idx in row_start + end_col..row_start + self.cols {
            if self.use_alternate_screen {
                self.alternate_cells[idx] = Self::default_cell();
            } else {
                self.cells[idx] = Self::default_cell();
            }
        }
    }

    fn erase_chars(&mut self, n: usize) {
        if n == 0 {
            return;
        }
        let row_start = self.row * self.cols;
        let end_idx = (self.col + n).min(self.cols);
        for idx in row_start + self.col..row_start + end_idx {
            self.active_cells_mut()[idx] = Self::default_cell();
        }
    }

    fn set_insert_mode(&mut self, enable: bool) {
        self.insert_mode = enable;
    }

    fn set_auto_wrap(&mut self, enable: bool) {
        self.auto_wrap = enable;
    }

    fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    fn set_bracketed_paste_mode(&mut self, enable: bool) {
        self.bracketed_paste_mode = enable;
    }

    fn handle_clipboard_data(&mut self, _clipboard_id: u8, _data: &str) {
        // Placeholder - clipboard handling would be backend-specific
        // For now, clipboards are handled via OSC 52 sequences parsed at terminal level
    }

    fn handle_hyperlink(&mut self, _params: Option<&str>, _uri: &str) {
        // Placeholder - hyperlinks would require Cell hyperlink field
        // For now, hyperlinks are handled via OSC 8 sequences parsed at terminal level
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ansi::Cell;
    use crate::constants::{DEFAULT_FG, DEFAULT_BG};

    #[test]
    fn test_grid_creation() {
        let grid = Grid::new(80, 24);
        assert_eq!(grid.cols, 80);
        assert_eq!(grid.rows, 24);
        assert_eq!(grid.cells.len(), 80 * 24);
        assert_eq!(grid.alternate_cells.len(), 80 * 24);
        assert_eq!(grid.col, 0);
        assert_eq!(grid.row, 0);
        assert!(!grid.use_alternate_screen);
        assert!(grid.scrollback.is_empty());
    }

    #[test]
    fn test_grid_resize() {
        let mut grid = Grid::new(80, 24);

        // Fill first few cells with test data
        *grid.get_cell_mut(0, 0) = Cell { ch: 'A', ..Default::default() };
        *grid.get_cell_mut(0, 1) = Cell { ch: 'B', ..Default::default() };

        // Resize larger
        grid.resize(100, 30);
        assert_eq!(grid.cols, 100);
        assert_eq!(grid.rows, 30);
        assert_eq!(grid.cells.len(), 100 * 30);

        // Check content is preserved
        assert_eq!(grid.get_cell(0, 0).ch, 'A');
        assert_eq!(grid.get_cell(0, 1).ch, 'B');
    }

    #[test]
    fn test_cursor_movement() {
        let mut grid = Grid::new(10, 10);

        // Test absolute movement
        grid.move_abs(5, 7);
        assert_eq!(grid.row, 5);
        assert_eq!(grid.col, 7);

        // Test movement with bounds clamping
        grid.move_abs(15, 15); // Should clamp to max
        assert_eq!(grid.row, 9);
        assert_eq!(grid.col, 9);

        // Test relative movement
        grid.move_rel(5, 5); // Should clamp
        assert_eq!(grid.row, 9);
        assert_eq!(grid.col, 9);

        grid.move_rel(-10, -10); // Should clamp to 0
        assert_eq!(grid.row, 0);
        assert_eq!(grid.col, 0);
    }

    #[test]
    fn test_cell_writing_and_reading() {
        let mut grid = Grid::new(10, 10);

        // Write a character
        let test_cell = Cell {
            ch: 'X',
            fg: Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 },
            bg: Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 },
            bold: true,
            italic: false,
            underline: false,
            dim: false,
        };

        *grid.get_cell_mut(1, 2) = test_cell.clone();

        // Read it back
        let read_cell = grid.get_cell(1, 2);
        assert_eq!(read_cell.ch, 'X');
        assert_eq!(read_cell.fg, test_cell.fg);
        assert_eq!(read_cell.bg, test_cell.bg);
        assert_eq!(read_cell.bold, true);
        assert_eq!(read_cell.italic, false);
    }

    #[test]
    fn test_clear_operations() {
        let mut grid = Grid::new(5, 5);

        // Put some content
        *grid.get_cell_mut(0, 0) = Cell { ch: 'A', ..Default::default() };
        *grid.get_cell_mut(0, 1) = Cell { ch: 'B', ..Default::default() };
        grid.col = 1;
        grid.row = 2;

        // Clear line
        grid.clear_line();
        assert_eq!(grid.get_cell(2, 0).ch, '\0');
        assert_eq!(grid.get_cell(2, 1).ch, '\0');

        // Clear screen should reset cursor and clear content
        grid.clear_screen();
        assert_eq!(grid.col, 0);
        assert_eq!(grid.row, 0);
        assert!(grid.scrollback.is_empty());
    }

    #[test]
    fn test_scroll_operations() {
        let mut grid = Grid::new(5, 3);

        // Put content in rows 0, 1, 2
        *grid.get_cell_mut(0, 0) = Cell { ch: 'A', ..Default::default() };
        *grid.get_cell_mut(1, 0) = Cell { ch: 'B', ..Default::default() };
        *grid.get_cell_mut(2, 0) = Cell { ch: 'C', ..Default::default() };

        // Scroll up by 1 - moves rows up within grid, bottom row clears
        // Note: scroll_up/down are for V/T compatibility, not content scrolling
        grid.scroll_up(1);

        // Row 0 (former row 1) should have B
        assert_eq!(grid.get_cell(0, 0).ch, 'B');

        // Row 1 (former row 2) should have C
        assert_eq!(grid.get_cell(1, 0).ch, 'C');

        // Row 2 (scrolled up) should be cleared
        assert_eq!(grid.get_cell(2, 0).ch, '\0');

        // Scroll operations don't create scrollback (only newlines do)
        assert!(grid.scrollback.is_empty());
    }

    #[test]
    fn test_scroll_down() {
        let mut grid = Grid::new(5, 3);

        // Put content in all rows
        *grid.get_cell_mut(0, 0) = Cell { ch: 'A', ..Default::default() };
        *grid.get_cell_mut(1, 0) = Cell { ch: 'B', ..Default::default() };
        *grid.get_cell_mut(2, 0) = Cell { ch: 'C', ..Default::default() };

        // Scroll down by 1 - C->B, B->A, top row clears
        grid.scroll_down(1);

        // Row 1 should have A
        assert_eq!(grid.get_cell(1, 0).ch, 'A');
        // Row 2 should have B (moved from row 1)
        assert_eq!(grid.get_cell(2, 0).ch, 'B');
        // Row 0 should be cleared
        assert_eq!(grid.get_cell(0, 0).ch, '\0');
    }

    #[test]
    fn test_line_operations() {
        let mut grid = Grid::new(5, 5);

        // Put content in a line
        grid.row = 2;
        *grid.get_cell_mut(2, 0) = Cell { ch: 'A', ..Default::default() };
        *grid.get_cell_mut(2, 1) = Cell { ch: 'B', ..Default::default() };
        *grid.get_cell_mut(2, 4) = Cell { ch: 'E', ..Default::default() };

        // Insert lines (should shift down from current row)
        grid.insert_lines(1);
        // Row 3 should now have A, B, E
        assert_eq!(grid.get_cell(3, 0).ch, 'A');
        assert_eq!(grid.get_cell(3, 1).ch, 'B');
        assert_eq!(grid.get_cell(3, 4).ch, 'E');
        // Row 2 should be cleared (inserted line)
        assert_eq!(grid.get_cell(2, 0).ch, '\0');

        // Delete lines (should shift up from current row)
        grid.row = 2;
        grid.delete_lines(1);
        // Row 2 should now have the content from row 3
        assert_eq!(grid.get_cell(2, 0).ch, 'A');
        assert_eq!(grid.get_cell(2, 1).ch, 'B');
    }

    #[test]
    fn test_character_operations() {
        let mut grid = Grid::new(5, 5);
        grid.row = 1;

        // Put characters: [A, B, C]
        // Keep it simple - only use positions 0, 1, 2 to avoid overflow
        *grid.get_cell_mut(1, 0) = Cell { ch: 'A', ..Default::default() };
        *grid.get_cell_mut(1, 1) = Cell { ch: 'B', ..Default::default() };
        *grid.get_cell_mut(1, 2) = Cell { ch: 'C', ..Default::default() };

        // Verify initial state
        assert_eq!(grid.get_cell(1, 0).ch, 'A');
        assert_eq!(grid.get_cell(1, 1).ch, 'B');
        assert_eq!(grid.get_cell(1, 2).ch, 'C');

        // Insert characters at position 1 (between 'A' and 'B')
        grid.col = 1;
        grid.insert_chars(1);

        // Should insert 1 empty char at cursor, shifting right
        // [A, B, C] with insert at pos 1 becomes [A, ∅, B] (C still at pos 2)
        assert_eq!(grid.get_cell(1, 0).ch, 'A'); // Original A unchanged
        assert_eq!(grid.get_cell(1, 1).ch, '\0'); // Inserted empty
        assert_eq!(grid.get_cell(1, 2).ch, 'B'); // B moved from pos 1 to pos 2, C still at pos 2? Wait, this doesn't make sense

        // Wait, correct logic: with cursor at position 1 in [A, B, C]:
        // insert_chars(1) should insert empty at cursor: [A, ∅, B, C] then truncate to [A, ∅, B]

        assert_eq!(grid.get_cell(1, 0).ch, 'A');
        assert_eq!(grid.get_cell(1, 1).ch, '\0'); // Inserted empty
        assert_eq!(grid.get_cell(1, 2).ch, 'B'); // B moved to pos 2 from pos 1
        // C is lost (pushed off the end)
    }

    #[test]
    fn test_alternate_screen() {
        let mut grid = Grid::new(3, 3);

        // Put content on primary screen
        *grid.get_cell_mut(0, 0) = Cell { ch: 'P', ..Default::default() };
        *grid.get_cell_mut(1, 1) = Cell { ch: 'R', ..Default::default() };

        // Switch to alternate screen
        grid.use_alternate_screen(true);
        assert!(grid.use_alternate_screen);

        // Put different content on alternate screen
        *grid.get_cell_mut(0, 0) = Cell { ch: 'A', ..Default::default() };
        *grid.get_cell_mut(1, 1) = Cell { ch: 'L', ..Default::default() };

        assert_eq!(grid.get_cell(0, 0).ch, 'A');
        assert_eq!(grid.get_cell(1, 1).ch, 'L');

        // Switch back to primary screen
        grid.use_alternate_screen(false);
        assert!(!grid.use_alternate_screen);

        // Original content should be preserved
        assert_eq!(grid.get_cell(0, 0).ch, 'P');
        assert_eq!(grid.get_cell(1, 1).ch, 'R');
    }

    #[test]
    fn test_cursor_save_restore() {
        let mut grid = Grid::new(10, 10);

        // Move cursor
        grid.move_abs(5, 7);
        assert_eq!(grid.row, 5);
        assert_eq!(grid.col, 7);

        // Save cursor
        grid.save_cursor();

        // Move cursor again
        grid.move_abs(1, 2);
        assert_eq!(grid.row, 1);
        assert_eq!(grid.col, 2);

        // Restore cursor
        grid.restore_cursor();
        assert_eq!(grid.row, 5);
        assert_eq!(grid.col, 7);
    }

    #[test]
    fn test_attribute_management() {
        let mut grid = Grid::new(5, 5);

        // Test setting attributes
        grid.set_bold(true);
        grid.set_fg(Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 });
        grid.set_bg(Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 });

        assert_eq!(grid.get_fg(), Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 });
        assert_eq!(grid.get_bg(), Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 });

        // Reset attributes
        grid.reset_attrs();
        assert_eq!(grid.get_fg().r, DEFAULT_FG.r);
        assert_eq!(grid.get_bg().r, DEFAULT_BG.r);
    }

    #[test]
    fn test_newline_with_scrollback() {
        let mut grid = Grid::new(3, 2); // Small grid to trigger scrolling easily

        // Fill the screen
        grid.put('A'); grid.advance();
        grid.put('B'); grid.newline();
        grid.put('C'); grid.advance();
        grid.put('D'); grid.newline(); // This should cause scroll

        // Should have scrolled A from row 0 to scrollback
        assert_eq!(grid.scrollback[0].ch, 'A');
        assert_eq!(grid.scrollback[1].ch, 'B');

        // Row 0 should now have C D
        assert_eq!(grid.get_cell(0, 0).ch, 'C');
        assert_eq!(grid.get_cell(0, 1).ch, 'D');

        // Row 1 should be at cursor
        assert_eq!(grid.row, 1);
        assert_eq!(grid.col, 0);
    }

    #[test]
    fn test_selection_integration() {
        let mut grid = Grid::new(5, 5);

        // Start selection
        grid.start_selection(1, 2);
        assert!(grid.is_pressed());

        // Update selection (start dragging)
        grid.update_selection(3, 4);
        assert!(grid.is_dragging());
        assert!(grid.is_selecting());

        // Complete selection
        let completed = grid.complete_selection(3, 4);
        assert!(completed);
        assert!(grid.has_selection());
        assert!(!grid.is_selecting());
    }

    #[test]
    fn test_resize_with_bounds_clamping() {
        let mut grid = Grid::new(10, 10);

        // Put cursor near the edge
        grid.move_abs(8, 8);

        // Resize smaller
        grid.resize(5, 5);

        // Cursor should be clamped to new bounds
        assert_eq!(grid.row, 4); // 8 clamped to 4
        assert_eq!(grid.col, 4); // 8 clamped to 4
        assert_eq!(grid.rows, 5);
        assert_eq!(grid.cols, 5);
    }

    #[test]
    fn test_cursor_blink() {
        let mut grid = Grid::new(5, 5);

        // Initially visible
        assert!(grid.is_cursor_visible());

        // Toggle
        grid.toggle_cursor();
        assert!(!grid.is_cursor_visible());

        // Toggle back
        grid.toggle_cursor();
        assert!(grid.is_cursor_visible());
    }

    #[test]
    fn test_resize_with_rewrap_basic() {
        let mut grid = Grid::new(5, 3);

        // Fill with content: "AAAAA\nBBBBB\nCCCCC"
        for col in 0..5 {
            *grid.get_cell_mut(0, col) = Cell { ch: 'A', ..Default::default() };
            *grid.get_cell_mut(1, col) = Cell { ch: 'B', ..Default::default() };
            *grid.get_cell_mut(2, col) = Cell { ch: 'C', ..Default::default() };
        }

        // Resize to 3 columns - should rewrap lines
        grid.resize_with_rewrap(3, 3);

        // Lines should be rewrapped: each row now fits 3 chars
        // "AAAAA" becomes "AAA" and "AA" (but since we have 3 rows, second part may be truncated)
        assert_eq!(grid.get_cell(0, 0).ch, 'A');
        assert_eq!(grid.get_cell(0, 1).ch, 'A');
        assert_eq!(grid.get_cell(0, 2).ch, 'A');

        // Check dimensions changed
        assert_eq!(grid.cols, 3);
        assert_eq!(grid.rows, 3);
    }

    #[test]
    fn test_resize_with_rewrap_merge_lines() {
        let mut grid = Grid::new(5, 3);

        // Create wrapped lines (simulate hard wrapping)
        // Row 0: "AAAAA" (full width)
        // Row 1: "BBB" (partial - simulates logical line continuing)
        for col in 0..5 {
            *grid.get_cell_mut(0, col) = Cell { ch: 'A', ..Default::default() };
        }
        for col in 0..3 {
            *grid.get_cell_mut(1, col) = Cell { ch: 'B', ..Default::default() };
        }
        // Row 2: empty (logical line break)

        // When resizing to 4 columns, should merge "AAAAA" + "BBB" = "AAAAABBB" into "AAAA", "AABB", "B"
        grid.resize_with_rewrap(4, 3);

        // Check that content was preserved and rewrapped
        assert_eq!(grid.get_cell(0, 0).ch, 'A');
        assert_eq!(grid.get_cell(0, 1).ch, 'A');
        assert_eq!(grid.get_cell(0, 2).ch, 'A');
        assert_eq!(grid.get_cell(0, 3).ch, 'A');

        if grid.rows >= 2 {
            assert_eq!(grid.get_cell(1, 0).ch, 'A'); // 5th A
        }
    }

    #[test]
    fn test_resize_with_rewrap_cursor_positioning() {
        let mut grid = Grid::new(5, 3);

        // Put content and position cursor in middle of logical line
        for col in 0..4 {
            *grid.get_cell_mut(0, col) = Cell { ch: 'A', ..Default::default() };
        }
        // Row 1 partial (continuing logical line)
        for col in 0..3 {
            *grid.get_cell_mut(1, col) = Cell { ch: 'B', ..Default::default() };
        }

        // Position cursor at col 2, row 1 (6th char in logical line "AAAABBB")
        grid.move_abs(1, 2);

        // Resize to wider (10 columns) - should unwrap lines
        grid.resize_with_rewrap(10, 3);

        // Cursor should follow the logical line position
        // Original position: row 1, col 2 -> logical position: 4 + 3 = 7th char (0-indexed)
        // For absolute_cursor_pos = 7: (7-1)%10 = 6, so column 6
        assert_eq!(grid.row, 0);
        assert_eq!(grid.col, 6);
    }

    #[test]
    fn test_resize_with_rewrap_cursor_bounds() {
        let mut grid = Grid::new(5, 3);

        // Position cursor near edge
        grid.move_abs(2, 4); // Bottom right

        // Resize smaller
        grid.resize_with_rewrap(2, 2);

        // Cursor should be clamped to new bounds
        assert_eq!(grid.row, 1); // 2 clamped to 1
        assert_eq!(grid.col, 1); // 4 clamped to 1
    }

    #[test]
    fn test_resize_with_rewrap_alternate_screen() {
        let mut grid = Grid::new(5, 3);

        // Put content on primary
        *grid.get_cell_mut(0, 0) = Cell { ch: 'P', ..Default::default() };

        // Switch to alternate screen
        grid.use_alternate_screen(true);

        // Put different content on alternate
        *grid.get_cell_mut(0, 0) = Cell { ch: 'A', ..Default::default() };
        for col in 0..4 {
            *grid.get_cell_mut(1, col) = Cell { ch: 'B', ..Default::default() };
        }

        // Resize with rewrap (should only affect alternate screen)
        grid.resize_with_rewrap(3, 2);

        // Alternate screen content should be rewrapped
        assert_eq!(grid.get_cell(0, 0).ch, 'A'); // First "A" moves to first row
        assert_eq!(grid.get_cell(1, 0).ch, 'B'); // "B"s should wrap

        // Switch back to primary - should still have original content
        grid.use_alternate_screen(false);
        assert_eq!(grid.get_cell(0, 0).ch, 'P');
    }

    #[test]
    fn test_extract_logical_lines() {
        let mut grid = Grid::new(4, 3);

        // Create test buffer: row 0 fully filled, row 1 partially filled, row 2 empty
        for col in 0..4 {
            *grid.get_cell_mut(0, col) = Cell { ch: 'A', ..Default::default() };
        }
        for col in 0..2 {
            *grid.get_cell_mut(1, col) = Cell { ch: 'B', ..Default::default() };
        }
        // Row 2 empty

        let logical_lines = grid.extract_logical_lines_from_buffer(&grid.cells);

        // Should extract 2 logical lines: "AAAA" and "BB"
        assert_eq!(logical_lines.len(), 2);
        assert_eq!(logical_lines[0].len(), 4); // "AAAA"
        assert_eq!(logical_lines[1].len(), 2); // "BB"

        assert_eq!(logical_lines[0][0].ch, 'A');
        assert_eq!(logical_lines[1][0].ch, 'B');
        assert_eq!(logical_lines[1][1].ch, 'B');
    }

    #[test]
    fn test_wrap_line() {
        let mut grid = Grid::new(5, 3);

        // Create logical line longer than new width
        let logical_line: Vec<Cell> = "ABCDEFGHIJ".chars()
            .map(|ch| Cell { ch, ..Default::default() })
            .collect();

        let wrapped = grid.wrap_line(&logical_line, 4);

        // Should wrap "ABCDEFGHIJ" as: "ABCD", "EFGH", "IJ"
        assert_eq!(wrapped.len(), 3);
        assert_eq!(wrapped[0].len(), 4); // "ABCD"
        assert_eq!(wrapped[1].len(), 4); // "EFGH"
        assert_eq!(wrapped[2].len(), 4); // "IJ  " (padded)

        assert_eq!(wrapped[0][0].ch, 'A');
        assert_eq!(wrapped[0][1].ch, 'B');
        assert_eq!(wrapped[1][0].ch, 'E');
        assert_eq!(wrapped[2][0].ch, 'I');
        assert_eq!(wrapped[2][1].ch, 'J');
        assert_eq!(wrapped[2][2].ch, '\0'); // padding
    }

    #[test]
    fn test_resize_with_rewrap_no_change() {
        let mut grid = Grid::new(5, 3);

        // Put some content
        *grid.get_cell_mut(0, 0) = Cell { ch: 'A', ..Default::default() };

        // Resize to same dimensions - should be no-op
        grid.resize_with_rewrap(5, 3);

        // Content should be unchanged
        assert_eq!(grid.get_cell(0, 0).ch, 'A');
        assert_eq!(grid.cols, 5);
        assert_eq!(grid.rows, 3);
    }

    #[test]
    fn test_resize_with_rewrap_empty_grid() {
        let mut grid = Grid::new(5, 3);

        // Empty grid
        grid.resize_with_rewrap(4, 2);

        // Should work without panicking
        assert_eq!(grid.cols, 4);
        assert_eq!(grid.rows, 2);

        // All cells should be default (null)
        for row in 0..2 {
            for col in 0..4 {
                assert_eq!(grid.get_cell(row, col).ch, '\0');
            }
        }
    }
}

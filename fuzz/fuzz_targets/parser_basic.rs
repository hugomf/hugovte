#![no_main]
use libfuzzer_sys::fuzz_target;

// Adjust crate name as needed - if your crate is named "hugo_term" use that
// For standalone parsing module, use appropriate name
use hugovte::ansi::{AnsiParser, AnsiGrid, Cell, Color};

#[derive(Default)]
struct FuzzGrid {
    cells: Vec<Cell>,
    cols: usize,
    rows: usize,
    row: usize,
    col: usize,
    fg: Color,
    bg: Color,
    bold: bool,
    italic: bool,
    underline: bool,
    dim: bool,
    cursor_stack: Vec<(usize, usize)>,
    output_len: usize,
}

impl FuzzGrid {
    fn new() -> Self {
        Self {
            cells: vec![Cell::default(); 80 * 24],
            cols: 80,
            rows: 24,
            row: 0,
            col: 0,
            fg: Color::default(),
            bg: Color::rgb(0., 0., 0.),
            bold: false,
            italic: false,
            underline: false,
            dim: false,
            cursor_stack: Vec::new(),
            output_len: 0,
        }
    }
    
    fn check_invariants(&self) {
        assert!(self.row < self.rows, "row {} >= rows {}", self.row, self.rows);
        assert!(self.col < self.cols, "col {} >= cols {}", self.col, self.cols);
        assert!(self.cursor_stack.len() < 1000, "cursor stack overflow");
    }
}

impl AnsiGrid for FuzzGrid {
    fn put(&mut self, ch: char) {
        if self.row < self.rows && self.col < self.cols {
            let idx = self.row * self.cols + self.col;
            if idx < self.cells.len() {
                self.cells[idx] = Cell {
                    ch,
                    fg: self.fg,
                    bg: self.bg,
                    bold: self.bold,
                    italic: self.italic,
                    underline: self.underline,
                    dim: self.dim,
                };
            }
        }
        self.output_len = self.output_len.saturating_add(1);
    }
    
    fn advance(&mut self) { 
        self.col = self.col.saturating_add(1);
        if self.col >= self.cols {
            self.col = self.cols.saturating_sub(1);
        }
    }
    
    fn left(&mut self, n: usize) { 
        self.col = self.col.saturating_sub(n); 
    }
    
    fn right(&mut self, n: usize) { 
        self.col = self.col.saturating_add(n);
        if self.col >= self.cols {
            self.col = self.cols.saturating_sub(1);
        }
    }
    
    fn up(&mut self, n: usize) { 
        self.row = self.row.saturating_sub(n); 
    }
    
    fn down(&mut self, n: usize) { 
        self.row = self.row.saturating_add(n);
        if self.row >= self.rows {
            self.row = self.rows.saturating_sub(1);
        }
    }
    
    fn newline(&mut self) { 
        self.row = self.row.saturating_add(1);
        if self.row >= self.rows {
            self.row = self.rows.saturating_sub(1);
        }
        self.col = 0;
    }
    
    fn carriage_return(&mut self) { 
        self.col = 0; 
    }
    
    fn backspace(&mut self) { 
        self.col = self.col.saturating_sub(1); 
    }
    
    fn move_rel(&mut self, dx: i32, dy: i32) {
        let new_col = (self.col as i32).saturating_add(dx).max(0) as usize;
        let new_row = (self.row as i32).saturating_add(dy).max(0) as usize;
        self.col = new_col.min(self.cols.saturating_sub(1));
        self.row = new_row.min(self.rows.saturating_sub(1));
    }
    
    fn move_abs(&mut self, row: usize, col: usize) {
        self.row = row.min(self.rows.saturating_sub(1));
        self.col = col.min(self.cols.saturating_sub(1));
    }
    
    fn clear_screen(&mut self) {
        for cell in &mut self.cells {
            *cell = Cell::default();
        }
        self.row = 0;
        self.col = 0;
    }
    
    fn clear_line(&mut self) {
        if self.row < self.rows {
            let start = self.row * self.cols;
            let end = start + self.cols;
            if end <= self.cells.len() {
                for cell in &mut self.cells[start..end] {
                    *cell = Cell::default();
                }
            }
        }
    }
    
    fn clear_line_right(&mut self) {
        if self.row < self.rows {
            let start = self.row * self.cols + self.col;
            let end = (self.row + 1) * self.cols;
            if start < self.cells.len() && end <= self.cells.len() {
                for cell in &mut self.cells[start..end] {
                    *cell = Cell::default();
                }
            }
        }
    }
    
    fn clear_line_left(&mut self) {
        if self.row < self.rows && self.col < self.cols {
            let start = self.row * self.cols;
            let end = (self.row * self.cols + self.col + 1).min(self.cells.len());
            for cell in &mut self.cells[start..end] {
                *cell = Cell::default();
            }
        }
    }
    
    fn clear_screen_down(&mut self) {
        self.clear_line_right();
        if self.row + 1 < self.rows {
            let start = (self.row + 1) * self.cols;
            let end = self.cells.len();
            for cell in &mut self.cells[start..end] {
                *cell = Cell::default();
            }
        }
    }
    
    fn clear_screen_up(&mut self) {
        self.clear_line_left();
        if self.row > 0 {
            let end = self.row * self.cols;
            for cell in &mut self.cells[0..end] {
                *cell = Cell::default();
            }
        }
    }
    
    fn reset_attrs(&mut self) {
        self.fg = Color::default();
        self.bg = Color::rgb(0., 0., 0.);
        self.bold = false;
        self.italic = false;
        self.underline = false;
        self.dim = false;
    }
    
    fn set_bold(&mut self, v: bool) { self.bold = v; }
    fn set_italic(&mut self, v: bool) { self.italic = v; }
    fn set_underline(&mut self, v: bool) { self.underline = v; }
    fn set_dim(&mut self, v: bool) { self.dim = v; }
    fn set_fg(&mut self, c: Color) { self.fg = c; }
    fn set_bg(&mut self, c: Color) { self.bg = c; }
    fn get_fg(&self) -> Color { self.fg }
    fn get_bg(&self) -> Color { self.bg }
    
    fn save_cursor(&mut self) {
        // Prevent unbounded stack growth
        if self.cursor_stack.len() < 100 {
            self.cursor_stack.push((self.row, self.col));
        }
    }
    
    fn restore_cursor(&mut self) {
        if let Some((row, col)) = self.cursor_stack.pop() {
            self.row = row.min(self.rows.saturating_sub(1));
            self.col = col.min(self.cols.saturating_sub(1));
        }
    }
    
    fn set_cursor_visible(&mut self, _visible: bool) {}
    fn scroll_up(&mut self, _n: usize) {}
    fn scroll_down(&mut self, _n: usize) {}
    fn insert_lines(&mut self, _n: usize) {}
    fn delete_lines(&mut self, _n: usize) {}
    fn insert_chars(&mut self, _n: usize) {}
    fn delete_chars(&mut self, _n: usize) {}
    fn erase_chars(&mut self, _n: usize) {}
    fn use_alternate_screen(&mut self, _enable: bool) {}
    fn set_insert_mode(&mut self, _enable: bool) {}
    fn set_auto_wrap(&mut self, _enable: bool) {}
}

fuzz_target!(|data: &[u8]| {
    // Skip empty input
    if data.is_empty() {
        return;
    }
    
    // Limit input size to prevent timeouts
    let data = if data.len() > 10000 {
        &data[..10000]
    } else {
        data
    };
    
    let mut parser = AnsiParser::new();
    let mut grid = FuzzGrid::new();
    
    // Track errors for debugging
    let error_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let error_count_clone = error_count.clone();
    
    parser = parser.with_error_callback(move |_err| {
        error_count_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    });
    
    // Feed the fuzzer-generated data
    //parser.feed_bytes(data, &mut grid);
    for &byte in data {
        parser.process(byte, &mut grid);
    }
    
    
    // Verify grid integrity after parsing
    grid.check_invariants();
    
    // Stats should be reasonable
    let stats = parser.stats();
    assert!(stats.max_params_seen <= 32, "max_params_seen too large");
    assert!(stats.max_osc_length_seen <= 2048, "osc length too large");
});
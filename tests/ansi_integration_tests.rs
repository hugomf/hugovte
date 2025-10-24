// tests/integration_tests.rs
//! Integration tests for realistic terminal scenarios

use hugovte::{AnsiParser, AnsiGrid, Cell, Color};

/// Mock grid for integration testing
#[derive(Default)]
struct TestGrid {
    output: String,
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
}

impl TestGrid {
    fn new(cols: usize, rows: usize) -> Self {
        Self {
            output: String::new(),
            cells: vec![Cell::default(); cols * rows],
            cols,
            rows,
            row: 0,
            col: 0,
            fg: Color::default(),
            bg: Color::rgb(0., 0., 0.),
            bold: false,
            italic: false,
            underline: false,
            dim: false,
        }
    }
    
    fn get_cell(&self, row: usize, col: usize) -> &Cell {
        &self.cells[row * self.cols + col]
    }
}

impl AnsiGrid for TestGrid {
    fn put(&mut self, ch: char) {
        if self.row < self.rows && self.col < self.cols {
            let idx = self.row * self.cols + self.col;
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
        self.output.push(ch);
    }
    
    fn advance(&mut self) { self.col += 1; }
    fn left(&mut self, n: usize) { self.col = self.col.saturating_sub(n); }
    fn right(&mut self, n: usize) { self.col += n; }
    fn up(&mut self, n: usize) { self.row = self.row.saturating_sub(n); }
    fn down(&mut self, n: usize) { self.row += n; }
    fn newline(&mut self) { self.output.push('\n'); self.row += 1; self.col = 0; }
    fn carriage_return(&mut self) { self.col = 0; }
    fn backspace(&mut self) { self.col = self.col.saturating_sub(1); }
    fn move_rel(&mut self, dx: i32, dy: i32) {
        self.col = (self.col as i32 + dx).max(0) as usize;
        self.row = (self.row as i32 + dy).max(0) as usize;
    }
    fn move_abs(&mut self, row: usize, col: usize) {
        self.row = row;
        self.col = col;
    }
    fn clear_screen(&mut self) {
        self.cells.fill(Cell::default());
        self.output.push_str("[CLEAR]");
    }
    fn clear_line(&mut self) { self.output.push_str("[CLEAR_LINE]"); }
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
}

#[test]
fn test_ls_color_output() {
    // Simulate 'ls --color' output
    let mut parser = AnsiParser::new();
    let mut grid = TestGrid::new(80, 24);
    
    let ls_output = "\x1B[0m\x1B[01;34mDocuments\x1B[0m  \x1B[01;32mscript.sh\x1B[0m  \x1B[01;31marchive.zip\x1B[0m\n";
    parser.feed_str(ls_output, &mut grid);
    
    // Check output text
    assert!(grid.output.contains("Documents"));
    assert!(grid.output.contains("script.sh"));
    assert!(grid.output.contains("archive.zip"));
    
    // Verify colors were applied (Documents should be blue/bold)
    assert!(grid.get_cell(0, 0).bold);
}

#[test]
fn test_vim_like_application() {
    // Simulate vim-like application with alternate screen
    let mut parser = AnsiParser::new();
    let mut grid = TestGrid::new(80, 24);
    
    // Enter alternate screen, clear, position cursor, draw content
    let vim_seq = concat!(
        "\x1B[?1049h",           // Enter alt screen
        "\x1B[2J",               // Clear
        "\x1B[H",                // Home cursor
        "\x1B[7m",               // Reverse video (we don't support but won't crash)
        "~ VIM MODE ~",
        "\x1B[0m",               // Reset
        "\x1B[?1049l",           // Exit alt screen
    );
    
    parser.feed_str(vim_seq, &mut grid);
    
    // Should contain the text
    assert!(grid.output.contains("VIM MODE"));
}

#[test]
fn test_progress_bar() {
    // Simulate a progress bar with cursor movement
    let mut parser = AnsiParser::new();
    let mut grid = TestGrid::new(80, 24);
    
    for i in 0..=10 {
        let progress = format!(
            "\rDownloading: [{}{}] {}%",
            "=".repeat(i),
            " ".repeat(10 - i),
            i * 10
        );
        parser.feed_str(&progress, &mut grid);
    }
    
    assert!(grid.output.contains("100%"));
    assert!(grid.output.contains("=========="));
}

#[test]
fn test_multiline_colored_output() {
    // Test output with multiple lines and colors
    let mut parser = AnsiParser::new();
    let mut grid = TestGrid::new(80, 24);
    
    let output = concat!(
        "\x1B[31mError:\x1B[0m Something went wrong\n",
        "\x1B[33mWarning:\x1B[0m Check this\n",
        "\x1B[32mSuccess:\x1B[0m All good\n",
    );
    
    parser.feed_str(output, &mut grid);
    
    assert!(grid.output.contains("Error:"));
    assert!(grid.output.contains("Warning:"));
    assert!(grid.output.contains("Success:"));
    assert_eq!(grid.output.matches('\n').count(), 3);
}

#[test]
fn test_table_drawing_with_box_chars() {
    // Test UTF-8 box drawing characters
    let mut parser = AnsiParser::new();
    let mut grid = TestGrid::new(80, 24);
    
    let table = concat!(
        "┌─────────┬─────────┐\n",
        "│ Header1 │ Header2 │\n",
        "├─────────┼─────────┤\n",
        "│ Data1   │ Data2   │\n",
        "└─────────┴─────────┘\n",
    );
    
    parser.feed_str(table, &mut grid);
    
    assert!(grid.output.contains("┌"));
    assert!(grid.output.contains("│"));
    assert!(grid.output.contains("Data1"));
}

#[test]
fn test_emoji_and_unicode() {
    // Test emoji and international characters
    let mut parser = AnsiParser::new();
    let mut grid = TestGrid::new(80, 24);
    
    let text = "Hello 世界 🌍 Привет 🎉\n";
    parser.feed_str(text, &mut grid);
    
    assert!(grid.output.contains("世界"));
    assert!(grid.output.contains("🌍"));
    assert!(grid.output.contains("Привет"));
    assert!(grid.output.contains("🎉"));
}

#[test]
fn test_htop_like_output() {
    // Simulate htop with colors, positioning, and updates
    let mut parser = AnsiParser::new();
    let mut grid = TestGrid::new(80, 24);
    
    let htop = concat!(
        "\x1B[H\x1B[2J",                    // Clear and home
        "\x1B[1;33m  CPU\x1B[0m [",         // Bold yellow header
        "\x1B[32m||||",                     // Green bars
        "\x1B[0m     ] 40.0%\n",
        "\x1B[1;33m  Mem\x1B[0m [",
        "\x1B[31m||||||||",                 // Red bars
        "\x1B[0m ] 80.0%\n",
    );
    
    parser.feed_str(htop, &mut grid);
    
    assert!(grid.output.contains("CPU"));
    assert!(grid.output.contains("Mem"));
    assert!(grid.output.contains("40.0%"));
    assert!(grid.output.contains("80.0%"));
}

#[test]
fn test_cursor_save_restore_complex() {
    // Test complex cursor save/restore scenario
    let mut parser = AnsiParser::new();
    let mut grid = TestGrid::new(80, 24);
    
    parser.feed_str("Line 1", &mut grid);
    parser.feed_str("\x1B[s", &mut grid);     // Save position
    parser.feed_str("\n\n\nLine 4", &mut grid);
    parser.feed_str("\x1B[u", &mut grid);     // Restore
    parser.feed_str(" (restored)", &mut grid);
    
    assert!(grid.output.contains("Line 1"));
    assert!(grid.output.contains("Line 4"));
    assert!(grid.output.contains("restored"));
}

#[test]
fn test_large_output_streaming() {
    // Test handling large output (simulating cat of a big file)
    let mut parser = AnsiParser::new();
    let mut grid = TestGrid::new(80, 24);
    
    // Generate 1000 lines of output
    for i in 0..1000 {
        let line = format!("Line {}: {}\n", i, "x".repeat(70));
        parser.feed_str(&line, &mut grid);
    }
    
    // Should not panic and should contain last line
    assert!(grid.output.contains("Line 999:"));
}

#[test]
fn test_mixed_sgr_and_cursor() {
    // Test interleaved SGR and cursor movements
    let mut parser = AnsiParser::new();
    let mut grid = TestGrid::new(80, 24);
    
    parser.feed_str("\x1B[31mRed", &mut grid);
    parser.feed_str("\x1B[5D", &mut grid);          // Move back 5
    parser.feed_str("\x1B[32mGreen", &mut grid);
    parser.feed_str("\x1B[10C", &mut grid);         // Move forward 10
    parser.feed_str("\x1B[1;34mBold Blue", &mut grid);
    
    assert!(grid.output.contains("Red"));
    assert!(grid.output.contains("Green"));
    assert!(grid.output.contains("Bold Blue"));
}

#[test]
fn test_osc_title_sequences() {
    // Test OSC title setting
    let mut parser = AnsiParser::new();
    let mut grid = TestGrid::new(80, 24);
    
    // OSC with BEL terminator
    parser.feed_str("\x1B]0;Window Title\x07", &mut grid);
    
    // OSC with ST terminator
    parser.feed_str("\x1B]0;Another Title\x1B\\", &mut grid);
    
    // Should not crash
}

#[test]
fn test_malformed_sequences_resilience() {
    // Test that malformed sequences don't crash
    let mut parser = AnsiParser::new();
    let mut grid = TestGrid::new(80, 24);
    
    let malformed = vec![
        "Normal text",              // Normal text
        "\x1B[9999999999m",         // Huge param
        "\x1B[;;;;;;;;;;;m",        // Many empty params
        "\x1B[38;5;999m",           // Invalid 256-color index
        "\x1B[38;2;999;999;999m",   // Invalid RGB values
    ];
    
    for seq in malformed {
        parser.feed_str(seq, &mut grid);
    }
    
    // Should not panic
    parser.feed_str("Still working!\n", &mut grid);
    assert!(grid.output.contains("Still working!"));
}

#[test]
fn test_zero_width_and_control_chars() {
    // Test handling of various control characters
    let mut parser = AnsiParser::new();
    let mut grid = TestGrid::new(80, 24);
    
    // Mix of printable and control chars
    parser.feed_str("Before\tTab\rReturn\nNewline\x08Backspace", &mut grid);
    
    // Should handle tabs, returns, newlines, backspaces
    assert!(grid.output.contains("Before"));
    assert!(grid.output.contains("Tab"));
}

#[test]
fn test_performance_many_sgr_changes() {
    // Test performance with many SGR changes
    let mut parser = AnsiParser::new();
    let mut grid = TestGrid::new(80, 24);
    
    for i in 0..1000 {
        let color = 30 + (i % 8);
        parser.feed_str(&format!("\x1B[{}mX", color), &mut grid);
    }
    
    // Should complete without significant slowdown
    assert_eq!(grid.output.chars().filter(|&c| c == 'X').count(), 1000);
}

#[test]
fn test_realistic_shell_session() {
    // Simulate a realistic shell session
    let mut parser = AnsiParser::new();
    let mut grid = TestGrid::new(80, 24);
    
    let session = concat!(
        "\x1B[32muser@host\x1B[0m:\x1B[34m~/projects\x1B[0m$ ",
        "ls -la\n",
        "total 48\n",
        "drwxr-xr-x  5 user user 4096 Jan 1 12:00 \x1B[34m.\x1B[0m\n",
        "drwxr-xr-x 10 user user 4096 Jan 1 11:00 \x1B[34m..\x1B[0m\n",
        "-rw-r--r--  1 user user  220 Jan 1 10:00 \x1B[0mREADME.md\x1B[0m\n",
        "\x1B[32muser@host\x1B[0m:\x1B[34m~/projects\x1B[0m$ ",
    );
    
    parser.feed_str(session, &mut grid);
    
    assert!(grid.output.contains("user@host"));
    assert!(grid.output.contains("~/projects"));
    assert!(grid.output.contains("README.md"));
    assert!(grid.output.contains("total 48"));
}

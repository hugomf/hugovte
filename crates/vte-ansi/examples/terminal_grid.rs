//! Basic terminal grid implementation
//!
//! This example shows how to implement a basic terminal emulator grid
//! that tracks text content, cursor position, and style attributes.

use vte_ansi::{AnsiParser, AnsiGrid, Color};

// Simple terminal cell
#[derive(Clone, Default, Debug)]
struct TerminalCell {
    ch: char,
    fg: Color,
    bg: Color,
    bold: bool,
    italic: bool,
    underline: bool,
}

// Basic terminal grid implementation
struct TerminalGrid {
    width: usize,
    height: usize,
    cursor_x: usize,
    cursor_y: usize,
    cells: Vec<Vec<TerminalCell>>,
    current_fg: Color,
    current_bg: Color,
    bold: bool,
    italic: bool,
    underline: bool,
}

impl TerminalGrid {
    fn new(width: usize, height: usize) -> Self {
        let mut cells = Vec::with_capacity(height);
        for _ in 0..height {
            let mut row = Vec::with_capacity(width);
            for _ in 0..width {
                row.push(TerminalCell::default());
            }
            cells.push(row);
        }

        Self {
            width,
            height,
            cursor_x: 0,
            cursor_y: 0,
            cells,
            current_fg: Color::default(),
            current_bg: Color::rgb(0., 0., 0.),
            bold: false,
            italic: false,
            underline: false,
        }
    }

    fn display(&self) {
        println!("Terminal Grid (showing text content):");
        println!("Cursor at ({}, {})", self.cursor_x, self.cursor_y);
        println!();

        for (y, row) in self.cells.iter().enumerate() {
            print!("{:2}: ", y);
            for cell in row {
                print!("{}", cell.ch);
            }
            if y < 5 { println!(); } // Show first 5 rows
        }
        if self.height > 5 {
            println!("... ({} more rows)", self.height - 5);
        }
    }
}

impl AnsiGrid for TerminalGrid {
    fn put(&mut self, ch: char) {
        // Only place printable chars within bounds
        if self.cursor_y < self.height && self.cursor_x < self.width {
            let cell = &mut self.cells[self.cursor_y][self.cursor_x];
            cell.ch = ch;
            cell.fg = self.current_fg;
            cell.bg = self.current_bg;
            cell.bold = self.bold;
            cell.italic = self.italic;
            cell.underline = self.underline;
        }
        self.cursor_x += 1;
    }

    fn advance(&mut self) {
        self.cursor_x += 1;

        // Auto-wrap to next line if needed
        if self.cursor_x >= self.width {
            self.cursor_x = 0;
            self.cursor_y += 1;
        }
    }

    fn newline(&mut self) {
        self.cursor_y += 1;
        self.cursor_x = 0;

        // Scroll if at bottom
        if self.cursor_y >= self.height {
            self.cursor_y = self.height - 1;
            // In a real terminal, we'd scroll up here
        }
    }

    fn carriage_return(&mut self) {
        self.cursor_x = 0;
    }

    // Style attributes
    fn set_bold(&mut self, bold: bool) { self.bold = bold; }
    fn set_italic(&mut self, italic: bool) { self.italic = italic; }
    fn set_underline(&mut self, underline: bool) { self.underline = underline; }
    fn set_dim(&mut self, _dim: bool) { /* dim not implemented in basic demo */ }
    fn set_fg(&mut self, color: Color) { self.current_fg = color; }
    fn set_bg(&mut self, color: Color) { self.current_bg = color; }

    fn reset_attrs(&mut self) {
        self.current_fg = Color::default();
        self.current_bg = Color::rgb(0., 0., 0.);
        self.bold = false;
        self.italic = false;
        self.underline = false;
    }

    // Cursor movement
    fn left(&mut self, n: usize) {
        self.cursor_x = self.cursor_x.saturating_sub(n);
    }

    fn right(&mut self, n: usize) {
        self.cursor_x += n;
    }

    fn up(&mut self, n: usize) {
        self.cursor_y = self.cursor_y.saturating_sub(n);
    }

    fn down(&mut self, n: usize) {
        self.cursor_y += n;
    }

    fn move_abs(&mut self, row: usize, col: usize) {
        self.cursor_y = row.min(self.height.saturating_sub(1));
        self.cursor_x = col.min(self.width.saturating_sub(1));
    }

    fn move_rel(&mut self, dx: i32, dy: i32) {
        self.cursor_x = ((self.cursor_x as i32 + dx).max(0) as usize).min(self.width.saturating_sub(1));
        self.cursor_y = ((self.cursor_y as i32 + dy).max(0) as usize).min(self.height.saturating_sub(1));
    }

    fn get_fg(&self) -> Color { self.current_fg }
    fn get_bg(&self) -> Color { self.current_bg }

    // Empty implementations for other methods (not needed for basic demo)
    fn clear_screen(&mut self) {}
    fn clear_line(&mut self) {}
    fn backspace(&mut self) {}
    fn save_cursor(&mut self) {}
    fn restore_cursor(&mut self) {}
    fn set_cursor_visible(&mut self, _visible: bool) {}
}

fn main() {
    println!("Creating a simple 10x5 terminal grid...\n");

    let mut grid = TerminalGrid::new(10, 5);
    let mut parser = AnsiParser::new();

    // Parse some sample text with ANSI sequences
    let sample_text = "\x1b[31mRed\x1b[0m text\n\x1b[32mGreen line\x1b[0m\nNormal\n\x1b[1;33mBold Yellow\x1b[0m";

    parser.feed_str(sample_text, &mut grid);

    // Display the result
    grid.display();

    println!("\nTerminal state after parsing:");
    println!("Final cursor: ({}, {})", grid.cursor_x, grid.cursor_y);
    println!("Current style - Bold: {}, FG: {}, BG: {}", grid.bold, grid.current_fg, grid.current_bg);
    println!("\nThis demonstrates basic text placement and cursor movement!");
}

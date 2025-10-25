// examples/sgr_demo.rs
// Demonstrates all SGR color and text formatting capabilities
// Improvements:
// - Programmatic generation of color swatches using unicode blocks (█) for better visibility.
// - Separate sections for FG/BG demos with contrasting colors to avoid invisibility on black HTML bg.
// - Added demos for SGR 39/49 (defaults) and combined dim with colors.
// - Enhanced HTML: Added <pre> wrapper, better spacing, section headers, and CSS for monospace/grid-like feel.
// - Expanded programmatic tests: Include dim, defaults, and full attr dumps.
// - More tests: Coverage for swatches, defaults, and incomplete sequences (no panic).
// - CLI arg: Optional input file for parsing real ANSI output (e.g., `ls --color`).

use std::env;
use std::fs;
use hugovte::ansi::{AnsiParser, AnsiGrid, Color, Cell};

struct SimpleGrid {
    cells: Vec<Cell>,
    cursor: usize,
    current_cell: Cell,
    width: usize,  // For wrapping in render
}

impl SimpleGrid {
    fn new(width: usize) -> Self {
        Self {
            cells: Vec::new(),
            cursor: 0,
            current_cell: Cell::default(),
            width,
        }
    }

    fn render_to_html(&self) -> String {
        let mut html = String::from(
            r#"<html><head><title>ANSI SGR Demo</title>
            <style>
                body { background: #000; color: #fff; padding: 20px; font-family: 'Courier New', monospace; }
                pre { background: #000; padding: 15px; border: 1px solid #333; line-height: 1.2; white-space: pre; }
                .section { margin-bottom: 20px; }
                .swatch { display: inline-block; width: 2em; text-align: center; margin: 1px; }
            </style></head><body>
            <h1>ANSI SGR Demo (hugovte)</h1>
            <p>Open <code>ansi_demo.html</code> in a browser to view rendered output.</p>
            <div class="section">"#
        );
        
        let mut line = String::new();
        for cell in &self.cells {
            let swatch_class = if cell.ch == '█' { "swatch" } else { "" };
            let fg_style = format!(
                "color: rgba({}, {}, {}, {});",
                (cell.fg.r * 255.0) as u8,
                (cell.fg.g * 255.0) as u8,
                (cell.fg.b * 255.0) as u8,
                cell.fg.a
            );
            
            let bg_style = format!(
                "background-color: rgba({}, {}, {}, {});",
                (cell.bg.r * 255.0) as u8,
                (cell.bg.g * 255.0) as u8,
                (cell.bg.b * 255.0) as u8,
                cell.bg.a
            );
            
            let mut text_style = String::new();
            if cell.bold { text_style.push_str("font-weight: bold;"); }
            if cell.italic { text_style.push_str("font-style: italic;"); }
            if cell.underline { text_style.push_str("text-decoration: underline;"); }
            if cell.dim { text_style.push_str("opacity: 0.6;"); }
            
            let span = format!(
                "<span class=\"{}\" style=\"{}{}{}\">{}</span>",
                swatch_class, fg_style, bg_style, text_style,
                if cell.ch == '\n' { "<br>" } else { &cell.ch.to_string() }
            );
            
            line.push_str(&span);
            if cell.ch == '\n' || line.len() > 80 {  // Rough wrap
                html.push_str(&format!("<pre>{}</pre>", line));
                line.clear();
            }
        }
        if !line.is_empty() {
            html.push_str(&format!("<pre>{}</pre>", line));
        }
        
        html.push_str("</div></body></html>");
        html
    }
}

impl AnsiGrid for SimpleGrid {
    fn put(&mut self, ch: char) {
        self.current_cell.ch = ch;
        if self.cursor >= self.cells.len() {
            self.cells.push(self.current_cell);
        } else {
            self.cells[self.cursor] = self.current_cell;
        }
    }

    fn advance(&mut self) {
        self.cursor += 1;
        // Pad with spaces if needed for width
        while self.cursor >= self.cells.len() {
            self.cells.push(Cell::default());
        }
    }

    fn left(&mut self, n: usize) {
        self.cursor = self.cursor.saturating_sub(n);
    }

    fn right(&mut self, n: usize) {
        self.cursor += n;
    }

    fn up(&mut self, _n: usize) {}
    fn down(&mut self, _n: usize) {}

    fn newline(&mut self) {
        // Pad to width
        while !self.cursor.is_multiple_of(self.width) {
            self.advance();
        }
        self.put('\n');
        self.cursor += 1;
    }

    fn carriage_return(&mut self) {
        self.cursor -= self.cursor % self.width;
    }

    fn backspace(&mut self) {
        self.left(1);
        self.put(' ');
    }

    fn move_rel(&mut self, _dx: i32, _dy: i32) {}
    fn move_abs(&mut self, _row: usize, _col: usize) {}
    fn clear_screen(&mut self) {
        self.cells.clear();
        self.cursor = 0;
    }
    fn clear_line(&mut self) {
        let start = self.cursor - (self.cursor % self.width);
        for i in start..start + self.width {
            if i < self.cells.len() {
                self.cells[i] = Cell::default();
            }
        }
    }

    fn reset_attrs(&mut self) {
        self.current_cell = Cell::default();
    }

    fn set_bold(&mut self, bold: bool) { self.current_cell.bold = bold; }
    fn set_italic(&mut self, italic: bool) { self.current_cell.italic = italic; }
    fn set_underline(&mut self, underline: bool) { self.current_cell.underline = underline; }
    fn set_dim(&mut self, dim: bool) { self.current_cell.dim = dim; }
    fn set_fg(&mut self, color: Color) { self.current_cell.fg = color; }
    fn set_bg(&mut self, color: Color) { self.current_cell.bg = color; }
    fn get_fg(&self) -> Color { self.current_cell.fg }
    fn get_bg(&self) -> Color { self.current_cell.bg }
}

fn generate_demo_text(_width: usize) -> String {
    let mut demo = String::new();

    // Headers
    demo.push_str("=== ANSI SGR Demo ===\n\n");

    // Text Attributes
    demo.push_str("Text Attributes:\n");
    demo.push_str("\x1B[1mBold Text\x1B[22m\n");
    demo.push_str("\x1B[3mItalic Text\x1B[23m\n");
    demo.push_str("\x1B[4mUnderlined Text\x1B[24m\n");
    demo.push_str("\x1B[2mDim Text\x1B[22m\n");
    demo.push_str("\x1B[1;3;4;2mBold + Italic + Underline + Dim\x1B[0m\n\n");

    // Standard FG Swatches (on white BG for visibility)
    demo.push_str("Standard FG (30-37) on White BG:\n\x1B[47m");
    for i in 0..8u8 {
        let code = format!("\x1B[3{}m█ ", 30 + i);
        demo.push_str(&code);
    }
    demo.push_str("\x1B[0m\n\n");

    // Bright FG Swatches
    demo.push_str("Bright FG (90-97) on White BG:\n\x1B[47m");
    for i in 0..8u8 {
        let code = format!("\x1B[9{}m█ ", 90 + i);
        demo.push_str(&code);
    }
    demo.push_str("\x1B[0m\n\n");

    // Standard BG Swatches (black text on colored BG)
    demo.push_str("Standard BG (40-47) with Black Text:\n\x1B[30m");
    for i in 0..8u8 {
        let code = format!(" \x1B[4{}m█\x1B[49m", 40 + i);
        demo.push_str(&code);
    }
    demo.push_str("\x1B[0m\n\n");

    // Bright BG Swatches
    demo.push_str("Bright BG (100-107) with Black Text:\n\x1B[30m");
    for i in 0..8u8 {
        let code = format!(" \x1B[10{}m█\x1B[109m", 100 + i);
        demo.push_str(&code);
    }
    demo.push_str("\x1B[0m\n\n");

    // 256 Colors FG (select palette)
    demo.push_str("256 FG Examples on White BG:\n\x1B[47m");
    let indices = [196, 21, 46, 226, 201, 51, 240];  // Red, Blue, Green, Yellow, Magenta, Cyan, Gray
    for &idx in &indices {
        let code = format!("\x1B[38;5;{}m█ ", idx);
        demo.push_str(&code);
    }
    demo.push_str("\x1B[0m\n\n");

    // 256 BG
    demo.push_str("256 BG Examples with Black Text:\n\x1B[30m");
    for &idx in &indices {
        let code = format!(" \x1B[48;5;{}m█\x1B[49m", idx);
        demo.push_str(&code);
    }
    demo.push_str("\x1B[0m\n\n");

    // RGB Examples
    demo.push_str("RGB FG on White BG:\n\x1B[47m");
    let rgbs = [
        (255, 100, 50),  // Orange
        (138, 43, 226),  // BlueViolet
        (255, 20, 147),  // DeepPink
        (50, 205, 50),   // LimeGreen
    ];
    for (r, g, b) in rgbs {
        let code = format!("\x1B[38;2;{};{};{}m█ ", r, g, b);
        demo.push_str(&code);
    }
    demo.push_str("\x1B[0m\n\n");

    // RGB BG
    demo.push_str("RGB BG with Black Text:\n\x1B[30m");
    for (r, g, b) in rgbs {
        let code = format!(" \x1B[48;2;{};{};{}m█\x1B[49m", r, g, b);
        demo.push_str(&code);
    }
    demo.push_str("\x1B[0m\n\n");

    // Combined
    demo.push_str("Combined Styles:\n");
    demo.push_str("\x1B[1;31;44mBold Red on Blue█\x1B[0m\n");
    demo.push_str("\x1B[3;4;92;103mItalic Underline Bright Green on Bright Yellow█\x1B[0m\n");
    demo.push_str("\x1B[2;38;2;255;165;0;48;2;25;25;112mDim Orange on MidnightBlue (RGB)█\x1B[0m\n\n");

    // Grayscale
    demo.push_str("Grayscale (232-255):\n\x1B[47m");
    for i in (232..=255).step_by(4) {
        let code = format!("\x1B[38;5;{}m█", i);
        demo.push_str(&code);
    }
    demo.push_str("\x1B[0m Gradient\n\n");

    // Defaults
    demo.push_str("Defaults (39/49):\n");
    demo.push_str("\x1B[31;44mCustom Red on Blue\x1B[39;49mReset to Default FG (White) on Default BG (Black)█\x1B[0m\n\n");

    demo
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut parser = AnsiParser::new();
    let mut grid = SimpleGrid::new(80);  // 80-col width

    let input = if args.len() > 1 {
        fs::read_to_string(&args[1]).expect("Failed to read input file")
    } else {
        generate_demo_text(80)
    };

    // Parse
    parser.feed_str(&input, &mut grid);
    println!("Parsed {} cells from {}", grid.cells.len(), if args.len() > 1 { &args[1] } else { "built-in demo" });

    // HTML
    let html = grid.render_to_html();
    fs::write("ansi_demo.html", &html).expect("Failed to write HTML");
    println!("HTML saved to ansi_demo.html");

    // Programmatic Tests
    println!("\n=== Programmatic Tests ===");
    let tests = vec![
        ("\x1B[31mRed█\x1B[0m", "Standard red FG"),
        ("\x1B[1;31mBold Red█\x1B[0m", "Bold + red FG"),
        ("\x1B[2;91mDim Bright Red█\x1B[0m", "Dim + bright red FG"),
        ("\x1B[38;5;196mBright Red (256)█\x1B[0m", "256-color"),
        ("\x1B[38;2;255;0;0mPure Red (RGB)█\x1B[0m", "RGB"),
        ("\x1B[41;30mRed BG w/ Black Text█\x1B[0m", "Red BG"),
        ("\x1B[31;44mRed on Blue█\x1B[39;49mDefault FG/BG█\x1B[0m", "Defaults (39/49)"),
    ];

    for (seq, desc) in tests {
        let mut test_grid = SimpleGrid::new(80);
        let mut test_parser = AnsiParser::new();
        test_parser.feed_str(seq, &mut test_grid);
        
        println!("{}: {}", desc, seq.replace('\x1B', "ESC").replace('█', "[BLOCK]"));
        for (i, cell) in test_grid.cells.iter().enumerate().filter(|(_, c)| c.ch != ' ') {
            println!("  Cell {}: '{}' | FG: rgba({:.2},{:.2},{:.2},{:.2}) | BG: rgba({:.2},{:.2},{:.2},{:.2}) | Bold:{}, Italic:{}, Underline:{}, Dim:{}",
                i, cell.ch,
                cell.fg.r, cell.fg.g, cell.fg.b, cell.fg.a,
                cell.bg.r, cell.bg.g, cell.bg.b, cell.bg.a,
                cell.bold, cell.italic, cell.underline, cell.dim);
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_generates_html() {
        let input = generate_demo_text(80);
        let mut parser = AnsiParser::new();
        let mut grid = SimpleGrid::new(80);
        parser.feed_str(&input, &mut grid);
        let html = grid.render_to_html();
        assert!(html.contains("<html>"));
        assert!(html.contains("ANSI SGR Demo"));
        assert!(html.contains("rgba"));
        assert!(html.contains("█"));  // Swatches
    }

    #[test]
    fn test_defaults() {
        let mut parser = AnsiParser::new();
        let mut grid = SimpleGrid::new(80);
        parser.feed_str("\x1B[31mRed\x1B[39mDefault\x1B[44mBlueBG\x1B[49mDefaultBG█", &mut grid);
        let last = grid.cells.last().unwrap();
        assert_eq!(last.bg.r, 0.0);  // Default black
        assert_eq!(last.fg.r, 1.0);  // Default white
    }

    #[test]
    fn test_dim_combined() {
        let mut parser = AnsiParser::new();
        let mut grid = SimpleGrid::new(80);
        parser.feed_str("\x1B[2;31mDim Red█\x1B[0m", &mut grid);
        let cell = &grid.cells[0];
        assert!(cell.dim);
        assert_eq!(cell.fg.r, 1.0);  // Red
        assert_eq!(cell.fg.g, 0.0);
        assert_eq!(cell.fg.b, 0.0);
    }

    #[test]
    fn incomplete_sequence_no_panic() {
        let mut parser = AnsiParser::new();
        let mut grid = SimpleGrid::new(80);
        parser.feed_str("\x1B[38;2;255", &mut grid);  // Incomplete RGB
        assert!(grid.cells.is_empty());  // No output, but no panic
    }

    #[test]
    fn swatch_generation() {
        let input = generate_demo_text(80);
        assert!(input.contains("\x1B[38;5;196m"));  // 256 check
        assert!(input.contains("\x1B[38;2;255;100;50m"));  // RGB
        assert!(input.contains("\x1B[39;49m"));  // Defaults
    }
}

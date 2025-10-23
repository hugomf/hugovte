// examples/sgr_demo.rs
// Demonstrates all SGR color and text formatting capabilities

use hugovte::ansi::{AnsiParser, AnsiGrid, Color, Cell};

struct SimpleGrid {
    cells: Vec<Cell>,
    cursor: usize,
    current_cell: Cell,
}

impl SimpleGrid {
    fn new() -> Self {
        Self {
            cells: Vec::new(),
            cursor: 0,
            current_cell: Cell::default(),
        }
    }

    fn render_to_html(&self) -> String {
        let mut html = String::from("<div style=\"background: black; padding: 10px; font-family: monospace;\">\n");
        
        for cell in &self.cells {
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
            if cell.bold {
                text_style.push_str("font-weight: bold;");
            }
            if cell.italic {
                text_style.push_str("font-style: italic;");
            }
            if cell.underline {
                text_style.push_str("text-decoration: underline;");
            }
            if cell.dim {
                text_style.push_str("opacity: 0.6;");
            }
            
            html.push_str(&format!(
                "<span style=\"{}{}{}\">{}</span>",
                fg_style, bg_style, text_style,
                if cell.ch == '\n' { "<br>" } else { &cell.ch.to_string() }
            ));
        }
        
        html.push_str("</div>");
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
        self.current_cell.ch = '\n';
        self.cells.push(self.current_cell);
        self.cursor = self.cells.len();
    }

    fn carriage_return(&mut self) {}
    fn backspace(&mut self) {}
    fn move_rel(&mut self, _dx: i32, _dy: i32) {}
    fn move_abs(&mut self, _row: usize, _col: usize) {}
    fn clear_screen(&mut self) {
        self.cells.clear();
        self.cursor = 0;
    }
    fn clear_line(&mut self) {}

    fn reset_attrs(&mut self) {
        self.current_cell = Cell::default();
    }

    fn set_bold(&mut self, bold: bool) {
        self.current_cell.bold = bold;
    }

    fn set_italic(&mut self, italic: bool) {
        self.current_cell.italic = italic;
    }

    fn set_underline(&mut self, underline: bool) {
        self.current_cell.underline = underline;
    }

    fn set_dim(&mut self, dim: bool) {
        self.current_cell.dim = dim;
    }

    fn set_fg(&mut self, color: Color) {
        self.current_cell.fg = color;
    }

    fn set_bg(&mut self, color: Color) {
        self.current_cell.bg = color;
    }

    fn get_fg(&self) -> Color {
        self.current_cell.fg
    }

    fn get_bg(&self) -> Color {
        self.current_cell.bg
    }
}


fn main() {
    let mut parser = AnsiParser::new();
    let mut grid = SimpleGrid::new();

    // Demonstrate all SGR features
    let demo_text = concat!(
        "=== ANSI SGR Demo ===\n\n",
        
        "Text Attributes:\n",
        "\x1B[1mBold Text\x1B[0m\n",
        "\x1B[3mItalic Text\x1B[0m\n",
        "\x1B[4mUnderlined Text\x1B[0m\n",
        "\x1B[2mDim Text\x1B[0m\n",
        "\x1B[1;3;4mBold + Italic + Underline\x1B[0m\n\n",
        
        "Standard Colors (30-37):\n",
        "\x1B[30mBlack \x1B[31mRed \x1B[32mGreen \x1B[33mYellow ",
        "\x1B[34mBlue \x1B[35mMagenta \x1B[36mCyan \x1B[37mWhite\x1B[0m\n\n",
        
        "Bright Colors (90-97):\n",
        "\x1B[90mBright Black \x1B[91mBright Red \x1B[92mBright Green ",
        "\x1B[93mBright Yellow \x1B[94mBright Blue \x1B[95mBright Magenta ",
        "\x1B[96mBright Cyan \x1B[97mBright White\x1B[0m\n\n",
        
        "Background Colors (40-47):\n",
        "\x1B[40;37m Black \x1B[41;37m Red \x1B[42;30m Green \x1B[43;30m Yellow ",
        "\x1B[44;37m Blue \x1B[45;37m Magenta \x1B[46;30m Cyan \x1B[47;30m White \x1B[0m\n\n",
        
        "256 Colors (38;5;n):\n",
        "\x1B[38;5;196mRed \x1B[38;5;21mBlue \x1B[38;5;46mGreen ",
        "\x1B[38;5;226mYellow \x1B[38;5;201mMagenta \x1B[38;5;51mCyan\x1B[0m\n\n",
        
        "RGB Colors (38;2;r;g;b):\n",
        "\x1B[38;2;255;100;50mOrange \x1B[38;2;138;43;226mPurple ",
        "\x1B[38;2;255;20;147mPink \x1B[38;2;50;205;50mLime\x1B[0m\n\n",
        
        "Combined Styles:\n",
        "\x1B[1;31;44mBold Red on Blue\x1B[0m\n",
        "\x1B[3;4;92;103mItalic Underline Bright Green on Bright Yellow\x1B[0m\n",
        "\x1B[1;38;2;255;165;0;48;2;25;25;112mBold Orange on MidnightBlue (RGB)\x1B[0m\n\n",
        
        "Grayscale (232-255):\n",
        "\x1B[38;5;232m█\x1B[38;5;237m█\x1B[38;5;242m█\x1B[38;5;247m█",
        "\x1B[38;5;252m█\x1B[38;5;255m█\x1B[0m Gradient\n\n",
    );

    // Parse the demo text
    parser.feed_str(demo_text, &mut grid);

    // Output results
    println!("Parsed {} cells", grid.cells.len());
    println!("\nHTML Output:");
    println!("{}", grid.render_to_html());

    // Save HTML to file for viewing
    std::fs::write("ansi_demo.html", grid.render_to_html())
        .expect("Failed to write HTML file");
    println!("\nHTML output saved to ansi_demo.html");
    
    // Also demonstrate programmatic usage
    println!("\n\n=== Programmatic Color Test ===");
    
    let color_tests = vec![
        ("\x1B[31mRed", "Standard red foreground"),
        ("\x1B[1;31mBold Red", "Bold + red"),
        ("\x1B[38;5;196mBright Red (256)", "256-color mode"),
        ("\x1B[38;2;255;0;0mPure Red (RGB)", "RGB mode"),
    ];
    
    for (seq, desc) in color_tests {
        let mut test_grid = SimpleGrid::new();
        let mut test_parser = AnsiParser::new();
        test_parser.feed_str(&format!("{}\x1B[0m", seq), &mut test_grid);
        
        println!("{}: {}", desc, seq.replace('\x1B', "ESC"));
        if !test_grid.cells.is_empty() {
            let cell = &test_grid.cells[0];
            println!("  Color: rgba({:.2}, {:.2}, {:.2}, {:.2})", 
                     cell.fg.r, cell.fg.g, cell.fg.b, cell.fg.a);
            println!("  Bold: {}, Italic: {}, Underline: {}", 
                     cell.bold, cell.italic, cell.underline);
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_runs_without_panic() {
        let mut parser = AnsiParser::new();
        let mut grid = SimpleGrid::new();
        
        parser.feed_str("\x1B[1;31mHello\x1B[0m World", &mut grid);
        assert!(grid.cells.len() > 0);
        assert_eq!(grid.cells[0].ch, 'H');
        assert!(grid.cells[0].bold);
    }

    #[test]
    fn html_generation() {
        let mut parser = AnsiParser::new();
        let mut grid = SimpleGrid::new();
        
        parser.feed_str("\x1B[31mRed\x1B[0m", &mut grid);
        let html = grid.render_to_html();
        
        assert!(html.contains("color: rgba"));
        assert!(html.contains("Red"));
    }
}

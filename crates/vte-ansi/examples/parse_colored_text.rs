//! # Simple ANSI Color Parser
//!
//! This example demonstrates parsing ANSI escape sequences and extracting
//! text with color/style information in a simple format.

use vte_ansi::{AnsiParser, AnsiGrid, Color};

// Simple result collector
#[derive(Clone, Debug)]
struct ColoredChar {
    ch: char,
    fg: Color,
    bg: Color,
    bold: bool,
    italic: bool,
}

struct ColorCollector {
    chars: Vec<ColoredChar>,
    current_fg: Color,
    current_bg: Color,
    bold: bool,
    italic: bool,
}

impl ColorCollector {
    fn new() -> Self {
        Self {
            chars: Vec::new(),
            current_fg: Color::default(),
            current_bg: Color::rgb(0., 0., 0.),
            bold: false,
            italic: false,
        }
    }

    fn display(&self) {
        println!("Parsed colored text:");
        for chunk in self.chars.chunks(20) { // Show in chunks for readability
            for chr in chunk {
                print!("{}", chr.ch);
            }
            println!();
            for chr in chunk {
                if chr.fg != Color::default() {
                    print!("^");
                } else {
                    print!(" ");
                }
            }
            println!();
        }
        println!("Total: {} characters parsed", self.chars.len());
    }
}

impl AnsiGrid for ColorCollector {
    fn put(&mut self, ch: char) {
        self.chars.push(ColoredChar {
            ch,
            fg: self.current_fg,
            bg: self.current_bg,
            bold: self.bold,
            italic: self.italic,
        });
    }

    fn advance(&mut self) {
        // For simple collection, we don't advance without characters
    }

    // Style setting methods
    fn set_bold(&mut self, bold: bool) { self.bold = bold; }
    fn set_italic(&mut self, italic: bool) { self.italic = italic; }
    fn set_fg(&mut self, color: Color) { self.current_fg = color; }
    fn set_bg(&mut self, color: Color) { self.current_bg = color; }
    fn reset_attrs(&mut self) {
        self.current_fg = Color::default();
        self.current_bg = Color::rgb(0., 0., 0.);
        self.bold = false;
        self.italic = false;
    }

    fn set_underline(&mut self, _underline: bool) {}
    fn set_dim(&mut self, _dim: bool) {}

    // Empty implementations for required methods we don't use
    fn left(&mut self, _n: usize) {}
    fn right(&mut self, _n: usize) {}
    fn up(&mut self, _n: usize) {}
    fn down(&mut self, _n: usize) {}
    fn newline(&mut self) {}
    fn carriage_return(&mut self) {}
    fn backspace(&mut self) {}
    fn move_rel(&mut self, _dx: i32, _dy: i32) {}
    fn move_abs(&mut self, _row: usize, _col: usize) {}
    fn clear_screen(&mut self) {}
    fn clear_line(&mut self) {}
    fn get_fg(&self) -> Color { self.current_fg }
    fn get_bg(&self) -> Color { self.current_bg }
}

fn main() {
    println!("Simple ANSI Color Parser Example");
    println!("===============================\n");

    // Create ANSI text using manual byte construction to avoid syntax issues
    let ansi_bytes: Vec<u8> = vec![
        b'H', b'e', b'l', b'l', b'o', b' ', 27, b'[', b'3', b'1', b'm',
        b'R', b'e', b'd', 27, b'[', b'0', b'm', b' ', b'W', b'o', b'r', b'l', b'd',
        b'!', b'\n',
        27, b'[', b'1', b';', b'3', b'2', b'm', b'B', b'o', b'l', b'd', b' ',
        b'G', b'r', b'e', b'e', b'n', 27, b'[', b'0', b'm'
    ];

    let ansi_text = String::from_utf8_lossy(&ansi_bytes);

    println!("Input ANSI text: {}", ansi_text);
    println!("(containing {} bytes)", ansi_bytes.len());

    let mut parser = AnsiParser::new();
    let mut collector = ColorCollector::new();

    parser.feed_str(&ansi_text, &mut collector);

    println!();
    collector.display();

    println!("\nThis example demonstrates basic ANSI sequence parsing");
    println!("and character color/style extraction!");
}

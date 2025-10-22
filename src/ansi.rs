use std::fmt;

/// Color in 0.0..=1.0 space
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color { pub r: f64, pub g: f64, pub b: f64 }
impl Default for Color { fn default() -> Self { Color { r: 1.0, g: 1.0, b: 1.0 } } }
impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "rgb({:.2}, {:.2}, {:.2})", self.r, self.g, self.b)
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Cell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub dim: bool,
}

pub trait AnsiGrid {
    fn put(&mut self, ch: char);
    fn advance(&mut self);
    fn left(&mut self, n: usize);
    fn right(&mut self, n: usize);
    fn up(&mut self, n: usize);
    fn down(&mut self, n: usize);
    fn newline(&mut self);
    fn carriage_return(&mut self);
    fn backspace(&mut self);
    fn move_rel(&mut self, dx: i32, dy: i32);
    fn move_abs(&mut self, row: usize, col: usize);
    fn clear_screen(&mut self);
    fn clear_line(&mut self);
    fn reset_attrs(&mut self);
    fn set_bold(&mut self, bold: bool);
    fn set_italic(&mut self, italic: bool);
    fn set_underline(&mut self, underline: bool);
    fn set_dim(&mut self, dim: bool);
    fn set_fg(&mut self, color: Color);
    fn set_bg(&mut self, color: Color);
    fn set_title(&mut self, title: &str) { let _ = title; }
    fn get_fg(&self) -> Color;
    fn get_bg(&self) -> Color;
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum State {
    Normal,
    Escape,
    Csi,
    Osc,
}

pub struct Parser {
    state: State,
    params: Vec<u16>,
    current_param: u16,
    osc_buffer: String,
    in_osc_escape: bool,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            state: State::Normal,
            params: Vec::new(),
            current_param: 0,
            osc_buffer: String::new(),
            in_osc_escape: false,
        }
    }

    pub fn process(&mut self, byte: u8, grid: &mut dyn AnsiGrid) {
        match self.state {
            State::Normal => self.process_normal(byte, grid),
            State::Escape => self.process_escape(byte, grid),
            State::Csi => self.process_csi(byte, grid),
            State::Osc => self.process_osc(byte, grid),
        }
    }

    fn process_normal(&mut self, byte: u8, grid: &mut dyn AnsiGrid) {
        match byte {
            0x1B => {
                self.state = State::Escape;
            }
            b'\n' => grid.newline(),
            b'\r' => grid.carriage_return(),
            0x08 | 0x7F => grid.backspace(),
            b'\t' => {
                // Simple tab - just advance 4 spaces
                for _ in 0..4 {
                    grid.put(' ');
                    grid.advance();
                }
            }
            0x20..=0x7E => {
                grid.put(byte as char);
                grid.advance();
            }
            _ => {} // Ignore other control characters
        }
    }

    fn process_escape(&mut self, byte: u8, grid: &mut dyn AnsiGrid) {
        match byte {
            b'[' => {
                self.state = State::Csi;
                self.params.clear();
                self.current_param = 0;
            }
            b']' => {
                self.state = State::Osc;
                self.osc_buffer.clear();
                self.in_osc_escape = false;
            }
            b'c' => {
                // Reset
                grid.reset_attrs();
                grid.clear_screen();
                self.state = State::Normal;
            }
            b'D' => {
                // Index
                grid.newline();
                self.state = State::Normal;
            }
            b'E' => {
                // Next line
                grid.carriage_return();
                grid.newline();
                self.state = State::Normal;
            }
            b'M' => {
                // Reverse index
                grid.up(1);
                self.state = State::Normal;
            }
            _ => {
                // Unknown escape sequence, return to normal
                self.state = State::Normal;
            }
        }
    }

    fn process_csi(&mut self, byte: u8, grid: &mut dyn AnsiGrid) {
        match byte {
            b'0'..=b'9' => {
                self.current_param = self.current_param * 10 + (byte - b'0') as u16;
            }
            b';' => {
                self.params.push(self.current_param);
                self.current_param = 0;
            }
            b'?' => {
                // Private mode character, ignore for now
            }
            _ => {
                // Final byte
                if self.current_param > 0 || self.params.is_empty() {
                    self.params.push(self.current_param);
                }
                self.execute_csi(byte, grid);
                self.state = State::Normal;
                self.params.clear();
                self.current_param = 0;
            }
        }
    }

    fn execute_csi(&mut self, byte: u8, grid: &mut dyn AnsiGrid) {
        match byte {
            b'A' => grid.up(self.get_param(0, 1)),
            b'B' => grid.down(self.get_param(0, 1)),
            b'C' => grid.right(self.get_param(0, 1)),
            b'D' => grid.left(self.get_param(0, 1)),
            b'H' | b'f' => {
                let row = self.get_param(0, 1).saturating_sub(1);
                let col = self.get_param(1, 1).saturating_sub(1);
                grid.move_abs(row as usize, col as usize);
            }
            b'J' => {
                let mode = self.get_param(0, 0);
                if mode == 2 {
                    grid.clear_screen();
                }
            }
            b'K' => {
                let mode = self.get_param(0, 0);
                if mode == 2 {
                    grid.clear_line();
                }
            }
            b'm' => self.execute_sgr(grid),
            _ => {} // Ignore unsupported CSI sequences
        }
    }

    fn execute_sgr(&mut self, grid: &mut dyn AnsiGrid) {
        if self.params.is_empty() {
            grid.reset_attrs();
            return;
        }

        let mut i = 0;
        while i < self.params.len() {
            match self.params[i] {
                0 => grid.reset_attrs(),
                1 => grid.set_bold(true),
                3 => grid.set_italic(true),
                4 => grid.set_underline(true),
                7 => {
                    // Reverse video
                    let temp = grid.get_fg();
                    grid.set_fg(grid.get_bg());
                    grid.set_bg(temp);
                }
                22 => grid.set_bold(false),
                23 => grid.set_italic(false),
                24 => grid.set_underline(false),
                27 => {
                    // Reverse video off
                    let temp = grid.get_fg();
                    grid.set_fg(grid.get_bg());
                    grid.set_bg(temp);
                }
                30..=37 => grid.set_fg(ansi_color(self.params[i] - 30)),
                40..=47 => grid.set_bg(ansi_color(self.params[i] - 40)),
                90..=97 => grid.set_fg(ansi_bright_color(self.params[i] - 90)),
                100..=107 => grid.set_bg(ansi_bright_color(self.params[i] - 100)),
                38 | 48 => {
                    if i + 1 < self.params.len() {
                        match self.params[i + 1] {
                            5 if i + 2 < self.params.len() => {
                                let color = ansi_256_color(self.params[i + 2]);
                                if self.params[i] == 38 {
                                    grid.set_fg(color);
                                } else {
                                    grid.set_bg(color);
                                }
                                i += 2;
                            }
                            2 if i + 4 < self.params.len() => {
                                let r = self.params[i + 2] as f64 / 255.0;
                                let g = self.params[i + 3] as f64 / 255.0;
                                let b = self.params[i + 4] as f64 / 255.0;
                                let color = Color { r, g, b };
                                if self.params[i] == 38 {
                                    grid.set_fg(color);
                                } else {
                                    grid.set_bg(color);
                                }
                                i += 4;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }

    fn process_osc(&mut self, byte: u8, grid: &mut dyn AnsiGrid) {
        if self.in_osc_escape {
            if byte == b'\\' {
                // OSC sequence terminated by ESC \
                self.finish_osc(grid);
            } else {
                // Invalid escape in OSC, treat ESC as data
                self.osc_buffer.push('\x1B');
                self.osc_buffer.push(byte as char);
                self.in_osc_escape = false;
            }
        } else if byte == 0x1B {
            self.in_osc_escape = true;
        } else if byte == 0x07 {
            // OSC sequence terminated by BEL
            self.finish_osc(grid);
        } else {
            self.osc_buffer.push(byte as char);
        }
    }

    fn finish_osc(&mut self, grid: &mut dyn AnsiGrid) {
        if let Some((num, text)) = self.osc_buffer.split_once(';') {
            if num == "0" || num == "2" {
                grid.set_title(text);
            }
        }
        self.state = State::Normal;
        self.osc_buffer.clear();
        self.in_osc_escape = false;
    }

    fn get_param(&self, index: usize, default: u16) -> usize {
        self.params.get(index).copied().unwrap_or(default) as usize
    }
}

// Color conversion functions
fn ansi_color(index: u16) -> Color {
    match index {
        0 => Color { r: 0.0, g: 0.0, b: 0.0 },       // Black
        1 => Color { r: 0.8, g: 0.0, b: 0.0 },       // Red
        2 => Color { r: 0.0, g: 0.8, b: 0.0 },       // Green
        3 => Color { r: 0.8, g: 0.8, b: 0.0 },       // Yellow
        4 => Color { r: 0.0, g: 0.0, b: 0.8 },       // Blue
        5 => Color { r: 0.8, g: 0.0, b: 0.8 },       // Magenta
        6 => Color { r: 0.0, g: 0.8, b: 0.8 },       // Cyan
        7 => Color { r: 0.8, g: 0.8, b: 0.8 },       // White
        _ => Color::default(),
    }
}

fn ansi_bright_color(index: u16) -> Color {
    match index {
        0 => Color { r: 0.4, g: 0.4, b: 0.4 },       // Bright Black (Gray)
        1 => Color { r: 1.0, g: 0.0, b: 0.0 },       // Bright Red
        2 => Color { r: 0.0, g: 1.0, b: 0.0 },       // Bright Green
        3 => Color { r: 1.0, g: 1.0, b: 0.0 },       // Bright Yellow
        4 => Color { r: 0.0, g: 0.0, b: 1.0 },       // Bright Blue
        5 => Color { r: 1.0, g: 0.0, b: 1.0 },       // Bright Magenta
        6 => Color { r: 0.0, g: 1.0, b: 1.0 },       // Bright Cyan
        7 => Color { r: 1.0, g: 1.0, b: 1.0 },       // Bright White
        _ => Color::default(),
    }
}

fn ansi_256_color(index: u16) -> Color {
    match index {
        0..=7 => ansi_color(index),
        8..=15 => ansi_bright_color(index - 8),
        16..=231 => {
            let idx = index - 16;
            let r = (idx / 36) % 6;
            let g = (idx / 6) % 6;
            let b = idx % 6;
            Color {
                r: r as f64 / 5.0,
                g: g as f64 / 5.0,
                b: b as f64 / 5.0,
            }
        }
        232..=255 => {
            let gray = (index - 232) as f64 / 23.0;
            Color { r: gray, g: gray, b: gray }
        }
        _ => Color::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Mock {
        fg: Color,
        bg: Color,
    }
    
    impl Mock {
        fn new() -> Self {
            Self {
                fg: Color::default(),
                bg: Color { r: 0.0, g: 0.0, b: 0.0 },
            }
        }
    }

    impl AnsiGrid for Mock {
        fn put(&mut self, ch: char) { print!("{ch}"); }
        fn advance(&mut self) {}
        fn left(&mut self, _: usize) {}
        fn right(&mut self, _: usize) {}
        fn up(&mut self, _: usize) {}
        fn down(&mut self, _: usize) {}
        fn newline(&mut self) { println!(); }
        fn carriage_return(&mut self) {}
        fn backspace(&mut self) {}
        fn move_rel(&mut self, _: i32, _: i32) {}
        fn move_abs(&mut self, _: usize, _: usize) {}
        fn clear_screen(&mut self) {}
        fn clear_line(&mut self) {}
        fn reset_attrs(&mut self) {
            self.fg = Color::default();
            self.bg = Color { r: 0.0, g: 0.0, b: 0.0 };
        }
        fn set_bold(&mut self, _: bool) {}
        fn set_italic(&mut self, _: bool) {}
        fn set_underline(&mut self, _: bool) {}
        fn set_dim(&mut self, _: bool) {}
        fn set_fg(&mut self, c: Color) { 
            self.fg = c;
            println!("[FG {c}]"); 
        }
        fn set_bg(&mut self, c: Color) { 
            self.bg = c;
            println!("[BG {c}]"); 
        }
        fn set_title(&mut self, t: &str) { println!("[TITLE {t}]"); }
        fn get_fg(&self) -> Color { self.fg }
        fn get_bg(&self) -> Color { self.bg }
    }

    #[test]
    fn test_basic_parsing() {
        let mut p = Parser::new();
        let mut m = Mock::new();
        let data = b"Hello World\n";
        for &b in data { p.process(b, &mut m); }
    }

    #[test]
    fn test_colors() {
        let mut p = Parser::new();
        let mut m = Mock::new();
        let data = b"\x1B[31mRed\x1B[0m";
        for &b in data { p.process(b, &mut m); }
    }
}
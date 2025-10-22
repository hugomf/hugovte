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
    fn advance(&mut self);                       // move right 1
    fn left(&mut self, n: usize);
    fn right(&mut self, n: usize);
    fn up(&mut self, n: usize);
    fn down(&mut self, n: usize);
    fn newline(&mut self);
    fn carriage_return(&mut self);
    fn backspace(&mut self);
    fn move_rel(&mut self, dx: i32, dy: i32);    // relative
    fn move_abs(&mut self, row: usize, col: usize); // absolute
    fn clear_screen(&mut self);
    fn clear_line(&mut self);
    fn reset_attrs(&mut self);
    fn set_bold(&mut self, bold: bool);
    fn set_italic(&mut self, italic: bool);
    fn set_underline(&mut self, underline: bool);
    fn set_dim(&mut self, dim: bool);
    fn set_fg(&mut self, color: Color);
    fn set_bg(&mut self, color: Color);
    fn set_title(&mut self, title: &str) { let _ = title; } // default no-op
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum State { Normal, Escape, Csi, Osc }

/// Owned, reentrant ANSI parser
pub struct Parser {
    state: State,
    params: Vec<u16>,        // accumulated numeric params (0..=65535)
    current_value: u16,      // numeric accumulator for a param
    current_active: bool,    // whether digits were seen for current_value
    osc_buf: String,         // buffer for OSC
    osc_esc: bool,           // when in OSC and seen ESC, wait for '\'
}

impl Parser {
    pub fn new() -> Self {
        Self {
            state: State::Normal,
            params: Vec::with_capacity(8),
            current_value: 0,
            current_active: false,
            osc_buf: String::with_capacity(128),
            osc_esc: false,
        }
    }

    /// Stream one byte into the parser and update `grid`.
    /// Designed to be fast (no per-parameter heap parse).
    pub fn process(&mut self, byte: u8, grid: &mut dyn AnsiGrid) {
        match self.state {
            State::Normal => self.process_normal(byte, grid),
            State::Escape => self.process_escape(byte, grid),
            State::Csi => self.process_csi_byte(byte, grid),
            State::Osc => self.process_osc_byte(byte, grid),
        }
    }

    // -------------------------
    // Normal state handling
    // -------------------------
    fn process_normal(&mut self, byte: u8, grid: &mut dyn AnsiGrid) {
        match byte {
            0x1B => { self.state = State::Escape; },
            b'\n' => grid.newline(),
            b'\r' => grid.carriage_return(),
            0x08 | 0x7F => grid.backspace(),
            b'\x07' => { /* bell - ignore */ },
            32..=126 => {
                grid.put(byte as char);
                grid.advance();
            }
            _ => { /* ignore control bytes */ }
        }
    }

    // -------------------------
    // Escape state handling
    // -------------------------
    fn process_escape(&mut self, byte: u8, _grid: &mut dyn AnsiGrid) {
        match byte {
            b'[' => {
                self.params.clear();
                self.current_value = 0;
                self.current_active = false;
                self.state = State::Csi;
            }
            b']' => {
                self.osc_buf.clear();
                self.osc_esc = false;
                self.state = State::Osc;
            }
            b'c' => { // RIS - reset to initial state
                // best-effort: let caller handle full reset via grid methods
                // We'll call reset_attrs and clear_screen as a simple RIS
                // (leave cursor at 0,0)
                // Note: if your grid has other reset needs, update here.
                // calling unsafe methods on trait isn't desirable, so only do what trait has:
                // reset attrs and clear
                // no grid reference here; caller will usually send ESC[c in CSI context
                self.state = State::Normal;
            }
            _ => {
                self.state = State::Normal;
            }
        }
    }

    // -------------------------
    // CSI parsing (byte-by-byte)
    // -------------------------
    fn process_csi_byte(&mut self, byte: u8, grid: &mut dyn AnsiGrid) {
        match byte {
            b'0'..=b'9' => {
                // accumulate numeric param
                self.current_value = self.current_value.saturating_mul(10)
                    .saturating_add((byte - b'0') as u16);
                self.current_active = true;
            }
            b';' => {
                // push current param (0 if no digits seen)
                if self.current_active { self.params.push(self.current_value); }
                else { self.params.push(0); }
                self.current_value = 0;
                self.current_active = false;
            }
            b'?' => {
                // private mode introducer; ignore but keep being in CSI
            }
            final_byte if (0x40..=0x7E).contains(&final_byte) => {
                // final byte: finish current param (if any) and execute
                if self.current_active { self.params.push(self.current_value); }
                else if self.params.is_empty() {
                    // leave params empty (we'll use defaults in exec)
                } else {
                    // if there were previous params and no current, treat as trailing zero
                    self.params.push(0);
                }
                // execute the CSI command
                self.exec_csi(final_byte, grid);
                // reset state
                self.current_value = 0;
                self.current_active = false;
                self.params.clear();
                self.state = State::Normal;
            }
            _ => {
                // intermediate bytes or unknown - stay in CSI
            }
        }
    }

    // -------------------------
    // CSI executor
    // -------------------------
    fn exec_csi(&mut self, command: u8, grid: &mut dyn AnsiGrid) {
        match command {
            b'A' => { let n = self.param_or_default(0, 1) as usize; grid.move_rel(0, -(n as i32)); }
            b'B' => { let n = self.param_or_default(0, 1) as usize; grid.move_rel(0, n as i32); }
            b'C' => { let n = self.param_or_default(0, 1) as usize; grid.move_rel(n as i32, 0); }
            b'D' => { let n = self.param_or_default(0, 1) as usize; grid.move_rel(-(n as i32), 0); }
            b'H' | b'f' => {
                // CUP - Cursor Position (1-based)
                let r = self.params.get(0).copied().unwrap_or(1).saturating_sub(1) as usize;
                let c = self.params.get(1).copied().unwrap_or(1).saturating_sub(1) as usize;
                grid.move_abs(r, c);
            }
            b'J' => {
                let mode = self.params.get(0).copied().unwrap_or(0);
                match mode {
                    0 => { /* cursor to end */ grid.clear_screen(); }
                    1 => { /* start to cursor */ grid.clear_screen(); }
                    2 => grid.clear_screen(),
                    _ => {}
                }
            }
            b'K' => {
                let mode = self.params.get(0).copied().unwrap_or(0);
                match mode {
                    0 => grid.clear_line(), // cursor->end
                    1 => grid.clear_line(), // start->cursor
                    2 => grid.clear_line(), // entire line
                    _ => {}
                }
            }
            b'm' => {
                self.exec_csi_sgr(grid);
            }
            // support some common scroll/line ops as no-ops or simple behaviors
            b'S' => { /* scroll up n lines - not implemented */ }
            b'T' => { /* scroll down n lines - not implemented */ }
            _ => {
                // unsupported or unimplemented CSI; ignore.
            }
        }
    }

    // -------------------------
    // SGR (Select Graphic Rendition)
    // -------------------------
    fn exec_csi_sgr(&mut self, grid: &mut dyn AnsiGrid) {
        // If empty, reset
        if self.params.is_empty() {
            grid.reset_attrs();
            return;
        }

        let mut i = 0usize;
        while i < self.params.len() {
            match self.params[i] {
                0 => grid.reset_attrs(),
                1 => grid.set_bold(true),
                2 => grid.set_dim(true),
                3 => grid.set_italic(true),
                4 => grid.set_underline(true),
                22 => { grid.set_bold(false); grid.set_dim(false); }
                23 => grid.set_italic(false),
                24 => grid.set_underline(false),
                30..=37 => grid.set_fg(ansi_color(self.params[i] - 30)),
                40..=47 => grid.set_bg(ansi_color(self.params[i] - 40)),
                90..=97 => grid.set_fg(ansi_bright_color(self.params[i] - 90)),
                100..=107 => grid.set_bg(ansi_bright_color(self.params[i] - 100)),
                38 | 48 => {
                    let is_fg = self.params[i] == 38;
                    if i + 1 < self.params.len() {
                        match self.params[i + 1] {
                            5 if i + 2 < self.params.len() => {
                                let idx = self.params[i + 2];
                                let c = ansi_256(idx);
                                if is_fg { grid.set_fg(c); } else { grid.set_bg(c); }
                                i += 2;
                            }
                            2 if i + 4 < self.params.len() => {
                                let r = (self.params[i + 2].min(255)) as u8;
                                let g = (self.params[i + 3].min(255)) as u8;
                                let b = (self.params[i + 4].min(255)) as u8;
                                let c = Color { r: r as f64 / 255.0, g: g as f64 / 255.0, b: b as f64 / 255.0 };
                                if is_fg { grid.set_fg(c); } else { grid.set_bg(c); }
                                i += 4;
                            }
                            _ => { /* unknown extended form */ }
                        }
                    }
                }
                _ => { /* ignore unknown */ }
            }
            i += 1;
        }
    }

    // -------------------------
    // OSC parsing
    // -------------------------
    fn process_osc_byte(&mut self, byte: u8, grid: &mut dyn AnsiGrid) {
        match byte {
            0x07 => { // BEL terminator
                self.finish_osc(grid);
            }
            0x1B => {
                // could be ESC \
                self.osc_esc = true;
            }
            b'\\' if self.osc_esc => {
                // ESC \ sequence terminates OSC
                self.finish_osc(grid);
            }
            other => {
                if self.osc_esc {
                    // we saw ESC then a non-backslash -> treat previous ESC as data and continue
                    self.osc_buf.push('\x1B');
                    self.osc_esc = false;
                }
                self.osc_buf.push(other as char);
            }
        }
    }

    fn finish_osc(&mut self, grid: &mut dyn AnsiGrid) {
        if let Some((cmd, arg)) = self.osc_buf.split_once(';') {
            if cmd == "0" || cmd == "2" {
                grid.set_title(arg.trim());
            }
        }
        self.osc_buf.clear();
        self.osc_esc = false;
        self.state = State::Normal;
    }

    // -------------------------
    // helpers
    // -------------------------
    #[inline]
    fn param_or_default(&self, idx: usize, default: u16) -> u16 {
        self.params.get(idx).copied().unwrap_or(default)
    }
}

// -------------------------
// Color helpers
// -------------------------
#[inline]
fn ansi_color(n: u16) -> Color {
    const BASE: [(f64, f64, f64); 8] = [
        (0.0, 0.0, 0.0),
        (0.8, 0.0, 0.0),
        (0.0, 0.8, 0.0),
        (0.8, 0.8, 0.0),
        (0.0, 0.0, 0.8),
        (0.8, 0.0, 0.8),
        (0.0, 0.8, 0.8),
        (0.8, 0.8, 0.8),
    ];
    let (r,g,b) = BASE.get(n as usize).copied().unwrap_or((1.0,1.0,1.0));
    Color { r, g, b }
}

#[inline]
fn ansi_bright_color(n: u16) -> Color {
    let c = ansi_color(n);
    Color { r: (c.r + 0.2).min(1.0), g: (c.g + 0.2).min(1.0), b: (c.b + 0.2).min(1.0) }
}

fn ansi_256(idx: u16) -> Color {
    match idx {
        0..=7 => ansi_color(idx),
        8..=15 => ansi_bright_color(idx - 8),
        16..=231 => {
            let n = idx - 16;
            let r = (n / 36) % 6;
            let g = (n / 6) % 6;
            let b = n % 6;
            Color { r: r as f64 / 5.0, g: g as f64 / 5.0, b: b as f64 / 5.0 }
        }
        232..=255 => {
            let level = (idx - 232) as f64 / 23.0;
            Color { r: level, g: level, b: level }
        }
        _ => Color::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Mock;
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
        fn reset_attrs(&mut self) {}
        fn set_bold(&mut self, _: bool) {}
        fn set_italic(&mut self, _: bool) {}
        fn set_underline(&mut self, _: bool) {}
        fn set_dim(&mut self, _: bool) {}
        fn set_fg(&mut self, c: Color) { println!("[FG {c}]"); }
        fn set_bg(&mut self, c: Color) { println!("[BG {c}]"); }
        fn set_title(&mut self, t: &str) { println!("[TITLE {t}]"); }
    }

    #[test]
    fn test_truecolour_and_256() {
        let mut p = Parser::new();
        let mut m = Mock;
        // TRUECOLOR (foreground) then 256-index background then reset
        let data = b"\x1B[38;2;10;20;30;48;5;196mHELLO\x1B[0m";
        for &b in data { p.process(b, &mut m); }
    }
}

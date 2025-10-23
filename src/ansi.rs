// src/ansi.rs
//! UTF-8-safe ANSI/VT parser.
//! Drop-in replacement: old `process(byte)` still exists but is deprecated;
//! new public API is `feed_str(&str)`.

use crate::constants::COLOR_PALETTE;
use std::fmt;

// ---------- Error handling ----------

/// Errors that can occur during ANSI parsing
#[derive(Debug, Clone, PartialEq)]
pub enum AnsiError {
    /// Too many parameters in a CSI sequence (exceeded MAX_PARAMS)
    TooManyParams { sequence: String, count: usize },
    /// OSC buffer exceeded maximum length
    OscTooLong { length: usize },
    /// Parameter value exceeded maximum
    ParamTooLarge { value: u16 },
    /// Malformed escape sequence
    MalformedSequence { context: String },
}

impl fmt::Display for AnsiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnsiError::TooManyParams { sequence, count } => {
                write!(f, "Too many parameters ({}) in sequence: {}", count, sequence)
            }
            AnsiError::OscTooLong { length } => {
                write!(f, "OSC sequence too long: {} bytes (max {})", length, MAX_OSC_LEN)
            }
            AnsiError::ParamTooLarge { value } => {
                write!(f, "Parameter value {} exceeded maximum {}", value, MAX_PARAM_VALUE)
            }
            AnsiError::MalformedSequence { context } => {
                write!(f, "Malformed escape sequence: {}", context)
            }
        }
    }
}

impl std::error::Error for AnsiError {}

/// Optional callback for reporting non-fatal parsing errors
pub type ErrorCallback = Box<dyn FnMut(AnsiError)>;

// ---------- safety constants ----------
const MAX_PARAMS: usize = 32;
const MAX_OSC_LEN: usize = 2048;
const MAX_PARAM_VALUE: u16 = 9999;

// ---------- Colour ----------

/// Color in 0.0..=1.0 space with alpha channel
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Default for Color {
    fn default() -> Self {
        Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "rgba({:.2}, {:.2}, {:.2}, {:.2})",
            self.r, self.g, self.b, self.a
        )
    }
}

impl Color {
    pub fn rgba(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }
    pub fn rgb(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b, a: 1.0 }
    }
}

// ---------- Cell ----------

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

// ---------- Grid trait ----------

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
    fn set_title(&mut self, title: &str) {
        let _ = title;
    }
    fn get_fg(&self) -> Color;
    fn get_bg(&self) -> Color;

    // Phase-2 extensions with default no-op impls
    fn clear_screen_down(&mut self) {}
    fn clear_screen_up(&mut self) {}
    fn clear_line_right(&mut self) {}
    fn clear_line_left(&mut self) {}
    fn save_cursor(&mut self) {}
    fn restore_cursor(&mut self) {}
    fn set_cursor_visible(&mut self, _visible: bool) {}
    
    // Phase-2 scrolling operations
    fn scroll_up(&mut self, _n: usize) {}
    fn scroll_down(&mut self, _n: usize) {}
}

// ---------- Parser state ----------

#[derive(PartialEq, Clone, Copy, Debug)]
enum AnsiState {
    Normal,
    Escape,
    Csi,
    Osc,
}

pub struct AnsiParser {
    state: AnsiState,
    params: Vec<u16>,
    current_param: u16,
    osc_buffer: String,
    in_osc_escape: bool,
    private: bool, // for '?'
    error_callback: Option<ErrorCallback>,
    // Statistics for monitoring
    stats: ParserStats,
}

/// Statistics about parser behavior (useful for debugging and monitoring)
#[derive(Debug, Default, Clone)]
pub struct ParserStats {
    pub sequences_processed: u64,
    pub errors_encountered: u64,
    pub max_params_seen: usize,
    pub max_osc_length_seen: usize,
}

impl ParserStats {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

impl AnsiParser {
    pub fn new() -> Self {
        Self {
            state: AnsiState::Normal,
            params: Vec::new(),
            current_param: 0,
            osc_buffer: String::new(),
            in_osc_escape: false,
            private: false,
            error_callback: None,
            stats: ParserStats::default(),
        }
    }

    /// Create a parser with an error callback for diagnostics
    pub fn with_error_callback<F>(mut self, callback: F) -> Self
    where
        F: FnMut(AnsiError) + 'static,
    {
        self.error_callback = Some(Box::new(callback));
        self
    }

    /// Get current parser statistics
    pub fn stats(&self) -> &ParserStats {
        &self.stats
    }

    /// Reset statistics counters
    pub fn reset_stats(&mut self) {
        self.stats.reset();
    }

    /// Report an error through the callback if set
    fn report_error(&mut self, error: AnsiError) {
        self.stats.errors_encountered += 1;
        if let Some(ref mut callback) = self.error_callback {
            callback(error);
        }
    }

    // ===== NEW PUBLIC UTF-8 API =====
    pub fn feed_str(&mut self, s: &str, grid: &mut dyn AnsiGrid) {
        self.feed_bytes(s.as_bytes(), grid)
    }

    // ===== INTERNAL BYTE DRIVER =====
    fn feed_bytes(&mut self, bytes: &[u8], grid: &mut dyn AnsiGrid) {
        let mut i = 0;
        while i < bytes.len() {
            // fast skip until next control byte
            let ctrl_pos = memchr::memchr3(0x1B, b'\n', b'\r', &bytes[i..])
                .map(|p| i + p)
                .unwrap_or(bytes.len());

            // safe chunk: iterate by chars, not by bytes
            if let Ok(chunk) = std::str::from_utf8(&bytes[i..ctrl_pos]) {
                for ch in chunk.chars() {
                    self.process_char(ch, grid);
                }
            } else {
                // extremely rare: fall back to byte-by-byte
                for &b in &bytes[i..ctrl_pos] {
                    self.process_char(b as char, grid);
                }
            }
            i = ctrl_pos;
            if i >= bytes.len() {
                break;
            }

            // slow path: one char (may be multi-byte)
            let (ch, size) = decode_utf8(&bytes[i..]);
            self.process_char(ch, grid);
            i += size;
        }
    }

    // ===== OLD BYTE API (deprecated) =====
    #[doc(hidden)]
    #[deprecated(note = "use feed_str")]
    pub fn process(&mut self, byte: u8, grid: &mut dyn AnsiGrid) {
        self.process_char(byte as char, grid);
    }

    // ===== internal char driver =====
    fn process_char(&mut self, ch: char, grid: &mut dyn AnsiGrid) {
        match self.state {
            AnsiState::Normal => self.normal_char(ch, grid),
            AnsiState::Escape => self.escape_char(ch, grid),
            AnsiState::Csi => self.csi_char(ch, grid),
            AnsiState::Osc => self.osc_char(ch, grid),
        }
    }

    // ---------- normal state ----------
    fn normal_char(&mut self, ch: char, grid: &mut dyn AnsiGrid) {
        match ch {
            '\x1B' => self.state = AnsiState::Escape,
            '\n' => grid.newline(),
            '\r' => grid.carriage_return(),
            '\x08' => grid.backspace(),
            '\t' => {
                for _ in 0..4 {
                    grid.put(' ');
                    grid.advance();
                }
            }
            c if c >= ' ' && c != '\x7F' => {
                grid.put(c);
                grid.advance();
            }
            _ => {}
        }
    }

    // ---------- escape state ----------
    fn escape_char(&mut self, ch: char, grid: &mut dyn AnsiGrid) {
        match ch {
            '[' => {
                self.state = AnsiState::Csi;
                self.params.clear();
                self.current_param = 0;
                self.private = false;
            }
            ']' => {
                self.state = AnsiState::Osc;
                self.osc_buffer.clear();
                self.in_osc_escape = false;
            }
            '7' => {
                grid.save_cursor();
                self.state = AnsiState::Normal;
            }
            '8' => {
                grid.restore_cursor();
                self.state = AnsiState::Normal;
            }
            'c' => {
                grid.reset_attrs();
                grid.clear_screen();
                self.state = AnsiState::Normal;
            }
            'D' => {
                grid.newline();
                self.state = AnsiState::Normal;
            }
            'E' => {
                grid.carriage_return();
                grid.newline();
                self.state = AnsiState::Normal;
            }
            'M' => {
                grid.up(1);
                self.state = AnsiState::Normal;
            }
            _ => self.state = AnsiState::Normal,
        }
    }

    // ---------- CSI state ----------
    fn csi_char(&mut self, ch: char, grid: &mut dyn AnsiGrid) {
        match ch {
            '0'..='9' => {
                let new_param = self
                    .current_param
                    .saturating_mul(10)
                    .saturating_add((ch as u16).wrapping_sub(b'0' as u16));
                
                if new_param > MAX_PARAM_VALUE {
                    self.report_error(AnsiError::ParamTooLarge { value: new_param });
                    self.current_param = MAX_PARAM_VALUE;
                } else {
                    self.current_param = new_param;
                }
            }
            ';' => {
                if self.params.len() >= MAX_PARAMS {
                    self.report_error(AnsiError::TooManyParams {
                        sequence: format!("CSI with {} params", self.params.len() + 1),
                        count: self.params.len() + 1,
                    });
                } else {
                    self.params.push(self.current_param);
                }
                self.current_param = 0;
            }
            '?' => self.private = true,
            _ => {
                if self.params.len() < MAX_PARAMS
                    && (self.current_param > 0 || self.params.is_empty())
                {
                    self.params.push(self.current_param);
                }
                
                // Update stats
                self.stats.sequences_processed += 1;
                self.stats.max_params_seen = self.stats.max_params_seen.max(self.params.len());
                
                self.execute_csi(ch, grid);
                self.state = AnsiState::Normal;
                self.params.clear();
                self.current_param = 0;
                self.private = false;
            }
        }
    }

    fn execute_csi(&mut self, ch: char, grid: &mut dyn AnsiGrid) {
        match ch {
            'A' => grid.up(self.get_param(0, 1)),
            'B' => grid.down(self.get_param(0, 1)),
            'C' => grid.right(self.get_param(0, 1)),
            'D' => grid.left(self.get_param(0, 1)),
            'H' | 'f' => {
                let row = self.get_param(0, 1).saturating_sub(1);
                let col = self.get_param(1, 1).saturating_sub(1);
                grid.move_abs(row, col);
            }
            'J' => match self.get_param(0, 0) {
                0 => grid.clear_screen_down(),
                1 => grid.clear_screen_up(),
                2 => grid.clear_screen(),
                _ => {}
            },
            'K' => match self.get_param(0, 0) {
                0 => grid.clear_line_right(),
                1 => grid.clear_line_left(),
                2 => grid.clear_line(),
                _ => {}
            },
            'm' => self.execute_sgr(grid),
            'h' if self.private => {
                if self.params.first() == Some(&25) {
                    grid.set_cursor_visible(true);
                }
            }
            'l' if self.private => {
                if self.params.first() == Some(&25) {
                    grid.set_cursor_visible(false);
                }
            }
            'S' => grid.scroll_up(self.get_param(0, 1)),
            'T' => grid.scroll_down(self.get_param(0, 1)),
            's' => grid.save_cursor(),
            'u' => grid.restore_cursor(),
            _ => {}
        }
    }

    // ---------- OSC state ----------
    fn osc_char(&mut self, ch: char, grid: &mut dyn AnsiGrid) {
        if self.osc_buffer.len() >= MAX_OSC_LEN {
            self.state = AnsiState::Normal;
            return;
        }
        if self.in_osc_escape {
            if ch == '\\' {
                self.finish_osc(grid);
            } else {
                self.osc_buffer.push('\x1B');
                self.osc_buffer.push(ch);
                self.in_osc_escape = false;
            }
        } else if ch == '\x1B' {
            self.in_osc_escape = true;
        } else if ch == '\x07' {
            self.finish_osc(grid);
        } else {
            self.osc_buffer.push(ch);
        }
    }

    fn finish_osc(&mut self, grid: &mut dyn AnsiGrid) {
        if let Some((num, text)) = self.osc_buffer.split_once(';') {
            if num == "0" || num == "2" {
                grid.set_title(text);
            }
        }
        self.state = AnsiState::Normal;
        self.osc_buffer.clear();
        self.in_osc_escape = false;
    }

    fn get_param(&self, idx: usize, default: u16) -> usize {
        self.params.get(idx).copied().unwrap_or(default) as usize
    }

    // ---------- SGR ----------
    fn execute_sgr(&mut self, grid: &mut dyn AnsiGrid) {
        if self.params.is_empty() {
            grid.reset_attrs();
            return;
        }
        let mut i = 0;
        while i < self.params.len() {
            let param = self.params[i];
            match param {
                0 => grid.reset_attrs(),
                1 => grid.set_bold(true),
                2 => grid.set_dim(true),
                3 => grid.set_italic(true),
                4 => grid.set_underline(true),
                22 => {
                    grid.set_bold(false);
                    grid.set_dim(false);
                }
                23 => grid.set_italic(false),
                24 => grid.set_underline(false),
                30..=37 => grid.set_fg(ansi_color(param - 30)),
                38 => {
                    if i + 1 < self.params.len() {
                        match self.params[i + 1] {
                            5 if i + 2 < self.params.len() => {
                                let idx = self.params[i + 2];
                                grid.set_fg(ansi_256_color(idx));
                                i += 2;
                            }
                            2 => {
                                let r = self.params.get(i + 2).copied().unwrap_or(0).min(255) as f64 / 255.0;
                                let g = self.params.get(i + 3).copied().unwrap_or(0).min(255) as f64 / 255.0;
                                let b = self.params.get(i + 4).copied().unwrap_or(0).min(255) as f64 / 255.0;
                                grid.set_fg(Color::rgb(r, g, b));
                                i += 4;
                            }
                            _ => {}
                        }
                    }
                }
                39 => grid.set_fg(Color::default()),
                40..=47 => grid.set_bg(ansi_color(param - 40)),
                48 => {
                    if i + 1 < self.params.len() {
                        match self.params[i + 1] {
                            5 if i + 2 < self.params.len() => {
                                let idx = self.params[i + 2];
                                grid.set_bg(ansi_256_color(idx));
                                i += 2;
                            }
                            2 => {
                                let r = self.params.get(i + 2).copied().unwrap_or(0).min(255) as f64 / 255.0;
                                let g = self.params.get(i + 3).copied().unwrap_or(0).min(255) as f64 / 255.0;
                                let b = self.params.get(i + 4).copied().unwrap_or(0).min(255) as f64 / 255.0;
                                grid.set_bg(Color::rgb(r, g, b));
                                i += 4;
                            }
                            _ => {}
                        }
                    }
                }
                49 => grid.set_bg(Color::rgb(0.0, 0.0, 0.0)),
                90..=97 => grid.set_fg(ansi_bright_color(param - 90)),
                100..=107 => grid.set_bg(ansi_bright_color(param - 100)),
                _ => {}
            }
            i += 1;
        }
    }
}

// ---------- colour helpers ----------
fn ansi_color(idx: u16) -> Color {
    COLOR_PALETTE
        .get(idx as usize & 7)
        .copied()
        .unwrap_or_default()
}
fn ansi_bright_color(idx: u16) -> Color {
    COLOR_PALETTE
        .get((idx as usize & 7) + 8)
        .copied()
        .unwrap_or_default()
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
                a: 1.0,
            }
        }
        232..=255 => {
            let gray = (index - 232) as f64 / 23.0;
            Color {
                r: gray,
                g: gray,
                b: gray,
                a: 1.0,
            }
        }
        _ => Color::default(),
    }
}

// ---------- tiny UTF-8 ----------
fn decode_utf8(buf: &[u8]) -> (char, usize) {
    match std::str::from_utf8(buf) {
        Ok(s) => {
            let ch = s.chars().next().unwrap_or('\u{FFFD}');
            (ch, ch.len_utf8())
        }
        Err(e) => {
            let valid = e.valid_up_to();
            let size = (valid + 1).max(1).min(buf.len());
            (std::char::REPLACEMENT_CHARACTER, size)
        }
    }
}

// ---------- tests ----------
#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MockGrid {
        fg: Color,
        bg: Color,
        output: String,
        bold: bool,
        italic: bool,
        underline: bool,
        dim: bool,
    }
    
    impl MockGrid {
        fn new() -> Self {
            Self {
                fg: Color::default(),
                bg: Color::rgb(0., 0., 0.),
                output: String::new(),
                bold: false,
                italic: false,
                underline: false,
                dim: false,
            }
        }
    }


    impl AnsiGrid for MockGrid {
        fn put(&mut self, ch: char) { self.output.push(ch); }
        fn advance(&mut self) {}
        fn left(&mut self, _: usize) {}
        fn right(&mut self, _: usize) {}
        fn up(&mut self, _: usize) {}
        fn down(&mut self, _: usize) {}
        fn newline(&mut self) { self.output.push('\n'); }
        fn carriage_return(&mut self) {}
        fn backspace(&mut self) {}
        fn move_rel(&mut self, _: i32, _: i32) {}
        fn move_abs(&mut self, _: usize, _: usize) {}
        fn clear_screen(&mut self) { self.output.push_str("[CLEAR]"); }
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
        fn set_title(&mut self, t: &str) { self.output.push_str(&format!("[TITLE: {}]", t)); }
        fn get_fg(&self) -> Color { self.fg }
        fn get_bg(&self) -> Color { self.bg }
    }

    #[test]
    fn utf8_emoji() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::default();
        p.feed_str("Hi 😀\n", &mut g);
        assert_eq!(g.output, "Hi 😀\n"); 
    }

    #[test]
    fn legacy_byte_api_still_works() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::default();
        for &b in b"Hello\n" {
            p.process(b, &mut g);
        }
        assert_eq!(g.output, "Hello\n");
    }

    // ---------- Phase-1 safety tests ----------
    #[test]
    fn safety_max_params() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::default();
        let s = format!("\x1B[{}m", (0..50).map(|i| i.to_string()).collect::<Vec<_>>().join(";"));
        p.feed_str(&s, &mut g); // must not panic
    }

    #[test]
    fn safety_max_osc() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::default();
        let big = format!("\x1B]0;{}\x07", "x".repeat(10_000));
        p.feed_str(&big, &mut g); // must not panic
    }

    #[test]
    fn clear_modes() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::default();
        p.feed_str("\x1B[0J\x1B[1J\x1B[2J\x1B[0K\x1B[1K\x1B[2K", &mut g);
    }

    #[test]
    fn cursor_save_restore_esc() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::default();
        // ESC 7 and ESC 8 (DEC style)
        p.feed_str("\x1B7\x1B8", &mut g);
    }

    #[test]
    fn cursor_save_restore_csi() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::default();
        // CSI s and CSI u (SCO style)
        p.feed_str("\x1B[s\x1B[u", &mut g);
    }

    #[test]
    fn cursor_visibility() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::default();
        // Show and hide cursor
        p.feed_str("\x1B[?25h\x1B[?25l", &mut g);
    }

    #[test]
    fn scrolling_operations() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::default();
        // Scroll up and down
        p.feed_str("\x1B[S\x1B[3S\x1B[T\x1B[2T", &mut g);
    }

    #[test]
    fn sgr_reset() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();
        
        // Set some attributes
        p.feed_str("\x1B[1;4;31m", &mut g);
        assert!(g.bold);
        assert!(g.underline);
        
        // Reset
        p.feed_str("\x1B[0m", &mut g);
        assert!(!g.bold);
        assert!(!g.underline);
        assert_eq!(g.fg, Color::default());
    }

    #[test]
    fn sgr_text_attributes() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();
        
        // Bold
        p.feed_str("\x1B[1m", &mut g);
        assert!(g.bold);
        
        // Italic
        p.feed_str("\x1B[3m", &mut g);
        assert!(g.italic);
        
        // Underline
        p.feed_str("\x1B[4m", &mut g);
        assert!(g.underline);
        
        // Dim
        p.feed_str("\x1B[2m", &mut g);
        assert!(g.dim);
    }

    #[test]
    fn sgr_reset_specific_attributes() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();
        
        // Set bold and dim
        p.feed_str("\x1B[1;2m", &mut g);
        assert!(g.bold);
        assert!(g.dim);
        
        // Reset bold/dim (SGR 22)
        p.feed_str("\x1B[22m", &mut g);
        assert!(!g.bold);
        assert!(!g.dim);
        
        // Set italic
        p.feed_str("\x1B[3m", &mut g);
        assert!(g.italic);
        
        // Reset italic (SGR 23)
        p.feed_str("\x1B[23m", &mut g);
        assert!(!g.italic);
        
        // Set underline
        p.feed_str("\x1B[4m", &mut g);
        assert!(g.underline);
        
        // Reset underline (SGR 24)
        p.feed_str("\x1B[24m", &mut g);
        assert!(!g.underline);
    }

    #[test]
    fn sgr_standard_foreground_colors() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();
        
        // Black
        p.feed_str("\x1B[30m", &mut g);
        assert_eq!(g.fg, ansi_color(0));
        
        // Red
        p.feed_str("\x1B[31m", &mut g);
        assert_eq!(g.fg, ansi_color(1));
        
        // Green
        p.feed_str("\x1B[32m", &mut g);
        assert_eq!(g.fg, ansi_color(2));
        
        // Yellow
        p.feed_str("\x1B[33m", &mut g);
        assert_eq!(g.fg, ansi_color(3));
        
        // Blue
        p.feed_str("\x1B[34m", &mut g);
        assert_eq!(g.fg, ansi_color(4));
        
        // Magenta
        p.feed_str("\x1B[35m", &mut g);
        assert_eq!(g.fg, ansi_color(5));
        
        // Cyan
        p.feed_str("\x1B[36m", &mut g);
        assert_eq!(g.fg, ansi_color(6));
        
        // White
        p.feed_str("\x1B[37m", &mut g);
        assert_eq!(g.fg, ansi_color(7));
    }

    #[test]
    fn sgr_standard_background_colors() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();
        
        // Red background
        p.feed_str("\x1B[41m", &mut g);
        assert_eq!(g.bg, ansi_color(1));
        
        // Blue background
        p.feed_str("\x1B[44m", &mut g);
        assert_eq!(g.bg, ansi_color(4));
        
        // White background
        p.feed_str("\x1B[47m", &mut g);
        assert_eq!(g.bg, ansi_color(7));
    }

    #[test]
    fn sgr_bright_foreground_colors() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();
        
        // Bright red
        p.feed_str("\x1B[91m", &mut g);
        assert_eq!(g.fg, ansi_bright_color(1));
        
        // Bright green
        p.feed_str("\x1B[92m", &mut g);
        assert_eq!(g.fg, ansi_bright_color(2));
        
        // Bright yellow
        p.feed_str("\x1B[93m", &mut g);
        assert_eq!(g.fg, ansi_bright_color(3));
    }

    #[test]
    fn sgr_bright_background_colors() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();
        
        // Bright red background
        p.feed_str("\x1B[101m", &mut g);
        assert_eq!(g.bg, ansi_bright_color(1));
        
        // Bright blue background
        p.feed_str("\x1B[104m", &mut g);
        assert_eq!(g.bg, ansi_bright_color(4));
    }

    #[test]
    fn sgr_256_color_foreground() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();
        
        // 256-color mode: ESC[38;5;n m
        p.feed_str("\x1B[38;5;196m", &mut g); // Bright red
        assert_eq!(g.fg, ansi_256_color(196));
        
        p.feed_str("\x1B[38;5;21m", &mut g); // Blue
        assert_eq!(g.fg, ansi_256_color(21));
        
        p.feed_str("\x1B[38;5;240m", &mut g); // Gray
        assert_eq!(g.fg, ansi_256_color(240));
    }

    #[test]
    fn sgr_256_color_background() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();
        
        // 256-color mode: ESC[48;5;n m
        p.feed_str("\x1B[48;5;196m", &mut g);
        assert_eq!(g.bg, ansi_256_color(196));
        
        p.feed_str("\x1B[48;5;21m", &mut g);
        assert_eq!(g.bg, ansi_256_color(21));
    }

    #[test]
    fn sgr_rgb_foreground() {
        const EPS: f64 = 1e-10;
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();
        
        // RGB mode: ESC[38;2;r;g;b m
        p.feed_str("\x1B[38;2;255;128;0m", &mut g);

        let expected = Color::rgb(1.0, 128.0/255.0, 0.0);
        assert!((g.fg.r - expected.r).abs() < EPS);
        assert!((g.fg.g - expected.g).abs() < EPS);
        assert!((g.fg.b - expected.b).abs() < EPS);
    }

    #[test]
    fn sgr_rgb_background() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();
        
        // RGB mode: ESC[48;2;r;g;b m
        p.feed_str("\x1B[48;2;64;128;255m", &mut g);
        let expected = Color::rgb(64.0/255.0, 128.0/255.0, 1.0);
        assert!((g.bg.r - expected.r).abs() < 0.01);
        assert!((g.bg.g - expected.g).abs() < 0.01);
        assert!((g.bg.b - expected.b).abs() < 0.01);
    }

    #[test]
    fn sgr_default_colors() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();
        
        // Set custom colors
        p.feed_str("\x1B[31;44m", &mut g);
        assert_eq!(g.fg, ansi_color(1));
        assert_eq!(g.bg, ansi_color(4));
        
        // Reset to default foreground
        p.feed_str("\x1B[39m", &mut g);
        assert_eq!(g.fg, Color::default());
        
        // Reset to default background
        p.feed_str("\x1B[49m", &mut g);
        assert_eq!(g.bg, Color::rgb(0.0, 0.0, 0.0));
    }

    #[test]
    fn sgr_combined_attributes() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();
        
        // Multiple attributes in one sequence
        p.feed_str("\x1B[1;4;31;44m", &mut g);
        assert!(g.bold);
        assert!(g.underline);
        assert_eq!(g.fg, ansi_color(1)); // Red
        assert_eq!(g.bg, ansi_color(4)); // Blue
        
        // Reset and set different combo
        p.feed_str("\x1B[0;3;92;103m", &mut g);
        assert!(!g.bold);
        assert!(!g.underline);
        assert!(g.italic);
        assert_eq!(g.fg, ansi_bright_color(2)); // Bright green
        assert_eq!(g.bg, ansi_bright_color(3)); // Bright yellow
    }

    #[test]
    fn sgr_empty_resets() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();
        
        // Set some attributes
        p.feed_str("\x1B[1;31m", &mut g);
        assert!(g.bold);
        
        // Empty SGR should reset
        p.feed_str("\x1B[m", &mut g);
        assert!(!g.bold);
        assert_eq!(g.fg, Color::default());
    }

    #[test]
    fn sgr_multiple_sequences() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();
        
        // Apply multiple sequences
        p.feed_str("\x1B[1m\x1B[31m\x1B[44m", &mut g);
        assert!(g.bold);
        assert_eq!(g.fg, ansi_color(1));
        assert_eq!(g.bg, ansi_color(4));
    }

    #[test]
    fn sgr_with_text() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();
        
        // Colored text
        p.feed_str("\x1B[1;31mHello\x1B[0m World", &mut g);
        assert_eq!(g.output, "Hello World");
    }

    #[test]
    fn sgr_256_color_ranges() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();
        
        // Test standard colors (0-7)
        p.feed_str("\x1B[38;5;7m", &mut g);
        assert_eq!(g.fg, ansi_color(7));
        
        // Test bright colors (8-15)
        p.feed_str("\x1B[38;5;15m", &mut g);
        assert_eq!(g.fg, ansi_bright_color(7));
        
        // Test 216 color cube (16-231)
        p.feed_str("\x1B[38;5;16m", &mut g);
        let expected = Color::rgb(0.0, 0.0, 0.0);
        assert_eq!(g.fg, expected);
        
        // Test grayscale (232-255)
        p.feed_str("\x1B[38;5;232m", &mut g);
        let gray = Color::rgb(0.0, 0.0, 0.0);
        assert!((g.fg.r - gray.r).abs() < 0.01);
    }

    #[test]
    fn sgr_rgb_clamping() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();
        
        // Test that values > 255 are clamped
        p.feed_str("\x1B[38;2;300;128;500m", &mut g);
        assert_eq!(g.fg.r, 1.0); // 255/255 = 1.0
        assert!((g.fg.g - 128.0/255.0).abs() < 0.01);
        assert_eq!(g.fg.b, 1.0); // 255/255 = 1.0
    }

    #[test]
    fn sgr_incomplete_sequences() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();
        
        // Incomplete 256-color (missing index)
        p.feed_str("\x1B[38;5m", &mut g);
        // Should not crash, just ignore
        
        // Incomplete RGB (missing components)
        p.feed_str("\x1B[38;2;100m", &mut g);
        // Should not crash, just ignore
        
        p.feed_str("\x1B[38;2;100;200m", &mut g);
        // Should not crash, just ignore
    }

}
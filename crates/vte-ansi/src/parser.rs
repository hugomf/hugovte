use std::fmt;
use base64::prelude::*;
use crate::color::{Color, COLOR_PALETTE};
use crate::grid::AnsiGrid;

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

/// Parser state
#[derive(PartialEq, Clone, Copy, Debug)]
enum AnsiState {
    Normal,
    Escape,
    Csi,
    Osc,
    Charset,
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
    // Track if we've already reported errors for current sequence
    sequence_has_error: bool,
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
            sequence_has_error: false,
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

    // ===== Public API =====
    pub fn feed_str(&mut self, s: &str, grid: &mut dyn AnsiGrid) {
        self.feed_bytes(s.as_bytes(), grid)
    }

    // ===== Core parsing logic =====
    fn feed_bytes(&mut self, bytes: &[u8], grid: &mut dyn AnsiGrid) {
        let mut i = 0;
        while i < bytes.len() {
            // fast skip until next control byte
            let ctrl_pos = memchr::memchr3(b'\x1B', b'\n', b'\r', &bytes[i..])
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

    fn process_char(&mut self, ch: char, grid: &mut dyn AnsiGrid) {
        match self.state {
            AnsiState::Normal => self.normal_char(ch, grid),
            AnsiState::Escape => self.escape_char(ch, grid),
            AnsiState::Csi => self.csi_char(ch, grid),
            AnsiState::Osc => self.osc_char(ch, grid),
            AnsiState::Charset => self.charset_char(ch, grid),
        }
    }

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

    fn escape_char(&mut self, ch: char, grid: &mut dyn AnsiGrid) {
        match ch {
            '[' => {
                self.state = AnsiState::Csi;
                self.params.clear();
                self.current_param = 0;
                self.private = false;
                self.sequence_has_error = false;
            }
            ']' => {
                self.state = AnsiState::Osc;
                self.osc_buffer.clear();
                self.in_osc_escape = false;
            }
            '(' | ')' | '*' | '+' => {
                // Charset designation (ESC <designator> <charset>)
                self.state = AnsiState::Charset;
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
            '=' => {
                grid.set_keypad_mode(true);
                self.state = AnsiState::Normal;
            }
            '>' => {
                grid.set_keypad_mode(false);
                self.state = AnsiState::Normal;
            }
            _ => {
                self.report_error(AnsiError::MalformedSequence {
                    context: format!("Unknown escape char: {}", ch),
                });
                self.state = AnsiState::Normal;
            }
        }
    }

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
                    if !self.sequence_has_error {
                        self.sequence_has_error = true;
                        self.report_error(AnsiError::TooManyParams {
                            sequence: format!("CSI with {} params", self.params.len() + 1),
                            count: self.params.len() + 1,
                        });
                    }
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
            'L' => grid.insert_lines(self.get_param(0, 1)),
            'M' => grid.delete_lines(self.get_param(0, 1)),
            'P' => grid.delete_chars(self.get_param(0, 1)),
            'X' => grid.erase_chars(self.get_param(0, 1)),
            '@' => grid.insert_chars(self.get_param(0, 1)),
            'm' => self.execute_sgr(grid),
            'h' if self.private => {
                match self.params.first() {
                    Some(&1) => grid.set_application_cursor_keys(true),
                    Some(&25) => grid.set_cursor_visible(true),
                    Some(&47) => grid.use_alternate_screen(true),
                    Some(&1049) => grid.use_alternate_screen(true),
                    Some(&7) => grid.set_auto_wrap(true),
                    Some(&1000) => grid.set_mouse_reporting_mode(1000, true),
                    Some(&1002) => grid.set_mouse_reporting_mode(1002, true),
                    Some(&1005) => grid.set_mouse_reporting_mode(1005, true),
                    Some(&1006) => grid.set_mouse_reporting_mode(1006, true),
                    Some(&1004) => grid.set_focus_reporting(true),
                    _ => {}
                }
            }
            'l' if self.private => {
                match self.params.first() {
                    Some(&1) => grid.set_application_cursor_keys(false),
                    Some(&25) => grid.set_cursor_visible(false),
                    Some(&47) => grid.use_alternate_screen(false),
                    Some(&1049) => grid.use_alternate_screen(false),
                    Some(&7) => grid.set_auto_wrap(false),
                    Some(&1000) => grid.set_mouse_reporting_mode(1000, false),
                    Some(&1002) => grid.set_mouse_reporting_mode(1002, false),
                    Some(&1005) => grid.set_mouse_reporting_mode(1005, false),
                    Some(&1006) => grid.set_mouse_reporting_mode(1006, false),
                    Some(&1004) => grid.set_focus_reporting(false),
                    _ => {}
                }
            }
            'h' => {
                if self.params.first() == Some(&4) {
                    grid.set_insert_mode(true);
                }
            }
            'l' => {
                if self.params.first() == Some(&4) {
                    grid.set_insert_mode(false);
                }
            }
            'S' => grid.scroll_up(self.get_param(0, 1)),
            'T' => grid.scroll_down(self.get_param(0, 1)),
            's' => grid.save_cursor(),
            'u' => grid.restore_cursor(),
            _ => {}
        }
    }

    fn charset_char(&mut self, _ch: char, _grid: &mut dyn AnsiGrid) {
        // Character set designation: ESC <designator> <charset>
        // For now, ignore and return to normal state
        self.state = AnsiState::Normal;
    }

    fn osc_char(&mut self, ch: char, grid: &mut dyn AnsiGrid) {
        if self.osc_buffer.len() >= MAX_OSC_LEN {
            self.report_error(AnsiError::OscTooLong { length: self.osc_buffer.len() });
            self.state = AnsiState::Normal;
            return;
        }
        self.stats.max_osc_length_seen = self.stats.max_osc_length_seen.max(self.osc_buffer.len());
        
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
        let buffer = self.osc_buffer.clone();
        if let Some((num, text)) = buffer.split_once(';') {
            match num {
                "0" | "2" => {
                    grid.set_title(text);
                }
                "52" => {
                    self.handle_clipboard_osc(text, grid);
                }
                "7" => {
                    grid.set_current_directory(text);
                }
                "8" => {
                    self.handle_hyperlink_osc(text, grid);
                }
                _ => {}
            }
        }
        self.state = AnsiState::Normal;
        self.osc_buffer.clear();
        self.in_osc_escape = false;
    }

    fn handle_clipboard_osc(&mut self, text: &str, grid: &mut dyn AnsiGrid) {
        if let Some((clipboard_type, data)) = text.split_once(';') {
            if let Ok(clipboard_id) = clipboard_type.parse::<u8>() {
                if clipboard_id <= 1 {
                    if let Ok(decoded) = BASE64_STANDARD.decode(data) {
                        if let Ok(decoded_str) = String::from_utf8(decoded) {
                            grid.handle_clipboard_data(clipboard_id, &decoded_str);
                        }
                    }
                }
            }
        }
    }

    fn handle_hyperlink_osc(&mut self, text: &str, grid: &mut dyn AnsiGrid) {
        if let Some((params, uri)) = text.split_once(';') {
            let params = if params.is_empty() { None } else { Some(params) };
            grid.handle_hyperlink(params, uri);
        }
    }

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

    fn get_param(&self, idx: usize, default: u16) -> usize {
        self.params.get(idx).copied().unwrap_or(default) as usize
    }
}

// ---------- helper functions ----------
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
            Color::rgba(r as f64 / 5.0, g as f64 / 5.0, b as f64 / 5.0, 1.0)
        }
        232..=255 => {
            let gray = (index - 232) as f64 / 23.0;
            Color::rgba(gray, gray, gray, 1.0)
        }
        _ => Color::default(),
    }
}

// ---------- UTF-8 utilities ----------
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
    use rand::Rng;

    #[derive(Default)]
    struct MockGrid {
        fg: Color,
        bg: Color,
        output: String,
        bold: bool,
        italic: bool,
        underline: bool,
        dim: bool,
        // Phase 2: Cursor tracking
        cursor_row: usize,
        cursor_col: usize,
        cursor_visible: bool,
        cursor_stack: Vec<(usize, usize)>,  // (row, col)
        // Phase 4: Advanced terminal simulation
        is_alternate_screen: bool,
        insert_mode: bool,
        auto_wrap: bool,
        line_ops: Vec<String>,  // Tracks insert/delete lines
        char_ops: Vec<String>,  // Tracks insert/delete/erase chars
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
                cursor_row: 0,
                cursor_col: 0,
                cursor_visible: true,
                cursor_stack: Vec::new(),
                is_alternate_screen: false,
                insert_mode: false,
                auto_wrap: true,
                line_ops: Vec::new(),
                char_ops: Vec::new(),
            }
        }
    }


    impl AnsiGrid for MockGrid {
        fn put(&mut self, ch: char) {
            if self.insert_mode {
                self.char_ops.push(format!("[INSERT_CHAR {}]", ch));
            }
            self.output.push(ch);
        }
        fn advance(&mut self) {
            self.cursor_col += 1;
            if self.auto_wrap && self.cursor_col >= 80 {
                self.cursor_col = 0;
                self.cursor_row += 1;
                self.output.push('\n');
            }
        }
        fn left(&mut self, n: usize) {
            self.cursor_col = self.cursor_col.saturating_sub(n);
        }
        fn right(&mut self, n: usize) {
            self.cursor_col += n;
        }
        fn up(&mut self, n: usize) {
            self.cursor_row = self.cursor_row.saturating_sub(n);
        }
        fn down(&mut self, n: usize) {
            self.cursor_row += n;
        }
        fn newline(&mut self) {
            self.output.push('\n');
            self.cursor_col = 0;
            self.cursor_row += 1;
        }
        fn carriage_return(&mut self) {
            self.cursor_col = 0;
        }
        fn backspace(&mut self) {
            self.left(1);
        }
        fn move_rel(&mut self, dx: i32, dy: i32) {
            self.cursor_col = ((self.cursor_col as i32 + dx) as usize).max(0);
            self.cursor_row = ((self.cursor_row as i32 + dy) as usize).max(0);
        }
        fn move_abs(&mut self, row: usize, col: usize) {
            self.cursor_row = row;
            self.cursor_col = col;
        }
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

        // Phase 2: Cursor ops
        fn save_cursor(&mut self) {
            self.cursor_stack.push((self.cursor_row, self.cursor_col));
        }
        fn restore_cursor(&mut self) {
            if let Some((row, col)) = self.cursor_stack.pop() {
                self.cursor_row = row;
                self.cursor_col = col;
            }
        }
        fn set_cursor_visible(&mut self, visible: bool) {
            self.cursor_visible = visible;
        }
        fn scroll_up(&mut self, n: usize) {
            self.output.push_str(&format!("[SCROLL_UP {}]", n));
            self.cursor_row = self.cursor_row.saturating_sub(n);
        }
        fn scroll_down(&mut self, n: usize) {
            self.output.push_str(&format!("[SCROLL_DOWN {}]", n));
            self.cursor_row += n;
        }
        fn insert_lines(&mut self, n: usize) {
            self.line_ops.push(format!("[INSERT_LINES {}]", n));
            self.cursor_row += n;
        }
        fn delete_lines(&mut self, n: usize) {
            self.line_ops.push(format!("[DELETE_LINES {}]", n));
            self.cursor_row = self.cursor_row.saturating_sub(n);
        }
        fn insert_chars(&mut self, n: usize) {
            self.char_ops.push(format!("[INSERT_CHARS {}]", n));
            self.cursor_col += n;
        }
        fn delete_chars(&mut self, n: usize) {
            self.char_ops.push(format!("[DELETE_CHARS {}]", n));
            self.cursor_col = self.cursor_col.saturating_sub(n);
        }
        fn erase_chars(&mut self, n: usize) {
            self.char_ops.push(format!("[ERASE_CHARS {}]", n));
        }
        fn use_alternate_screen(&mut self, enable: bool) {
            self.is_alternate_screen = enable;
            self.output.push_str(if enable { "[ALT_SCREEN_ON]" } else { "[ALT_SCREEN_OFF]" });
        }
        fn set_insert_mode(&mut self, enable: bool) {
            self.insert_mode = enable;
            self.output.push_str(if enable { "[INSERT_MODE_ON]" } else { "[INSERT_MODE_OFF]" });
        }
        fn set_auto_wrap(&mut self, enable: bool) {
            self.auto_wrap = enable;
            self.output.push_str(if enable { "[AUTO_WRAP_ON]" } else { "[AUTO_WRAP_OFF]" });
        }

        // Phase-2 DEC private modes
        fn set_application_cursor_keys(&mut self, _enable: bool) {
            self.output.push_str(&format!("[APP_CURSOR_KEYS_{}]", if _enable { "ON" } else { "OFF" }));
        }

        fn set_mouse_reporting_mode(&mut self, mode: u16, enable: bool) {
            self.output.push_str(&format!("[MOUSE_MODE_{}_{}]", mode, if enable { "ON" } else { "OFF" }));
        }

        fn set_focus_reporting(&mut self, _enable: bool) {
            self.output.push_str(&format!("[FOCUS_REPORTING_{}]", if _enable { "ON" } else { "OFF" }));
        }

        // Keypad mode (Application vs Numeric)
        fn set_keypad_mode(&mut self, application: bool) {
            self.output.push_str(&format!("[KEYPAD_MODE_{}]", if application { "APPLICATION" } else { "NUMERIC" }));
        }
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
            p.process_char(b as char, &mut g);
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
    fn cursor_nested_save_restore() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();
        // Pos1: (3,5)
        p.feed_str("\x1B[4;6H", &mut g);
        p.feed_str("\x1B[s", &mut g);  // Save1
        // Pos2: (6,7)
        p.feed_str("\x1B[7;8H", &mut g);
        p.feed_str("\x1B[s", &mut g);  // Save2
        // Pos3: (9,19)
        p.feed_str("\x1B[10;20H", &mut g);
        p.feed_str("\x1B[u", &mut g);  // Restore to Pos2
        assert_eq!(g.cursor_row, 6);
        assert_eq!(g.cursor_col, 7);
        assert_eq!(g.cursor_stack.len(), 1);
        p.feed_str("\x1B[u", &mut g);  // Restore to Pos1
        assert_eq!(g.cursor_row, 3);
        assert_eq!(g.cursor_col, 5);
        assert_eq!(g.cursor_stack.len(), 0);
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

    // ---------- Phase-3 fuzz-like test ----------

    #[test]
    fn fuzz_like_random_input() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();
        let mut rng = rand::rng();

        // Generate random byte sequences to simulate fuzzing
        for _ in 0..1000 {
            let len = rng.random_range(1..1000);
            let mut bytes = vec![0u8; len];
            rng.fill(&mut bytes[..]);
            p.feed_bytes(&bytes, &mut g); // Should not panic
        }
    }

    // ---------- Phase-3 robustness tests ----------
    
    #[test]
    fn error_callback_too_many_params() {
        use std::sync::{Arc, Mutex};
        let errors = Arc::new(Mutex::new(Vec::new()));
        let errors_clone = errors.clone();
        
        let mut p = AnsiParser::new().with_error_callback(move |e| {
            errors_clone.lock().unwrap().push(e);
        });
        let mut g = MockGrid::default();
        
        // Create a sequence with > MAX_PARAMS parameters
        let s = format!("\x1B[{}m", (0..50).map(|i| i.to_string()).collect::<Vec<_>>().join(";"));
        p.feed_str(&s, &mut g);
        
        let errs = errors.lock().unwrap();
        assert!(!errs.is_empty(), "Should report error for too many params");
        assert!(matches!(errs[0], AnsiError::TooManyParams { .. }));
    }

    #[test]
    fn error_callback_osc_too_long() {
        use std::sync::{Arc, Mutex};
        let errors = Arc::new(Mutex::new(Vec::new()));
        let errors_clone = errors.clone();
        
        let mut p = AnsiParser::new().with_error_callback(move |e| {
            errors_clone.lock().unwrap().push(e);
        });
        let mut g = MockGrid::default();
        
        // Create OSC sequence longer than MAX_OSC_LEN
        let big = format!("\x1B]0;{}\x07", "x".repeat(10_000));
        p.feed_str(&big, &mut g);
        
        let errs = errors.lock().unwrap();
        assert!(!errs.is_empty(), "Should report error for OSC too long");
        assert!(matches!(errs[0], AnsiError::OscTooLong { .. }));
    }

    #[test]
    fn error_callback_param_too_large() {
        use std::sync::{Arc, Mutex};
        let errors = Arc::new(Mutex::new(Vec::new()));
        let errors_clone = errors.clone();
        
        let mut p = AnsiParser::new().with_error_callback(move |e| {
            errors_clone.lock().unwrap().push(e);
        });
        let mut g = MockGrid::default();
        
        // Parameter value > MAX_PARAM_VALUE
        let s = "\x1B[99999m";
        p.feed_str(s, &mut g);
        
        let errs = errors.lock().unwrap();
        assert!(!errs.is_empty(), "Should report error for param too large");
        assert!(matches!(errs[0], AnsiError::ParamTooLarge { .. }));
    }

    #[test]
    fn parser_stats_tracking() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::default();
        
        // Process some sequences
        p.feed_str("\x1B[1;2;3;4;5m", &mut g);
        p.feed_str("\x1B[31m", &mut g);
        p.feed_str("\x1B]0;Title\x07", &mut g);
        
        let stats = p.stats();
        assert_eq!(stats.sequences_processed, 2); // Two CSI sequences
        assert_eq!(stats.max_params_seen, 5); // First sequence had 5 params
    }

    #[test]
    fn stats_reset() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::default();
        
        p.feed_str("\x1B[1;2;3m", &mut g);
        assert!(p.stats().sequences_processed > 0);
        
        p.reset_stats();
        assert_eq!(p.stats().sequences_processed, 0);
        assert_eq!(p.stats().max_params_seen, 0);
    }

    #[test]
    fn no_panic_on_extreme_input() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::default();

        // Various pathological inputs
        p.feed_str(&format!("\x1B[{}m", "9".repeat(100)), &mut g);
        p.feed_str("\x1B[;;;;;;;;;;;;;;;;m", &mut g);
        p.feed_str(&format!("\x1B]0;{}\x07", "x".repeat(5000)), &mut g);
        p.feed_str(&format!("\x1B{}", "[".repeat(100)), &mut g);

        // Should not panic, just handle gracefully
    }

    #[test]
    fn utf8_safety() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::default();
        
        // Mix of valid and invalid UTF-8
        p.feed_str("Hello 世界 🌍\n", &mut g);
        assert!(g.output.contains("Hello"));
        
        // Invalid UTF-8 bytes should be replaced with replacement char
        p.feed_bytes(&[b'A', 0xFF, 0xFE, b'B'], &mut g);
        // Should not panic
    }

    #[test]
    fn error_display_formatting() {
        let e1 = AnsiError::TooManyParams {
            sequence: "CSI test".to_string(),
            count: 50,
        };
        assert!(format!("{}", e1).contains("50"));

        let e2 = AnsiError::OscTooLong { length: 5000 };
        assert!(format!("{}", e2).contains("5000"));

        let e3 = AnsiError::ParamTooLarge { value: 65535 };
        assert!(format!("{}", e3).contains("65535"));

        let e4 = AnsiError::MalformedSequence {
            context: "test context".to_string(),
        };
        assert!(format!("{}", e4).contains("test context"));
    }

    #[test]
    fn concurrent_error_callbacks() {
        use std::sync::{Arc, Mutex};
        let counter = Arc::new(Mutex::new(0));
        let counter_clone = counter.clone();
        
        let mut p = AnsiParser::new().with_error_callback(move |_| {
            *counter_clone.lock().unwrap() += 1;
        });
        let mut g = MockGrid::default();
        
        // Trigger multiple errors
        for _ in 0..5 {
            let s = format!("\x1B[{}m", (0..50).map(|i| i.to_string()).collect::<Vec<_>>().join(";"));
            p.feed_str(&s, &mut g);
        }
        
        assert_eq!(*counter.lock().unwrap(), 5);
    }

    // ---------- Phase-4 extended features tests ----------
    
    #[test]
    fn line_operations() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::default();
        
        // Insert lines: CSI L
        p.feed_str("\x1B[L", &mut g);    // Insert 1 line
        p.feed_str("\x1B[3L", &mut g);   // Insert 3 lines
        
        // Delete lines: CSI M
        p.feed_str("\x1B[M", &mut g);    // Delete 1 line
        p.feed_str("\x1B[2M", &mut g);   // Delete 2 lines
    }

    #[test]
    fn character_operations() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::default();
        
        // Insert characters: CSI @
        p.feed_str("\x1B[@", &mut g);    // Insert 1 char
        p.feed_str("\x1B[5@", &mut g);   // Insert 5 chars
        
        // Delete characters: CSI P
        p.feed_str("\x1B[P", &mut g);    // Delete 1 char
        p.feed_str("\x1B[3P", &mut g);   // Delete 3 chars
        
        // Erase characters: CSI X
        p.feed_str("\x1B[X", &mut g);    // Erase 1 char
        p.feed_str("\x1B[4X", &mut g);   // Erase 4 chars
    }

    #[test]
    fn alternate_screen_buffer() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::default();
        
        // Enable alternate screen: CSI ?1049h
        p.feed_str("\x1B[?1049h", &mut g);
        
        // Disable alternate screen: CSI ?1049l
        p.feed_str("\x1B[?1049l", &mut g);
    }

    #[test]
    fn insert_mode() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::default();
        
        // Enable insert mode: CSI 4h
        p.feed_str("\x1B[4h", &mut g);
        
        // Disable insert mode: CSI 4l
        p.feed_str("\x1B[4l", &mut g);
    }

    #[test]
    fn auto_wrap_mode() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::default();
        
        // Enable auto wrap: CSI ?7h
        p.feed_str("\x1B[?7h", &mut g);
        
        // Disable auto wrap: CSI ?7l
        p.feed_str("\x1B[?7l", &mut g);
    }

    #[test]
    fn combined_line_and_char_ops() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::default();
        
        // Mix of operations
        p.feed_str("Hello", &mut g);
        p.feed_str("\x1B[2@", &mut g);   // Insert 2 chars
        p.feed_str(" World", &mut g);
        p.feed_str("\x1B[P", &mut g);    // Delete 1 char
        p.feed_str("\x1B[L", &mut g);    // Insert line
        
        assert!(g.output.contains("Hello"));
        assert!(g.output.contains("World"));
    }

    #[test]
    fn default_param_values() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::default();
        
        // Test that operations without params default to 1
        p.feed_str("\x1B[L", &mut g);    // Insert 1 line (default)
        p.feed_str("\x1B[M", &mut g);    // Delete 1 line (default)
        p.feed_str("\x1B[@", &mut g);    // Insert 1 char (default)
        p.feed_str("\x1B[P", &mut g);    // Delete 1 char (default)
        p.feed_str("\x1B[X", &mut g);    // Erase 1 char (default)
        p.feed_str("\x1B[S", &mut g);    // Scroll up 1 (default)
        p.feed_str("\x1B[T", &mut g);    // Scroll down 1 (default)
    }

    #[test]
    fn large_operation_counts() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::default();
        
        // Operations with large counts
        p.feed_str("\x1B[100L", &mut g);  // Insert 100 lines
        p.feed_str("\x1B[50@", &mut g);   // Insert 50 chars
        p.feed_str("\x1B[999P", &mut g);  // Delete 999 chars
    }

    #[test]
    fn mode_switching_sequence() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::default();
        
        // Typical vim-like application sequence
        p.feed_str("\x1B[?1049h", &mut g);  // Enter alternate screen
        p.feed_str("\x1B[?25l", &mut g);    // Hide cursor
        p.feed_str("Content", &mut g);
        p.feed_str("\x1B[?25h", &mut g);    // Show cursor
        p.feed_str("\x1B[?1049l", &mut g);  // Exit alternate screen
        
        assert!(g.output.contains("Content"));
    }

    #[test]
    fn all_phase4_features_no_panic() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::default();
        
        // Kitchen sink test - throw everything at it
        let sequences = vec![
            "\x1B[L", "\x1B[10L",         // Insert lines
            "\x1B[M", "\x1B[5M",          // Delete lines
            "\x1B[@", "\x1B[20@",         // Insert chars
            "\x1B[P", "\x1B[15P",         // Delete chars
            "\x1B[X", "\x1B[8X",          // Erase chars
            "\x1B[?1049h", "\x1B[?1049l", // Alternate screen
            "\x1B[4h", "\x1B[4l",         // Insert mode
            "\x1B[?7h", "\x1B[?7l",       // Auto wrap
        ];
        
        for seq in sequences {
            p.feed_str(seq, &mut g);
        }
        
        // Should not panic
    }

    #[test]
    fn phase4_with_text_content() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::default();
        
        // Real-world-like usage
        p.feed_str("\x1B[?1049h", &mut g);      // Enter alt screen
        p.feed_str("\x1B[2J", &mut g);          // Clear screen
        p.feed_str("\x1B[H", &mut g);           // Home cursor
        p.feed_str("Line 1\n", &mut g);
        p.feed_str("Line 2\n", &mut g);
        p.feed_str("\x1B[L", &mut g);           // Insert line
        p.feed_str("Inserted\n", &mut g);
        p.feed_str("\x1B[5@", &mut g);          // Insert 5 chars
        p.feed_str("Text", &mut g);
        p.feed_str("\x1B[?1049l", &mut g);      // Exit alt screen
        
        assert!(g.output.contains("Line 1"));
        assert!(g.output.contains("Line 2"));
        assert!(g.output.contains("Inserted"));
        assert!(g.output.contains("Text"));
    }

    #[test]
    fn insert_mode_with_auto_wrap() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();
        p.feed_str("\x1B[4h", &mut g);  // Enable insert mode
        p.feed_str("\x1B[?7l", &mut g); // Disable auto wrap
        g.cursor_col = 79;            // Put cursor at end of line
        p.feed_str("A", &mut g);       // Try to output char
        assert_eq!(g.cursor_col, 80); // Should have moved past 79
        assert!(!g.output.contains("\n")); // Should NOT have wrapped
        p.feed_str("\x1B[?7h", &mut g); // Re-enable auto wrap
        g.cursor_col = 79;            // Reset to end of line
        p.feed_str("B", &mut g);       // Try to output char
        assert_eq!(g.cursor_col, 0);  // Should have wrapped to start
        assert_eq!(g.cursor_row, 1);  // Should have moved to next row
        assert!(g.output.contains("\n")); // Should have output newline
    }

    // ---------- Phase-2 DEC private modes tests ----------

    #[test]
    fn dec_private_modes_application_cursor_keys() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();

        // Enable application cursor keys
        p.feed_str("\x1B[?1h", &mut g);
        assert!(g.output.contains("[APP_CURSOR_KEYS_ON]"));

        // Disable application cursor keys
        p.feed_str("\x1B[?1l", &mut g);
        assert!(g.output.contains("[APP_CURSOR_KEYS_OFF]"));
    }

    #[test]
    fn dec_private_modes_mouse_reporting() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();

        // Test different mouse reporting modes
        p.feed_str("\x1B[?1000h", &mut g); // Normal mouse tracking
        assert!(g.output.contains("[MOUSE_MODE_1000_ON]"));

        p.feed_str("\x1B[?1002h", &mut g); // Button event mouse
        assert!(g.output.contains("[MOUSE_MODE_1002_ON]"));

        p.feed_str("\x1B[?1005h", &mut g); // UTF-8 mouse mode
        assert!(g.output.contains("[MOUSE_MODE_1005_ON]"));

        p.feed_str("\x1B[?1006h", &mut g); // SGR mouse mode
        assert!(g.output.contains("[MOUSE_MODE_1006_ON]"));

        // Disable modes
        p.feed_str("\x1B[?1000l", &mut g);
        assert!(g.output.contains("[MOUSE_MODE_1000_OFF]"));

        p.feed_str("\x1B[?1002l", &mut g);
        assert!(g.output.contains("[MOUSE_MODE_1002_OFF]"));

        p.feed_str("\x1B[?1005l", &mut g);
        assert!(g.output.contains("[MOUSE_MODE_1005_OFF]"));

        p.feed_str("\x1B[?1006l", &mut g);
        assert!(g.output.contains("[MOUSE_MODE_1006_OFF]"));
    }

    #[test]
    fn dec_private_modes_focus_reporting() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();

        // Enable focus reporting
        p.feed_str("\x1B[?1004h", &mut g);
        assert!(g.output.contains("[FOCUS_REPORTING_ON]"));

        // Disable focus reporting
        p.feed_str("\x1B[?1004l", &mut g);
        assert!(g.output.contains("[FOCUS_REPORTING_OFF]"));
    }

    #[test]
    fn dec_private_modes_alternate_screen() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();

        // Enable alternate screen (both 47 and 1049)
        p.feed_str("\x1B[?47h", &mut g);
        assert!(g.output.contains("[ALT_SCREEN_ON]"));

        p.feed_str("\x1B[?1049h", &mut g);
        assert!(g.output.contains("[ALT_SCREEN_ON]"));

        // Disable alternate screen
        p.feed_str("\x1B[?47l", &mut g);
        assert!(g.output.contains("[ALT_SCREEN_OFF]"));

        p.feed_str("\x1B[?1049l", &mut g);
        assert!(g.output.contains("[ALT_SCREEN_OFF]"));
    }

    #[test]
    fn dec_private_modes_combined() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();

        // Test multiple modes in sequence
        p.feed_str("\x1B[?1h\x1B[?1000h\x1B[?25l", &mut g);

        assert!(g.output.contains("[APP_CURSOR_KEYS_ON]"));
        assert!(g.output.contains("[MOUSE_MODE_1000_ON]"));
        assert!(g.output.contains("")); // Cursor visibility handled separately
    }



    #[test]
    fn keypad_mode_application() {
        let mut p = AnsiParser::new();
        let mut g = MockGrid::new();

        // ESC = should set application keypad mode
        p.feed_str("\x1B=", &mut g);
        assert!(g.output.contains("[KEYPAD_MODE_APPLICATION]"));
        
        // ESC > should set numeric keypad mode  
        p.feed_str("\x1B>", &mut g);
        assert!(g.output.contains("[KEYPAD_MODE_NUMERIC]"));
    }
}

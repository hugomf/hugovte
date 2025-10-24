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

    // ===== Internal processing logic =====
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

    // ===== Character processing =====
    fn process_char(&mut self, ch: char, grid: &mut dyn AnsiGrid) {
        match self.state {
            AnsiState::Normal => self.normal_char(ch, grid),
            AnsiState::Escape => self.escape_char(ch, grid),
            AnsiState::Csi => self.csi_char(ch, grid),
            AnsiState::Osc => self.osc_char(ch, grid),
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

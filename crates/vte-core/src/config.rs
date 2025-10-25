// src/config.rs
use crate::ansi::Color;
use crate::constants::{DEFAULT_FONT_SIZE, DEFAULT_FONT_FAMILY, SCROLLBACK_LIMIT,
                      CURSOR_BLINK_INTERVAL_MS, CLICK_TIMEOUT_MS, DEFAULT_FG, DEFAULT_BG,
                      DEFAULT_BOLD_IS_BRIGHT};

#[derive(Clone, Debug)]
pub struct TerminalConfig {
    pub font_size: f64,
    pub font_family: String,
    pub scrollback_limit: usize,
    pub cursor_blink_interval_ms: u64,
    pub click_timeout_ms: u128,
    pub default_fg: Color,
    pub default_bg: Color,
    pub enable_cursor_blink: bool,
    pub enable_selection: bool,
    pub draw_grid_lines: bool,
    pub grid_line_alpha: f64,
    /// Legacy compatibility: bold also makes colors bright (ANSI 8-15 instead of 0-7)
    pub bold_is_bright: bool,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            font_size: DEFAULT_FONT_SIZE,
            font_family: DEFAULT_FONT_FAMILY.to_string(),
            scrollback_limit: SCROLLBACK_LIMIT,
            cursor_blink_interval_ms: CURSOR_BLINK_INTERVAL_MS,
            click_timeout_ms: CLICK_TIMEOUT_MS,
            default_fg: DEFAULT_FG,
            default_bg: DEFAULT_BG,
            enable_cursor_blink: true,
            enable_selection: true,
            draw_grid_lines: false,
            grid_line_alpha: 0.8,
            bold_is_bright: DEFAULT_BOLD_IS_BRIGHT,
        }
    }
}

impl TerminalConfig {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_font_size(mut self, size: f64) -> Self {
        self.font_size = size;
        self
    }
    
    pub fn with_font_family(mut self, family: &str) -> Self {
        self.font_family = family.to_string();
        self
    }
    
    pub fn with_background_color(mut self, color: Color) -> Self {
        self.default_bg = color;
        self
    }
    
    pub fn with_foreground_color(mut self, color: Color) -> Self {
        self.default_fg = color;
        self
    }
    
    pub fn with_colors(mut self, fg: Color, bg: Color) -> Self {
        self.default_fg = fg;
        self.default_bg = bg;
        self
    }
    
    pub fn with_grid_lines(mut self, enabled: bool) -> Self {
        self.draw_grid_lines = enabled;
        self
    }
    
    pub fn with_grid_line_alpha(mut self, alpha: f64) -> Self {
        self.grid_line_alpha = alpha.clamp(0.0, 1.0);
        self
    }
}

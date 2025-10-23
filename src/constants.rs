// src/constants.rs
use crate::ansi::Color;

// Display constants
// pub const DEFAULT_FONT_SIZE: f64 = 14.0;
// pub const DEFAULT_FONT_FAMILY: &str = "Monospace";

pub const DEFAULT_FONT_SIZE: f64 = 12.0;
pub const DEFAULT_FONT_FAMILY: &str = "MenloLGS NF";

pub const SCROLLBACK_LIMIT: usize = 1000;
pub const TAB_WIDTH: usize = 4;

// Timing constants
pub const CURSOR_BLINK_INTERVAL_MS: u64 = 500;
pub const CLICK_TIMEOUT_MS: u128 = 200;

// Color constants - with transparency support
pub const DEFAULT_FG: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
pub const DEFAULT_BG: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }; // Fully transparent by default
pub const SELECTION_BG: Color = Color { r: 0.3, g: 0.5, b: 0.8, a: 0.7 }; // Semi-transparent selection
pub const GRID_LINE_COLOR: Color = Color { r: 0.2, g: 0.0, b: 0.0, a: 0.3 };

// 16-color ANSI palette
pub const COLOR_PALETTE: [Color; 16] = [
    // Basic 8 colors
    Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },      // Black
    Color { r: 0.8, g: 0.0, b: 0.0, a: 1.0 },      // Red
    Color { r: 0.0, g: 0.8, b: 0.0, a: 1.0 },      // Green
    Color { r: 0.8, g: 0.8, b: 0.0, a: 1.0 },      // Yellow
    Color { r: 0.0, g: 0.0, b: 0.8, a: 1.0 },      // Blue
    Color { r: 0.8, g: 0.0, b: 0.8, a: 1.0 },      // Magenta
    Color { r: 0.0, g: 0.8, b: 0.8, a: 1.0 },      // Cyan
    Color { r: 0.8, g: 0.8, b: 0.8, a: 1.0 },      // White
    // Bright colors
    Color { r: 0.4, g: 0.4, b: 0.4, a: 1.0 },      // Bright Black (Gray)
    Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 },      // Bright Red
    Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 },      // Bright Green
    Color { r: 1.0, g: 1.0, b: 0.0, a: 1.0 },      // Bright Yellow
    Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 },      // Bright Blue
    Color { r: 1.0, g: 0.0, b: 1.0, a: 1.0 },      // Bright Magenta
    Color { r: 0.0, g: 1.0, b: 1.0, a: 1.0 },      // Bright Cyan
    Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 },      // Bright White
];
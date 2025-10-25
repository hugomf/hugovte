//! VTE Core - GTK-agnostic virtual terminal emulator core
//!
//! This crate provides the core functionality for a terminal emulator,
//! independent of any specific UI backend.

pub mod ansi;
pub mod config;
pub mod constants;
pub mod drawing;
pub mod dummy_backend;
pub mod error;
pub mod grid;
pub mod input;
pub mod security;
pub mod selection;
pub mod terminal;

// Re-export main types
pub use ansi::{AnsiParser, AnsiGrid, Color, Cell, KeyEvent, MouseEvent};
pub use config::TerminalConfig;
pub use error::TerminalError;
pub use grid::Grid;
pub use security::{sanitize_paste, validate_osc_sequence, RateLimiter, SecurityConfig};
pub use terminal::VteTerminalCore;

// Define core traits for backend-agnostic implementation

use std::io::Write;

/// Main renderer trait for different backends
pub trait Renderer {
    fn text_renderer(&mut self) -> &mut dyn TextRenderer;
    fn graphics_renderer(&mut self) -> &mut dyn GraphicsRenderer;
    fn ui_renderer(&mut self) -> &mut dyn UIRenderer;
}

/// Text rendering sub-trait
pub trait TextRenderer {
    fn draw_cell(&mut self, row: usize, col: usize, cell: &Cell);
    fn set_font(&mut self, family: &str, size: f64);
    fn get_char_metrics(&self, ch: char) -> CharMetrics;
}

/// Graphics rendering sub-trait
pub trait GraphicsRenderer {
    fn draw_sixel(&mut self, data: &[u8], x: usize, y: usize);
    fn draw_image(&mut self, image: ImageData, x: usize, y: usize);
}

/// UI rendering sub-trait
pub trait UIRenderer {
    fn clear(&mut self);
    fn flush(&mut self);
    fn set_cursor_shape(&mut self, shape: CursorShape);
}

/// Input handling trait
pub trait InputHandler {
    fn handle_key(&mut self, key: crate::ansi::KeyEvent, grid: &std::sync::Arc<std::sync::RwLock<Grid>>, writer: &std::sync::Arc<std::sync::Mutex<Box<dyn Write + Send>>>);
    fn handle_mouse(&mut self, event: crate::ansi::MouseEvent, grid: &std::sync::Arc<std::sync::RwLock<Grid>>);
    fn handle_scroll(&mut self, delta: f64, grid: &std::sync::Arc<std::sync::RwLock<Grid>>);
}

/// Event loop trait
pub trait EventLoop {
    fn schedule_redraw(&mut self, callback: Box<dyn FnMut()>);
    fn schedule_timer(&mut self, interval_ms: u64, callback: Box<dyn FnMut() -> bool>) -> bool;
}

// Core data structures for backends

/// Character metrics returned by font renderers
#[derive(Clone, Copy, Debug)]
pub struct CharMetrics {
    pub width: f64,
    pub height: f64,
    pub ascent: f64,
}

/// Available cursor shapes for terminals
#[derive(Clone, Copy, Debug)]
pub enum CursorShape {
    /// Solid block cursor
    Block,
    /// Underscore cursor
    Underline,
    /// Vertical bar cursor
    Bar,
}

/// Image data for graphics rendering
pub struct ImageData {
    pub data: Vec<u8>,
    pub width: usize,
    pub height: usize,
}

/// Memory usage information
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    pub primary_buffer_bytes: usize,
    pub alternate_buffer_bytes: usize,
    pub scrollback_buffer_bytes: usize,
    pub total_grid_bytes: usize,
}

use crate::ansi::{Cell, KeyEvent, MouseEvent};
use crate::drawing::CharMetrics;
use crate::grid::Grid;

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

use std::sync::{Arc, RwLock, Mutex};
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
    /// Handle hyperlink click (OSC 8) - return true if handled
    fn handle_hyperlink(&mut self, url: &str) -> bool;
}

/// Input handling trait
pub trait InputHandler {
    fn handle_key(&mut self, key: KeyEvent, grid: &Arc<RwLock<Grid>>,
                  writer: &Arc<Mutex<Box<dyn Write + Send>>>);
    fn handle_mouse(&mut self, event: MouseEvent, grid: &Arc<RwLock<Grid>>);
    fn handle_scroll(&mut self, delta: f64, grid: &Arc<RwLock<Grid>>);
}

/// Event loop trait
pub trait EventLoop {
    fn schedule_redraw(&mut self, callback: Box<dyn FnMut()>);
    fn schedule_timer(&mut self, interval_ms: u64, callback: Box<dyn FnMut() -> bool>) -> bool;
}

// Complementary traits for testing and headless operation

/// Backend-agnostic testing interface for headless terminal operation
pub trait Backend: Renderer + InputHandler + EventLoop {
    fn resize(&mut self, cols: usize, rows: usize);
}

/// Keyboard input abstraction for cross-platform compatibility
pub trait KeyboardHandler {
    fn process_key_event(&mut self, key: KeyEvent) -> KeyEventResult;
}

/// Mouse input abstraction for cross-platform compatibility
pub trait MouseHandler {
    fn process_mouse_event(&mut self, event: MouseEvent) -> MouseEventResult;
}

/// Clipboard operations for terminal applications
pub trait ClipboardHandler {
    fn set_clipboard_text(&mut self, text: &str) -> Result<(), String>;
    fn get_clipboard_text(&mut self) -> Result<String, String>;
}

// Data structures

/// Image data for graphics rendering
#[derive(Clone, Debug)]
pub struct ImageData {
    pub data: Vec<u8>,
    pub width: usize,
    pub height: usize,
}

/// Result types for input processing
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyEventResult {
    /// Event was handled and processed
    Handled,
    /// Event was ignored or passed through
    Ignored,
    /// Event should be forwarded to application
    Forward,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseEventResult {
    /// Event was handled and processed
    Handled,
    /// Event was ignored or passed through
    Ignored,
    /// Event should be forwarded to application
    Forward,
}

/// Error types for backend operations
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("Rendering error: {0}")]
    Rendering(String),

    #[error("Font error: {0}")]
    Font(String),

    #[error("Input handling error: {0}")]
    Input(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

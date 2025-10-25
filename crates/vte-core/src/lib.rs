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
pub mod traits;

// Re-export main types
pub use ansi::{AnsiParser, AnsiGrid, Color, Cell, KeyEvent, MouseEvent};
pub use config::TerminalConfig;
pub use error::TerminalError;
pub use grid::Grid;
pub use security::{sanitize_paste, validate_osc_sequence, RateLimiter, SecurityConfig};
pub use terminal::VteTerminalCore;

// Re-export traits and types
pub use traits::*;

// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    pub primary_buffer_bytes: usize,
    pub alternate_buffer_bytes: usize,
    pub scrollback_buffer_bytes: usize,
    pub total_grid_bytes: usize,
}

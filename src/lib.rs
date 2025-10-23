//! HugoVTE - A terminal emulator written in Rust
//!
//! This crate provides a GTK4-based terminal emulator with support for:
//! - ANSI escape sequences
//! - Text selection and clipboard operations
//! - Customizable appearance
//! - PTY integration

pub mod ansi;
pub mod config;
pub mod constants;
pub mod drawing;
pub mod grid;
pub mod input;
pub mod selection;
pub mod terminal;

// Re-export main types for convenience
pub use ansi::{AnsiParser, AnsiGrid, Color, Cell};
pub use config::TerminalConfig;
pub use grid::Grid;
pub use terminal::VteTerminal;


#[cfg(target_os = "macos")]
unsafe extern "C" {
    pub fn init_blur_api();
    pub fn set_opacity_and_blur(
        gtk_window: *mut std::ffi::c_void,
        opacity: f64,
        blur_amount: f64,
        red: f64, 
        green: f64, 
        blue: f64
    ) -> i32;
}

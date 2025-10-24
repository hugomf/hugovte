//! HugoVTE - A terminal emulator written in Rust
//!
//! This crate provides a GTK4-based terminal emulator with support for:
//! - ANSI escape sequences
//! - Text selection and clipboard operations
//! - Customizable appearance
//! - PTY integration

// Re-export from vte-core
pub use vte_core::*;

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

//! VTE GTK4 - GTK4 backend for vte-core terminal emulator
//!
//! This crate provides a complete GTK4 implementation of the vte-core traits,
//! enabling terminal emulation with GTK4 user interface components.

use crate::backend::Gtk4Backend;
use crate::terminal::VteTerminalWidget;
use crate::cairo_renderer::{CairoTextRenderer, CairoGraphicsRenderer, CairoUIRenderer};
use crate::input::{Gtk4InputHandler, Gtk4EventLoop};
use gtk4::prelude::*;
use vte_core::{Renderer, InputHandler, EventLoop, TerminalConfig};

mod cairo_renderer;
mod input;
mod backend;
mod terminal;



// Re-export vte-core types for convenience
pub use vte_core::*;

// Placeholder for GTK backend implementation
// TODO: Implement GTK-specific Renderer, InputHandler, EventLoop

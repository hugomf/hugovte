//! # VTE ANSI Parser
//!
//! A comprehensive UTF-8-safe ANSI/VT escape sequence parser crate.
//! This crate provides the core ANSI parsing functionality that was originally
//! part of the `vte-core` library, extracted as a standalone crate.

pub mod color;
pub mod grid;
pub mod parser;

pub use color::{Color, COLOR_PALETTE};
pub use grid::{AnsiGrid, Cell, KeyEvent, MouseEvent};
pub use parser::{AnsiParser, AnsiError, ErrorCallback};

//! GTK4 terminal widget implementation

use crate::backend::Gtk4Backend;
use gtk4::{DrawingArea, prelude::*};
use vte_core::{TerminalConfig, TerminalError};

/// GTK4 terminal widget wrapper
pub struct VteTerminalWidget {
    area: DrawingArea,
    backend: Gtk4Backend,
}

impl VteTerminalWidget {
    /// Create a new GTK4 terminal widget with default configuration
    pub fn new() -> Result<Self, TerminalError> {
        Self::with_config(TerminalConfig::default())
    }

    /// Create a new GTK4 terminal widget with custom configuration
    pub fn with_config(config: TerminalConfig) -> Result<Self, TerminalError> {
        let area = DrawingArea::new();
        area.set_focusable(true);
        area.set_hexpand(true);
        area.set_vexpand(true);
        area.grab_focus();

        let backend = Gtk4Backend::new(config, &area)?;

        Ok(VteTerminalWidget { area, backend })
    }

    /// Get the GTK widget
    pub fn widget(&self) -> &DrawingArea {
        &self.area
    }

    /// Get access to the backend
    pub fn backend(&self) -> &Gtk4Backend {
        &self.backend
    }

    /// Get access to the backend mutably
    pub fn backend_mut(&mut self) -> &mut Gtk4Backend {
        &mut self.backend
    }
}

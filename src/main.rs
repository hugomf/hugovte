// src/main.rs
mod ansi;
mod grid;           // NEW - Grid state & AnsiGrid implementation
mod terminal;       // NEW - Terminal widget (was vte.rs)
mod input;          // NEW - Input handlers
mod selection;
mod config;
mod constants;
mod drawing;

use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, gdk, CssProvider};
use crate::terminal::VteTerminal;
use crate::config::TerminalConfig;
use crate::ansi::Color;

fn main() {
    let app = Application::builder()
        .application_id("com.example.hugovte")
        .build();

    app.connect_activate(|app| {
        // Create custom configuration with transparency
        let config = TerminalConfig::default()
            .with_background_color(Color::rgba(0.0, 0.0, 0.0, 0.0)) // Fully transparent
            .with_foreground_color(Color::rgb(1.0, 1.0, 1.0))
            .with_grid_lines(false);

        // Main window
        let window = ApplicationWindow::builder()
            .application(app)
            .title("HugoTerm")
            .default_width(800)
            .default_height(600)
            .build();

        // Enable transparency via CSS
        setup_transparency();

        // Create terminal widget
        let terminal = VteTerminal::with_config(config);
        terminal.area.set_vexpand(true);
        terminal.area.set_hexpand(true);
        
        window.set_child(Some(terminal.widget()));
        window.present();
        terminal.area.queue_draw();
    });

    app.run();
}

fn setup_transparency() {
    let css = CssProvider::new();
    css.load_from_data(
        "window { background-color: transparent; }
         drawingarea { background-color: transparent; }"
    );
    
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &css,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        
        if display.is_composited() {
            println!("✓ Compositor available - transparency enabled");
        } else {
            println!("⚠ No compositor detected - transparency may not work");
        }
    }
}
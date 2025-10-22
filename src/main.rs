mod ansi;
mod vte;

use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow};
use gtk4::gdk;
use crate::vte::VteTerminal; // ✅ use your own module’s terminal

fn main() {
    let app = Application::builder()
        .application_id("com.example.hugovte")
        .build();

    app.connect_activate(|app| {
        // Dark background CSS
        let provider = gtk4::CssProvider::new();
        provider
            .load_from_data(
                "window {
                    background-color: black;
                }
                drawingarea {
                    background-color: black;
                }",
            );

        gtk4::style_context_add_provider_for_display(
            &gdk::Display::default().expect("Cannot open display"),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        // Main window
        let window = ApplicationWindow::builder()
            .application(app)
            .title("HugoTerm")
            .default_width(800)
            .default_height(600)
            .build();

        // ✅ use your custom terminal widget
        let terminal = VteTerminal::new();
        window.set_child(Some(terminal.widget()));
        window.present();

        // Force initial draw
        terminal.area.queue_draw();
    });

    app.run();
}
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


// Declare the external C functions
#[cfg(target_os = "macos")]
unsafe extern "C" {
    
    fn set_opacity_and_blur(
        gtk_window: *mut std::ffi::c_void,
        opacity: f64,
        blur_amount: f64,
        red: f64, 
        green: f64, 
        blue: f64
    ) -> i32;
    
    fn init_blur_api();
}

fn hex_to_rgb(hex: &str) -> Option<(f64, f64, f64)> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    
    let rgb = u32::from_str_radix(hex, 16).ok()?;
    let red = ((rgb >> 16) & 0xff) as f64 / 255.0;
    let green = ((rgb >> 8) & 0xff) as f64 / 255.0;
    let blue = (rgb & 0xff) as f64 / 255.0;
    
    Some((red, green, blue))
}


fn main() {
    let app = Application::builder()
        .application_id("com.example.hugovte")
        .build();

    app.connect_activate(|app| {
        // Create custom configuration with transparency
        let config = TerminalConfig::default()
            .with_background_color(Color::rgba(0.0, 0.0, 0.0, 0.0)) // Fully transparent
            .with_foreground_color(Color::rgb(1.0, 1.0, 1.0))
            .with_grid_lines(false);  // Enable grid lines

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



        // Apply macOS transparency and blur
        #[cfg(target_os = "macos")]
        {
            use std::time::Duration;
            let window_clone = window.clone();
            


            // Initialize blur API first
            unsafe {
                init_blur_api();
            }

            let opacity = 0.4;     // 0.0 = fully transparent, 1.0 = fully opaque
            let blur_amount = 0.1;  // 0.0 = no blur, 1.0 = maximum blur
            let tint_color = "#1e1e1e";
            println!("🎨 Setting opacity: {}, blur: {}", opacity, blur_amount);

            if let Some((red, green, blue)) = hex_to_rgb(tint_color) {
                println!("🎡 Converting {} to RGB: ({:.4}, {:.4}, {:.4})", tint_color, red, green, blue);
            
                glib::timeout_add_local(Duration::from_millis(100), move || {
                    unsafe {
                        set_opacity_and_blur(
                            window_clone.as_ptr() as *mut _,
                            opacity,
                            blur_amount,
                            red,
                            green,
                            blue
                        );
                    }
                    glib::ControlFlow::Break
                });
            }
        }



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

//! GTK4 backend implementation combining all traits

use crate::cairo_renderer::{CairoTextRenderer, CairoGraphicsRenderer, CairoUIRenderer};
use crate::input::{Gtk4InputHandler, Gtk4EventLoop};
use gtk4::DrawingArea;
use cairo;
use vte_core::{VteTerminalCore, TerminalConfig, Renderer, ImageData, Cell, Color, CharMetrics, CursorShape, TerminalError};
use vte_core::drawing::DrawingCache;
use async_channel::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::io::Write;

/// Complete GTK4 backend for the vte-core terminal
pub struct Gtk4Backend {
    terminal: VteTerminalCore,
    event_loop: Gtk4EventLoop,
    redraw_rx: Receiver<()>,
    redraw_tx: Sender<()>,
    char_w: f64,
    char_h: f64,
}

impl Gtk4Backend {
    /// Create a new GTK4 backend with the given configuration
    pub fn new(config: TerminalConfig, area: &DrawingArea) -> Result<Self, TerminalError> {
        // Estimate character dimensions
        let char_w = 10.0; // Approximate monospace width
        let char_h = 16.0; // Approximate monospace height

        // Create async channel for redraw signals
        let (redraw_tx, redraw_rx) = async_channel::unbounded::<()>();

        // Create terminal core
        let mut terminal = VteTerminalCore::with_config(config.clone());

        // Set up drawing
        let terminal_clone = Arc::clone(&terminal.grid);
        let redraw_tx_clone = redraw_tx.clone();

        let drawing_config = config.clone();
        area.set_draw_func(move |area, cr, _w, _h| {
            // Handle drawing through renderer
            let mut renderer = Gtk4Renderer::new(cr, area, char_w, char_h);

            // Draw from terminal grid
            if let Ok(g) = terminal_clone.read() {
                for r in 0..g.rows {
                    for c in 0..g.cols {
                        let cell = g.get_cell(r, c);
                        renderer.text_renderer().draw_cell(r, c, cell);
                    }
                }

                // Draw cursor if visible
                if g.row < g.rows && g.col < g.cols && g.is_cursor_visible() && g.scroll_offset == 0 {
                    // Draw cursor outline
                    renderer.ui_renderer().set_cursor_shape(CursorShape::Block);
                }
            }

            // Signal redraw completion
            let _ = redraw_tx_clone.send_blocking(());
        });

        // Set up input handling
        let writer_arc: Arc<Mutex<Box<dyn Write + Send>>> = Arc::new(Mutex::new(Box::new(std::io::sink())));

        Gtk4InputHandler::setup_keyboard(area, Arc::clone(&terminal.grid), writer_arc, redraw_tx.clone());
        Gtk4InputHandler::setup_mouse(area, Arc::clone(&terminal.grid), redraw_tx.clone(), char_w, char_h);

        // Create event loop
        let mut event_loop = Gtk4EventLoop::new();
        event_loop.set_area(area);

        Ok(Gtk4Backend {
            terminal,
            event_loop,
            redraw_rx,
            redraw_tx,
            char_w,
            char_h,
        })
    }

    /// Get the terminal core
    pub fn terminal(&self) -> &VteTerminalCore {
        &self.terminal
    }

    /// Get the terminal core mutably
    pub fn terminal_mut(&mut self) -> &mut VteTerminalCore {
        &mut self.terminal
    }

    /// Get the event loop
    pub fn event_loop(&self) -> &Gtk4EventLoop {
        &self.event_loop
    }

    /// Schedule a redraw
    pub fn schedule_redraw(&self) {
        let _ = self.redraw_tx.send_blocking(());
    }

    /// Process pending redraws
    pub fn process_events(&self) {
        // Try to receive redraw signals (non-blocking)
        while let Ok(_) = self.redraw_rx.try_recv() {}
    }
}

/// Composite GTK4 renderer
pub struct Gtk4Renderer {
    text_renderer: CairoTextRenderer,
    graphics_renderer: CairoGraphicsRenderer,
    ui_renderer: CairoUIRenderer,
}

impl Gtk4Renderer {
    pub fn new(context: &cairo::Context, area: &DrawingArea, char_w: f64, char_h: f64) -> Self {
        // Create backend-agnostic drawing cache
        let drawing_cache = DrawingCache::new("monospace", 13.0)
            .unwrap_or_else(|_| panic!("Failed to create drawing cache"));

        let text_renderer = CairoTextRenderer::new(context.clone(), drawing_cache)
            .unwrap_or_else(|_| panic!("Failed to create text renderer"));
        let graphics_renderer = CairoGraphicsRenderer::new(context.clone());
        let ui_renderer = CairoUIRenderer::new(context.clone());

        Gtk4Renderer {
            text_renderer,
            graphics_renderer,
            ui_renderer,
        }
    }
}

impl Renderer for Gtk4Renderer {
    fn text_renderer(&mut self) -> &mut dyn vte_core::TextRenderer {
        &mut self.text_renderer
    }

    fn graphics_renderer(&mut self) -> &mut dyn vte_core::GraphicsRenderer {
        &mut self.graphics_renderer
    }

    fn ui_renderer(&mut self) -> &mut dyn vte_core::UIRenderer {
        &mut self.ui_renderer
    }
}

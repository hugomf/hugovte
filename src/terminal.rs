// src/terminal.rs
use crate::grid::Grid;
use crate::ansi::AnsiParser;
use crate::config::TerminalConfig;
use crate::drawing::DrawingCache;
use crate::constants::{SELECTION_BG, GRID_LINE_COLOR};
use crate::input::InputHandler;

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use gtk4::prelude::*;
use gtk4::DrawingArea;
use cairo::{FontSlant, FontWeight};
use std::sync::{Arc, RwLock, Mutex};
use std::thread;
use std::io::{Read, Write};
use std::time::Duration;

/// Main terminal widget - coordinates GTK, PTY, and rendering
pub struct VteTerminal {
    pub area: DrawingArea,
    pub drawing_cache: DrawingCache,
    pub grid: Arc<RwLock<Grid>>,
    pty_pair: Arc<RwLock<Option<portable_pty::PtyPair>>>,
}

impl VteTerminal {
    pub fn new() -> Self {
        Self::with_config(TerminalConfig::default())
    }

    pub fn with_config(config: TerminalConfig) -> Self {
        eprintln!("INFO: Creating new VteTerminal with config: grid_lines={}, grid_alpha={:.2}",
                 config.draw_grid_lines, config.grid_line_alpha);
        let area = DrawingArea::new();
        area.set_focusable(true);
        area.grab_focus();

        // Create drawing cache
        let drawing_cache = DrawingCache::new(&config.font_family, config.font_size)
            .expect("Failed to create drawing cache");

        let char_w = drawing_cache.char_width();
        let char_h = drawing_cache.char_height();
        let ascent = drawing_cache.ascent();

        let init_cols = ((800.0 / char_w).max(1.0) as usize).min(120);
        let init_rows = ((600.0 / char_h).max(1.0) as usize).min(50);
        
        // Create grid with config colors
        let mut grid = Grid::new(init_cols, init_rows);
        grid.fg = config.default_fg;
        grid.bg = config.default_bg;
        
        let grid = Arc::new(RwLock::new(grid));

        // Spawn PTY
        let pty_pair = Self::spawn_pty(init_cols, init_rows);
        
        // Get reader and writer from master PTY
        let (reader, writer) = {
            let pair_guard = pty_pair.read().unwrap();
            let pair = pair_guard.as_ref().unwrap();
            let reader = pair.master.try_clone_reader().expect("Failed to clone reader");
            let writer = pair.master.take_writer().expect("Failed to take writer");
            (reader, writer)
        };

        let writer_arc: Arc<Mutex<Box<dyn Write + Send>>> = Arc::new(Mutex::new(writer));

        // Redraw channel
        let (tx, rx) = async_channel::unbounded::<()>();
        let area_weak = area.downgrade();
        glib::MainContext::default().spawn_local(async move {
            while rx.recv().await.is_ok() {
                if let Some(area) = area_weak.upgrade() {
                    area.queue_draw();
                }
            }
        });

        // Start cursor blink timer
        if config.enable_cursor_blink {
            Self::start_cursor_blink(Arc::clone(&grid), tx.clone(), config.cursor_blink_interval_ms);
        }

        // Start PTY reader thread
        Self::start_reader_thread(reader, Arc::clone(&grid), tx.clone());

        // Send initial welcome message
        {
            let mut w = writer_arc.lock().unwrap();
            writeln!(w, "echo 'Welcome to HugoTerm!'").unwrap();
            w.flush().unwrap();
        }
        let _ = tx.send_blocking(());

        // Clone config for drawing function to avoid move issues
        let drawing_config = config.clone();
        eprintln!("DEBUG: About to pass config to drawing function - grid_lines: {}", drawing_config.draw_grid_lines);

        // Setup drawing
        Self::setup_drawing(
            &area,
            Arc::clone(&grid),
            Arc::clone(&pty_pair),
            drawing_cache.clone(),
            drawing_config,  // Pass cloned config to drawing function
            char_w,
            char_h,
            ascent,
        );

        // Setup input handlers
        InputHandler::setup_keyboard(&area, Arc::clone(&grid), Arc::clone(&writer_arc), tx.clone());

        if config.enable_selection {
            InputHandler::setup_mouse(&area, Arc::clone(&grid), tx.clone(), char_w, char_h);
        }

        area.queue_draw();

        Self {
            area,
            drawing_cache,
            grid,
            pty_pair,
        }
    }

    /// Spawn PTY with bash shell
    fn spawn_pty(cols: usize, rows: usize) -> Arc<RwLock<Option<portable_pty::PtyPair>>> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: rows as u16,
                cols: cols as u16,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("Failed to open PTY");

        let mut cmd = CommandBuilder::new("bash");
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("CLICOLOR", "1");
        cmd.env("LSCOLORS", "ExGxFxdxCxDxDxBxBxExEx");
        
        let _child = pair.slave.spawn_command(cmd).expect("Failed to spawn shell");
        
        Arc::new(RwLock::new(Some(pair)))
    }

    /// Start cursor blink timer
    fn start_cursor_blink(
        grid: Arc<RwLock<Grid>>,
        tx: async_channel::Sender<()>,
        interval_ms: u64,
    ) {
        glib::timeout_add_local(Duration::from_millis(interval_ms), move || {
            if let Ok(mut g) = grid.write() {
                g.toggle_cursor();
            }
            let _ = tx.send_blocking(());
            glib::ControlFlow::Continue
        });
    }

    /// Start PTY reader thread
    fn start_reader_thread(
        mut reader: Box<dyn Read + Send>,
        grid: Arc<RwLock<Grid>>,
        tx: async_channel::Sender<()>,
    ) {
        thread::spawn(move || {
            let mut parser = AnsiParser::new();
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Ok(mut g) = grid.write() {
                            for &b in &buf[..n] {
                                parser.process(b, &mut *g);
                            }
                        }
                        let _ = tx.send_blocking(());
                    }
                    Err(e) => {
                        eprintln!("PTY read error: {}", e);
                        break;
                    }
                }
            }
        });
    }

    /// Setup drawing function with transparency support
    #[allow(clippy::too_many_arguments)]
    fn setup_drawing(
        area: &DrawingArea,
        grid: Arc<RwLock<Grid>>,
        pty_pair: Arc<RwLock<Option<portable_pty::PtyPair>>>,
        drawing_cache: DrawingCache,
        config: TerminalConfig,
        char_w: f64,
        char_h: f64,
        ascent: f64,
    ) {
        eprintln!("DEBUG: setup_drawing received config - grid_lines: {}", config.draw_grid_lines);
        area.set_draw_func(move |area, cr, _w, _h| {
            // CRITICAL: Do NOT clear or paint - preserves transparency
            
            let cols = (area.width() as f64 / char_w).max(1.0) as usize;
            let rows = (area.height() as f64 / char_h).max(1.0) as usize;

            // Handle resize
            {
                if let Ok(mut g) = grid.write() {
                    if g.cols != cols || g.rows != rows {
                        g.resize(cols, rows);
                        if let Ok(pair_guard) = pty_pair.read() {
                            if let Some(ref pair) = *pair_guard {
                                let _ = pair.master.resize(PtySize {
                                    rows: rows as u16,
                                    cols: cols as u16,
                                    pixel_width: 0,
                                    pixel_height: 0,
                                });
                            }
                        }
                    }
                }
            }

            let g = grid.read().unwrap();

            // Log when drawing starts (only first time to avoid spam)
            if cfg!(debug_assertions) {
                eprintln!("INFO: Starting to draw {}x{} grid", cols, rows);
                eprintln!("DEBUG: Config in draw function - grid_lines: {}, alpha: {:.2}", config.draw_grid_lines, config.grid_line_alpha);
            }

            // Draw cells with proper font metrics
            for r in 0..g.rows.min(rows) {
                let mut current_x = 0.0; // Track actual X position for this row
                for c in 0..g.cols.min(cols) {
                    let cell = g.get_cell(r, c);
                    let y = r as f64 * char_h;

                    // Use cell position for background and grid, but character positioning uses font metrics
                    let cell_x = c as f64 * char_w;

                    // Background (with selection highlight)
                    if g.is_selected(r + g.scrollback.len() / g.cols, c) {
                        cr.set_source_rgba(SELECTION_BG.r, SELECTION_BG.g, SELECTION_BG.b, SELECTION_BG.a);
                        cr.rectangle(cell_x, y, char_w, char_h);
                        cr.fill().unwrap();
                    } else if cell.bg.a > 0.01 {
                        // Only draw background if it has opacity
                        cr.set_source_rgba(cell.bg.r, cell.bg.g, cell.bg.b, cell.bg.a);
                        cr.rectangle(cell_x, y, char_w, char_h);
                        cr.fill().unwrap();
                    }

                    // Text
                    if cell.ch != '\0' && cell.ch != ' ' {
                        cr.set_source_rgb(cell.fg.r, cell.fg.g, cell.fg.b);

                        let slant = if cell.italic { FontSlant::Italic } else { FontSlant::Normal };
                        let weight = if cell.bold { FontWeight::Bold } else { FontWeight::Normal };

                        if let Some(font) = drawing_cache.get_font(slant, weight) {
                            cr.set_scaled_font(font);

                            // Use actual font metrics for character positioning
                            let text = &cell.ch.to_string();

                            // For monospace fonts, use left alignment within each cell
                            // This gives proper terminal-like character spacing
                            let pos_x = cell_x;

                            // Debug output for character spacing analysis (first few chars only)
                            if cfg!(debug_assertions) && c < 3 && r < 5 {
                                let char_advance = drawing_cache.get_char_advance(cell.ch);
                                eprintln!("DEBUG: Char '{}' at pos: {:.2}, advance: {:.2}, cell: {:.2}",
                                    cell.ch, pos_x, char_advance, char_w);
                            }

                            cr.move_to(pos_x, y + ascent);
                            cr.show_text(text).unwrap();
                        }
                    }

                    // Underline
                    if cell.underline {
                        cr.set_source_rgb(cell.fg.r, cell.fg.g, cell.fg.b);
                        cr.move_to(cell_x, y + char_h - 1.0);
                        cr.line_to(cell_x + char_w, y + char_h - 1.0);
                        cr.set_line_width(1.0);
                        cr.stroke().unwrap();
                    }

                    // Grid lines (optional) - drawn last so they appear on top
                    if config.draw_grid_lines {
                        cr.set_source_rgba(
                            GRID_LINE_COLOR.r,
                            GRID_LINE_COLOR.g,
                            GRID_LINE_COLOR.b,
                            config.grid_line_alpha,
                        );
                        cr.set_line_width(1.0);

                        // Draw vertical lines
                        cr.move_to(cell_x + char_w, y);
                        cr.line_to(cell_x + char_w, y + char_h);

                        // Draw horizontal lines
                        cr.move_to(cell_x, y + char_h);
                        cr.line_to(cell_x + char_w, y + char_h);

                        cr.stroke().unwrap();

                        // Always log first grid line to verify drawing
                        if r == 0 && c == 0 {
                            eprintln!("GRID: Drawing grid line at cell (0,0) - enabled: {}, pos: ({:.1}, {:.1}) to ({:.1}, {:.1})",
                                config.draw_grid_lines, cell_x + char_w, y, cell_x + char_w, y + char_h);
                        }
                    }
                }
            }

            // Draw cursor
            if g.row < g.rows && g.col < g.cols && g.is_cursor_visible() {
                let cursor_x = g.col as f64 * char_w;
                let cursor_y = g.row as f64 * char_h;
                let cursor_cell = g.get_cell(g.row, g.col);

                // Draw cursor as outline
                cr.set_source_rgb(
                    1.0 - cursor_cell.bg.r,
                    1.0 - cursor_cell.bg.g,
                    1.0 - cursor_cell.bg.b,
                );
                cr.rectangle(cursor_x, cursor_y, char_w, char_h);
                cr.set_line_width(2.0);
                cr.stroke().unwrap();

                // Draw cursor cell content
                if cursor_cell.ch != '\0' && cursor_cell.ch != ' ' {
                    cr.set_source_rgb(cursor_cell.fg.r, cursor_cell.fg.g, cursor_cell.fg.b);
                    let slant = if cursor_cell.italic { FontSlant::Italic } else { FontSlant::Normal };
                    let weight = if cursor_cell.bold { FontWeight::Bold } else { FontWeight::Normal };

                    if let Some(font) = drawing_cache.get_font(slant, weight) {
                        cr.set_scaled_font(font);

                        // Left-align cursor character within its cell for consistent spacing
                        let text = &cursor_cell.ch.to_string();

                        // Position cursor character at the left edge of its cell
                        let pos_x = cursor_x;

                        cr.move_to(pos_x, cursor_y + ascent);
                        cr.show_text(text).unwrap();
                    }
                }
            }
        });
    }

    pub fn widget(&self) -> &DrawingArea {
        &self.area
    }
}

impl Drop for VteTerminal {
    fn drop(&mut self) {
        if let Ok(mut pair_guard) = self.pty_pair.write() {
            *pair_guard = None;
        }
    }
}

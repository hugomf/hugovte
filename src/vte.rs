use crate::ansi::{AnsiGrid, Cell, Color, Parser};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use gtk4::prelude::*;
use gtk4::{DrawingArea, EventControllerKey, cairo};
use gtk4::gdk;
use cairo::{FontSlant, FontWeight, Operator};
use std::sync::{Arc, Mutex};
use std::thread;
use std::io::{Read, Write};
use glib::Propagation;

const GRID_LINE_COLOR: Color = Color { r: 0.2, g: 0.0, b: 0.0 };

/// ----------  Grid  ----------
pub struct Grid {
    pub cols: usize,
    pub rows: usize,
    pub cells: Vec<Vec<Cell>>,
    pub scrollback: Vec<Vec<Cell>>,
    pub scroll_offset: usize,
    pub col: usize,
    pub row: usize,
    pub fg:  Color,
    pub bg:  Color,
    bold: bool,
    italic: bool,
    underline: bool,
    dim: bool,
}

impl Grid {
    pub fn new(cols: usize, rows: usize) -> Self {
        let default_fg = Color { r: 1.0, g: 1.0, b: 1.0 };
        let default_bg = Color { r: 0.0, g: 0.0, b: 0.0 };
        let default_cell = Cell {
            ch: '\0',
            fg: default_fg,
            bg: default_bg,
            ..Default::default()
        };
        Self {
            cols, rows,
            cells: vec![vec![default_cell; cols]; rows],
            scrollback: Vec::new(),
            scroll_offset: 0,
            col: 0, row: 0,
            fg: default_fg,
            bg: default_bg,
            bold: false, italic: false, underline: false, dim: false,
        }
    }

    pub fn clear(&mut self) {
        let default_fg = Color { r: 1.0, g: 1.0, b: 1.0 };
        let default_bg = Color { r: 0.0, g: 0.0, b: 0.0 };
        let default_cell = Cell {
            ch: '\0',
            fg: default_fg,
            bg: default_bg,
            ..Default::default()
        };
        for r in &mut self.cells { 
            r.fill(default_cell); 
        }
        self.col = 0; 
        self.row = 0;
        self.scrollback.clear();
        self.scroll_offset = 0;
    }

    pub fn resize(&mut self, new_cols: usize, new_rows: usize) {
        let mut new_cells = vec![vec![Cell::default(); new_cols]; new_rows];
        
        // Copy existing content
        for (r, row) in self.cells.iter().enumerate().take(new_rows) {
            for (c, cell) in row.iter().enumerate().take(new_cols) {
                if r < new_rows && c < new_cols {
                    new_cells[r][c] = *cell;
                }
            }
        }
        
        self.cells = new_cells;
        self.cols = new_cols;
        self.rows = new_rows;
        self.col = self.col.min(new_cols.saturating_sub(1));
        self.row = self.row.min(new_rows.saturating_sub(1));
    }
}

impl AnsiGrid for Grid {
    fn put(&mut self, ch: char) {
        if self.col < self.cols && self.row < self.rows {
            self.cells[self.row][self.col] = Cell {
                ch, fg: self.fg, bg: self.bg,
                bold: self.bold, italic: self.italic, underline: self.underline, dim: self.dim,
            };
            eprintln!("[{:2},{:2}] = '{}'", self.row, self.col, ch);
        }
    }
    
    fn advance(&mut self) {
        self.col += 1;
        if self.col >= self.cols { self.newline(); }
    }
    
    fn left(&mut self, n: usize) { self.col = self.col.saturating_sub(n); }
    fn right(&mut self, n: usize) { self.col = (self.col + n).min(self.cols - 1); }
    fn up(&mut self, n: usize) { self.row = self.row.saturating_sub(n); }
    fn down(&mut self, n: usize) { self.row = (self.row + n).min(self.rows - 1); }
    
    fn newline(&mut self) {
        self.col = 0;
        self.row += 1;
        if self.row >= self.rows {
            // Save the line being scrolled off to scrollback
            let scrolled_line = std::mem::replace(&mut self.cells[0], vec![]);
            self.scrollback.push(scrolled_line);
            
            // Keep only recent scrollback (limit to 1000 lines)
            if self.scrollback.len() > 1000 {
                self.scrollback.remove(0);
            }
            
            // Shift lines up
            for i in 1..self.rows {
                self.cells[i-1] = std::mem::replace(&mut self.cells[i], vec![]);
            }
            
            // Add new empty line at bottom
            let default_cell = Cell {
                ch: '\0',
                fg: self.fg,
                bg: self.bg,
                ..Default::default()
            };
            self.cells[self.rows-1] = vec![default_cell; self.cols];
            self.row = self.rows - 1;
        }
    }
    
    fn carriage_return(&mut self) { self.col = 0; }
    fn backspace(&mut self) { if self.col > 0 { self.col -= 1; } }
    
    fn move_rel(&mut self, dx: i32, dy: i32) {
        let new_col = (self.col as i32 + dx).max(0) as usize;
        let new_row = (self.row as i32 + dy).max(0) as usize;
        self.col = new_col.min(self.cols - 1);
        self.row = new_row.min(self.rows - 1);
    }
    
    fn move_abs(&mut self, row: usize, col: usize) {
        self.col = col.min(self.cols.saturating_sub(1));
        self.row = row.min(self.rows.saturating_sub(1));
    }
    
    fn clear_screen(&mut self) { 
        self.clear(); 
    }
    
    fn clear_line(&mut self) {
        let default_cell = Cell {
            ch: '\0',
            fg: self.fg,
            bg: self.bg,
            ..Default::default()
        };
        self.cells[self.row].fill(default_cell);
    }
    
    fn reset_attrs(&mut self) {
        self.fg = Color { r: 1.0, g: 1.0, b: 1.0 };
        self.bg = Color { r: 0.0, g: 0.0, b: 0.0 };
        self.bold = false; 
        self.italic = false; 
        self.underline = false; 
        self.dim = false;
    }
    
    fn set_bold(&mut self, bold: bool) { self.bold = bold; }
    fn set_italic(&mut self, italic: bool) { self.italic = italic; }
    fn set_underline(&mut self, underline: bool) { self.underline = underline; }
    fn set_dim(&mut self, dim: bool) { self.dim = dim; }
    fn set_fg(&mut self, color: Color) { self.fg = color; }
    fn set_bg(&mut self, color: Color) { self.bg = color; }
    
    // These are the required methods from the trait
    fn get_fg(&self) -> Color { self.fg }
    fn get_bg(&self) -> Color { self.bg }
}

/// ----------  Terminal Widget  ----------
pub struct VteTerminal {
    pub area: DrawingArea,
    pub char_w: f64,
    pub char_h: f64,
    pub ascent: f64,
    pub grid: Arc<Mutex<Grid>>,
}

impl VteTerminal {
    pub fn new() -> Self {
        let area = DrawingArea::new();

        area.grab_focus();
        area.set_focusable(true);

        let (char_w, char_h, ascent) = {
            let surf = cairo::ImageSurface::create(cairo::Format::ARgb32, 1, 1).unwrap();
            let cr = cairo::Context::new(&surf).unwrap();
            cr.select_font_face("Monospace", FontSlant::Normal, FontWeight::Normal);
            cr.set_font_size(14.0);
            let te = cr.text_extents("M").unwrap();
            (te.width(), te.height(), te.y_bearing().abs())
        };

        let init_cols = ((800.0 / char_w).max(1.0) as usize).min(120);
        let init_rows = ((600.0 / char_h).max(1.0) as usize).min(50);
        let grid = Arc::new(Mutex::new(Grid::new(init_cols, init_rows)));

        // spawn a PTY and attach a shell
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: init_rows as u16,
                cols: init_cols as u16,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();

        let mut cmd = CommandBuilder::new("bash");
        cmd.env("TERM", "xterm-256color");
        let _child = pair.slave.spawn_command(cmd).unwrap();
        let master = Arc::new(Mutex::new(pair.master));

        // Take writer once and share it
        let writer_arc: Arc<Mutex<Box<dyn Write + Send>>> = {
            let mguard = master.lock().unwrap();
            Arc::new(Mutex::new(mguard.take_writer().unwrap()))
        };

        // a channel to request redraws on the main loop
        let (tx, rx) = async_channel::unbounded::<()>();
        let area_weak = area.downgrade();
        glib::MainContext::default().spawn_local(async move {
            while let Ok(_) = rx.recv().await {
                if let Some(area) = area_weak.upgrade() {
                    area.queue_draw();
                }
            }
        });

        // Reader thread: read from PTY, feed parser, request redraws
        let grid_clone = Arc::clone(&grid);
        let master_clone = Arc::clone(&master);
        let tx_thread = tx.clone();

        thread::spawn(move || {
            eprintln!("Reader thread started");
            let mut reader = master_clone.lock().unwrap().try_clone_reader().unwrap();
            let mut parser = Parser::new();
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        eprintln!("Reader EOF");
                        break;
                    }
                    Ok(n) => {
                        eprintln!("Read {} bytes: {:?}", n, &buf[..n]);
                        let mut g = grid_clone.lock().unwrap();
                        for &b in &buf[..n] {
                            parser.process(b, &mut *g);
                        }
                        drop(g);
                        let _ = tx_thread.send_blocking(());
                        eprintln!("REDRAW QUEUED (read {} bytes)", n); 
                    }
                    Err(e) => {
                        eprintln!("Reader error: {:?}", e);
                        break;
                    }
                }
            }
        });

        // ----------  send a prompt immediately ----------
        {
            let mut w = writer_arc.lock().unwrap();
            writeln!(w, "echo 'Type something:'").unwrap();
            w.flush().unwrap();
            eprintln!("Echo command sent and flushed");
        }
        let _ = tx.send_blocking(());

        // drawing code — render the grid to the cairo context
        let grid_draw = Arc::clone(&grid);
        let master_draw = Arc::clone(&master);
        let ascent_draw = ascent;
        let char_w_draw = char_w;
        let char_h_draw = char_h;
        area.set_draw_func(move |area, cr, _w, _h| {
            eprintln!("DRAW ENTERED (size: {}x{})", area.width(), area.height()); 
            cr.set_operator(Operator::Clear);
            cr.paint().unwrap();
            cr.set_operator(Operator::Over);

            let cols = (area.width() as f64 / char_w_draw).max(1.0) as usize;
            let rows = (area.height() as f64 / char_h_draw).max(1.0) as usize;
            
            // Handle resize
            {
                let mut g = grid_draw.lock().unwrap();
                if g.cols != cols || g.rows != rows {
                    eprintln!("Resizing grid to {}x{}", cols, rows);
                    
                    // Resize the grid preserving content
                    g.resize(cols, rows);
                    
                    // Resize PTY
                    if let Ok(m) = master_draw.lock() {
                        let _ = m.resize(PtySize {
                            rows: rows as u16,
                            cols: cols as u16,
                            pixel_width: 0,
                            pixel_height: 0,
                        });
                    }
                }
            }

            let g = grid_draw.lock().unwrap();
            let mut non_empty = 0;
            for r in &g.cells {
                for c in r {
                    if c.ch != '\0' { non_empty += 1; }
                }
            }
            eprintln!("Drawing {}x{} grid, cursor at ({},{}) - {} non-empty cells", g.cols, g.rows, g.col, g.row, non_empty);

            for r in 0..g.rows.min(rows) {
                for c in 0..g.cols.min(cols) {
                    let cell = &g.cells[r][c];
                    let x = c as f64 * char_w_draw;
                    let y = r as f64 * char_h_draw;

                    // Always draw background
                    cr.set_source_rgb(cell.bg.r, cell.bg.g, cell.bg.b);
                    cr.rectangle(x, y, char_w_draw, char_h_draw);
                    cr.fill().unwrap();

                    // Draw grid lines (border around cell)
                    cr.set_source_rgb(GRID_LINE_COLOR.r, GRID_LINE_COLOR.g, GRID_LINE_COLOR.b);
                    cr.set_line_width(0.5);
                    cr.rectangle(x, y, char_w_draw, char_h_draw);
                    cr.stroke().unwrap();

                    // Draw text if present
                    if cell.ch != '\0' {
                        cr.set_source_rgb(cell.fg.r, cell.fg.g, cell.fg.b);
                        cr.select_font_face("Monospace", 
                            if cell.italic { FontSlant::Italic } else { FontSlant::Normal },
                            if cell.bold { FontWeight::Bold } else { FontWeight::Normal });
                        cr.set_font_size(14.0);
                        cr.move_to(x, y + ascent_draw);
                        cr.show_text(&cell.ch.to_string()).unwrap();
                    }

                    // Underline after text
                    if cell.underline {
                        cr.set_source_rgb(cell.fg.r, cell.fg.g, cell.fg.b);
                        cr.move_to(x, y + char_h_draw - 1.0);
                        cr.line_to(x + char_w_draw, y + char_h_draw - 1.0);
                        cr.set_line_width(1.0);
                        cr.stroke().unwrap();
                    }
                }
            }

            // Improved cursor - inverted colors
            let cursor_x = g.col as f64 * char_w_draw;
            let cursor_y = g.row as f64 * char_h_draw;
            
            if g.row < g.rows && g.col < g.cols {
                let cursor_cell = &g.cells[g.row][g.col];
                
                // Draw inverted background
                cr.set_source_rgb(cursor_cell.fg.r, cursor_cell.fg.g, cursor_cell.fg.b);
                cr.rectangle(cursor_x, cursor_y, char_w_draw, char_h_draw);
                cr.fill().unwrap();
                
                // Draw character in inverted colors
                if cursor_cell.ch != '\0' {
                    cr.set_source_rgb(cursor_cell.bg.r, cursor_cell.bg.g, cursor_cell.bg.b);
                    cr.select_font_face("Monospace", 
                        if cursor_cell.italic { FontSlant::Italic } else { FontSlant::Normal },
                        if cursor_cell.bold { FontWeight::Bold } else { FontWeight::Normal });
                    cr.set_font_size(14.0);
                    cr.move_to(cursor_x, cursor_y + ascent_draw);
                    cr.show_text(&cursor_cell.ch.to_string()).unwrap();
                }
            }
        });

        // keyboard handling — write keys into PTY master
        let writer_key = Arc::clone(&writer_arc);
        let tx_key = tx.clone();
        area.set_can_focus(true);
        let key_controller = EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, keyval, _keycode, state| {
            let mut w = writer_key.lock().unwrap();
            match keyval {
                gdk::Key::Return => { let _ = w.write_all(b"\r"); }
                gdk::Key::BackSpace => { let _ = w.write_all(b"\x7f"); }
                gdk::Key::Tab => { let _ = w.write_all(b"\t"); }
                gdk::Key::Up => { let _ = w.write_all(b"\x1b[A"); }
                gdk::Key::Down => { let _ = w.write_all(b"\x1b[B"); }
                gdk::Key::Left => { let _ = w.write_all(b"\x1b[D"); }
                gdk::Key::Right => { let _ = w.write_all(b"\x1b[C"); }
                gdk::Key::Escape => { let _ = w.write_all(b"\x1b"); }
                _ if state.contains(gdk::ModifierType::CONTROL_MASK) && keyval == gdk::Key::c => {
                    let _ = w.write_all(b"\x03");
                }
                _ if state.contains(gdk::ModifierType::CONTROL_MASK) && keyval == gdk::Key::d => {
                    let _ = w.write_all(b"\x04");
                }
                _ if state.contains(gdk::ModifierType::CONTROL_MASK) && keyval == gdk::Key::l => {
                    let _ = w.write_all(b"\x0c");
                }
                _ => {
                    if let Some(c) = keyval.to_unicode() {
                        let _ = w.write_all(c.to_string().as_bytes());
                    }
                }
            }
            w.flush().unwrap();
            drop(w);
            let _ = tx_key.send_blocking(());
            Propagation::Stop
        });
        area.add_controller(key_controller);

        // Initial queue draw
        std::thread::sleep(std::time::Duration::from_millis(100));
        area.queue_draw();

        Self { area, char_w, char_h, ascent, grid }
    }

    pub fn widget(&self) -> &DrawingArea { &self.area }
}
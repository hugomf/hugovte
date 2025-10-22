use crate::ansi::{AnsiGrid, Cell, Color, AnsiParser};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use gtk4::prelude::*;
use gtk4::{DrawingArea, EventControllerKey, cairo};
use gtk4::gdk;
use cairo::{FontSlant, FontWeight, Operator};
use std::sync::{Arc, Mutex};
use std::thread;
use std::io::{Read, Write};
use glib::Propagation;

const DEFAULT_FG: Color = Color { r: 1.0, g: 1.0, b: 1.0 };
const DEFAULT_BG: Color = Color { r: 0.0, g: 0.0, b: 0.0 };
const GRID_LINE_COLOR: Color = Color { r: 0.2, g: 0.0, b: 0.0 };
const SCROLLBACK_LIMIT: usize = 1000;
const FONT_SIZE: f64 = 14.0;

/// ----------  Grid  ----------
pub struct Grid {
    pub cols: usize,
    pub rows: usize,
    pub cells: Vec<Vec<Cell>>,
    pub scrollback: Vec<Vec<Cell>>,
    pub scroll_offset: usize,
    pub col: usize,
    pub row: usize,
    pub fg: Color,
    pub bg: Color,
    bold: bool,
    italic: bool,
    underline: bool,
    dim: bool,
    // Selection support
    pub selection_start: Option<(usize, usize)>,
    pub selection_end: Option<(usize, usize)>,
}

impl Grid {
    fn default_cell() -> Cell {
        Cell {
            ch: '\0',
            fg: DEFAULT_FG,
            bg: DEFAULT_BG,
            bold: false,
            italic: false,
            underline: false,
            dim: false,
        }
    }

    pub fn new(cols: usize, rows: usize) -> Self {
        let cells = vec![vec![Self::default_cell(); cols]; rows];
        Self {
            cols,
            rows,
            cells,
            scrollback: Vec::new(),
            scroll_offset: 0,
            col: 0,
            row: 0,
            fg: DEFAULT_FG,
            bg: DEFAULT_BG,
            bold: false,
            italic: false,
            underline: false,
            dim: false,
            selection_start: None,
            selection_end: None,
        }
    }

    pub fn clear(&mut self) {
        for row in &mut self.cells {
            row.fill(Self::default_cell());
        }
        self.col = 0;
        self.row = 0;
        self.scrollback.clear();
        self.scroll_offset = 0;
        self.clear_selection();
    }

    pub fn resize(&mut self, new_cols: usize, new_rows: usize) {
        let mut new_cells = vec![vec![Self::default_cell(); new_cols]; new_rows];

        for (r, old_row) in self.cells.iter().enumerate().take(new_rows) {
            for (c, cell) in old_row.iter().enumerate().take(new_cols) {
                new_cells[r][c] = *cell;
            }
        }

        self.cells = new_cells;
        self.cols = new_cols;
        self.rows = new_rows;
        self.col = self.col.min(new_cols.saturating_sub(1));
        self.row = self.row.min(new_rows.saturating_sub(1));
        self.clear_selection();
    }

    pub fn clear_selection(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
    }

    pub fn set_selection(&mut self, start: (usize, usize), end: (usize, usize)) {
        self.selection_start = Some(start);
        self.selection_end = Some(end);
    }

    pub fn get_selected_text(&self) -> String {
        let (start, end) = match (self.selection_start, self.selection_end) {
            (Some(s), Some(e)) => (s, e),
            _ => return String::new(),
        };

        let total_rows = self.scrollback.len() + self.rows;
        let (start_row, start_col) = start;
        let (end_row, end_col) = end;

        if start_row >= total_rows || end_row >= total_rows {
            return String::new();
        }

        let mut result = String::new();

        for row in start_row..=end_row {
            let line = if row < self.scrollback.len() {
                &self.scrollback[row]
            } else {
                let grid_row = row - self.scrollback.len();
                if grid_row < self.cells.len() {
                    &self.cells[grid_row]
                } else {
                    continue;
                }
            };

            let line_len = line.len();
            let start_c = if row == start_row { start_col.min(line_len.saturating_sub(1)) } else { 0 };
            let end_c = if row == end_row { end_col.min(line_len.saturating_sub(1)) } else { line_len.saturating_sub(1) };

            for col in start_c..=end_c {
                let ch = line.get(col).map_or(' ', |cell| if cell.ch == '\0' { ' ' } else { cell.ch });
                result.push(ch);
            }

            if row < end_row {
                result.push('\n');
            }
        }

        result
    }

    pub fn is_selected(&self, row: usize, col: usize) -> bool {
        let (Some(start), Some(end)) = (self.selection_start, self.selection_end) else {
            return false;
        };

        let (min_row, min_col, max_row, max_col) = if start <= end {
            (start.0, start.1, end.0, end.1)
        } else {
            (end.0, end.1, start.0, start.1)
        };

        row >= min_row && row <= max_row && col >= min_col && col <= max_col
    }
}

impl AnsiGrid for Grid {
    fn put(&mut self, ch: char) {
        if self.col < self.cols && self.row < self.rows {
            self.cells[self.row][self.col] = Cell {
                ch,
                fg: self.fg,
                bg: self.bg,
                bold: self.bold,
                italic: self.italic,
                underline: self.underline,
                dim: self.dim,
            };
        }
    }

    fn advance(&mut self) {
        self.col += 1;
        if self.col >= self.cols {
            self.newline();
        }
    }

    fn left(&mut self, n: usize) {
        self.col = self.col.saturating_sub(n);
    }
    fn right(&mut self, n: usize) {
        self.col = (self.col + n).min(self.cols - 1);
    }
    fn up(&mut self, n: usize) {
        self.row = self.row.saturating_sub(n);
    }
    fn down(&mut self, n: usize) {
        self.row = (self.row + n).min(self.rows - 1);
    }

    fn newline(&mut self) {
        self.col = 0;
        self.row += 1;
        if self.row >= self.rows {
            self.scrollback.push(self.cells.remove(0));
            if self.scrollback.len() > SCROLLBACK_LIMIT {
                self.scrollback.remove(0);
            }
            self.cells.push(vec![Self::default_cell(); self.cols]);
            self.row = self.rows - 1;
        }
    }

    fn carriage_return(&mut self) {
        self.col = 0;
    }
    fn backspace(&mut self) {
        if self.col > 0 {
            self.col -= 1;
        }
    }

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
        let default = Self::default_cell();
        self.cells[self.row].fill(default);
    }

    fn reset_attrs(&mut self) {
        self.fg = DEFAULT_FG;
        self.bg = DEFAULT_BG;
        self.bold = false;
        self.italic = false;
        self.underline = false;
        self.dim = false;
    }

    fn set_bold(&mut self, bold: bool) {
        self.bold = bold;
    }
    fn set_italic(&mut self, italic: bool) {
        self.italic = italic;
    }
    fn set_underline(&mut self, underline: bool) {
        self.underline = underline;
    }
    fn set_dim(&mut self, dim: bool) {
        self.dim = dim;
    }
    fn set_fg(&mut self, color: Color) {
        self.fg = color;
    }
    fn set_bg(&mut self, color: Color) {
        self.bg = color;
    }

    fn get_fg(&self) -> Color {
        self.fg
    }
    fn get_bg(&self) -> Color {
        self.bg
    }
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
        area.set_focusable(true);
        area.grab_focus();

        // Compute font metrics once
        let (char_w, char_h, ascent) = {
            let surf = cairo::ImageSurface::create(cairo::Format::ARgb32, 1, 1).unwrap();
            let cr = cairo::Context::new(&surf).unwrap();
            cr.select_font_face("Monospace", FontSlant::Normal, FontWeight::Normal);
            cr.set_font_size(FONT_SIZE);
            let te = cr.text_extents("M").unwrap();
            (te.width(), te.height(), te.y_bearing().abs())
        };

        let init_cols = ((800.0 / char_w).max(1.0) as usize).min(120);
        let init_rows = ((600.0 / char_h).max(1.0) as usize).min(50);
        let grid = Arc::new(Mutex::new(Grid::new(init_cols, init_rows)));

        // Spawn PTY
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: init_rows as u16,
                cols: init_cols as u16,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("Failed to open PTY");

        let mut cmd = CommandBuilder::new("bash");
        cmd.env("TERM", "xterm-256color");
        pair.slave.spawn_command(cmd).expect("Failed to spawn shell");
        let master = Arc::new(Mutex::new(Some(pair.master)));

        // Shared writer
        let writer_arc: Arc<Mutex<Box<dyn Write + Send>>> = {
            let mut mguard = master.lock().unwrap();
            let writer = mguard.as_mut().unwrap().take_writer().unwrap();
            Arc::new(Mutex::new(writer))
        };

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

        // Reader thread
        let grid_clone = Arc::clone(&grid);
        let master_clone = Arc::clone(&master);
        let tx_thread = tx.clone();

        thread::spawn(move || {
            let mut reader = {  // ← ADD 'mut' HERE
                let mut m = master_clone.lock().unwrap();
                m.as_mut().unwrap().try_clone_reader().unwrap()
            };
            let mut parser = AnsiParser::new();
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        {
                            let mut g = grid_clone.lock().unwrap();
                            for &b in &buf[..n] {
                                parser.process(b, &mut *g);
                            }
                        }
                        let _ = tx_thread.send_blocking(());
                    }
                    Err(e) => {
                        eprintln!("PTY read error: {}", e);
                        break;
                    }
                }
            }
        });

        // Send initial prompt
        {
            let mut w = writer_arc.lock().unwrap();
            writeln!(w, "echo 'Type something:'").unwrap();
            w.flush().unwrap();
        }
        let _ = tx.send_blocking(());

        // Drawing
        let grid_draw = Arc::clone(&grid);
        let master_draw = Arc::clone(&master);
        area.set_draw_func(move |area, cr, _w, _h| {
            cr.set_operator(Operator::Clear);
            cr.paint().unwrap();
            cr.set_operator(Operator::Over);

            let cols = (area.width() as f64 / char_w).max(1.0) as usize;
            let rows = (area.height() as f64 / char_h).max(1.0) as usize;

            // Handle resize
            {
                let mut g = grid_draw.lock().unwrap();
                if g.cols != cols || g.rows != rows {
                    g.resize(cols, rows);
                    if let Ok(m) = master_draw.lock() {
                        if let Some(ref master) = *m {
                            let _ = master.resize(PtySize {
                                rows: rows as u16,
                                cols: cols as u16,
                                pixel_width: 0,
                                pixel_height: 0,
                            });
                        }
                    }
                }
            }

            let g = grid_draw.lock().unwrap();

            for r in 0..g.rows.min(rows) {
                for c in 0..g.cols.min(cols) {
                    let cell = &g.cells[r][c];
                    let x = c as f64 * char_w;
                    let y = r as f64 * char_h;

                    // Background (with selection highlight)
                    if g.is_selected(r + g.scrollback.len(), c) {
                        cr.set_source_rgb(0.3, 0.5, 0.8);
                    } else {
                        cr.set_source_rgb(cell.bg.r, cell.bg.g, cell.bg.b);
                    }
                    cr.rectangle(x, y, char_w, char_h);
                    cr.fill().unwrap();

                    // Grid lines
                    cr.set_source_rgb(GRID_LINE_COLOR.r, GRID_LINE_COLOR.g, GRID_LINE_COLOR.b);
                    cr.set_line_width(0.5);
                    cr.rectangle(x, y, char_w, char_h);
                    cr.stroke().unwrap();

                    // Text
                    if cell.ch != '\0' {
                        cr.set_source_rgb(cell.fg.r, cell.fg.g, cell.fg.b);
                        cr.select_font_face(
                            "Monospace",
                            if cell.italic { FontSlant::Italic } else { FontSlant::Normal },
                            if cell.bold { FontWeight::Bold } else { FontWeight::Normal },
                        );
                        cr.set_font_size(FONT_SIZE);
                        cr.move_to(x, y + ascent);
                        cr.show_text(&cell.ch.to_string()).unwrap();
                    }

                    // Underline
                    if cell.underline {
                        cr.set_source_rgb(cell.fg.r, cell.fg.g, cell.fg.b);
                        cr.move_to(x, y + char_h - 1.0);
                        cr.line_to(x + char_w, y + char_h - 1.0);
                        cr.set_line_width(1.0);
                        cr.stroke().unwrap();
                    }
                }
            }

            // Cursor
            if g.row < g.rows && g.col < g.cols {
                let cursor_x = g.col as f64 * char_w;
                let cursor_y = g.row as f64 * char_h;
                let cursor_cell = &g.cells[g.row][g.col];

                cr.set_source_rgb(cursor_cell.fg.r, cursor_cell.fg.g, cursor_cell.fg.b);
                cr.rectangle(cursor_x, cursor_y, char_w, char_h);
                cr.fill().unwrap();

                if cursor_cell.ch != '\0' {
                    cr.set_source_rgb(cursor_cell.bg.r, cursor_cell.bg.g, cursor_cell.bg.b);
                    cr.select_font_face(
                        "Monospace",
                        if cursor_cell.italic { FontSlant::Italic } else { FontSlant::Normal },
                        if cursor_cell.bold { FontWeight::Bold } else { FontWeight::Normal },
                    );
                    cr.set_font_size(FONT_SIZE);
                    cr.move_to(cursor_x, cursor_y + ascent);
                    cr.show_text(&cursor_cell.ch.to_string()).unwrap();
                }
            }
        });

        // Keyboard
        let writer_key = Arc::clone(&writer_arc);
        let tx_key = tx.clone();
        let grid_key = Arc::clone(&grid);
        let key_controller = EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, keyval, _keycode, state| {
            // Copy
            if (state.contains(gdk::ModifierType::META_MASK) && keyval == gdk::Key::c)
                || (state.contains(gdk::ModifierType::CONTROL_MASK) && keyval == gdk::Key::c)
            {
                let g = grid_key.lock().unwrap();
                let text = g.get_selected_text();
                if !text.is_empty() {
                    if let Some(display) = gdk::Display::default() {
                        display.clipboard().set_text(&text);
                    }
                }
                return Propagation::Stop;
            }

            // Paste — SYNCHRONOUS, NO GIO
            if (state.contains(gdk::ModifierType::META_MASK) && keyval == gdk::Key::v)
                || (state.contains(gdk::ModifierType::CONTROL_MASK) && keyval == gdk::Key::v)
            {
                let clipboard = gdk::Display::default().unwrap().clipboard();
                let writer_key_clone = Arc::clone(&writer_key);
                let tx_key_clone = tx_key.clone();

                clipboard.read_text_async(None::<&gtk4::gio::Cancellable>, move |result| {
                    if let Ok(Some(text)) = result {
                        if let Ok(mut writer) = writer_key_clone.lock() {
                            let _ = writer.write_all(text.as_bytes());
                            let _ = writer.flush();
                            let _ = tx_key_clone.send_blocking(());
                        }
                    }
                });
                return Propagation::Stop;
            }

            // Regular key
            let mut w = writer_key.lock().unwrap();
            match keyval {
                gdk::Key::Return => w.write_all(b"\r").unwrap(),
                gdk::Key::BackSpace => w.write_all(b"\x7f").unwrap(),
                gdk::Key::Tab => w.write_all(b"\t").unwrap(),
                gdk::Key::Up => w.write_all(b"\x1b[A").unwrap(),
                gdk::Key::Down => w.write_all(b"\x1b[B").unwrap(),
                gdk::Key::Left => w.write_all(b"\x1b[D").unwrap(),
                gdk::Key::Right => w.write_all(b"\x1b[C").unwrap(),
                gdk::Key::Escape => w.write_all(b"\x1b").unwrap(),
                _ if state.contains(gdk::ModifierType::CONTROL_MASK) && keyval == gdk::Key::d => {
                    w.write_all(b"\x04").unwrap()
                }
                _ if state.contains(gdk::ModifierType::CONTROL_MASK) && keyval == gdk::Key::l => {
                    w.write_all(b"\x0c").unwrap()
                }
                _ => {
                    if let Some(c) = keyval.to_unicode() {
                        w.write_all(c.to_string().as_bytes()).unwrap();
                    }
                }
            }
            w.flush().unwrap();
            drop(w);
            let _ = tx_key.send_blocking(());
            Propagation::Stop
        });
        area.add_controller(key_controller);

        // Mouse selection
        let grid_mouse = Arc::clone(&grid);
        let tx_mouse = tx.clone();
        let click_controller = gtk4::GestureClick::new();
        click_controller.set_button(0);
        click_controller.connect_pressed(move |_, _, x, y| {
            let mut g = grid_mouse.lock().unwrap();
            let col = (x / char_w) as usize;
            let row = (y / char_h) as usize + g.scrollback.len();
            g.clear_selection();
            g.set_selection((row, col), (row, col));
            drop(g);
            let _ = tx_mouse.send_blocking(());
        });

        let grid_mouse2 = Arc::clone(&grid);
        let tx_mouse2 = tx.clone();
        let motion_controller = gtk4::EventControllerMotion::new();
        motion_controller.connect_motion(move |_, x, y| {
            let mut g = grid_mouse2.lock().unwrap();
            if let Some(start) = g.selection_start {
                let col = (x / char_w) as usize;
                let row = (y / char_h) as usize + g.scrollback.len();
                g.set_selection(start, (row, col));
                drop(g);
                let _ = tx_mouse2.send_blocking(());
            }
        });

        area.add_controller(click_controller);
        area.add_controller(motion_controller);

        area.queue_draw();

        Self {
            area,
            char_w,
            char_h,
            ascent,
            grid,
        }
    }

    pub fn widget(&self) -> &DrawingArea {
        &self.area
    }
}
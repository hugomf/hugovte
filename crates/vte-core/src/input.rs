use crate::grid::Grid;
use gtk4::prelude::*;
use gtk4::{
    DrawingArea, EventControllerKey, EventControllerMotion,
    EventControllerScroll, EventControllerScrollFlags, GestureClick, gdk,
};
use std::io::Write;
use std::sync::{Arc, RwLock, Mutex};
use glib::Propagation;

pub struct InputHandler;

impl InputHandler {
    /* ------------------------------------------------------------------ */
    /*  keyboard                                                            */
    /* ------------------------------------------------------------------ */
    pub fn setup_keyboard(
        area: &DrawingArea,
        grid: Arc<RwLock<Grid>>,
        writer: Arc<Mutex<Box<dyn Write + Send>>>,
        tx: async_channel::Sender<()>,
    ) {
        let key_controller = EventControllerKey::new();

        key_controller.connect_key_pressed(move |_, keyval, _keycode, state| {
            // copy / paste
            if Self::handle_copy_paste(keyval, state, &grid, &writer, &tx) {
                return Propagation::Stop;
            }

            // keyboard scrolling (Shift + Page/Arrow keys)
            if state.contains(gdk::ModifierType::SHIFT_MASK) && Self::handle_scroll_keys(keyval, &grid, &tx) {
                return Propagation::Stop;
            }

            // escape
            if keyval == gdk::Key::Escape {
                Self::handle_escape(&grid, &tx);
                return Propagation::Stop;
            }

            // special keys
            if let Some(seq) = Self::handle_special_keys(keyval, state) {
                Self::write_to_writer(&writer, seq);
                let _ = tx.send_blocking(());
                return Propagation::Stop;
            }

            // unicode
            if let Some(c) = keyval.to_unicode() {
                let mut buf = [0u8; 4];
                Self::write_to_writer(&writer, c.encode_utf8(&mut buf).as_bytes());
                let _ = tx.send_blocking(());
            }

            Propagation::Stop
        });

        area.add_controller(key_controller);
    }

    /* ------------------------------------------------------------------ */
    /*  mouse                                                               */
    /* ------------------------------------------------------------------ */
    pub fn setup_mouse(
        area: &DrawingArea,
        grid: Arc<RwLock<Grid>>,
        tx: async_channel::Sender<()>,
        char_w: f64,
        char_h: f64,
    ) {
        /* ---------- click (press / release) ---------- */
        let click = GestureClick::new();
        click.set_button(0);

        let g = grid.clone();
        let t = tx.clone();
        click.connect_pressed(move |_, _, x, y| {
            let (r, c) = Self::xy_to_cell(x, y, char_w, char_h, &g);
            g.write().map(|mut gr| {
                if !gr.is_selected(r, c) {
                    gr.clear_selection();
                }
                gr.start_selection(r, c);
            }).ok();
            let _ = t.send_blocking(());
        });

        let g = grid.clone();
        let t = tx.clone();
        click.connect_released(move |_, _, x, y| {
            let (r, c) = Self::xy_to_cell(x, y, char_w, char_h, &g);
            g.write().map(|mut gr| {
                if !gr.complete_selection(r, c) && !gr.has_selection() {
                    gr.clear_selection();
                }
            }).ok();
            let _ = t.send_blocking(());
        });

        area.add_controller(click);

        /* ---------- motion ---------- */
        let g = grid.clone();
        let t = tx.clone();
        let motion = EventControllerMotion::new();
        motion.connect_motion(move |_, x, y| {
            let (r, c) = Self::xy_to_cell(x, y, char_w, char_h, &g);

            g.write().map(|mut gr| {
                if gr.is_selecting() {
                    gr.update_selection(r, c);
                    let _ = t.send_blocking(());
                }
            }).ok();
        });
        area.add_controller(motion);

        /* ---------- scroll ---------- */
        let g = grid;
        let t = tx;
        let scroll = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
        scroll.connect_scroll(move |_, _, dy| {
            g.write().map(|mut gr| {
                let lines = (dy * 3.0) as isize;
                gr.scroll_offset = if lines > 0 {
                    gr.scroll_offset.saturating_sub(lines as usize)
                } else {
                    let max = gr.scrollback.len() / gr.cols;
                    (gr.scroll_offset as isize - lines).min(max as isize) as usize
                };
            }).ok();
            let _ = t.send_blocking(());
            Propagation::Stop
        });
        area.add_controller(scroll);
    }

    /* ------------------------------------------------------------------ */
    /*  helpers                                                             */
    /* ------------------------------------------------------------------ */
    #[inline]
    fn xy_to_cell(x: f64, y: f64, cw: f64, ch: f64, grid: &Arc<RwLock<Grid>>) -> (usize, usize) {
        let gr = grid.read().unwrap();
        let c = (x / cw) as usize;
        let screen_r = (y / ch) as usize;
        let scrollback_rows = gr.scrollback.len() / gr.cols;
        let r = if gr.scroll_offset == 0 {
            scrollback_rows + screen_r
        } else {
            scrollback_rows - gr.scroll_offset + screen_r
        };
        (r, c)
    }

    #[inline]
    fn write_to_writer(writer: &Arc<Mutex<Box<dyn Write + Send>>>, data: &[u8]) {
        let _ = writer.lock().map(|mut w| w.write_all(data).and_then(|_| w.flush()));
    }

    fn handle_escape(grid: &Arc<RwLock<Grid>>, tx: &async_channel::Sender<()>) {
        grid.write().map(|mut g| g.clear_selection()).ok();
        let _ = tx.send_blocking(());
    }

    fn handle_scroll_keys(keyval: gdk::Key, grid: &Arc<RwLock<Grid>>, tx: &async_channel::Sender<()>) -> bool {
        use gdk::Key;
        let lines = match keyval {
            Key::Page_Up => 10,
            Key::Page_Down => -10,
            Key::Up => 1,
            Key::Down => -1,
            _ => return false,
        };

        grid.write().map(|mut gr| {
            let new_offset = if lines > 0 {
                gr.scroll_offset.saturating_sub(lines as usize)
            } else {
                let max = (gr.scrollback.len() / gr.cols).max(gr.scroll_offset);
                gr.scroll_offset + (-lines as usize).min(max - gr.scroll_offset)
            };
            gr.scroll_offset = new_offset;
        }).ok();

        let _ = tx.send_blocking(());
        true
    }

    fn handle_copy_paste(
        keyval: gdk::Key,
        state: gdk::ModifierType,
        grid: &Arc<RwLock<Grid>>,
        writer: &Arc<Mutex<Box<dyn Write + Send>>>,
        tx: &async_channel::Sender<()>,
    ) -> bool {
        // copy
        let copy = (state.contains(gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK)
                   && keyval == gdk::Key::c)
            || (state.contains(gdk::ModifierType::META_MASK) && keyval == gdk::Key::c);
        if copy {
            if let Ok(g) = grid.read() {
                if g.has_selection() {
                    let text = g.get_selected_text();
                    if !text.is_empty() {
                        gdk::Display::default().map(|d| d.clipboard().set_text(&text));
                    }

                }
            }
            return true;
        }

        // paste
        let paste = (state.contains(gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK)
                    && keyval == gdk::Key::v)
            || (state.contains(gdk::ModifierType::META_MASK) && keyval == gdk::Key::v);
        if paste {
            let w = writer.clone();
            let t = tx.clone();
            gdk::Display::default()
                .unwrap()
                .clipboard()
                .read_text_async(None::<&gtk4::gio::Cancellable>, move |res| {
                    if let Ok(Some(txt)) = res {
                        Self::write_to_writer(&w, txt.as_bytes());
                        let _ = t.send_blocking(());
                    }
                });
            return true;
        }

        false
    }

    fn handle_special_keys(keyval: gdk::Key, state: gdk::ModifierType) -> Option<&'static [u8]> {
        use gdk::Key;
        match keyval {
            Key::Return => Some(b"\r"),
            Key::BackSpace => Some(b"\x7f"),
            Key::Tab => Some(b"\t"),
            Key::Home => Some(b"\x1b[H"),
            Key::End => Some(b"\x1b[F"),
            Key::Delete => Some(b"\x1b[3~"),
            Key::Insert => Some(b"\x1b[2~"),
            Key::Page_Up => Some(b"\x1b[5~"),
            Key::Page_Down => Some(b"\x1b[6~"),
            Key::Up => Some(b"\x1b[A"),
            Key::Down => Some(b"\x1b[B"),
            Key::Right => Some(b"\x1b[C"),
            Key::Left => Some(b"\x1b[D"),
            Key::F1 => Some(b"\x1bOP"),
            Key::F2 => Some(b"\x1bOQ"),
            Key::F3 => Some(b"\x1bOR"),
            Key::F4 => Some(b"\x1bOS"),
            Key::F5 => Some(b"\x1b[15~"),
            Key::F6 => Some(b"\x1b[17~"),
            Key::F7 => Some(b"\x1b[18~"),
            Key::F8 => Some(b"\x1b[19~"),
            Key::F9 => Some(b"\x1b[20~"),
            Key::F10 => Some(b"\x1b[21~"),
            Key::F11 => Some(b"\x1b[23~"),
            Key::F12 => Some(b"\x1b[24~"),
            _ if state.contains(gdk::ModifierType::CONTROL_MASK) => match keyval {
                Key::d => Some(b"\x04"),
                Key::l => Some(b"\x0c"),
                Key::c => Some(b"\x03"),
                Key::z => Some(b"\x1a"),
                _ => None,
            },
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ansi::Cell;
    use gdk::Key;
    use gtk4::gdk;

    fn b(s: &[u8]) -> &[u8] { s }

    #[test]
    fn special_keys_plain() {
        assert_eq!(InputHandler::handle_special_keys(Key::Return, gdk::ModifierType::empty()), Some(b(b"\r")));
        assert_eq!(InputHandler::handle_special_keys(Key::F1,   gdk::ModifierType::empty()), Some(b(b"\x1bOP")));
        assert_eq!(InputHandler::handle_special_keys(Key::Up,  gdk::ModifierType::empty()), Some(b(b"\x1b[A")));
    }

    #[test]
    fn special_keys_unknown() {
        assert_eq!(InputHandler::handle_special_keys(Key::a, gdk::ModifierType::empty()), None);
    }

    #[test]
    fn copy_paste_hotkeys() {
        use gdk::ModifierType;

        // headless CI has no display
        if gdk::Display::default().is_none() {
            eprintln!("no GDK display, skipping paste half of test");
            return;
        }

        let grid = Arc::new(RwLock::new(Grid::new(0, 0)));
        let writer = Arc::new(Mutex::new(Box::new(std::io::sink()) as Box<dyn Write + Send>));
        let (tx, _rx) = async_channel::bounded(1);

        // copy
        let mut st = ModifierType::CONTROL_MASK | ModifierType::SHIFT_MASK;
        assert!(InputHandler::handle_copy_paste(Key::c, st, &grid, &writer, &tx));
        st = ModifierType::META_MASK;
        assert!(InputHandler::handle_copy_paste(Key::c, st, &grid, &writer, &tx));

        // paste
        let mut st = ModifierType::CONTROL_MASK | ModifierType::SHIFT_MASK;
        assert!(InputHandler::handle_copy_paste(Key::v, st, &grid, &writer, &tx));
        st = ModifierType::META_MASK;
        assert!(InputHandler::handle_copy_paste(Key::v, st, &grid, &writer, &tx));

        // not a hot-key
        assert!(!InputHandler::handle_copy_paste(Key::a, ModifierType::empty(), &grid, &writer, &tx));
    }
    
    #[test]
    fn xy_to_cell_conversion() {
        let grid = Arc::new(RwLock::new(Grid::new(10, 5)));
        {
            let mut g = grid.write().unwrap();
            g.scrollback = (0..30).map(|_| Cell::default()).collect();
        }
        // (0,0)  -> row 3 (30/10), col 0
        let (r, c) = InputHandler::xy_to_cell(0.0, 0.0, 10.0, 10.0, &grid);
        assert_eq!((r, c), (3, 0));

        // (25,15)  -> col 2, row 4  (15/10 = 1.5 -> 1  => 3+1)
        let (r, c) = InputHandler::xy_to_cell(25.0, 15.0, 10.0, 10.0, &grid);
        assert_eq!((r, c), (4, 2));
    }
}

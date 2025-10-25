//! Input handling for GTK4 backend

use gtk4::{DrawingArea, EventControllerKey, EventControllerMotion, EventControllerScroll, GestureClick};
use gtk4::gdk;
use gtk4::prelude::*;
use glib;
use std::sync::{Arc, Mutex};
use std::io::Write;
use vte_core::{InputHandler, EventLoop};
use async_channel::{Sender, Receiver};

/// Combined GTK4 input handler and event loop
pub struct Gtk4EventLoop {
    area: Option<DrawingArea>,
}

impl Gtk4EventLoop {
    pub fn new() -> Self {
        Gtk4EventLoop { area: None }
    }

    pub fn set_area(&mut self, area: &DrawingArea) {
        self.area = Some(area.clone());
    }
}

impl EventLoop for Gtk4EventLoop {
    fn schedule_redraw(&mut self, callback: Box<dyn FnMut()>) {
        if let Some(ref area) = self.area {
            area.queue_draw();

            // Run the callback after draw completes
            let mut callback = callback;
            glib::timeout_add_local_once(std::time::Duration::from_millis(1), move || {
                callback();
            });
        }
    }

    fn schedule_timer(&mut self, interval_ms: u64, callback: Box<dyn FnMut() -> bool>) -> bool {
        let mut callback = callback;
        glib::timeout_add_local(std::time::Duration::from_millis(interval_ms), move || {
            if callback() {
                glib::ControlFlow::Continue
            } else {
                glib::ControlFlow::Break
            }
        });
        true
    }
}

/// GTK4 input handler implementation
pub struct Gtk4InputHandler;

impl Gtk4InputHandler {
    pub fn setup_keyboard(
        area: &DrawingArea,
        grid: Arc<std::sync::RwLock<vte_core::Grid>>,
        writer: Arc<Mutex<Box<dyn Write + Send>>>,
        redraw_tx: Sender<()>,
    ) {
        let key_controller = EventControllerKey::new();

        key_controller.connect_key_pressed(move |_, keyval, _keycode, state| {
            Self::handle_key_event(keyval, state, &grid, &writer, &redraw_tx)
        });

        area.add_controller(key_controller);
    }

    pub fn setup_mouse(
        area: &DrawingArea,
        grid: Arc<std::sync::RwLock<vte_core::Grid>>,
        redraw_tx: Sender<()>,
        char_w: f64,
        char_h: f64,
    ) {
        // Mouse click gestures
        let click_gesture = GestureClick::new();
        click_gesture.set_button(0); // Any button

        click_gesture.connect_pressed(move |gesture, n_press, x, y| {
            let (r, c) = Self::xy_to_cell(x, y, char_w, char_h, &grid);
            let button = gesture.current_button();

            // Handle selection
            if let Ok(mut g) = grid.write() {
                if n_press == 1 {
                    g.start_selection(r, c);
                } else if n_press == 2 {
                    g.select_word(r, c);
                } else if n_press == 3 {
                    g.select_line(r);
                }
                let _ = redraw_tx.send_blocking(());
            }
        });

        click_gesture.connect_released(move |_, _, x, y| {
            let (r, c) = Self::xy_to_cell(x, y, char_w, char_h, &grid);
            if let Ok(mut g) = grid.write() {
                if g.complete_selection(r, c) {
                    let _ = redraw_tx.send_blocking(());
                }
            }
        });

        area.add_controller(click_gesture);

        // Mouse motion for selection dragging
        let motion_controller = EventControllerMotion::new();
        motion_controller.connect_motion(move |_, x, y| {
            let (r, c) = Self::xy_to_cell(x, y, char_w, char_h, &grid);
            if let Ok(mut g) = grid.write() {
                g.update_selection(r, c);
                if g.is_dragging() {
                    let _ = redraw_tx.send_blocking(());
                }
            }
        });

        area.add_controller(motion_controller);

        // Mouse wheel scrolling
        let scroll_controller = EventControllerScroll::new();
        scroll_controller.connect_scroll(move |_, _, dy| {
            if let Ok(mut g) = grid.write() {
                let lines = (dy * 3.0) as isize; // 3 lines per scroll unit
                g.scroll_offset = (g.scroll_offset as isize + lines)
                    .max(0) as usize;
                let _ = redraw_tx.send_blocking(());
            }
            gtk4::Propagation::Stop
        });

        area.add_controller(scroll_controller);
    }

    fn handle_key_event(
        keyval: gdk::Key,
        state: gdk::ModifierType,
        grid: &Arc<std::sync::RwLock<vte_core::Grid>>,
        writer: &Arc<Mutex<Box<dyn Write + Send>>>,
        redraw_tx: &Sender<()>,
    ) -> gtk4::Propagation {
        // Copy/Paste handling
        if Self::handle_copy_paste(keyval, state, grid, writer, redraw_tx) {
            return gtk4::Propagation::Stop;
        }

        // Keyboard scrolling (Shift + Page/Arrow keys)
        if state.contains(gdk::ModifierType::SHIFT_MASK) && Self::handle_scroll_keys(keyval, grid, redraw_tx) {
            return gtk4::Propagation::Stop;
        }

        // Special keys
        if let Some(seq) = Self::handle_special_keys(keyval, state) {
            Self::write_to_writer(writer, &seq);
            let _ = redraw_tx.send_blocking(());
            return gtk4::Propagation::Stop;
        }

        // Unicode input
        if let Some(ch) = keyval.to_unicode() {
            let mut buf = [0u8; 4];
            Self::write_to_writer(writer, ch.encode_utf8(&mut buf).as_bytes());
            let _ = redraw_tx.send_blocking(());
        }

        gtk4::Propagation::Stop
    }

    fn handle_copy_paste(
        keyval: gdk::Key,
        state: gdk::ModifierType,
        grid: &Arc<std::sync::RwLock<vte_core::Grid>>,
        writer: &Arc<Mutex<Box<dyn Write + Send>>>,
        redraw_tx: &Sender<()>,
    ) -> bool {
        // Copy (Ctrl+Shift+C or Cmd+C)
        let copy = (state.contains(gdk::ModifierType::META_MASK) ||
                   state.contains(gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK))
                  && keyval == gdk::Key::c;

        if copy {
            if let Ok(g) = grid.read() {
                if g.has_selection() {
                    let text = g.get_selected_text();
                    if !text.is_empty() {
                        if let Some(display) = gdk::Display::default() {
                            display.clipboard().set_text(&text);
                        }
                    }
                }
            }
            return true;
        }

        // Paste (Ctrl+Shift+V or Cmd+V)
        let paste = (state.contains(gdk::ModifierType::META_MASK) ||
                    state.contains(gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK))
                   && keyval == gdk::Key::v;

        if paste {
            let writer_clone = Arc::clone(writer);
            let tx_clone = redraw_tx.clone();

            if let Some(display) = gdk::Display::default() {
                display.clipboard().read_text_async(None::<&gtk4::gio::Cancellable>, move |res| {
                    if let Ok(Some(text)) = res {
                        Self::write_to_writer(&writer_clone, text.as_bytes());
                        let _ = tx_clone.send_blocking(());
                    }
                });
            }
            return true;
        }

        false
    }

    fn handle_scroll_keys(
        keyval: gdk::Key,
        grid: &Arc<std::sync::RwLock<vte_core::Grid>>,
        redraw_tx: &Sender<()>,
    ) -> bool {
        let lines = match keyval {
            gdk::Key::Page_Up => 10,
            gdk::Key::Page_Down => -10,
            gdk::Key::Up => 1,
            gdk::Key::Down => -1,
            _ => return false,
        };

        if let Ok(mut g) = grid.write() {
            g.scroll_offset = (g.scroll_offset as isize + lines)
                .max(0) as usize;
            let _ = redraw_tx.send_blocking(());
        }
        true
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

    fn xy_to_cell(
        x: f64,
        y: f64,
        char_w: f64,
        char_h: f64,
        grid: &Arc<std::sync::RwLock<vte_core::Grid>>,
    ) -> (usize, usize) {
        let (c, r) = if let Ok(g) = grid.read() {
            (
                (x / char_w) as usize,
                (y / char_h) as usize,
            )
        } else {
            (0, 0)
        };
        (r, c)
    }

    #[inline]
    fn write_to_writer(writer: &Arc<Mutex<Box<dyn Write + Send>>>, data: &[u8]) {
        let _ = writer.lock().map(|mut w| {
            w.write_all(data)?;
            w.flush()
        });
    }
}

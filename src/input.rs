// src/input.rs
use crate::grid::Grid;
use gtk4::prelude::*;
use gtk4::{DrawingArea, EventControllerKey, gdk};
use std::sync::{Arc, RwLock, Mutex};
use std::io::Write;
use glib::Propagation;

pub struct InputHandler;

impl InputHandler {
    /// Setup keyboard input handling
    pub fn setup_keyboard(
        area: &DrawingArea,
        grid: Arc<RwLock<Grid>>,
        writer: Arc<Mutex<Box<dyn Write + Send>>>,
        tx: async_channel::Sender<()>,
    ) {
        let key_controller = EventControllerKey::new();
        
        key_controller.connect_key_pressed(move |_, keyval, _keycode, state| {
            // Copy - Use Ctrl+Shift+C or Cmd+C (avoids conflict with Ctrl+C interrupt)
            if (state.contains(gdk::ModifierType::CONTROL_MASK) 
                && state.contains(gdk::ModifierType::SHIFT_MASK) 
                && keyval == gdk::Key::c)
                || (state.contains(gdk::ModifierType::META_MASK) && keyval == gdk::Key::c)
            {
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
                return Propagation::Stop;
            }

            // Paste - Use Ctrl+Shift+V or Cmd+V
            if (state.contains(gdk::ModifierType::CONTROL_MASK) 
                && state.contains(gdk::ModifierType::SHIFT_MASK) 
                && keyval == gdk::Key::v)
                || (state.contains(gdk::ModifierType::META_MASK) && keyval == gdk::Key::v)
            {
                let clipboard = gdk::Display::default().unwrap().clipboard();
                let writer_clone = Arc::clone(&writer);
                let tx_clone = tx.clone();

                clipboard.read_text_async(None::<&gtk4::gio::Cancellable>, move |result| {
                    if let Ok(Some(text)) = result {
                        if let Ok(mut w) = writer_clone.lock() {
                            let _ = w.write_all(text.as_bytes());
                            let _ = w.flush();
                            let _ = tx_clone.send_blocking(());
                        }
                    }
                });
                return Propagation::Stop;
            }

            // Clear selection on ESC
            if keyval == gdk::Key::Escape {
                if let Ok(mut g) = grid.write() {
                    g.clear_selection();
                }
                let _ = tx.send_blocking(());
                return Propagation::Stop;
            }

            // Handle special keys
            if let Some(sequence) = Self::handle_special_keys(keyval, state) {
                if let Ok(mut w) = writer.lock() {
                    let _ = w.write_all(sequence);
                    let _ = w.flush();
                }
                let _ = tx.send_blocking(());
                return Propagation::Stop;
            }

            // Regular key input
            if let Some(c) = keyval.to_unicode() {
                if let Ok(mut w) = writer.lock() {
                    let _ = w.write_all(c.to_string().as_bytes());
                    let _ = w.flush();
                }
                let _ = tx.send_blocking(());
            }

            Propagation::Stop
        });
        
        area.add_controller(key_controller);
    }

    /// Setup mouse input handling (selection)
    pub fn setup_mouse(
        area: &DrawingArea,
        grid: Arc<RwLock<Grid>>,
        tx: async_channel::Sender<()>,
        char_w: f64,
        char_h: f64,
    ) {
        // Mouse click - start/end selection
        let grid_click = Arc::clone(&grid);
        let tx_click = tx.clone();
        let click_controller = gtk4::GestureClick::new();
        click_controller.set_button(0);
        
        click_controller.connect_pressed(move |_, _, x, y| {
            if let Ok(mut g) = grid_click.write() {
                let col = (x / char_w) as usize;
                let row = (y / char_h) as usize + g.scrollback.len() / g.cols;
                
                if !g.is_selected(row, col) {
                    g.clear_selection();
                }
                g.start_selection(row, col);
            }
            let _ = tx_click.send_blocking(());
        });
        
        let grid_released = Arc::clone(&grid);
        let tx_released = tx.clone();
        click_controller.connect_released(move |_, _, x, y| {
            if let Ok(mut g) = grid_released.write() {
                let col = (x / char_w) as usize;
                let row = (y / char_h) as usize + g.scrollback.len() / g.cols;
                
                let selection_created = g.complete_selection(row, col);
                
                if !selection_created && !g.has_selection() {
                    g.clear_selection();
                }
            }
            let _ = tx_released.send_blocking(());
        });

        // Mouse motion - update selection while dragging
        let grid_motion = Arc::clone(&grid);
        let tx_motion = tx.clone();
        let motion_controller = gtk4::EventControllerMotion::new();
        motion_controller.connect_motion(move |_, x, y| {
            if let Ok(mut g) = grid_motion.write() {
                if g.is_selecting() {
                    let col = (x / char_w) as usize;
                    let row = (y / char_h) as usize + g.scrollback.len() / g.cols;
                    g.update_selection(row, col);
                    let _ = tx_motion.send_blocking(());
                }
            }
        });

        // Mouse wheel - scrolling
        let grid_scroll = Arc::clone(&grid);
        let tx_scroll = tx.clone();
        let scroll_controller = gtk4::EventControllerScroll::new(
            gtk4::EventControllerScrollFlags::VERTICAL
        );
        scroll_controller.connect_scroll(move |_, _, dy| {
            if let Ok(mut g) = grid_scroll.write() {
                let scroll_lines = (dy * 3.0) as isize;
                if scroll_lines > 0 {
                    g.scroll_offset = g.scroll_offset.saturating_sub(scroll_lines as usize);
                } else {
                    let max_scroll = g.scrollback.len() / g.cols;
                    g.scroll_offset = (g.scroll_offset as isize - scroll_lines)
                        .min(max_scroll as isize) as usize;
                }
            }
            let _ = tx_scroll.send_blocking(());
            Propagation::Stop
        });

        area.add_controller(click_controller);
        area.add_controller(motion_controller);
        area.add_controller(scroll_controller);
    }

    /// Convert special keys to ANSI sequences
    fn handle_special_keys(keyval: gdk::Key, state: gdk::ModifierType) -> Option<&'static [u8]> {
        match keyval {
            gdk::Key::Return => Some(b"\r"),
            gdk::Key::BackSpace => Some(b"\x7f"),
            gdk::Key::Tab => Some(b"\t"),
            gdk::Key::Up => Some(b"\x1b[A"),
            gdk::Key::Down => Some(b"\x1b[B"),
            gdk::Key::Left => Some(b"\x1b[D"),
            gdk::Key::Right => Some(b"\x1b[C"),
            gdk::Key::Home => Some(b"\x1b[H"),
            gdk::Key::End => Some(b"\x1b[F"),
            gdk::Key::Delete => Some(b"\x1b[3~"),
            gdk::Key::Insert => Some(b"\x1b[2~"),
            gdk::Key::Page_Up => Some(b"\x1b[5~"),
            gdk::Key::Page_Down => Some(b"\x1b[6~"),
            gdk::Key::F1 => Some(b"\x1bOP"),
            gdk::Key::F2 => Some(b"\x1bOQ"),
            gdk::Key::F3 => Some(b"\x1bOR"),
            gdk::Key::F4 => Some(b"\x1bOS"),
            gdk::Key::F5 => Some(b"\x1b[15~"),
            gdk::Key::F6 => Some(b"\x1b[17~"),
            gdk::Key::F7 => Some(b"\x1b[18~"),
            gdk::Key::F8 => Some(b"\x1b[19~"),
            gdk::Key::F9 => Some(b"\x1b[20~"),
            gdk::Key::F10 => Some(b"\x1b[21~"),
            gdk::Key::F11 => Some(b"\x1b[23~"),
            gdk::Key::F12 => Some(b"\x1b[24~"),
            _ if state.contains(gdk::ModifierType::CONTROL_MASK) && keyval == gdk::Key::d => {
                Some(b"\x04") // Ctrl+D
            }
            _ if state.contains(gdk::ModifierType::CONTROL_MASK) && keyval == gdk::Key::l => {
                Some(b"\x0c") // Ctrl+L
            }
            _ if state.contains(gdk::ModifierType::CONTROL_MASK) && keyval == gdk::Key::c => {
                Some(b"\x03") // Ctrl+C (interrupt)
            }
            _ if state.contains(gdk::ModifierType::CONTROL_MASK) && keyval == gdk::Key::z => {
                Some(b"\x1a") // Ctrl+Z
            }
            _ => None,
        }
    }
}
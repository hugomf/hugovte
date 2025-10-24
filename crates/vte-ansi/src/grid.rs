use crate::color::Color;

/// Grid cell with styling information
#[derive(Clone, Copy, Default, Debug)]
pub struct Cell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub dim: bool,
}

/// Key event for input handling
#[derive(Debug, Clone)]
pub struct KeyEvent {
    pub keyval: u32,
    pub state: u32,
}

/// Mouse event for input handling
#[derive(Debug, Clone)]
pub struct MouseEvent {
    pub button: u32,
    pub x: f64,
    pub y: f64,
    pub modifiers: u32,
}

/// Trait for ANSI escape sequence grid operations
pub trait AnsiGrid {
    fn put(&mut self, ch: char);
    fn advance(&mut self);
    fn left(&mut self, n: usize);
    fn right(&mut self, n: usize);
    fn up(&mut self, n: usize);
    fn down(&mut self, n: usize);
    fn newline(&mut self);
    fn carriage_return(&mut self);
    fn backspace(&mut self);
    fn move_rel(&mut self, dx: i32, dy: i32);
    fn move_abs(&mut self, row: usize, col: usize);
    fn clear_screen(&mut self);
    fn clear_line(&mut self);
    fn reset_attrs(&mut self);
    fn set_bold(&mut self, bold: bool);
    fn set_italic(&mut self, italic: bool);
    fn set_underline(&mut self, underline: bool);
    fn set_dim(&mut self, dim: bool);
    fn set_fg(&mut self, color: Color);
    fn set_bg(&mut self, color: Color);
    fn set_title(&mut self, title: &str) {
        let _ = title;
    }
    fn get_fg(&self) -> Color;
    fn get_bg(&self) -> Color;

    // Phase-2 extensions with default no-op impls
    fn clear_screen_down(&mut self) {}
    fn clear_screen_up(&mut self) {}
    fn clear_line_right(&mut self) {}
    fn clear_line_left(&mut self) {}
    fn save_cursor(&mut self) {}
    fn restore_cursor(&mut self) {}
    fn set_cursor_visible(&mut self, _visible: bool) {}

    // Phase-2 scrolling operations
    fn scroll_up(&mut self, _n: usize) {}
    fn scroll_down(&mut self, _n: usize) {}

    // Phase-4 line operations
    fn insert_lines(&mut self, _n: usize) {}
    fn delete_lines(&mut self, _n: usize) {}

    // Phase-4 character operations
    fn insert_chars(&mut self, _n: usize) {}
    fn delete_chars(&mut self, _n: usize) {}
    fn erase_chars(&mut self, _n: usize) {}

    // Phase-4 alternate screen
    fn use_alternate_screen(&mut self, _enable: bool) {}

    // Phase-4 additional modes
    fn set_insert_mode(&mut self, _enable: bool) {}
    fn set_auto_wrap(&mut self, _enable: bool) {}

    // Phase-2 DEC private modes
    fn set_application_cursor_keys(&mut self, _enable: bool) {}
    fn set_mouse_reporting_mode(&mut self, _mode: u16, _enable: bool) {}
    fn set_focus_reporting(&mut self, _enable: bool) {}

    // Phase-2 OSC sequences
    fn set_current_directory(&mut self, _directory: &str) {}
    fn handle_clipboard_data(&mut self, _clipboard_id: u8, _data: &str) {}
    fn handle_hyperlink(&mut self, _params: Option<&str>, _uri: &str) {}

    // Bracketed paste mode
    fn set_bracketed_paste_mode(&mut self, _enable: bool) {}

    // Synchronized output mode
    fn set_synchronized_output(&mut self, _enable: bool) {}

    // Keypad mode (Application vs Numeric)
    fn set_keypad_mode(&mut self, _application: bool) {}
}

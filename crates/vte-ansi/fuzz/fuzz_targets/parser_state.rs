//! Fuzzing target for ANSI parser state management

#![no_main]
use libfuzzer_sys::fuzz_target;

use vte_ansi::{AnsiParser, AnsiGrid, Color};

#[derive(Default)]
struct FuzzGrid {
    output_len: usize,
}

impl AnsiGrid for FuzzGrid {
    fn put(&mut self, _: char) { self.output_len = self.output_len.saturating_add(1); }
    fn advance(&mut self) {}
    fn left(&mut self, _: usize) {}
    fn right(&mut self, _: usize) {}
    fn up(&mut self, _: usize) {}
    fn down(&mut self, _: usize) {}
    fn newline(&mut self) {}
    fn carriage_return(&mut self) {}
    fn backspace(&mut self) {}
    fn move_rel(&mut self, _: i32, _: i32) {}
    fn move_abs(&mut self, _: usize, _: usize) {}
    fn clear_screen(&mut self) {}
    fn clear_line(&mut self) {}
    fn reset_attrs(&mut self) {}
    fn set_bold(&mut self, _: bool) {}
    fn set_italic(&mut self, _: bool) {}
    fn set_underline(&mut self, _: bool) {}
    fn set_dim(&mut self, _: bool) {}
    fn set_fg(&mut self, _: Color) {}
    fn set_bg(&mut self, _: Color) {}
    fn get_fg(&self) -> Color { Color::default() }
    fn get_bg(&self) -> Color { Color::default() }
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    // Limit input size to prevent timeouts
    let data = if data.len() > 10000 {
        &data[..10000]
    } else {
        data
    };

    let mut parser = AnsiParser::new();
    let mut grid = FuzzGrid::default();

    // Test state management with various input streams
    let text = String::from_utf8_lossy(data);
    parser.feed_str(&text, &mut grid);

    // Stats should be reasonable
    let stats = parser.stats();
    assert!(stats.max_params_seen <= 32, "max_params_seen too large");
    assert!(stats.max_osc_length_seen <= 2048, "osc length too large");
});

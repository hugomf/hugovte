#![no_main]
use libfuzzer_sys::fuzz_target;
use vte_ansi::{AnsiParser, AnsiGrid, Color};

#[derive(Default)]
struct FuzzGrid;

impl AnsiGrid for FuzzGrid {
    fn put(&mut self, _: char) {}
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
    if data.is_empty() || data.len() > 50 {
        return;
    }
    
    let mut parser = AnsiParser::new();
    let mut grid = FuzzGrid::default();
    
    let mut seq = String::from("\x1B[");
    for (i, &byte) in data.iter().enumerate() {
        if i > 0 { seq.push(';'); }
        seq.push_str(&((byte as u16) % 300).to_string());
        if i >= 10 { break; }
    }
    seq.push('m');
    
    parser.feed_str(&seq, &mut grid);
});
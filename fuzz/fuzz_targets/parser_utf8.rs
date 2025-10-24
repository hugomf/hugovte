#![no_main]
use libfuzzer_sys::fuzz_target;
use hugovte::ansi::{AnsiParser, AnsiGrid, Cell, Color};

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
    let mut parser = AnsiParser::new();
    let mut grid = FuzzGrid::default();
    
    if let Ok(s) = std::str::from_utf8(data) {
        parser.feed_str(s, &mut grid);
    } else {

        for &byte in data {
            parser.process(byte, &mut grid);
        }

    }
});
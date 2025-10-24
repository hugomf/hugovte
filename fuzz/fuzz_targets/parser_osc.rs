#![no_main]
use libfuzzer_sys::fuzz_target;
use hugovte::ansi::{AnsiParser, AnsiGrid, Color};

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
    if data.is_empty() {
        return;
    }
    
    let mut parser = AnsiParser::new();
    let mut grid = FuzzGrid::default();
    
    // Test OSC with BEL terminator
    let mut seq = Vec::from(b"\x1B]0;" as &[u8]);
    seq.extend_from_slice(&data[..data.len().min(100)]);
    seq.push(0x07);
    for &byte in data {
        parser.process(byte, &mut grid);
    }
    
    // Reset and test with ST terminator
    parser = AnsiParser::new();
    grid = FuzzGrid::default();
    
    let mut seq2 = Vec::from(b"\x1B]0;" as &[u8]);
    seq2.extend_from_slice(&data[..data.len().min(100)]);
    seq2.extend_from_slice(b"\x1B\\");

    for &byte in data {
        parser.process(byte, &mut grid);
    }

});
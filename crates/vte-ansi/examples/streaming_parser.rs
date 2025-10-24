//! Streaming parser example
//!
//! This example demonstrates how to parse streamed data incrementally,
//! useful for processing large files or continuous data streams.

use std::io::{self, Read};
use vte_ansi::{AnsiParser, AnsiGrid, Color};

// Simple stream processor that counts characters and sequences
struct StreamProcessor {
    char_count: usize,
    sequence_count: usize,
    total_processed: usize,
}

impl StreamProcessor {
    fn new() -> Self {
        Self {
            char_count: 0,
            sequence_count: 0,
            total_processed: 0,
        }
    }

    fn print_stats(&self) {
        println!("Processed: {} bytes", self.total_processed);
        println!("Characters: {}", self.char_count);
        println!("ANSI sequences: {}", self.sequence_count);
    }
}

impl AnsiGrid for StreamProcessor {
    fn put(&mut self, _ch: char) {
        self.char_count += 1;
    }

    fn advance(&mut self) {}

    // Implement minimal trait requirements
    fn left(&mut self, _n: usize) {}
    fn right(&mut self, _n: usize) {}
    fn up(&mut self, _n: usize) {}
    fn down(&mut self, _n: usize) {}
    fn newline(&mut self) {}
    fn carriage_return(&mut self) {}
    fn backspace(&mut self) {}
    fn move_rel(&mut self, _dx: i32, _dy: i32) {}
    fn move_abs(&mut self, _row: usize, _col: usize) {}
    fn clear_screen(&mut self) {}
    fn clear_line(&mut self) {}
    fn reset_attrs(&mut self) {}
    fn set_bold(&mut self, _bold: bool) {}
    fn set_italic(&mut self, _italic: bool) {}
    fn set_underline(&mut self, _underline: bool) {}
    fn set_dim(&mut self, _dim: bool) {}
    fn set_fg(&mut self, _color: Color) {}
    fn set_bg(&mut self, _color: Color) {}
    fn get_fg(&self) -> Color { Color::default() }
    fn get_bg(&self) -> Color { Color::rgb(0., 0., 0.) }
}

fn main() {
    println!("Streaming ANSI Parser Example");
    println!("Reading from stdin... (Ctrl+D or Ctrl+C to exit)\n");

    let mut parser = AnsiParser::new();
    let mut processor = StreamProcessor::new();
    let mut buffer = [0u8; 1024]; // 1KB chunks

    loop {
        match io::stdin().read(&mut buffer) {
            Ok(0) => break, // EOF
            Ok(n) => {
                // Convert bytes to string (assume valid UTF-8 for demo)
                if let Ok(chunk) = std::str::from_utf8(&buffer[..n]) {
                    // Record how many bytes processed
                    processor.total_processed += chunk.len();

                    // Parse the chunk
                    parser.feed_str(chunk, &mut processor);
                }
            }
            Err(e) => {
                eprintln!("Error reading stdin: {}", e);
                break;
            }
        }

        // Every 10KB, print progress
        if processor.total_processed.is_multiple_of(10000) && processor.total_processed > 0 {
            print!("\rStreaming... {}", processor.total_processed);
            io::Write::flush(&mut io::stdout()).unwrap();
        }
    }

    println!("\n\nProcessing complete!");
    processor.print_stats();

    let stats = parser.stats();
    println!("LTParser stats:");
    println!("  - Sequences parsed: {}", stats.sequences_processed);
    println!("  - Max params seen: {}", stats.max_params_seen);
    println!("  - Max OSC length: {}", stats.max_osc_length_seen);

    println!("\nThis demonstrates streaming ANSI parsing for continuous data streams!");
}

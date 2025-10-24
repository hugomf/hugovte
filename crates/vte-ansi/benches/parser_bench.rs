use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use vte_ansi::{AnsiParser, AnsiGrid, Color};

/// Minimal grid implementation for benchmarking
#[derive(Default)]
struct BenchGrid {
    fg: Color,
    bg: Color,
    bold: bool,
    italic: bool,
    underline: bool,
    dim: bool,
    row: usize,
    col: usize,
    output_count: usize,
}

impl AnsiGrid for BenchGrid {
    fn put(&mut self, _: char) { self.output_count += 1; }
    fn advance(&mut self) { self.col += 1; }
    fn left(&mut self, n: usize) { self.col = self.col.saturating_sub(n); }
    fn right(&mut self, n: usize) { self.col += n; }
    fn up(&mut self, n: usize) { self.row = self.row.saturating_sub(n); }
    fn down(&mut self, n: usize) { self.row += n; }
    fn newline(&mut self) { self.row += 1; self.col = 0; }
    fn carriage_return(&mut self) { self.col = 0; }
    fn backspace(&mut self) { self.col = self.col.saturating_sub(1); }
    fn move_rel(&mut self, dx: i32, dy: i32) {
        self.col = (self.col as i32 + dx).max(0) as usize;
        self.row = (self.row as i32 + dy).max(0) as usize;
    }
    fn move_abs(&mut self, row: usize, col: usize) {
        self.row = row;
        self.col = col;
    }
    fn clear_screen(&mut self) { self.row = 0; self.col = 0; }
    fn clear_line(&mut self) {}
    fn reset_attrs(&mut self) {
        self.fg = Color::default();
        self.bg = Color::rgb(0., 0., 0.);
        self.bold = false;
        self.italic = false;
        self.underline = false;
        self.dim = false;
    }
    fn set_bold(&mut self, v: bool) { self.bold = v; }
    fn set_italic(&mut self, v: bool) { self.italic = v; }
    fn set_underline(&mut self, v: bool) { self.underline = v; }
    fn set_dim(&mut self, v: bool) { self.dim = v; }
    fn set_fg(&mut self, c: Color) { self.fg = c; }
    fn set_bg(&mut self, c: Color) { self.bg = c; }
    fn get_fg(&self) -> Color { self.fg }
    fn get_bg(&self) -> Color { self.bg }
}

fn bench_plain_text(c: &mut Criterion) {
    let mut group = c.benchmark_group("plain_text");
    
    for size in [100, 1000, 10000] {
        let text = "a".repeat(size);
        
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &text, |b, text| {
            b.iter(|| {
                let mut parser = AnsiParser::new();
                let mut grid = BenchGrid::default();
                parser.feed_str(black_box(text), &mut grid);
            });
        });
    }
    group.finish();
}

fn bench_plain_text_with_newlines(c: &mut Criterion) {
    let mut group = c.benchmark_group("plain_text_newlines");
    
    for lines in [10, 100, 1000] {
        let text = format!("{}\n", "Hello world".repeat(10)).repeat(lines);
        let size = text.len();
        
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(lines), &text, |b, text| {
            b.iter(|| {
                let mut parser = AnsiParser::new();
                let mut grid = BenchGrid::default();
                parser.feed_str(black_box(text), &mut grid);
            });
        });
    }
    group.finish();
}

fn bench_colored_output(c: &mut Criterion) {
    let mut group = c.benchmark_group("colored_output");
    
    // Simulate ls --color output
    let ls_line = "\x1B[0m\x1B[01;34mdir\x1B[0m  \x1B[01;32mexec.sh\x1B[0m  \x1B[0mfile.txt\x1B[0m\n";
    
    for count in [10, 100, 1000] {
        let text = ls_line.repeat(count);
        let size = text.len();
        
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &text, |b, text| {
            b.iter(|| {
                let mut parser = AnsiParser::new();
                let mut grid = BenchGrid::default();
                parser.feed_str(black_box(text), &mut grid);
            });
        });
    }
    group.finish();
}

fn bench_sgr_sequences(c: &mut Criterion) {
    let mut group = c.benchmark_group("sgr_sequences");
    
    let patterns = vec![
        ("simple", "\x1B[31mRed\x1B[0m"),
        ("complex", "\x1B[1;4;31;44mBold Underline Red on Blue\x1B[0m"),
        ("256color", "\x1B[38;5;196mBright Red\x1B[0m"),
        ("truecolor", "\x1B[38;2;255;128;0mOrange\x1B[0m"),
    ];
    
    for (name, pattern) in patterns {
        let text = pattern.repeat(100);
        let size = text.len();
        
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), &text, |b, text| {
            b.iter(|| {
                let mut parser = AnsiParser::new();
                let mut grid = BenchGrid::default();
                parser.feed_str(black_box(text), &mut grid);
            });
        });
    }
    group.finish();
}

fn bench_cursor_movement(c: &mut Criterion) {
    let mut group = c.benchmark_group("cursor_movement");
    
    let patterns = vec![
        ("simple", "\x1B[AText\x1B[B"),
        ("absolute", "\x1B[10;20HText"),
        ("save_restore", "\x1B[sText\x1B[u"),
        ("complex", "\x1B[5A\x1B[3C\x1B[2B\x1B[1DText"),
    ];
    
    for (name, pattern) in patterns {
        let text = pattern.repeat(100);
        let size = text.len();
        
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), &text, |b, text| {
            b.iter(|| {
                let mut parser = AnsiParser::new();
                let mut grid = BenchGrid::default();
                parser.feed_str(black_box(text), &mut grid);
            });
        });
    }
    group.finish();
}

fn bench_mixed_content(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_content");
    
    // Realistic terminal output with mixed content
    let mixed = concat!(
        "\x1B[32muser@host\x1B[0m:\x1B[34m~/dir\x1B[0m$ ls -la\n",
        "total 48\n",
        "drwxr-xr-x  5 user user 4096 Jan 1 12:00 \x1B[34m.\x1B[0m\n",
        "drwxr-xr-x 10 user user 4096 Jan 1 11:00 \x1B[34m..\x1B[0m\n",
        "-rw-r--r--  1 user user  220 Jan 1 10:00 file.txt\n",
        "\x1B[32muser@host\x1B[0m:\x1B[34m~/dir\x1B[0m$ ",
    );
    
    for count in [10, 100, 1000] {
        let text = mixed.repeat(count);
        let size = text.len();
        
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &text, |b, text| {
            b.iter(|| {
                let mut parser = AnsiParser::new();
                let mut grid = BenchGrid::default();
                parser.feed_str(black_box(text), &mut grid);
            });
        });
    }
    group.finish();
}

fn bench_osc_sequences(c: &mut Criterion) {
    let mut group = c.benchmark_group("osc_sequences");
    
    let patterns = vec![
        ("title", "\x1B]0;Window Title\x07"),
        ("title_long", "\x1B]0;Very Long Window Title With Many Characters\x07"),
        ("hyperlink", "\x1B]8;;https://example.com\x07Link\x1B]8;;\x07"),
    ];
    
    for (name, pattern) in patterns {
        let text = pattern.repeat(100);
        let size = text.len();
        
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), &text, |b, text| {
            b.iter(|| {
                let mut parser = AnsiParser::new();
                let mut grid = BenchGrid::default();
                parser.feed_str(black_box(text), &mut grid);
            });
        });
    }
    group.finish();
}

fn bench_utf8_content(c: &mut Criterion) {
    let mut group = c.benchmark_group("utf8_content");
    
    let patterns = vec![
        ("ascii", "Hello World! "),
        ("latin", "Héllo Wörld! "),
        ("chinese", "你好世界 "),
        ("emoji", "Hello 🌍 "),
        ("mixed", "Hello 世界 🎉 Привет "),
    ];
    
    for (name, pattern) in patterns {
        let text = pattern.repeat(100);
        let size = text.len();
        
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), &text, |b, text| {
            b.iter(|| {
                let mut parser = AnsiParser::new();
                let mut grid = BenchGrid::default();
                parser.feed_str(black_box(text), &mut grid);
            });
        });
    }
    group.finish();
}

fn bench_worst_case_escapes(c: &mut Criterion) {
    let mut group = c.benchmark_group("worst_case");
    
    // Patterns that might be slower
    let long_osc = format!("\x1B]0;{}\x07", "x".repeat(1000));
    let patterns = vec![
        ("many_params", "\x1B[1;2;3;4;5;6;7;8;9;10m".to_string()),
        ("long_osc", long_osc),
        ("rapid_sgr", "\x1B[31m\x1B[32m\x1B[33m\x1B[34m\x1B[35m".to_string()),
    ];
    
    for (name, pattern) in patterns {
        let text = pattern.repeat(100);
        let size = text.len();
        
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), &text, |b, text| {
            b.iter(|| {
                let mut parser = AnsiParser::new();
                let mut grid = BenchGrid::default();
                parser.feed_str(black_box(text), &mut grid);
            });
        });
    }
    group.finish();
}

fn bench_parser_reuse(c: &mut Criterion) {
    c.bench_function("parser_reuse", |b| {
        let text = "\x1B[31mRed\x1B[0m Normal \x1B[32mGreen\x1B[0m\n";
        let mut parser = AnsiParser::new();
        let mut grid = BenchGrid::default();
        
        b.iter(|| {
            parser.feed_str(black_box(text), &mut grid);
            grid.output_count = 0; // Reset for next iteration
        });
    });
}

fn bench_streaming_chunks(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_chunks");
    
    let full_text = "Hello World! ".repeat(1000);
    
    for chunk_size in [10, 100, 1000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(chunk_size),
            &chunk_size,
            |b, &chunk_size| {
                b.iter(|| {
                    let mut parser = AnsiParser::new();
                    let mut grid = BenchGrid::default();
                    
                    for chunk in full_text.as_bytes().chunks(chunk_size) {
                        if let Ok(s) = std::str::from_utf8(chunk) {
                            parser.feed_str(black_box(s), &mut grid);
                        }
                    }
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_plain_text,
    bench_plain_text_with_newlines,
    bench_colored_output,
    bench_sgr_sequences,
    bench_cursor_movement,
    bench_mixed_content,
    bench_osc_sequences,
    bench_utf8_content,
    bench_worst_case_escapes,
    bench_parser_reuse,
    bench_streaming_chunks,
);

criterion_main!(benches);
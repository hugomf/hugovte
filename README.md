# HugoVTE - Terminal Emulator

A modern terminal emulator written in Rust using GTK4, featuring full ANSI escape sequence support, text selection, and customizable appearance.

## Features

- ✅ **Full ANSI Escape Sequence Support** - Colors, text formatting, cursor movement
- ✅ **256-Color and RGB Color Support** - Modern terminal color capabilities
- ✅ **Text Selection & Clipboard** - Copy/paste functionality
- ✅ **Customizable Appearance** - Fonts, colors, transparency
- ✅ **PTY Integration** - Real terminal functionality
- ✅ **Cross-Platform** - Linux, macOS, Windows support
- ✅ **GTK4 Native** - Modern, responsive UI

## Quick Start

### Running the Terminal

```bash
cargo run
```

### Running Examples

```bash
# ANSI color and formatting demo
cargo run --example sgr_demo

# This will also generate ansi_demo.html for viewing in a browser
```

## Library Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
hugovte = "0.1.0"
```

### Basic Example

```rust
use hugovte::ansi::{AnsiParser, AnsiGrid, Color, Cell};
use hugovte::terminal::VteTerminal;
use hugovte::config::TerminalConfig;

// Create a simple grid for parsing ANSI sequences
struct SimpleGrid {
    cells: Vec<Cell>,
    current_cell: Cell,
}

impl AnsiGrid for SimpleGrid {
    fn put(&mut self, ch: char) {
        self.current_cell.ch = ch;
        self.cells.push(self.current_cell);
    }

    fn advance(&mut self) {
        // Implementation...
    }

    // ... implement other required methods
}

// Parse colored text
let mut parser = AnsiParser::new();
let mut grid = SimpleGrid::new();

parser.feed_str("\x1B[1;31mHello \x1B[32mWorld\x1B[0m!", &mut grid);

// Access parsed cells with colors and formatting
for cell in &grid.cells {
    println!("Char: {}, Color: {}, Bold: {}",
             cell.ch, cell.fg, cell.bold);
}
```

## Configuration

```rust
use hugovte::config::TerminalConfig;
use hugovte::ansi::Color;

let config = TerminalConfig::default()
    .with_font_size(14.0)
    .with_font_family("Monaco")
    .with_background_color(Color::rgba(0.0, 0.0, 0.0, 0.8)) // Semi-transparent
    .with_grid_lines(true);
```

## Building

### Dependencies

- Rust 1.70+
- GTK4 development libraries
- Cairo development libraries
- Pango development libraries

#### Ubuntu/Debian
```bash
sudo apt install libgtk-4-dev libcairo2-dev libpango1.0-dev
```

#### macOS
```bash
brew install gtk4 cairo pango
```

#### Windows
Install GTK4 from [gtk.org](https://gtk.org)

### Development

```bash
# Run tests
cargo test

# Check code quality
cargo check

# Run with debug info
cargo run

# Build release
cargo build --release
```

## Architecture

- **`ansi.rs`** - ANSI escape sequence parsing and color management
- **`terminal.rs`** - Main terminal widget and PTY integration
- **`grid.rs`** - Terminal grid state and cell management
- **`drawing.rs`** - Font rendering and text metrics
- **`input.rs`** - Keyboard and mouse input handling
- **`selection.rs`** - Text selection state machine
- **`config.rs`** - Terminal configuration options

## License

MIT License - see LICENSE file for details.

## Contributing

Contributions welcome! Please read the contributing guidelines and submit pull requests.

## Examples

See the `examples/` directory for usage demonstrations:

- **`sgr_demo.rs`** - Comprehensive ANSI color and formatting demo

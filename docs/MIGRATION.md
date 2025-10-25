# Migration Guide

## Overview

This guide helps developers migrate from other terminal emulator libraries to the VTE Terminal Emulator. It covers migration from popular alternatives and provides examples for common use cases.

## From GTK VTE (GNOME Terminal)

### Before (GTK VTE)
```rust
use vte::prelude::*;
use vte::{Terminal, TerminalExt};

// Create terminal
let terminal = Terminal::new();
terminal.spawn_sync(
    vte::PtyFlags::DEFAULT,
    None,
    &["bash"],
    None,
    glib::SpawnFlags::DEFAULT,
    None,
    None,
);

// Connect to signals
terminal.connect_child_exited(|_| {
    println!("Child exited");
});
```

### After (VTE Terminal)
```rust
use vte_core::{VteTerminalCore, TerminalConfig};
use vte_gtk4::VteTerminalWidget;

// Create terminal widget
let terminal = VteTerminalWidget::new();

// Configure terminal
let config = TerminalConfig::default()
    .with_font_size(13.0)
    .with_colors(
        Color::rgb(1.0, 1.0, 1.0),    // White foreground
        Color::rgb(0.0, 0.0, 0.0)     // Black background
    );

// Access core functionality if needed
let core = terminal.core();
core.feed_bytes(b"echo 'Hello from VTE!'\n");
```

### Key Changes
- **Widget Creation:** Use `VteTerminalWidget::new()` instead of `Terminal::new()`
- **Configuration:** Use `TerminalConfig` builder pattern
- **Signals:** Connect to widget signals instead of terminal signals
- **PTY Management:** Handled automatically by the widget

## From Alacritty Terminal

### Before (Alacritty Terminal)
```rust
use alacritty_terminal::{
    config::Config,
    event::{Event, WindowSize},
    term::{Term, TermMode},
    tty::{EventedPty, Options},
};

// Create PTY
let pty = EventedPty::new(Options {
    shell: Some("/bin/bash".into()),
    ..Default::default()
})?;

// Create terminal
let config = Config::default();
let size = WindowSize {
    num_lines: 24,
    num_cols: 80,
    cell_width: 8,
    cell_height: 16,
};
let mut terminal = Term::new(&config, size, pty.0);

// Process events
loop {
    match event_queue.next()? {
        Event::Pty(data) => {
            terminal.write(data);
        }
        Event::Resize(size) => {
            terminal.resize(size);
        }
        // ... other events
    }
}
```

### After (VTE Terminal)
```rust
use vte_core::{VteTerminalCore, TerminalConfig};
use vte_gtk4::VteTerminalWidget;

// Create terminal with configuration
let config = TerminalConfig::default()
    .with_font_size(13.0)
    .with_scrollback_size(1000);

let mut terminal = VteTerminalCore::new(config);

// Feed data (equivalent to alacritty's write)
terminal.feed_bytes(b"echo 'Hello from VTE!'\n");

// Resize (equivalent to alacritty's resize)
terminal.resize(80, 24);

// For GTK integration
let widget = VteTerminalWidget::new();
```

### Key Changes
- **Simplified API:** No need for separate PTY and terminal management
- **Configuration:** Builder pattern instead of config files
- **Event Handling:** Integrated event loop instead of manual event processing
- **Memory Management:** Automatic cleanup instead of manual resource management

## From WezTerm

### Before (WezTerm)
```rust
use wezterm_term::{Terminal, TerminalSize};

// Create terminal
let mut terminal = Terminal::new(
    TerminalSize {
        rows: 24,
        cols: 80,
        cell_width: 8,
        cell_height: 16,
        dpi: 96,
    },
    Default::default(),
    "bash",
    None,
    None,
)?;

// Process input
terminal.key_down(&wezterm_term::KeyCode::Char('a'), &wezterm_term::KeyModifiers::default());
terminal.key_down(&wezterm_term::KeyCode::Enter, &wezterm_term::KeyModifiers::default());

// Render
let screen = terminal.render();
```

### After (VTE Terminal)
```rust
use vte_core::{VteTerminalCore, TerminalConfig, KeyEvent};

// Create terminal
let config = TerminalConfig::default();
let mut terminal = VteTerminalCore::new(config);

// Feed input (equivalent to wezterm's key_down)
terminal.feed_bytes(b"a\n");

// Access rendered content
let grid = terminal.grid();
let cell = grid.get_cell(0, 0);
println!("Character: {}", cell.ch);
```

### Key Changes
- **Input Method:** Use `feed_bytes()` instead of key events
- **Rendering:** Access grid directly instead of render API
- **State Management:** Grid-based state instead of screen-based
- **Configuration:** Rust-native configuration instead of Lua

## From Windows Terminal/ConPTY

### Before (Windows Terminal)
```rust
use windows::Win32::System::Console::{
    CreatePseudoConsole, ResizePseudoConsole, ClosePseudoConsole,
    HPCON, COORD,
};

// Create pseudo console
let mut console_size = COORD { X: 80, Y: 24 };
let h_pc = CreatePseudoConsole(
    console_size,
    h_input,
    h_output,
    0,
    &mut h_pc,
)?;

// Resize
ResizePseudoConsole(h_pc, console_size);
```

### After (VTE Terminal)
```rust
use vte_core::{VteTerminalCore, TerminalConfig};

// Create terminal (works cross-platform)
let config = TerminalConfig::default();
let mut terminal = VteTerminalCore::new(config);

// Resize (cross-platform)
terminal.resize(80, 24);

// PTY is managed automatically
```

### Key Changes
- **Cross-Platform:** No need for platform-specific PTY code
- **Simplified API:** Automatic PTY management
- **No Handles:** No need to manage Windows handles or file descriptors
- **Automatic Cleanup:** Resources cleaned up automatically

## From Custom Terminal Implementation

### Before (Custom Terminal)
```rust
struct MyTerminal {
    grid: Vec<Vec<char>>,
    cursor_x: usize,
    cursor_y: usize,
    pty: Pty,
}

impl MyTerminal {
    fn new() -> Self {
        Self {
            grid: vec![vec![' '; 80]; 24],
            cursor_x: 0,
            cursor_y: 0,
            pty: Pty::new(),
        }
    }

    fn handle_escape_sequence(&mut self, seq: &str) {
        // Custom ANSI parsing
        match seq {
            "[2J" => self.clear_screen(),
            "[1;1H" => self.move_cursor(0, 0),
            // ... more parsing
        }
    }
}
```

### After (VTE Terminal)
```rust
use vte_core::{VteTerminalCore, TerminalConfig, AnsiGrid};

struct MyTerminal {
    core: VteTerminalCore,
}

impl MyTerminal {
    fn new() -> Self {
        let config = TerminalConfig::default();
        Self {
            core: VteTerminalCore::new(config),
        }
    }

    fn handle_input(&mut self, data: &[u8]) {
        // Automatic ANSI parsing and grid management
        self.core.feed_bytes(data);
    }

    fn render(&self) -> &Grid {
        // Direct grid access
        self.core.grid()
    }
}

// Grid implements AnsiGrid trait automatically
```

### Key Changes
- **ANSI Parsing:** Built-in comprehensive parser instead of custom implementation
- **Grid Management:** Automatic grid operations instead of manual cursor management
- **State Handling:** Automatic state preservation (cursor, attributes, scrollback)
- **Error Handling:** Built-in error recovery instead of manual error handling

## Configuration Migration

### Color Configuration
```rust
// Before: Various formats
let colors = Colors {
    background: Rgb { r: 0, g: 0, b: 0 },
    foreground: Rgb { r: 255, g: 255, b: 255 },
    // ...
};

// After: Normalized color format
let config = TerminalConfig::default()
    .with_colors(
        Color::rgb(1.0, 1.0, 1.0),    // Foreground (0.0-1.0 range)
        Color::rgb(0.0, 0.0, 0.0)     // Background
    );
```

### Font Configuration
```rust
// Before: Font configuration varies
let font_config = FontConfig {
    name: "Monaco".to_string(),
    size: 13.0,
    // ...
};

// After: Simplified font configuration
let config = TerminalConfig::default()
    .with_font_family("Monaco")
    .with_font_size(13.0);
```

### Feature Flags
```rust
// Before: Compile-time features
[features]
default = ["mouse", "scrollback"]
mouse = []
scrollback = []

// After: Runtime configuration
let config = TerminalConfig::default()
    .with_mouse_enabled(true)
    .with_scrollback_size(1000);
```

## API Compatibility Layer

For easier migration, consider creating a compatibility layer:

```rust
// src/compat/alacritty.rs
pub struct AlacrittyCompat {
    core: VteTerminalCore,
}

impl AlacrittyCompat {
    pub fn new(config: TerminalConfig) -> Self {
        Self {
            core: VteTerminalCore::new(config),
        }
    }

    // Alacritty-compatible API
    pub fn write(&mut self, data: &[u8]) {
        self.core.feed_bytes(data);
    }

    pub fn resize(&mut self, size: WindowSize) {
        self.core.resize(size.num_cols, size.num_lines);
    }

    pub fn render(&self) -> &Grid {
        self.core.grid()
    }
}
```

## Performance Migration

### Memory Usage
- **Before:** Manual memory management and buffer allocation
- **After:** Automatic memory management with configurable limits
```rust
let config = TerminalConfig::default()
    .with_scrollback_size(1000)  // Limit scrollback memory
    .with_memory_limit(50 * 1024 * 1024); // 50MB total limit
```

### CPU Usage
- **Before:** Manual optimization and caching
- **After:** Built-in optimizations with performance targets
```rust
// Performance is automatically optimized
// - <2ms redraw for 80x24 grid
// - >10MB/s PTY throughput
// - Efficient memory layout
```

## Testing Migration

### Unit Tests
```rust
// Before: Custom test utilities
#[test]
fn test_cursor_movement() {
    let mut terminal = MyTerminal::new();
    terminal.handle_escape_sequence("[2;3H");
    assert_eq!(terminal.cursor(), (2, 3));
}

// After: Built-in testing utilities
#[test]
fn test_cursor_movement() {
    let mut terminal = VteTerminalCore::new(TerminalConfig::default());
    terminal.feed_bytes(b"\x1b[3;4H"); // Move to row 3, col 4
    let grid = terminal.grid();
    assert_eq!(grid.row, 2); // 0-indexed
    assert_eq!(grid.col, 3);
}
```

### Integration Tests
```rust
// Before: Manual PTY setup
#[test]
fn test_vim_integration() {
    let mut pty = Pty::new();
    pty.write(b"vim test.txt\n");
    // Manual verification
}

// After: Built-in integration testing
#[test]
fn test_vim_integration() {
    let mut terminal = VteTerminalCore::new(TerminalConfig::default());
    terminal.feed_bytes(b"vim test.txt\n");
    // Automatic state verification
    let grid = terminal.grid();
    assert!(grid.using_alternate); // Vim uses alternate screen
}
```

## Common Migration Patterns

### 1. Replace Manual ANSI Parsing
```rust
// Before
fn parse_ansi(input: &str) -> TerminalAction {
    // Custom parsing logic
    match input {
        "[2J" => ClearScreen,
        "[1;1H" => MoveCursor(0, 0),
        // ... hundreds of cases
    }
}

// After
let mut terminal = VteTerminalCore::new(config);
terminal.feed_bytes(input.as_bytes()); // Automatic parsing
```

### 2. Replace Manual Grid Management
```rust
// Before
struct Grid {
    cells: Vec<Vec<char>>,
    cursor_x: usize,
    cursor_y: usize,
}

impl Grid {
    fn move_cursor(&mut self, x: usize, y: usize) {
        self.cursor_x = x;
        self.cursor_y = y;
    }

    fn put_char(&mut self, ch: char) {
        self.cells[self.cursor_y][self.cursor_x] = ch;
        self.cursor_x += 1;
    }
}

// After
let grid = terminal.grid(); // Direct access to managed grid
let cell = grid.get_cell(row, col); // Safe access with bounds checking
```

### 3. Replace Manual PTY Management
```rust
// Before
struct Terminal {
    pty: Pty,
    reader: BufReader,
    writer: BufWriter,
}

impl Terminal {
    fn read_output(&mut self) -> Result<String> {
        let mut buffer = [0; 1024];
        let n = self.reader.read(&mut buffer)?;
        Ok(String::from_utf8_lossy(&buffer[..n]).to_string())
    }
}

// After
let terminal = VteTerminalCore::new(config);
// PTY is managed automatically, just feed bytes
terminal.feed_bytes(output_bytes);
```

## Troubleshooting Migration

### Common Issues

#### Issue: "Method not found"
**Problem:** API methods have different names
**Solution:** Check the API documentation for correct method names
```rust
// Before: terminal.write(data)
// After: terminal.feed_bytes(data)

// Before: terminal.get_cursor()
// After: terminal.grid().row, terminal.grid().col
```

#### Issue: "Type mismatch"
**Problem:** Different type systems
**Solution:** Use the provided conversion utilities
```rust
// Color conversion
let color = Color::rgb(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);

// Size conversion
terminal.resize(cols as usize, rows as usize);
```

#### Issue: "Performance regression"
**Problem:** Different performance characteristics
**Solution:** Adjust configuration for your use case
```rust
let config = TerminalConfig::default()
    .with_scrollback_size(10000)  // Increase for more history
    .with_memory_limit(100 * 1024 * 1024); // Increase memory limit
```

#### Issue: "Missing features"
**Problem:** Some advanced features not yet implemented
**Solution:** Check feature flags and roadmap
```rust
// Enable optional features
let config = TerminalConfig::default()
    .with_bracketed_paste(true)
    .with_mouse_reporting(true);

// Check if feature is available
if cfg!(feature = "sixel") {
    // Sixel graphics available
}
```

## Migration Checklist

### Pre-Migration
- [ ] Identify current terminal library and version
- [ ] Document current configuration and features used
- [ ] Check compatibility with target applications (tmux, vim, etc.)
- [ ] Set up test environment with current implementation

### During Migration
- [ ] Start with simple configuration and basic functionality
- [ ] Migrate core features first (text input/output, colors)
- [ ] Add advanced features incrementally (mouse, clipboard, etc.)
- [ ] Update tests to use new API
- [ ] Verify compatibility with target applications

### Post-Migration
- [ ] Performance testing and optimization
- [ ] Security audit of new implementation
- [ ] User acceptance testing
- [ ] Documentation updates
- [ ] Rollback plan if issues arise

## Support and Resources

### Documentation
- [API Documentation](https://docs.rs/vte-core)
- [Architecture Guide](docs/ARCHITECTURE.md)
- [Compatibility Matrix](docs/COMPATIBILITY.md)
- [Migration Examples](examples/)

### Community
- **GitHub Issues:** Report migration issues and get help
- **Discussions:** Share migration experiences
- **Examples:** Community-contributed migration examples

### Professional Support
- **Consulting:** Available for complex migrations
- **Training:** Migration workshops and training sessions
- **Custom Development:** Backend implementations for specific needs

## Version Compatibility

### API Stability
- **0.1.x:** Beta API, may have breaking changes
- **1.0.0+:** Stable API with backward compatibility guarantees
- **Migration Path:** Clear upgrade path between versions

### Feature Flags
- **Current:** Core features available
- **Beta:** Advanced features as feature flags
- **Stable:** All features in default build

## Conclusion

Migrating to the VTE Terminal Emulator provides:

- **Better Performance:** Optimized rendering and memory usage
- **Enhanced Security:** Built-in security features and safe defaults
- **Improved Compatibility:** Comprehensive ANSI/VT support
- **Modern Architecture:** Trait-based backends and clean APIs
- **Active Development:** Regular updates and community support

The migration process is designed to be straightforward, with comprehensive documentation and examples to help developers transition smoothly.

**Next Steps:**
1. Review the compatibility documentation for your use case
2. Start with a simple migration example
3. Test with your target applications
4. Reach out if you need assistance

Happy migrating! 🚀

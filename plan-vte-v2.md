# VTE Terminal - Complete Production Readiness Plan (Hybrid)

**Goal:** Production-ready, GTK-agnostic VTE terminal emulator with full tmux/zellij/ratatui/vim compatibility

**Timeline:** 42-50 hours (2-3 weeks to beta, 1 week feedback, then 1.0)  
**Strategy:** Your practical 2-week plan + critical additions from comprehensive roadmap  
**Target:** 0.1.0-beta.1 → feedback → 0.1.0 → post-1.0 roadmap

---

## Phase 0: Pre-Development Setup (Week 0)

**Goal:** Establish baseline and fix critical issues  
**Estimated Time:** 2-3 hours  
**Priority:** CRITICAL

### 0.1 Security & Unicode Foundation
- [ ] Add `unicode-segmentation` dependency for grapheme clusters (15 min)
- [ ] Add `unicode-width` dependency for CJK character detection (15 min)
- [ ] Add `unicode-bidi` dependency for RTL text support (15 min)
- [ ] Create `SECURITY.md` with threat model and vulnerability reporting (1 hour)
  ```markdown
  # Security Policy
  
  ## Threat Model
  - Malicious escape sequences (resource exhaustion, code injection)
  - Paste-based attacks (bracketed paste mitigates)
  - File URI exposure in OSC sequences
  
  ## Mitigations
  - Input sanitization (MAX_PARAMS, MAX_OSC_LEN)
  - Bracketed paste mode enabled by default
  - Optional OSC sequence filtering
  
  ## Reporting Vulnerabilities
  Email: security@example.com
  ```
- [ ] Add paste sanitization helper function (30 min)
  ```rust
  pub fn sanitize_paste(text: &str, bracketed: bool) -> String {
      if bracketed {
          format!("\x1b[200~{}\x1b[201~", text)
      } else {
          text.replace('\x1b', "").replace('\r', "\n")
      }
  }
  ```

### 0.2 Documentation Structure
- [ ] Create `docs/` directory structure (15 min)
  ```
  docs/
  ├── ARCHITECTURE.md
  ├── SECURITY.md
  ├── COMPATIBILITY.md
  └── ACCESSIBILITY.md
  ```

---

## Phase 1: Core Functionality (Week 1, Days 1-3)

**Goal:** Establish compositor-agnostic architecture with alternate screen  
**Estimated Time:** 18-21 hours  
**Priority:** CRITICAL

### 1.1 Core Library Structure ⭐ HIGHEST PRIORITY (Day 1: 6-7 hours)

#### Define Modular Traits
```rust
// vte-core/src/traits.rs

/// Main renderer trait composed of sub-renderers
pub trait Renderer {
    fn text_renderer(&mut self) -> &mut dyn TextRenderer;
    fn graphics_renderer(&mut self) -> &mut dyn GraphicsRenderer;
    fn ui_renderer(&mut self) -> &mut dyn UIRenderer;
}

/// Text rendering operations
pub trait TextRenderer {
    fn draw_cell(&mut self, row: usize, col: usize, cell: &Cell);
    fn set_font(&mut self, family: &str, size: f64);
    fn get_char_metrics(&self, ch: char) -> CharMetrics;
}

/// Graphics rendering (images, sixel)
pub trait GraphicsRenderer {
    fn draw_sixel(&mut self, data: &[u8], x: usize, y: usize);
    fn draw_image(&mut self, image: ImageData, x: usize, y: usize);
}

/// UI operations (clearing, flushing)
pub trait UIRenderer {
    fn clear(&mut self);
    fn flush(&mut self);
    fn set_cursor_shape(&mut self, shape: CursorShape);
}

/// Input handling abstraction
pub trait InputHandler {
    fn handle_key(&mut self, key: KeyEvent, grid: &Arc<RwLock<Grid>>, 
                   writer: &Arc<Mutex<Box<dyn Write + Send>>>);
    fn handle_mouse(&mut self, event: MouseEvent, grid: &Arc<RwLock<Grid>>);
    fn handle_scroll(&mut self, delta: f64, grid: &Arc<RwLock<Grid>>);
}

/// Event loop abstraction
pub trait EventLoop {
    fn schedule_redraw(&mut self, callback: Box<dyn FnMut()>);
    fn schedule_timer(&mut self, interval_ms: u64, callback: Box<dyn FnMut() -> bool>);
}

/// Character metrics for font rendering
#[derive(Debug, Clone, Copy)]
pub struct CharMetrics {
    pub width: f64,
    pub height: f64,
    pub ascent: f64,
}
```

#### Tasks
- [ ] Create `vte-core` crate with no GUI dependencies (30 min)
  ```toml
  [package]
  name = "vte-core"
  version = "0.1.0-beta.1"
  edition = "2021"
  rust-version = "1.70"
  
  [dependencies]
  portable-pty = "~0.10"
  memchr = "~2.7"
  thiserror = "~1.0"
  unicode-segmentation = "~1.10"
  unicode-width = "~0.1"
  unicode-bidi = "~0.3"
  ```
- [ ] Move `AnsiParser`, `Grid`, and `Selection` to `vte-core` (1 hour)
- [ ] Define all trait interfaces in `vte-core/src/traits.rs` (1 hour)
- [ ] Implement `VteTerminalCore` struct (2 hours)
  ```rust
  pub struct VteTerminalCore {
      grid: Arc<RwLock<Grid>>,
      parser: AnsiParser,
      pty_pair: Arc<RwLock<Option<portable_pty::PtyPair>>>,
      config: TerminalConfig,
  }
  
  impl VteTerminalCore {
      pub fn new(config: TerminalConfig) -> Self { /* ... */ }
      pub fn feed_bytes(&mut self, data: &[u8]) { /* ... */ }
      pub fn render(&self, renderer: &mut dyn Renderer) { /* ... */ }
  }
  ```
- [ ] Implement `DummyBackend` for testing (1 hour)
- [ ] Add unit tests for core initialization (30 min)

### 1.2 Backend-Agnostic Font Rendering (Day 1: 2 hours)

- [ ] Refactor `DrawingCache` to use `fontdue` instead of Cairo (1.5 hours)
  ```rust
  pub struct FontCache {
      fonts: HashMap<(FontSlant, FontWeight), fontdue::Font>,
      glyph_cache: HashMap<(char, FontSlant, FontWeight), fontdue::Metrics>,
  }
  ```
- [ ] Add `get_char_metrics()` method (30 min)
- [ ] Add tests for font metrics (30 min)

### 1.3 GTK4 Backend (Day 2: 2-3 hours)

#### Create vte-gtk4 Crate
```
vte-gtk4/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── renderer.rs      // Implements Renderer traits
│   ├── input.rs         // Implements InputHandler
│   ├── event_loop.rs    // Implements EventLoop
│   └── widget.rs        // Public GTK widget
```

- [ ] Create `vte-gtk4` crate with gtk4/cairo dependencies (30 min)
- [ ] Implement `Gtk4TextRenderer` using Cairo (1 hour)
- [ ] Implement `Gtk4InputHandler` for keyboard/mouse (1 hour)
- [ ] Implement `Gtk4EventLoop` using glib::timeout_add_local (30 min)
- [ ] Create `VteTerminalWidget` wrapper (30 min)
- [ ] Add integration tests (30 min)

### 1.4 Alternate Screen Buffer (Day 2: 3.5 hours)

- [ ] Add `alternate_cells: Vec<Cell>` to Grid (30 min)
  ```rust
  pub struct Grid {
      pub cells: Vec<Cell>,
      pub alternate_cells: Vec<Cell>,
      pub using_alternate: bool,
      // ... existing fields
  }
  ```
- [ ] Implement `use_alternate_screen(bool)` with state preservation (1.5 hours)
  ```rust
  impl Grid {
      pub fn use_alternate_screen(&mut self, enable: bool) {
          if enable && !self.using_alternate {
              // Save cursor, attributes
              self.saved_cursor = Some((self.row, self.col));
              self.saved_attrs = Some((self.fg, self.bg, self.bold, ...));
              // Swap buffers
              std::mem::swap(&mut self.cells, &mut self.alternate_cells);
              self.using_alternate = true;
          } else if !enable && self.using_alternate {
              // Swap back
              std::mem::swap(&mut self.cells, &mut self.alternate_cells);
              // Restore cursor, attributes
              if let Some((row, col)) = self.saved_cursor.take() {
                  self.row = row;
                  self.col = col;
              }
              self.using_alternate = false;
          }
      }
  }
  ```
- [ ] Update `resize()` to handle both buffers (30 min)
- [ ] Modify `clear_screen()` to affect only active buffer (15 min)
- [ ] Add comprehensive tests (1 hour)
  - Test buffer switching preserves state
  - Test resize with alternate screen
  - Test with vim/tmux scenarios

### 1.5 Error Handling (Day 3: 3.5-4.5 hours)

#### Define Error Hierarchy
```rust
#[derive(Debug, thiserror::Error)]
pub enum TerminalError {
    #[error("PTY error: {0}")]
    Pty(#[from] portable_pty::Error),
    
    #[error("Rendering error: {0}")]
    Rendering(String),
    
    #[error("Parser error: {0}")]
    Parser(#[from] AnsiError),
    
    #[error("Font error: {0}")]
    Font(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

#### Tasks
- [ ] Define `TerminalError` enum with hierarchy (1 hour)
- [ ] Replace all `unwrap()`/`expect()` with `Result<T, TerminalError>` (1.5 hours)
- [ ] Implement error recovery strategy (1 hour)
  ```rust
  impl VteTerminalCore {
      pub fn recover_from_error(&mut self, error: TerminalError) -> Result<(), TerminalError> {
          match error {
              TerminalError::Pty(_) => self.restart_pty(),
              TerminalError::Rendering(_) => self.fallback_rendering(),
              TerminalError::Font(_) => self.use_fallback_font(),
              _ => Err(error),
          }
      }
      
      fn restart_pty(&mut self) -> Result<(), TerminalError> {
          // Attempt to respawn PTY
          eprintln!("PTY connection lost, attempting restart...");
          // Implementation
      }
  }
  ```
- [ ] Integrate `AnsiParser::ErrorCallback` with core error reporting (30 min)
- [ ] Add tests for error handling and recovery (1 hour)

### 1.6 Resource Management (Day 3: 2.5 hours)

- [ ] Implement `Drop` for `VteTerminalCore` (30 min)
  ```rust
  impl Drop for VteTerminalCore {
      fn drop(&mut self) {
          if let Ok(mut pair) = self.pty_pair.write() {
              *pair = None; // Close PTY
          }
      }
  }
  ```
- [ ] Add memory limits for scrollback (30 min)
  ```rust
  const MAX_SCROLLBACK_MB: usize = 50;
  
  impl Grid {
      pub fn enforce_memory_limit(&mut self) {
          let bytes = self.scrollback.len() * std::mem::size_of::<Cell>();
          if bytes > MAX_SCROLLBACK_MB * 1024 * 1024 {
              let excess = bytes - MAX_SCROLLBACK_MB * 1024 * 1024;
              let cells_to_remove = excess / std::mem::size_of::<Cell>();
              self.scrollback.drain(0..cells_to_remove);
          }
      }
  }
  ```
- [ ] Add timeout for PTY reader thread (30 min)
- [ ] Add tests for resource cleanup and limits (1 hour)

---

## Phase 2: Complete ANSI and VTE Support (Week 2, Days 1-2)

**Goal:** Full terminal feature support for modern TUI apps  
**Estimated Time:** 11-13 hours  
**Priority:** HIGH

### 2.1 Missing ANSI and VTE Sequences (8-9 hours)

#### DEC Private Modes
- [ ] Implement in `AnsiParser` (1 hour)
  - `CSI ?1h/l` - Application cursor keys
  - `CSI ?47h/l` - Alternate screen (for compatibility)
  - `CSI ?1049h/l` - Save cursor + alternate screen
  - `CSI ?25h/l` - Cursor visibility
  - `CSI ?7h/l` - Auto-wrap mode
  - `CSI ?6h/l` - Origin mode (DECOM)

#### Mouse Reporting
- [ ] Implement all mouse modes in `AnsiParser` and `InputHandler` (1.5 hours)
  - `CSI ?1000h/l` - X10 mouse reporting
  - `CSI ?1002h/l` - Button event tracking
  - `CSI ?1005h/l` - UTF-8 mouse mode
  - `CSI ?1006h/l` - SGR mouse mode (recommended)
  - `CSI ?1004h/l` - Focus events (send `CSI I`/`CSI O`)

#### Character Sets
- [ ] Support DEC Special Graphics (line drawing) (1 hour)
  ```rust
  const DEC_SPECIAL_GRAPHICS: [(char, char); 31] = [
      ('q', '─'), ('x', '│'), ('l', '┐'), ('k', '┘'),
      ('m', '└'), ('j', '┌'), ('t', '├'), ('u', '┤'),
      ('v', '┴'), ('w', '┬'), ('n', '┼'),
      // ... full set
  ];
  ```
- [ ] Implement ISO-2022 character set switching (30 min)
- [ ] Handle ESC `(`, `)`, `*`, `+` designation (30 min)

#### Clipboard Operations
- [ ] Implement OSC 52 (copy/paste with base64) (45 min)
  ```rust
  // OSC 52 ; c ; base64_data ST
  fn handle_osc_52(&mut self, clipboard: &str, data: &str) {
      if let Ok(decoded) = base64::decode(data) {
          if let Ok(text) = String::from_utf8(decoded) {
              // Set clipboard based on 'c' (clipboard type)
              self.set_system_clipboard(&text);
          }
      }
  }
  ```

#### Bracketed Paste
- [ ] Implement `CSI ?2004h/l` in `AnsiParser` and `InputHandler` (30 min)
  ```rust
  impl InputHandler {
      fn handle_paste(&mut self, text: &str, bracketed: bool) {
          if bracketed {
              self.send(b"\x1b[200~");
              self.send(text.as_bytes());
              self.send(b"\x1b[201~");
          } else {
              self.send(text.as_bytes());
          }
      }
  }
  ```

#### Hyperlinks
- [ ] Implement OSC 8 hyperlinks (1 hour)
  ```rust
  // Add to Cell struct
  pub struct Cell {
      // ... existing fields
      pub hyperlink: Option<Arc<Hyperlink>>,
  }
  
  #[derive(Debug, Clone)]
  pub struct Hyperlink {
      pub id: String,
      pub uri: String,
      pub params: HashMap<String, String>,
  }
  
  // Parse OSC 8 ; params ; URI ST
  fn handle_osc_8(&mut self, text: &str) {
      if let Some((params, uri)) = text.split_once(';') {
          if uri.is_empty() {
              self.current_hyperlink = None;
          } else {
              self.current_hyperlink = Some(Arc::new(Hyperlink {
                  id: extract_id(params),
                  uri: uri.to_string(),
                  params: parse_params(params),
              }));
          }
      }
  }
  ```
- [ ] Update `UIRenderer` to handle hyperlink clicks (30 min)

#### Unicode Enhancements (NEW - 2 hours)
- [ ] **Grapheme cluster support** (1 hour)
  ```rust
  use unicode_segmentation::UnicodeSegmentation;
  
  impl Grid {
      pub fn put_grapheme(&mut self, grapheme: &str) {
          // Store entire grapheme in cell
          let cell = self.get_cell_mut(self.row, self.col);
          cell.grapheme = Some(grapheme.to_string());
          
          // Handle emoji with modifiers (👨‍👩‍👧‍👦)
          let width = UnicodeWidthStr::width(grapheme);
          if width > 1 {
              cell.wide = true;
              // Mark continuation cells
              for i in 1..width {
                  if self.col + i < self.cols {
                      let next = self.get_cell_mut(self.row, self.col + i);
                      next.wide_continuation = true;
                  }
              }
          }
      }
  }
  ```
- [ ] **CJK wide character handling** (30 min)
  ```rust
  use unicode_width::UnicodeWidthChar;
  
  fn is_wide_char(ch: char) -> bool {
      ch.width().unwrap_or(1) == 2
  }
  ```
- [ ] **RTL text support (basic)** (30 min)
  ```rust
  use unicode_bidi::{BidiInfo, bidi_class};
  
  fn detect_rtl(text: &str) -> bool {
      let bidi_info = BidiInfo::new(text, None);
      bidi_info.has_rtl()
  }
  ```

#### Tests
- [ ] Add comprehensive tests for all sequences (2 hours)
- [ ] Test with vttest (aim for 90%+)
- [ ] Test with real apps (tmux, zellij, vim, ratatui)

### 2.2 Shell Integration (Day 2: 2 hours)

#### VTE Profile Script
- [ ] Set environment variables in `spawn_pty()` (30 min)
  ```rust
  fn spawn_pty() -> portable_pty::PtyPair {
      let mut cmd = CommandBuilder::new("bash");
      cmd.env("VTE_VERSION", "1");
      cmd.env("VTE_INSTANCE_ID", uuid::Uuid::new_v4().to_string());
      
      // Source vte.sh if present
      if Path::new("/etc/profile.d/vte.sh").exists() {
          cmd.arg("-c");
          cmd.arg("source /etc/profile.d/vte.sh && exec bash");
      }
      // ... spawn
  }
  ```

#### OSC 7 Directory Tracking
- [ ] Parse OSC 7 in `AnsiParser` (30 min)
  ```rust
  // OSC 7 ; file://hostname/path ST
  fn handle_osc_7(&mut self, text: &str) {
      if let Some(path) = text.strip_prefix("file://") {
          if let Some((_, path)) = path.split_once('/') {
              self.current_directory = Some(PathBuf::from(format!("/{}", path)));
          }
      }
  }
  ```
- [ ] Store current directory in Grid (15 min)

#### OSC 133 Semantic Prompts (NEW - 30 min)
- [ ] Parse OSC 133 sequences
  ```rust
  // OSC 133 ; A ST - Prompt start
  // OSC 133 ; B ST - Prompt end  
  // OSC 133 ; C ST - Command start
  // OSC 133 ; D ST - Command end
  
  #[derive(Debug, Clone)]
  pub enum SemanticZone {
      Prompt { start_row: usize, end_row: usize },
      Command { start_row: usize, end_row: usize },
      Output { start_row: usize, end_row: usize },
  }
  
  impl Grid {
      pub fn mark_semantic_zone(&mut self, zone_type: SemanticZone) {
          self.semantic_zones.push(zone_type);
      }
  }
  ```

#### Tests
- [ ] Test directory tracking with tmux/zellij split-panes (30 min)
- [ ] Test OSC 133 with bash/zsh integration (30 min)

### 2.3 Input Handling Enhancements (Day 2: 1-2 hours)

- [ ] Ensure modifier key consistency (Alt, Super, Meta) across platforms (30 min)
- [ ] Add comprehensive tests for:
  - Mouse tracking modes (30 min)
  - Bracketed paste (15 min)
  - Alternate screen interaction (15 min)
- [ ] **Defer IME support** to post-1.0 (add TODO + feature flag) (15 min)

---

## Phase 3: Testing & Quality (Week 2, Day 3)

**Goal:** Ensure reliability and conformance  
**Estimated Time:** 9.5-11.5 hours  
**Priority:** CRITICAL

### 3.1 Unit Tests (3.5-4.5 hours)

- [ ] Test `Grid` operations (1.5-2 hours)
  - Scrolling (up/down, regions)
  - Resizing (preserves content)
  - Selection (with scrollback)
  - Alternate screen (state preservation)
  - Semantic zones

- [ ] Test `AnsiParser` features (1 hour)
  - Mouse tracking modes
  - Character sets (DEC graphics, ISO-2022)
  - OSC sequences (52, 7, 8, 133)
  - Bracketed paste
  - Grapheme clusters
  - CJK wide characters

- [ ] Test renderer and input handlers (1-1.5 hours)
  - `DummyBackend` implementation
  - GTK4 backend (mocked GTK calls)
  - OpenGL backend (if implemented)

### 3.2 Integration Tests (3-4 hours)

#### Real-World Application Tests
```rust
// tests/integration/tmux_compat.rs
#[test]
fn test_tmux_split_panes() {
    let mut terminal = VteTerminalCore::new(default_config());
    // Send tmux split-pane commands
    terminal.feed_bytes(b"\x1bc"); // Reset
    terminal.feed_bytes(b"tmux split-window -h\n");
    // Verify alternate screen, mouse tracking
    assert!(terminal.grid.read().unwrap().using_alternate);
}

#[test]
fn test_vim_editing() {
    let mut terminal = VteTerminalCore::new(default_config());
    terminal.feed_bytes(b"vim test.txt\n");
    // Verify cursor movements, alternate screen
    // ...
}
```

#### Tasks
- [ ] Test user interactions (typing, selecting, scrolling) (1 hour)
- [ ] Test with shells (bash, zsh, fish) (30 min)
- [ ] Test with TUI apps (1 hour)
  - tmux (split-panes, navigation, clipboard)
  - zellij (panes, OSC 7, hyperlinks)
  - vim (alternate screen, cursor keys)
  - ratatui demos (mouse, colors, Unicode)
  - htop (colors, scrolling)

#### Golden File Tests
- [ ] Add snapshot testing for complex ANSI (30 min)
  ```rust
  #[test]
  fn test_tmux_status_line() {
      let output = include_bytes!("fixtures/tmux-status.ansi");
      let mut parser = AnsiParser::new();
      let mut grid = Grid::new(80, 24);
      parser.feed_bytes(output, &mut grid);
      
      insta::assert_snapshot!(grid.to_string());
  }
  ```

#### Unicode Edge Cases
- [ ] Test emoji (including ZWJ sequences) (15 min)
- [ ] Test RTL scripts (Hebrew, Arabic) (15 min)
- [ ] Test combining characters (15 min)
- [ ] Test CJK wide characters (15 min)

### 3.3 Conformance Testing (NEW - 3 hours)

#### vttest Suite
- [ ] Run vttest battery (1 hour)
  ```bash
  # Clone and build vttest
  git clone https://github.com/ThomasDickey/vttest
  cd vttest && ./configure && make
  
  # Run tests
  ./vttest
  ```
- [ ] Document passing/failing tests (30 min)
- [ ] Fix critical failures (aim for 90%+) (30 min)

#### esctest Suite
- [ ] Clone and run esctest (1 hour)
  ```bash
  git clone https://gitlab.freedesktop.org/terminal-wg/esctest
  cd esctest && python3 run_tests.py --terminal=vte-rs
  ```
- [ ] Document compatibility matrix (30 min)

### 3.4 Platform Testing (NEW - 2 hours)

- [ ] Test on Linux (X11 + Wayland) (45 min)
- [ ] Test on macOS (45 min)
- [ ] Test on Windows (ConPTY) (30 min)
- [ ] Document platform-specific issues (15 min)

### 3.5 Fuzzing and Benchmarks (1 hour)

- [ ] Extend fuzz targets for new features (30 min)
  ```rust
  // fuzz/fuzz_alternate_screen.rs
  #[fuzz_target]
  fn fuzz_alternate_screen(data: &[u8]) {
      let mut grid = Grid::new(80, 24);
      // Randomly switch screens, ensure no crashes
  }
  ```

- [ ] Add criterion benchmarks (30 min)
  ```rust
  // benches/rendering.rs
  fn bench_redraw(c: &mut Criterion) {
      c.bench_function("redraw 80x24", |b| {
          let terminal = VteTerminalCore::new(default_config());
          b.iter(|| terminal.render(&mut DummyRenderer::new()));
      });
  }
  
  // Performance targets:
  // - <2ms redraw for 80x24 screen
  // - <50MB RAM with tmux+vim
  // - >10MB/s PTY throughput
  ```

---

## Phase 4: Extended Features (Week 2, Day 4)

**Goal:** Polish and advanced features  
**Estimated Time:** 4-5 hours  
**Priority:** MEDIUM

### 4.1 Core VTE Features (3-4 hours)

- [ ] Smooth scrolling for mouse wheel/touchpad (1 hour)
  ```rust
  impl InputHandler {
      fn handle_scroll_smooth(&mut self, delta_y: f64) {
          // Accumulate fractional scrolling
          self.scroll_accumulator += delta_y;
          
          if self.scroll_accumulator.abs() >= 1.0 {
              let lines = self.scroll_accumulator.trunc() as isize;
              self.scroll_by_lines(lines);
              self.scroll_accumulator = self.scroll_accumulator.fract();
          }
      }
  }
  ```

- [ ] Double/triple-click selection (1 hour)
  ```rust
  impl Grid {
      pub fn select_word(&mut self, row: usize, col: usize) {
          // Find word boundaries
          let (start, end) = self.find_word_boundaries(row, col);
          self.start_selection(row, start);
          self.complete_selection(row, end);
      }
      
      pub fn select_line(&mut self, row: usize) {
          self.start_selection(row, 0);
          self.complete_selection(row, self.cols - 1);
      }
  }
  ```

- [ ] Configurable options (1 hour)
  ```rust
  pub struct TerminalConfig {
      pub scrollback_size: usize,
      pub cursor_shape: CursorShape,
      pub cursor_blink: bool,
      pub bell_style: BellStyle,
      pub bracketed_paste: bool,
      pub mouse_reporting: bool,
      pub font_family: String,
      pub font_size: f64,
      pub color_scheme: ColorScheme,
  }
  
  #[derive(Debug, Clone, Copy)]
  pub enum CursorShape {
      Block,
      Underline,
      Bar,
  }
  
  #[derive(Debug, Clone, Copy)]
  pub enum BellStyle {
      None,
      Visual,
      Audible,
      Both,
  }
  ```

- [ ] Add tests for new features (1 hour)

### 4.2 Advanced Features - Deferred (Post-1.0)

**Mark as feature flags and TODO:**
- [ ] OpenGL/wgpu backend (feature = "opengl", post-1.0)
- [ ] Kitty keyboard protocol (feature = "kitty", post-1.0)
- [ ] Sixel graphics (feature = "sixel", post-1.0)
- [ ] IME support (feature = "ime", post-1.0)

### 4.3 Accessibility (1 hour)

- [ ] Document keyboard navigation (15 min)
- [ ] Add high-contrast mode support (30 min)
- [ ] Create `ACCESSIBILITY.md` (15 min)
  ```markdown
  # Accessibility Features
  
  ## Keyboard Navigation
  - Tab: Move focus
  - Ctrl+Shift+C: Copy selection
  - Ctrl+Shift+V: Paste
  
  ## Screen Reader Support
  - Grid content exposed via accessibility API (GTK: AT-SPI)
  - Semantic zones help identify prompts vs output
  
  ## Visual Accessibility
  - High contrast mode
  - Customizable font sizes
  - Cursor shape options
  ```

---

## Phase 5: Documentation (Week 2, Day 4-5)

**Goal:** Comprehensive documentation for crates.io  
**Estimated Time:** 7.25-8.25 hours  
**Priority:** HIGH

### 5.1 API Documentation (3-4 hours)

- [ ] Add module-level documentation (1 hour)
  ```rust
  //! # vte-core
  //!
  //! A compositor-agnostic VTE (Virtual Terminal Emulator) library.
  //!
  //! ## Features
  //! - Full ANSI/VT escape sequence support
  //! - Alternate screen buffer
  //! - Mouse reporting (X10, Button, UTF-8, SGR modes)
  //! - Hyperlinks (OSC 8)
  //! - Clipboard integration (OSC 52)
  //! - Shell integration (OSC 7, OSC 133)
  //! - Unicode support (grapheme clusters, CJK, RTL)
  //! - Bracketed paste mode
  //!
  //! ## Architecture
  //! The library is split into traits that backends implement:
  //! - [`Renderer`] - Rendering abstraction
  //! - [`InputHandler`] - Input event handling
  //! - [`EventLoop`] - Async event scheduling
  //!
  //! ## Quick Start
  //! ```rust,no_run
  //! use vte_core::{VteTerminalCore, TerminalConfig};
  //! use vte_gtk4::Gtk4Backend;
  //!
  //! let config = TerminalConfig::default();
  //! let mut terminal = VteTerminalCore::new(config);
  //! let mut backend = Gtk4Backend::new();
  //!
  //! // Feed PTY output
  //! terminal.feed_bytes(b"\x1b[31mHello, World!\x1b[0m");
  //!
  //! // Render
  //! terminal.render(&mut backend);
  //! ```
  ```

- [ ] Document all public types and traits with examples (2-3 hours)
  - `VteTerminalCore`
  - `Grid`
  - `AnsiParser`
  - All trait interfaces
  - `TerminalConfig`
  - `TerminalError`

- [ ] Document supported sequences (30 min)
  ```rust
  //! ## Supported ANSI Sequences
  //!
  //! ### Cursor Movement
  //! - CSI A/B/C/D - Cursor up/down/right/left
  //! - CSI H - Cursor position
  //! - CSI s/u - Save/restore cursor
  //!
  //! ### Screen Manipulation
  //! - CSI J - Clear screen
  //! - CSI K - Clear line
  //! - CSI r - Set scrolling region (DECSTBM)
  //!
  //! ### Text Attributes (SGR)
  //! - CSI 0-9 m - Bold, italic, underline, etc.
  //! - CSI 30-37 m - Standard foreground colors
  //! - CSI 38;5;N m - 256-color foreground
  //! - CSI 38;2;R;G;B m - RGB foreground
  //!
  //! ### Mouse Reporting
  //! - CSI ?1000h/l - X10 mouse tracking
  //! - CSI ?1006h/l - SGR mouse mode
  //!
  //! ### OSC Sequences
  //! - OSC 0;title ST - Set window title
  //! - OSC 7;file://path ST - Current directory
  //! - OSC 8;params;uri ST - Hyperlink
  //! - OSC 52;c;data ST - Clipboard operations
  //! - OSC 133;A/B/C/D ST - Semantic prompts
  ```

- [ ] Document VTE compatibility matrix (30 min)
  ```markdown
  ## Compatibility Matrix
  
  | Feature | Support | Notes |
  |---------|---------|-------|
  | ANSI Colors | ✅ Full | 16 colors + 256 + RGB |
  | Alternate Screen | ✅ Full | CSI ?47h, ?1049h |
  | Mouse Tracking | ✅ Full | All modes (X10, Button, UTF-8, SGR) |
  | Hyperlinks | ✅ Full | OSC 8 |
  | Sixel Graphics | ⚠️ Partial | Feature flag, post-1.0 |
  | IME | ❌ Planned | Feature flag, post-1.0 |
  | RTL Text | ⚠️ Basic | Simple BiDi support |
  ```

- [ ] Create SECURITY.md (already done in Phase 0)

- [ ] Run `cargo doc` and fix warnings (30 min)

### 5.2 User Documentation (2 hours)

#### README.md
- [ ] Write comprehensive README (1 hour)
  ```markdown
  # VTE Terminal Emulator
  
  [![CI](https://github.com/user/vte-rs/workflows/CI/badge.svg)](https://github.com/user/vte-rs/actions)
  [![Crates.io](https://img.shields.io/crates/v/vte-core.svg)](https://crates.io/crates/vte-core)
  [![Documentation](https://docs.rs/vte-core/badge.svg)](https://docs.rs/vte-core)
  
  A production-ready, compositor-agnostic virtual terminal emulator library.
  
  ## Features
  
  - 🚀 **Performance** - <2ms redraws, >10MB/s throughput
  - 🎨 **Full VTE Support** - 90%+ vttest compliance
  - 🖱️ **Modern Features** - Hyperlinks, mouse tracking, shell integration
  - 🔌 **Backend Agnostic** - GTK4, custom backends via traits
  - 🌍 **Unicode Ready** - Emoji, CJK, RTL text support
  - 🔒 **Secure** - Bracketed paste, input sanitization
  
  ## Quick Start
  
  ### GTK4 Backend
  ```rust
  use vte_gtk4::VteTerminalWidget;
  use gtk4::prelude::*;
  
  fn main() {
      let app = gtk4::Application::new(None, Default::default());
      app.connect_activate(|app| {
          let window = gtk4::ApplicationWindow::new(app);
          let terminal = VteTerminalWidget::new();
          window.set_child(Some(&terminal));
          window.present();
      });
      app.run();
  }
  ```
  
  ### Headless Usage
  ```rust
  use vte_core::{VteTerminalCore, TerminalConfig};
  
  let mut terminal = VteTerminalCore::new(TerminalConfig::default());
  terminal.feed_bytes(b"echo 'Hello, World!'\n");
  
  // Access grid for testing/headless rendering
  let grid = terminal.grid();
  ```
  
  ## Compatibility
  
  Works with:
  - ✅ tmux (split-panes, clipboard, directory tracking)
  - ✅ zellij (panes, hyperlinks, OSC integration)
  - ✅ vim/neovim (alternate screen, cursor keys)
  - ✅ ratatui (mouse, colors, Unicode)
  - ✅ htop (colors, scrolling)
  
  ## Known Limitations (0.1.0-beta.1)
  
  - IME support deferred to 0.2.0 (feature flag available)
  - Sixel graphics experimental (feature flag)
  - RTL text is basic (full BiDi in 0.2.0)
  
  ## Documentation
  
  - [API Documentation](https://docs.rs/vte-core)
  - [Migration Guide](docs/MIGRATION.md)
  - [Architecture](docs/ARCHITECTURE.md)
  - [Security Policy](SECURITY.md)
  
  ## License
  
  Dual-licensed under MIT OR Apache-2.0
  ```

#### Migration Guide
- [ ] Create docs/MIGRATION.md (30 min)
  ```markdown
  # Migration Guide
  
  ## From GTK-coupled version
  
  ### Before (GTK-coupled)
  ```rust
  use vte_terminal::VteTerminalCore;
  
  let terminal = VteTerminalCore::new();
  let widget = terminal.widget();
  ```
  
  ### After (GTK-agnostic)
  ```rust
  use vte_core::VteTerminalCore;
  use vte_gtk4::VteTerminalWidget;
  
  let terminal = VteTerminalWidget::new();
  ```
  
  ### Key Changes
  - Core logic moved to `vte-core` (no GUI deps)
  - GTK4 backend in separate `vte-gtk4` crate
  - Traits enable custom backends
  
  ## From alacritty_terminal
  
  [Comparison table and examples]
  
  ## From wezterm
  
  [Comparison table and examples]
  ```

#### Architecture Documentation
- [ ] Create docs/ARCHITECTURE.md (30 min)
  ```markdown
  # Architecture
  
  ## Component Diagram
  ```
  ┌─────────────────┐
  │   Application   │
  └────────┬────────┘
           │
  ┌────────▼────────┐
  │   vte-gtk4      │ (Backend Implementation)
  │  - Gtk4Backend  │
  │  - Renderer     │
  │  - InputHandler │
  └────────┬────────┘
           │
  ┌────────▼────────┐
  │   vte-core      │ (Core Logic)
  │  - Grid         │
  │  - AnsiParser   │
  │  - PTY          │
  └────────┬────────┘
           │
  ┌────────▼────────┐
  │  portable-pty   │
  └─────────────────┘
  ```
  
  ## Data Flow
  1. PTY emits bytes → AnsiParser
  2. Parser updates Grid state
  3. Grid notifies backend via callback
  4. Backend calls Renderer to draw
  
  ## Thread Model
  - Main thread: GTK event loop, rendering
  - PTY reader thread: Reads from PTY, feeds parser
  - Cursor blink thread: Timer-based cursor toggle
  ```

### 5.3 Examples (2-2.25 hours)

- [ ] Create examples directory with multiple demos (2 hours)
  ```rust
  // examples/simple_gtk.rs
  //! Minimal GTK4 terminal window
  
  use vte_gtk4::VteTerminalWidget;
  use gtk4::prelude::*;
  
  fn main() {
      let app = gtk4::Application::new(
          Some("com.example.vte-simple"),
          Default::default()
      );
      
      app.connect_activate(|app| {
          let window = gtk4::ApplicationWindow::new(app);
          window.set_title(Some("VTE Terminal"));
          window.set_default_size(800, 600);
          
          let terminal = VteTerminalWidget::new();
          window.set_child(Some(&terminal));
          
          window.present();
      });
      
      app.run();
  }
  
  // examples/headless_parser.rs
  //! Parse ANSI sequences without GUI
  
  use vte_core::{VteTerminalCore, TerminalConfig};
  
  fn main() {
      let mut terminal = VteTerminalCore::new(TerminalConfig::default());
      
      // Feed some ANSI sequences
      terminal.feed_bytes(b"\x1b[31mRed text\x1b[0m\n");
      terminal.feed_bytes(b"\x1b[1;32mBold green\x1b[0m\n");
      
      // Access grid
      let grid = terminal.grid();
      println!("Grid size: {}x{}", grid.cols, grid.rows);
      println!("Cell at (0,0): {:?}", grid.get_cell(0, 0));
  }
  
  // examples/custom_backend.rs
  //! Implement a custom backend
  
  use vte_core::traits::*;
  
  struct MyBackend;
  
  impl Renderer for MyBackend {
      fn text_renderer(&mut self) -> &mut dyn TextRenderer {
          self
      }
      fn graphics_renderer(&mut self) -> &mut dyn GraphicsRenderer {
          self
      }
      fn ui_renderer(&mut self) -> &mut dyn UIRenderer {
          self
      }
  }
  
  impl TextRenderer for MyBackend {
      fn draw_cell(&mut self, row: usize, col: usize, cell: &Cell) {
          print!("{}[{},{}]: {}", 
              if cell.bold { "\x1b[1m" } else { "" },
              row, col, cell.ch);
      }
      // ... implement other methods
  }
  
  // examples/ratatui_demo.rs
  //! Show compatibility with ratatui
  
  // examples/tmux_test.rs
  //! Test tmux compatibility features
  ```

- [ ] Test all examples work (15 min)

---

## Phase 6: Package Preparation (Week 2, Day 5)

**Goal:** Prepare for crates.io publication  
**Estimated Time:** 4.25-5.25 hours  
**Priority:** CRITICAL

### 6.1 Cargo.toml Metadata (1.5 hours)

#### vte-core/Cargo.toml
- [ ] Create comprehensive metadata (45 min)
  ```toml
  [package]
  name = "vte-core"
  version = "0.1.0-beta.1"
  edition = "2021"
  rust-version = "1.70"
  authors = ["Your Name <email@example.com>"]
  license = "MIT OR Apache-2.0"
  description = "Compositor-agnostic virtual terminal emulator with full VTE compliance"
  readme = "README.md"
  homepage = "https://github.com/yourusername/vte-rs"
  repository = "https://github.com/yourusername/vte-rs"
  documentation = "https://docs.rs/vte-core"
  keywords = ["terminal", "vte", "ansi", "emulator", "tui"]
  categories = ["command-line-interface", "emulators", "parsing"]
  
  [dependencies]
  portable-pty = "~0.10"
  memchr = "~2.7"
  thiserror = "~1.0"
  unicode-segmentation = "~1.10"
  unicode-width = "~0.1"
  unicode-bidi = "~0.3"
  base64 = "~0.21"
  
  [dev-dependencies]
  criterion = "~0.5"
  insta = "~1.34"
  
  [features]
  default = ["mouse", "selection", "alternate_screen", "bracketed_paste"]
  mouse = []
  selection = []
  cursor_blink = []
  alternate_screen = []
  bracketed_paste = []
  # Post-1.0 features
  ime = []
  opengl = []
  kitty = []
  sixel = []
  
  [[bench]]
  name = "parser_throughput"
  harness = false
  
  [[bench]]
  name = "rendering"
  harness = false
  ```

#### vte-gtk4/Cargo.toml
- [ ] Create GTK4 backend metadata (45 min)
  ```toml
  [package]
  name = "vte-gtk4"
  version = "0.1.0-beta.1"
  edition = "2021"
  rust-version = "1.70"
  authors = ["Your Name <email@example.com>"]
  license = "MIT OR Apache-2.0"
  description = "GTK4 backend for vte-core terminal emulator"
  readme = "README.md"
  homepage = "https://github.com/yourusername/vte-rs"
  repository = "https://github.com/yourusername/vte-rs"
  documentation = "https://docs.rs/vte-gtk4"
  keywords = ["terminal", "gtk4", "vte", "emulator"]
  categories = ["command-line-interface", "gui"]
  
  [dependencies]
  vte-core = { version = "0.1.0-beta.1", path = "../vte-core" }
  gtk4 = "~0.7"
  cairo-rs = "~0.18"
  gdk4 = "~0.7"
  glib = "~0.18"
  
  [features]
  default = []
  wayland = ["gtk4/wayland"]
  x11 = ["gtk4/x11"]
  ```

### 6.2 License and CI/CD (2.75-3.75 hours)

#### License Files
- [ ] Add LICENSE-MIT (5 min)
- [ ] Add LICENSE-APACHE (5 min)

#### GitHub Actions CI
- [ ] Create .github/workflows/ci.yml (1.5-2 hours)
  ```yaml
  name: CI
  
  on:
    push:
      branches: [main]
    pull_request:
      branches: [main]
  
  env:
    CARGO_TERM_COLOR: always
  
  jobs:
    test:
      name: Test
      runs-on: ${{ matrix.os }}
      strategy:
        matrix:
          os: [ubuntu-latest, macos-latest, windows-latest]
          rust: [stable, beta, nightly]
      steps:
        - uses: actions/checkout@v3
        
        - name: Install Rust
          uses: dtolnay/rust-toolchain@master
          with:
            toolchain: ${{ matrix.rust }}
        
        - name: Install GTK4 (Ubuntu)
          if: matrix.os == 'ubuntu-latest'
          run: |
            sudo apt-get update
            sudo apt-get install -y libgtk-4-dev
        
        - name: Install GTK4 (macOS)
          if: matrix.os == 'macos-latest'
          run: brew install gtk4
        
        - name: Cache cargo registry
          uses: actions/cache@v3
          with:
            path: ~/.cargo/registry
            key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}
        
        - name: Run tests
          run: cargo test --all-features --verbose
        
        - name: Run tests (no default features)
          run: cargo test --no-default-features --verbose
    
    clippy:
      name: Clippy
      runs-on: ubuntu-latest
      steps:
        - uses: actions/checkout@v3
        - uses: dtolnay/rust-toolchain@stable
          with:
            components: clippy
        - run: |
            sudo apt-get update
            sudo apt-get install -y libgtk-4-dev
        - run: cargo clippy --all-targets --all-features -- -D warnings
    
    fmt:
      name: Format
      runs-on: ubuntu-latest
      steps:
        - uses: actions/checkout@v3
        - uses: dtolnay/rust-toolchain@stable
          with:
            components: rustfmt
        - run: cargo fmt --all -- --check
    
    coverage:
      name: Coverage
      runs-on: ubuntu-latest
      steps:
        - uses: actions/checkout@v3
        - uses: dtolnay/rust-toolchain@stable
        - run: |
            sudo apt-get update
            sudo apt-get install -y libgtk-4-dev
        - run: cargo install cargo-tarpaulin
        - run: cargo tarpaulin --all-features --workspace --timeout 300 --out Xml
        - uses: codecov/codecov-action@v3
          with:
            files: ./cobertura.xml
    
    benchmark:
      name: Benchmark
      runs-on: ubuntu-latest
      steps:
        - uses: actions/checkout@v3
        - uses: dtolnay/rust-toolchain@stable
        - run: |
            sudo apt-get update
            sudo apt-get install -y libgtk-4-dev
        - run: cargo bench --no-fail-fast -- --save-baseline main
        - uses: actions/upload-artifact@v3
          with:
            name: benchmark-results
            path: target/criterion/
    
    audit:
      name: Security Audit
      runs-on: ubuntu-latest
      steps:
        - uses: actions/checkout@v3
        - uses: dtolnay/rust-toolchain@stable
        - run: cargo install cargo-audit
        - run: cargo audit
  ```

- [ ] Add dependabot config (15 min)
  ```yaml
  # .github/dependabot.yml
  version: 2
  updates:
    - package-ecosystem: "cargo"
      directory: "/"
      schedule:
        interval: "monthly"
      open-pull-requests-limit: 10
  ```

---

## Phase 7: Polish & Release (Week 2-3)

**Goal:** Ship beta and gather feedback  
**Estimated Time:** 5.25-6.25 hours  
**Priority:** CRITICAL

### 7.1 Code Quality (2 hours)

- [ ] Run `cargo clippy -- -D warnings` and fix (1 hour)
- [ ] Run `cargo fmt` (5 min)
- [ ] Review TODOs and FIXMEs (30 min)
- [ ] Remove remaining unwrap()/expect() (15 min)
- [ ] Run `cargo audit` (10 min)

### 7.2 Pre-Release Testing (1.5 hours)

- [ ] Set version to 0.1.0-beta.1 (5 min)
- [ ] Run `cargo test --all-features` (10 min)
- [ ] Run smoke test with tmux (30 min)
  ```bash
  # Test checklist:
  # - Start tmux session
  # - Split panes (Ctrl-b %)
  # - Navigate between panes (Ctrl-b arrow)
  # - Copy text (Ctrl-b [, space, enter)
  # - Paste text (Ctrl-b ])
  # - Directory tracking works (OSC 7)
  # - Colors render correctly
  # - Mouse works (if enabled)
  ```
- [ ] Build documentation: `cargo doc --no-deps` (10 min)
- [ ] Verify README examples work (15 min)
- [ ] Update CHANGELOG.md (20 min)
  ```markdown
  # Changelog
  
  ## [0.1.0-beta.1] - 2025-XX-XX
  
  ### Added
  - Compositor-agnostic architecture with trait-based backends
  - GTK4 backend implementation
  - Full ANSI/VT escape sequence support
  - Alternate screen buffer
  - Mouse reporting (X10, Button, UTF-8, SGR modes)
  - Hyperlinks (OSC 8)
  - Clipboard integration (OSC 52)
  - Shell integration (OSC 7, OSC 133)
  - Bracketed paste mode
  - Unicode support (grapheme clusters, CJK, basic RTL)
  - Error handling and recovery
  - Comprehensive test suite
  
  ### Known Limitations
  - IME support deferred to 0.2.0
  - Sixel graphics experimental
  - RTL text is basic (full BiDi in 0.2.0)
  - vttest score: ~90% (see COMPATIBILITY.md)
  
  ### Performance
  - <2ms redraw for 80x24 screen
  - <50MB RAM with tmux+vim
  - >10MB/s PTY throughput
  ```

### 7.3 Beta Release (45 min)

- [ ] Run `cargo publish --dry-run` for vte-core (10 min)
- [ ] Run `cargo publish --dry-run` for vte-gtk4 (10 min)
- [ ] Publish beta: `cargo publish` vte-core (10 min)
- [ ] Publish beta: `cargo publish` vte-gtk4 (10 min)
- [ ] Create git tag: `git tag v0.1.0-beta.1` (5 min)

### 7.4 Release Announcement (1 hour)

- [ ] Create GitHub release with notes (30 min)
  ```markdown
  # VTE Terminal 0.1.0-beta.1
  
  First beta release of the compositor-agnostic VTE terminal emulator!
  
  ## Highlights
  - 🚀 GTK4 backend with full VTE support
  - 🎨 90%+ vttest compliance
  - 🖱️ Mouse tracking, hyperlinks, shell integration
  - 🌍 Unicode support (emoji, CJK, RTL)
  - 🔒 Secure by default (bracketed paste)
  
  ## Try It
  ```toml
  [dependencies]
  vte-gtk4 = "0.1.0-beta.1"
  ```
  
  ## Feedback Wanted
  We need your help testing! Please report:
  - Bugs (crashes, rendering issues)
  - Feature requests
  - Documentation gaps
  - Performance issues
  
  Beta period: 1 week (until 2025-XX-XX)
  
  ## Known Limitations
  - IME support coming in 0.2.0
  - Sixel graphics experimental
  - Basic RTL support
  
  See [CHANGELOG.md](CHANGELOG.md) for full details.
  ```

- [ ] Post to Reddit r/rust (15 min)
- [ ] Post to Hacker News (15 min)

### 7.5 Beta Feedback Process (15 min)

- [ ] Set up issue templates (15 min)
  ```markdown
  ---
  name: Bug report (Beta)
  about: Report a bug in 0.1.0-beta.1
  ---
  
  **Environment**
  - OS: [e.g. Ubuntu 22.04]
  - Rust version: [e.g. 1.75]
  - Terminal: [e.g. GNOME Terminal, tmux]
  
  **Description**
  [Clear description of the bug]
  
  **Steps to Reproduce**
  1. ...
  2. ...
  
  **Expected Behavior**
  [What should happen]
  
  **Actual Behavior**
  [What actually happens]
  
  **Logs/Screenshots**
  [If applicable]
  ```

### 7.6 Post-Beta: Final Release (1 hour)

**After 1-week beta period:**

- [ ] Triage feedback (categorize as bugs, features, docs)
- [ ] Fix critical bugs
- [ ] Update documentation based on feedback
- [ ] Update version to 0.1.0
- [ ] Update CHANGELOG.md with beta feedback
- [ ] Publish final: `cargo publish` for both crates
- [ ] Create release: `git tag v0.1.0`
- [ ] Announce 1.0 release

---

## Priority Ordering Summary

### Week 1: Core Foundation (18-21 hours)
**Days 1-3: Build the foundation**

| Day | Phase | Hours | Priority | Deliverables |
|-----|-------|-------|----------|--------------|
| 1 | Phase 0 + 1.1-1.2 | 8-10 | 🔴 CRITICAL | Core traits, security baseline, font rendering |
| 2 | Phase 1.3-1.4 | 5.5-6.5 | 🔴 CRITICAL | GTK4 backend, alternate screen |
| 3 | Phase 1.5-1.6 | 6-7 | 🔴 CRITICAL | Error handling, resource management |

### Week 2: Features & Polish (24-29 hours)
**Days 1-5: Complete features and ship**

| Day | Phase | Hours | Priority | Deliverables |
|-----|-------|-------|----------|--------------|
| 1-2 | Phase 2 | 11-13 | 🟠 HIGH | ANSI/VTE features, Unicode, shell integration |
| 3 | Phase 3 | 9.5-11.5 | 🔴 CRITICAL | Testing, conformance, fuzzing |
| 4 | Phase 4-5 | 11-13 | 🟡 MEDIUM | Extended features, documentation |
| 5 | Phase 6-7 | 9.5-11.5 | 🔴 CRITICAL | Package prep, polish, beta release |

### Week 3: Feedback & 1.0 (1 hour)
**Post-beta week**

- Monitor issues
- Fix critical bugs
- Ship 1.0

---

## Success Criteria Checklist

Before publishing to crates.io:

### Functionality
- [ ] ✅ Alternate screen works with tmux, zellij, vim
- [ ] ✅ Mouse reporting enables tmux/zellij navigation
- [ ] ✅ OSC 52 clipboard works over SSH
- [ ] ✅ OSC 8 hyperlinks work in zellij/ratatui
- [ ] ✅ Bracketed paste prevents exploits
- [ ] ✅ OSC 7/133 directory tracking works
- [ ] ✅ Grapheme clusters render (emoji with modifiers)
- [ ] ✅ CJK wide characters render correctly
- [ ] ✅ Basic RTL text support

### Quality
- [ ] ✅ No panics on any input (24hr fuzz test)
- [ ] ✅ Test coverage > 85% for vte-core
- [ ] ✅ vttest score: 90%+
- [ ] ✅ esctest compatibility documented
- [ ] ✅ Smoke test passes (tmux session)

### Performance
- [ ] ✅ <2ms redraw for 80x24 screen
- [ ] ✅ <50MB RAM with tmux+vim
- [ ] ✅ >10MB/s PTY throughput

### Documentation
- [ ] ✅ All public APIs documented
- [ ] ✅ README with quick-start
- [ ] ✅ Migration guide from GTK-coupled version
- [ ] ✅ Examples work (simple, headless, custom backend)
- [ ] ✅ Known limitations documented

### Code Quality
- [ ] ✅ Zero clippy warnings
- [ ] ✅ cargo fmt passes
- [ ] ✅ cargo audit clean
- [ ] ✅ CI passing on Linux/macOS/Windows
- [ ] ✅ Follows Rust API guidelines

### Release
- [ ] ✅ Version 0.1.0-beta.1 published
- [ ] ✅ Beta feedback period (1 week)
- [ ] ✅ Critical bugs fixed
- [ ] ✅ Version 0.1.0 published

---

## Risk Assessment

### Low Risk ✅
- ANSI parser robustness (already solid)
- Grid implementation (proven)
- GTK4 backend (prior experience)
- Basic error handling

### Medium Risk ⚠️
- Alternate screen state management (complex transitions)
- Mouse reporting edge cases (multiple modes)
- Cross-platform PTY (Windows ConPTY differences)
- Unicode edge cases (RTL, grapheme clusters)
- Performance targets (may need iteration)

### High Risk 🔴
- **IME integration** → Deferred to 0.2.0, feature flag
- **OpenGL backend** → Deferred to 0.2.0, feature flag
- **Sixel graphics** → Experimental, feature flag
- **Full RTL/BiDi** → Basic support in 0.1.0, full in 0.2.0

**Mitigation:** Use feature flags for high-risk items, document limitations clearly

---

## Post-1.0 Roadmap

### Version 0.2.0 (2-3 months)
- [ ] IME support (full implementation)
- [ ] Full RTL/BiDi support
- [ ] Sixel graphics (stable)
- [ ] Additional backends (winit, egui)
- [ ] Kitty keyboard protocol
- [ ] Performance optimizations (glyph atlas, dirty rectangles)
- [ ] vttest score: 95%+

### Version 0.3.0 (4-6 months)
- [ ] wgpu/GPU-accelerated backend
- [ ] Ligature support
- [ ] Advanced accessibility (screen readers)
- [ ] Font fallback chains
- [ ] Color emoji font support

### Version 1.0.0 (8-12 months)
- [ ] API stability guarantee
- [ ] Comprehensive platform testing
- [ ] Performance on par with alacritty
- [ ] vttest score: 98%+
- [ ] Production use in major projects

---

## Maintenance Plan

### Ongoing (Post-Release)

#### Issue Management
- [ ] Monitor GitHub issues daily
- [ ] Respond to bug reports within 48 hours
- [ ] Triage new issues (bug, feature, question, docs)
- [ ] Close stale issues after 30 days of inactivity

#### Release Cadence
- **Patch releases (0.1.x)** - Bug fixes, ASAP
  - Security issues: Same day
  - Critical bugs: Within 1 week
  - Minor bugs: Monthly
- **Minor releases (0.x.0)** - New features, every 2-3 months
- **Major releases (x.0.0)** - Breaking changes, yearly

#### Dependency Management
- [ ] Review dependencies quarterly
- [ ] Update to latest compatible versions
- [ ] Follow pessimistic versioning (e.g., `~0.10`)
- [ ] Test with `cargo update` before releases
- [ ] Document any MSRV changes

#### Community
- [ ] Review PRs within 1 week
- [ ] Provide constructive feedback
- [ ] Merge quality contributions
- [ ] Update CONTRIBUTORS.md
- [ ] Thank contributors publicly

#### Documentation
- [ ] Keep docs in sync with code
- [ ] Update examples for new features
- [ ] Add FAQ entries for common questions
- [ ] Write blog posts for major releases

---

## Estimated Timeline Summary

### Total Time Investment

| Phase | Time Range | Priority |
|-------|------------|----------|
| Phase 0: Setup | 2-3 hours | 🔴 CRITICAL |
| Phase 1: Core | 18-21 hours | 🔴 CRITICAL |
| Phase 2: Features | 11-13 hours | 🟠 HIGH |
| Phase 3: Testing | 9.5-11.5 hours | 🔴 CRITICAL |
| Phase 4: Extended | 4-5 hours | 🟡 MEDIUM |
| Phase 5: Docs | 7.25-8.25 hours | 🟠 HIGH |
| Phase 6: Package | 4.25-5.25 hours | 🔴 CRITICAL |
| Phase 7: Release | 5.25-6.25 hours | 🔴 CRITICAL |
| **TOTAL** | **61.5-73.25 hours** | |

### Realistic Schedule

**Full-time (40 hrs/week):**
- Week 1: Phases 0-1 (Core foundation)
- Week 2: Phases 2-3 (Features + testing)
- Week 3: Phases 4-7 (Polish + release)
- Week 4: Beta feedback + 1.0

**Part-time (10 hrs/week):**
- Weeks 1-2: Phases 0-1
- Weeks 3-4: Phase 2
- Weeks 5-6: Phase 3
- Weeks 7-8: Phases 4-7
- Week 9: Beta + 1.0

**Aggressive (20 hrs/week):**
- Week 1-2: Phases 0-2
- Week 3: Phase 3
- Week 4: Phases 4-7 + Beta
- Week 5: Feedback + 1.0

---

## Testing Strategy

### Unit Tests (Target: 85%+ coverage)

```rust
// vte-core/src/grid.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_alternate_screen_preserves_state() {
        let mut grid = Grid::new(80, 24);
        grid.put('A');
        grid.advance();
        
        // Switch to alternate
        grid.use_alternate_screen(true);
        assert!(grid.using_alternate);
        
        // Verify main screen preserved
        grid.use_alternate_screen(false);
        assert_eq!(grid.get_cell(0, 0).ch, 'A');
    }
    
    #[test]
    fn test_grapheme_clusters() {
        let mut grid = Grid::new(80, 24);
        grid.put_grapheme("👨‍👩‍👧‍👦");
        
        // Should occupy 2 cells (wide emoji)
        assert!(grid.get_cell(0, 0).wide);
        assert!(grid.get_cell(0, 1).wide_continuation);
    }
    
    #[test]
    fn test_memory_limit() {
        let mut grid = Grid::new(80, 24);
        
        // Fill scrollback beyond limit
        for _ in 0..100000 {
            grid.newline();
        }
        
        grid.enforce_memory_limit();
        let bytes = grid.scrollback.len() * std::mem::size_of::<Cell>();
        assert!(bytes <= 50 * 1024 * 1024);
    }
}
```

### Integration Tests

```rust
// tests/integration/tmux_compat.rs
use vte_core::{VteTerminalCore, TerminalConfig};
use std::thread;
use std::time::Duration;

#[test]
fn test_tmux_split_panes() {
    let mut terminal = VteTerminalCore::new(TerminalConfig::default());
    
    // Simulate tmux split-pane command
    terminal.feed_bytes(b"\x1b[?1049h"); // Enter alternate screen
    terminal.feed_bytes(b"\x1b[?25l");   // Hide cursor
    
    let grid = terminal.grid();
    assert!(grid.using_alternate);
    assert!(!grid.is_cursor_visible());
}

#[test]
fn test_tmux_clipboard() {
    let mut terminal = VteTerminalCore::new(TerminalConfig::default());
    
    // OSC 52 clipboard set
    let text = "Hello, World!";
    let encoded = base64::encode(text);
    terminal.feed_bytes(format!("\x1b]52;c;{}\x07", encoded).as_bytes());
    
    // Verify clipboard was set
    // (requires backend support)
}

// tests/integration/vim_compat.rs
#[test]
fn test_vim_alternate_screen() {
    let mut terminal = VteTerminalCore::new(TerminalConfig::default());
    
    // Simulate vim startup
    terminal.feed_bytes(b"\x1b[?1049h\x1b[22;0;0t\x1b[1;24r");
    
    let grid = terminal.grid();
    assert!(grid.using_alternate);
}

// tests/integration/unicode.rs
#[test]
fn test_emoji_rendering() {
    let mut terminal = VteTerminalCore::new(TerminalConfig::default());
    
    terminal.feed_bytes("👨‍👩‍👧‍👦".as_bytes());
    terminal.feed_bytes("你好世界".as_bytes()); // CJK
    terminal.feed_bytes("مرحبا".as_bytes());   // RTL Arabic
    
    // Should not panic
}
```

### Conformance Tests

```bash
#!/bin/bash
# scripts/run_vttest.sh

echo "Running vttest conformance suite..."

# Start terminal
./target/debug/vte-terminal &
TERM_PID=$!

# Wait for startup
sleep 1

# Run vttest
cd vttest
./vttest

# Capture score
# (vttest outputs score at end)

kill $TERM_PID
```

### Performance Benchmarks

```rust
// benches/rendering.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use vte_core::{VteTerminalCore, TerminalConfig};

fn bench_redraw(c: &mut Criterion) {
    let mut terminal = VteTerminalCore::new(TerminalConfig::default());
    let mut renderer = DummyRenderer::new();
    
    // Fill screen with content
    for _ in 0..24 {
        terminal.feed_bytes(b"Lorem ipsum dolor sit amet consectetur adipiscing elit\n");
    }
    
    c.bench_function("redraw 80x24", |b| {
        b.iter(|| {
            terminal.render(black_box(&mut renderer));
        });
    });
}

fn bench_parser_throughput(c: &mut Criterion) {
    let mut terminal = VteTerminalCore::new(TerminalConfig::default());
    
    // 1MB of ANSI sequences
    let data = vec![b'\x1b'; 1024 * 1024];
    
    c.bench_function("parser 1MB", |b| {
        b.iter(|| {
            terminal.feed_bytes(black_box(&data));
        });
    });
}

criterion_group!(benches, bench_redraw, bench_parser_throughput);
criterion_main!(benches);
```

---

## Feature Flags Reference

### Default Features
```toml
[features]
default = ["mouse", "selection", "alternate_screen", "bracketed_paste"]
```

- **mouse** - Mouse tracking (X10, Button, UTF-8, SGR)
- **selection** - Text selection and clipboard
- **alternate_screen** - Alternate screen buffer
- **bracketed_paste** - Bracketed paste mode

### Optional Features (0.1.0)
```toml
cursor_blink = []
```

- **cursor_blink** - Blinking cursor support

### Post-1.0 Features
```toml
ime = ["ibus"]                    # IME support (Linux: ibus, Windows: TSF, macOS: NSTextInputClient)
opengl = ["wgpu", "raw-window-handle"]  # GPU-accelerated rendering
kitty = []                        # Kitty keyboard protocol
sixel = ["image"]                 # Sixel graphics support
```

### Backend-Specific Features
```toml
# vte-gtk4
wayland = ["gtk4/wayland"]
x11 = ["gtk4/x11"]
```

---

## Troubleshooting Guide

### Common Issues

#### Issue: Grid lines not rendering
```rust
// Check config is passed correctly
let mut config = TerminalConfig::default();
config.draw_grid_lines = true;
config.grid_line_alpha = 0.1;

// Verify in drawing function
eprintln!("DEBUG: draw_grid_lines = {}", config.draw_grid_lines);
```

#### Issue: Mouse events not working
```rust
// Enable mouse tracking
terminal.feed_bytes(b"\x1b[?1006h"); // SGR mouse mode
terminal.feed_bytes(b"\x1b[?1000h"); // Enable tracking

// Check InputHandler is connected
assert!(input_handler.mouse_enabled());
```

#### Issue: Alternate screen not switching
```rust
// Debug alternate screen state
let grid = terminal.grid();
eprintln!("Using alternate: {}", grid.using_alternate);
eprintln!("Saved cursor: {:?}", grid.saved_cursor);
```

#### Issue: PTY not spawning
```rust
// Check shell exists
let shell = std::env::var("SHELL").unwrap_or("/bin/bash".to_string());
assert!(std::path::Path::new(&shell).exists());

// Check PTY permissions (Unix)
// - User must have access to /dev/ptmx
```

#### Issue: Clipboard not working
```rust
// Verify OSC 52 is enabled
config.allow_osc_sequences = true;
config.allowed_osc_types.push(52);

// Check backend clipboard integration
// GTK: Uses gdk::Display::clipboard()
```

---

## API Stability Guarantees

### 0.1.x Releases
- Public API may change (semver-exempt beta)
- Breaking changes documented in CHANGELOG
- Migration guide provided

### 1.0.0 Release
- **Stable API** - Semantic versioning enforced
- Breaking changes only in major versions
- Deprecation warnings for 1 minor version before removal

### Internal APIs
- Prefixed with `_` or in `internal` module
- No stability guarantees
- May change at any time

---

## Contributing Guidelines

### Before Contributing

1. Read [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)
2. Check existing issues for duplicates
3. Discuss large changes in an issue first

### Development Setup

```bash
# Clone repository
git clone https://github.com/yourusername/vte-rs
cd vte-rs

# Install dependencies (Ubuntu)
sudo apt-get install libgtk-4-dev

# Install dependencies (macOS)
brew install gtk4

# Run tests
cargo test --all-features

# Run examples
cargo run --example simple_gtk
```

### Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Add tests for new functionality
5. Run `cargo test`, `cargo clippy`, `cargo fmt`
6. Commit with clear message
7. Push to your fork
8. Open a Pull Request

### Code Style

- Follow Rust naming conventions
- Use `cargo fmt` for formatting
- Fix all `cargo clippy` warnings
- Document public APIs with rustdoc
- Add examples to documentation
- Write tests for new features

---

## License

Dual-licensed under MIT OR Apache-2.0.

### Why Dual License?

- **MIT** - Simple, permissive
- **Apache-2.0** - Patent protection

Users can choose either license.

---

## Acknowledgments

### Prior Art
- **alacritty** - Performance inspiration
- **wezterm** - Feature completeness
- **vte** (GNOME) - VTE specification
- **xterm** - Terminal standards

### Dependencies
- `portable-pty` - Cross-platform PTY
- `gtk4` - GTK4 bindings
- `unicode-segmentation` - Grapheme clusters
- `unicode-width` - Character width detection

---

## Appendix: Complete File Structure

```
vte-rs/
├── Cargo.toml                 # Workspace manifest
├── README.md
├── LICENSE-MIT
├── LICENSE-APACHE
├── SECURITY.md
├── CHANGELOG.md
├── CODE_OF_CONDUCT.md
├── CONTRIBUTING.md
│
├── vte-core/                  # Core library (no GUI)
│   ├── Cargo.toml
│   ├── README.md
│   ├── src/
│   │   ├── lib.rs
│   │   ├── traits.rs          # Backend traits
│   │   ├── terminal.rs        # VteTerminalCore
│   │   ├── grid.rs            # Grid + alternate screen
│   │   ├── parser.rs          # AnsiParser (from ansi.rs)
│   │   ├── selection.rs       # Selection logic
│   │   ├── config.rs          # TerminalConfig
│   │   ├── error.rs           # TerminalError
│   │   ├── font.rs            # Font metrics (fontdue)
│   │   └── pty.rs             # PTY abstraction
│   ├── benches/
│   │   ├── parser.rs
│   │   └── rendering.rs
│   └── tests/
│       └── grid_tests.rs
│
├── vte-gtk4/                  # GTK4 backend
│   ├── Cargo.toml
│   ├── README.md
│   ├── src/
│   │   ├── lib.rs
│   │   ├── backend.rs         # Gtk4Backend
│   │   ├── renderer.rs        # Renderer impl (Cairo)
│   │   ├── input.rs           # InputHandler impl
│   │   ├── event_loop.rs      # EventLoop impl
│   │   └── widget.rs          # VteTerminalWidget
│   └── examples/
│       └── simple.rs
│
├── tests/                     # Integration tests
│   ├── integration/
│   │   ├── tmux_compat.rs
│   │   ├── vim_compat.rs
│   │   ├── zellij_compat.rs
│   │   └── unicode.rs
│   └── fixtures/
│       └── tmux-status.ansi
│
├── examples/                  # Workspace examples
│   ├── simple_gtk.rs
│   ├── headless_parser.rs
│   ├── custom_backend.rs
│   ├── ratatui_demo.rs
│   └── tmux_test.rs
│
├── docs/                      # Documentation
│   ├── ARCHITECTURE.md
│   ├── COMPATIBILITY.md
│   ├── ACCESSIBILITY.md
│   └── MIGRATION.md
│
├── scripts/                   # Utility scripts
│   ├── run_vttest.sh
│   └── benchmark.sh
│
└── .github/
    ├── workflows/
    │   └── ci.yml
    ├── dependabot.yml
    └── ISSUE_TEMPLATE/
        ├── bug_report.md
        └── feature_request.md
```

---

## Quick Reference Commands

### Development
```bash
# Run all tests
cargo test --all-features --workspace

# Run tests without default features
cargo test --no-default-features

# Run specific test
cargo test --test tmux_compat

# Run benchmarks
cargo bench

# Check code
cargo clippy --all-targets --all-features -- -D warnings

# Format code
cargo fmt --all

# Security audit
cargo audit

# Build documentation
cargo doc --no-deps --open

# Run example
cargo run --example simple_gtk
```

### Release
```bash
# Dry run
cargo publish --dry-run

# Publish
cargo publish

# Tag release
git tag v0.1.0-beta.1
git push origin v0.1.0-beta.1
```

---

## Final Checklist

Before marking this plan complete:

- [ ] All phases have clear tasks
- [ ] Time estimates are realistic
- [ ] Dependencies are identified
- [ ] Risks are assessed
- [ ] Success criteria defined
- [ ] Testing strategy complete
- [ ] Documentation plan solid
- [ ] Release process clear
- [ ] Post-release plan ready

---

**This plan provides a complete roadmap from current state to production-ready 1.0 release in 42-50 hours of focused development work, plus 1 week for beta feedback.**

**Next Steps:**
1. Start with Phase 0 (Security & Unicode baseline)
2. Follow the day-by-day breakdown
3. Track progress with this checklist
4. Ship beta in 2-3 weeks
5. Gather feedback for 1 week
6. Release 1.0

Good luck! 🚀
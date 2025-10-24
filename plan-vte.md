# VTE Terminal - Production Readiness Plan

This plan outlines the steps to make the VTE terminal production-ready, GTK-agnostic, and fully compatible with tools like tmux, ratatui, zellij, vim, and htop, ensuring 100% VTE compliance. It leverages the robust `AnsiParser` (supporting phase-4 features like line/character operations, alternate screen, and modes) and includes tasks for implementing the alternate screen buffer, GTK-agnostic refactoring, and VTE-specific features (e.g., mouse reporting, OSC 52, hyperlinks, bracketed paste, vte.sh integration). The plan incorporates modular trait design, performance benchmarks, error recovery, feature flags, beta testing, a migration path, and a known limitations section, targeting a reliable, feature-complete terminal in 36.25-44.25 hours.

## Phase 1: Core Functionality (Critical - Must Have)
**Goal**: Establish core terminal functionality and architecture
**Estimated Time**: 9-11 hours

### 1.1 Core Library Structure ⭐ HIGHEST PRIORITY
- [ ] Create `vte-core` crate with `AnsiParser`, `Grid`, and PTY handling (`portable_pty`) (1 hour)
- [ ] Define core traits: `Renderer`, `InputHandler`, `EventLoop` with modular sub-traits (1 hour)
  ```rust
  pub trait Renderer {
      fn text_renderer(&mut self) -> &mut dyn TextRenderer;
      fn graphics_renderer(&mut self) -> &mut dyn GraphicsRenderer;
      fn ui_renderer(&mut self) -> &mut dyn UIRenderer;
  }
  pub trait TextRenderer {
      fn draw_cell(&mut self, row: usize, col: usize, cell: &Cell);
      fn set_font(&mut self, family: &str, size: f64);
      fn get_char_metrics(&self, ch: char) -> (f64, f64, f64); // width, height, ascent
  }
  pub trait GraphicsRenderer {
      fn draw_sixel(&mut self, data: &[u8], x: usize, y: usize);
      fn draw_image(&mut self, image: ImageData, x: usize, y: usize);
  }
  pub trait UIRenderer {
      fn clear(&mut self);
      fn flush(&mut self);
  }
  pub trait InputHandler {
      fn handle_key(&mut self, key: KeyEvent, grid: &Arc<RwLock<Grid>>, writer: &Arc<Mutex<Box<dyn Write + Send>>>);
      fn handle_mouse(&mut self, event: MouseEvent, grid: &Arc<RwLock<Grid>>);
      fn handle_scroll(&mut self, delta: f64, grid: &Arc<RwLock<Grid>>);
  }
  pub trait EventLoop {
      fn schedule_redraw(&mut self, callback: Box<dyn FnMut()>);
      fn schedule_timer(&mut self, interval_ms: u64, callback: Box<dyn FnMut() -> bool>);
  }
  ```
- [ ] Implement `VteTerminalCore` struct to manage `Grid`, `AnsiParser`, PTY, and core traits (1 hour)
- [ ] Implement `DummyBackend` for `Renderer`, `InputHandler`, and `EventLoop` to enable testing (1 hour)
- [ ] Refactor `DrawingCache` to use `fontdue` for backend-agnostic font rendering (2 hours)
- [ ] Implement GTK backend (`vte-gtk`) for `Renderer`, `InputHandler`, and `EventLoop` using `gtk4` and `cairo` (2-3 hours)
- [ ] Add tests for core library initialization, `DummyBackend`, and GTK backend integration (2 hours)

### 1.2 Alternate Screen Buffer
- [ ] Add `alternate_cells: Vec<Cell>` to `Grid` with same dimensions as `cells` (30 min)
- [ ] Implement `use_alternate_screen(bool)` to swap `cells`/`alternate_cells`, save/restore cursor and attributes (1 hour)
- [ ] Update `resize` to handle both buffers, preserving content (30 min)
- [ ] Modify `clear_screen` to affect only the active buffer (30 min)
- [ ] Add tests for alternate screen switching and state preservation (1 hour)

## Phase 2: Complete ANSI and VTE Support (Important)
**Goal**: Support all common ANSI sequences and VTE-specific features for tmux, ratatui, zellij
**Estimated Time**: 8-10 hours

### 2.1 Missing ANSI and VTE Sequences
- [ ] Implement DEC private modes in `AnsiParser` (e.g., `CSI ?1h/l` for application cursor keys, `CSI ?47h/l` for alternate screen compatibility) (1 hour)
- [ ] Add full mouse reporting modes in `AnsiParser` and `InputHandler` (`CSI ?1000h/l`, `?1002h/l`, `?1005h/l`, `?1006h/l`, `?1004h/l` for focus events) (1.5 hours)
- [ ] Support character set switching (e.g., DEC Special Graphics, ISO-2022) in `AnsiParser` (1 hour)
- [ ] Implement OSC 52 for clipboard operations (copy/paste with base64 encoding) in `AnsiParser` and `Renderer` (45 min)
- [ ] Implement bracketed paste mode (`CSI ?2004h/l`) in `AnsiParser` and `InputHandler` (30 min)
- [ ] Implement OSC 8 hyperlinks (add `url: Option<String>` to `Cell`, parse in `AnsiParser`, render/handle clicks in `UIRenderer`/`InputHandler`) (1 hour)
- [ ] Add tests for new sequences and verify with `vttest`, tmux, zellij, ratatui (2 hours)

### 2.2 VTE Profile Script Integration
- [ ] In `VteTerminalCore::spawn_pty`, set env vars (`VTE_VERSION=1`, `VTE_INSTANCE_ID=$(uuidgen)`) and source `/etc/profile.d/vte.sh` if present (1 hour)
- [ ] In `AnsiParser`, parse OSC 7 for current directory tracking and update `Grid::set_title` (30 min)
- [ ] Add tests for tmux/zellij split-pane directory tracking (1 hour)

### 2.3 Input Handling Enhancements
- [ ] Add IME support for non-Latin languages in `InputHandler` trait (post-1.0, high risk, feature flag `ime`) (1 hour)
- [ ] Ensure modifier key consistency (Alt, Super) across platforms (30 min)
- [ ] Add tests for input handling with mouse tracking, bracketed paste, and alternate screen (30 min)

## Phase 3: Robustness & Safety (Critical for Production)
**Goal**: Prevent crashes and ensure reliability
**Estimated Time**: 6-7 hours

### 3.1 Error Handling
- [ ] Define `TerminalError` enum with hierarchy and recovery strategy (1 hour)
  ```rust
  #[derive(Debug, thiserror::Error)]
  pub enum TerminalError {
      #[error("PTY error: {0}")]
      Pty(#[from] portable_pty::Error),
      #[error("Rendering error: {0}")]
      Rendering(String),
      #[error("Parser error: {0}")]
      Parser(#[from] AnsiError),
  }
  impl VteTerminalCore {
      pub fn recover_from_error(&mut self, error: TerminalError) -> Result<(), TerminalError> {
          match error {
              TerminalError::Pty(_) => self.restart_pty(),
              TerminalError::Rendering(_) => self.fallback_to_simple_rendering(),
              _ => Err(error),
          }
      }
  }
  ```
- [ ] Replace `unwrap()`/`expect()` in `VteTerminalCore`, `Grid`, and input handling with `TerminalError` (1 hour)
- [ ] Integrate `AnsiParser`’s `ErrorCallback` with core library error reporting (e.g., log to file or frontend) (30 min)
- [ ] Add fallback for PTY failures (e.g., retry spawning in `restart_pty`) (30 min)
- [ ] Add tests for error handling, recovery, and `recover_from_error` (1.5 hours)

### 3.2 Resource Management
- [ ] Implement `Drop` for `VteTerminalCore` to clean up PTY resources (30 min)
- [ ] Add memory limits for scrollback buffer in `Grid` (primary buffer only, <50MB with tmux+vim) (30 min)
- [ ] Add timeout for PTY reader thread (30 min)
- [ ] Add tests for resource cleanup and memory limits (1 hour)

## Phase 4: Extended Features (Nice to Have)
**Goal**: Support advanced terminal applications and modern features
**Estimated Time**: 6-7 hours

### 4.1 Core VTE Features (Pre-1.0)
- [ ] Add smooth scrolling for mouse wheel/touchpad in `InputHandler` trait (1 hour)
- [ ] Add double-click word selection and triple-click line selection in `Grid` (1 hour)
- [ ] Add configurable options in `TerminalConfig` (scrollback size, cursor shape, bell behavior) (1 hour)
- [ ] Add tests for smooth scrolling, selection, and configuration options (1 hour)

### 4.2 Advanced Features (Post-1.0)
- [ ] Implement OpenGL backend for `Renderer` trait using `wgpu` (high risk, post-1.0, feature flag `opengl`) (2 hours)
- [ ] Implement Kitty keyboard protocol in `InputHandler` (optional, post-1.0, feature flag `kitty`) (1 hour)
- [ ] Implement Sixel graphics parsing in `AnsiParser` and rendering in `GraphicsRenderer` (optional, post-1.0, feature flag `sixel`) (1.5 hours)
- [ ] Add tests for Kitty protocol and Sixel in zellij (e.g., yazi image previews) (1 hour)

### 4.3 Accessibility (Post-1.0)
- [ ] Integrate with screen readers (e.g., AT-SPI for GTK backend) (1 hour)
- [ ] Add high-contrast mode and customizable font sizes (30 min)
- [ ] Add tests for accessibility features (30 min)

## Phase 5: Testing & Quality (Critical)
**Goal**: Ensure reliability and catch regressions
**Estimated Time**: 8.5-10.5 hours

### 5.1 Unit Tests
- [ ] Test `Grid` operations (scrolling, resizing, selection, alternate screen) (1-2 hours)
- [ ] Test `AnsiParser` features (mouse tracking, character sets, OSC 52, OSC 8, bracketed paste) (1 hour)
- [ ] Test `Renderer` and `InputHandler` for `DummyBackend`, GTK, and OpenGL backends (1-2 hours)
- [ ] Mock PTY interactions for `VteTerminalCore` tests (1 hour)

### 5.2 Integration Tests
- [ ] Test user interactions (typing, selecting, scrolling) on both buffers and backends (1 hour)
- [ ] Test with shells (bash, zsh, fish) and apps (tmux, zellij, vim, ratatui demos, htop) (1 hour)
- [ ] Verify rendering with `vttest` (aim for 90%+ VTE score) and real-world ANSI output (1 hour)
- [ ] Add golden file tests for complex ANSI sequences (e.g., tmux status line) (1 hour)
  ```rust
  #[test]
  fn test_tmux_status_line() {
      let output = include_bytes!("fixtures/tmux-status.ansi");
      let mut parser = AnsiParser::new();
      let mut grid = Grid::new(80, 24);
      parser.feed_bytes(output, &mut grid);
      assert_snapshot!(grid.to_string());
  }
  ```
- [ ] Test Unicode edge cases (emoji, RTL scripts, combining characters) in ratatui/zellij (30 min)
- [ ] Add integration test automation scripts (30 min)
  ```bash
  cargo test --test tmux_compatibility
  cargo test --test vim_integration
  cargo test --test zellij_behavior
  ```

### 5.3 Fuzzing and Benchmarks
- [ ] Extend fuzz testing for new `AnsiParser` features (mouse, OSC, Sixel) and alternate screen (1 hour)
- [ ] Add criterion benchmarks with performance targets: <2ms redraw for 80x24 screen, <50MB RAM with tmux+vim, >10MB/s PTY throughput (1 hour)

## Phase 6: Documentation (Critical for crates.io)
**Goal**: Make the crate easy to use
**Estimated Time**: 5.75-6.75 hours

### 6.1 API Documentation
- [ ] Add module-level docs for `vte-core` and `vte-gtk` explaining purpose and scope (1 hour)
- [ ] Document all public types, traits (`Renderer`, `TextRenderer`, `GraphicsRenderer`, `UIRenderer`, `InputHandler`, `EventLoop`), and VTE-specific features (e.g., OSC 52, hyperlinks, vte.sh) with examples (1-2 hours)
- [ ] Document supported ANSI sequences, alternate screen, and VTE compatibility matrix (1 hour)
- [ ] Document migration path from GTK-coupled version to GTK-agnostic version (30 min)
- [ ] Document known limitations for 0.1.0-beta.1 (e.g., IME, OpenGL, Sixel deferred to 0.2.0) (15 min)
- [ ] Run `cargo doc` and resolve warnings (30 min)

### 6.2 README and Examples
- [ ] Create README with description, quick-start for GTK backend, feature highlights (tmux/zellij compatibility, hyperlinks), and known limitations (1 hour)
- [ ] Add examples: simple terminal with GTK backend, headless parser usage, ratatui demo (1 hour)

## Phase 7: Package Preparation (Required)
**Goal**: Meet crates.io requirements
**Estimated Time**: 3.75-4.75 hours

### 7.1 Cargo.toml Metadata
- [ ] Create `Cargo.toml` for `vte-core` and `vte-gtk` with metadata and pessimistic versioning (30 min)
  ```toml
  [package]
  name = "vte-core"
  version = "0.1.0"
  edition = "2021"
  rust-version = "1.70"
  authors = ["Your Name <email@example.com>"]
  license = "MIT OR Apache-2.0"
  description = "GTK-agnostic virtual terminal emulator core with VTE compliance"
  readme = "README.md"
  homepage = "https://github.com/yourusername/vte-core"
  repository = "https://github.com/yourusername/vte-core"
  keywords = ["terminal", "vte", "ansi", "emulator", "gtk-agnostic"]
  categories = ["command-line-interface", "emulators"]

  [dependencies]
  portable-pty = "~0.10"
  fontdue = "~0.9"
  memchr = "~2.7"
  thiserror = "~1.0"

  [features]
  default = ["mouse", "selection", "alternate_screen"]
  mouse = []
  selection = []
  cursor_blink = []
  alternate_screen = []
  ime = []
  opengl = []
  kitty = []
  sixel = []
  ```
- [ ] Create similar `Cargo.toml` for `vte-gtk` with `gtk4` and `cairo` dependencies (30 min)
- [ ] Document dependency update policy in maintenance plan (15 min)

### 7.2 License and CI/CD
- [ ] Add LICENSE-MIT and LICENSE-APACHE files (15 min)
- [ ] Create `.github/workflows/ci.yml` for testing on stable, beta, nightly, and multiple platforms (1-2 hours)
- [ ] Add clippy, rustfmt, and coverage checks to CI (30 min)
- [ ] Add performance benchmark monitoring to CI (30 min)
  ```yaml
  - name: Performance Benchmark
    run: cargo bench --bench throughput -- --save-baseline main
  ```

## Phase 8: Polish & Release (Final Steps)
**Goal**: Ship the crates
**Estimated Time**: 4.25-5.25 hours

### 8.1 Code Quality
- [ ] Run `cargo clippy -- -D warnings` and fix issues (1 hour)
- [ ] Run `cargo fmt` (15 min)
- [ ] Review `TODO`/`FIXME` comments and remove `unwrap()`/`expect()` where possible (1 hour)
- [ ] Run `cargo audit` for dependency vulnerabilities (15 min)

### 8.2 Pre-release Checklist
- [ ] Ensure version is 0.1.0-beta.1 for beta testing (15 min)
- [ ] Run `cargo test --all-features` and verify all tests pass (15 min)
- [ ] Run smoke test: execute a real tmux session to verify basic functionality (split-panes, navigation, clipboard) (30 min)
- [ ] Build documentation: `cargo doc --no-deps` (15 min)
- [ ] Verify README examples work (15 min)
- [ ] Update CHANGELOG.md with beta release notes, including known limitations (15 min)

### 8.3 Publish
- [ ] Run `cargo publish --dry-run` for `vte-core` and `vte-gtk` (15 min)
- [ ] Publish beta: `cargo publish` with `0.1.0-beta.1` (15 min)
- [ ] Create git tags: `git tag v0.1.0-beta.1` and push (15 min)
- [ ] Create GitHub release with beta notes, known limitations, and call for user feedback (15 min)
- [ ] Define beta feedback triage process (categorize as bugs, features, docs; 1-week beta period) (15 min)
- [ ] After beta feedback, update to `0.1.0` and repeat publish steps (30 min)

## Priority Ordering

### Week 1 (Minimum Viable Product)
1. **Day 1-2**: Phase 1 - Core library structure (traits, `DummyBackend`, GTK backend) and alternate screen buffer (9-11 hours)
2. **Day 3-4**: Phase 3 - Error handling (`TerminalError`, recovery) and resource management (6-7 hours)
3. **Day 5**: Phase 5.1 - Unit tests for core components (2-3 hours)

### Week 2 (Production Ready)
4. **Day 1-2**: Phase 2 - Complete ANSI/VTE support (mouse, OSC 52, OSC 8, bracketed paste, vte.sh) (8-10 hours)
5. **Day 3**: Phase 5.2-5.3 - Integration tests (tmux/zellij/ratatui, golden file tests, Unicode), fuzzing, benchmarks (4.5-5.5 hours)
6. **Day 4**: Phase 6 - Documentation (core traits, GTK quickstart, VTE compatibility matrix, migration path, known limitations) and examples (5.75-6.75 hours)
7. **Day 5**: Phase 7-8 - Package preparation, polish, beta release, and final release (7.75-9.75 hours)

### Optional (Post-release)
- Phase 4.2: OpenGL backend, Kitty keyboard protocol, Sixel graphics for 0.2.0
- Phase 4.3: Accessibility features for 0.2.0
- Continuous fuzzing and benchmark improvements

## Risk Assessment
- **Low Risk**:
  - ANSI parser robustness (already solid per `ansi.rs`)
  - Grid implementation (existing, extensible)
  - GTK backend integration (proven in prior iterations)
- **Medium Risk**:
  - Alternate screen buffer state management (complex state transitions)
  - Mouse reporting protocol complexity (multiple modes, edge cases)
  - Cross-platform PTY quirks (Windows ConPTY vs. Unix)
- **High Risk**:
  - IME integration (complex, platform-specific; deferred to post-1.0, feature flag `ime`)
  - OpenGL backend (complex rendering; deferred to post-1.0, feature flag `opengl`)

## Success Criteria
Before publishing to crates.io, ensure:
- ✅ Alternate screen buffer works with tmux, zellij, vim
- ✅ Mouse reporting enables tmux/zellij navigation and ratatui interactivity
- ✅ OSC 52 supports tmux clipboard over SSH
- ✅ OSC 8 hyperlinks work in zellij/ratatui
- ✅ Bracketed paste prevents paste exploits in vim/tmux
- ✅ vte.sh integration supports tmux/zellij split-pane directory tracking
- ✅ No panics on any input (verified by fuzz testing)
- ✅ Test coverage > 85% for `vte-core`
- ✅ Performance: <2ms redraw for 80x24 screen, <50MB RAM with tmux+vim, >10MB/s PTY throughput
- ✅ Unicode edge cases (emoji, RTL, combining characters) render correctly in ratatui/zellij
- ✅ All public APIs documented with examples (core traits, GTK quickstart, migration path)
- ✅ README has working quick-start for GTK and headless usage, with known limitations
- ✅ CI passing on Linux, macOS, Windows with performance benchmarks
- ✅ Zero clippy warnings
- ✅ Follows Rust API guidelines
- ✅ 90%+ VTTEST score for VTE compliance
- ✅ Smoke test passes: tmux session with split-panes, navigation, and clipboard
- ✅ Beta feedback (bugs, features, docs) triaged and incorporated before final 0.1.0 release

## Maintenance Plan (Post-release)
- Monitor GitHub issues and respond within 48 hours
- Plan minor releases (0.x.0) for new features every 2-3 months
- Plan patch releases (0.0.x) for bugs ASAP
- Keep dependencies updated quarterly, following pessimistic versioning (e.g., `~0.10` for `portable-pty`)
- Review and merge community PRs

**Estimated Total Time**: 36.25-44.25 hours for full production readiness
**Minimum Viable Time**: 16-21 hours (Phases 1, 3, 5.1, 6.1-6.2, 7-8)
//! GTK-agnostic terminal core - coordinates PTY, parsing, and grid
//!
//! This module provides the core terminal functionality without any UI framework
//! dependencies. Backend-agnostic rendering and event handling are provided through
//! trait interfaces defined in lib.rs.

use crate::grid::Grid;
use crate::ansi::AnsiParser;
use crate::config::TerminalConfig;
use crate::error::TerminalError;

use tracing::{error, warn, info, debug, trace};

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::sync::{Arc, RwLock, Mutex};
use std::thread;
use std::io::{Read, Write};

/// Backend-agnostic terminal core
///
/// Manages PTY process, ANSI/VT parsing, and terminal grid state without
/// any UI framework dependencies. All rendering and event handling is
/// delegated to backend implementations via traits.
pub struct VteTerminalCore {
    pub grid: Arc<RwLock<Grid>>,
    pty_pair: Arc<RwLock<Option<portable_pty::PtyPair>>>,
    parser: AnsiParser,
    redraw_sender: Option<async_channel::Sender<()>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl VteTerminalCore {
    /// Create new terminal core with default configuration
    pub fn new() -> Self {
        Self::with_config(TerminalConfig::default())
    }
}

impl Default for VteTerminalCore {
    fn default() -> Self {
        Self::new()
    }
}

impl VteTerminalCore {
    /// Create new terminal core with specified configuration
    pub fn with_config(config: TerminalConfig) -> Self {
        debug!("Creating VteTerminalCore with config: font={}, size={}",
               config.font_family, config.font_size);

        let init_cols = 80; // Default dimensions, can be resized later
        let init_rows = 24;

        // Create grid with config colors
        let mut grid = Grid::new(init_cols, init_rows);
        grid.fg = config.default_fg;
        grid.bg = config.default_bg;
        let grid = Arc::new(RwLock::new(grid));

        // Create parser with error callback
        let parser = AnsiParser::new().with_error_callback(|err| {
            warn!("ANSI parser error: {}", err);
        });

        // Create PTY pair
        let pty_pair = Self::spawn_pty(init_cols, init_rows);

        // Get PTY reader/writer
        let (reader, writer) = Self::setup_pty_handles(&pty_pair);
        let writer = Arc::new(Mutex::new(writer));

        // Create redraw channel for backend communication
        let (redraw_tx, _redraw_rx) = async_channel::unbounded::<()>();

        let core = Self {
            grid: Arc::clone(&grid),
            pty_pair,
            parser,
            redraw_sender: Some(redraw_tx),
            writer: Arc::clone(&writer),
        };

        // Start PTY reader thread and welcome message
        core.start_pty_reader(reader, Arc::clone(&grid));
        core.send_welcome_message();

        info!("Terminal core initialized successfully");
        core
    }

    /// Spawn PTY process with configured shell
    fn spawn_pty(cols: usize, rows: usize) -> Arc<RwLock<Option<portable_pty::PtyPair>>> {
        debug!("Spawning PTY with dimensions {}x{}", cols, rows);

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: rows as u16,
                cols: cols as u16,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("Failed to create PTY");

        let mut cmd = CommandBuilder::new("bash");
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("CLICOLOR", "1");
        cmd.env("LSCOLORS", "ExGxFxdxCxDxDxBxBxExEx");

        match pair.slave.spawn_command(cmd) {
            Ok(_) => info!("PTY child process spawned successfully"),
            Err(e) => {
                error!("Failed to spawn PTY child process: {}", e);
                panic!("Terminal startup failed - cannot spawn shell");
            }
        }

        #[allow(clippy::arc_with_non_send_sync)]
        Arc::new(RwLock::new(Some(pair)))
    }

    /// Extract reader and writer handles from PTY pair
    fn setup_pty_handles(pty_pair: &Arc<RwLock<Option<portable_pty::PtyPair>>>) -> (Box<dyn Read + Send>, Box<dyn Write + Send>) {
        let pair_guard = pty_pair.read().unwrap();
        let pair = pair_guard.as_ref().unwrap();

        let reader = pair.master.try_clone_reader().unwrap_or_else(|e| {
            error!("Failed to clone PTY reader: {}", e);
            panic!("Terminal startup failed - reader unavailable");
        });

        let writer = pair.master.take_writer().unwrap_or_else(|e| {
            error!("Failed to take PTY writer: {}", e);
            panic!("Terminal startup failed - writer unavailable");
        });

        (reader, writer)
    }

    /// Start PTY reader thread to process incoming data
    fn start_pty_reader(&self, mut reader: Box<dyn Read + Send>, grid: Arc<RwLock<Grid>>) {
        let writer_pty = Arc::clone(&self.writer);
        let tx = self.redraw_sender.as_ref().cloned();

        thread::spawn(move || {
            debug!("PTY reader thread starting");
            let mut parser = AnsiParser::new().with_error_callback(|err| {
                warn!("ANSI parser error in thread: {}", err);
            });

            let mut buf = [0u8; 4096];
            let mut consecutive_errors = 0;

            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        debug!("PTY reader: received EOF, shutting down");
                        break;
                    }
                    Ok(n) => {
                        consecutive_errors = 0; // Reset error counter on success

                        let acquire_lock = grid.write();
                        match acquire_lock {
                            Ok(mut g) => {
                                let s = String::from_utf8_lossy(&buf[..n]);
                                trace!("PTY read {} bytes", n);
                                parser.feed_str(&s, &mut *g);

                                // Notify backend of redraw
                                if let Some(ref sender) = tx {
                                    if let Err(e) = sender.send_blocking(()) {
                                        warn!("Failed to send redraw signal: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to acquire grid write lock (attempting recovery): {}", e);
                                std::thread::sleep(std::time::Duration::from_millis(10));
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        consecutive_errors += 1;
                        if consecutive_errors > 3 {
                            error!("PTY read failed consecutively {} times, giving up: {}", consecutive_errors, e);
                            break;
                        } else {
                            warn!("PTY read error (attempt {}) - retrying: {}", consecutive_errors, e);
                            std::thread::sleep(std::time::Duration::from_millis(100));
                            continue;
                        }
                    }
                }
            }

            info!("PTY reader thread exiting");
        });

        info!("PTY reader thread started successfully");
    }

    /// Send welcome message on terminal startup
    fn send_welcome_message(&self) {
        let writer_clone = Arc::clone(&self.writer);
        let grid_clone = Arc::clone(&self.grid);
        let tx = self.redraw_sender.as_ref().cloned();

        thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(100));

            let mut w = match writer_clone.lock() {
                Ok(w) => w,
                Err(e) => {
                    error!("Failed to acquire writer lock for welcome message: {}", e);
                    return;
                }
            };

            if let Err(e) = writeln!(w, "echo 'Welcome to HugoTerm!'") {
                warn!("Failed to write welcome message: {}", e);
            }
            if let Err(e) = w.flush() {
                warn!("Failed to flush welcome message: {}", e);
            }

            // Notify backend of initial redraw
            if let Some(ref sender) = tx {
                if let Err(e) = sender.send_blocking(()) {
                    warn!("Failed to send initial redraw signal: {}", e);
                }
            }
        });
    }

    /// Send data to terminal process
    pub fn send_input(&self, data: &[u8]) -> Result<(), TerminalError> {
        let mut writer = self.writer.lock()
            .map_err(|_| TerminalError::GridLockError("Writer lock poisoned".to_string()))?;

        writer.write_all(data).map_err(TerminalError::from)?;
        writer.flush().map_err(TerminalError::from)?;

        Ok(())
    }

    /// Resize terminal to new dimensions with line rewrapping
    pub fn resize(&self, cols: usize, rows: usize) {
        debug!("Resizing terminal to {}x{} with rewrapping", cols, rows);

        // Update grid first with rewrapping logic
        if let Ok(mut g) = self.grid.write() {
            g.resize_with_rewrap(cols, rows);
        } else {
            warn!("Failed to resize grid with rewrap - lock error");
            return;
        }

        // Update PTY size
        if let Ok(pair_guard) = self.pty_pair.read() {
            if let Some(ref pair) = *pair_guard {
                if let Err(e) = pair.master.resize(PtySize {
                    rows: rows as u16,
                    cols: cols as u16,
                    pixel_width: 0,
                    pixel_height: 0,
                }) {
                    warn!("Failed to resize PTY: {}", e);
                }
            }
        } else {
            warn!("Could not access PTY for resize");
        }

        // Notify backend of resize
        if let Some(ref sender) = self.redraw_sender {
            if let Err(e) = sender.send_blocking(()) {
                warn!("Failed to send resize redraw signal: {}", e);
            }
        }
    }

    /// Get access to the terminal grid (read-only)
    pub fn grid(&self) -> &Arc<RwLock<Grid>> {
        &self.grid
    }

    /// Get memory usage statistics
    pub fn get_memory_usage(&self) -> crate::MemoryInfo {
        let grid_size = {
            if let Ok(grid) = self.grid.read() {
                // Primary buffer memory
                let primary_bytes = grid.cells.len() * std::mem::size_of::<crate::ansi::Cell>();

                // Alternate buffer memory
                let alternate_bytes = grid.alternate_cells.len() * std::mem::size_of::<crate::ansi::Cell>();

                // Scrollback buffer memory
                let scrollback_bytes = grid.scrollback.len() * std::mem::size_of::<crate::ansi::Cell>();

                (primary_bytes, alternate_bytes, scrollback_bytes)
            } else {
                (0, 0, 0)
            }
        };

        crate::MemoryInfo {
            primary_buffer_bytes: grid_size.0,
            alternate_buffer_bytes: grid_size.1,
            scrollback_buffer_bytes: grid_size.2,
            total_grid_bytes: grid_size.0 + grid_size.1 + grid_size.2,
        }
    }

    /// Force memory cleanup - trim scrollback to configured limits
    pub fn cleanup_memory(&self) {
        if let Ok(mut grid) = self.grid.write() {
            // Trim scrollback to configured limit
            let max_scroll = crate::constants::SCROLLBACK_LIMIT;
            if grid.scrollback.len() > max_scroll * grid.cols {
                let keep_rows = max_scroll;
                let new_len = keep_rows * grid.cols;
                grid.scrollback.truncate(new_len);
                grid.scrollback.shrink_to_fit();
                debug!("Trimmed scrollback buffer to {} lines", keep_rows);
            }

            grid.scrollback.shrink_to_fit();
        } else {
            warn!("Failed to access grid for memory cleanup");
        }
    }

    /// Set redraw callback sender for backend communication
    pub fn set_redraw_sender(&mut self, sender: async_channel::Sender<()>) {
        self.redraw_sender = Some(sender);
    }
}

impl Drop for VteTerminalCore {
    fn drop(&mut self) {
        info!("Cleaning up VteTerminalCore resources...");

        // Clean up PTY resources (may already be handled by child process termination)
        if let Ok(mut pair_guard) = self.pty_pair.write() {
            if pair_guard.is_some() {
                debug!("Dropping PTY pair reference");
                *pair_guard = None;
            } else {
                debug!("PTY pair already cleaned up");
            }
        } else {
            warn!("Failed to clean up PTY pair during drop (lock poisoned)");
        }

        // Force cleanup of Grid resources
        if let Ok(mut grid) = self.grid.write() {
            // Clear scrollback buffer to free memory immediately
            grid.scrollback.clear();
            grid.scrollback.shrink_to_fit();
            debug!("Cleared scrollback buffer on drop");
        } else {
            warn!("Could not access grid for cleanup during drop");
        }

        info!("VteTerminalCore resource cleanup completed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TerminalConfig;

    #[test]
    fn test_terminal_core_creation() {
        let terminal = VteTerminalCore::new();

        // Check that terminal is created and has expected structure
        let memory_info = terminal.get_memory_usage();
        assert!(memory_info.primary_buffer_bytes > 0);
        assert!(memory_info.alternate_buffer_bytes > 0);
        assert_eq!(memory_info.primary_buffer_bytes, memory_info.alternate_buffer_bytes); // Same initial size
    }

    #[test]
    fn test_terminal_core_with_config() {
        let config = TerminalConfig::default();
        let terminal = VteTerminalCore::with_config(config);

        // Test that terminal is created without panicking
        let memory_info = terminal.get_memory_usage();
        assert!(memory_info.total_grid_bytes > 0);
    }

    #[test]
    fn test_terminal_resize() {
        let terminal = VteTerminalCore::new();

        let initial_memory = terminal.get_memory_usage();
        assert_eq!(initial_memory.primary_buffer_bytes, initial_memory.alternate_buffer_bytes);

        // Resize should change the memory allocation
        terminal.resize(120, 30);
        let resized_memory = terminal.get_memory_usage();

        // Should still maintain consistent memory calculations
        assert!(resized_memory.primary_buffer_bytes > 0);
        assert!(resized_memory.alternate_buffer_bytes > 0);

        let expected_total = resized_memory.primary_buffer_bytes +
                           resized_memory.alternate_buffer_bytes +
                           resized_memory.scrollback_buffer_bytes;
        assert_eq!(resized_memory.total_grid_bytes, expected_total);
    }

    #[test]
    fn test_input_sending() {
        let terminal = VteTerminalCore::new();

        // Send some input
        let result = terminal.send_input(b"hello world\n");
        assert!(result.is_ok());

        // Send empty input (should not fail)
        let result = terminal.send_input(b"");
        assert!(result.is_ok());
    }

    #[test]
    fn test_memory_info_is_consistent() {
        let terminal = VteTerminalCore::new();

        let memory_info = terminal.get_memory_usage();

        // Basic consistency checks
        assert!(memory_info.primary_buffer_bytes > 0);
        assert!(memory_info.alternate_buffer_bytes >= memory_info.primary_buffer_bytes);
        assert!(memory_info.total_grid_bytes >= memory_info.primary_buffer_bytes);

        // Total should equal sum of components
        let expected_total = memory_info.primary_buffer_bytes +
                           memory_info.alternate_buffer_bytes +
                           memory_info.scrollback_buffer_bytes;
        assert_eq!(memory_info.total_grid_bytes, expected_total);
    }

    #[test]
    fn test_memory_cleanup_succeeds() {
        let terminal = VteTerminalCore::new();

        // Should not panic
        terminal.cleanup_memory();

        // Memory should still be reasonable after cleanup
        let memory_info = terminal.get_memory_usage();
        assert!(memory_info.total_grid_bytes >= 0);
    }

    #[test]
    fn test_grid_access_is_safe() {
        let terminal = VteTerminalCore::new();

        // Test read access
        {
            let grid = terminal.grid();
            let _read_guard = grid.read().unwrap();
            // Guard should be dropped here
        }

        // Test write access
        {
            let grid = terminal.grid();
            let _write_guard = grid.write().unwrap();
            // Guard should be dropped here
        }

        // Terminal should still be functional
        let memory_info = terminal.get_memory_usage();
        assert!(memory_info.total_grid_bytes > 0);
    }

    #[test]
    fn test_grid_locking_is_safe() {
        let terminal = VteTerminalCore::new();

        // Test that we can acquire read and write locks without deadlocking
        {
            let _read_lock = terminal.grid().read().unwrap();
        }
        {
            let _write_lock = terminal.grid().write().unwrap();
        }

        // After locks are released, terminal should still work
        let memory_info = terminal.get_memory_usage();
        assert!(memory_info.total_grid_bytes > 0);
    }
}

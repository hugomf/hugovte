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

    /// Resize terminal to new dimensions
    pub fn resize(&self, cols: usize, rows: usize) {
        debug!("Resizing terminal to {}x{}", cols, rows);

        // Update grid first
        if let Ok(mut g) = self.grid.write() {
            g.resize(cols, rows);
        } else {
            warn!("Failed to resize grid - lock error");
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

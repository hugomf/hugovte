//! Dummy backend for testing trait implementations without GTK

use crate::{Renderer, TextRenderer, GraphicsRenderer, UIRenderer, InputHandler, EventLoop, CursorShape, ImageData, Grid, Cell};
use crate::drawing::CharMetrics;
use std::io::Write;
use std::sync::{Arc, RwLock, Mutex};

/// Dummy backend that implements all traits for testing
pub struct DummyBackend {
    text_renderer: DummyTextRenderer,
    graphics_renderer: DummyGraphicsRenderer,
    ui_renderer: DummyUIRenderer,
}

impl Default for DummyBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl DummyBackend {
    /// Create a new dummy backend
    pub fn new() -> Self {
        DummyBackend {
            text_renderer: DummyTextRenderer::default(),
            graphics_renderer: DummyGraphicsRenderer::default(),
            ui_renderer: DummyUIRenderer::default(),
        }
    }
}

impl Renderer for DummyBackend {
    fn text_renderer(&mut self) -> &mut dyn TextRenderer {
        &mut self.text_renderer
    }
    fn graphics_renderer(&mut self) -> &mut dyn GraphicsRenderer {
        &mut self.graphics_renderer
    }
    fn ui_renderer(&mut self) -> &mut dyn UIRenderer {
        &mut self.ui_renderer
    }
}

// Combine DummyBackend with separate input handler and event loop for proper trait composition
pub struct CompleteDummyBackend {
    backend: DummyBackend,
    input_handler: DummyInputHandler,
    event_loop: DummyEventLoop,
}

impl CompleteDummyBackend {
    pub fn new() -> Self {
        CompleteDummyBackend {
            backend: DummyBackend::new(),
            input_handler: DummyInputHandler {
                key_events: Vec::new(),
                mouse_events: Vec::new(),
                scroll_events: Vec::new(),
            },
            event_loop: DummyEventLoop {
                redraws: Vec::new(),
                timers: Vec::new(),
            },
        }
    }
}

use crate::Backend;
impl Backend for CompleteDummyBackend {
    fn resize(&mut self, _cols: usize, _rows: usize) {
        // For testing, we could track resize operations
        // For now, just a placeholder
    }
}

impl Renderer for CompleteDummyBackend {
    fn text_renderer(&mut self) -> &mut dyn TextRenderer {
        self.backend.text_renderer()
    }

    fn graphics_renderer(&mut self) -> &mut dyn GraphicsRenderer {
        self.backend.graphics_renderer()
    }

    fn ui_renderer(&mut self) -> &mut dyn UIRenderer {
        self.backend.ui_renderer()
    }
}

impl InputHandler for CompleteDummyBackend {
    fn handle_key(&mut self, key: crate::ansi::KeyEvent, grid: &Arc<RwLock<Grid>>, writer: &Arc<Mutex<Box<dyn Write + Send>>>) {
        self.input_handler.handle_key(key, grid, writer);
    }

    fn handle_mouse(&mut self, event: crate::ansi::MouseEvent, grid: &Arc<RwLock<Grid>>) {
        self.input_handler.handle_mouse(event, grid);
    }

    fn handle_scroll(&mut self, delta: f64, grid: &Arc<RwLock<Grid>>) {
        self.input_handler.handle_scroll(delta, grid);
    }
}

impl EventLoop for CompleteDummyBackend {
    fn schedule_redraw(&mut self, callback: Box<dyn FnMut()>) {
        self.event_loop.schedule_redraw(callback);
    }

    fn schedule_timer(&mut self, interval_ms: u64, callback: Box<dyn FnMut() -> bool>) -> bool {
        self.event_loop.schedule_timer(interval_ms, callback)
    }
}

/// Dummy text renderer - records operations for testing
pub struct DummyTextRenderer {
    pub cells: Vec<(usize, usize, Cell)>,
    pub fonts: Vec<(String, f64)>,
}

impl Default for DummyTextRenderer {
    fn default() -> Self {
        DummyTextRenderer {
            cells: Vec::new(),
            fonts: Vec::new(),
        }
    }
}

impl DummyTextRenderer {
    /// Get drawn cells for testing
    pub fn get_cells(&self) -> &[(usize, usize, Cell)] {
        &self.cells
    }

    /// Clear recorded operations
    pub fn clear(&mut self) {
        self.cells.clear();
        self.fonts.clear();
    }
}

impl TextRenderer for DummyTextRenderer {
    fn draw_cell(&mut self, row: usize, col: usize, cell: &Cell) {
        self.cells.push((row, col, cell.clone()));
    }

    fn set_font(&mut self, family: &str, size: f64) {
        self.fonts.push((family.to_string(), size));
    }

    fn get_char_metrics(&self, _ch: char) -> CharMetrics {
        // Return standard monospace metrics
        CharMetrics {
            width: 8.0,
            height: 16.0,
            ascent: 12.0,
        }
    }
}

/// Dummy graphics renderer - records operations
pub struct DummyGraphicsRenderer {
    pub sixel_data: Vec<Vec<u8>>,
    pub images: Vec<ImageData>,
}

impl Default for DummyGraphicsRenderer {
    fn default() -> Self {
        DummyGraphicsRenderer {
            sixel_data: Vec::new(),
            images: Vec::new(),
        }
    }
}

impl DummyGraphicsRenderer {
    /// Clear recorded operations
    pub fn clear(&mut self) {
        self.sixel_data.clear();
        self.images.clear();
    }
}

impl GraphicsRenderer for DummyGraphicsRenderer {
    fn draw_sixel(&mut self, data: &[u8], _x: usize, _y: usize) {
        self.sixel_data.push(data.to_vec());
    }

    fn draw_image(&mut self, image: ImageData, _x: usize, _y: usize) {
        self.images.push(image);
    }
}

/// Dummy UI renderer - records operations
pub struct DummyUIRenderer {
    pub cleared: bool,
    pub flushed: bool,
    pub cursor_shape: Option<CursorShape>,
}

impl Default for DummyUIRenderer {
    fn default() -> Self {
        DummyUIRenderer {
            cleared: false,
            flushed: false,
            cursor_shape: None,
        }
    }
}

impl DummyUIRenderer {
    /// Clear recorded operations
    pub fn clear(&mut self) {
        self.cleared = false;
        self.flushed = false;
        self.cursor_shape = None;
    }
}

impl UIRenderer for DummyUIRenderer {
    fn clear(&mut self) {
        self.cleared = true;
    }

    fn flush(&mut self) {
        self.flushed = true;
    }

    fn set_cursor_shape(&mut self, shape: CursorShape) {
        self.cursor_shape = Some(shape);
    }
}

/// Dummy input handler - records operations
pub struct DummyInputHandler {
    pub key_events: Vec<crate::ansi::KeyEvent>,
    pub mouse_events: Vec<crate::ansi::MouseEvent>,
    pub scroll_events: Vec<f64>,
}

impl DummyInputHandler {
    /// Clear recorded operations
    pub fn clear(&mut self) {
        self.key_events.clear();
        self.mouse_events.clear();
        self.scroll_events.clear();
    }
}

impl InputHandler for DummyInputHandler {
    fn handle_key(
        &mut self,
        key: crate::ansi::KeyEvent,
        _grid: &Arc<RwLock<Grid>>,
        _writer: &Arc<Mutex<Box<dyn Write + Send>>>,
    ) {
        self.key_events.push(key);
    }

    fn handle_mouse(&mut self, event: crate::ansi::MouseEvent, _grid: &Arc<RwLock<Grid>>) {
        self.mouse_events.push(event);
    }

    fn handle_scroll(&mut self, delta: f64, _grid: &Arc<RwLock<Grid>>) {
        self.scroll_events.push(delta);
    }
}

/// Dummy event loop - records operations
pub struct DummyEventLoop {
    pub redraws: Vec<Box<dyn FnMut()>>,
    pub timers: Vec<u64>,
}

impl DummyEventLoop {
    /// Clear recorded operations
    pub fn clear(&mut self) {
        self.redraws.clear();
        self.timers.clear();
    }
}

impl EventLoop for DummyEventLoop {
    fn schedule_redraw(&mut self, callback: Box<dyn FnMut()>) {
        self.redraws.push(callback);
    }

    fn schedule_timer(&mut self, interval_ms: u64, _callback: Box<dyn FnMut() -> bool>) -> bool {
        self.timers.push(interval_ms);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ansi::KeyEvent;

    #[test]
    fn test_dummy_renderer() {
        let mut backend = DummyBackend::new();

        // Test text rendering
        let cell = Cell::default();
        backend.text_renderer().draw_cell(0, 0, &cell);
        backend.text_renderer().set_font("monospace", 12.0);

        let metrics = backend.text_renderer().get_char_metrics('A');
        assert_eq!(metrics.width, 8.0);
        assert_eq!(metrics.height, 16.0);
        assert_eq!(metrics.ascent, 12.0);

        // Test UI renderer
        backend.ui_renderer().set_cursor_shape(CursorShape::Block);
        backend.ui_renderer().clear();
        backend.ui_renderer().flush();

        // Test graphics renderer
        let image_data = ImageData {
            data: vec![0, 1, 2, 3],
            width: 2,
            height: 1,
        };
        backend.graphics_renderer().draw_image(image_data, 10, 20);
    }

    #[test]
    fn test_dummy_input_handler() {
        let mut handler = DummyInputHandler {
            key_events: Vec::new(),
            mouse_events: Vec::new(),
            scroll_events: Vec::new(),
        };

        let grid = Arc::new(RwLock::new(Grid::new(80, 24)));
        let writer = Arc::new(Mutex::new(Box::new(std::io::sink()) as Box<dyn Write + Send>));

        let key_event = KeyEvent { keyval: 'a' as u32, state: 0 };
        handler.handle_key(key_event, &grid, &writer);
        handler.handle_scroll(1.0, &grid);

        assert_eq!(handler.key_events.len(), 1);
        assert_eq!(handler.scroll_events.len(), 1);
    }

    #[test]
    fn test_dummy_event_loop() {
        let mut event_loop = DummyEventLoop {
            redraws: Vec::new(),
            timers: Vec::new(),
        };

        event_loop.schedule_redraw(Box::new(|| {}));
        let _ = event_loop.schedule_timer(1000, Box::new(|| false));

        assert_eq!(event_loop.redraws.len(), 1);
        assert_eq!(event_loop.timers.len(), 1);
    }
}

#[cfg(test)]
mod resource_management_tests {

    use super::*;
    use crate::terminal::VteTerminalCore;
    use crate::config::TerminalConfig;

    #[test]
    fn test_memory_usage_reporting() {
        let config = TerminalConfig {
            draw_grid_lines: false,
            grid_line_alpha: 0.0,
            default_fg: Default::default(),
            default_bg: Default::default(),
            font_family: "monospace".to_string(),
            font_size: 12.0,
            enable_cursor_blink: false,
            cursor_blink_interval_ms: 500,
            enable_selection: false,
            scrollback_limit: 1000,
            click_timeout_ms: 300,
        };

        let terminal = VteTerminalCore::with_config(config);
        let memory_info = terminal.get_memory_usage();

        // MemoryInfo should have positive values for all buffers
        assert!(memory_info.primary_buffer_bytes > 0);
        assert!(memory_info.alternate_buffer_bytes > 0);
        assert!(memory_info.total_grid_bytes > 0);

        // Total should equal sum of all buffers
        let expected_total = memory_info.primary_buffer_bytes +
                           memory_info.alternate_buffer_bytes +
                           memory_info.scrollback_buffer_bytes;
        assert_eq!(memory_info.total_grid_bytes, expected_total);

        // Primary and alternate buffers should have same size initially
        assert_eq!(memory_info.primary_buffer_bytes, memory_info.alternate_buffer_bytes);
    }

    #[test]
    fn test_memory_cleanup_functionality() {
        let config = TerminalConfig {
            draw_grid_lines: false,
            grid_line_alpha: 0.0,
            default_fg: Default::default(),
            default_bg: Default::default(),
            font_family: "monospace".to_string(),
            font_size: 12.0,
            enable_cursor_blink: false,
            cursor_blink_interval_ms: 500,
            enable_selection: false,
            scrollback_limit: 1000,
            click_timeout_ms: 300,
        };

        let terminal = VteTerminalCore::with_config(config);

        // Get initial memory usage
        let initial_memory = terminal.get_memory_usage();
        let initial_scrollback = initial_memory.scrollback_buffer_bytes;

        // Call cleanup_memory to test the function doesn't panic
        // This will shrink scrollback to its configured limit if needed
        terminal.cleanup_memory();

        // Get memory usage after cleanup
        let after_cleanup_memory = terminal.get_memory_usage();

        // Scrollback should be <= initial size (may be smaller after shrink_to_fit)
        assert!(after_cleanup_memory.scrollback_buffer_bytes <= initial_scrollback);

        // Total memory should still be reasonable
        assert!(after_cleanup_memory.total_grid_bytes > 0);

        // This is more of a smoke test to ensure cleanup_memory runs without panicking
        // In a real terminal with lots of scrollback, this would actually reduce memory usage
    }

    #[test]
    fn test_memory_info_direct_access() {
        // Test MemoryInfo struct directly
        let memory_info = crate::MemoryInfo {
            primary_buffer_bytes: 1024,
            alternate_buffer_bytes: 1024,
            scrollback_buffer_bytes: 512,
            total_grid_bytes: 2560,
        };

        assert_eq!(memory_info.primary_buffer_bytes, 1024);
        assert_eq!(memory_info.alternate_buffer_bytes, 1024);
        assert_eq!(memory_info.scrollback_buffer_bytes, 512);
        assert_eq!(memory_info.total_grid_bytes, 2560);
    }
}

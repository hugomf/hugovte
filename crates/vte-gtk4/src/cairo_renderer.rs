//! Cairo renderer implementations for GTK4 backend

use cairo::{Context, FontSlant, FontWeight, ImageSurface, Format};
use vte_core::{
    ImageData, Cell, Color, CursorShape,
    TextRenderer, GraphicsRenderer, UIRenderer
};
use vte_core::font::{FontCache, FontWeight as VteFontWeight, FontSlant as VteFontSlant};
use vte_core::drawing::{CharMetrics, DrawingCache};
use std::f64::consts::PI;

/// Cairo-based text renderer using FontCache with fallback support
pub struct CairoTextRenderer {
    context: cairo::Context,
    font_cache: FontCache,
    cell_width: f64,
    cell_height: f64,
}

impl CairoTextRenderer {
    pub fn new(
        context: cairo::Context,
        font_cache: FontCache,
        cell_width: f64,
        cell_height: f64,
    ) -> Result<Self, cairo::Error> {
        Ok(CairoTextRenderer {
            context,
            font_cache,
            cell_width,
            cell_height,
        })
    }
}

impl TextRenderer for CairoTextRenderer {
    fn draw_cell(&mut self, row: usize, col: usize, cell: &Cell) {
        // Draw background if not transparent
        if cell.bg.a > 0.01 {
            self.context.set_source_rgba(cell.bg.r, cell.bg.g, cell.bg.b, cell.bg.a);
            self.context.rectangle(
                col as f64 * self.cell_width,
                row as f64 * self.cell_height,
                self.cell_width,
                self.cell_height,
            );
            self.context.fill().unwrap();
        }

        // Draw text if not null character
        if cell.ch != '\0' {
            // Select font with fallback support
            let vte_font_weight = if cell.bold { VteFontWeight::Bold } else { VteFontWeight::Normal };
            let vte_font_slant = if cell.italic { VteFontSlant::Italic } else { VteFontSlant::Normal };

            // Try to get font metrics with fallback
            match self.font_cache.get_font_metrics(cell.ch, vte_font_weight, vte_font_slant) {
                Ok((_font, metrics)) => {
                    // Use fontdue rasterization for best Unicode support
                    match self.font_cache.rasterize_glyph(cell.ch, vte_font_weight, vte_font_slant) {
                        Ok((bitmap, width, height)) => {
                            // Create Cairo surface from glyph bitmap and draw it
                            if let Ok(surface) = ImageSurface::create_for_data(
                                bitmap,
                                Format::A8, // Grayscale alpha-only
                                width as i32,
                                height as i32,
                                width as i32, // stride = width for A8
                            ) {
                                let x = col as f64 * self.cell_width;
                                let y = row as f64 * self.cell_height;

                                // Position glyph using estimated ascent (cell height * 0.75)
                                let glyph_x = x;
                                let glyph_y = y + self.cell_height * 0.75;

                                self.context.set_source_rgba(cell.fg.r, cell.fg.g, cell.fg.b, cell.fg.a);
                                self.context.mask_surface(&surface, glyph_x, glyph_y).unwrap();
                            } else {
                                // Fallback to Cairo text rendering
                                self.fallback_draw_text(cell, row, col);
                            }
                        }
                        Err(_) => {
                            // Fallback to Cairo text rendering
                            self.fallback_draw_text(cell, row, col);
                        }
                    }
                }
                Err(_) => {
                    // Fallback to Cairo text rendering if font system fails
                    self.fallback_draw_text(cell, row, col);
                }
            }
        }

        // Draw underline if needed
        if cell.underline {
            self.context.set_source_rgba(cell.fg.r, cell.fg.g, cell.fg.b, cell.fg.a);
            let underline_y = row as f64 * self.cell_height + (self.cell_height * 0.85); // Baseline + descent
            self.context.set_line_width(self.cell_height * 0.05); // 5% of cell height

            let start_x = col as f64 * self.cell_width;
            let end_x = (col + 1) as f64 * self.cell_width;

            self.context.move_to(start_x, underline_y);
            self.context.line_to(end_x, underline_y);
            self.context.stroke().unwrap();
        }
    }

    fn set_font(&mut self, _family: &str, _size: f64) {
        // Font is managed by FontCache - this method is for compatibility
        // Actual font selection happens in draw_cell with fallback chains
    }

    fn get_char_metrics(&self, _ch: char) -> CharMetrics {
        // Return default monospace metrics for trait compatibility
        // Actual glyph metrics are handled in draw_cell with caching
        CharMetrics {
            width: self.cell_width,
            height: self.cell_height,
            ascent: self.cell_height * 0.75,
        }
    }
}

impl CairoTextRenderer {
    /// Fallback text rendering using Cairo's built-in font system
    fn fallback_draw_text(&self, cell: &Cell, row: usize, col: usize) {
        // Use system monospace font as last resort
        self.context.select_font_face("monospace", FontSlant::Normal, FontWeight::Normal);
        self.context.set_font_size(self.cell_height * 0.7);

        let x = col as f64 * self.cell_width;
        let y = row as f64 * self.cell_height + (self.cell_height * 0.75); // Baseline

        self.context.set_source_rgba(cell.fg.r, cell.fg.g, cell.fg.b, cell.fg.a);
        self.context.move_to(x, y);
        self.context.show_text(&cell.ch.to_string()).unwrap();
    }
}

/// Cairo-based graphics renderer for images and sixel graphics
pub struct CairoGraphicsRenderer {
    context: cairo::Context,
}

impl CairoGraphicsRenderer {
    pub fn new(context: cairo::Context) -> Self {
        CairoGraphicsRenderer { context }
    }
}

impl GraphicsRenderer for CairoGraphicsRenderer {
    fn draw_sixel(&mut self, _data: &[u8], _x: usize, _y: usize) {
        // TODO: Implement sixel graphics support
        // For now, just draw a placeholder
        self.context.set_source_rgb(0.5, 0.5, 0.5);
        self.context.rectangle(_x as f64, _y as f64, 10.0, 10.0);
        self.context.fill().unwrap();
    }

    fn draw_image(&mut self, image: ImageData, x: usize, y: usize) {
        if image.data.is_empty() {
            return;
        }

        // Create a surface from the image data
        if let Ok(surface) = cairo::ImageSurface::create_for_data(
            image.data,
            cairo::Format::ARgb32,
            image.width as i32,
            image.height as i32,
            image.width as i32 * 4, // 4 bytes per pixel for ARGB32
        ) {
            self.context.set_source_surface(&surface, x as f64, y as f64);
            self.context.paint().unwrap();
        }
    }
}

/// Cairo-based UI renderer for clear/flush operations
pub struct CairoUIRenderer {
    context: cairo::Context,
}

impl CairoUIRenderer {
    pub fn new(context: cairo::Context) -> Self {
        CairoUIRenderer { context }
    }
}

impl UIRenderer for CairoUIRenderer {
    fn clear(&mut self) {
        // Don't clear - preserve transparency for GTK
    }

    fn flush(&mut self) {
        // Cairo operations are already flushed
    }

    fn set_cursor_shape(&mut self, _shape: vte_core::CursorShape) {
        // GTK handles cursor shape through CSS/properties
    }

    fn handle_hyperlink(&mut self, url: &str) -> bool {
        // Handle HTTPS hyperlinks by opening them in the default browser
        if url.starts_with("https://") || url.starts_with("http://") {
            use std::process::Command;

            // Cross-platform: try xdg-open (Linux), open (macOS), start (Windows)
            #[cfg(target_os = "linux")]
            let cmd_result = Command::new("xdg-open").arg(url).spawn();

            #[cfg(target_os = "macos")]
            let cmd_result = Command::new("open").arg(url).spawn();

            #[cfg(target_os = "windows")]
            let cmd_result = {
                use std::os::windows::process::CommandExt;
                Command::new("cmd")
                    .args(&["/C", "start", url])
                    .creation_flags(0x00000008) // DETACHED_PROCESS
                    .spawn()
            };

            #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
            let cmd_result = Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "Unsupported platform for hyperlink handling"));

            match cmd_result {
                Ok(_) => {
                    eprintln!("Opened hyperlink: {}", url);
                    true
                }
                Err(e) => {
                    eprintln!("Failed to open hyperlink {}: {}", url, e);
                    false
                }
            }
        } else {
            // For non-HTTPS links, we could emit a signal or call a callback
            // For now, just log and return false
            eprintln!("Unsupported hyperlink protocol: {}", url);
            false
        }
    }
}

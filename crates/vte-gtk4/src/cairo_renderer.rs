//! Cairo renderer implementations for GTK4 backend

use cairo::{Context, FontSlant, FontWeight};
use vte_core::{
    ImageData, CharMetrics, Cell, Color, CursorShape,
    TextRenderer, GraphicsRenderer, UIRenderer
};
use vte_core::drawing::DrawingCache;
use std::f64::consts::PI;

/// Cairo-based text renderer using fontdue-based DrawingCache
pub struct CairoTextRenderer {
    context: cairo::Context,
    drawing_cache: DrawingCache,
}

impl CairoTextRenderer {
    pub fn new(
        context: cairo::Context,
        drawing_cache: DrawingCache,
    ) -> Result<Self, cairo::Error> {
        Ok(CairoTextRenderer {
            context,
            drawing_cache,
        })
    }
}

impl TextRenderer for CairoTextRenderer {
    fn draw_cell(&mut self, row: usize, col: usize, cell: &Cell) {
        // Get metrics from drawing cache
        let cell_metrics = self.drawing_cache.get_char_metrics(cell.ch);
        let char_width = self.drawing_cache.char_width();
        let char_height = self.drawing_cache.char_height();
        let char_ascent = self.drawing_cache.ascent();

        // Select appropriate font variant
        let slant = if cell.italic { FontSlant::Italic } else { FontSlant::Normal };
        let weight = if cell.bold { FontWeight::Bold } else { FontWeight::Normal };

        self.context.select_font_face("monospace", slant, weight);
        self.context.set_font_size(self.drawing_cache.font_size());

        // Draw background if not transparent
        if cell.bg.a > 0.01 {
            self.context.set_source_rgba(cell.bg.r, cell.bg.g, cell.bg.b, cell.bg.a);
            self.context.rectangle(
                col as f64 * char_width,
                row as f64 * char_height,
                char_width,
                char_height,
            );
            self.context.fill().unwrap();
        }

        // Draw text if not null character
        if cell.ch != '\0' {
            let x = col as f64 * char_width;
            let y = row as f64 * char_height + char_ascent;

            self.context.set_source_rgba(cell.fg.r, cell.fg.g, cell.fg.b, cell.fg.a);
            self.context.move_to(x, y);
            self.context.show_text(&cell.ch.to_string()).unwrap();
        }

        // Draw underline if needed
        if cell.underline {
            self.context.set_source_rgba(cell.fg.r, cell.fg.g, cell.fg.b, cell.fg.a);
            let underline_pos = self.drawing_cache.get_underline_position();
            let underline_thickness = self.drawing_cache.get_underline_thickness();
            let y = row as f64 * char_height + underline_pos;
            self.context.set_line_width(underline_thickness);
            self.context.move_to(col as f64 * char_width, y);
            self.context.line_to((col + 1) as f64 * char_width, y);
            self.context.stroke().unwrap();
        }
    }

    fn set_font(&mut self, family: &str, size: f64) {
        self.context.select_font_face(family, FontSlant::Normal, FontWeight::Normal);
        self.context.set_font_size(size);
    }

    fn get_char_metrics(&self, ch: char) -> CharMetrics {
        let metrics = self.drawing_cache.get_char_metrics(ch);
        CharMetrics {
            width: metrics.width,
            height: metrics.height,
            ascent: metrics.ascent,
        }
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
}

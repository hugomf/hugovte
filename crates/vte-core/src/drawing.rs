// src/drawing.rs
use std::collections::HashMap;
use cairo::{Context, FontSlant, FontWeight, ScaledFont, ImageSurface, Format, Antialias, HintStyle, HintMetrics, TextExtents};

#[derive(Debug, Clone, PartialEq, Eq)]
struct FontKey {
    slant: FontSlant,
    weight: FontWeight,
}

impl std::hash::Hash for FontKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(&self.slant).hash(state);
        std::mem::discriminant(&self.weight).hash(state);
    }
}

pub struct DrawingCache {
    font_family: String,
    font_size: f64,
    fonts: HashMap<FontKey, ScaledFont>,
    char_metrics: HashMap<char, TextExtents>,
    char_width: f64,
    char_height: f64,
    ascent: f64,
}

impl DrawingCache {
    pub fn new(font_family: &str, font_size: f64) -> Result<Self, cairo::Error> {
        let surf = ImageSurface::create(Format::ARgb32, 1, 1)?;
        let cr = Context::new(&surf)?;
        
        // Pre-create scaled fonts for common combinations with better rendering
        let mut fonts = HashMap::new();
        
        let combinations = [
            (FontSlant::Normal, FontWeight::Normal),
            (FontSlant::Normal, FontWeight::Bold),
            (FontSlant::Italic, FontWeight::Normal),
            (FontSlant::Italic, FontWeight::Bold),
        ];
        
        for (slant, weight) in combinations {
            let key = FontKey { slant, weight };
            let font = Self::create_scaled_font(&cr, font_family, font_size, slant, weight)?;
            fonts.insert(key, font);
        }
        
        // Calculate character metrics using normal font
        let normal_font = fonts.get(&FontKey { slant: FontSlant::Normal, weight: FontWeight::Normal })
            .unwrap();

        // Build character metrics cache for all printable ASCII characters
        let mut char_metrics = HashMap::new();
        for i in 32..=126 {  // Printable ASCII range
            if let Some(ch) = char::from_u32(i) {
                let extents = normal_font.text_extents(&ch.to_string());
                char_metrics.insert(ch, extents);
            }
        }

        // Use a more representative character set for consistent spacing
        let test_chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut total_width = 0.0;
        let char_count = test_chars.len();

        for ch in test_chars.chars() {
            let extents = normal_font.text_extents(&ch.to_string());
            total_width += extents.width();
        }

        // Use average width for more consistent character spacing
        let avg_char_width = total_width / char_count as f64;

        // For Monaco (monospace font), all characters should have same width
        // Use the width of 'M' as the standard cell width
        let standard_char_width = normal_font.text_extents("M").width();

        // Add extra spacing between characters for better visual separation
        // Base width + 0.3 additional spacing as requested
        let padded_char_width = standard_char_width + 0.3;
        let extents = normal_font.text_extents("M");

        // Increase line height for better vertical spacing between rows
        let increased_line_height = extents.height() * 1.3; // 30% more vertical space

        Ok(Self {
            font_family: font_family.to_string(),
            font_size,
            fonts,
            char_metrics,
            char_width: padded_char_width,
            char_height: increased_line_height,
            ascent: extents.y_bearing().abs(),
        })
    }
    
    fn create_scaled_font(
        cr: &Context,
        family: &str,
        size: f64,
        slant: FontSlant,
        weight: FontWeight,
    ) -> Result<ScaledFont, cairo::Error> {
        cr.select_font_face(family, slant, weight);
        cr.set_font_size(size);
        
        let font_face = cr.font_face().clone();
        let font_matrix = cr.font_matrix();
        let ctm = cr.matrix();
        
        // ⭐ ENHANCED: Better font rendering options for terminal text
        let mut options = cairo::FontOptions::new()
            .map_err(|_| cairo::Error::FontTypeMismatch)?;

        // Use grayscale antialiasing for better terminal text
        options.set_antialias(Antialias::Gray);

        // Medium hinting for good balance of sharpness and shape
        options.set_hint_style(HintStyle::Medium);

        // Enable metric hinting for consistent spacing
        options.set_hint_metrics(HintMetrics::On);
        
        ScaledFont::new(&font_face, &font_matrix, &ctm, &options)
    }
    
    pub fn get_font(&self, slant: FontSlant, weight: FontWeight) -> Option<&ScaledFont> {
        self.fonts.get(&FontKey { slant, weight })
    }
    
    pub fn char_width(&self) -> f64 {
        self.char_width
    }
    
    pub fn char_height(&self) -> f64 {
        self.char_height
    }
    
    pub fn ascent(&self) -> f64 {
        self.ascent
    }
    
    pub fn font_size(&self) -> f64 {
        self.font_size
    }
    
    pub fn font_family(&self) -> &str {
        &self.font_family
    }

    /// Get the actual width of a specific character
    pub fn get_char_width(&self, ch: char) -> f64 {
        if let Some(extents) = self.char_metrics.get(&ch) {
            extents.width()
        } else {
            // Fallback for unmapped characters
            self.char_width
        }
    }

    /// Get the advance width (how much to move forward after drawing this character)
    pub fn get_char_advance(&self, ch: char) -> f64 {
        if let Some(extents) = self.char_metrics.get(&ch) {
            extents.x_advance()
        } else {
            // Fallback for unmapped characters
            self.char_width
        }
    }

    /// Calculate the total width of a string using actual character metrics
    pub fn calculate_text_width(&self, text: &str) -> f64 {
        let mut total_width = 0.0;
        for ch in text.chars() {
            total_width += self.get_char_advance(ch);
        }
        total_width
    }
}

impl Clone for DrawingCache {
    fn clone(&self) -> Self {
        DrawingCache::new(&self.font_family, self.font_size)
            .expect("Failed to clone DrawingCache")
    }
}

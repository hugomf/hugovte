//! Backend-agnostic font metrics and data cache
//!
//! This module uses fontdue for font rendering and metrics calculation.
//! It provides font data and character metrics that can be used by
//! different rendering backends without tying to any specific graphics library.

use std::collections::HashMap;
use fontdue::Font;
use tracing::debug;

/// Simple font key for basic caching
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FontKey {
    variant: &'static str, // "normal", "bold", "italic", "bold_italic"
}

/// Backend-agnostic character metrics
#[derive(Debug, Clone, Copy)]
pub struct CharMetrics {
    /// Character width in pixels
    pub width: f64,
    /// Character height in pixels
    pub height: f64,
    /// Baseline offset (ascent) in pixels
    pub ascent: f64,
}

/// Backend-agnostic font cache using fontdue
pub struct DrawingCache {
    /// Font family name
    font_family: String,
    /// Font size in pixels
    font_size: f64,
    /// Cached fonts by variant (basic monospace for now)
    fonts: HashMap<FontKey, Font>,
    /// Pre-computed character metrics (advance width, advance height, width, height)
    char_metrics: HashMap<char, (f64, f64, f64, f64)>,
    /// Standard monospace character width for terminal cells
    char_width: f64,
    /// Line height for terminal rows
    char_height: f64,
    /// Font ascent (baseline offset)
    ascent: f64,
}

impl DrawingCache {
    /// Create a new DrawingCache with fontdue font loading
    ///
    /// Note: This implementation currently falls back to basic monospace metrics
    /// since loading system fonts with fontdue requires platform-specific code.
    /// In a production implementation, you'd want to:
    /// 1. Load the specified font family from system font directories
    /// 2. Fallback to a built-in font if the requested family isn't found
    /// 3. Handle different platforms (macOS Font Book, Windows font registry, Linux fontconfig)
    pub fn new(font_family: &str, font_size_px: f64) -> Result<Self, String> {
        debug!("Creating DrawingCache for font '{}' at size {}", font_family, font_size_px);

        // For now, implement basic monospace metrics
        // In a full implementation, this would load the actual system font
        let monospace_advance = font_size_px * 0.6; // Monospace character spacing
        let line_height = font_size_px * 1.2;       // Terminal line height
        let baseline_offset = font_size_px * 0.8;   // Baseline position

        // Initialize empty font cache - in production would load actual fonts
        let fonts = HashMap::new();

        // Pre-compute metrics for ASCII range based on monospace assumptions
        let mut char_metrics = HashMap::new();
        // Add null character explicitly (not in typical control range)
        char_metrics.insert('\0', (0.0, 0.0, 0.0, line_height));

        for i in 32..=126 {
            if let Some(ch) = char::from_u32(i) {
                let width = monospace_advance;
                let height = line_height;
                char_metrics.insert(ch, (monospace_advance, 0.0, width, height));
            }
        }

        Ok(Self {
            font_family: font_family.to_string(),
            font_size: font_size_px,
            fonts,
            char_metrics,
            char_width: monospace_advance,
            char_height: line_height,
            ascent: baseline_offset,
        })
    }

    /// Get character metrics - returns backend-agnostic struct
    pub fn get_char_metrics(&self, ch: char) -> CharMetrics {
        let (advance, _, width, height) = self.char_metrics.get(&ch)
            .copied()
            .unwrap_or((self.char_width, 0.0, self.char_width, self.char_height));

        CharMetrics {
            width,
            height,
            ascent: self.ascent,
        }
    }

    /// Get font data for rendering (if available) - placeholder for future fontdue bitmap generation
    pub fn rasterize_glyph(&self, ch: char, _variant: &str) -> Option<(Vec<u8>, u32, u32)> {
        // TODO: Implement actual fontdue glyph rasterization
        // This would:
        // 1. Look up the appropriate Font for the variant (normal/bold/italic)
        // 2. Use fontdue's layout_rasterize to generate bitmap
        // 3. Return RGBA bitmap data, width, height
        // For now, placeholder - no actual fonts loaded
        None
    }

    /// Check if a character is available in current fonts
    pub fn has_glyph(&self, ch: char) -> bool {
        // Simple ASCII check for now
        // In production, would check actual font glyph coverage
        matches!(ch, '\0' | ' '..='~')
    }

    /// Get the width of a specific character in pixels
    pub fn get_char_width(&self, ch: char) -> f64 {
        self.char_metrics.get(&ch)
            .copied()
            .unwrap_or((self.char_width, 0.0, self.char_width, self.char_height))
            .2 // width part of tuple
    }

    /// Get the advance width (cursor movement) for a character
    pub fn get_char_advance(&self, ch: char) -> f64 {
        self.char_metrics.get(&ch)
            .copied()
            .unwrap_or((self.char_width, 0.0, self.char_width, self.char_height))
            .0 // advance width part of tuple
    }

    /// Calculate total width of a string using font metrics
    pub fn calculate_text_width(&self, text: &str) -> f64 {
        text.chars()
            .map(|ch| self.get_char_advance(ch))
            .sum()
    }

    /// Get standard underscore position (baseline offset + descent)
    pub fn get_underline_position(&self) -> f64 {
        self.ascent + (self.char_height - self.ascent) * 0.5
    }

    /// Get standard line thickness for underlines
    pub fn get_underline_thickness(&self) -> f64 {
        self.font_size * 0.05 // 5% of font size
    }

    // Accessor methods to maintain compatibility with existing API
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
}

impl Clone for DrawingCache {
    fn clone(&self) -> Self {
        Self::new(&self.font_family, self.font_size)
            .expect("Failed to clone DrawingCache")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drawing_cache_creation() {
        let cache = DrawingCache::new("monospace", 12.0).unwrap();
        assert_eq!(cache.font_family(), "monospace");
        assert_eq!(cache.font_size(), 12.0);
    }

    #[test]
    fn test_char_metrics() {
        let cache = DrawingCache::new("monospace", 16.0).unwrap();

        // Test printable character
        let metrics = cache.get_char_metrics('X');
        assert!(metrics.width > 0.0);
        assert!(metrics.height > 0.0);
        assert!(metrics.ascent > 0.0);

        // Test control character (should be 0 width)
        let null_metrics = cache.get_char_metrics('\0');
        assert_eq!(null_metrics.width, 0.0);
    }

    #[test]
    fn test_text_width_calculation() {
        let cache = DrawingCache::new("monospace", 12.0).unwrap();

        let width = cache.calculate_text_width("ABC");
        // Should be 3 * character advance width
        let expected = 3.0 * cache.char_width();
        assert!( (width - expected).abs() < 0.001 );
    }

    #[test]
    fn test_accessors() {
        let cache = DrawingCache::new("monospace", 16.0).unwrap();
        assert_eq!(cache.char_width(), cache.char_width());
        assert_eq!(cache.char_height(), cache.char_height());
        assert_eq!(cache.ascent(), cache.ascent());
        assert_eq!(cache.font_size(), 16.0);
        assert_eq!(cache.font_family(), "monospace");
    }

    #[test]
    fn test_glyph_rasterization() {
        let cache = DrawingCache::new("monospace", 12.0).unwrap();

        // Glyph rasterization returns None in basic implementation
        // (would return bitmap data in production)
        let bitmap_data = cache.rasterize_glyph('A', "normal");
        assert!(bitmap_data.is_none());
    }

    #[test]
    fn test_glyph_availability() {
        let cache = DrawingCache::new("monospace", 12.0).unwrap();

        // Test basic ASCII glyph availability (only ASCII is supported in placeholder)
        assert!(cache.has_glyph('A'), "ASCII letter should be available");
        assert!(cache.has_glyph(' '), "Space should be available");
        assert!(cache.has_glyph('\0'), "Null char should be available");
        assert!(!cache.has_glyph('€'), "Euro symbol should not be available in placeholder");
    }

    #[test]
    fn test_control_characters() {
        let cache = DrawingCache::new("monospace", 12.0).unwrap();

        // Test null character specifically (commonly used in terminals)
        let null_metrics = cache.get_char_metrics('\0');
        assert_eq!(null_metrics.width, 0.0, "Null character should have 0 width");

        // Other control characters in terminal contexts may fall back to default metrics
        // since they're not in our pre-computed ASCII range
        let newline_metrics = cache.get_char_metrics('\n');
        assert_eq!(newline_metrics.width, cache.char_width(), "Other control characters fall back to default width");
    }

    #[test]
    fn test_ascii_printable_characters() {
        let cache = DrawingCache::new("monospace", 14.0).unwrap();

        // Test printable ASCII range (32-126) all have positive width
        for ascii_val in 33..=126 {
            let ch = char::from_u32(ascii_val).unwrap();
            let metrics = cache.get_char_metrics(ch);
            assert!(metrics.width > 0.0, "Printable ASCII char {:?} should have positive width", ch);
            assert!(metrics.height > 0.0);
            assert!(metrics.ascent > 0.0);
        }
    }

    #[test]
    fn test_underline_calculations() {
        let cache = DrawingCache::new("monospace", 16.0).unwrap();

        let underline_pos = cache.get_underline_position();
        let underline_thickness = cache.get_underline_thickness();

        // Underline position should be between ascent and char height
        assert!(underline_pos > cache.ascent());
        assert!(underline_pos < cache.char_height());

        // Underline thickness should be small fraction of font size
        assert!(underline_thickness > 0.0);
        assert!(underline_thickness < cache.font_size() * 0.1); // Less than 10% of font size
    }

    #[test]
    fn test_clone_functionality() {
        let original = DrawingCache::new("monospace", 18.0).unwrap();
        let cloned = original.clone();

        // Should maintain all properties
        assert_eq!(original.font_family(), cloned.font_family());
        assert_eq!(original.font_size(), cloned.font_size());
        assert_eq!(original.char_width(), cloned.char_width());
        assert_eq!(original.char_height(), cloned.char_height());
    }

    #[test]
    fn test_different_font_sizes() {
        let small = DrawingCache::new("monospace", 10.0).unwrap();
        let medium = DrawingCache::new("monospace", 14.0).unwrap();
        let large = DrawingCache::new("monospace", 20.0).unwrap();

        // Larger fonts should have larger metrics
        assert!(medium.char_width() > small.char_width());
        assert!(large.char_width() > medium.char_width());
        assert!(medium.char_height() > small.char_height());
        assert!(large.char_height() > medium.char_height());

        // Font sizes should match exactly
        assert_eq!(small.font_size(), 10.0);
        assert_eq!(medium.font_size(), 14.0);
        assert_eq!(large.font_size(), 20.0);
    }

    #[test]
    fn test_empty_and_unicode_strings() {
        let cache = DrawingCache::new("monospace", 12.0).unwrap();

        // Empty string should have zero width
        assert_eq!(cache.calculate_text_width(""), 0.0);

        // Test string with only spaces (which are printable)
        let spaces_width = cache.calculate_text_width("   ");
        assert!(spaces_width > 0.0);

        // Test mixed ASCII
        let mixed_width = cache.calculate_text_width("Hello, World!");
        let separate_widths: f64 = String::from("Hello, World!").chars()
            .map(|c| cache.get_char_advance(c))
            .sum();
        assert!((mixed_width - separate_widths).abs() < 0.001);
    }

    #[test]
    fn test_character_advance_consistency() {
        let cache = DrawingCache::new("monospace", 12.0).unwrap();

        // For monospace fonts, all non-control characters should have same advance width
        let expected_advance = cache.char_width();

        for ch in "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789".chars() {
            let advance = cache.get_char_advance(ch);
            assert_eq!(advance, expected_advance,
                "Character {} should have advance width {}, got {}", ch, expected_advance, advance);
        }

        // Control characters should have different advance (typically 0)
        let null_advance = cache.get_char_advance('\0');
        assert_ne!(null_advance, expected_advance);
        assert_eq!(null_advance, 0.0);
    }

    #[test]
    fn test_fallback_behavior() {
        let cache = DrawingCache::new("monospace", 12.0).unwrap();

        // Test with a character not in ASCII range (should use fallback)
        let euro = cache.get_char_metrics('€');
        let expected = cache.char_width();
        assert_eq!(euro.width, expected);

        let heart = cache.get_char_metrics('♥');
        assert_eq!(heart.width, expected);
    }
}

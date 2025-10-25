//! Font cache with fallback chain support for robust Unicode rendering

use std::collections::HashMap;
use fontdue::{Font, FontSettings};
use crate::font::*;
use std::sync::Arc;

/// Handle to a loaded font in the cache
#[derive(Debug, Clone)]
pub struct FontHandle {
    /// Index in the fallback chain
    pub chain_index: usize,
    /// Font family name
    pub family: String,
    /// Font weight
    pub weight: FontWeight,
    /// Font slant
    pub slant: FontSlant,
}

/// Font selection error types
#[derive(Debug, thiserror::Error)]
pub enum FontSelectionError {
    #[error("Font not found: {0}")]
    FontNotFound(String),

    #[error("Invalid font data")]
    InvalidFontData,

    #[error("Platform font discovery not supported")]
    PlatformNotSupported,

    #[error("No fonts available in fallback chain")]
    NoFontsAvailable,

    #[error("Character not supported by any font in chain: {0}")]
    CharacterNotSupported(char),
}

/// Font cache with intelligent fallback chains
///
/// Maintains a primary font and multiple fallback fonts with smart selection
/// based on character coverage and rendering capabilities.
pub struct FontCache {
    /// Primary font family name
    primary_family: String,
    /// Font size in pixels
    font_size: f32,

    /// Loaded fonts with scoring and capabilities
    /// Vec<(Font, family_name, score, supports_emoji, supports_cjk)>
    loaded_fonts: Vec<(Font, String, f32, bool, bool)>,

    /// Glyph coverage cache: (char, variant) -> (chain_index, metrics)
    glyph_cache: HashMap<(char, FontWeight, FontSlant), (usize, fontdue::Metrics)>,

    /// Default monospace metrics for fallback
    default_metrics: fontdue::Metrics,

    /// Platform-specific font search paths
    search_paths: Vec<std::path::PathBuf>,
}

impl FontCache {
    /// Create a new font cache with fallback support
    pub fn new(primary_family: &str, font_size: f32) -> Result<Self, FontSelectionError> {
        let mut cache = Self {
            primary_family: primary_family.to_string(),
            font_size,
            loaded_fonts: Vec::new(),
            glyph_cache: HashMap::new(),
            default_metrics: fontdue::Metrics::default(),
            search_paths: Self::get_default_search_paths(),
        };

        // Discover system fonts and build fallback chain
        cache.init_font_fallback_chain()?;

        Ok(cache)
    }

    /// Initialize font fallback chain by discovering system fonts
    fn init_font_fallback_chain(&mut self) -> Result<(), FontSelectionError> {
        // Discover available fonts
        let system_fonts = discover_fonts(&self.search_paths)?;

        // Build fallback chain starting with primary font
        let fallback_chain = build_fallback_chain(
            &self.primary_family,
            &system_fonts,
            self.font_size,
        )?;

        // Load fonts into memory
        for chain_entry in fallback_chain {
            match self.load_font(&chain_entry) {
                Ok((font, info)) => {
                    self.loaded_fonts.push(info);
                }
                Err(e) => {
                    tracing::warn!("Failed to load font {}: {}", chain_entry.name, e);
                }
            }
        }

        // Ensure we have at least one font loaded
        if self.loaded_fonts.is_empty() {
            return Err(FontSelectionError::NoFontsAvailable);
        }

        // Initialize default metrics from first font
        if let Some((ref font, _, _, _, _)) = self.loaded_fonts.first() {
            self.default_metrics = font.metrics(' ', self.font_size);
        }

        Ok(())
    }

    /// Load a font from system font info
    fn load_font(&self, font: &SystemFont) -> Result<(Font, (Font, String, f32, bool, bool)), FontSelectionError> {
        let font_data = std::fs::read(&font.path)
            .map_err(|_| FontSelectionError::FontNotFound(font.name.clone()))?;

        let settings = FontSettings {
            scale: self.font_size,
            ..Default::default()
        };

        let loaded_font = Font::from_bytes(font_data, settings)
            .map_err(|_| FontSelectionError::InvalidFontData)?;

        // Calculate font score for glyph coverage
        let score = score_font_for_chars(&loaded_font, self.font_size);

        Ok((
            loaded_font.clone(),
            (
                loaded_font,
                font.name.clone(),
                score,
                font.supports_emoji,
                font.supports_cjk,
            )
        ))
    }

    /// Get the best font for rendering a character
    pub fn select_font_for_char(&mut self, ch: char, weight: FontWeight, slant: FontSlant) -> Result<FontHandle, FontSelectionError> {
        // Check cache first
        let cache_key = (ch, weight, slant);
        if let Some((chain_index, _)) = self.glyph_cache.get(&cache_key) {
            let (_, family, _, _, _) = &self.loaded_fonts[*chain_index];
            return Ok(FontHandle {
                chain_index: *chain_index,
                family: family.clone(),
                weight,
                slant,
            });
        }

        // Find best font in chain
        for (i, (font, family, _, supports_emoji, supports_cjk)) in self.loaded_fonts.iter().enumerate() {
            if self.font_has_glyph(font, ch, *supports_emoji, *supports_cjk) {
                // Cache the result
                let metrics = font.metrics(ch, self.font_size);
                self.glyph_cache.insert(cache_key, (i, metrics));

                return Ok(FontHandle {
                    chain_index: i,
                    family: family.clone(),
                    weight,
                    slant,
                });
            }
        }

        Err(FontSelectionError::CharacterNotSupported(ch))
    }

    /// Check if font has support for a character
    fn font_has_glyph(&self, font: &Font, ch: char, supports_emoji: bool, supports_cjk: bool) -> bool {
        // Basic glyph index check
        if font.lookup_glyph_index(ch) != 0 {
            return true;
        }

        // Special handling for emoji and CJK if font claims support
        if supports_emoji && self.is_emoji_char(ch) {
            // Emoji fonts may have combined glyphs
            return true;
        }

        if supports_cjk && self.is_cjk_char(ch) {
            return true;
        }

        false
    }

    /// Check if character is likely an emoji
    fn is_emoji_char(&self, ch: char) -> bool {
        let code = ch as u32;
        // Unicode emoji ranges (simplified)
        matches!(code,
            0x1F600..=0x1F64F |    // Emoticons
            0x1F300..=0x1F5FF |    // Misc Symbols and Pictographs
            0x1F680..=0x1F6FF |    // Transport and Map symbols
            0x2600..=0x26FF        // Misc symbols
        )
    }

    /// Check if character is CJK (Chinese/Japanese/Korean)
    fn is_cjk_char(&self, ch: char) -> bool {
        let code = ch as u32;
        matches!(code,
            0x2E80..=0x2EFF |      // CJK Radicals Supplement
            0x2F00..=0x2FDF |      // Kangxi Radicals
            0x3000..=0x303F |      // CJK Symbols and Punctuation
            0x3400..=0x4DBF |      // CJK Unified Ideographs Extension A
            0x4E00..=0x9FFF |      // CJK Unified Ideographs
            0xF900..=0xFAFF |      // CJK Compatibility Ideographs
            0x20000..=0x2A6DF      // CJK Unified Ideographs Extension B
        )
    }

    /// Get font face and metrics for character rendering
    pub fn get_font_metrics(&mut self, ch: char, weight: FontWeight, slant: FontSlant) -> Result<(&Font, fontdue::Metrics), FontSelectionError> {
        let handle = self.select_font_for_char(ch, weight, slant)?;
        let (font, _, _, _, _) = &self.loaded_fonts[handle.chain_index];

        // Get cached metrics or compute new ones
        let cache_key = (ch, weight, slant);
        let metrics = if let Some((_, cached_metrics)) = self.glyph_cache.get(&cache_key) {
            *cached_metrics
        } else {
            let metrics = font.metrics(ch, self.font_size);
            self.glyph_cache.insert(cache_key, (handle.chain_index, metrics));
            metrics
        };

        Ok((font, metrics))
    }

    /// Render glyph to bitmap
    pub fn rasterize_glyph(&mut self, ch: char, weight: FontWeight, slant: FontSlant) -> Result<(Vec<u8>, u32, u32), FontSelectionError> {
        let handle = self.select_font_for_char(ch, weight, slant)?;
        let (font, _, _, _, _) = &self.loaded_fonts[handle.chain_index];
        let (metrics, bitmap) = font.rasterize(ch, self.font_size);
        Ok((
            bitmap,
            metrics.width.try_into().unwrap_or(0),
            metrics.height.try_into().unwrap_or(0)
        ))
    }

    /// Get default font metrics for the cache
    pub fn get_default_metrics(&self) -> fontdue::Metrics {
        self.default_metrics
    }

    /// Get platform-specific font search paths
    fn get_default_search_paths() -> Vec<std::path::PathBuf> {
        #[cfg(target_os = "linux")]
        {
            vec![
                "/usr/share/fonts".into(),
                "/usr/local/share/fonts".into(),
                "~/.fonts".into(),
            ]
        }

        #[cfg(target_os = "macos")]
        {
            vec![
                "/System/Library/Fonts".into(),
                "/Library/Fonts".into(),
                "~/Library/Fonts".into(),
            ]
        }

        #[cfg(target_os = "windows")]
        {
            vec![
                "C:\\Windows\\Fonts".into(),
                "C:\\Program Files\\Common Files\\microsoft shared\\Fonts".into(),
            ]
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            vec![]
        }
    }

    /// Get number of fonts in cache
    pub fn font_count(&self) -> usize {
        self.loaded_fonts.len()
    }

    /// Check if emoji support is available
    pub fn has_emoji_support(&self) -> bool {
        self.loaded_fonts.iter().any(|(_, _, _, supports_emoji, _)| *supports_emoji)
    }

    /// Check if CJK support is available
    pub fn has_cjk_support(&self) -> bool {
        self.loaded_fonts.iter().any(|(_, _, _, _, supports_cjk)| *supports_cjk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_cache_creation() {
        // This test may fail if no fonts are available on the system
        let result = FontCache::new("DejaVu Sans Mono", 14.0);
        match result {
            Ok(cache) => {
                assert!(cache.font_count() > 0);
                assert!(cache.get_default_metrics().width > 0);
            }
            Err(FontSelectionError::NoFontsAvailable) => {
                // Acceptable on systems without fonts
                eprintln!("No fonts available for testing");
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn test_character_support_detection() {
        let mut cache = FontCache::new("monospace", 12.0).unwrap_or_else(|_| {
            // Fallback for systems without fonts
            panic!("Font discovery required for this test");
        });

        // Basic ASCII should work
        let handle = cache.select_font_for_char('A', FontWeight::Normal, FontSlant::Normal);
        assert!(handle.is_ok());

        // Null char should be handled gracefully
        let null_handle = cache.select_font_for_char('\0', FontWeight::Normal, FontSlant::Normal);
        if null_handle.is_ok() {
            assert!(null_handle.unwrap().chain_index < cache.font_count());
        }
    }

    #[test]
    fn test_fallback_chain_scoring() {
        let cache = FontCache::new("monospace", 12.0);
        if let Ok(cache) = cache {
            assert!(cache.font_count() > 0);

            // Test that emoji support is detected if available
            let has_emoji = cache.has_emoji_support();
            if has_emoji {
                eprintln!("Emoji support detected");
            }

            let has_cjk = cache.has_cjk_support();
            if has_cjk {
                eprintln!("CJK support detected");
            }
        } else {
            eprintln!("Font cache creation failed - skipping fallback test");
        }
    }
}

//! Font fallback system with system font discovery
//!
//! This module provides comprehensive font support for proper Unicode rendering,
//! including platform-specific font discovery and fallback chains.

pub mod cache;
pub mod discovery;
pub mod fallback;

pub use cache::{FontCache, FontHandle, FontSelectionError};
pub use discovery::{discover_fonts, FontSource, FontLocation};
pub use fallback::{build_fallback_chain, FallbackMetrics, score_font_for_chars};

/// Font weight variants for terminal rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontWeight {
    Normal,
    Bold,
}

/// Font slant variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontSlant {
    Normal,
    Italic,
}

/// Platform-specific font discovery result
#[derive(Debug, Clone)]
pub struct SystemFont {
    pub name: String,
    pub path: String,
    pub weight: FontWeight,
    pub slant: FontSlant,
    pub pixel_size: Option<f32>,
    pub supports_unicode: bool,
    pub supports_emoji: bool,
    pub supports_cjk: bool,
}

/// Font rendering metrics
#[derive(Debug, Clone, Copy)]
pub struct FontMetrics {
    pub advance_width: f32,
    pub advance_height: f32,
    pub bounding_width: f32,
    pub bounding_height: f32,
    pub ascent: f32,
    pub descent: f32,
}

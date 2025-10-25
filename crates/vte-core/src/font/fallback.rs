//! Font fallback chain building and character scoring

use crate::font::*;

/// Builds an optimal font fallback chain for a terminal
///
/// Orders fonts from most to least suitable:
/// 1. Primary family (monospace)
/// 2. Common monospace alternatives
/// 3. Symbolic/emoji fonts
/// 4. Unicode-replete fonts
/// 5. System fallbacks
pub fn build_fallback_chain(
    primary_family: &str,
    system_fonts: &[SystemFont],
    font_size: f32,
) -> Result<Vec<SystemFont>, FontSelectionError> {
    let mut chain = Vec::new();
    let mut used_fonts = std::collections::HashSet::new();

    // Prioritize fonts by suitability score
    let mut scored_fonts: Vec<(f64, &SystemFont)> = system_fonts
        .iter()
        .filter(|font| !used_fonts.contains(&font.name))
        .map(|font| (calculate_font_score(primary_family, font, font_size), font))
        .collect();

    // Sort by score descending (highest score first)
    scored_fonts.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    // Build chain with deduplication
    for (_, font) in scored_fonts {
        if used_fonts.insert(&font.name) {
            chain.push((*font).clone());
        }
    }

    // Ensure at least basic ASCII support
    if chain.is_empty() {
        return Err(FontSelectionError::NoFontsAvailable);
    }

    Ok(chain)
}

/// Calculate suitability score for a font in terminal use
fn calculate_font_score(primary_family: &str, font: &SystemFont, _font_size: f32) -> f64 {
    let mut score = 0.0;

    // Base score for any usable font
    score += 10.0;

    // Primary family gets highest score
    if font.name.to_lowercase().contains(&primary_family.to_lowercase()) {
        score += 1000.0;
    }

    // Monospace fonts preferred for terminal use
    if is_monospace_font(&font.name) {
        score += 500.0;
    }

    // Emoji support bonus
    if font.supports_emoji {
        score += 200.0;
    }

    // CJK support bonus
    if font.supports_cjk {
        score += 150.0;
    }

    // Weight penalties (prefer normal weight for terminals)
    match font.weight {
        FontWeight::Normal => score += 50.0,
        FontWeight::Bold => score += 10.0,
    }

    // Slant penalties (prefer upright fonts)
    match font.slant {
        FontSlant::Normal => score += 50.0,
        FontSlant::Italic => score += 10.0,
    }

    // Known good terminal fonts get bonus
    if is_known_terminal_font(&font.name) {
        score += 100.0;
    }

    score
}

/// Check if font name indicates monospace characteristics
fn is_monospace_font(name: &str) -> bool {
    let name_lower = name.to_lowercase();

    // Common monospace font indicators
    name_lower.contains("mono") ||
    name_lower.contains("typewriter") ||
    name_lower.contains("console") ||
    name_lower.contains("terminal") ||
    name_lower.contains("code") ||
    name_lower.contains("programming") ||
    name_lower.contains("fixed") ||
    // Specific monospace font families
    name_lower.contains("courier") ||
    name_lower.contains("menlo") ||
    name_lower.contains("consolas") ||
    name_lower.contains("inconsolata") ||
    name_lower.contains("source code") ||
    name_lower.contains("firacode") ||
    name_lower.contains("hack") ||
    name_lower.contains("monoid") ||
    name_lower.contains("fira mono") ||
    name_lower.contains("dejavu sans mono")
}

/// Check if font is known to work well in terminals
fn is_known_terminal_font(name: &str) -> bool {
    let name_lower = name.to_lowercase();

    match name_lower.as_str() {
        // Popular monospace fonts
        "dejavu sans mono" |
        "liberation mono" |
        "ubuntu mono" |
        "source code pro" |
        "firacode" |
        "inconsolata" |
        "hack" |
        "courier new" |
        "menlo" |
        "consolas" |
        "lucida console" |
        "terminus" |
        // Emoji/symbol fonts
        "noco color emoji" |
        "joypixels" |
        "apple color emoji" |
        "segoe ui emoji" |
        // CJK fonts
        "wenquanyi micro hei" |
        "droid sans fallback" |
        "noto sans cjk" => true,
        _ => false,
    }
}

pub fn score_font_for_chars(font: &fontdue::Font, _font_size: f32) -> f32 {
    let mut score = 0.0;
    let mut tested_chars = 0;

    // Test ASCII characters (essential)
    for ch in ' '..='~' {
        tested_chars += 1;
        if font.lookup_glyph_index(ch) != 0 {
            score += 1.0;

            // Extra bonus for programming chars
            if "!@#$%^&*()[]{}|\\:;\"'<>,.?/".contains(ch) {
                score += 0.5;
            }
        }
    }

    // Test common Unicode characters (extended ASCII)
    let common_unicode_chars = [
        '©', '®', '™', '€', '£', '¥', '•', '°', '±', '²', '³', '¼', '½', '¾',
        'µ', '¶', '†', '‡', '…', '‰', '‹', '›', '«', '»', '‽', '※',
        '–', '—', '―', '‾', '·', '•', '‣', '◦', '⁃', '⁌', '⁍', '⁎', '⁏',
        '€', '℃', '℉', '∞', '∆', '∇', '∈', '∉', '∋', '∏', '∑', '−', '+',
        '×', '÷', '√', '∝', '∫', '≈', '≠', '≡', '≤', '≥', '⊂', '⊃', '⊆', '⊇',
        '⊥', '∥', '∠', '⌒', '⌘', '⇧', '⇪', '↩', '↖', '↑', '↗', '→', '↘', '↓', '↙', '←',
        '⇞', '⇟', '⇤', '⇥', '⌫', '⌦', '⎋', '⎉', '⏏', '↳',
    ];

    for &ch in &common_unicode_chars {
        tested_chars += 1;
        if font.lookup_glyph_index(ch) != 0 {
            score += 0.5; // Half points for extended chars
        }
    }

    // Normalize by characters tested
    if tested_chars > 0 {
        score / tested_chars as f32 * 100.0
    } else {
        0.0
    }
}

/// Font metrics for fallback chain management
#[derive(Debug, Clone)]
pub struct FallbackMetrics {
    pub family: String,
    pub supports_emoji: bool,
    pub supports_cjk: bool,
    pub monospace: bool,
    pub glyph_coverage_score: f32,
}

impl FallbackMetrics {
    /// Create metrics for a font
    pub fn new(font: &fontdue::Font, family: &str, size: f32) -> Self {
        Self {
            family: family.to_string(),
            supports_emoji: check_emoji_support(font),
            supports_cjk: check_cjk_support(font),
            monospace: check_monospace_property(font),
            glyph_coverage_score: score_font_for_chars(font, size),
        }
    }
}

/// Check if font has emoji support by testing common emoji chars
fn check_emoji_support(font: &fontdue::Font) -> bool {
    let emoji_chars = ['😀', '😂', '🤔', '💖', '👍', '👋', '🎉', '🔥'];
    let covered = emoji_chars.iter()
        .filter(|&&ch| font.lookup_glyph_index(ch) != 0)
        .count();

    covered > emoji_chars.len() / 2 // More than half covered
}

/// Check if font has CJK support by testing common CJK chars
fn check_cjk_support(font: &fontdue::Font) -> bool {
    let cjk_chars = ['中', '文', '日', '本', '한', '국', '語'];
    let covered = cjk_chars.iter()
        .filter(|&&ch| font.lookup_glyph_index(ch) != 0)
        .count();

    covered > cjk_chars.len() / 2 // More than half covered
}

/// Check if font is monospace by comparing character widths
fn check_monospace_property(font: &fontdue::Font) -> bool {
    let test_chars = ['i', 'm', 'w', '1', '8', 'a', 'A', '@'];
    let mut widths = Vec::new();
    let default_size = 12.0; // Use default size for monospace checking

    for ch in test_chars {
        if let Some(metrics) = get_font_metrics_for_char(font, ch, default_size) {
            widths.push(metrics.advance_width);
        }
    }

    if widths.is_empty() {
        return false;
    }

    // Check if all widths are within small tolerance (monospace)
    let first_width = widths[0];
    let tolerance = first_width * 0.05; // 5% tolerance for antialiasing

    widths.iter().all(|&w| (w - first_width).abs() <= tolerance)
}

/// Get metrics for a character (fontdue internal)
fn get_font_metrics_for_char(font: &fontdue::Font, ch: char, font_size: f32) -> Option<fontdue::Metrics> {
    let glyph_index = font.lookup_glyph_index(ch);
    if glyph_index != 0 {
        Some(font.metrics_indexed(glyph_index, font_size))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_chain_building() {
        let system_fonts = vec![
            SystemFont {
                name: "DejaVu Sans Mono".to_string(),
                path: "/usr/share/fonts/dejavu/DejaVuSansMono.ttf".into(),
                weight: FontWeight::Normal,
                slant: FontSlant::Normal,
                pixel_size: Some(12.0),
                supports_unicode: true,
                supports_emoji: false,
                supports_cjk: false,
            },
            SystemFont {
                name: "Noto Color Emoji".to_string(),
                path: "/usr/share/fonts/noto/NotoColorEmoji.ttf".into(),
                weight: FontWeight::Normal,
                slant: FontSlant::Normal,
                pixel_size: Some(12.0),
                supports_unicode: true,
                supports_emoji: true,
                supports_cjk: false,
            },
            SystemFont {
                name: "DejaVu Sans".to_string(),
                path: "/usr/share/fonts/dejavu/DejaVuSans.ttf".into(),
                weight: FontWeight::Normal,
                slant: FontSlant::Normal,
                pixel_size: Some(12.0),
                supports_unicode: true,
                supports_emoji: false,
                supports_cjk: false,
            },
        ];

        let chain = build_fallback_chain("DejaVu Sans Mono", &system_fonts, 12.0);
        assert!(chain.is_ok());

        let fonts = chain.unwrap();
        assert!(fonts.len() >= 1);

        // Primary font should be first (highest score)
        assert_eq!(fonts[0].name, "DejaVu Sans Mono");
    }

    #[test]
    fn test_font_scoring() {
        let font = SystemFont {
            name: "DejaVu Sans Mono".to_string(),
            path: "/usr/share/fonts/dejavu/DejaVuSansMono.ttf".into(),
            weight: FontWeight::Normal,
            slant: FontSlant::Normal,
            pixel_size: Some(12.0),
            supports_unicode: true,
            supports_emoji: false,
            supports_cjk: false,
        };

        // Primary family exact match should get high score
        let score = calculate_font_score("DejaVu Sans Mono", &font, 12.0);
        assert!(score > 1000.0);

        // Different font should get lower score
        let score2 = calculate_font_score("Liberation Mono", &font, 12.0);
        assert!(score2 > 500.0); // Should still get monospace bonus
    }

    #[test]
    fn test_monospace_detection() {
        assert!(is_monospace_font("DejaVu Sans Mono"));
        assert!(is_monospace_font("Liberation Mono"));
        assert!(is_monospace_font("Source Code Pro"));
        assert!(is_monospace_font("Fira Code"));

        assert!(!is_monospace_font("DejaVu Sans"));
        assert!(!is_monospace_font("Arial"));
        assert!(!is_monospace_font("Times New Roman"));
    }

    #[test]
    fn test_known_terminal_font_detection() {
        assert!(is_known_terminal_font("DejaVu Sans Mono"));
        assert!(is_known_terminal_font("Source Code Pro"));
        assert!(is_known_terminal_font("Fira Code"));
        assert!(is_known_terminal_font("Noto Color Emoji"));

        assert!(!is_known_terminal_font("Arial"));
        assert!(!is_known_terminal_font("Unknown Font"));
    }

    #[test]
    fn test_monospace_property_check() {
        // This test would need real font data to be meaningful
        // For now, just ensure the function doesn't panic
        if let Ok(font_data) = std::fs::read("/usr/share/fonts/dejavu/DejaVuSansMono.ttf") {
            if let Ok(font) = fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default()) {
                let is_mono = check_monospace_property(&font);
                // We can't assert true/false without known fonts, but function should work
                let _ = is_mono;
            }
        }
    }
}

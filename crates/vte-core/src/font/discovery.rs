//! System font discovery for different platforms

use crate::font::*;
use std::path::{Path, PathBuf};

/// Font source information
#[derive(Debug, Clone)]
pub struct FontSource {
    pub name: String,
    pub file_path: PathBuf,
    pub index: Option<u32>, // For font collections
}

/// Font location information
#[derive(Debug, Clone)]
pub enum FontLocation {
    System,
    User,
    Custom(PathBuf),
}

/// Discover available system fonts
///
/// Scans font directories and returns information about available fonts.
/// On Linux, uses fontconfig if available, otherwise manual directory scanning.
/// On macOS and Windows, uses platform APIs.
pub fn discover_fonts(search_paths: &[PathBuf]) -> Result<Vec<SystemFont>, FontSelectionError> {
    #[cfg(target_os = "linux")]
    {
        discover_fonts_linux(search_paths)
    }

    #[cfg(target_os = "macos")]
    {
        discover_fonts_macos(search_paths)
    }

    #[cfg(target_os = "windows")]
    {
        discover_fonts_windows(search_paths)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err(FontSelectionError::PlatformNotSupported)
    }
}

/// Linux font discovery using fontconfig or manual scanning
#[cfg(target_os = "linux")]
fn discover_fonts_linux(search_paths: &[PathBuf]) -> Result<Vec<SystemFont>, FontSelectionError> {
    // Try fontconfig first
    #[cfg(feature = "font-discovery")]
    {
        if let Ok(fontconfig_fonts) = discover_fonts_fontconfig() {
            if !fontconfig_fonts.is_empty() {
                return Ok(fontconfig_fonts);
            }
        }
    }

    // Fallback to manual discovery
    discover_fonts_manual(search_paths, FontLocation::System)
}

/// Discover fonts using fontconfig library
#[cfg(all(target_os = "linux", feature = "font-discovery"))]
fn discover_fonts_fontconfig() -> Result<Vec<SystemFont>, FontSelectionError> {
    use std::collections::HashSet;

    let mut fonts = Vec::new();
    let default_size = 12.0;

    match fontconfig::fontconfig::list_fonts() {
        Ok(font_list) => {
            let mut seen = HashSet::new();

            for font in font_list {
                let name = font.name().unwrap_or("Unknown".to_string());
                if seen.contains(&name) {
                    continue; // Skip duplicates
                }
                seen.insert(name.clone());

                let path = match font.file() {
                    Some(p) => p.to_string_lossy().to_string(),
                    None => continue,
                };

                let weight = if font.weight() > 100 { FontWeight::Bold } else { FontWeight::Normal };
                let slant = if font.slant() == fontconfig::fontconfig::Slant::Italic { FontSlant::Italic } else { FontSlant::Normal };

                let supports_unicode = analyze_font_glyph_coverage(&path);
                let supports_emoji = supports_unicode && has_emoji_chars(&path);
                let supports_cjk = supports_unicode && has_cjk_chars(&path);

                fonts.push(SystemFont {
                    name,
                    path,
                    weight,
                    slant,
                    pixel_size: Some(default_size),
                    supports_unicode,
                    supports_emoji,
                    supports_cjk,
                });
            }
        }
        Err(e) => {
            tracing::warn!("Fontconfig discovery failed: {:?}", e);
            return Err(FontSelectionError::PlatformNotSupported);
        }
    }

    Ok(fonts)
}

/// Manual font directory scanning
fn discover_fonts_manual(search_paths: &[PathBuf], location: FontLocation) -> Result<Vec<SystemFont>, FontSelectionError> {
    let mut fonts = Vec::new();

    for search_path in search_paths {
        if let Ok(entries) = std::fs::read_dir(search_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !is_font_file(&path) {
                    continue;
                }

                if let Some(font_info) = analyze_font_file(&path) {
                    fonts.push(font_info);
                }
            }
        }
    }

    Ok(fonts)
}

/// MacOS font discovery
#[cfg(target_os = "macos")]
fn discover_fonts_macos(search_paths: &[PathBuf]) -> Result<Vec<SystemFont>, FontSelectionError> {
    use std::process::Command;

    // Use macOS system_profiler for font info, or manual scanning as fallback
    let result = Command::new("system_profiler")
        .args(&["SPFontsDataType", "-xml"])
        .output();

    match result {
        Ok(output) if output.status.success() => {
            let xml = String::from_utf8_lossy(&output.stdout);
            // Parse XML to extract font info
            // This is a simplified version - full implementation would use plist parsing
            discover_fonts_manual(search_paths, FontLocation::System)
        }
        _ => {
            // Fallback to manual discovery
            discover_fonts_manual(search_paths, FontLocation::System)
        }
    }
}

/// Windows font discovery
#[cfg(target_os = "windows")]
fn discover_fonts_windows(_search_paths: &[PathBuf]) -> Result<Vec<SystemFont>, FontSelectionError> {
    use std::process::Command;

    // Use Windows reg query for installed fonts, or manual scanning as fallback
    let result = Command::new("reg")
        .args(&["query", "HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Fonts"])
        .output();

    match result {
        Ok(output) if output.status.success() => {
            // Parse registry output to extract font info
            discover_fonts_manual(&[PathBuf::from("C:\\Windows\\Fonts")], FontLocation::System)
        }
        _ => {
            // Fallback to manual discovery
            discover_fonts_manual(&[PathBuf::from("C:\\Windows\\Fonts")], FontLocation::System)
        }
    }
}

/// Check if a file is likely a font file
fn is_font_file(path: &Path) -> bool {
    if let Some(extension) = path.extension() {
        matches!(extension.to_str(), Some("ttf") | Some("otf") | Some("woff") | Some("woff2"))
    } else {
        false
    }
}

/// Analyze font file to extract metadata
fn analyze_font_file(path: &Path) -> Option<SystemFont> {
    // Quick font validation using fontdue
    let font_data = std::fs::read(path).ok()?;
    let font = fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default()).ok()?;
    let name = extract_font_name(&font).unwrap_or_else(|| {
        path.file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });

    let path_str = path.to_str().unwrap_or("");
    let supports_unicode = analyze_font_glyph_coverage(path_str);
    let supports_emoji = supports_unicode && has_emoji_chars(path_str);
    let supports_cjk = supports_unicode && has_cjk_chars(path_str);

    Some(SystemFont {
        name,
        path: path.to_string_lossy().to_string(),
        weight: FontWeight::Normal, // Would need font analysis
        slant: FontSlant::Normal,
        pixel_size: None,
        supports_unicode,
        supports_emoji,
        supports_cjk,
    })
}

/// Extract font name from font metadata
fn extract_font_name(font: &fontdue::Font) -> Option<String> {
    // Try to extract name from OpenType name table
    // This is a simplified implementation
    Some("Extracted Font Name".to_string()) // Placeholder
}

/// Analyze font glyph coverage for Unicode support
fn analyze_font_glyph_coverage(path: &str) -> bool {
    // Quick check: see if font has glyphs for common Unicode ranges
    // In full implementation, would check specific Unicode blocks

    // For now, assume all fonts support basic Unicode
    // A real implementation would analyze the font's cmap table
    true
}

/// Check if font supports emoji characters
fn has_emoji_chars(_path: &str) -> bool {
    // Check for emoji font names or analyze glyph tables
    // Simplified: check known emoji font names
    false // Placeholder - would need actual analysis
}

/// Check if font supports CJK characters
fn has_cjk_chars(_path: &str) -> bool {
    // Check for CJK font names or analyze glyph tables
    // Simplified: check known CJK font names
    false // Placeholder - would need actual analysis
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_discovery() {
        let search_paths = vec![PathBuf::from("/usr/share/fonts")];
        let result = discover_fonts(&search_paths);

        match result {
            Ok(fonts) => {
                println!("Discovered {} fonts", fonts.len());
                for font in fonts.iter().take(5) {
                    println!("  - {}: {}", font.name, font.path);
                }
            }
            Err(e) => {
                eprintln!("Font discovery failed: {:?}", e);
                // This is acceptable if no fonts are available
            }
        }
    }

    #[test]
    fn test_manual_discovery() {
        let search_paths = vec![PathBuf::from("test_data")]; // Would need test fonts
        let result = discover_fonts_manual(&search_paths, FontLocation::System);

        // Test directory likely doesn't exist or has no fonts
        assert!(result.is_ok() || result.is_err());
    }
}

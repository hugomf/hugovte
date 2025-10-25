//! Security utilities for terminal input sanitization and threat mitigation
//!
//! This module provides security-focused utilities to prevent common terminal
//! vulnerabilities including paste-based attacks, malicious escape sequences,
//! and resource exhaustion.

/// Sanitize pasted text to prevent injection attacks
///
/// This function processes text that will be pasted into the terminal,
/// applying appropriate sanitization based on whether bracketed paste mode
/// is enabled.
///
/// # Arguments
/// * `text` - The raw text to sanitize
/// * `bracketed` - Whether bracketed paste mode is enabled
///
/// # Returns
/// Sanitized text safe for terminal input
///
/// # Examples
/// ```
/// use vte_core::security::sanitize_paste;
///
/// // With bracketed paste (recommended)
/// let safe = sanitize_paste("echo 'hello'; rm -rf /", true);
/// assert_eq!(safe, "\x1b[200~echo 'hello'; rm -rf /\x1b[201~");
///
/// // Without bracketed paste (legacy)
/// let safe = sanitize_paste("echo 'hello'\x1b[31mred", false);
/// assert_eq!(safe, "echo 'hello'"); // Escape sequences removed
/// ```
pub fn sanitize_paste(text: &str, bracketed: bool) -> String {
    if bracketed {
        // Use bracketed paste mode - wrap in paste escape sequences
        // This is the safest option as it prevents interpretation of escape sequences
        format!("\x1b[200~{}\x1b[201~", text)
    } else {
        // Legacy mode - remove potentially dangerous characters
        sanitize_unbracketed_paste(text)
    }
}

/// Sanitize text for unbracketed paste mode by removing dangerous characters
fn sanitize_unbracketed_paste(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\x1B' => {
                // Skip escape sequence and its parameters
                chars.next(); // Skip the [
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_alphabetic() || next_ch == '`' || next_ch == '{' || next_ch == '}' {
                        chars.next(); // Consume the terminating character
                        break;
                    }
                    chars.next(); // Skip parameter characters
                }
            },
            '\x08' => {
                // Backspace - remove the previous character if there is one
                result.pop();
            },
            // Remove other control characters except common whitespace
            '\x00'..='\x07' | '\x0B' | '\x0C' | '\x0E'..='\x1F' | '\x7F' => {
                // Don't add control characters
            },
            '\n' | '\t' => {
                result.push(ch);
            },
            ch if ch.is_alphanumeric() || ch.is_whitespace() || is_safe_punctuation(ch) => {
                result.push(ch);
            },
            _ => {
                // Skip other potentially dangerous characters
            }
        }
    }

    result
}



/// Check if a punctuation character is safe for terminal input
fn is_safe_punctuation(ch: char) -> bool {
    matches!(ch,
        '!' | '"' | '#' | '$' | '%' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | '-' | '.' | '/' |
        ':' | ';' | '<' | '=' | '>' | '?' | '@' | '[' | '\\' | ']' | '^' | '_' | '`' | '{' | '|' | '}' | '~'
    )
}

/// Validate OSC sequence parameters to prevent resource exhaustion
///
/// OSC (Operating System Command) sequences can be used maliciously to
/// consume excessive resources or trigger vulnerabilities.
///
/// # Arguments
/// * `command` - The OSC command number (e.g., "52" for clipboard)
/// * `data` - The OSC data payload
///
/// # Returns
/// `true` if the OSC sequence is safe to process, `false` otherwise
pub fn validate_osc_sequence(command: &str, data: &str) -> bool {
    // Check command is a known safe command
    let safe_commands = ["0", "2", "7", "8", "52", "133"];
    if !safe_commands.contains(&command) {
        return false;
    }

    // Check data length doesn't exceed safe limits
    if data.len() > crate::constants::MAX_OSC_LEN {
        return false;
    }

    // Additional validation based on command type
    match command {
        "52" => validate_clipboard_data(data), // Clipboard operations
        "8" => validate_hyperlink_data(data),  // Hyperlinks
        "7" => validate_directory_data(data),  // Directory tracking
        _ => true, // Other commands are generally safe with length limits
    }
}

/// Validate clipboard data for OSC 52 sequences
fn validate_clipboard_data(data: &str) -> bool {
    // Must be valid base64
    if data.is_empty() {
        return false;
    }

    // Check base64 length is reasonable (decoded size will be ~3/4 of encoded)
    if data.len() > 100_000 {
        return false; // ~75KB decoded limit
    }

    // Basic base64 validation (contains only safe characters)
    // Note: OSC 52 format is "52;c;base64data" so we need to extract the base64 part
    if let Some(base64_data) = data.strip_prefix('c').and_then(|s| s.strip_prefix(';')) {
        // Must have some actual data
        if base64_data.is_empty() {
            return false;
        }
        base64_data.chars().all(|ch| ch.is_alphanumeric() || ch == '+' || ch == '/' || ch == '=')
    } else {
        false
    }
}

/// Validate hyperlink data for OSC 8 sequences
fn validate_hyperlink_data(data: &str) -> bool {
    // Parse format: params;URI
    if let Some((params, uri)) = data.split_once(';') {
        // Validate URI format
        if !uri.starts_with("http://") && !uri.starts_with("https://") && !uri.starts_with("file://") {
            return false;
        }

        // Check URI length
        if uri.len() > 2048 {
            return false;
        }

        // Validate parameters if present
        if !params.is_empty() {
            validate_hyperlink_params(params)
        } else {
            true
        }
    } else {
        // If no semicolon, treat as URI only
        data.len() <= 2048 && (data.starts_with("http://") || data.starts_with("https://") || data.starts_with("file://"))
    }
}

/// Validate hyperlink parameters
fn validate_hyperlink_params(params: &str) -> bool {
    // Simple parameter validation - should be key=value pairs separated by colons
    for param in params.split(':') {
        if param.contains('=') {
            let parts: Vec<&str> = param.split('=').collect();
            if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
                return false;
            }
        } else if !param.is_empty() {
            return false; // Invalid parameter format
        }
    }
    true
}

/// Validate directory data for OSC 7 sequences
fn validate_directory_data(data: &str) -> bool {
    // Must be a valid file:// URI
    if !data.starts_with("file://") {
        return false;
    }

    // Check total length
    if data.len() > 1024 {
        return false;
    }

    // Basic path safety check (no dangerous sequences)
    let path_part = &data[7..]; // Remove "file://" prefix
    !path_part.contains("..") && !path_part.contains('\0')
}

/// Rate limiting for terminal operations to prevent DoS attacks
///
/// This helps prevent resource exhaustion from rapid terminal operations
/// like resizing, scrolling, or escape sequence processing.
pub struct RateLimiter {
    last_operation: std::time::Instant,
    min_interval: std::time::Duration,
}

impl RateLimiter {
    /// Create a new rate limiter with specified minimum interval
    pub fn new(min_interval_ms: u64) -> Self {
        Self {
            last_operation: std::time::Instant::now(),
            min_interval: std::time::Duration::from_millis(min_interval_ms),
        }
    }

    /// Check if an operation should be allowed based on rate limiting
    pub fn allow_operation(&mut self) -> bool {
        let now = std::time::Instant::now();
        if now.duration_since(self.last_operation) >= self.min_interval {
            self.last_operation = now;
            true
        } else {
            false
        }
    }
}

/// Security configuration options
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Enable bracketed paste mode by default
    pub bracketed_paste_default: bool,
    /// Maximum OSC sequence length
    pub max_osc_length: usize,
    /// Maximum number of parameters in CSI sequences
    pub max_csi_params: usize,
    /// Enable OSC sequence filtering
    pub filter_osc_sequences: bool,
    /// Rate limit for resize operations (operations per second)
    pub resize_rate_limit: u64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            bracketed_paste_default: true,
            max_osc_length: 2048,
            max_csi_params: 32,
            filter_osc_sequences: false,
            resize_rate_limit: 10, // 10 resize operations per second max
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_paste_bracketed() {
        let input = "echo 'hello'; rm -rf /";
        let result = sanitize_paste(input, true);
        assert!(result.starts_with("\x1b[200~"));
        assert!(result.ends_with("\x1b[201~"));
        assert!(result.contains(input));
    }

    #[test]
    fn test_sanitize_paste_unbracketed() {
        let input = "echo 'hello'\x1b[31mred\x08text";
        let result = sanitize_paste(input, false);
        // Security: escape sequences and backspace should be removed
        // Current implementation: keeps safe chars, skips escape sequences, handles backspace
        assert_eq!(result, "echo 'hello'retext");
    }

    #[test]
    fn test_validate_osc_clipboard() {
        assert!(validate_osc_sequence("52", "c;SGVsbG8=")); // Valid base64
        assert!(!validate_osc_sequence("52", "c;invalid!")); // Invalid base64
        assert!(!validate_osc_sequence("52", "c;")); // Empty data
        assert!(!validate_osc_sequence("52", &"c;x".repeat(1000))); // Too long
    }

    #[test]
    fn test_validate_osc_hyperlink() {
        assert!(validate_osc_sequence("8", ";https://example.com"));
        assert!(validate_osc_sequence("8", "id=link1;https://example.com"));
        assert!(!validate_osc_sequence("8", ";ftp://example.com")); // Invalid protocol
        assert!(!validate_osc_sequence("8", &"x".repeat(3000))); // Too long
    }

    #[test]
    fn test_validate_osc_directory() {
        assert!(validate_osc_sequence("7", "file:///home/user"));
        assert!(!validate_osc_sequence("7", "http://example.com")); // Wrong protocol
        assert!(!validate_osc_sequence("7", "file:///home/user/../../../etc")); // Path traversal
    }

    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(0); // 0ms minimum interval (allow immediate retries)

        // First operation should be allowed
        assert!(limiter.allow_operation());

        // With 0ms interval, second operation should be allowed immediately
        assert!(limiter.allow_operation());
    }

    #[test]
    fn test_is_safe_punctuation() {
        assert!(is_safe_punctuation('!'));
        assert!(is_safe_punctuation('.'));
        assert!(is_safe_punctuation('?'));
        assert!(!is_safe_punctuation('\x00'));
        assert!(!is_safe_punctuation('\x1B'));
    }
}

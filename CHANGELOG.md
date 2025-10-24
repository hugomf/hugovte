# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial release preparation

## [0.1.0] - 2025-10-24

### Added
- **vte-ansi crate**: Fast, memory-safe ANSI/VT escape sequence parser
  - Support for comprehensive VT100/ANSI escape sequences
  - Zero-copy UTF-8-safe parsing with robust error handling
  - Rich color support: 16/256-color palettes and truecolor RGB
  - Full terminal grid simulation without external dependencies
  - Optimized performance using SIMD-enabled search (memchr)
  - No-panic guarantees with graceful error recovery

### Features
- **ANSI Sequence Support**:
  - Text attributes and colors (SGR sequences)
  - Cursor positioning and movement
  - Screen clearing and scrolling
  - Character/line insert/delete operations
  - Alternate screen buffer management
  - Keyboard and mouse mode settings (private DEC modes)
  - Title setting and OSC sequences
  - Hyperlink support

- **Performance Optimizations**:
  - Fast-path for plain text using memchr
  - Efficient parameter parsing
  - Streaming-friendly incremental processing
  - Comprehensive benchmark suite (>30 benchmarks)

- **Quality Assurance**:
  - Full test coverage (72 unit + 15 integration tests)
  - Fuzz testing with overflowing input protection
  - Extensive examples with real-world usage patterns
  - Zero unsafe code blocks

- **Documentation**:
  - Complete API documentation with examples
  - Running examples guide with 3 different use cases:
    - Terminal grid emulation
    - ANSI color extraction
    - Streaming data processing

### Changed
- None (initial release)

### Deprecated
- None

### Removed
- Development artifacts removed for clean publication

### Fixed
- None (initial release)

### Security
- Memory-safe parsing with bounds checking
- No panic on malformed input
- UTF-8 safety guaranteed

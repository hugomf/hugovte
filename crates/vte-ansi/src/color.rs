/// Color in 0.0..=1.0 space with alpha channel
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Default for Color {
    fn default() -> Self {
        Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "rgba({:.2}, {:.2}, {:.2}, {:.2})",
            self.r, self.g, self.b, self.a
        )
    }
}

impl Color {
    pub fn rgba(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }
    pub fn rgb(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b, a: 1.0 }
    }
}

// 16-color ANSI palette
pub const COLOR_PALETTE: [Color; 16] = [
    // Basic 8 colors
    Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },      // Black
    Color { r: 0.8, g: 0.0, b: 0.0, a: 1.0 },      // Red
    Color { r: 0.0, g: 0.8, b: 0.0, a: 1.0 },      // Green
    Color { r: 0.8, g: 0.8, b: 0.0, a: 1.0 },      // Yellow
    Color { r: 0.0, g: 0.0, b: 0.8, a: 1.0 },      // Blue
    Color { r: 0.8, g: 0.0, b: 0.8, a: 1.0 },      // Magenta
    Color { r: 0.0, g: 0.8, b: 0.8, a: 1.0 },      // Cyan
    Color { r: 0.8, g: 0.8, b: 0.8, a: 1.0 },      // White
    // Bright colors
    Color { r: 0.4, g: 0.4, b: 0.4, a: 1.0 },      // Bright Black (Gray)
    Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 },      // Bright Red
    Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 },      // Bright Green
    Color { r: 1.0, g: 1.0, b: 0.0, a: 1.0 },      // Bright Yellow
    Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 },      // Bright Blue
    Color { r: 1.0, g: 0.0, b: 1.0, a: 1.0 },      // Bright Magenta
    Color { r: 0.0, g: 1.0, b: 1.0, a: 1.0 },      // Bright Cyan
    Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 },      // Bright White
];

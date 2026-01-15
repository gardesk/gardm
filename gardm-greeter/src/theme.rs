//! Theme and accessibility configuration
//!
//! Supports color customization and accessibility options.

use serde::Deserialize;

/// RGBA color (0.0-1.0 range)
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub fn rgb(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub fn rgba(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    /// Parse color from hex (#RRGGBB or #RRGGBBAA) or rgba(r, g, b, a) format
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();

        // Try hex format
        if s.starts_with('#') {
            return Self::parse_hex(s);
        }

        // Try rgba() format
        if s.starts_with("rgba(") && s.ends_with(')') {
            return Self::parse_rgba(s);
        }

        // Try rgb() format
        if s.starts_with("rgb(") && s.ends_with(')') {
            return Self::parse_rgb(s);
        }

        None
    }

    fn parse_hex(s: &str) -> Option<Self> {
        let s = s.trim_start_matches('#');

        match s.len() {
            6 => {
                let r = u8::from_str_radix(&s[0..2], 16).ok()?;
                let g = u8::from_str_radix(&s[2..4], 16).ok()?;
                let b = u8::from_str_radix(&s[4..6], 16).ok()?;
                Some(Self::rgb(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0))
            }
            8 => {
                let r = u8::from_str_radix(&s[0..2], 16).ok()?;
                let g = u8::from_str_radix(&s[2..4], 16).ok()?;
                let b = u8::from_str_radix(&s[4..6], 16).ok()?;
                let a = u8::from_str_radix(&s[6..8], 16).ok()?;
                Some(Self::rgba(
                    r as f64 / 255.0,
                    g as f64 / 255.0,
                    b as f64 / 255.0,
                    a as f64 / 255.0,
                ))
            }
            _ => None,
        }
    }

    fn parse_rgba(s: &str) -> Option<Self> {
        let inner = s.trim_start_matches("rgba(").trim_end_matches(')');
        let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
        if parts.len() != 4 {
            return None;
        }

        let r: f64 = parts[0].parse().ok()?;
        let g: f64 = parts[1].parse().ok()?;
        let b: f64 = parts[2].parse().ok()?;
        let a: f64 = parts[3].parse().ok()?;

        // Support both 0-255 and 0-1 ranges for rgb components
        let (r, g, b) = if r > 1.0 || g > 1.0 || b > 1.0 {
            (r / 255.0, g / 255.0, b / 255.0)
        } else {
            (r, g, b)
        };

        Some(Self::rgba(r, g, b, a))
    }

    fn parse_rgb(s: &str) -> Option<Self> {
        let inner = s.trim_start_matches("rgb(").trim_end_matches(')');
        let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
        if parts.len() != 3 {
            return None;
        }

        let r: f64 = parts[0].parse().ok()?;
        let g: f64 = parts[1].parse().ok()?;
        let b: f64 = parts[2].parse().ok()?;

        // Support both 0-255 and 0-1 ranges
        let (r, g, b) = if r > 1.0 || g > 1.0 || b > 1.0 {
            (r / 255.0, g / 255.0, b / 255.0)
        } else {
            (r, g, b)
        };

        Some(Self::rgb(r, g, b))
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::rgb(1.0, 1.0, 1.0)
    }
}

/// Theme configuration for visual customization
#[derive(Debug, Clone)]
pub struct Theme {
    // Panel/background colors
    pub panel_background: Color,
    pub background_overlay: Color,

    // Text colors
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_error: Color,
    pub text_info: Color,

    // Accent colors
    pub accent: Color,
    pub accent_hover: Color,

    // Input field colors
    pub input_background: Color,
    pub input_background_focused: Color,
    pub input_border: Color,

    // Button colors
    pub button_background: Color,
    pub button_background_disabled: Color,

    // Typography
    pub font_family: String,
    pub font_size_normal: i32,
    pub font_size_large: i32,
    pub font_size_title: i32,

    // Layout
    pub corner_radius: f64,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            panel_background: Color::rgba(0.1, 0.1, 0.1, 0.85),
            background_overlay: Color::rgba(0.0, 0.0, 0.0, 0.0),

            text_primary: Color::rgb(1.0, 1.0, 1.0),
            text_secondary: Color::rgba(0.8, 0.8, 0.8, 1.0),
            text_error: Color::rgb(1.0, 0.3, 0.3),
            text_info: Color::rgba(0.7, 0.7, 0.7, 1.0),

            accent: Color::rgb(0.2, 0.5, 0.8),
            accent_hover: Color::rgb(0.3, 0.6, 0.9),

            input_background: Color::rgba(0.25, 0.25, 0.25, 1.0),
            input_background_focused: Color::rgba(0.2, 0.4, 0.6, 1.0),
            input_border: Color::rgb(0.3, 0.6, 0.9),

            button_background: Color::rgba(0.2, 0.5, 0.8, 1.0),
            button_background_disabled: Color::rgba(0.3, 0.3, 0.3, 1.0),

            font_family: "Sans".to_string(),
            font_size_normal: 14,
            font_size_large: 18,
            font_size_title: 24,

            corner_radius: 16.0,
        }
    }
}

impl Theme {
    /// Create a high-contrast theme variant
    pub fn high_contrast() -> Self {
        Self {
            panel_background: Color::rgba(0.0, 0.0, 0.0, 0.95),
            background_overlay: Color::rgba(0.0, 0.0, 0.0, 0.5),

            text_primary: Color::rgb(1.0, 1.0, 1.0),
            text_secondary: Color::rgb(1.0, 1.0, 0.0), // Yellow for visibility
            text_error: Color::rgb(1.0, 0.2, 0.2),
            text_info: Color::rgb(0.2, 1.0, 0.2),

            accent: Color::rgb(0.0, 0.8, 1.0),      // Bright cyan
            accent_hover: Color::rgb(0.2, 1.0, 1.0),

            input_background: Color::rgba(0.0, 0.0, 0.0, 1.0),
            input_background_focused: Color::rgba(0.0, 0.2, 0.4, 1.0),
            input_border: Color::rgb(1.0, 1.0, 0.0), // Yellow border

            button_background: Color::rgba(0.0, 0.6, 0.8, 1.0),
            button_background_disabled: Color::rgba(0.3, 0.3, 0.3, 1.0),

            font_family: "Sans".to_string(),
            font_size_normal: 16, // Slightly larger
            font_size_large: 20,
            font_size_title: 28,

            corner_radius: 8.0,
        }
    }

    /// Apply large text accessibility option
    pub fn with_large_text(mut self) -> Self {
        self.font_size_normal = (self.font_size_normal as f64 * 1.25) as i32;
        self.font_size_large = (self.font_size_large as f64 * 1.25) as i32;
        self.font_size_title = (self.font_size_title as f64 * 1.25) as i32;
        self
    }
}

/// Accessibility configuration
#[derive(Debug, Clone, Deserialize)]
pub struct AccessibilityConfig {
    /// Enable high contrast mode
    #[serde(default)]
    pub high_contrast: bool,

    /// Enable larger text
    #[serde(default)]
    pub large_text: bool,

    /// Disable fade transitions
    #[serde(default)]
    pub reduce_motion: bool,
}

impl Default for AccessibilityConfig {
    fn default() -> Self {
        Self {
            high_contrast: false,
            large_text: false,
            reduce_motion: false,
        }
    }
}

/// Raw theme config for deserialization
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ThemeConfig {
    #[serde(default)]
    pub panel_background: Option<String>,
    #[serde(default)]
    pub background_overlay: Option<String>,
    #[serde(default)]
    pub text_primary: Option<String>,
    #[serde(default)]
    pub text_secondary: Option<String>,
    #[serde(default)]
    pub text_error: Option<String>,
    #[serde(default)]
    pub text_info: Option<String>,
    #[serde(default)]
    pub accent: Option<String>,
    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default)]
    pub font_size_normal: Option<i32>,
    #[serde(default)]
    pub font_size_large: Option<i32>,
    #[serde(default)]
    pub font_size_title: Option<i32>,
    #[serde(default)]
    pub corner_radius: Option<f64>,
}

impl ThemeConfig {
    /// Convert to Theme, applying parsed colors over defaults
    pub fn into_theme(self, accessibility: &AccessibilityConfig) -> Theme {
        let mut base = if accessibility.high_contrast {
            Theme::high_contrast()
        } else {
            Theme::default()
        };

        // Apply custom colors
        if let Some(ref s) = self.panel_background {
            if let Some(c) = Color::parse(s) {
                base.panel_background = c;
            }
        }
        if let Some(ref s) = self.background_overlay {
            if let Some(c) = Color::parse(s) {
                base.background_overlay = c;
            }
        }
        if let Some(ref s) = self.text_primary {
            if let Some(c) = Color::parse(s) {
                base.text_primary = c;
            }
        }
        if let Some(ref s) = self.text_secondary {
            if let Some(c) = Color::parse(s) {
                base.text_secondary = c;
            }
        }
        if let Some(ref s) = self.text_error {
            if let Some(c) = Color::parse(s) {
                base.text_error = c;
            }
        }
        if let Some(ref s) = self.text_info {
            if let Some(c) = Color::parse(s) {
                base.text_info = c;
            }
        }
        if let Some(ref s) = self.accent {
            if let Some(c) = Color::parse(s) {
                base.accent = c;
                // Derive hover color (slightly brighter)
                base.accent_hover = Color::rgba(
                    (c.r + 0.1).min(1.0),
                    (c.g + 0.1).min(1.0),
                    (c.b + 0.1).min(1.0),
                    c.a,
                );
                base.input_background_focused = Color::rgba(c.r * 0.5, c.g * 0.5, c.b * 0.5, 1.0);
                base.input_border = c;
                base.button_background = c;
            }
        }

        // Typography
        if let Some(font) = self.font_family {
            base.font_family = font;
        }
        if let Some(size) = self.font_size_normal {
            base.font_size_normal = size;
        }
        if let Some(size) = self.font_size_large {
            base.font_size_large = size;
        }
        if let Some(size) = self.font_size_title {
            base.font_size_title = size;
        }

        // Layout
        if let Some(radius) = self.corner_radius {
            base.corner_radius = radius;
        }

        // Apply accessibility modifiers
        if accessibility.large_text {
            base = base.with_large_text();
        }

        base
    }
}

//! Greeter configuration
//!
//! Loads visual settings and garbg integration options.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

/// Main greeter configuration
#[derive(Debug, Deserialize, Default)]
pub struct GreeterConfig {
    #[serde(default)]
    pub visual: VisualConfig,
    #[serde(default)]
    pub garbg: GarbgIntegration,
}

/// Visual appearance settings
#[derive(Debug, Deserialize)]
pub struct VisualConfig {
    /// Blur radius for background
    #[serde(default = "default_blur_radius")]
    pub blur_radius: f32,

    /// Background brightness (0.0-1.0, lower = darker)
    #[serde(default = "default_brightness")]
    pub brightness: f32,

    /// Corner radius for UI elements
    #[serde(default = "default_corner_radius")]
    pub corner_radius: f64,

    /// Fade-out duration in milliseconds when starting session
    #[serde(default = "default_fade_duration")]
    pub fade_duration_ms: u64,
}

impl Default for VisualConfig {
    fn default() -> Self {
        Self {
            blur_radius: default_blur_radius(),
            brightness: default_brightness(),
            corner_radius: default_corner_radius(),
            fade_duration_ms: default_fade_duration(),
        }
    }
}

/// garbg integration settings
#[derive(Debug, Deserialize)]
pub struct GarbgIntegration {
    /// Whether to use garbg wallpaper
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Fallback wallpaper if garbg not configured
    #[serde(default = "default_fallback")]
    pub fallback: String,
}

impl Default for GarbgIntegration {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            fallback: default_fallback(),
        }
    }
}

fn default_blur_radius() -> f32 {
    25.0
}
fn default_brightness() -> f32 {
    0.6
}
fn default_corner_radius() -> f64 {
    16.0
}
fn default_fade_duration() -> u64 {
    200
}
fn default_true() -> bool {
    true
}
fn default_fallback() -> String {
    "/usr/share/gardm/backgrounds/default.jpg".to_string()
}

impl GreeterConfig {
    /// Load configuration from file or use defaults
    pub fn load() -> Result<Self> {
        // Try system config location
        let config_paths = [
            PathBuf::from("/etc/gardm/greeter.toml"),
            PathBuf::from("/usr/share/gardm/greeter.toml"),
        ];

        for path in &config_paths {
            if path.exists() {
                let content = std::fs::read_to_string(path)
                    .with_context(|| format!("Failed to read config from {:?}", path))?;
                let config: GreeterConfig = toml::from_str(&content)
                    .with_context(|| format!("Failed to parse config from {:?}", path))?;
                tracing::info!("Loaded greeter config from {:?}", path);
                return Ok(config);
            }
        }

        // Use defaults
        tracing::debug!("No config file found, using defaults");
        Ok(Self::default())
    }
}

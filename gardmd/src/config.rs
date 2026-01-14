//! Configuration for gardmd

use serde::Deserialize;
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub greeter: GreeterConfig,
    #[serde(default)]
    pub security: SecurityConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            greeter: GreeterConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

/// General daemon settings
#[derive(Debug, Clone, Deserialize)]
pub struct GeneralConfig {
    /// Default session if user hasn't selected one
    #[serde(default = "default_session")]
    pub default_session: String,

    /// Path to greeter executable
    #[serde(default = "default_greeter")]
    pub greeter: PathBuf,

    /// VT to use (0 = auto-select)
    #[serde(default)]
    pub vt: u32,

    /// X11 display to use
    #[serde(default = "default_display")]
    pub display: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            default_session: default_session(),
            greeter: default_greeter(),
            vt: 0,
            display: default_display(),
        }
    }
}

/// Greeter visual settings
#[derive(Debug, Clone, Deserialize)]
pub struct GreeterConfig {
    /// Blur radius for background
    #[serde(default = "default_blur_radius")]
    pub blur_radius: u32,

    /// Background brightness (0.0-1.0)
    #[serde(default = "default_blur_brightness")]
    pub blur_brightness: f32,

    /// Show power buttons
    #[serde(default = "default_true")]
    pub show_power_buttons: bool,

    /// Show session selector
    #[serde(default = "default_true")]
    pub show_session_selector: bool,

    /// Use garbg wallpaper
    #[serde(default = "default_true")]
    pub use_garbg_wallpaper: bool,

    /// Fallback wallpaper path
    #[serde(default = "default_fallback_wallpaper")]
    pub fallback_wallpaper: PathBuf,
}

impl Default for GreeterConfig {
    fn default() -> Self {
        Self {
            blur_radius: default_blur_radius(),
            blur_brightness: default_blur_brightness(),
            show_power_buttons: true,
            show_session_selector: true,
            use_garbg_wallpaper: true,
            fallback_wallpaper: default_fallback_wallpaper(),
        }
    }
}

/// Security settings
#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
    /// Allow empty passwords
    #[serde(default)]
    pub allow_empty_password: bool,

    /// Lock after N failed attempts (0 = disabled)
    #[serde(default = "default_lockout_attempts")]
    pub lockout_attempts: u32,

    /// Lockout duration in seconds
    #[serde(default = "default_lockout_duration")]
    pub lockout_duration: u64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            allow_empty_password: false,
            lockout_attempts: default_lockout_attempts(),
            lockout_duration: default_lockout_duration(),
        }
    }
}

// Default value functions
fn default_session() -> String { "gar".to_string() }
fn default_greeter() -> PathBuf { PathBuf::from("/usr/bin/gardm-greeter") }
fn default_display() -> String { ":0".to_string() }
fn default_blur_radius() -> u32 { 20 }
fn default_blur_brightness() -> f32 { 0.7 }
fn default_true() -> bool { true }
fn default_fallback_wallpaper() -> PathBuf {
    PathBuf::from("/usr/share/gardm/backgrounds/default.jpg")
}
fn default_lockout_attempts() -> u32 { 5 }
fn default_lockout_duration() -> u64 { 300 }

impl Config {
    /// Load configuration from default path
    pub fn load() -> anyhow::Result<Self> {
        Self::load_from("/etc/gardm/config.toml")
    }

    /// Load configuration from specified path
    pub fn load_from(path: &str) -> anyhow::Result<Self> {
        let path = PathBuf::from(path);
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(toml::from_str(&content)?)
        } else {
            tracing::info!("Config file not found at {}, using defaults", path.display());
            Ok(Config::default())
        }
    }
}

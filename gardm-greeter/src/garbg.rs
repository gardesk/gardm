//! garbg integration for wallpaper resolution
//!
//! Reads garbg configuration and playlist state to determine which wallpaper
//! to display, creating a seamless transition from greeter to desktop.

use nix::unistd::User;
use serde::Deserialize;
use std::path::PathBuf;

/// garbg configuration file structure
#[derive(Debug, Deserialize, Default)]
pub struct GarbgConfig {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub default: DefaultConfig,
}

#[derive(Debug, Deserialize, Default)]
pub struct GeneralConfig {
    #[serde(default = "default_mode")]
    pub mode: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct DefaultConfig {
    #[serde(default)]
    pub source: String,
}

fn default_mode() -> String {
    "fill".to_string()
}

impl GarbgConfig {
    /// Load garbg config from user or system location
    pub fn load(username: Option<&str>) -> Option<Self> {
        // Try user config first (if we know the username)
        if let Some(user) = username {
            if let Some(home) = get_user_home(user) {
                let user_config = home.join(".config/garbg/config.toml");
                if let Ok(content) = std::fs::read_to_string(&user_config) {
                    if let Ok(config) = toml::from_str(&content) {
                        tracing::debug!("Loaded garbg config from {:?}", user_config);
                        return Some(config);
                    }
                }
            }
        }

        // Try system-wide default
        let system_config = PathBuf::from("/etc/garbg/config.toml");
        if let Ok(content) = std::fs::read_to_string(&system_config) {
            if let Ok(config) = toml::from_str(&content) {
                tracing::debug!("Loaded garbg config from {:?}", system_config);
                return Some(config);
            }
        }

        None
    }

    /// Get the default wallpaper path from config
    pub fn default_wallpaper(&self) -> Option<String> {
        if self.default.source.is_empty() {
            None
        } else {
            Some(expand_path(&self.default.source))
        }
    }
}

/// garbg playlist state (runtime state file)
#[derive(Debug, Deserialize)]
pub struct PlaylistState {
    pub source: String,
    pub images: Vec<String>,
    pub current_index: usize,
    #[serde(default)]
    pub shuffled: bool,
}

impl PlaylistState {
    /// Load current playlist state from runtime directory
    pub fn load(username: Option<&str>) -> Option<Self> {
        // Try XDG_RUNTIME_DIR first
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            let state_file = PathBuf::from(&runtime_dir).join("garbg-state.json");
            if let Ok(content) = std::fs::read_to_string(&state_file) {
                if let Ok(state) = serde_json::from_str(&content) {
                    tracing::debug!("Loaded playlist state from {:?}", state_file);
                    return Some(state);
                }
            }
        }

        // Try user-specific runtime directory
        if let Some(user) = username {
            if let Some(uid) = get_user_uid(user) {
                let user_runtime = format!("/run/user/{}/garbg-state.json", uid);
                if let Ok(content) = std::fs::read_to_string(&user_runtime) {
                    if let Ok(state) = serde_json::from_str(&content) {
                        tracing::debug!("Loaded playlist state from {}", user_runtime);
                        return Some(state);
                    }
                }
            }
        }

        None
    }

    /// Get the current wallpaper path
    pub fn current_wallpaper(&self) -> Option<&str> {
        self.images.get(self.current_index).map(|s| s.as_str())
    }
}

/// Resolves which wallpaper to use for the greeter
pub struct WallpaperResolver {
    fallback_path: String,
}

impl WallpaperResolver {
    pub fn new(fallback: &str) -> Self {
        Self {
            fallback_path: fallback.to_string(),
        }
    }

    /// Resolve wallpaper path, trying multiple sources in priority order
    pub fn resolve(&self, username: Option<&str>) -> String {
        // Priority 1: Current playlist state (what user was last seeing)
        if let Some(state) = PlaylistState::load(username) {
            if let Some(current) = state.current_wallpaper() {
                let expanded = expand_path(current);
                if std::path::Path::new(&expanded).exists() {
                    tracing::info!("Using wallpaper from playlist: {}", expanded);
                    return expanded;
                }
            }
        }

        // Priority 2: garbg config default
        if let Some(config) = GarbgConfig::load(username) {
            if let Some(default) = config.default_wallpaper() {
                let path = std::path::Path::new(&default);
                if path.is_dir() {
                    // If it's a directory, pick first image
                    if let Some(first) = first_image_in_dir(&default) {
                        tracing::info!("Using first image from garbg source dir: {}", first);
                        return first;
                    }
                } else if path.exists() {
                    tracing::info!("Using wallpaper from garbg config: {}", default);
                    return default;
                }
            }
        }

        // Priority 3: Fallback
        tracing::info!("Using fallback wallpaper: {}", self.fallback_path);
        self.fallback_path.clone()
    }
}

/// Get user's home directory
fn get_user_home(username: &str) -> Option<PathBuf> {
    User::from_name(username)
        .ok()?
        .map(|u| PathBuf::from(u.dir.to_string_lossy().to_string()))
}

/// Get user's UID
fn get_user_uid(username: &str) -> Option<u32> {
    User::from_name(username).ok()?.map(|u| u.uid.as_raw())
}

/// Expand ~ and environment variables in path
fn expand_path(path: &str) -> String {
    shellexpand::full(path)
        .map(|s| s.to_string())
        .unwrap_or_else(|_| path.to_string())
}

/// Get first image file in a directory (sorted alphabetically)
fn first_image_in_dir(dir: &str) -> Option<String> {
    const EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "bmp"];

    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            if let Some(ext) = path.extension() {
                EXTENSIONS.contains(&ext.to_string_lossy().to_lowercase().as_str())
            } else {
                false
            }
        })
        .collect();

    entries.sort_by_key(|e| e.path());
    entries
        .first()
        .map(|e| e.path().to_string_lossy().to_string())
}

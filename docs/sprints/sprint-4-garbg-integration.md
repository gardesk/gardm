# Sprint 4: garbg Integration

**Goal:** Integrate with garbg to display the same wallpaper that will be shown after login, creating a seamless visual transition.

## Objectives

- Read garbg configuration to determine wallpaper source
- Read garbg playlist state for current wallpaper
- Apply consistent blur settings
- Ensure seamless transition from greeter to gar session
- Handle fallback when garbg config not available

## Background: Seamless Transition

The goal is that when a user logs in, the transition from greeter to desktop should feel smooth:

```
┌─────────────────────────────────────────────────────────────────┐
│ Greeter                                                         │
│ - Shows blurred version of current garbg wallpaper              │
│ - User enters credentials                                       │
│ - Login succeeds                                                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼ (fade out greeter)
┌─────────────────────────────────────────────────────────────────┐
│ gar Session                                                     │
│ - garbg starts and sets SAME wallpaper (unblurred)              │
│ - Only visible change: blur fades away, UI elements appear      │
└─────────────────────────────────────────────────────────────────┘
```

## Tasks

### 4.1 Read garbg Configuration

```rust
// gardm-greeter/src/garbg.rs

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
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

fn default_mode() -> String { "fill".to_string() }

impl GarbgConfig {
    /// Load garbg config from user or system location
    pub fn load(username: Option<&str>) -> Option<Self> {
        // Try user config first (if we know the username)
        if let Some(user) = username {
            if let Some(home) = get_user_home(user) {
                let user_config = home.join(".config/garbg/config.toml");
                if let Ok(content) = std::fs::read_to_string(&user_config) {
                    if let Ok(config) = toml::from_str(&content) {
                        return Some(config);
                    }
                }
            }
        }

        // Try system-wide default
        let system_config = PathBuf::from("/etc/garbg/config.toml");
        if let Ok(content) = std::fs::read_to_string(&system_config) {
            if let Ok(config) = toml::from_str(&content) {
                return Some(config);
            }
        }

        None
    }

    /// Get the default wallpaper path
    pub fn default_wallpaper(&self) -> Option<String> {
        if self.default.source.is_empty() {
            None
        } else {
            Some(expand_path(&self.default.source))
        }
    }
}

fn get_user_home(username: &str) -> Option<PathBuf> {
    use nix::unistd::User;
    User::from_name(username).ok()?
        .map(|u| PathBuf::from(u.dir))
}

fn expand_path(path: &str) -> String {
    shellexpand::tilde(path).to_string()
}
```

### 4.2 Read garbg Playlist State

```rust
// gardm-greeter/src/garbg.rs (continued)

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
        // Playlist state is stored in XDG_RUNTIME_DIR
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").ok()?;
        let state_file = PathBuf::from(&runtime_dir).join("garbg-state.json");

        // If that doesn't exist, try user-specific location
        if !state_file.exists() {
            if let Some(user) = username {
                // Try /run/user/<uid>/garbg-state.json
                if let Some(uid) = get_user_uid(user) {
                    let user_runtime = format!("/run/user/{}/garbg-state.json", uid);
                    if let Ok(content) = std::fs::read_to_string(&user_runtime) {
                        if let Ok(state) = serde_json::from_str(&content) {
                            return Some(state);
                        }
                    }
                }
            }
        }

        let content = std::fs::read_to_string(&state_file).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Get the current wallpaper path
    pub fn current_wallpaper(&self) -> Option<&str> {
        self.images.get(self.current_index).map(|s| s.as_str())
    }
}

fn get_user_uid(username: &str) -> Option<u32> {
    use nix::unistd::User;
    User::from_name(username).ok()?
        .map(|u| u.uid.as_raw())
}
```

### 4.3 Wallpaper Resolution Logic

```rust
// gardm-greeter/src/garbg.rs (continued)

/// Resolve which wallpaper to use for the greeter
pub struct WallpaperResolver {
    fallback_path: String,
}

impl WallpaperResolver {
    pub fn new(fallback: &str) -> Self {
        Self {
            fallback_path: fallback.to_string(),
        }
    }

    /// Resolve wallpaper path, trying multiple sources
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
                // If it's a directory, pick first image
                let path = std::path::Path::new(&default);
                if path.is_dir() {
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

/// Get first image file in a directory
fn first_image_in_dir(dir: &str) -> Option<String> {
    let extensions = ["jpg", "jpeg", "png", "webp"];

    let mut entries: Vec<_> = std::fs::read_dir(dir).ok()?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            if let Some(ext) = path.extension() {
                extensions.contains(&ext.to_string_lossy().to_lowercase().as_str())
            } else {
                false
            }
        })
        .collect();

    entries.sort_by_key(|e| e.path());
    entries.first().map(|e| e.path().to_string_lossy().to_string())
}
```

### 4.4 Shared Visual Settings

```rust
// gardm-greeter/src/config.rs

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GreeterConfig {
    #[serde(default)]
    pub visual: VisualConfig,
    #[serde(default)]
    pub garbg: GarbgIntegration,
}

#[derive(Debug, Deserialize, Default)]
pub struct VisualConfig {
    /// Blur radius for background (should match gar lock screen)
    #[serde(default = "default_blur_radius")]
    pub blur_radius: f32,

    /// Background brightness (0.0-1.0, lower = darker)
    #[serde(default = "default_brightness")]
    pub blur_brightness: f32,

    /// Corner radius for UI elements (match gar's corner_radius)
    #[serde(default = "default_corner_radius")]
    pub corner_radius: f64,
}

#[derive(Debug, Deserialize, Default)]
pub struct GarbgIntegration {
    /// Whether to use garbg wallpaper
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Fallback wallpaper if garbg not configured
    #[serde(default = "default_fallback")]
    pub fallback: String,
}

fn default_blur_radius() -> f32 { 20.0 }
fn default_brightness() -> f32 { 0.7 }
fn default_corner_radius() -> f64 { 18.0 }
fn default_true() -> bool { true }
fn default_fallback() -> String { "/usr/share/gardm/backgrounds/default.jpg".to_string() }
```

### 4.5 Integration with Greeter Main Loop

```rust
// gardm-greeter/src/main.rs (updated)

fn main() -> anyhow::Result<()> {
    let config = GreeterConfig::load()?;
    let window = GreeterWindow::new()?;

    // Resolve wallpaper using garbg integration
    let wallpaper_path = if config.garbg.enabled {
        let resolver = WallpaperResolver::new(&config.garbg.fallback);
        // Initially no username known, use system defaults
        resolver.resolve(None)
    } else {
        config.garbg.fallback.clone()
    };

    // Load and blur background
    let background = load_blurred_background(
        &wallpaper_path,
        window.width() as u32,
        window.height() as u32,
        config.visual.blur_radius,
        config.visual.blur_brightness,
    )?;

    let mut form = LoginForm::new(
        window.width() as f64,
        window.height() as f64,
        config.visual.corner_radius,
    );

    // When username is entered, potentially update wallpaper
    // to user-specific one (optional enhancement)

    // ... rest of event loop ...
}
```

### 4.6 Session Transition Effect

```rust
// gardm-greeter/src/transition.rs

use std::time::{Duration, Instant};

/// Fade out transition before starting session
pub struct FadeOutTransition {
    start_time: Instant,
    duration: Duration,
}

impl FadeOutTransition {
    pub fn new(duration_ms: u64) -> Self {
        Self {
            start_time: Instant::now(),
            duration: Duration::from_millis(duration_ms),
        }
    }

    /// Get current opacity (1.0 -> 0.0)
    pub fn opacity(&self) -> f64 {
        let elapsed = self.start_time.elapsed();
        if elapsed >= self.duration {
            0.0
        } else {
            1.0 - (elapsed.as_secs_f64() / self.duration.as_secs_f64())
        }
    }

    pub fn is_complete(&self) -> bool {
        self.start_time.elapsed() >= self.duration
    }
}

/// Apply fade during session start
pub fn render_with_fade(
    ctx: &cairo::Context,
    background: &image::RgbaImage,
    form: &LoginForm,
    fade: Option<&FadeOutTransition>,
) -> anyhow::Result<()> {
    // Render background (always full opacity)
    render_background(ctx, background)?;

    // Render UI with fade
    if let Some(fade) = fade {
        ctx.push_group();
        form.render(ctx, &pango_ctx)?;
        ctx.pop_group_to_source()?;
        ctx.paint_with_alpha(fade.opacity())?;
    } else {
        form.render(ctx, &pango_ctx)?;
    }

    Ok(())
}
```

## Acceptance Criteria

1. Greeter shows same wallpaper as garbg will after login
2. Blur settings are consistent with gar lock screen
3. Fallback works when garbg is not configured
4. User-specific wallpaper is used when username is known
5. Smooth fade transition when starting session
6. No flash of different wallpaper during login

## Pitfalls to Avoid

1. **File permissions** - greeter runs as gardm user, may not access user home
2. **Race conditions** - garbg state file might be stale
3. **Directory vs file** - garbg source might be a directory
4. **Missing files** - wallpaper might have been deleted
5. **Large images** - blur on 4K images is slow, cache if needed

## Testing

```bash
# Test wallpaper resolution
# 1. With garbg configured and running
garbg set ~/Pictures/wallpapers --random
DISPLAY=:1 ./target/release/gardm-greeter

# 2. With garbg config but no playlist
rm ~/.local/state/garbg-state.json
DISPLAY=:1 ./target/release/gardm-greeter

# 3. Without any garbg config (should use fallback)
mv ~/.config/garbg/config.toml ~/.config/garbg/config.toml.bak
DISPLAY=:1 ./target/release/gardm-greeter
```

## Dependencies for This Sprint

```toml
# gardm-greeter/Cargo.toml
[dependencies]
shellexpand = "3.0"
toml = "0.8"
serde_json = "1.0"
```

## Integration Checklist

- [ ] Greeter reads `~/.config/garbg/config.toml`
- [ ] Greeter reads `$XDG_RUNTIME_DIR/garbg-state.json`
- [ ] Blur radius matches gar lock screen (if implemented)
- [ ] Corner radius matches gar's `corner_radius` setting
- [ ] Fallback image is bundled with gardm package
- [ ] Fade transition timing feels smooth (150-200ms)

## Future Enhancements

- [ ] Per-user wallpaper preview before login
- [ ] Animated wallpaper support (match garbg animation)
- [ ] Workspace-specific wallpaper from garbg config
- [ ] Live wallpaper update if garbg changes during greeter

## Next Steps

After Sprint 4, consider:
- Sprint 5: Power buttons and session selector
- Sprint 6: User list with avatars
- Sprint 7: Accessibility and theming
- Sprint 8: Multi-monitor support

//! User avatar loading and rendering
//!
//! Loads user avatars from standard locations and renders them as circles.

use anyhow::Result;
use cairo::Context;
use image::{imageops, RgbaImage};
use std::collections::HashMap;
use std::path::PathBuf;

/// Avatar cache to avoid reloading images
pub struct AvatarCache {
    avatars: HashMap<String, Option<RgbaImage>>,
    size: u32,
}

impl AvatarCache {
    /// Create a new avatar cache with specified avatar size
    pub fn new(size: u32) -> Self {
        Self {
            avatars: HashMap::new(),
            size,
        }
    }

    /// Get or load avatar for a user
    pub fn get(&mut self, username: &str, home: Option<&str>) -> Option<&RgbaImage> {
        if !self.avatars.contains_key(username) {
            let avatar = load_avatar(username, home, self.size);
            self.avatars.insert(username.to_string(), avatar);
        }
        self.avatars.get(username).and_then(|a| a.as_ref())
    }
}

/// Load avatar for a user from standard locations
fn load_avatar(username: &str, home: Option<&str>, size: u32) -> Option<RgbaImage> {
    let paths = avatar_paths(username, home);

    for path in paths {
        if path.exists() {
            if let Ok(img) = image::open(&path) {
                let rgba = img.to_rgba8();
                // Scale to size (square, will be masked to circle when rendering)
                let scaled = scale_to_square(&rgba, size);
                return Some(scaled);
            }
        }
    }

    None
}

/// Get possible avatar paths for a user
fn avatar_paths(username: &str, home: Option<&str>) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // User home directory paths
    if let Some(home) = home {
        let home = PathBuf::from(home);
        // freedesktop standard
        paths.push(home.join(".face"));
        paths.push(home.join(".face.icon"));
        // KDE
        paths.push(home.join(".face.png"));
        // GNOME/config
        paths.push(home.join(".config/face.png"));
    }

    // AccountsService (system-wide)
    paths.push(PathBuf::from(format!(
        "/var/lib/AccountsService/icons/{}",
        username
    )));

    paths
}

/// Scale image to a square of given size
fn scale_to_square(img: &RgbaImage, size: u32) -> RgbaImage {
    let (w, h) = img.dimensions();

    // Crop to square first (center crop)
    let min_dim = w.min(h);
    let x = (w - min_dim) / 2;
    let y = (h - min_dim) / 2;
    let cropped = imageops::crop_imm(img, x, y, min_dim, min_dim).to_image();

    // Scale to target size
    imageops::resize(&cropped, size, size, imageops::FilterType::Lanczos3)
}

/// Render an avatar image as a circle
pub fn render_avatar_image(
    ctx: &Context,
    img: &RgbaImage,
    x: f64,
    y: f64,
    size: f64,
) -> Result<()> {
    let (width, height) = img.dimensions();

    // Convert RGBA to BGRA for Cairo
    let bgra_data = rgba_to_bgra(img);

    let surface = cairo::ImageSurface::create_for_data(
        bgra_data,
        cairo::Format::ARgb32,
        width as i32,
        height as i32,
        (width * 4) as i32,
    )?;

    // Create circular clip
    ctx.save()?;
    ctx.new_path(); // Clear any previous path
    let radius = size / 2.0;
    ctx.arc(x + radius, y + radius, radius, 0.0, 2.0 * std::f64::consts::PI);
    ctx.clip();

    // Draw the image scaled to size
    let scale = size / width as f64;
    ctx.translate(x, y);
    ctx.scale(scale, scale);
    ctx.set_source_surface(&surface, 0.0, 0.0)?;
    ctx.paint()?;

    ctx.restore()?;

    Ok(())
}

/// Render a fallback avatar with initials
pub fn render_avatar_fallback(
    ctx: &Context,
    pango_ctx: &pango::Context,
    initials: &str,
    x: f64,
    y: f64,
    size: f64,
    hue: f64, // 0.0-1.0 for color variety
) -> Result<()> {
    let radius = size / 2.0;
    let cx = x + radius;
    let cy = y + radius;

    // Clear any previous path
    ctx.new_path();

    // Background circle with hue-based color
    let (r, g, b) = hue_to_rgb(hue);
    ctx.set_source_rgb(r * 0.6, g * 0.6, b * 0.6);
    ctx.arc(cx, cy, radius, 0.0, 2.0 * std::f64::consts::PI);
    ctx.fill()?;

    // Initials text
    let mut font = pango::FontDescription::new();
    font.set_family("Sans");
    font.set_weight(pango::Weight::Bold);
    font.set_size((size * 0.4) as i32 * pango::SCALE);

    let layout = pango::Layout::new(pango_ctx);
    layout.set_font_description(Some(&font));
    layout.set_text(initials);

    let (text_w, text_h) = layout.pixel_size();

    ctx.set_source_rgb(1.0, 1.0, 1.0);
    ctx.move_to(cx - text_w as f64 / 2.0, cy - text_h as f64 / 2.0);
    pangocairo::functions::show_layout(ctx, &layout);

    Ok(())
}

/// Render avatar with border (for selected state)
pub fn render_avatar_border(ctx: &Context, x: f64, y: f64, size: f64, selected: bool) -> Result<()> {
    // Clear any previous path
    ctx.new_path();

    let radius = size / 2.0;
    let cx = x + radius;
    let cy = y + radius;

    if selected {
        // Highlight ring
        ctx.set_source_rgba(0.3, 0.6, 1.0, 1.0);
        ctx.set_line_width(3.0);
        ctx.arc(cx, cy, radius + 2.0, 0.0, 2.0 * std::f64::consts::PI);
        ctx.stroke()?;
    }

    Ok(())
}

/// Convert RGBA to BGRA for Cairo
fn rgba_to_bgra(img: &RgbaImage) -> Vec<u8> {
    let (width, height) = img.dimensions();
    let mut data = vec![0u8; (width * height * 4) as usize];

    for (y, row) in img.rows().enumerate() {
        for (x, pixel) in row.enumerate() {
            let offset = y * (width as usize * 4) + x * 4;
            data[offset] = pixel[2];     // B
            data[offset + 1] = pixel[1]; // G
            data[offset + 2] = pixel[0]; // R
            data[offset + 3] = pixel[3]; // A
        }
    }

    data
}

/// Convert hue (0.0-1.0) to RGB for avatar colors
fn hue_to_rgb(h: f64) -> (f64, f64, f64) {
    let h = h * 6.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();

    match h as i32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    }
}

/// Get initials from a name
pub fn get_initials(name: &str) -> String {
    let parts: Vec<&str> = name.split_whitespace().collect();
    match parts.len() {
        0 => "?".to_string(),
        1 => parts[0].chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or("?".to_string()),
        _ => {
            let first = parts[0].chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
            let last = parts.last().and_then(|s| s.chars().next()).map(|c| c.to_uppercase().to_string()).unwrap_or_default();
            format!("{}{}", first, last)
        }
    }
}

/// Generate a consistent hue from a string (for fallback avatar colors)
pub fn string_to_hue(s: &str) -> f64 {
    let hash: u32 = s.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    (hash % 360) as f64 / 360.0
}

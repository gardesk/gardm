//! Background image loading and processing for the greeter
//!
//! Loads background images, scales to fit screen, applies blur and brightness adjustments.

use anyhow::{Context, Result};
use image::{imageops, RgbaImage};

/// Load and process a background image for the greeter
pub fn load_blurred_background(
    path: &str,
    width: u32,
    height: u32,
    blur_radius: f32,
    brightness: f32,
) -> Result<RgbaImage> {
    let img = image::open(path)
        .with_context(|| format!("Failed to open background image: {}", path))?
        .to_rgba8();

    // Scale to screen size (cover mode - fills screen, may crop)
    let scaled = scale_to_cover(&img, width, height);

    // Apply gaussian blur
    let blurred = imageops::blur(&scaled, blur_radius);

    // Adjust brightness (typically darken for better text contrast)
    let adjusted = adjust_brightness(&blurred, brightness);

    Ok(adjusted)
}

/// Generate a solid color fallback background
pub fn solid_background(width: u32, height: u32, r: u8, g: u8, b: u8) -> RgbaImage {
    RgbaImage::from_fn(width, height, |_, _| image::Rgba([r, g, b, 255]))
}

/// Scale image to cover the target dimensions (may crop)
fn scale_to_cover(img: &RgbaImage, target_w: u32, target_h: u32) -> RgbaImage {
    let (src_w, src_h) = img.dimensions();

    // Calculate scale to cover entire target
    let scale = (target_w as f32 / src_w as f32).max(target_h as f32 / src_h as f32);

    let new_w = (src_w as f32 * scale).ceil() as u32;
    let new_h = (src_h as f32 * scale).ceil() as u32;

    let resized = imageops::resize(img, new_w, new_h, imageops::FilterType::Lanczos3);

    // Crop to center
    let x = (new_w.saturating_sub(target_w)) / 2;
    let y = (new_h.saturating_sub(target_h)) / 2;

    imageops::crop_imm(&resized, x, y, target_w, target_h).to_image()
}

/// Adjust image brightness by a factor (0.0-1.0 darkens, >1.0 brightens)
fn adjust_brightness(img: &RgbaImage, factor: f32) -> RgbaImage {
    let mut result = img.clone();
    for pixel in result.pixels_mut() {
        pixel[0] = (pixel[0] as f32 * factor).min(255.0) as u8;
        pixel[1] = (pixel[1] as f32 * factor).min(255.0) as u8;
        pixel[2] = (pixel[2] as f32 * factor).min(255.0) as u8;
        // Alpha unchanged
    }
    result
}

/// Convert RGBA image to BGRA for Cairo/X11 (little-endian ARGB32)
pub fn rgba_to_bgra(img: &RgbaImage) -> Vec<u8> {
    let (width, height) = img.dimensions();
    let stride = (width * 4) as usize;
    let mut data = vec![0u8; stride * height as usize];

    for (y, row) in img.rows().enumerate() {
        for (x, pixel) in row.enumerate() {
            let offset = y * stride + x * 4;
            // RGBA -> BGRA
            data[offset] = pixel[2]; // B
            data[offset + 1] = pixel[1]; // G
            data[offset + 2] = pixel[0]; // R
            data[offset + 3] = pixel[3]; // A
        }
    }

    data
}

/// Render background image to Cairo context
pub fn render_to_cairo(
    ctx: &cairo::Context,
    img: &RgbaImage,
) -> Result<()> {
    let (width, height) = img.dimensions();
    let bgra_data = rgba_to_bgra(img);

    let surface = cairo::ImageSurface::create_for_data(
        bgra_data,
        cairo::Format::ARgb32,
        width as i32,
        height as i32,
        (width * 4) as i32,
    )
    .context("Failed to create Cairo surface from background")?;

    ctx.set_source_surface(&surface, 0.0, 0.0)?;
    ctx.paint()?;

    Ok(())
}

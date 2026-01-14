//! Cairo rendering for the greeter
//!
//! Provides a Cairo surface that can be rendered to X11.

use anyhow::{Context, Result};
use cairo::{Context as CairoContext, Format, ImageSurface};

/// Cairo-based renderer
pub struct Renderer {
    surface: ImageSurface,
    width: i32,
    height: i32,
}

impl Renderer {
    /// Create a new renderer with the given dimensions
    pub fn new(width: u16, height: u16) -> Result<Self> {
        let surface = ImageSurface::create(Format::ARgb32, width as i32, height as i32)
            .context("Failed to create Cairo surface")?;

        Ok(Self {
            surface,
            width: width as i32,
            height: height as i32,
        })
    }

    /// Get a Cairo context for drawing
    pub fn context(&self) -> Result<CairoContext> {
        CairoContext::new(&self.surface).context("Failed to create Cairo context")
    }

    /// Get the raw pixel data for X11 (BGRA format on little-endian)
    pub fn data(&mut self) -> Result<Vec<u8>> {
        self.surface.flush();

        let stride = self.surface.stride() as usize;
        let height = self.height as usize;

        let data = self
            .surface
            .data()
            .context("Failed to get surface data")?;

        // Cairo uses ARGB, which on little-endian is BGRA in memory
        // X11 with depth 24/32 typically expects the same
        Ok(data[..stride * height].to_vec())
    }

    /// Get the width
    pub fn width(&self) -> i32 {
        self.width
    }

    /// Get the height
    pub fn height(&self) -> i32 {
        self.height
    }

    /// Clear the surface with a solid color
    pub fn clear(&self, r: f64, g: f64, b: f64) -> Result<()> {
        let ctx = self.context()?;
        ctx.set_source_rgb(r, g, b);
        ctx.paint()?;
        Ok(())
    }
}

/// Draw a rounded rectangle path
pub fn rounded_rectangle(ctx: &CairoContext, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let degrees = std::f64::consts::PI / 180.0;
    ctx.new_sub_path();
    ctx.arc(x + w - r, y + r, r, -90.0 * degrees, 0.0);
    ctx.arc(x + w - r, y + h - r, r, 0.0, 90.0 * degrees);
    ctx.arc(x + r, y + h - r, r, 90.0 * degrees, 180.0 * degrees);
    ctx.arc(x + r, y + r, r, 180.0 * degrees, 270.0 * degrees);
    ctx.close_path();
}

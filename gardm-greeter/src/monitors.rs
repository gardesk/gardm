//! Multi-monitor detection using XRandR
//!
//! Detects connected monitors and their positions for proper UI placement.

use anyhow::{Context, Result};
use x11rb::connection::Connection;
use x11rb::protocol::randr::{self, ConnectionExt as RandrConnectionExt};
use x11rb::protocol::xproto::Window;
use x11rb::rust_connection::RustConnection;

/// Information about a connected monitor
#[derive(Debug, Clone)]
pub struct Monitor {
    /// X position in virtual screen
    pub x: i16,
    /// Y position in virtual screen
    pub y: i16,
    /// Width in pixels
    pub width: u16,
    /// Height in pixels
    pub height: u16,
    /// Whether this is the primary monitor
    pub primary: bool,
    /// Monitor name (e.g., "DP-1", "HDMI-A-1")
    pub name: String,
}

impl Monitor {
    /// Get the center X coordinate of this monitor
    pub fn center_x(&self) -> f64 {
        self.x as f64 + self.width as f64 / 2.0
    }

    /// Get the center Y coordinate of this monitor
    pub fn center_y(&self) -> f64 {
        self.y as f64 + self.height as f64 / 2.0
    }
}

/// Detected monitor configuration
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    /// All connected monitors
    pub monitors: Vec<Monitor>,
    /// Total virtual screen width
    pub total_width: u16,
    /// Total virtual screen height
    pub total_height: u16,
}

impl MonitorConfig {
    /// Detect monitors using RandR
    pub fn detect(conn: &RustConnection, root: Window) -> Result<Self> {
        // Get screen resources
        let resources = conn
            .randr_get_screen_resources(root)
            .context("Failed to get screen resources")?
            .reply()
            .context("Failed to get screen resources reply")?;

        // Get primary output
        let primary_output = conn
            .randr_get_output_primary(root)
            .context("Failed to get primary output")?
            .reply()
            .context("Failed to get primary output reply")?
            .output;

        let mut monitors = Vec::new();

        for output in &resources.outputs {
            let output_info = match conn.randr_get_output_info(*output, 0) {
                Ok(cookie) => match cookie.reply() {
                    Ok(info) => info,
                    Err(_) => continue,
                },
                Err(_) => continue,
            };

            // Skip disconnected outputs
            if output_info.connection != randr::Connection::CONNECTED {
                continue;
            }

            // Skip outputs without a CRTC (not active)
            let crtc = match output_info.crtc {
                0 => continue,
                c => c,
            };

            let crtc_info = match conn.randr_get_crtc_info(crtc, 0) {
                Ok(cookie) => match cookie.reply() {
                    Ok(info) => info,
                    Err(_) => continue,
                },
                Err(_) => continue,
            };

            // Skip CRTCs with zero dimensions
            if crtc_info.width == 0 || crtc_info.height == 0 {
                continue;
            }

            let name = String::from_utf8_lossy(&output_info.name).to_string();
            let is_primary = *output == primary_output;

            monitors.push(Monitor {
                x: crtc_info.x,
                y: crtc_info.y,
                width: crtc_info.width,
                height: crtc_info.height,
                primary: is_primary,
                name,
            });
        }

        // Calculate total virtual screen size
        let (total_width, total_height) = if monitors.is_empty() {
            // Fallback to root window size
            let screen = &conn.setup().roots[0];
            (screen.width_in_pixels, screen.height_in_pixels)
        } else {
            let max_x = monitors
                .iter()
                .map(|m| m.x as i32 + m.width as i32)
                .max()
                .unwrap_or(0);
            let max_y = monitors
                .iter()
                .map(|m| m.y as i32 + m.height as i32)
                .max()
                .unwrap_or(0);
            (max_x as u16, max_y as u16)
        };

        // If no primary is set, mark the largest monitor as primary
        if !monitors.iter().any(|m| m.primary) && !monitors.is_empty() {
            let largest_idx = monitors
                .iter()
                .enumerate()
                .max_by_key(|(_, m)| m.width as u32 * m.height as u32)
                .map(|(i, _)| i)
                .unwrap_or(0);
            monitors[largest_idx].primary = true;
        }

        tracing::info!(
            count = monitors.len(),
            total_width,
            total_height,
            "Detected monitors"
        );

        for monitor in &monitors {
            tracing::debug!(
                name = %monitor.name,
                x = monitor.x,
                y = monitor.y,
                width = monitor.width,
                height = monitor.height,
                primary = monitor.primary,
                "Monitor"
            );
        }

        Ok(Self {
            monitors,
            total_width,
            total_height,
        })
    }

    /// Get the primary monitor
    pub fn primary(&self) -> Option<&Monitor> {
        self.monitors.iter().find(|m| m.primary)
    }

    /// Get the primary monitor, or the first one if no primary
    pub fn primary_or_first(&self) -> Option<&Monitor> {
        self.primary().or_else(|| self.monitors.first())
    }

    /// Check if this is a single-monitor setup
    pub fn is_single_monitor(&self) -> bool {
        self.monitors.len() <= 1
    }
}

/// Fallback monitor config when RandR fails
pub fn fallback_config(screen_width: u16, screen_height: u16) -> MonitorConfig {
    MonitorConfig {
        monitors: vec![Monitor {
            x: 0,
            y: 0,
            width: screen_width,
            height: screen_height,
            primary: true,
            name: "default".to_string(),
        }],
        total_width: screen_width,
        total_height: screen_height,
    }
}

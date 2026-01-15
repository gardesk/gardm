//! Power control buttons widget
//!
//! Shutdown, reboot, and suspend buttons with hover effects.

use crate::icons;
use crate::render::rounded_rectangle;
use anyhow::Result;
use cairo::Context;

/// Power button action types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerAction {
    Shutdown,
    Reboot,
    Suspend,
}

/// Individual power button
struct PowerButton {
    action: PowerAction,
    x: f64,
    y: f64,
    size: f64,
    hovered: bool,
}

impl PowerButton {
    fn new(action: PowerAction, x: f64, y: f64, size: f64) -> Self {
        Self {
            action,
            x,
            y,
            size,
            hovered: false,
        }
    }

    fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x && px <= self.x + self.size && py >= self.y && py <= self.y + self.size
    }

    fn render(&self, ctx: &Context) -> Result<()> {
        let icon_size = self.size * 0.5;
        let icon_x = self.x + (self.size - icon_size) / 2.0;
        let icon_y = self.y + (self.size - icon_size) / 2.0;

        // Background with hover effect
        if self.hovered {
            // Glow effect on hover
            ctx.set_source_rgba(1.0, 1.0, 1.0, 0.15);
            rounded_rectangle(ctx, self.x, self.y, self.size, self.size, 12.0);
            ctx.fill()?;

            // Brighter icon on hover
            let (r, g, b) = self.hover_color();
            ctx.set_source_rgb(r, g, b);
        } else {
            // Normal semi-transparent background
            ctx.set_source_rgba(1.0, 1.0, 1.0, 0.05);
            rounded_rectangle(ctx, self.x, self.y, self.size, self.size, 12.0);
            ctx.fill()?;

            // Normal icon color
            ctx.set_source_rgba(0.8, 0.8, 0.8, 0.9);
        }

        // Draw the icon
        match self.action {
            PowerAction::Shutdown => icons::draw_power(ctx, icon_x, icon_y, icon_size),
            PowerAction::Reboot => icons::draw_reboot(ctx, icon_x, icon_y, icon_size),
            PowerAction::Suspend => icons::draw_suspend(ctx, icon_x, icon_y, icon_size),
        }

        Ok(())
    }

    /// Get the hover highlight color for each action
    fn hover_color(&self) -> (f64, f64, f64) {
        match self.action {
            PowerAction::Shutdown => (1.0, 0.4, 0.4), // Red tint
            PowerAction::Reboot => (0.4, 0.8, 1.0),   // Blue tint
            PowerAction::Suspend => (0.9, 0.8, 0.4),  // Yellow/gold tint
        }
    }
}

/// Power buttons panel
pub struct PowerButtons {
    buttons: Vec<PowerButton>,
}

impl PowerButtons {
    /// Create power buttons positioned in bottom-right corner of given area
    /// For multi-monitor: pass the primary monitor's x, y, width, height
    pub fn new(area_x: f64, area_y: f64, area_width: f64, area_height: f64) -> Self {
        let button_size = 48.0;
        let spacing = 12.0;
        let margin = 24.0;

        // Position in bottom-right corner of the area
        let start_x = area_x + area_width - margin - (button_size * 3.0 + spacing * 2.0);
        let y = area_y + area_height - margin - button_size;

        let buttons = vec![
            PowerButton::new(PowerAction::Suspend, start_x, y, button_size),
            PowerButton::new(
                PowerAction::Reboot,
                start_x + button_size + spacing,
                y,
                button_size,
            ),
            PowerButton::new(
                PowerAction::Shutdown,
                start_x + (button_size + spacing) * 2.0,
                y,
                button_size,
            ),
        ];

        Self { buttons }
    }

    /// Render all power buttons
    pub fn render(&self, ctx: &Context) -> Result<()> {
        for button in &self.buttons {
            button.render(ctx)?;
        }
        Ok(())
    }

    /// Update hover state based on mouse position
    /// Returns true if any hover state changed
    pub fn update_hover(&mut self, mouse_x: f64, mouse_y: f64) -> bool {
        let mut changed = false;
        for button in &mut self.buttons {
            let was_hovered = button.hovered;
            button.hovered = button.contains(mouse_x, mouse_y);
            if button.hovered != was_hovered {
                changed = true;
            }
        }
        changed
    }

    /// Clear all hover states
    pub fn clear_hover(&mut self) {
        for button in &mut self.buttons {
            button.hovered = false;
        }
    }

    /// Check if a click hit any button and return the action
    pub fn handle_click(&self, click_x: f64, click_y: f64) -> Option<PowerAction> {
        for button in &self.buttons {
            if button.contains(click_x, click_y) {
                return Some(button.action);
            }
        }
        None
    }

    /// Get which button is currently hovered (for cursor changes)
    pub fn hovered_action(&self) -> Option<PowerAction> {
        self.buttons
            .iter()
            .find(|b| b.hovered)
            .map(|b| b.action)
    }
}

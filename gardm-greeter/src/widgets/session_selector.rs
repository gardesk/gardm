//! Session selector dropdown widget
//!
//! Allows users to choose which desktop session to start.

use crate::icons;
use crate::render::rounded_rectangle;
use crate::theme::Theme;
use anyhow::Result;
use cairo::Context;
use gardm_ipc::SessionInfo;
use pango::{FontDescription, Layout};

/// Session selector dropdown
pub struct SessionSelector {
    sessions: Vec<SessionInfo>,
    selected_index: usize,
    expanded: bool,
    hovered_index: Option<usize>,

    // Layout
    x: f64,
    y: f64,
    width: f64,
    item_height: f64,
}

impl SessionSelector {
    /// Create a new session selector
    pub fn new(sessions: Vec<SessionInfo>, x: f64, y: f64, width: f64) -> Self {
        Self {
            sessions,
            selected_index: 0,
            expanded: false,
            hovered_index: None,
            x,
            y,
            width,
            item_height: 40.0,
        }
    }

    /// Get the currently selected session
    pub fn selected(&self) -> Option<&SessionInfo> {
        self.sessions.get(self.selected_index)
    }

    /// Get the exec command for the selected session
    pub fn selected_exec(&self) -> Option<&str> {
        self.selected().map(|s| s.exec.as_str())
    }

    /// Check if dropdown is expanded
    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    /// Toggle dropdown expanded state
    pub fn toggle(&mut self) {
        self.expanded = !self.expanded;
        if !self.expanded {
            self.hovered_index = None;
        }
    }

    /// Close the dropdown
    pub fn close(&mut self) {
        self.expanded = false;
        self.hovered_index = None;
    }

    /// Render the session selector
    pub fn render(&self, ctx: &Context, pango_ctx: &pango::Context, theme: &Theme) -> Result<()> {
        // Main button (always visible)
        self.render_button(ctx, pango_ctx, theme)?;

        // Dropdown list (when expanded)
        if self.expanded && !self.sessions.is_empty() {
            self.render_dropdown(ctx, pango_ctx, theme)?;
        }

        Ok(())
    }

    fn render_button(&self, ctx: &Context, pango_ctx: &pango::Context, theme: &Theme) -> Result<()> {
        // Background
        let bg = &theme.input_background;
        ctx.set_source_rgba(bg.r, bg.g, bg.b, 0.9);
        rounded_rectangle(ctx, self.x, self.y, self.width, self.item_height, 8.0);
        ctx.fill()?;

        // Border
        ctx.set_source_rgba(0.4, 0.4, 0.4, 0.8);
        rounded_rectangle(ctx, self.x, self.y, self.width, self.item_height, 8.0);
        ctx.set_line_width(1.0);
        ctx.stroke()?;

        // Session name
        let name = self
            .selected()
            .map(|s| s.name.as_str())
            .unwrap_or("No sessions");

        let mut font = FontDescription::new();
        font.set_family(&theme.font_family);
        font.set_size(13 * pango::SCALE);

        let layout = Layout::new(pango_ctx);
        layout.set_font_description(Some(&font));
        layout.set_text(name);

        let tc = &theme.text_primary;
        ctx.set_source_rgb(tc.r, tc.g, tc.b);
        ctx.move_to(self.x + 12.0, self.y + (self.item_height - 16.0) / 2.0);
        pangocairo::functions::show_layout(ctx, &layout);

        // Chevron icon
        let chevron_size = 16.0;
        let chevron_x = self.x + self.width - chevron_size - 12.0;
        let chevron_y = self.y + (self.item_height - chevron_size) / 2.0;
        let sc = &theme.text_secondary;
        ctx.set_source_rgba(sc.r, sc.g, sc.b, sc.a);
        icons::draw_chevron_down(ctx, chevron_x, chevron_y, chevron_size);

        Ok(())
    }

    fn render_dropdown(&self, ctx: &Context, pango_ctx: &pango::Context, theme: &Theme) -> Result<()> {
        let dropdown_height = self.sessions.len() as f64 * self.item_height;
        let dropdown_y = self.y - dropdown_height - 4.0; // Above the button

        // Dropdown background
        let bg = &theme.panel_background;
        ctx.set_source_rgba(bg.r * 0.8, bg.g * 0.8, bg.b * 0.8, 0.95);
        rounded_rectangle(ctx, self.x, dropdown_y, self.width, dropdown_height, 8.0);
        ctx.fill()?;

        // Border
        ctx.set_source_rgba(0.3, 0.3, 0.3, 0.8);
        rounded_rectangle(ctx, self.x, dropdown_y, self.width, dropdown_height, 8.0);
        ctx.set_line_width(1.0);
        ctx.stroke()?;

        // Items
        let mut font = FontDescription::new();
        font.set_family(&theme.font_family);
        font.set_size(13 * pango::SCALE);

        for (i, session) in self.sessions.iter().enumerate() {
            let item_y = dropdown_y + i as f64 * self.item_height;
            let is_selected = i == self.selected_index;
            let is_hovered = self.hovered_index == Some(i);

            // Item background on hover
            if is_hovered {
                let ac = &theme.accent;
                ctx.set_source_rgba(ac.r, ac.g, ac.b, 0.5);
                if i == 0 {
                    // First item - round top corners
                    rounded_rectangle(ctx, self.x + 2.0, item_y + 2.0, self.width - 4.0, self.item_height - 2.0, 6.0);
                } else if i == self.sessions.len() - 1 {
                    // Last item - round bottom corners
                    rounded_rectangle(ctx, self.x + 2.0, item_y, self.width - 4.0, self.item_height - 2.0, 6.0);
                } else {
                    ctx.rectangle(self.x + 2.0, item_y, self.width - 4.0, self.item_height);
                }
                ctx.fill()?;
            }

            // Checkmark for selected item
            if is_selected {
                ctx.set_source_rgba(0.4, 0.8, 0.4, 1.0);
                icons::draw_checkmark(ctx, self.x + 8.0, item_y + 8.0, 24.0);
            }

            // Session name
            let layout = Layout::new(pango_ctx);
            layout.set_font_description(Some(&font));
            layout.set_text(&session.name);

            let tc = &theme.text_primary;
            if is_selected {
                ctx.set_source_rgb(tc.r, tc.g, tc.b);
            } else {
                ctx.set_source_rgba(tc.r, tc.g, tc.b, 0.9);
            }
            ctx.move_to(self.x + 36.0, item_y + (self.item_height - 16.0) / 2.0);
            pangocairo::functions::show_layout(ctx, &layout);
        }

        Ok(())
    }

    /// Update hover state based on mouse position
    pub fn update_hover(&mut self, mouse_x: f64, mouse_y: f64) -> bool {
        if !self.expanded {
            return false;
        }

        let dropdown_height = self.sessions.len() as f64 * self.item_height;
        let dropdown_y = self.y - dropdown_height - 4.0;

        let old_hover = self.hovered_index;

        // Check if mouse is in dropdown area
        if mouse_x >= self.x
            && mouse_x <= self.x + self.width
            && mouse_y >= dropdown_y
            && mouse_y <= dropdown_y + dropdown_height
        {
            let relative_y = mouse_y - dropdown_y;
            self.hovered_index = Some((relative_y / self.item_height) as usize);
        } else {
            self.hovered_index = None;
        }

        self.hovered_index != old_hover
    }

    /// Check if click is on the main button
    pub fn button_contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.item_height
    }

    /// Check if click is in the dropdown and handle selection
    /// Returns true if a selection was made
    pub fn handle_dropdown_click(&mut self, click_x: f64, click_y: f64) -> bool {
        if !self.expanded {
            return false;
        }

        let dropdown_height = self.sessions.len() as f64 * self.item_height;
        let dropdown_y = self.y - dropdown_height - 4.0;

        if click_x >= self.x
            && click_x <= self.x + self.width
            && click_y >= dropdown_y
            && click_y <= dropdown_y + dropdown_height
        {
            let relative_y = click_y - dropdown_y;
            let index = (relative_y / self.item_height) as usize;
            if index < self.sessions.len() {
                self.selected_index = index;
                self.close();
                return true;
            }
        }

        false
    }

    /// Check if a point is within the selector bounds (button or dropdown)
    pub fn contains(&self, x: f64, y: f64) -> bool {
        // Check main button
        if self.button_contains(x, y) {
            return true;
        }

        // Check dropdown if expanded
        if self.expanded {
            let dropdown_height = self.sessions.len() as f64 * self.item_height;
            let dropdown_y = self.y - dropdown_height - 4.0;

            if x >= self.x
                && x <= self.x + self.width
                && y >= dropdown_y
                && y <= dropdown_y + dropdown_height
            {
                return true;
            }
        }

        false
    }
}

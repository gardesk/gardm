//! User list widget with avatars
//!
//! Displays available users with circular avatars for quick selection.

use crate::avatar::{
    get_initials, render_avatar_border, render_avatar_fallback, render_avatar_image,
    string_to_hue, AvatarCache,
};
use crate::render::rounded_rectangle;
use crate::theme::Theme;
use anyhow::Result;
use cairo::Context;
use gardm_ipc::UserInfo;
use pango::{FontDescription, Layout};

/// User list widget showing avatars in a horizontal row
pub struct UserList {
    users: Vec<UserInfo>,
    selected_index: Option<usize>,
    hovered_index: Option<usize>,
    avatar_cache: AvatarCache,

    // Layout
    x: f64,
    y: f64,
    avatar_size: f64,
    spacing: f64,
}

impl UserList {
    /// Create a new user list centered above the login form
    /// center_x, center_y is the center point of the primary monitor
    pub fn new(users: Vec<UserInfo>, center_x: f64, center_y: f64) -> Self {
        let avatar_size = 64.0;
        let spacing = 24.0;
        let total_width = if users.is_empty() {
            0.0
        } else {
            users.len() as f64 * (avatar_size + spacing) - spacing
        };

        // Center horizontally on primary monitor, position above center
        let x = center_x - total_width / 2.0;
        let y = center_y - 220.0; // Above the login form

        Self {
            users,
            selected_index: None,
            hovered_index: None,
            avatar_cache: AvatarCache::new(avatar_size as u32 * 2), // 2x for quality
            x,
            y,
            avatar_size,
            spacing,
        }
    }

    /// Check if the user list has any users
    pub fn is_empty(&self) -> bool {
        self.users.is_empty()
    }

    /// Get the selected user's username
    pub fn selected_username(&self) -> Option<&str> {
        self.selected_index
            .and_then(|i| self.users.get(i))
            .map(|u| u.name.as_str())
    }

    /// Select a user by index
    pub fn select(&mut self, index: usize) {
        if index < self.users.len() {
            self.selected_index = Some(index);
        }
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selected_index = None;
    }

    /// Render the user list
    pub fn render(&mut self, ctx: &Context, pango_ctx: &pango::Context, theme: &Theme) -> Result<()> {
        if self.users.is_empty() {
            return Ok(());
        }

        for (i, user) in self.users.iter().enumerate() {
            let item_x = self.x + i as f64 * (self.avatar_size + self.spacing);
            let item_y = self.y;

            let is_selected = self.selected_index == Some(i);
            let is_hovered = self.hovered_index == Some(i);

            // Hover/selection background
            if is_hovered || is_selected {
                let bg_padding = 8.0;
                let bg_x = item_x - bg_padding;
                let bg_y = item_y - bg_padding;
                let bg_w = self.avatar_size + bg_padding * 2.0;
                let bg_h = self.avatar_size + 28.0 + bg_padding * 2.0; // Include name

                if is_selected {
                    let ac = &theme.accent;
                    ctx.set_source_rgba(ac.r, ac.g, ac.b, 0.3);
                } else {
                    ctx.set_source_rgba(1.0, 1.0, 1.0, 0.1);
                }
                rounded_rectangle(ctx, bg_x, bg_y, bg_w, bg_h, theme.corner_radius * 0.75);
                ctx.fill()?;
            }

            // Avatar
            let home = user.home.to_str();
            if let Some(avatar_img) = self.avatar_cache.get(&user.name, home) {
                render_avatar_image(ctx, avatar_img, item_x, item_y, self.avatar_size)?;
            } else {
                // Fallback to initials
                let display_name = user.full_name.as_deref().unwrap_or(&user.name);
                let initials = get_initials(display_name);
                let hue = string_to_hue(&user.name);
                render_avatar_fallback(ctx, pango_ctx, &initials, item_x, item_y, self.avatar_size, hue)?;
            }

            // Selection border
            render_avatar_border(ctx, item_x, item_y, self.avatar_size, is_selected)?;

            // Username below avatar
            let mut font = FontDescription::new();
            font.set_family(&theme.font_family);
            font.set_size(11 * pango::SCALE);

            let layout = Layout::new(pango_ctx);
            layout.set_font_description(Some(&font));

            // Use display name if available, otherwise username
            let display_name = user
                .full_name
                .as_ref()
                .and_then(|n| n.split_whitespace().next())
                .unwrap_or(&user.name);
            layout.set_text(display_name);

            let (text_w, _) = layout.pixel_size();
            let text_x = item_x + (self.avatar_size - text_w as f64) / 2.0;
            let text_y = item_y + self.avatar_size + 8.0;

            let tc = &theme.text_primary;
            if is_selected {
                ctx.set_source_rgb(tc.r, tc.g, tc.b);
            } else {
                ctx.set_source_rgba(tc.r, tc.g, tc.b, 0.9);
            }
            ctx.move_to(text_x, text_y);
            pangocairo::functions::show_layout(ctx, &layout);
        }

        Ok(())
    }

    /// Update hover state based on mouse position
    pub fn update_hover(&mut self, mouse_x: f64, mouse_y: f64) -> bool {
        let old_hover = self.hovered_index;

        self.hovered_index = None;

        for (i, _) in self.users.iter().enumerate() {
            let item_x = self.x + i as f64 * (self.avatar_size + self.spacing);
            let item_y = self.y;

            // Check if mouse is over this avatar (with some padding for the name)
            let hit_width = self.avatar_size;
            let hit_height = self.avatar_size + 28.0;

            if mouse_x >= item_x
                && mouse_x <= item_x + hit_width
                && mouse_y >= item_y
                && mouse_y <= item_y + hit_height
            {
                self.hovered_index = Some(i);
                break;
            }
        }

        self.hovered_index != old_hover
    }

    /// Handle click and return selected username if a user was clicked
    pub fn handle_click(&mut self, click_x: f64, click_y: f64) -> Option<String> {
        for (i, user) in self.users.iter().enumerate() {
            let item_x = self.x + i as f64 * (self.avatar_size + self.spacing);
            let item_y = self.y;

            let hit_width = self.avatar_size;
            let hit_height = self.avatar_size + 28.0;

            if click_x >= item_x
                && click_x <= item_x + hit_width
                && click_y >= item_y
                && click_y <= item_y + hit_height
            {
                self.selected_index = Some(i);
                return Some(user.name.clone());
            }
        }

        None
    }

    /// Check if a point is within the user list bounds
    pub fn contains(&self, x: f64, y: f64) -> bool {
        if self.users.is_empty() {
            return false;
        }

        let total_width = self.users.len() as f64 * (self.avatar_size + self.spacing) - self.spacing;
        let total_height = self.avatar_size + 28.0;

        x >= self.x && x <= self.x + total_width && y >= self.y && y <= self.y + total_height
    }
}

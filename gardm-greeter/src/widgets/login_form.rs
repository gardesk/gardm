//! Login form widget for the greeter
//!
//! Renders username/password fields, login button, and handles keyboard input.

use crate::render::rounded_rectangle;
use crate::theme::Theme;
use anyhow::Result;
use cairo::Context;
use pango::{FontDescription, Layout, Weight};

/// Which field currently has focus
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FocusedField {
    Username,
    Password,
}

/// Login form widget state and rendering
pub struct LoginForm {
    pub username: String,
    pub password: String,
    pub focused_field: FocusedField,
    pub error_message: Option<String>,
    pub info_message: Option<String>,
    pub is_loading: bool,
    pub cursor_visible: bool,

    // Layout dimensions
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl LoginForm {
    /// Create a new login form centered at the specified point
    pub fn new(center_x: f64, center_y: f64) -> Self {
        let width = 400.0;
        let height = 320.0;

        Self {
            username: String::new(),
            password: String::new(),
            focused_field: FocusedField::Username,
            error_message: None,
            info_message: None,
            is_loading: false,
            cursor_visible: true,
            x: center_x - width / 2.0,
            y: center_y - height / 2.0,
            width,
            height,
        }
    }

    /// Render the login form
    pub fn render(&self, ctx: &Context, pango_ctx: &pango::Context, theme: &Theme) -> Result<()> {
        // Background panel
        let bg = &theme.panel_background;
        ctx.set_source_rgba(bg.r, bg.g, bg.b, bg.a);
        rounded_rectangle(ctx, self.x, self.y, self.width, self.height, theme.corner_radius);
        ctx.fill()?;

        // Title
        self.render_title(ctx, pango_ctx, theme)?;

        // Username field
        self.render_input_field(
            ctx,
            pango_ctx,
            theme,
            "Username",
            &self.username,
            self.y + 100.0,
            self.focused_field == FocusedField::Username,
        )?;

        // Password field (masked)
        let masked_password = "•".repeat(self.password.len());
        self.render_input_field(
            ctx,
            pango_ctx,
            theme,
            "Password",
            &masked_password,
            self.y + 170.0,
            self.focused_field == FocusedField::Password,
        )?;

        // Error message
        if let Some(ref msg) = self.error_message {
            let c = &theme.text_error;
            self.render_message(ctx, pango_ctx, theme, msg, (c.r, c.g, c.b))?;
        } else if let Some(ref msg) = self.info_message {
            let c = &theme.text_info;
            self.render_message(ctx, pango_ctx, theme, msg, (c.r, c.g, c.b))?;
        }

        // Login button
        self.render_button(ctx, pango_ctx, theme)?;

        Ok(())
    }

    fn render_title(&self, ctx: &Context, pango_ctx: &pango::Context, theme: &Theme) -> Result<()> {
        let layout = Layout::new(pango_ctx);
        let mut font = FontDescription::new();
        font.set_family(&theme.font_family);
        font.set_size(theme.font_size_title * pango::SCALE);
        font.set_weight(Weight::Bold);
        layout.set_font_description(Some(&font));
        layout.set_text("Welcome");

        let (text_width, _) = layout.pixel_size();

        let c = &theme.text_primary;
        ctx.set_source_rgb(c.r, c.g, c.b);
        ctx.move_to(
            self.x + (self.width - text_width as f64) / 2.0,
            self.y + 30.0,
        );
        pangocairo::functions::show_layout(ctx, &layout);

        Ok(())
    }

    fn render_input_field(
        &self,
        ctx: &Context,
        pango_ctx: &pango::Context,
        theme: &Theme,
        label: &str,
        value: &str,
        y: f64,
        focused: bool,
    ) -> Result<()> {
        let field_x = self.x + 30.0;
        let field_width = self.width - 60.0;
        let field_height = 40.0;

        // Label
        let mut font = FontDescription::new();
        font.set_family(&theme.font_family);
        font.set_size(11 * pango::SCALE);

        let label_layout = Layout::new(pango_ctx);
        label_layout.set_font_description(Some(&font));
        label_layout.set_text(label);

        let c = &theme.text_secondary;
        ctx.set_source_rgba(c.r, c.g, c.b, c.a);
        ctx.move_to(field_x, y - 18.0);
        pangocairo::functions::show_layout(ctx, &label_layout);

        // Input box background
        let bg = if focused {
            &theme.input_background_focused
        } else {
            &theme.input_background
        };
        ctx.set_source_rgba(bg.r, bg.g, bg.b, bg.a);
        rounded_rectangle(ctx, field_x, y, field_width, field_height, 8.0);
        ctx.fill()?;

        // Input box border (focused only)
        if focused {
            let bc = &theme.input_border;
            ctx.set_source_rgba(bc.r, bc.g, bc.b, bc.a);
            rounded_rectangle(ctx, field_x, y, field_width, field_height, 8.0);
            ctx.set_line_width(2.0);
            ctx.stroke()?;
        }

        // Text value
        let tc = &theme.text_primary;
        ctx.set_source_rgb(tc.r, tc.g, tc.b);
        font.set_size(theme.font_size_normal * pango::SCALE);
        let value_layout = Layout::new(pango_ctx);
        value_layout.set_font_description(Some(&font));
        value_layout.set_text(if value.is_empty() { " " } else { value });
        ctx.move_to(field_x + 12.0, y + 10.0);
        pangocairo::functions::show_layout(ctx, &value_layout);

        // Cursor (blinking)
        if focused && self.cursor_visible {
            let (text_width, _) = value_layout.pixel_size();
            let cursor_x = if value.is_empty() {
                field_x + 12.0
            } else {
                field_x + 12.0 + text_width as f64
            };
            ctx.set_source_rgb(tc.r, tc.g, tc.b);
            ctx.rectangle(cursor_x, y + 8.0, 2.0, 24.0);
            ctx.fill()?;
        }

        Ok(())
    }

    fn render_message(
        &self,
        ctx: &Context,
        pango_ctx: &pango::Context,
        theme: &Theme,
        msg: &str,
        color: (f64, f64, f64),
    ) -> Result<()> {
        ctx.set_source_rgb(color.0, color.1, color.2);

        let mut font = FontDescription::new();
        font.set_family(&theme.font_family);
        font.set_size(12 * pango::SCALE);

        let layout = Layout::new(pango_ctx);
        layout.set_font_description(Some(&font));
        layout.set_text(msg);
        layout.set_width((self.width - 60.0) as i32 * pango::SCALE);

        ctx.move_to(self.x + 30.0, self.y + 240.0);
        pangocairo::functions::show_layout(ctx, &layout);

        Ok(())
    }

    fn render_button(&self, ctx: &Context, pango_ctx: &pango::Context, theme: &Theme) -> Result<()> {
        let btn_width = 120.0;
        let btn_height = 36.0;
        let btn_x = self.x + (self.width - btn_width) / 2.0;
        let btn_y = self.y + 275.0;

        // Button background
        let bg = if self.is_loading {
            &theme.button_background_disabled
        } else {
            &theme.button_background
        };
        ctx.set_source_rgba(bg.r, bg.g, bg.b, bg.a);
        rounded_rectangle(ctx, btn_x, btn_y, btn_width, btn_height, 8.0);
        ctx.fill()?;

        // Button text
        let tc = &theme.text_primary;
        ctx.set_source_rgb(tc.r, tc.g, tc.b);
        let mut font = FontDescription::new();
        font.set_family(&theme.font_family);
        font.set_size(theme.font_size_normal * pango::SCALE);
        font.set_weight(Weight::Bold);

        let layout = Layout::new(pango_ctx);
        layout.set_font_description(Some(&font));
        layout.set_text(if self.is_loading { "..." } else { "Login" });

        let (text_w, _) = layout.pixel_size();
        ctx.move_to(btn_x + (btn_width - text_w as f64) / 2.0, btn_y + 8.0);
        pangocairo::functions::show_layout(ctx, &layout);

        Ok(())
    }

    /// Handle a character key press
    pub fn handle_key(&mut self, key: char) {
        if self.is_loading {
            return;
        }

        match self.focused_field {
            FocusedField::Username => self.username.push(key),
            FocusedField::Password => self.password.push(key),
        }
        self.clear_messages();
    }

    /// Handle backspace key
    pub fn handle_backspace(&mut self) {
        if self.is_loading {
            return;
        }

        match self.focused_field {
            FocusedField::Username => {
                self.username.pop();
            }
            FocusedField::Password => {
                self.password.pop();
            }
        }
        self.clear_messages();
    }

    /// Handle tab key (switch focus)
    pub fn handle_tab(&mut self) {
        if self.is_loading {
            return;
        }

        self.focused_field = match self.focused_field {
            FocusedField::Username => FocusedField::Password,
            FocusedField::Password => FocusedField::Username,
        };
    }

    /// Toggle cursor visibility for blinking effect
    pub fn toggle_cursor(&mut self) {
        self.cursor_visible = !self.cursor_visible;
    }

    /// Set error message
    pub fn set_error(&mut self, msg: String) {
        self.error_message = Some(msg);
        self.info_message = None;
    }

    /// Set info message
    pub fn set_info(&mut self, msg: String) {
        self.info_message = Some(msg);
        self.error_message = None;
    }

    /// Clear all messages
    pub fn clear_messages(&mut self) {
        self.error_message = None;
        self.info_message = None;
    }

    /// Clear password field
    pub fn clear_password(&mut self) {
        self.password.clear();
    }

    /// Check if form is ready to submit
    pub fn can_submit(&self) -> bool {
        !self.is_loading && !self.username.is_empty() && !self.password.is_empty()
    }
}

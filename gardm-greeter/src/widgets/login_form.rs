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

    // Cursor positions (character index)
    username_cursor: usize,
    password_cursor: usize,

    // Selection anchor (None = no selection, Some = selection start)
    username_selection: Option<usize>,
    password_selection: Option<usize>,

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
            username_cursor: 0,
            password_cursor: 0,
            username_selection: None,
            password_selection: None,
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
            self.username_cursor,
            self.username_selection,
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
            self.password_cursor,
            self.password_selection,
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
        cursor_pos: usize,
        selection_anchor: Option<usize>,
    ) -> Result<()> {
        let field_x = self.x + 30.0;
        let field_width = self.width - 60.0;
        let field_height = 40.0;
        let text_start_x = field_x + 12.0;

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

        // Set up font for text measurement
        font.set_size(theme.font_size_normal * pango::SCALE);

        // Helper to measure text width
        let measure_text = |s: &str| -> f64 {
            if s.is_empty() {
                return 0.0;
            }
            let layout = Layout::new(pango_ctx);
            layout.set_font_description(Some(&font));
            layout.set_text(s);
            layout.pixel_size().0 as f64
        };

        // Draw selection highlight if there's a selection
        if let Some(anchor) = selection_anchor {
            if anchor != cursor_pos && !value.is_empty() {
                let start = anchor.min(cursor_pos);
                let end = anchor.max(cursor_pos);

                let chars: Vec<char> = value.chars().collect();
                let text_before_start: String = chars.iter().take(start).collect();
                let text_before_end: String = chars.iter().take(end).collect();

                let start_x = text_start_x + measure_text(&text_before_start);
                let end_x = text_start_x + measure_text(&text_before_end);

                // Selection highlight (more pronounced blue)
                ctx.set_source_rgba(0.2, 0.5, 1.0, 0.6);
                ctx.rectangle(start_x, y + 8.0, end_x - start_x, 24.0);
                ctx.fill()?;
            }
        }

        // Text value
        let tc = &theme.text_primary;
        ctx.set_source_rgb(tc.r, tc.g, tc.b);
        let value_layout = Layout::new(pango_ctx);
        value_layout.set_font_description(Some(&font));
        value_layout.set_text(if value.is_empty() { " " } else { value });
        ctx.move_to(text_start_x, y + 10.0);
        pangocairo::functions::show_layout(ctx, &value_layout);

        // Cursor (blinking) - positioned at cursor_pos
        if focused && self.cursor_visible {
            let cursor_x = if value.is_empty() || cursor_pos == 0 {
                text_start_x
            } else {
                let text_before_cursor: String = value.chars().take(cursor_pos).collect();
                text_start_x + measure_text(&text_before_cursor)
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

        // Delete any selected text first
        self.delete_selection();

        match self.focused_field {
            FocusedField::Username => {
                // Insert at cursor position
                let pos = self.username_cursor.min(self.username.chars().count());
                let mut chars: Vec<char> = self.username.chars().collect();
                chars.insert(pos, key);
                self.username = chars.into_iter().collect();
                self.username_cursor = pos + 1;
            }
            FocusedField::Password => {
                let pos = self.password_cursor.min(self.password.chars().count());
                let mut chars: Vec<char> = self.password.chars().collect();
                chars.insert(pos, key);
                self.password = chars.into_iter().collect();
                self.password_cursor = pos + 1;
            }
        }
        self.clear_messages();
    }

    /// Handle backspace key
    pub fn handle_backspace(&mut self) {
        if self.is_loading {
            return;
        }

        // If there's a selection, delete it instead of single char
        if self.delete_selection() {
            self.clear_messages();
            return;
        }

        match self.focused_field {
            FocusedField::Username => {
                if self.username_cursor > 0 {
                    let mut chars: Vec<char> = self.username.chars().collect();
                    chars.remove(self.username_cursor - 1);
                    self.username = chars.into_iter().collect();
                    self.username_cursor -= 1;
                }
            }
            FocusedField::Password => {
                if self.password_cursor > 0 {
                    let mut chars: Vec<char> = self.password.chars().collect();
                    chars.remove(self.password_cursor - 1);
                    self.password = chars.into_iter().collect();
                    self.password_cursor -= 1;
                }
            }
        }
        self.clear_messages();
    }

    /// Handle delete key
    pub fn handle_delete(&mut self) {
        if self.is_loading {
            return;
        }

        // If there's a selection, delete it instead of single char
        if self.delete_selection() {
            self.clear_messages();
            return;
        }

        match self.focused_field {
            FocusedField::Username => {
                let len = self.username.chars().count();
                if self.username_cursor < len {
                    let mut chars: Vec<char> = self.username.chars().collect();
                    chars.remove(self.username_cursor);
                    self.username = chars.into_iter().collect();
                }
            }
            FocusedField::Password => {
                let len = self.password.chars().count();
                if self.password_cursor < len {
                    let mut chars: Vec<char> = self.password.chars().collect();
                    chars.remove(self.password_cursor);
                    self.password = chars.into_iter().collect();
                }
            }
        }
        self.clear_messages();
    }

    /// Handle left arrow key (shift = extend selection)
    pub fn handle_left(&mut self, shift: bool) {
        match self.focused_field {
            FocusedField::Username => {
                if shift && self.username_selection.is_none() {
                    self.username_selection = Some(self.username_cursor);
                } else if !shift {
                    self.username_selection = None;
                }
                if self.username_cursor > 0 {
                    self.username_cursor -= 1;
                }
            }
            FocusedField::Password => {
                if shift && self.password_selection.is_none() {
                    self.password_selection = Some(self.password_cursor);
                } else if !shift {
                    self.password_selection = None;
                }
                if self.password_cursor > 0 {
                    self.password_cursor -= 1;
                }
            }
        }
    }

    /// Handle right arrow key (shift = extend selection)
    pub fn handle_right(&mut self, shift: bool) {
        match self.focused_field {
            FocusedField::Username => {
                if shift && self.username_selection.is_none() {
                    self.username_selection = Some(self.username_cursor);
                } else if !shift {
                    self.username_selection = None;
                }
                if self.username_cursor < self.username.chars().count() {
                    self.username_cursor += 1;
                }
            }
            FocusedField::Password => {
                if shift && self.password_selection.is_none() {
                    self.password_selection = Some(self.password_cursor);
                } else if !shift {
                    self.password_selection = None;
                }
                if self.password_cursor < self.password.chars().count() {
                    self.password_cursor += 1;
                }
            }
        }
    }

    /// Handle Home key - move cursor to beginning (shift = extend selection)
    pub fn handle_home(&mut self, shift: bool) {
        match self.focused_field {
            FocusedField::Username => {
                if shift && self.username_selection.is_none() {
                    self.username_selection = Some(self.username_cursor);
                } else if !shift {
                    self.username_selection = None;
                }
                self.username_cursor = 0;
            }
            FocusedField::Password => {
                if shift && self.password_selection.is_none() {
                    self.password_selection = Some(self.password_cursor);
                } else if !shift {
                    self.password_selection = None;
                }
                self.password_cursor = 0;
            }
        }
    }

    /// Handle End key - move cursor to end (shift = extend selection)
    pub fn handle_end(&mut self, shift: bool) {
        match self.focused_field {
            FocusedField::Username => {
                if shift && self.username_selection.is_none() {
                    self.username_selection = Some(self.username_cursor);
                } else if !shift {
                    self.username_selection = None;
                }
                self.username_cursor = self.username.chars().count();
            }
            FocusedField::Password => {
                if shift && self.password_selection.is_none() {
                    self.password_selection = Some(self.password_cursor);
                } else if !shift {
                    self.password_selection = None;
                }
                self.password_cursor = self.password.chars().count();
            }
        }
    }

    /// Clear any active selection
    pub fn clear_selection(&mut self) {
        self.username_selection = None;
        self.password_selection = None;
    }

    /// Get selection range for current field (start, end) or None
    fn get_selection_range(&self) -> Option<(usize, usize)> {
        match self.focused_field {
            FocusedField::Username => {
                self.username_selection.map(|anchor| {
                    let start = anchor.min(self.username_cursor);
                    let end = anchor.max(self.username_cursor);
                    (start, end)
                })
            }
            FocusedField::Password => {
                self.password_selection.map(|anchor| {
                    let start = anchor.min(self.password_cursor);
                    let end = anchor.max(self.password_cursor);
                    (start, end)
                })
            }
        }
    }

    /// Delete selected text and return true if there was a selection
    fn delete_selection(&mut self) -> bool {
        if let Some((start, end)) = self.get_selection_range() {
            if start != end {
                match self.focused_field {
                    FocusedField::Username => {
                        let mut chars: Vec<char> = self.username.chars().collect();
                        chars.drain(start..end);
                        self.username = chars.into_iter().collect();
                        self.username_cursor = start;
                        self.username_selection = None;
                    }
                    FocusedField::Password => {
                        let mut chars: Vec<char> = self.password.chars().collect();
                        chars.drain(start..end);
                        self.password = chars.into_iter().collect();
                        self.password_cursor = start;
                        self.password_selection = None;
                    }
                }
                return true;
            }
        }
        false
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
        self.password_cursor = 0;
    }

    /// Set username (and move cursor to end)
    pub fn set_username(&mut self, username: String) {
        self.username_cursor = username.chars().count();
        self.username = username;
    }

    /// Check if form is ready to submit
    pub fn can_submit(&self) -> bool {
        !self.is_loading && !self.username.is_empty() && !self.password.is_empty()
    }

    /// Check if a click is on the login button
    pub fn button_contains(&self, click_x: f64, click_y: f64) -> bool {
        let btn_width = 120.0;
        let btn_height = 36.0;
        let btn_x = self.x + (self.width - btn_width) / 2.0;
        let btn_y = self.y + 275.0;

        click_x >= btn_x
            && click_x <= btn_x + btn_width
            && click_y >= btn_y
            && click_y <= btn_y + btn_height
    }

    /// Check if mouse is over an input field (for cursor change)
    pub fn is_over_input(&self, mouse_x: f64, mouse_y: f64) -> bool {
        let field_x = self.x + 30.0;
        let field_width = self.width - 60.0;
        let field_height = 40.0;

        // Username field (y = self.y + 100.0)
        let username_y = self.y + 100.0;
        let over_username = mouse_x >= field_x
            && mouse_x <= field_x + field_width
            && mouse_y >= username_y
            && mouse_y <= username_y + field_height;

        // Password field (y = self.y + 170.0)
        let password_y = self.y + 170.0;
        let over_password = mouse_x >= field_x
            && mouse_x <= field_x + field_width
            && mouse_y >= password_y
            && mouse_y <= password_y + field_height;

        over_username || over_password
    }

    /// Handle click on input field - returns true if click was handled
    pub fn handle_input_click(
        &mut self,
        click_x: f64,
        click_y: f64,
        pango_ctx: &pango::Context,
        font_family: &str,
        font_size: i32,
    ) -> bool {
        let field_x = self.x + 30.0;
        let field_width = self.width - 60.0;
        let field_height = 40.0;
        let text_start_x = field_x + 12.0;

        // Check username field
        let username_y = self.y + 100.0;
        if click_x >= field_x
            && click_x <= field_x + field_width
            && click_y >= username_y
            && click_y <= username_y + field_height
        {
            self.focused_field = FocusedField::Username;
            self.username_cursor = self.calculate_cursor_pos(
                click_x - text_start_x,
                &self.username,
                pango_ctx,
                font_family,
                font_size,
            );
            return true;
        }

        // Check password field
        let password_y = self.y + 170.0;
        if click_x >= field_x
            && click_x <= field_x + field_width
            && click_y >= password_y
            && click_y <= password_y + field_height
        {
            self.focused_field = FocusedField::Password;
            // For password, use masked characters for measurement
            let masked = "•".repeat(self.password.len());
            self.password_cursor = self.calculate_cursor_pos(
                click_x - text_start_x,
                &masked,
                pango_ctx,
                font_family,
                font_size,
            );
            return true;
        }

        false
    }

    /// Calculate cursor position from click x offset
    fn calculate_cursor_pos(
        &self,
        click_offset: f64,
        text: &str,
        pango_ctx: &pango::Context,
        font_family: &str,
        font_size: i32,
    ) -> usize {
        if text.is_empty() || click_offset <= 0.0 {
            return 0;
        }

        let mut font = FontDescription::new();
        font.set_family(font_family);
        font.set_size(font_size * pango::SCALE);

        let chars: Vec<char> = text.chars().collect();
        let mut best_pos = chars.len();
        let mut prev_width = 0.0;

        for i in 0..=chars.len() {
            let substring: String = chars.iter().take(i).collect();
            let layout = Layout::new(pango_ctx);
            layout.set_font_description(Some(&font));
            layout.set_text(&substring);
            let (width, _) = layout.pixel_size();
            let width = width as f64;

            // Check if click is closer to this position or the previous one
            if click_offset < width {
                // Click is between prev_width and width
                let mid = (prev_width + width) / 2.0;
                if click_offset < mid {
                    best_pos = if i > 0 { i - 1 } else { 0 };
                } else {
                    best_pos = i;
                }
                break;
            }
            prev_width = width;
        }

        best_pos
    }
}

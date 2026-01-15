//! Custom SVG-style icons rendered with Cairo
//!
//! Clean, minimal icon designs for power controls.

use cairo::Context;
use std::f64::consts::PI;

/// Icon size (icons are designed for this size and scale proportionally)
pub const ICON_SIZE: f64 = 24.0;

/// Draw a power/shutdown icon
/// A circle with a vertical line at the top (power symbol)
pub fn draw_power(ctx: &Context, x: f64, y: f64, size: f64) {
    let scale = size / ICON_SIZE;
    let cx = x + size / 2.0;
    let cy = y + size / 2.0;
    let radius = 9.0 * scale;
    let stroke_width = 2.0 * scale;

    ctx.set_line_width(stroke_width);
    ctx.set_line_cap(cairo::LineCap::Round);

    // Circle (arc from ~60° to ~300°, leaving gap at top)
    let gap_angle = PI / 6.0; // 30 degrees gap on each side of top
    ctx.arc(cx, cy, radius, PI / 2.0 + gap_angle, 5.0 * PI / 2.0 - gap_angle);
    ctx.stroke().ok();

    // Vertical line at top
    let line_length = 8.0 * scale;
    ctx.move_to(cx, cy - radius + stroke_width);
    ctx.line_to(cx, cy - radius + stroke_width - line_length);
    ctx.stroke().ok();
}

/// Draw a reboot/restart icon
/// Two curved arrows forming a circle
pub fn draw_reboot(ctx: &Context, x: f64, y: f64, size: f64) {
    let scale = size / ICON_SIZE;
    let cx = x + size / 2.0;
    let cy = y + size / 2.0;
    let radius = 8.0 * scale;
    let stroke_width = 2.0 * scale;
    let arrow_size = 4.0 * scale;

    ctx.set_line_width(stroke_width);
    ctx.set_line_cap(cairo::LineCap::Round);
    ctx.set_line_join(cairo::LineJoin::Round);

    // Top arc (right side, going clockwise)
    ctx.arc(cx, cy, radius, -PI * 0.75, PI * 0.1);
    ctx.stroke().ok();

    // Top arrow head (pointing right/down)
    let arrow_x = cx + radius * (PI * 0.1).cos();
    let arrow_y = cy + radius * (PI * 0.1).sin();
    ctx.move_to(arrow_x - arrow_size, arrow_y - arrow_size * 0.5);
    ctx.line_to(arrow_x, arrow_y);
    ctx.line_to(arrow_x - arrow_size * 0.5, arrow_y + arrow_size);
    ctx.stroke().ok();

    // Bottom arc (left side, going clockwise)
    ctx.arc(cx, cy, radius, PI * 0.25, PI * 1.1);
    ctx.stroke().ok();

    // Bottom arrow head (pointing left/up)
    let arrow_x = cx + radius * (PI * 1.1).cos();
    let arrow_y = cy + radius * (PI * 1.1).sin();
    ctx.move_to(arrow_x + arrow_size, arrow_y + arrow_size * 0.5);
    ctx.line_to(arrow_x, arrow_y);
    ctx.line_to(arrow_x + arrow_size * 0.5, arrow_y - arrow_size);
    ctx.stroke().ok();
}

/// Draw a suspend/sleep icon
/// A crescent moon
pub fn draw_suspend(ctx: &Context, x: f64, y: f64, size: f64) {
    let scale = size / ICON_SIZE;
    let cx = x + size / 2.0;
    let cy = y + size / 2.0;
    let outer_radius = 9.0 * scale;
    let inner_radius = 7.0 * scale;
    let offset = 4.0 * scale; // How much the inner circle is offset

    // Create crescent by subtracting inner circle from outer
    // Outer circle (full moon)
    ctx.arc(cx, cy, outer_radius, 0.0, 2.0 * PI);

    // Inner circle (shadow) - offset to create crescent
    ctx.arc_negative(cx + offset, cy - offset * 0.3, inner_radius, 2.0 * PI, 0.0);

    ctx.fill().ok();
}

/// Draw a dropdown/chevron icon
/// A simple downward-pointing chevron
pub fn draw_chevron_down(ctx: &Context, x: f64, y: f64, size: f64) {
    let scale = size / ICON_SIZE;
    let cx = x + size / 2.0;
    let cy = y + size / 2.0;
    let half_width = 5.0 * scale;
    let half_height = 3.0 * scale;
    let stroke_width = 2.0 * scale;

    ctx.set_line_width(stroke_width);
    ctx.set_line_cap(cairo::LineCap::Round);
    ctx.set_line_join(cairo::LineJoin::Round);

    ctx.move_to(cx - half_width, cy - half_height);
    ctx.line_to(cx, cy + half_height);
    ctx.line_to(cx + half_width, cy - half_height);
    ctx.stroke().ok();
}

/// Draw a checkmark icon (for selected session)
pub fn draw_checkmark(ctx: &Context, x: f64, y: f64, size: f64) {
    let scale = size / ICON_SIZE;
    let stroke_width = 2.5 * scale;

    ctx.set_line_width(stroke_width);
    ctx.set_line_cap(cairo::LineCap::Round);
    ctx.set_line_join(cairo::LineJoin::Round);

    // Checkmark shape
    ctx.move_to(x + 4.0 * scale, y + 12.0 * scale);
    ctx.line_to(x + 9.0 * scale, y + 17.0 * scale);
    ctx.line_to(x + 20.0 * scale, y + 6.0 * scale);
    ctx.stroke().ok();
}

/// Draw a user/person icon
pub fn draw_user(ctx: &Context, x: f64, y: f64, size: f64) {
    let scale = size / ICON_SIZE;
    let cx = x + size / 2.0;
    let cy = y + size / 2.0;

    // Head (circle)
    let head_radius = 5.0 * scale;
    let head_y = cy - 3.0 * scale;
    ctx.arc(cx, head_y, head_radius, 0.0, 2.0 * PI);
    ctx.fill().ok();

    // Body (rounded rectangle / shoulders)
    let body_width = 14.0 * scale;
    let body_height = 8.0 * scale;
    let body_y = cy + 4.0 * scale;
    let body_radius = 4.0 * scale;

    ctx.new_sub_path();
    ctx.arc(cx - body_width / 2.0 + body_radius, body_y + body_radius, body_radius, PI, 1.5 * PI);
    ctx.arc(cx + body_width / 2.0 - body_radius, body_y + body_radius, body_radius, 1.5 * PI, 2.0 * PI);
    ctx.line_to(cx + body_width / 2.0, body_y + body_height);
    ctx.line_to(cx - body_width / 2.0, body_y + body_height);
    ctx.close_path();
    ctx.fill().ok();
}

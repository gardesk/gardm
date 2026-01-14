# Sprint 3: Greeter UI

**Goal:** Build a sleek, centered login interface with blurred background using Cairo/Pango rendering.

## Objectives

- Create X11 window that covers the full screen
- Render blurred background image
- Implement centered login form (username, password inputs)
- Add session selector dropdown
- Add power buttons (shutdown, reboot, suspend)
- Handle keyboard input and focus
- Connect UI to daemon via IPC

## Design Reference

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│                         [Blurred Background]                        │
│                                                                     │
│                     ┌───────────────────────┐                       │
│                     │       User Avatar     │                       │
│                     │          👤           │                       │
│                     ├───────────────────────┤                       │
│                     │  Username: [       ]  │                       │
│                     │  Password: [*******]  │                       │
│                     │                       │                       │
│                     │  Session:  [gar    ▼] │                       │
│                     │                       │                       │
│                     │      [ Login ]        │                       │
│                     │                       │                       │
│                     │   Error message here  │                       │
│                     └───────────────────────┘                       │
│                                                                     │
│  12:34 PM                                          [⏻] [↻] [⏾]     │
│  January 14, 2026                                                   │
└─────────────────────────────────────────────────────────────────────┘
```

## Tasks

### 3.1 Window Setup

```rust
// gardm-greeter/src/window.rs

use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::wrapper::ConnectionExt;

pub struct GreeterWindow {
    conn: x11rb::rust_connection::RustConnection,
    screen_num: usize,
    window: Window,
    width: u16,
    height: u16,
    gc: Gcontext,
}

impl GreeterWindow {
    pub fn new() -> anyhow::Result<Self> {
        let (conn, screen_num) = x11rb::connect(None)?;
        let screen = &conn.setup().roots[screen_num];

        let width = screen.width_in_pixels;
        let height = screen.height_in_pixels;
        let root = screen.root;
        let depth = screen.root_depth;
        let visual = screen.root_visual;

        // Create window
        let window = conn.generate_id()?;
        conn.create_window(
            depth,
            window,
            root,
            0, 0,
            width, height,
            0,
            WindowClass::INPUT_OUTPUT,
            visual,
            &CreateWindowAux::new()
                .background_pixel(screen.black_pixel)
                .event_mask(
                    EventMask::EXPOSURE
                        | EventMask::KEY_PRESS
                        | EventMask::BUTTON_PRESS
                        | EventMask::STRUCTURE_NOTIFY
                ),
        )?;

        // Make it fullscreen (bypass WM)
        let net_wm_state = conn.intern_atom(false, b"_NET_WM_STATE")?.reply()?.atom;
        let fullscreen = conn.intern_atom(false, b"_NET_WM_STATE_FULLSCREEN")?.reply()?.atom;
        conn.change_property32(
            PropMode::REPLACE,
            window,
            net_wm_state,
            AtomEnum::ATOM,
            &[fullscreen],
        )?;

        // Override redirect for greeter (no WM decorations)
        conn.change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new().override_redirect(1),
        )?;

        // Create graphics context
        let gc = conn.generate_id()?;
        conn.create_gc(gc, window, &CreateGCAux::new())?;

        conn.map_window(window)?;
        conn.flush()?;

        Ok(Self {
            conn,
            screen_num,
            window,
            width,
            height,
            gc,
        })
    }

    pub fn width(&self) -> u16 { self.width }
    pub fn height(&self) -> u16 { self.height }
    pub fn window(&self) -> Window { self.window }
    pub fn conn(&self) -> &x11rb::rust_connection::RustConnection { &self.conn }
}
```

### 3.2 Cairo Rendering Surface

```rust
// gardm-greeter/src/render.rs

use cairo::{Context, ImageSurface, Format};
use x11rb::protocol::xproto::*;

pub struct Renderer {
    surface: ImageSurface,
    width: i32,
    height: i32,
}

impl Renderer {
    pub fn new(width: u16, height: u16) -> anyhow::Result<Self> {
        let surface = ImageSurface::create(Format::ARgb32, width as i32, height as i32)?;
        Ok(Self {
            surface,
            width: width as i32,
            height: height as i32,
        })
    }

    pub fn context(&self) -> anyhow::Result<Context> {
        Ok(Context::new(&self.surface)?)
    }

    /// Get raw pixel data for X11
    pub fn data(&self) -> Vec<u8> {
        let stride = self.surface.stride() as usize;
        let height = self.height as usize;
        let data = self.surface.data().unwrap();

        // Cairo uses ARGB, X11 uses BGRA - but with same byte order on little-endian
        data[..stride * height].to_vec()
    }

    pub fn width(&self) -> i32 { self.width }
    pub fn height(&self) -> i32 { self.height }
}
```

### 3.3 Background with Blur

```rust
// gardm-greeter/src/background.rs

use image::{RgbaImage, imageops};

/// Load and blur a background image
pub fn load_blurred_background(
    path: &str,
    width: u32,
    height: u32,
    blur_radius: f32,
    brightness: f32,
) -> anyhow::Result<RgbaImage> {
    // Load image
    let img = image::open(path)?.to_rgba8();

    // Scale to screen size (cover mode)
    let scaled = scale_to_cover(&img, width, height);

    // Apply gaussian blur
    let blurred = imageops::blur(&scaled, blur_radius);

    // Adjust brightness (darken for better text contrast)
    let adjusted = adjust_brightness(&blurred, brightness);

    Ok(adjusted)
}

fn scale_to_cover(img: &RgbaImage, target_w: u32, target_h: u32) -> RgbaImage {
    let (src_w, src_h) = img.dimensions();
    let scale = (target_w as f32 / src_w as f32)
        .max(target_h as f32 / src_h as f32);

    let new_w = (src_w as f32 * scale) as u32;
    let new_h = (src_h as f32 * scale) as u32;

    let resized = imageops::resize(img, new_w, new_h, imageops::FilterType::Lanczos3);

    // Crop to center
    let x = (new_w - target_w) / 2;
    let y = (new_h - target_h) / 2;

    imageops::crop_imm(&resized, x, y, target_w, target_h).to_image()
}

fn adjust_brightness(img: &RgbaImage, factor: f32) -> RgbaImage {
    let mut result = img.clone();
    for pixel in result.pixels_mut() {
        pixel[0] = (pixel[0] as f32 * factor).min(255.0) as u8;
        pixel[1] = (pixel[1] as f32 * factor).min(255.0) as u8;
        pixel[2] = (pixel[2] as f32 * factor).min(255.0) as u8;
    }
    result
}
```

### 3.4 Login Form Widget

```rust
// gardm-greeter/src/widgets/login_form.rs

use cairo::Context;
use pango::{FontDescription, Layout};

pub struct LoginForm {
    pub username: String,
    pub password: String,
    pub focused_field: FocusedField,
    pub error_message: Option<String>,
    pub is_loading: bool,

    // Layout
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Clone, Copy, PartialEq)]
pub enum FocusedField {
    Username,
    Password,
}

impl LoginForm {
    pub fn new(screen_width: f64, screen_height: f64) -> Self {
        let width = 400.0;
        let height = 300.0;

        Self {
            username: String::new(),
            password: String::new(),
            focused_field: FocusedField::Username,
            error_message: None,
            is_loading: false,
            x: (screen_width - width) / 2.0,
            y: (screen_height - height) / 2.0,
            width,
            height,
        }
    }

    pub fn render(&self, ctx: &Context, pango_ctx: &pango::Context) -> anyhow::Result<()> {
        // Background panel (semi-transparent)
        ctx.set_source_rgba(0.1, 0.1, 0.1, 0.85);
        rounded_rectangle(ctx, self.x, self.y, self.width, self.height, 16.0);
        ctx.fill()?;

        // Title
        let title_layout = Layout::new(pango_ctx);
        let mut font = FontDescription::new();
        font.set_family("Sans");
        font.set_size(24 * pango::SCALE);
        font.set_weight(pango::Weight::Bold);
        title_layout.set_font_description(Some(&font));
        title_layout.set_text("Welcome");

        ctx.set_source_rgb(1.0, 1.0, 1.0);
        ctx.move_to(self.x + self.width / 2.0 - 50.0, self.y + 30.0);
        pangocairo::show_layout(ctx, &title_layout);

        // Username field
        self.render_input_field(
            ctx, pango_ctx,
            "Username",
            &self.username,
            self.y + 100.0,
            self.focused_field == FocusedField::Username,
            false,
        )?;

        // Password field
        self.render_input_field(
            ctx, pango_ctx,
            "Password",
            &"•".repeat(self.password.len()),
            self.y + 160.0,
            self.focused_field == FocusedField::Password,
            true,
        )?;

        // Error message
        if let Some(ref msg) = self.error_message {
            ctx.set_source_rgb(1.0, 0.3, 0.3);
            let err_layout = Layout::new(pango_ctx);
            font.set_size(12 * pango::SCALE);
            font.set_weight(pango::Weight::Normal);
            err_layout.set_font_description(Some(&font));
            err_layout.set_text(msg);
            ctx.move_to(self.x + 30.0, self.y + 230.0);
            pangocairo::show_layout(ctx, &err_layout);
        }

        // Login button
        self.render_button(ctx, pango_ctx, "Login", self.y + 260.0)?;

        Ok(())
    }

    fn render_input_field(
        &self,
        ctx: &Context,
        pango_ctx: &pango::Context,
        label: &str,
        value: &str,
        y: f64,
        focused: bool,
        _is_password: bool,
    ) -> anyhow::Result<()> {
        let field_x = self.x + 30.0;
        let field_width = self.width - 60.0;
        let field_height = 40.0;

        // Label
        let mut font = FontDescription::new();
        font.set_family("Sans");
        font.set_size(11 * pango::SCALE);

        let label_layout = Layout::new(pango_ctx);
        label_layout.set_font_description(Some(&font));
        label_layout.set_text(label);

        ctx.set_source_rgba(0.8, 0.8, 0.8, 1.0);
        ctx.move_to(field_x, y - 18.0);
        pangocairo::show_layout(ctx, &label_layout);

        // Input box
        if focused {
            ctx.set_source_rgba(0.3, 0.5, 0.8, 1.0);
        } else {
            ctx.set_source_rgba(0.3, 0.3, 0.3, 1.0);
        }
        rounded_rectangle(ctx, field_x, y, field_width, field_height, 8.0);
        ctx.fill()?;

        // Text value
        ctx.set_source_rgb(1.0, 1.0, 1.0);
        font.set_size(14 * pango::SCALE);
        let value_layout = Layout::new(pango_ctx);
        value_layout.set_font_description(Some(&font));
        value_layout.set_text(if value.is_empty() { " " } else { value });
        ctx.move_to(field_x + 10.0, y + 10.0);
        pangocairo::show_layout(ctx, &value_layout);

        // Cursor
        if focused {
            let (text_width, _) = value_layout.pixel_size();
            ctx.set_source_rgb(1.0, 1.0, 1.0);
            ctx.rectangle(field_x + 10.0 + text_width as f64, y + 8.0, 2.0, 24.0);
            ctx.fill()?;
        }

        Ok(())
    }

    fn render_button(
        &self,
        ctx: &Context,
        pango_ctx: &pango::Context,
        text: &str,
        y: f64,
    ) -> anyhow::Result<()> {
        let btn_width = 120.0;
        let btn_height = 36.0;
        let btn_x = self.x + (self.width - btn_width) / 2.0;

        // Button background
        ctx.set_source_rgba(0.2, 0.5, 0.8, 1.0);
        rounded_rectangle(ctx, btn_x, y, btn_width, btn_height, 8.0);
        ctx.fill()?;

        // Button text
        ctx.set_source_rgb(1.0, 1.0, 1.0);
        let mut font = FontDescription::new();
        font.set_family("Sans");
        font.set_size(14 * pango::SCALE);
        font.set_weight(pango::Weight::Bold);

        let layout = Layout::new(pango_ctx);
        layout.set_font_description(Some(&font));
        layout.set_text(text);

        let (text_w, _) = layout.pixel_size();
        ctx.move_to(btn_x + (btn_width - text_w as f64) / 2.0, y + 8.0);
        pangocairo::show_layout(ctx, &layout);

        Ok(())
    }

    pub fn handle_key(&mut self, key: char) {
        match self.focused_field {
            FocusedField::Username => self.username.push(key),
            FocusedField::Password => self.password.push(key),
        }
    }

    pub fn handle_backspace(&mut self) {
        match self.focused_field {
            FocusedField::Username => { self.username.pop(); }
            FocusedField::Password => { self.password.pop(); }
        }
    }

    pub fn handle_tab(&mut self) {
        self.focused_field = match self.focused_field {
            FocusedField::Username => FocusedField::Password,
            FocusedField::Password => FocusedField::Username,
        };
    }
}

fn rounded_rectangle(ctx: &Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let degrees = std::f64::consts::PI / 180.0;
    ctx.new_sub_path();
    ctx.arc(x + w - r, y + r, r, -90.0 * degrees, 0.0 * degrees);
    ctx.arc(x + w - r, y + h - r, r, 0.0 * degrees, 90.0 * degrees);
    ctx.arc(x + r, y + h - r, r, 90.0 * degrees, 180.0 * degrees);
    ctx.arc(x + r, y + r, r, 180.0 * degrees, 270.0 * degrees);
    ctx.close_path();
}
```

### 3.5 Main Event Loop

```rust
// gardm-greeter/src/main.rs

use x11rb::protocol::Event;

fn main() -> anyhow::Result<()> {
    let window = GreeterWindow::new()?;
    let renderer = Renderer::new(window.width(), window.height())?;
    let mut form = LoginForm::new(window.width() as f64, window.height() as f64);

    // Load background
    let background = load_blurred_background(
        "/usr/share/gardm/backgrounds/default.jpg",
        window.width() as u32,
        window.height() as u32,
        20.0,
        0.7,
    )?;

    // Connect to daemon
    let mut daemon = DaemonClient::connect()?;

    loop {
        // Render frame
        {
            let ctx = renderer.context()?;
            render_background(&ctx, &background)?;
            form.render(&ctx, &pango_ctx)?;
        }

        // Copy to X11
        window.put_image(&renderer.data())?;

        // Handle events
        let event = window.conn().wait_for_event()?;
        match event {
            Event::Expose(_) => {
                // Redraw handled above
            }
            Event::KeyPress(e) => {
                match e.detail {
                    9 => break,  // Escape - exit (for testing)
                    36 => {      // Enter - submit
                        if form.focused_field == FocusedField::Password {
                            // Authenticate
                            daemon.create_session(&form.username)?;
                            match daemon.authenticate(&form.password)? {
                                Response::Success => {
                                    daemon.start_session(&["gar-session.sh"])?;
                                    break;
                                }
                                Response::AuthError { message } => {
                                    form.error_message = Some(message);
                                }
                                _ => {}
                            }
                        } else {
                            form.handle_tab();
                        }
                    }
                    23 => form.handle_tab(),  // Tab
                    22 => form.handle_backspace(),  // Backspace
                    _ => {
                        // Regular key
                        if let Some(c) = keycode_to_char(e.detail, e.state) {
                            form.handle_key(c);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}
```

## Acceptance Criteria

1. Greeter displays fullscreen with blurred background
2. Login form is centered and styled
3. Keyboard input works for username/password
4. Tab switches between fields
5. Enter submits form
6. Error messages display correctly
7. Successful auth triggers session start

## Pitfalls to Avoid

1. **X11 keycodes vary by layout** - use XKB for proper key mapping
2. **Cairo surface format** - ensure ARGB matches X11 expectations
3. **Font rendering** - initialize Pango context correctly
4. **Fullscreen on all monitors** - may need per-monitor handling
5. **Input focus** - greeter window must grab keyboard
6. **Memory leaks** - Cairo contexts need proper cleanup

## Testing

```bash
# Test in Xephyr
Xephyr -br -ac -noreset -screen 1280x720 :1 &
DISPLAY=:1 ./target/release/gardm-greeter

# Test keyboard input
# Test form submission
# Test error display
```

## Dependencies for This Sprint

```toml
# gardm-greeter/Cargo.toml
[dependencies]
x11rb = "0.13"
cairo-rs = { version = "0.18", features = ["png"] }
pango = "0.18"
pangocairo = "0.18"
image = "0.24"
```

## Next Sprint

Sprint 4 will add garbg integration for seamless wallpaper sync.

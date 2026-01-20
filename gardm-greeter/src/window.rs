//! X11 window management for the greeter
//!
//! Creates a fullscreen window for displaying the login interface.

use anyhow::{Context, Result};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

/// Standard X11 cursor font glyphs
mod cursor_glyphs {
    pub const XC_LEFT_PTR: u16 = 68;   // Default arrow cursor
    pub const XC_XTERM: u16 = 152;     // I-beam text cursor
    pub const XC_HAND2: u16 = 60;      // Pointing hand cursor
}

/// Cursor types available for the greeter
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CursorType {
    Default,
    Text,
    Pointer,
}

/// Greeter window wrapping X11 connection and window handle
pub struct GreeterWindow {
    conn: RustConnection,
    screen_num: usize,
    window: Window,
    gc: Gcontext,
    width: u16,
    height: u16,
    depth: u8,
    // Cursors
    cursor_default: Cursor,
    cursor_text: Cursor,
    cursor_pointer: Cursor,
    current_cursor: CursorType,
}

impl GreeterWindow {
    /// Create a new fullscreen greeter window
    pub fn new() -> Result<Self> {
        let (conn, screen_num) = x11rb::connect(None).context("Failed to connect to X server")?;

        let screen = &conn.setup().roots[screen_num];
        let width = screen.width_in_pixels;
        let height = screen.height_in_pixels;
        let root = screen.root;
        let depth = screen.root_depth;
        let visual = screen.root_visual;

        // Clear the root window to black to hide any leftover content from previous session
        // garbg sets the root background using a pixmap, so we must clear that first
        // BackPixmap::NONE (0) removes any background pixmap, then background_pixel takes effect
        conn.change_window_attributes(
            root,
            &ChangeWindowAttributesAux::new()
                .background_pixmap(x11rb::NONE)  // Remove any pixmap (e.g., from garbg)
                .background_pixel(screen.black_pixel),
        )?;
        conn.clear_area(false, root, 0, 0, width, height)?;
        conn.flush()?;

        // Create window
        let window = conn.generate_id().context("Failed to generate window ID")?;
        conn.create_window(
            depth,
            window,
            root,
            0,
            0,
            width,
            height,
            0,
            WindowClass::INPUT_OUTPUT,
            visual,
            &CreateWindowAux::new()
                .background_pixel(screen.black_pixel)
                .event_mask(
                    EventMask::EXPOSURE
                        | EventMask::KEY_PRESS
                        | EventMask::KEY_RELEASE
                        | EventMask::BUTTON_PRESS
                        | EventMask::BUTTON_RELEASE
                        | EventMask::POINTER_MOTION
                        | EventMask::STRUCTURE_NOTIFY
                        | EventMask::FOCUS_CHANGE,
                ),
        )
        .context("Failed to create window")?;

        // Set fullscreen hint
        let net_wm_state = conn
            .intern_atom(false, b"_NET_WM_STATE")
            .context("Failed to intern _NET_WM_STATE")?
            .reply()
            .context("Failed to get _NET_WM_STATE reply")?
            .atom;

        let fullscreen = conn
            .intern_atom(false, b"_NET_WM_STATE_FULLSCREEN")
            .context("Failed to intern fullscreen atom")?
            .reply()
            .context("Failed to get fullscreen atom reply")?
            .atom;

        conn.change_property32(PropMode::REPLACE, window, net_wm_state, AtomEnum::ATOM, &[
            fullscreen,
        ])
        .context("Failed to set fullscreen property")?;

        // Override redirect - no window manager decorations
        conn.change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new().override_redirect(1),
        )
        .context("Failed to set override redirect")?;

        // Create graphics context for image rendering
        let gc = conn.generate_id().context("Failed to generate GC ID")?;
        conn.create_gc(gc, window, &CreateGCAux::new())
            .context("Failed to create GC")?;

        // Load cursor font and create cursors
        let cursor_font = conn.generate_id().context("Failed to generate font ID")?;
        conn.open_font(cursor_font, b"cursor")
            .context("Failed to open cursor font")?;

        let cursor_default = conn.generate_id().context("Failed to generate cursor ID")?;
        conn.create_glyph_cursor(
            cursor_default,
            cursor_font,
            cursor_font,
            cursor_glyphs::XC_LEFT_PTR,
            cursor_glyphs::XC_LEFT_PTR + 1,
            0xFFFF, 0xFFFF, 0xFFFF, // White foreground
            0, 0, 0,                 // Black background
        )
        .context("Failed to create default cursor")?;

        let cursor_text = conn.generate_id().context("Failed to generate cursor ID")?;
        conn.create_glyph_cursor(
            cursor_text,
            cursor_font,
            cursor_font,
            cursor_glyphs::XC_XTERM,
            cursor_glyphs::XC_XTERM + 1,
            0xFFFF, 0xFFFF, 0xFFFF,
            0, 0, 0,
        )
        .context("Failed to create text cursor")?;

        let cursor_pointer = conn.generate_id().context("Failed to generate cursor ID")?;
        conn.create_glyph_cursor(
            cursor_pointer,
            cursor_font,
            cursor_font,
            cursor_glyphs::XC_HAND2,
            cursor_glyphs::XC_HAND2 + 1,
            0xFFFF, 0xFFFF, 0xFFFF,
            0, 0, 0,
        )
        .context("Failed to create pointer cursor")?;

        conn.close_font(cursor_font).context("Failed to close cursor font")?;

        // Set initial cursor
        conn.change_window_attributes(window, &ChangeWindowAttributesAux::new().cursor(cursor_default))
            .context("Failed to set initial cursor")?;

        // Map and flush
        conn.map_window(window).context("Failed to map window")?;
        conn.flush().context("Failed to flush X connection")?;

        // Grab keyboard focus
        conn.set_input_focus(InputFocus::POINTER_ROOT, window, x11rb::CURRENT_TIME)
            .context("Failed to set input focus")?;
        conn.flush()?;

        tracing::info!(width, height, "Created greeter window");

        Ok(Self {
            conn,
            screen_num,
            window,
            gc,
            width,
            height,
            depth,
            cursor_default,
            cursor_text,
            cursor_pointer,
            current_cursor: CursorType::Default,
        })
    }

    /// Get window width
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Get window height
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Get the X11 connection
    pub fn conn(&self) -> &RustConnection {
        &self.conn
    }

    /// Get the window ID
    pub fn window(&self) -> Window {
        self.window
    }

    /// Get the root window ID
    pub fn root(&self) -> Window {
        self.conn.setup().roots[self.screen_num].root
    }

    /// Get the screen number
    pub fn screen_num(&self) -> usize {
        self.screen_num
    }

    /// Put an ARGB image to the window
    /// Splits large images into chunks to avoid exceeding X11 request limits
    pub fn put_image(&self, data: &[u8]) -> Result<()> {
        let bytes_per_row = self.width as usize * 4;
        let total_rows = self.height as usize;

        // X11 max request is typically 4MB, use 1MB chunks to be safe
        const MAX_CHUNK_BYTES: usize = 1024 * 1024;
        let rows_per_chunk = (MAX_CHUNK_BYTES / bytes_per_row).max(1);

        let mut y_offset: i16 = 0;
        let mut remaining_rows = total_rows;
        let mut data_offset = 0;

        while remaining_rows > 0 {
            let chunk_rows = remaining_rows.min(rows_per_chunk);
            let chunk_bytes = chunk_rows * bytes_per_row;
            let chunk_data = &data[data_offset..data_offset + chunk_bytes];

            self.conn
                .put_image(
                    ImageFormat::Z_PIXMAP,
                    self.window,
                    self.gc,
                    self.width,
                    chunk_rows as u16,
                    0,
                    y_offset,
                    0,
                    self.depth,
                    chunk_data,
                )
                .context("Failed to put image chunk")?;

            y_offset += chunk_rows as i16;
            remaining_rows -= chunk_rows;
            data_offset += chunk_bytes;
        }

        self.conn.flush().context("Failed to flush after put_image")?;
        Ok(())
    }

    /// Wait for and return the next X11 event
    pub fn wait_for_event(&self) -> Result<x11rb::protocol::Event> {
        self.conn
            .wait_for_event()
            .context("Failed to wait for X11 event")
    }

    /// Poll for event without blocking
    pub fn poll_for_event(&self) -> Result<Option<x11rb::protocol::Event>> {
        self.conn
            .poll_for_event()
            .context("Failed to poll for X11 event")
    }

    /// Set the cursor type (only updates if different from current)
    pub fn set_cursor(&mut self, cursor_type: CursorType) {
        if self.current_cursor == cursor_type {
            return;
        }

        let cursor = match cursor_type {
            CursorType::Default => self.cursor_default,
            CursorType::Text => self.cursor_text,
            CursorType::Pointer => self.cursor_pointer,
        };

        let _ = self.conn.change_window_attributes(
            self.window,
            &ChangeWindowAttributesAux::new().cursor(cursor),
        );
        let _ = self.conn.flush();
        self.current_cursor = cursor_type;
    }
}

impl Drop for GreeterWindow {
    fn drop(&mut self) {
        let _ = self.conn.destroy_window(self.window);
        let _ = self.conn.flush();
    }
}

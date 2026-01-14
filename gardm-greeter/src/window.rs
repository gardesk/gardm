//! X11 window management for the greeter
//!
//! Creates a fullscreen window for displaying the login interface.

use anyhow::{Context, Result};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

/// Greeter window wrapping X11 connection and window handle
pub struct GreeterWindow {
    conn: RustConnection,
    screen_num: usize,
    window: Window,
    gc: Gcontext,
    width: u16,
    height: u16,
    depth: u8,
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

    /// Put an ARGB image to the window
    pub fn put_image(&self, data: &[u8]) -> Result<()> {
        self.conn
            .put_image(
                ImageFormat::Z_PIXMAP,
                self.window,
                self.gc,
                self.width,
                self.height,
                0,
                0,
                0,
                self.depth,
                data,
            )
            .context("Failed to put image")?;

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
}

impl Drop for GreeterWindow {
    fn drop(&mut self) {
        let _ = self.conn.destroy_window(self.window);
        let _ = self.conn.flush();
    }
}

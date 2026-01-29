//! X11 window management for the greeter
//!
//! Creates a fullscreen window for displaying the login interface.

use anyhow::{Context, Result};
use x11rb::connection::Connection;
use x11rb::protocol::xkb::{self, ConnectionExt as XkbConnectionExt};
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
        // garbg sets the root background using a pixmap AND stores the pixmap ID in
        // _XROOTPMAP_ID and ESETROOT_PMAP_ID atoms. We must clear all of these.

        // First, clear the root pixmap atoms that garbg sets
        // This prevents compositors from reading stale pixmap references
        let xrootpmap_atom = conn
            .intern_atom(false, b"_XROOTPMAP_ID")?
            .reply()
            .map(|r| r.atom)
            .unwrap_or(x11rb::NONE);
        let esetroot_atom = conn
            .intern_atom(false, b"ESETROOT_PMAP_ID")?
            .reply()
            .map(|r| r.atom)
            .unwrap_or(x11rb::NONE);

        // Try to free the old pixmap if it exists (to avoid memory leak)
        if xrootpmap_atom != x11rb::NONE {
            if let Ok(reply) = conn.get_property(false, root, xrootpmap_atom, AtomEnum::PIXMAP, 0, 1)?.reply() {
                if reply.format == 32 && !reply.value.is_empty() {
                    let pixmap_id = u32::from_ne_bytes([
                        reply.value[0], reply.value[1], reply.value[2], reply.value[3]
                    ]);
                    if pixmap_id != 0 {
                        let _ = conn.free_pixmap(pixmap_id);
                    }
                }
            }
            // Delete the property
            let _ = conn.delete_property(root, xrootpmap_atom);
        }
        if esetroot_atom != x11rb::NONE {
            let _ = conn.delete_property(root, esetroot_atom);
        }

        // Now clear the window's background pixmap attribute and set solid color
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

        // Sync X11 keyboard lock state with kernel state
        // X11 doesn't query kernel state at startup, so we must do it manually
        sync_keyboard_locks(&conn);

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

/// Sync X11 keyboard lock state with kernel state
///
/// X11/Xorg doesn't query the kernel's keyboard lock state at startup - it only
/// tracks state changes from key events. When returning from a Wayland session,
/// X11 starts fresh and doesn't know that caps/num lock was already active.
///
/// This function queries the kernel's lock state via evdev and sets X11's XKB
/// state to match, ensuring the greeter's keyboard behavior matches the
/// physical keyboard state.
fn sync_keyboard_locks(conn: &RustConnection) {

    // Initialize XKB extension
    if let Err(e) = conn.xkb_use_extension(1, 0) {
        tracing::warn!("Failed to initialize XKB: {}", e);
        return;
    }

    // Find a keyboard device and query its lock state
    let (caps_on, num_on) = match query_kernel_lock_state() {
        Some(state) => state,
        None => {
            tracing::debug!("Could not query kernel keyboard state");
            return;
        }
    };

    tracing::info!("Kernel keyboard state: caps_lock={}, num_lock={}", caps_on, num_on);

    // Build modifier mask to match kernel state
    let mut mod_locks = ModMask::from(0u16);
    if caps_on {
        mod_locks |= ModMask::LOCK;
    }
    if num_on {
        mod_locks |= ModMask::M2;
    }

    // Set X11 XKB state to match kernel
    let affect_locks = ModMask::LOCK | ModMask::M2;
    match conn.xkb_latch_lock_state(
        xkb::ID::USE_CORE_KBD.into(),
        affect_locks,
        mod_locks,
        false,
        xkb::Group::M1,
        ModMask::from(0u16),
        false,
        0u16,
    ) {
        Ok(_) => {
            let _ = conn.flush();
            tracing::debug!("Synced X11 lock state with kernel");
        }
        Err(e) => {
            tracing::warn!("Failed to sync keyboard locks: {}", e);
        }
    }
}

/// Query kernel keyboard lock state via evdev EVIOCGLED ioctl
fn query_kernel_lock_state() -> Option<(bool, bool)> {
    use std::fs::{self, File};
    use std::os::unix::io::AsRawFd;

    // evdev LED indices
    const LED_NUML: u8 = 0;
    const LED_CAPSL: u8 = 1;

    // EVIOCGLED ioctl - read LED state bitmap
    // _IOR('E', 0x19, len) where len is LED_MAX/8+1 bytes
    // For typical keyboards, we just need 1 byte
    const EVIOCGLED_1: libc::c_ulong = 0x80014519;

    // Find keyboard devices
    let input_dir = fs::read_dir("/dev/input").ok()?;

    for entry in input_dir.flatten() {
        let path = entry.path();
        let name = path.file_name()?.to_str()?;

        // Only check event devices
        if !name.starts_with("event") {
            continue;
        }

        // Try to open and query
        if let Ok(file) = File::open(&path) {
            let mut leds: u8 = 0;
            let fd = file.as_raw_fd();

            // Query LED state
            let ret = unsafe {
                libc::ioctl(fd, EVIOCGLED_1, &mut leds as *mut u8)
            };

            if ret >= 0 {
                let caps_on = (leds & (1 << LED_CAPSL)) != 0;
                let num_on = (leds & (1 << LED_NUML)) != 0;
                return Some((caps_on, num_on));
            }
        }
    }

    None
}

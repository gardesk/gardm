//! gardm-greeter - gar display manager greeter
//!
//! Graphical login UI that communicates with gardmd.

mod background;
mod keyboard;
mod render;
mod widgets;
mod window;

use anyhow::{Context, Result};
use gardm_ipc::{Client, Request, Response};
use std::time::{Duration, Instant};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::ConnectionExt;
use x11rb::protocol::Event;

use background::{load_blurred_background, render_to_cairo, solid_background};
use keyboard::{keycode_to_char, keycodes};
use render::Renderer;
use widgets::{FocusedField, LoginForm};
use window::GreeterWindow;

/// Default background image path
const DEFAULT_BACKGROUND: &str = "/usr/share/gardm/backgrounds/default.jpg";

/// Cursor blink interval
const CURSOR_BLINK_MS: u64 = 500;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("gardm-greeter starting");

    // Create X11 window
    let window = GreeterWindow::new().context("Failed to create window")?;
    let width = window.width();
    let height = window.height();
    tracing::info!(width, height, "Window created");

    // Create renderer
    let mut renderer = Renderer::new(width, height).context("Failed to create renderer")?;

    // Load background image (with fallback to solid color)
    let background = match load_blurred_background(
        DEFAULT_BACKGROUND,
        width as u32,
        height as u32,
        25.0, // blur radius
        0.6,  // brightness (darken for contrast)
    ) {
        Ok(bg) => {
            tracing::info!("Loaded background image");
            bg
        }
        Err(e) => {
            tracing::warn!("Failed to load background: {}, using solid color", e);
            solid_background(width as u32, height as u32, 30, 30, 40)
        }
    };

    // Create login form
    let mut form = LoginForm::new(width as f64, height as f64);

    // Connect to daemon
    let mut client = Client::connect().await.context("Failed to connect to gardmd")?;
    tracing::info!("Connected to gardmd");

    // Fetch available sessions
    let sessions = match client.request(&Request::ListSessions).await? {
        Response::Sessions { sessions } => sessions,
        _ => Vec::new(),
    };
    tracing::debug!(?sessions, "Available sessions");

    // Get default session command
    let default_session = sessions.first().map(|s| s.exec.clone()).unwrap_or_else(|| {
        "gar-session.sh".to_string()
    });

    // Create Pango context for text rendering
    let pango_ctx = pangocairo::functions::create_context(&renderer.context()?);

    // Timing for cursor blink
    let mut last_cursor_toggle = Instant::now();

    // Main event loop
    tracing::info!("Entering main loop");
    loop {
        // Toggle cursor blink
        if last_cursor_toggle.elapsed() >= Duration::from_millis(CURSOR_BLINK_MS) {
            form.toggle_cursor();
            last_cursor_toggle = Instant::now();
        }

        // Render frame
        {
            let ctx = renderer.context()?;

            // Draw background
            render_to_cairo(&ctx, &background)?;

            // Draw login form
            form.render(&ctx, &pango_ctx)?;
        }

        // Copy rendered frame to X11 window
        let data = renderer.data()?;
        window.put_image(&data)?;

        // Poll for X11 events (non-blocking)
        while let Some(event) = window.poll_for_event()? {
            match event {
                Event::Expose(_) => {
                    // Already rendering every frame
                }

                Event::KeyPress(e) => {
                    match e.detail {
                        keycodes::ESCAPE => {
                            tracing::info!("Escape pressed, exiting");
                            return Ok(());
                        }

                        keycodes::RETURN => {
                            if form.focused_field == FocusedField::Password && form.can_submit() {
                                // Attempt login
                                handle_login(&mut client, &mut form, &default_session).await?;
                            } else if form.focused_field == FocusedField::Username {
                                form.handle_tab();
                            }
                        }

                        keycodes::TAB => {
                            form.handle_tab();
                        }

                        keycodes::BACKSPACE => {
                            form.handle_backspace();
                        }

                        _ => {
                            // Regular character key
                            if let Some(c) = keycode_to_char(e.detail, e.state) {
                                form.handle_key(c);
                            }
                        }
                    }
                }

                Event::FocusOut(_) => {
                    // Regrab focus if we lose it
                    let _ = window.conn().set_input_focus(
                        x11rb::protocol::xproto::InputFocus::POINTER_ROOT,
                        window.window(),
                        x11rb::CURRENT_TIME,
                    );
                    let _ = window.conn().flush();
                }

                _ => {}
            }
        }

        // Small sleep to avoid busy-spinning
        std::thread::sleep(Duration::from_millis(16)); // ~60fps
    }
}

/// Handle login attempt
async fn handle_login(client: &mut Client, form: &mut LoginForm, default_session: &str) -> Result<()> {
    form.is_loading = true;
    form.clear_messages();

    // Create session for user
    tracing::info!(username = %form.username, "Creating auth session");
    let response = client
        .request(&Request::CreateSession {
            username: form.username.clone(),
        })
        .await?;

    match response {
        Response::AuthPrompt { prompt, .. } => {
            tracing::debug!(prompt, "Got auth prompt");
        }
        Response::Error { message } => {
            tracing::warn!(message, "Session creation failed");
            form.set_error(message);
            form.is_loading = false;
            return Ok(());
        }
        _ => {}
    }

    // Send password
    tracing::debug!("Sending password");
    let response = client
        .request(&Request::Authenticate {
            response: form.password.clone(),
        })
        .await?;

    match response {
        Response::Success => {
            tracing::info!("Authentication successful");
            form.set_info("Starting session...".to_string());

            // Start session
            let session_cmd = vec![default_session.to_string()];
            let response = client
                .request(&Request::StartSession {
                    cmd: session_cmd,
                    env: vec![],
                })
                .await?;

            match response {
                Response::Success => {
                    tracing::info!("Session started, greeter will exit");
                    // The daemon will kill us, but exit cleanly just in case
                    std::process::exit(0);
                }
                Response::Error { message } => {
                    tracing::error!(message, "Failed to start session");
                    form.set_error(message);
                }
                _ => {
                    form.set_error("Unexpected response".to_string());
                }
            }
        }
        Response::AuthError { message } => {
            tracing::warn!(message, "Authentication failed");
            form.set_error(message);
            form.clear_password();
        }
        Response::AuthPrompt { prompt, .. } => {
            // PAM wants more input (e.g., OTP)
            form.set_info(prompt);
        }
        Response::AuthInfo { message } => {
            form.set_info(message);
        }
        Response::Error { message } => {
            form.set_error(message);
        }
        _ => {
            form.set_error("Unexpected response".to_string());
        }
    }

    form.is_loading = false;
    Ok(())
}

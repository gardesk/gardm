//! gardm-greeter - gar display manager greeter
//!
//! Graphical login UI that communicates with gardmd.

mod avatar;
mod background;
mod config;
mod garbg;
mod icons;
mod keyboard;
mod monitors;
mod render;
mod theme;
mod transition;
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
use config::GreeterConfig;
use garbg::WallpaperResolver;
use keyboard::{keycode_to_char, keycodes};
use monitors::{fallback_config, MonitorConfig};
use render::Renderer;
use transition::{render_with_fade, FadeOutTransition};
use widgets::{FocusedField, LoginForm, PowerAction, PowerButtons, SessionSelector, UserList};
use window::GreeterWindow;

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

    // Load configuration
    let config = GreeterConfig::load().unwrap_or_default();
    tracing::debug!(?config, "Greeter configuration");

    // Build theme from config with accessibility options
    let theme = config.build_theme();
    tracing::debug!(
        high_contrast = config.accessibility.high_contrast,
        large_text = config.accessibility.large_text,
        reduce_motion = config.accessibility.reduce_motion,
        "Theme built"
    );

    // Create X11 window
    let window = GreeterWindow::new().context("Failed to create window")?;
    let width = window.width();
    let height = window.height();
    tracing::info!(width, height, "Window created");

    // Detect monitors using RandR
    let monitor_config = MonitorConfig::detect(window.conn(), window.root())
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to detect monitors: {}, using fallback", e);
            fallback_config(width, height)
        });

    // Get the primary monitor for UI positioning
    let primary = monitor_config.primary_or_first().cloned().unwrap_or_else(|| {
        monitors::Monitor {
            x: 0,
            y: 0,
            width,
            height,
            primary: true,
            name: "fallback".to_string(),
        }
    });

    let center_x = primary.center_x();
    let center_y = primary.center_y();
    tracing::info!(
        monitor = %primary.name,
        center_x,
        center_y,
        "UI centered on primary monitor"
    );

    // Create renderer for the full virtual screen
    let mut renderer = Renderer::new(width, height).context("Failed to create renderer")?;

    // Resolve wallpaper using garbg integration
    let wallpaper_path = if config.garbg.enabled {
        let resolver = WallpaperResolver::new(&config.garbg.fallback);
        resolver.resolve(None)
    } else {
        config.garbg.fallback.clone()
    };

    // Load background image (with fallback to solid color)
    let background = match load_blurred_background(
        &wallpaper_path,
        width as u32,
        height as u32,
        config.visual.blur_radius,
        config.visual.brightness,
    ) {
        Ok(bg) => {
            tracing::info!(path = %wallpaper_path, "Loaded background image");
            bg
        }
        Err(e) => {
            tracing::warn!("Failed to load background: {}, using solid color", e);
            solid_background(width as u32, height as u32, 30, 30, 40)
        }
    };

    // Create login form centered on primary monitor
    let mut form = LoginForm::new(center_x, center_y);

    // Connect to daemon
    let mut client = Client::connect().await.context("Failed to connect to gardmd")?;
    tracing::info!("Connected to gardmd");

    // Fetch available sessions
    let sessions = match client.request(&Request::ListSessions).await? {
        Response::Sessions { sessions } => sessions,
        _ => Vec::new(),
    };
    tracing::debug!(?sessions, "Available sessions");

    // Fetch available users
    let users = match client.request(&Request::ListUsers).await? {
        Response::Users { users } => users,
        _ => Vec::new(),
    };
    tracing::debug!(count = users.len(), "Available users");

    // Create user list centered on primary monitor (above login form)
    let mut user_list = UserList::new(users, center_x, center_y);

    // Create session selector (positioned below login form on primary monitor)
    let selector_width = 200.0;
    let selector_x = center_x - selector_width / 2.0;
    let selector_y = center_y + 180.0; // Below the login form
    let mut session_selector = SessionSelector::new(sessions, selector_x, selector_y, selector_width);

    // Create power buttons (bottom-right corner of primary monitor)
    let mut power_buttons = PowerButtons::new(
        primary.x as f64,
        primary.y as f64,
        primary.width as f64,
        primary.height as f64,
    );

    // Create Pango context for text rendering
    let pango_ctx = pangocairo::functions::create_context(&renderer.context()?);

    // Timing for cursor blink
    let mut last_cursor_toggle = Instant::now();

    // Fade transition (None until login succeeds)
    let mut fade_transition: Option<FadeOutTransition> = None;

    // Mouse position tracking
    let mut mouse_x: f64 = 0.0;
    let mut mouse_y: f64 = 0.0;

    // Main event loop
    tracing::info!("Entering main loop");
    loop {
        // Check if fade transition is complete
        if let Some(ref fade) = fade_transition {
            if fade.is_complete() {
                tracing::info!("Fade complete, exiting greeter");
                std::process::exit(0);
            }
        }

        // Toggle cursor blink (only when not fading)
        if fade_transition.is_none()
            && last_cursor_toggle.elapsed() >= Duration::from_millis(CURSOR_BLINK_MS)
        {
            form.toggle_cursor();
            last_cursor_toggle = Instant::now();
        }

        // Render frame
        {
            let ctx = renderer.context()?;

            // Draw background (always full opacity)
            render_to_cairo(&ctx, &background)?;

            // Draw UI elements (with fade if transitioning)
            let opacity = fade_transition
                .as_ref()
                .map(|f| f.opacity())
                .unwrap_or(1.0);

            render_with_fade(&ctx, opacity, |ctx| {
                // User list (above login form)
                user_list.render(ctx, &pango_ctx, &theme)?;

                // Login form
                form.render(ctx, &pango_ctx, &theme)?;

                // Session selector
                session_selector.render(ctx, &pango_ctx, &theme)?;

                // Power buttons
                power_buttons.render(ctx)?;

                Ok(())
            })?;
        }

        // Copy rendered frame to X11 window
        let data = renderer.data()?;
        window.put_image(&data)?;

        // Skip event handling during fade
        if fade_transition.is_some() {
            std::thread::sleep(Duration::from_millis(16));
            continue;
        }

        // Poll for X11 events (non-blocking)
        while let Some(event) = window.poll_for_event()? {
            match event {
                Event::Expose(_) => {
                    // Already rendering every frame
                }

                Event::MotionNotify(e) => {
                    mouse_x = e.event_x as f64;
                    mouse_y = e.event_y as f64;

                    // Update hover states
                    power_buttons.update_hover(mouse_x, mouse_y);
                    session_selector.update_hover(mouse_x, mouse_y);
                    user_list.update_hover(mouse_x, mouse_y);
                }

                Event::ButtonPress(e) => {
                    let click_x = e.event_x as f64;
                    let click_y = e.event_y as f64;

                    // Check user list clicks first
                    if let Some(username) = user_list.handle_click(click_x, click_y) {
                        tracing::info!(username, "User selected from list");
                        form.username = username;
                        form.password.clear();
                        form.focused_field = FocusedField::Password;
                        form.clear_messages();
                    }
                    // Check power buttons
                    else if let Some(action) = power_buttons.handle_click(click_x, click_y) {
                        handle_power_action(&mut client, action).await?;
                    }
                    // Check session selector
                    else if session_selector.button_contains(click_x, click_y) {
                        session_selector.toggle();
                    } else if session_selector.is_expanded() {
                        if session_selector.handle_dropdown_click(click_x, click_y) {
                            // Selection made
                            tracing::info!(
                                session = ?session_selector.selected(),
                                "Session selected"
                            );
                        } else if !session_selector.contains(click_x, click_y) {
                            // Click outside dropdown - close it
                            session_selector.close();
                        }
                    }
                }

                Event::KeyPress(e) => {
                    // Close dropdown on any key press
                    if session_selector.is_expanded() {
                        session_selector.close();
                    }

                    match e.detail {
                        keycodes::ESCAPE => {
                            tracing::info!("Escape pressed, exiting");
                            return Ok(());
                        }

                        keycodes::RETURN => {
                            if form.focused_field == FocusedField::Password && form.can_submit() {
                                // Get selected session exec command
                                let session_exec = session_selector
                                    .selected_exec()
                                    .unwrap_or("gar-session.sh")
                                    .to_string();

                                // Attempt login
                                if let Some(fade) = handle_login(
                                    &mut client,
                                    &mut form,
                                    &session_exec,
                                    config.effective_fade_duration(),
                                )
                                .await?
                                {
                                    fade_transition = Some(fade);
                                }
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

/// Handle power button action
async fn handle_power_action(client: &mut Client, action: PowerAction) -> Result<()> {
    let request = match action {
        PowerAction::Shutdown => {
            tracing::info!("Shutdown requested");
            Request::Shutdown
        }
        PowerAction::Reboot => {
            tracing::info!("Reboot requested");
            Request::Reboot
        }
        PowerAction::Suspend => {
            tracing::info!("Suspend requested");
            Request::Suspend
        }
    };

    match client.request(&request).await? {
        Response::Success => {
            tracing::info!("Power action succeeded");
        }
        Response::Error { message } => {
            tracing::warn!(message, "Power action failed");
            // TODO: Show error in UI
        }
        _ => {}
    }

    Ok(())
}

/// Handle login attempt, returns fade transition if successful
async fn handle_login(
    client: &mut Client,
    form: &mut LoginForm,
    session_exec: &str,
    fade_duration_ms: u64,
) -> Result<Option<FadeOutTransition>> {
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
            return Ok(None);
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

            // Start session with selected session command
            let session_cmd = vec![session_exec.to_string()];
            tracing::info!(?session_cmd, "Starting session");

            let response = client
                .request(&Request::StartSession {
                    cmd: session_cmd,
                    env: vec![],
                })
                .await?;

            match response {
                Response::Success => {
                    tracing::info!("Session started, beginning fade transition");
                    form.is_loading = false;
                    return Ok(Some(FadeOutTransition::new(fade_duration_ms)));
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
    Ok(None)
}

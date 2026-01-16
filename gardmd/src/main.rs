//! gardmd - gar display manager daemon
//!
//! Main entry point with X11 server management, greeter process handling,
//! and user session launching.

use anyhow::Result;
use clap::Parser;
use gardm_ipc::{Request, Response};
use gardmd::{
    auth::AuthSession, config::Config, greeter::GreeterProcess, ipc, session::UserSession,
    vt, x11::XServer,
};
use tokio::signal::unix::{signal, SignalKind};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// gar display manager daemon
#[derive(Parser, Debug)]
#[command(name = "gardmd", about = "gar display manager daemon")]
struct Args {
    /// Don't start X server (use existing DISPLAY for development)
    #[arg(long)]
    no_x: bool,

    /// X display to use (default: :0 or auto-detect)
    #[arg(long, short = 'd')]
    display: Option<String>,

    /// VT to use (default: auto-detect)
    #[arg(long)]
    vt: Option<u32>,

    /// Greeter command (default: from config)
    #[arg(long)]
    greeter: Option<String>,

    /// Run in test mode (IPC only, no greeter process management)
    #[arg(long)]
    test_mode: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();
    tracing::info!("gardmd starting");
    tracing::debug!(?args, "Command line arguments");

    // Load configuration
    let config = Config::load()?;
    tracing::debug!(?config, "Loaded configuration");

    if args.test_mode {
        run_test_mode(config).await
    } else {
        run_display_manager(args, config).await
    }
}

/// Run in test mode - IPC server only, for testing auth without X11
async fn run_test_mode(config: Config) -> Result<()> {
    tracing::info!("Running in test mode (IPC only)");

    let server = ipc::Server::new().await?;

    // Notify systemd we're ready
    if let Err(e) = sd_notify::notify(true, &[sd_notify::NotifyState::Ready]) {
        tracing::warn!("Failed to notify systemd: {}", e);
    }

    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;

    tracing::info!("gardmd ready (test mode)");

    loop {
        tokio::select! {
            result = server.accept() => {
                match result {
                    Ok(conn) => {
                        tokio::spawn(handle_test_client(conn, config.clone()));
                    }
                    Err(e) => {
                        tracing::error!("Failed to accept connection: {}", e);
                    }
                }
            }
            _ = sigterm.recv() => {
                tracing::info!("Received SIGTERM, shutting down");
                break;
            }
            _ = sigint.recv() => {
                tracing::info!("Received SIGINT, shutting down");
                break;
            }
        }
    }

    let _ = sd_notify::notify(true, &[sd_notify::NotifyState::Stopping]);
    tracing::info!("gardmd stopped");
    Ok(())
}

/// Run as full display manager
async fn run_display_manager(args: Args, config: Config) -> Result<()> {
    // Determine display and VT
    let (x_display, mut x_server) = if args.no_x {
        // Use existing X server
        let x_display = args
            .display
            .or_else(|| std::env::var("DISPLAY").ok())
            .unwrap_or_else(|| ":0".to_string());
        tracing::info!(display = %x_display, "Using existing X server");
        (x_display, None)
    } else {
        // Start our own X server
        let vt = args.vt.unwrap_or_else(|| {
            config.general.vt.max(1) // Use config VT or find one
        });
        let vt = if vt == 0 {
            vt::find_unused_vt()?
        } else {
            vt
        };

        let x_display = args
            .display
            .unwrap_or_else(|| gardmd::x11::find_available_x_display().unwrap_or(":0".into()));

        tracing::info!(display = %x_display, vt, "Starting X server");
        let x_server = XServer::start(&x_display, vt)?;

        // Switch to our VT
        vt::switch_to_vt(vt)?;

        (x_display, Some(x_server))
    };

    // Start IPC server
    let server = ipc::Server::new().await?;

    // Notify systemd we're ready
    if let Err(e) = sd_notify::notify(true, &[sd_notify::NotifyState::Ready]) {
        tracing::warn!("Failed to notify systemd: {}", e);
    }

    tracing::info!("gardmd ready");

    // Set up signal handlers
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;

    // Main greeter/session loop
    let greeter_cmd = args
        .greeter
        .unwrap_or_else(|| config.general.greeter.to_string_lossy().to_string());

    loop {
        // Start greeter
        tracing::info!("Starting greeter");
        let mut greeter = GreeterProcess::start(&greeter_cmd, &x_display)?;

        // Handle greeter authentication
        let session_result = tokio::select! {
            result = handle_greeter_session(&server, &x_display) => result,
            _ = sigterm.recv() => {
                tracing::info!("Received SIGTERM during greeter");
                break;
            }
            _ = sigint.recv() => {
                tracing::info!("Received SIGINT during greeter");
                break;
            }
        };

        // Kill greeter
        if let Err(e) = greeter.kill() {
            tracing::warn!(error = %e, "Failed to kill greeter");
        }

        match session_result {
            Ok(Some(session_info)) => {
                let vt = x_server.as_ref().map(|x| x.vt()).unwrap_or(1);
                let is_wayland = session_info.session_type == "wayland";

                if is_wayland {
                    // WAYLAND SESSION: Stop X server first, then start compositor
                    tracing::info!("Wayland session selected, stopping X server");

                    // Stop X server - compositor needs direct VT access
                    if let Some(x) = x_server.take() {
                        drop(x); // XServer::drop() handles graceful shutdown
                        tracing::info!("X server stopped");
                    }

                    // Start Wayland compositor directly on VT
                    let mut session = UserSession::start(
                        &session_info.username,
                        &session_info.cmd,
                        "wayland",
                        None, // No DISPLAY for Wayland
                        vt,
                    )?;

                    // Wait for session to end
                    let session_ended = tokio::select! {
                        _ = tokio::task::spawn_blocking(move || session.wait()) => {
                            tracing::info!("Wayland session ended");
                            true
                        }
                        _ = sigterm.recv() => {
                            tracing::info!("Received SIGTERM during Wayland session");
                            false
                        }
                        _ = sigint.recv() => {
                            tracing::info!("Received SIGINT during Wayland session");
                            false
                        }
                    };

                    if !session_ended {
                        break;
                    }

                    // Restart X server for greeter
                    tracing::info!("Restarting X server for greeter");
                    let new_x = XServer::start(&x_display, vt)?;
                    vt::switch_to_vt(vt)?;
                    x_server = Some(new_x);
                } else {
                    // X11 SESSION: Keep X server running (existing behavior)
                    let mut session = UserSession::start(
                        &session_info.username,
                        &session_info.cmd,
                        "x11",
                        Some(&x_display),
                        vt,
                    )?;

                    // Wait for session to end
                    tokio::select! {
                        _ = tokio::task::spawn_blocking(move || session.wait()) => {
                            tracing::info!("X11 session ended, restarting greeter");
                        }
                        _ = sigterm.recv() => {
                            tracing::info!("Received SIGTERM during session");
                            break;
                        }
                        _ = sigint.recv() => {
                            tracing::info!("Received SIGINT during session");
                            break;
                        }
                    }
                }
            }
            Ok(None) => {
                tracing::info!("Greeter disconnected without session, restarting");
            }
            Err(e) => {
                tracing::error!(error = %e, "Error during greeter session");
                // Brief delay before retry
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }

    // Cleanup
    let _ = sd_notify::notify(true, &[sd_notify::NotifyState::Stopping]);
    drop(x_server); // Stop X server
    tracing::info!("gardmd stopped");
    Ok(())
}

/// Information needed to start a user session
struct SessionStartInfo {
    username: String,
    cmd: Vec<String>,
    session_type: String,
}

/// Handle greeter IPC until we get a successful auth and StartSession
async fn handle_greeter_session(
    server: &ipc::Server,
    _display: &str,
) -> Result<Option<SessionStartInfo>> {
    let mut conn = server.accept().await?;
    let mut auth = AuthSession::new();

    loop {
        let request = match conn.recv().await? {
            Some(req) => req,
            None => return Ok(None), // Greeter disconnected
        };

        let (response, session_info) = handle_greeter_request(request, &mut auth).await;

        conn.send(&response).await?;

        if let Some(info) = session_info {
            return Ok(Some(info));
        }
    }
}

/// Handle a single greeter request, returning (response, optional session info)
async fn handle_greeter_request(
    request: Request,
    auth: &mut AuthSession,
) -> (Response, Option<SessionStartInfo>) {
    use gardmd::auth::AuthResponse;

    match request {
        Request::CreateSession { username } => {
            let response = match auth.create_session(&username) {
                AuthResponse::Prompt { prompt, echo } => Response::AuthPrompt { prompt, echo },
                AuthResponse::Error { message } => Response::Error { message },
                _ => Response::Error {
                    message: "Unexpected auth response".to_string(),
                },
            };
            (response, None)
        }

        Request::Authenticate { response: password } => {
            let response = match auth.authenticate(&password).await {
                AuthResponse::Success => Response::Success,
                AuthResponse::Prompt { prompt, echo } => Response::AuthPrompt { prompt, echo },
                AuthResponse::Error { message } => Response::AuthError { message },
                AuthResponse::Info { message } => Response::AuthInfo { message },
            };
            (response, None)
        }

        Request::StartSession { cmd, session_type, env: _ } => {
            if let Some(username) = auth.take_authenticated() {
                tracing::info!(%username, ?cmd, %session_type, "Session start requested");
                (
                    Response::Success,
                    Some(SessionStartInfo { username, cmd, session_type }),
                )
            } else {
                (
                    Response::Error {
                        message: "Not authenticated".to_string(),
                    },
                    None,
                )
            }
        }

        Request::CancelSession => {
            auth.cancel();
            (Response::Success, None)
        }

        Request::Shutdown => {
            tracing::info!("Shutdown requested");
            match gardmd::power::execute_async(gardmd::power::PowerAction::Shutdown).await {
                Ok(()) => (Response::Success, None),
                Err(e) => (
                    Response::Error {
                        message: format!("Shutdown failed: {}", e),
                    },
                    None,
                ),
            }
        }

        Request::Reboot => {
            tracing::info!("Reboot requested");
            match gardmd::power::execute_async(gardmd::power::PowerAction::Reboot).await {
                Ok(()) => (Response::Success, None),
                Err(e) => (
                    Response::Error {
                        message: format!("Reboot failed: {}", e),
                    },
                    None,
                ),
            }
        }

        Request::Suspend => {
            tracing::info!("Suspend requested");
            match gardmd::power::execute_async(gardmd::power::PowerAction::Suspend).await {
                Ok(()) => (Response::Success, None),
                Err(e) => (
                    Response::Error {
                        message: format!("Suspend failed: {}", e),
                    },
                    None,
                ),
            }
        }

        Request::ListSessions => {
            let sessions = gardmd::list_sessions();
            (Response::Sessions { sessions }, None)
        }

        Request::ListUsers => {
            let users = gardmd::list_users();
            (Response::Users { users }, None)
        }
    }
}

/// Handle a test mode client (same as before, for backwards compatibility)
async fn handle_test_client(mut conn: ipc::ClientConnection, _config: Config) {
    let mut auth = AuthSession::new();

    loop {
        match conn.recv().await {
            Ok(Some(request)) => {
                let (response, _) = handle_greeter_request(request, &mut auth).await;
                if let Err(e) = conn.send(&response).await {
                    tracing::error!("Failed to send response: {}", e);
                    break;
                }
            }
            Ok(None) => {
                tracing::debug!("Client disconnected");
                break;
            }
            Err(e) => {
                tracing::error!("Error receiving request: {}", e);
                break;
            }
        }
    }
}

//! gardmd - gar display manager daemon
//!
//! Main entry point with signal handling and systemd integration.

use anyhow::Result;
use gardmd::{config::Config, ipc};
use tokio::signal::unix::{signal, SignalKind};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("gardmd starting");

    // Load configuration
    let config = Config::load()?;
    tracing::debug!(?config, "Loaded configuration");

    // Create IPC server
    let server = ipc::Server::new().await?;

    // Set up signal handlers
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sighup = signal(SignalKind::hangup())?;

    // Notify systemd we're ready
    if let Err(e) = sd_notify::notify(true, &[sd_notify::NotifyState::Ready]) {
        tracing::warn!("Failed to notify systemd: {}", e);
    }

    tracing::info!("gardmd ready");

    // Main event loop
    loop {
        tokio::select! {
            // Handle new greeter connections
            result = server.accept() => {
                match result {
                    Ok(conn) => {
                        tokio::spawn(handle_client(conn, config.clone()));
                    }
                    Err(e) => {
                        tracing::error!("Failed to accept connection: {}", e);
                    }
                }
            }

            // Handle SIGTERM
            _ = sigterm.recv() => {
                tracing::info!("Received SIGTERM, shutting down");
                break;
            }

            // Handle SIGINT
            _ = sigint.recv() => {
                tracing::info!("Received SIGINT, shutting down");
                break;
            }

            // Handle SIGHUP (reload config)
            _ = sighup.recv() => {
                tracing::info!("Received SIGHUP, reloading config");
                // TODO: Implement config reload
            }
        }
    }

    // Notify systemd we're stopping
    let _ = sd_notify::notify(true, &[sd_notify::NotifyState::Stopping]);

    tracing::info!("gardmd stopped");
    Ok(())
}

/// Handle a connected greeter client
async fn handle_client(mut conn: ipc::ClientConnection, _config: Config) {
    loop {
        match conn.recv().await {
            Ok(Some(request)) => {
                let response = handle_request(request).await;
                if let Err(e) = conn.send(&response).await {
                    tracing::error!("Failed to send response: {}", e);
                    break;
                }
            }
            Ok(None) => {
                tracing::debug!("Greeter disconnected");
                break;
            }
            Err(e) => {
                tracing::error!("Error receiving request: {}", e);
                break;
            }
        }
    }
}

/// Process a request and return a response
async fn handle_request(request: gardm_ipc::Request) -> gardm_ipc::Response {
    use gardm_ipc::{Request, Response};

    match request {
        Request::CreateSession { username } => {
            tracing::info!("Creating session for user: {}", username);
            // TODO: Implement PAM session creation
            Response::AuthPrompt {
                prompt: "Password:".to_string(),
                echo: false,
            }
        }

        Request::Authenticate { response: _ } => {
            // TODO: Implement PAM authentication
            Response::Error {
                message: "PAM not yet implemented".to_string(),
            }
        }

        Request::StartSession { cmd, env } => {
            tracing::info!("Start session request: {:?} env={:?}", cmd, env);
            // TODO: Implement session start
            Response::Error {
                message: "Session start not yet implemented".to_string(),
            }
        }

        Request::CancelSession => {
            tracing::info!("Session cancelled");
            Response::Success
        }

        Request::Shutdown => {
            tracing::info!("Shutdown requested");
            // TODO: Implement via logind
            Response::Error {
                message: "Shutdown not yet implemented".to_string(),
            }
        }

        Request::Reboot => {
            tracing::info!("Reboot requested");
            // TODO: Implement via logind
            Response::Error {
                message: "Reboot not yet implemented".to_string(),
            }
        }

        Request::Suspend => {
            tracing::info!("Suspend requested");
            // TODO: Implement via logind
            Response::Error {
                message: "Suspend not yet implemented".to_string(),
            }
        }

        Request::ListSessions => {
            tracing::debug!("Listing sessions");
            // TODO: Enumerate /usr/share/xsessions and /usr/share/wayland-sessions
            Response::Sessions { sessions: vec![] }
        }

        Request::ListUsers => {
            tracing::debug!("Listing users");
            // TODO: Enumerate users from /etc/passwd
            Response::Users { users: vec![] }
        }
    }
}

//! gardm-greeter - gar display manager greeter
//!
//! Graphical login UI that communicates with gardmd.

use anyhow::Result;
use gardm_ipc::{Client, Request};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("gardm-greeter starting");

    // Connect to daemon
    let mut client = Client::connect().await?;
    tracing::info!("Connected to gardmd");

    // TODO: Initialize X11/rendering
    // TODO: Main UI loop

    // For now, just test the connection
    let response = client.request(&Request::ListSessions).await?;
    tracing::info!(?response, "Got sessions");

    let response = client.request(&Request::ListUsers).await?;
    tracing::info!(?response, "Got users");

    tracing::info!("gardm-greeter exiting (UI not yet implemented)");
    Ok(())
}

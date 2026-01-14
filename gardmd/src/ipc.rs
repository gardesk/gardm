//! IPC server for gardmd
//!
//! Handles connections from the greeter and processes requests.

use anyhow::Result;
use gardm_ipc::{Request, Response, SOCKET_PATH};
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;

/// IPC server for handling greeter connections
pub struct Server {
    listener: UnixListener,
}

impl Server {
    /// Create a new IPC server
    pub async fn new() -> Result<Self> {
        Self::bind(SOCKET_PATH).await
    }

    /// Bind to a specific socket path
    pub async fn bind(path: &str) -> Result<Self> {
        let path = Path::new(path);

        // Remove stale socket if it exists
        if path.exists() {
            std::fs::remove_file(path)?;
        }

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let listener = UnixListener::bind(path)?;
        tracing::info!("IPC server listening on {}", path.display());

        Ok(Self { listener })
    }

    /// Accept a new connection
    pub async fn accept(&self) -> Result<ClientConnection> {
        let (stream, _addr) = self.listener.accept().await?;
        tracing::debug!("New greeter connection");
        Ok(ClientConnection::new(stream))
    }
}

/// A connected greeter client
pub struct ClientConnection {
    reader: BufReader<tokio::net::unix::OwnedReadHalf>,
    writer: tokio::net::unix::OwnedWriteHalf,
}

impl ClientConnection {
    fn new(stream: UnixStream) -> Self {
        let (read, write) = stream.into_split();
        Self {
            reader: BufReader::new(read),
            writer: write,
        }
    }

    /// Receive a request from the greeter
    pub async fn recv(&mut self) -> Result<Option<Request>> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line).await?;
        if n == 0 {
            return Ok(None);
        }
        let request: Request = serde_json::from_str(&line)?;
        tracing::debug!(?request, "Received request");
        Ok(Some(request))
    }

    /// Send a response to the greeter
    pub async fn send(&mut self, response: &Response) -> Result<()> {
        let json = serde_json::to_string(response)?;
        self.writer.write_all(json.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;
        tracing::debug!(?response, "Sent response");
        Ok(())
    }
}

/// Commands from external sources (e.g., signal handlers)
#[derive(Debug)]
pub enum DaemonCommand {
    Shutdown,
    Reload,
}

/// Create a channel for daemon commands
pub fn command_channel() -> (mpsc::Sender<DaemonCommand>, mpsc::Receiver<DaemonCommand>) {
    mpsc::channel(16)
}

//! IPC protocol for gardm display manager
//!
//! Defines the JSON-based protocol between gardmd (daemon) and gardm-greeter (UI).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

/// Socket path for gardm IPC
pub const SOCKET_PATH: &str = "/run/gardm.sock";

/// Default session type for backward compatibility
fn default_session_type() -> String {
    "x11".to_string()
}

/// Requests from greeter to daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    /// Create a new authentication session for a user
    CreateSession { username: String },

    /// Provide authentication response (password, OTP, etc.)
    Authenticate { response: String },

    /// Start the user's session after successful auth
    StartSession {
        /// Session command (e.g., ["gar-session.sh"])
        cmd: Vec<String>,
        /// Session type: "x11" or "wayland"
        #[serde(default = "default_session_type")]
        session_type: String,
        /// Additional environment variables
        #[serde(default)]
        env: Vec<String>,
    },

    /// Cancel the current authentication attempt
    CancelSession,

    /// Request system shutdown
    Shutdown,

    /// Request system reboot
    Reboot,

    /// Request system suspend
    Suspend,

    /// Get list of available sessions
    ListSessions,

    /// Get list of available users
    ListUsers,
}

/// Responses from daemon to greeter
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    /// Operation completed successfully
    Success,

    /// PAM is requesting user input
    AuthPrompt {
        /// Prompt message (e.g., "Password:")
        prompt: String,
        /// Whether to echo input (false for passwords)
        echo: bool,
    },

    /// PAM informational message
    AuthInfo { message: String },

    /// Authentication failed
    AuthError { message: String },

    /// General error
    Error { message: String },

    /// List of available sessions
    Sessions { sessions: Vec<SessionInfo> },

    /// List of available users
    Users { users: Vec<UserInfo> },
}

/// Information about an available session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session identifier (desktop file name without .desktop)
    pub id: String,
    /// Display name
    pub name: String,
    /// Optional comment/description
    pub comment: Option<String>,
    /// Exec command
    pub exec: String,
    /// Session type (x11, wayland)
    pub session_type: String,
}

/// Information about a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    /// Username
    pub name: String,
    /// Full name (GECOS field)
    pub full_name: Option<String>,
    /// Home directory
    pub home: PathBuf,
    /// Path to user avatar (if available)
    pub avatar: Option<PathBuf>,
}

/// IPC client for connecting to gardmd
pub struct Client {
    reader: BufReader<tokio::net::unix::OwnedReadHalf>,
    writer: tokio::net::unix::OwnedWriteHalf,
}

impl Client {
    /// Connect to the daemon
    pub async fn connect() -> Result<Self, std::io::Error> {
        Self::connect_to(SOCKET_PATH).await
    }

    /// Connect to a specific socket path
    pub async fn connect_to(path: &str) -> Result<Self, std::io::Error> {
        let stream = UnixStream::connect(path).await?;
        let (read, write) = stream.into_split();
        Ok(Self {
            reader: BufReader::new(read),
            writer: write,
        })
    }

    /// Send a request to the daemon
    pub async fn send(&mut self, request: &Request) -> Result<(), std::io::Error> {
        let json = serde_json::to_string(request).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })?;
        self.writer.write_all(json.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;
        Ok(())
    }

    /// Receive a response from the daemon
    pub async fn recv(&mut self) -> Result<Response, std::io::Error> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line).await?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "daemon closed connection",
            ));
        }
        serde_json::from_str(&line).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })
    }

    /// Send request and wait for response
    pub async fn request(&mut self, request: &Request) -> Result<Response, std::io::Error> {
        self.send(request).await?;
        self.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let req = Request::CreateSession {
            username: "testuser".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("create_session"));
        assert!(json.contains("testuser"));
    }

    #[test]
    fn test_response_serialization() {
        let resp = Response::AuthPrompt {
            prompt: "Password:".to_string(),
            echo: false,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("auth_prompt"));
        assert!(json.contains("Password:"));
    }

    #[test]
    fn test_request_deserialization() {
        let json = r#"{"type":"authenticate","response":"secret123"}"#;
        let req: Request = serde_json::from_str(json).unwrap();
        match req {
            Request::Authenticate { response } => assert_eq!(response, "secret123"),
            _ => panic!("Wrong variant"),
        }
    }
}

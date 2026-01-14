//! gardmd - gar display manager daemon
//!
//! Handles PAM authentication, X11 server management, and session launching.

pub mod auth;
pub mod config;
pub mod greeter;
pub mod ipc;
pub mod session;
pub mod sessions;
pub mod vt;
pub mod x11;

pub use auth::{AuthResponse, AuthSession};
pub use config::Config;
pub use greeter::GreeterProcess;
pub use ipc::{ClientConnection, Server};
pub use session::UserSession;
pub use sessions::{list_sessions, list_users};
pub use x11::XServer;

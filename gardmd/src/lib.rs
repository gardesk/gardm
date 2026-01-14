//! gardmd - gar display manager daemon
//!
//! Handles PAM authentication, X11 server management, and session launching.

pub mod config;
pub mod ipc;

pub use config::Config;
pub use ipc::{ClientConnection, Server};

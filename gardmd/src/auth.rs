//! PAM authentication handling
//!
//! Implements a state machine for PAM-based authentication with
//! proper conversation handling for the greeter.

use anyhow::{Context, Result};
use pam_client::{Context as PamContext, Flag};

/// Service name for PAM configuration
const PAM_SERVICE: &str = "gardm";

/// Authentication state machine
#[derive(Debug)]
pub enum AuthState {
    /// No active authentication
    Idle,
    /// Waiting for password after CreateSession
    AwaitingPassword { username: String },
    /// Authentication succeeded
    Authenticated { username: String },
}

impl Default for AuthState {
    fn default() -> Self {
        Self::Idle
    }
}

/// Authentication session manager
pub struct AuthSession {
    state: AuthState,
}

/// Response from authentication operations
#[derive(Debug)]
pub enum AuthResponse {
    /// Request password from user
    Prompt { prompt: String, echo: bool },
    /// Authentication succeeded
    Success,
    /// Authentication failed
    Error { message: String },
    /// Informational message
    Info { message: String },
}

impl AuthSession {
    pub fn new() -> Self {
        Self {
            state: AuthState::Idle,
        }
    }

    /// Get current state for inspection
    pub fn state(&self) -> &AuthState {
        &self.state
    }

    /// Start a new authentication session for a user
    pub fn create_session(&mut self, username: &str) -> AuthResponse {
        tracing::info!(username, "Creating auth session");

        self.state = AuthState::AwaitingPassword {
            username: username.to_string(),
        };

        AuthResponse::Prompt {
            prompt: "Password:".to_string(),
            echo: false,
        }
    }

    /// Attempt authentication with provided password
    ///
    /// This runs PAM authentication in a blocking thread since PAM is synchronous.
    pub async fn authenticate(&mut self, password: &str) -> AuthResponse {
        let username = match &self.state {
            AuthState::AwaitingPassword { username } => username.clone(),
            _ => {
                return AuthResponse::Error {
                    message: "No authentication in progress".to_string(),
                }
            }
        };

        // Run PAM in blocking thread
        let password = password.to_string();
        let username_for_pam = username.clone();
        let result = tokio::task::spawn_blocking(move || {
            pam_authenticate(&username_for_pam, &password)
        })
        .await;

        match result {
            Ok(Ok(())) => {
                tracing::info!(%username, "Authentication succeeded");
                self.state = AuthState::Authenticated { username };
                AuthResponse::Success
            }
            Ok(Err(e)) => {
                tracing::warn!(%username, error = %e, "Authentication failed");
                self.state = AuthState::Idle;
                AuthResponse::Error {
                    message: format!("Authentication failed: {}", e),
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "PAM task panicked");
                self.state = AuthState::Idle;
                AuthResponse::Error {
                    message: "Internal authentication error".to_string(),
                }
            }
        }
    }

    /// Cancel the current authentication session
    pub fn cancel(&mut self) {
        if let AuthState::AwaitingPassword { username } | AuthState::Authenticated { username } =
            &self.state
        {
            tracing::info!(username, "Session cancelled");
        }
        self.state = AuthState::Idle;
    }

    /// Check if user is authenticated and ready to start session
    pub fn is_authenticated(&self) -> Option<&str> {
        match &self.state {
            AuthState::Authenticated { username } => Some(username),
            _ => None,
        }
    }

    /// Consume the authenticated state and return the username
    pub fn take_authenticated(&mut self) -> Option<String> {
        if matches!(self.state, AuthState::Authenticated { .. }) {
            let old = std::mem::replace(&mut self.state, AuthState::Idle);
            if let AuthState::Authenticated { username } = old {
                return Some(username);
            }
        }
        None
    }
}

/// Perform PAM authentication (blocking)
fn pam_authenticate(username: &str, password: &str) -> Result<()> {
    use pam_client::conv_mock::Conversation;

    // Create conversation handler that provides the password
    let conv = Conversation::with_credentials(username, password);

    // Create PAM context
    let mut ctx = PamContext::new(PAM_SERVICE, Some(username), conv)
        .context("Failed to create PAM context")?;

    // Authenticate
    ctx.authenticate(Flag::NONE)
        .context("PAM authentication failed")?;

    // Validate account (check expiry, etc.)
    ctx.acct_mgmt(Flag::NONE)
        .context("Account validation failed")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_state_machine() {
        let mut session = AuthSession::new();

        // Initial state is idle
        assert!(matches!(session.state(), AuthState::Idle));

        // Create session transitions to awaiting password
        let response = session.create_session("testuser");
        assert!(matches!(response, AuthResponse::Prompt { .. }));
        assert!(matches!(
            session.state(),
            AuthState::AwaitingPassword { .. }
        ));

        // Cancel returns to idle
        session.cancel();
        assert!(matches!(session.state(), AuthState::Idle));
    }
}

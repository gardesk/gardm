//! Power management via systemd-logind
//!
//! Implements shutdown, reboot, and suspend via D-Bus calls to logind.

use anyhow::{Context, Result};
use zbus::blocking::Connection;

/// Power action to perform
#[derive(Debug, Clone, Copy)]
pub enum PowerAction {
    Shutdown,
    Reboot,
    Suspend,
}

impl PowerAction {
    /// Get the logind method name for this action
    fn method_name(&self) -> &'static str {
        match self {
            PowerAction::Shutdown => "PowerOff",
            PowerAction::Reboot => "Reboot",
            PowerAction::Suspend => "Suspend",
        }
    }
}

/// Execute a power action via logind
pub fn execute(action: PowerAction) -> Result<()> {
    let conn = Connection::system().context("Failed to connect to system D-Bus")?;

    // Call org.freedesktop.login1.Manager.<Action>(interactive: bool)
    // interactive=false means no polkit prompt
    conn.call_method(
        Some("org.freedesktop.login1"),
        "/org/freedesktop/login1",
        Some("org.freedesktop.login1.Manager"),
        action.method_name(),
        &(false,), // interactive = false
    )
    .context(format!("Failed to execute {:?}", action))?;

    Ok(())
}

/// Async version of execute using zbus async API
pub async fn execute_async(action: PowerAction) -> Result<()> {
    let conn = zbus::Connection::system()
        .await
        .context("Failed to connect to system D-Bus")?;

    conn.call_method(
        Some("org.freedesktop.login1"),
        "/org/freedesktop/login1",
        Some("org.freedesktop.login1.Manager"),
        action.method_name(),
        &(false,),
    )
    .await
    .context(format!("Failed to execute {:?}", action))?;

    Ok(())
}

/// Check if a power action is allowed (can be performed without authentication)
pub async fn can_execute(action: PowerAction) -> Result<bool> {
    let conn = zbus::Connection::system()
        .await
        .context("Failed to connect to system D-Bus")?;

    let can_method = match action {
        PowerAction::Shutdown => "CanPowerOff",
        PowerAction::Reboot => "CanReboot",
        PowerAction::Suspend => "CanSuspend",
    };

    let reply: zbus::Message = conn
        .call_method(
            Some("org.freedesktop.login1"),
            "/org/freedesktop/login1",
            Some("org.freedesktop.login1.Manager"),
            can_method,
            &(),
        )
        .await
        .context(format!("Failed to check {:?} capability", action))?;

    let result: String = reply.body().deserialize().context("Failed to parse reply")?;

    // Returns "yes", "no", "challenge" (needs auth), or "na" (not applicable)
    Ok(result == "yes")
}

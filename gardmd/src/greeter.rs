//! Greeter process management
//!
//! Spawns and manages the greeter UI process.

use anyhow::{Context, Result};
use nix::unistd::User;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitStatus, Stdio};

/// Greeter process wrapper
pub struct GreeterProcess {
    process: Child,
}

impl GreeterProcess {
    /// Start the greeter process
    ///
    /// If a `gardm` user exists, runs the greeter as that user.
    /// Otherwise runs as the current user (for development).
    pub fn start(greeter_cmd: &str, display: &str) -> Result<Self> {
        let mut cmd = Command::new(greeter_cmd);

        cmd.env("DISPLAY", display)
            .env("XDG_SESSION_CLASS", "greeter")
            .env("XDG_SESSION_TYPE", "x11")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        // Try to run as gardm user if it exists
        if let Some(user) = User::from_name("gardm").ok().flatten() {
            let uid = user.uid;
            let gid = user.gid;
            let home = user.dir.to_string_lossy().to_string();

            cmd.env("HOME", &home);
            cmd.env("USER", "gardm");
            cmd.env("LOGNAME", "gardm");

            unsafe {
                cmd.pre_exec(move || {
                    nix::unistd::setgid(gid)?;
                    nix::unistd::setuid(uid)?;
                    Ok(())
                });
            }

            tracing::debug!("Starting greeter as gardm user");
        } else {
            tracing::warn!("gardm user not found, running greeter as current user");
        }

        let process = cmd.spawn().context("Failed to spawn greeter")?;

        tracing::info!(
            greeter = greeter_cmd,
            pid = process.id(),
            "Started greeter"
        );

        Ok(Self { process })
    }

    /// Check if greeter is still running
    pub fn is_running(&mut self) -> bool {
        matches!(self.process.try_wait(), Ok(None))
    }

    /// Wait for greeter to exit
    pub fn wait(&mut self) -> Result<ExitStatus> {
        self.process.wait().context("Failed to wait for greeter")
    }

    /// Kill the greeter process
    pub fn kill(&mut self) -> Result<()> {
        tracing::info!(pid = self.process.id(), "Killing greeter");

        // Send SIGTERM first
        let _ = nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(self.process.id() as i32),
            nix::sys::signal::Signal::SIGTERM,
        );

        // Wait briefly for graceful exit
        std::thread::sleep(std::time::Duration::from_millis(200));

        // Force kill if still running
        if self.is_running() {
            self.process.kill().ok();
        }

        self.process.wait().context("Failed to wait for greeter")?;
        Ok(())
    }

    /// Get the process ID
    pub fn pid(&self) -> u32 {
        self.process.id()
    }
}

impl Drop for GreeterProcess {
    fn drop(&mut self) {
        if self.is_running() {
            if let Err(e) = self.kill() {
                tracing::warn!(error = %e, "Failed to kill greeter on drop");
            }
        }
    }
}

//! User session management
//!
//! Launches and manages user desktop sessions.

use anyhow::{Context, Result};
use nix::unistd::User;
use std::ffi::CString;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitStatus, Stdio};

/// User session wrapper
pub struct UserSession {
    process: Child,
    username: String,
}

impl UserSession {
    /// Start a user session
    pub fn start(
        username: &str,
        session_cmd: &[String],
        display: &str,
        vt: u32,
    ) -> Result<Self> {
        let user = User::from_name(username)
            .context("Failed to look up user")?
            .ok_or_else(|| anyhow::anyhow!("User {} not found", username))?;

        let home = user.dir.to_string_lossy().to_string();
        let shell = user.shell.to_string_lossy().to_string();
        let uid = user.uid;
        let gid = user.gid;
        let username_cstr = CString::new(username).context("Invalid username")?;

        // Determine session command
        let (cmd_path, cmd_args) = if session_cmd.is_empty() {
            // Default: try gar-session.sh, fall back to shell
            let default_session = "/usr/local/bin/gar-session.sh";
            if std::path::Path::new(default_session).exists() {
                (default_session.to_string(), vec![])
            } else {
                (shell.clone(), vec!["-l".to_string()])
            }
        } else {
            (session_cmd[0].clone(), session_cmd[1..].to_vec())
        };

        // Set up XAUTHORITY path (even though we use -auth /dev/null, some apps expect it)
        let xauthority = format!("{}/.Xauthority", home);

        let mut cmd = Command::new(&cmd_path);
        cmd.args(&cmd_args)
            .env_clear()
            .env("DISPLAY", display)
            .env("XAUTHORITY", &xauthority)
            .env("HOME", &home)
            .env("USER", username)
            .env("LOGNAME", username)
            .env("SHELL", &shell)
            .env("PATH", format!("{}/.local/bin:{}/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin", home, home))
            .env("XDG_SESSION_TYPE", "x11")
            .env("XDG_VTNR", vt.to_string())
            .env("XDG_SEAT", "seat0")
            .env("XDG_SESSION_CLASS", "user")
            .env("XDG_SESSION_DESKTOP", "gar")
            .env("XDG_CURRENT_DESKTOP", "gar")
            .env("DBUS_SESSION_BUS_ADDRESS", format!("unix:path=/run/user/{}/bus", uid.as_raw()))
            .env("XDG_RUNTIME_DIR", format!("/run/user/{}", uid.as_raw()))
            .env("XDG_DATA_DIRS", "/usr/local/share:/usr/share")
            .env("XDG_CONFIG_DIRS", "/etc/xdg")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        // Set up process to run as the user
        unsafe {
            let home_for_closure = home.clone();
            cmd.pre_exec(move || {
                // Initialize supplementary groups
                nix::unistd::initgroups(&username_cstr, gid)?;

                // Set GID and UID
                nix::unistd::setgid(gid)?;
                nix::unistd::setuid(uid)?;

                // Change to home directory
                std::env::set_current_dir(&home_for_closure)?;

                Ok(())
            });
        }

        let process = cmd.spawn().context("Failed to spawn session")?;

        tracing::info!(
            username,
            session = cmd_path,
            pid = process.id(),
            "Started user session"
        );

        Ok(Self {
            process,
            username: username.to_string(),
        })
    }

    /// Get the username this session belongs to
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Check if session is still running
    pub fn is_running(&mut self) -> bool {
        matches!(self.process.try_wait(), Ok(None))
    }

    /// Wait for session to exit
    pub fn wait(&mut self) -> Result<ExitStatus> {
        let status = self.process.wait().context("Failed to wait for session")?;
        tracing::info!(
            username = %self.username,
            exit_code = ?status.code(),
            "Session ended"
        );
        Ok(status)
    }

    /// Get the process ID
    pub fn pid(&self) -> u32 {
        self.process.id()
    }
}

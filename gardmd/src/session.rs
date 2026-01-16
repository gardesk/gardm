//! User session management
//!
//! Launches and manages user desktop sessions.

use anyhow::{Context, Result};
use nix::unistd::User;
use std::ffi::CString;
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitStatus, Stdio};

/// User session wrapper
pub struct UserSession {
    process: Child,
    username: String,
}

impl UserSession {
    /// Start a user session
    ///
    /// # Arguments
    /// * `username` - The user to run the session as
    /// * `session_cmd` - Command to execute (e.g., ["Hyprland"] or ["gar-session.sh"])
    /// * `session_type` - "x11" or "wayland"
    /// * `display` - X11 display (e.g., ":0"), None for Wayland sessions
    /// * `vt` - Virtual terminal number
    pub fn start(
        username: &str,
        session_cmd: &[String],
        session_type: &str,
        display: Option<&str>,
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

        let is_wayland = session_type == "wayland";

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

        let mut cmd = Command::new(&cmd_path);
        cmd.args(&cmd_args)
            .env_clear()
            .env("HOME", &home)
            .env("USER", username)
            .env("LOGNAME", username)
            .env("SHELL", &shell)
            .env("PATH", format!("{}/.local/bin:{}/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin", home, home))
            .env("XDG_VTNR", vt.to_string())
            .env("XDG_SEAT", "seat0")
            .env("XDG_SESSION_CLASS", "user")
            .env("DBUS_SESSION_BUS_ADDRESS", format!("unix:path=/run/user/{}/bus", uid.as_raw()))
            .env("XDG_RUNTIME_DIR", format!("/run/user/{}", uid.as_raw()))
            .env("XDG_DATA_DIRS", "/usr/local/share:/usr/share")
            .env("XDG_CONFIG_DIRS", "/etc/xdg")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        // Session-type specific environment
        if is_wayland {
            cmd.env("XDG_SESSION_TYPE", "wayland");
            // Wayland compositors create their own WAYLAND_DISPLAY
            // Don't set DISPLAY - that's X11-specific
            tracing::info!(vt, "Configuring Wayland session environment");
        } else {
            // X11 session
            cmd.env("XDG_SESSION_TYPE", "x11");
            if let Some(display) = display {
                cmd.env("DISPLAY", display);
                cmd.env("XAUTHORITY", format!("{}/.Xauthority", home));
            }
            cmd.env("XDG_SESSION_DESKTOP", "gar");
            cmd.env("XDG_CURRENT_DESKTOP", "gar");
        }

        // Set up process to run as the user
        let tty_path = format!("/dev/tty{}", vt);
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

                // For Wayland sessions, set up controlling TTY
                // Wayland compositors need direct TTY access for input/output
                if is_wayland {
                    // Create new session (detach from parent's controlling terminal)
                    libc::setsid();

                    // Open the VT and make it our controlling terminal
                    let tty = std::fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .open(&tty_path)?;

                    // Set as controlling terminal (TIOCSCTTY)
                    if libc::ioctl(tty.as_raw_fd(), libc::TIOCSCTTY, 0) < 0 {
                        tracing::warn!("Failed to set controlling TTY, compositor may handle this");
                    }
                }

                Ok(())
            });
        }

        let process = cmd.spawn().context("Failed to spawn session")?;

        tracing::info!(
            username,
            session = cmd_path,
            session_type,
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

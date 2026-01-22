//! User session management
//!
//! Launches and manages user desktop sessions using fork/exec with proper
//! PAM session handling in the child process.

use anyhow::{anyhow, Context, Result};
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::{ForkResult, Pid, User};
use pam::Client;
use std::ffi::CString;
use std::os::unix::io::RawFd;
use std::path::Path;
use std::process::ExitStatus;

use crate::auth::PAM_SERVICE_NAME;

/// Standard PATH for session processes
const SESSION_PATH: &str = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";

/// Resolve a command to its full path by searching PATH directories
fn resolve_command(cmd: &str, home: &str) -> String {
    if cmd.starts_with('/') {
        return cmd.to_string();
    }

    let search_path = format!(
        "{}/.local/bin:{}/.cargo/bin:{}",
        home, home, SESSION_PATH
    );

    for dir in search_path.split(':') {
        let full_path = Path::new(dir).join(cmd);
        if full_path.exists() && full_path.is_file() {
            if let Ok(metadata) = full_path.metadata() {
                use std::os::unix::fs::PermissionsExt;
                if metadata.permissions().mode() & 0o111 != 0 {
                    return full_path.to_string_lossy().to_string();
                }
            }
        }
    }

    cmd.to_string()
}

/// User session wrapper
pub struct UserSession {
    pid: Pid,
    username: String,
}

impl UserSession {
    /// Start a user session
    ///
    /// This forks and in the child process:
    /// 1. Authenticates with PAM and opens a session (registers with logind)
    /// 2. Drops privileges to the target user
    /// 3. Sets up TTY for Wayland sessions
    /// 4. Execs the session command
    ///
    /// # Arguments
    /// * `username` - The user to run the session as
    /// * `password` - The user's password for PAM authentication
    /// * `session_cmd` - Command to execute (e.g., ["Hyprland"] or ["gar-session.sh"])
    /// * `session_type` - "x11" or "wayland"
    /// * `display` - X11 display (e.g., ":0"), None for Wayland sessions
    /// * `vt` - Virtual terminal number
    pub fn start(
        username: &str,
        password: &str,
        session_cmd: &[String],
        session_type: &str,
        display: Option<&str>,
        vt: u32,
    ) -> Result<Self> {
        let user = User::from_name(username)
            .context("Failed to look up user")?
            .ok_or_else(|| anyhow!("User {} not found", username))?;

        let home = user.dir.to_string_lossy().to_string();
        let shell = user.shell.to_string_lossy().to_string();
        let uid = user.uid;
        let gid = user.gid;

        let is_wayland = session_type == "wayland";

        // Determine session command
        let (cmd_path, cmd_args) = if session_cmd.is_empty() {
            let default_session = "/usr/local/bin/gar-session.sh";
            if Path::new(default_session).exists() {
                (default_session.to_string(), vec![])
            } else {
                (shell.clone(), vec!["-l".to_string()])
            }
        } else {
            let resolved = resolve_command(&session_cmd[0], &home);
            tracing::debug!(
                original = %session_cmd[0],
                resolved = %resolved,
                "Resolved session command"
            );
            (resolved, session_cmd[1..].to_vec())
        };

        // Prepare TTY for Wayland sessions (as root, before fork)
        let tty_path = format!("/dev/tty{}", vt);
        let tty_fd: Option<RawFd> = if is_wayland {
            // Change TTY ownership to target user
            let tty_cstr = CString::new(tty_path.as_str()).context("Invalid TTY path")?;
            let ret = unsafe { libc::chown(tty_cstr.as_ptr(), uid.as_raw(), gid.as_raw()) };
            if ret != 0 {
                return Err(anyhow!(
                    "Failed to chown TTY: {}",
                    std::io::Error::last_os_error()
                ));
            }

            // Open TTY as root
            let tty = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&tty_path)
                .context(format!("Failed to open TTY {}", tty_path))?;

            use std::os::unix::io::IntoRawFd;
            Some(tty.into_raw_fd())
        } else {
            None
        };

        // Build environment for the session
        let env_vars = build_session_env(
            username, &home, &shell, uid.as_raw(), vt, session_type, display,
        );

        tracing::debug!(
            cmd = %cmd_path,
            args = ?cmd_args,
            session_type,
            vt,
            "About to fork for session"
        );

        // Fork!
        match unsafe { nix::unistd::fork() } {
            Ok(ForkResult::Parent { child }) => {
                // Parent: close the TTY fd if we opened one
                if let Some(fd) = tty_fd {
                    unsafe { libc::close(fd) };
                }

                tracing::info!(
                    username,
                    session = %cmd_path,
                    args = ?cmd_args,
                    session_type,
                    pid = child.as_raw(),
                    "Started user session"
                );

                Ok(Self {
                    pid: child,
                    username: username.to_string(),
                })
            }
            Ok(ForkResult::Child) => {
                // Child process - this will exec or exit
                // Note: we can't use tracing here as it's not fork-safe

                // Run child process setup - this calls exec() or exit(), never returns
                child_process_main(
                    username,
                    password,
                    &home,
                    uid,
                    gid,
                    &cmd_path,
                    &cmd_args,
                    env_vars,  // Pass ownership, will be modified after PAM opens session
                    is_wayland,
                    tty_fd,
                    vt,
                    session_type,
                )
            }
            Err(e) => {
                if let Some(fd) = tty_fd {
                    unsafe { libc::close(fd) };
                }
                Err(anyhow!("Fork failed: {}", e))
            }
        }
    }

    /// Get the username this session belongs to
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Check if session is still running
    pub fn is_running(&mut self) -> bool {
        matches!(
            waitpid(self.pid, Some(nix::sys::wait::WaitPidFlag::WNOHANG)),
            Ok(WaitStatus::StillAlive)
        )
    }

    /// Wait for session to exit
    pub fn wait(&mut self) -> Result<ExitStatus> {
        loop {
            match waitpid(self.pid, None) {
                Ok(WaitStatus::Exited(_, code)) => {
                    tracing::info!(
                        username = %self.username,
                        exit_code = code,
                        "Session ended"
                    );
                    // Create ExitStatus from code
                    return Ok(std::process::ExitStatus::from_raw(code << 8));
                }
                Ok(WaitStatus::Signaled(_, sig, _)) => {
                    tracing::info!(
                        username = %self.username,
                        signal = ?sig,
                        "Session killed by signal"
                    );
                    return Ok(std::process::ExitStatus::from_raw(128 + sig as i32));
                }
                Ok(_) => {
                    // Other status (stopped, continued) - keep waiting
                    continue;
                }
                Err(nix::errno::Errno::ECHILD) => {
                    // Child already reaped
                    tracing::info!(username = %self.username, "Session ended (already reaped)");
                    return Ok(std::process::ExitStatus::from_raw(0));
                }
                Err(e) => {
                    return Err(anyhow!("waitpid failed: {}", e));
                }
            }
        }
    }

    /// Get the process ID
    pub fn pid(&self) -> u32 {
        self.pid.as_raw() as u32
    }
}

/// Build environment variables for the session
fn build_session_env(
    username: &str,
    home: &str,
    shell: &str,
    uid: u32,
    vt: u32,
    session_type: &str,
    display: Option<&str>,
) -> Vec<CString> {
    let mut env = vec![
        format!("HOME={}", home),
        format!("USER={}", username),
        format!("LOGNAME={}", username),
        format!("SHELL={}", shell),
        format!("PATH={}/.local/bin:{}/.cargo/bin:{}", home, home, SESSION_PATH),
        format!("XDG_VTNR={}", vt),
        "XDG_SEAT=seat0".to_string(),
        "XDG_SESSION_CLASS=user".to_string(),
        format!("DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/{}/bus", uid),
        format!("XDG_RUNTIME_DIR=/run/user/{}", uid),
        "XDG_DATA_DIRS=/usr/local/share:/usr/share".to_string(),
        "XDG_CONFIG_DIRS=/etc/xdg".to_string(),
        format!("XDG_SESSION_TYPE={}", session_type),
    ];

    if session_type == "x11" {
        if let Some(d) = display {
            env.push(format!("DISPLAY={}", d));
            env.push(format!("XAUTHORITY={}/.Xauthority", home));
        }
        env.push("XDG_SESSION_DESKTOP=gar".to_string());
        env.push("XDG_CURRENT_DESKTOP=gar".to_string());
    }

    env.into_iter()
        .filter_map(|s| CString::new(s).ok())
        .collect()
}

/// Child process main function - handles PAM, privilege drop, TTY setup, and exec
/// This function either calls exec() or exit() - it never returns
fn child_process_main(
    username: &str,
    password: &str,
    home: &str,
    uid: nix::unistd::Uid,
    gid: nix::unistd::Gid,
    cmd_path: &str,
    cmd_args: &[String],
    mut env_vars: Vec<CString>,
    is_wayland: bool,
    tty_fd: Option<RawFd>,
    vt: u32,
    session_type: &str,
) -> ! {
    // Create new session (detach from parent's controlling terminal)
    // This must be done before PAM so pam_systemd sees us as a session leader
    if is_wayland {
        unsafe {
            let sid = libc::setsid();
            if sid < 0 {
                eprintln!("[SESSION] setsid() failed: {}", std::io::Error::last_os_error());
            }
        }
    }

    // PAM authentication and session opening
    // This registers the session with systemd-logind for device access
    if let Err(e) = pam_authenticate_and_open_session(username, password, vt, session_type) {
        eprintln!("[SESSION] PAM failed: {}", e);
        std::process::exit(1);
    }

    // After pam_open_session(), pam_systemd sets XDG_SESSION_ID in the environment
    // Read it and add to the environment we'll pass to the session
    if let Ok(session_id) = std::env::var("XDG_SESSION_ID") {
        if let Ok(cstr) = CString::new(format!("XDG_SESSION_ID={}", session_id)) {
            env_vars.push(cstr);
        }
    }

    // Initialize supplementary groups (must be done as root)
    let username_cstr = match CString::new(username) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[SESSION] Invalid username: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = nix::unistd::initgroups(&username_cstr, gid) {
        eprintln!("[SESSION] initgroups failed: {}", e);
        std::process::exit(1);
    }

    // Drop privileges
    if let Err(e) = nix::unistd::setgid(gid) {
        eprintln!("[SESSION] setgid failed: {}", e);
        std::process::exit(1);
    }

    if let Err(e) = nix::unistd::setuid(uid) {
        eprintln!("[SESSION] setuid failed: {}", e);
        std::process::exit(1);
    }

    // Change to home directory
    if std::env::set_current_dir(home).is_err() {
        eprintln!("[SESSION] Failed to chdir to home");
        // Non-fatal, continue
    }

    // Set up controlling TTY for Wayland sessions
    if let Some(fd) = tty_fd {
        unsafe {
            // Set as controlling terminal (steal if necessary)
            if libc::ioctl(fd, libc::TIOCSCTTY, 1) < 0 {
                eprintln!(
                    "[SESSION] TIOCSCTTY failed: {}",
                    std::io::Error::last_os_error()
                );
                // Non-fatal for some compositors
            }

            // Set up stdin/stdout/stderr to the TTY
            libc::dup2(fd, 0);
            libc::dup2(fd, 1);
            libc::dup2(fd, 2);

            if fd > 2 {
                libc::close(fd);
            }
        }
    }

    // Prepare exec arguments
    let cmd_cstr = match CString::new(cmd_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[SESSION] Invalid command path: {}", e);
            std::process::exit(1);
        }
    };

    let mut argv: Vec<CString> = vec![cmd_cstr.clone()];
    for arg in cmd_args {
        match CString::new(arg.as_str()) {
            Ok(c) => argv.push(c),
            Err(e) => {
                eprintln!("[SESSION] Invalid argument: {}", e);
                std::process::exit(1);
            }
        }
    }

    // execve replaces the process image
    match nix::unistd::execve(&cmd_cstr, &argv, &env_vars) {
        Ok(_) => unreachable!(), // execve doesn't return on success
        Err(e) => {
            eprintln!("[SESSION] execve failed: {}", e);
            std::process::exit(127);
        }
    }
}

/// Authenticate with PAM and open a session
/// This must be called in the child process before dropping privileges
fn pam_authenticate_and_open_session(
    username: &str,
    password: &str,
    vt: u32,
    session_type: &str,
) -> Result<()> {
    // Set environment variables BEFORE creating PAM client
    // pam_systemd reads these to determine session type and VT
    std::env::set_var("XDG_SESSION_TYPE", session_type);
    std::env::set_var("XDG_VTNR", vt.to_string());
    std::env::set_var("XDG_SEAT", "seat0");
    std::env::set_var("XDG_SESSION_CLASS", "user");

    let mut client = Client::with_password(PAM_SERVICE_NAME)
        .map_err(|e| anyhow!("Failed to create PAM client: {:?}", e))?;

    client.conversation_mut().set_credentials(username, password);

    client
        .authenticate()
        .map_err(|e| anyhow!("PAM authentication failed: {:?}", e))?;

    client
        .open_session()
        .map_err(|e| anyhow!("PAM open_session failed: {:?}", e))?;

    // Keep the client alive - don't let it drop and close the session
    // The session will be closed when the process exits
    std::mem::forget(client);

    Ok(())
}

// Trait implementation for ExitStatus::from_raw
use std::os::unix::process::ExitStatusExt;

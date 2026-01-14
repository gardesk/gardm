//! X11 server management
//!
//! Starts and manages Xorg server lifecycle.

use anyhow::{Context, Result};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Xorg server wrapper
pub struct XServer {
    process: Child,
    x_display: String,
    vt: u32,
}

impl XServer {
    /// Start Xorg server on specified x_display and VT
    pub fn start(x_display: &str, vt: u32) -> Result<Self> {
        let vt_arg = format!("vt{}", vt);

        let process = Command::new("/usr/bin/Xorg")
            .arg(x_display)
            .arg(&vt_arg)
            .arg("-keeptty")
            .arg("-noreset")
            .arg("-novtswitch")
            .arg("-nolisten")
            .arg("tcp")
            .arg("-auth")
            .arg("/dev/null") // We'll set up proper auth later
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn Xorg")?;

        tracing::info!(
            %x_display,
            vt,
            pid = process.id(),
            "Started Xorg"
        );

        let server = Self {
            process,
            x_display: x_display.to_string(),
            vt,
        };

        // Wait for X server to become ready
        server.wait_ready(Duration::from_secs(10))?;

        Ok(server)
    }

    /// Wait for X server to accept connections
    fn wait_ready(&self, timeout: Duration) -> Result<()> {
        let start = Instant::now();

        while start.elapsed() < timeout {
            // Try to connect to X server
            match x11rb::connect(Some(&self.x_display)) {
                Ok((conn, _)) => {
                    drop(conn);
                    tracing::debug!(x_display = %self.x_display, "X server ready");
                    return Ok(());
                }
                Err(_) => {
                    std::thread::sleep(Duration::from_millis(100));
                }
            }

            // Check if process died
            if !self.is_process_alive() {
                anyhow::bail!("X server process died during startup");
            }
        }

        anyhow::bail!("X server failed to become ready within {:?}", timeout)
    }

    /// Check if the X server process is still running
    fn is_process_alive(&self) -> bool {
        // We can't use try_wait without &mut self, so check /proc
        let proc_path = format!("/proc/{}", self.process.id());
        std::path::Path::new(&proc_path).exists()
    }

    /// Get the x_display string (e.g., ":0")
    pub fn x_display(&self) -> &str {
        &self.x_display
    }

    /// Get the VT number
    pub fn vt(&self) -> u32 {
        self.vt
    }

    /// Check if X server is still running
    pub fn is_running(&mut self) -> bool {
        matches!(self.process.try_wait(), Ok(None))
    }

    /// Stop the X server gracefully
    pub fn stop(&mut self) -> Result<()> {
        tracing::info!(x_display = %self.x_display, "Stopping X server");

        // Send SIGTERM first
        let _ = nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(self.process.id() as i32),
            nix::sys::signal::Signal::SIGTERM,
        );

        // Wait a bit for graceful shutdown
        std::thread::sleep(Duration::from_millis(500));

        // Force kill if still running
        if self.is_running() {
            self.process.kill().ok();
        }

        self.process.wait().context("Failed to wait for X server")?;
        Ok(())
    }

    /// Get the process ID
    pub fn pid(&self) -> u32 {
        self.process.id()
    }
}

impl Drop for XServer {
    fn drop(&mut self) {
        if let Err(e) = self.stop() {
            tracing::warn!(error = %e, "Failed to stop X server cleanly");
        }
    }
}

/// Find an available x_display number
pub fn find_available_x_display() -> Result<String> {
    for n in 0..10 {
        let x_display = format!(":{}", n);
        let lock_file = format!("/tmp/.X{}-lock", n);

        if !std::path::Path::new(&lock_file).exists() {
            return Ok(x_display);
        }
    }

    anyhow::bail!("No available X x_display found")
}

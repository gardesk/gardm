# Sprint 2: X11 Server Management

**Goal:** Start and manage Xorg server, launch greeter process, and handle session startup.

## Objectives

- Start Xorg on appropriate VT with correct permissions
- Launch greeter as unprivileged user
- Start user session after successful auth
- Handle X server lifecycle (crash recovery, clean shutdown)
- Integrate with systemd-logind for session registration

## Background: X11 Display Manager Flow

```
1. gardmd starts
2. gardmd spawns Xorg on VT (e.g., :0 on vt1)
3. gardmd waits for X to be ready (connect test)
4. gardmd spawns greeter as unprivileged user
5. Greeter authenticates user via IPC
6. gardmd kills greeter, starts user session
7. User session runs until logout
8. gardmd restarts greeter (loop back to 4)
```

## Tasks

### 2.1 X Server Launcher

```rust
// gardmd/src/x11.rs

use nix::unistd::{fork, ForkResult, setsid, Pid};
use std::process::{Command, Child};
use std::time::Duration;

pub struct XServer {
    process: Child,
    display: String,
    vt: u32,
}

impl XServer {
    /// Start Xorg server on specified display and VT
    pub fn start(display: &str, vt: u32) -> anyhow::Result<Self> {
        // Xorg arguments
        let mut cmd = Command::new("/usr/bin/Xorg");
        cmd.arg(display)
           .arg(&format!("vt{}", vt))
           .arg("-keeptty")
           .arg("-noreset")
           .arg("-novtswitch")
           .arg("-nolisten").arg("tcp");

        // For multi-seat support (future):
        // cmd.arg("-seat").arg("seat0");

        // Capture output for debugging
        cmd.stdout(std::process::Stdio::piped())
           .stderr(std::process::Stdio::piped());

        let process = cmd.spawn()?;

        tracing::info!("Started Xorg on {} (vt{}, pid={})",
            display, vt, process.id());

        let server = Self {
            process,
            display: display.to_string(),
            vt,
        };

        // Wait for X to be ready
        server.wait_ready(Duration::from_secs(10))?;

        Ok(server)
    }

    /// Wait for X server to be ready by attempting connection
    fn wait_ready(&self, timeout: Duration) -> anyhow::Result<()> {
        use std::time::Instant;

        let start = Instant::now();
        while start.elapsed() < timeout {
            // Try to connect to X server
            match x11rb::connect(Some(&self.display)) {
                Ok(_) => {
                    tracing::debug!("X server ready on {}", self.display);
                    return Ok(());
                }
                Err(_) => {
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }

        anyhow::bail!("X server failed to become ready within {:?}", timeout);
    }

    /// Get the display string (e.g., ":0")
    pub fn display(&self) -> &str {
        &self.display
    }

    /// Get the VT number
    pub fn vt(&self) -> u32 {
        self.vt
    }

    /// Check if X server is still running
    pub fn is_running(&mut self) -> bool {
        match self.process.try_wait() {
            Ok(None) => true,
            _ => false,
        }
    }

    /// Stop the X server
    pub fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("Stopping X server");
        self.process.kill()?;
        self.process.wait()?;
        Ok(())
    }
}

impl Drop for XServer {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
```

### 2.2 VT Allocation

```rust
// gardmd/src/vt.rs

use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use nix::ioctl_read_bad;

// ioctl for getting/setting VT
const VT_GETSTATE: u64 = 0x5603;
const VT_ACTIVATE: u64 = 0x5606;
const VT_WAITACTIVE: u64 = 0x5607;
const VT_OPENQRY: u64 = 0x5600;

#[repr(C)]
pub struct VtStat {
    pub v_active: u16,
    pub v_signal: u16,
    pub v_state: u16,
}

/// Find an unused VT
pub fn find_unused_vt() -> anyhow::Result<u32> {
    let console = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty0")?;

    let mut vt: i32 = 0;
    unsafe {
        if libc::ioctl(console.as_raw_fd(), VT_OPENQRY as _, &mut vt) < 0 {
            anyhow::bail!("Failed to find unused VT");
        }
    }

    if vt <= 0 {
        anyhow::bail!("No unused VT available");
    }

    Ok(vt as u32)
}

/// Switch to a specific VT
pub fn switch_to_vt(vt: u32) -> anyhow::Result<()> {
    let console = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty0")?;

    unsafe {
        if libc::ioctl(console.as_raw_fd(), VT_ACTIVATE as _, vt) < 0 {
            anyhow::bail!("Failed to activate VT {}", vt);
        }
        if libc::ioctl(console.as_raw_fd(), VT_WAITACTIVE as _, vt) < 0 {
            anyhow::bail!("Failed to wait for VT {}", vt);
        }
    }

    Ok(())
}
```

### 2.3 Greeter Process Manager

```rust
// gardmd/src/greeter.rs

use nix::unistd::{Uid, Gid, setuid, setgid, User};
use std::process::{Command, Child};

pub struct GreeterProcess {
    process: Child,
}

impl GreeterProcess {
    /// Start the greeter as the gardm user
    pub fn start(greeter_cmd: &str, display: &str) -> anyhow::Result<Self> {
        // Get gardm user info
        let user = User::from_name("gardm")?
            .ok_or_else(|| anyhow::anyhow!("gardm user not found"))?;

        let mut cmd = Command::new(greeter_cmd);
        cmd.env("DISPLAY", display)
           .env("XDG_SESSION_CLASS", "greeter")
           .env("XDG_SESSION_TYPE", "x11");

        // Run as gardm user
        unsafe {
            cmd.pre_exec(move || {
                setgid(Gid::from_raw(user.gid.as_raw()))?;
                setuid(Uid::from_raw(user.uid.as_raw()))?;
                Ok(())
            });
        }

        let process = cmd.spawn()?;
        tracing::info!("Started greeter (pid={})", process.id());

        Ok(Self { process })
    }

    /// Check if greeter is still running
    pub fn is_running(&mut self) -> bool {
        matches!(self.process.try_wait(), Ok(None))
    }

    /// Wait for greeter to exit
    pub fn wait(&mut self) -> anyhow::Result<std::process::ExitStatus> {
        Ok(self.process.wait()?)
    }

    /// Kill the greeter
    pub fn kill(&mut self) -> anyhow::Result<()> {
        self.process.kill()?;
        self.process.wait()?;
        Ok(())
    }
}
```

### 2.4 Session Launcher

```rust
// gardmd/src/session.rs

use nix::unistd::{Uid, Gid, setuid, setgid, User, initgroups, chdir};
use std::process::{Command, Child};
use std::ffi::CString;

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
    ) -> anyhow::Result<Self> {
        let user = User::from_name(username)?
            .ok_or_else(|| anyhow::anyhow!("User {} not found", username))?;

        let home = user.dir.to_string_lossy().to_string();
        let shell = user.shell.to_string_lossy().to_string();
        let uid = user.uid;
        let gid = user.gid;
        let username_c = CString::new(username)?;

        // Build session command
        let (cmd_path, cmd_args) = if session_cmd.is_empty() {
            // Default to gar-session.sh
            ("/usr/local/bin/gar-session.sh".to_string(), vec![])
        } else {
            (session_cmd[0].clone(), session_cmd[1..].to_vec())
        };

        let mut cmd = Command::new(&cmd_path);
        cmd.args(&cmd_args)
           .env("DISPLAY", display)
           .env("HOME", &home)
           .env("USER", username)
           .env("LOGNAME", username)
           .env("SHELL", &shell)
           .env("XDG_SESSION_TYPE", "x11")
           .env("XDG_VTNR", vt.to_string())
           .env("XDG_SEAT", "seat0");

        // Run as the user
        unsafe {
            cmd.pre_exec(move || {
                // Set groups
                initgroups(&username_c, gid)?;
                setgid(gid)?;
                setuid(uid)?;

                // Change to home directory
                let home_c = CString::new(home.as_str())?;
                chdir(&home_c)?;

                Ok(())
            });
        }

        let process = cmd.spawn()?;
        tracing::info!("Started session for {} (pid={})", username, process.id());

        Ok(Self {
            process,
            username: username.to_string(),
        })
    }

    /// Wait for session to end
    pub fn wait(&mut self) -> anyhow::Result<std::process::ExitStatus> {
        Ok(self.process.wait()?)
    }
}
```

### 2.5 Main Loop Integration

```rust
// gardmd/src/main.rs

async fn run() -> anyhow::Result<()> {
    let config = Config::load()?;

    // Find VT to use
    let vt = if config.general.vt > 0 {
        config.general.vt
    } else {
        vt::find_unused_vt()?
    };

    // Start X server
    let display = ":0";
    let mut x_server = XServer::start(display, vt)?;

    // Switch to our VT
    vt::switch_to_vt(vt)?;

    // Start IPC server
    let ipc = IpcServer::new("/run/gardm.sock").await?;
    let mut auth = AuthSession::new();

    sd_notify::notify(true, &[sd_notify::NotifyState::Ready])?;

    loop {
        // Start greeter
        let mut greeter = GreeterProcess::start(&config.general.greeter, display)?;

        // Handle greeter authentication
        let session_info = handle_greeter_auth(&ipc, &mut auth).await?;

        // Kill greeter before starting session
        greeter.kill()?;

        // Start user session
        let mut session = UserSession::start(
            &session_info.username,
            &session_info.cmd,
            display,
            vt,
        )?;

        // Wait for session to end
        let status = session.wait()?;
        tracing::info!("Session ended with status: {:?}", status);

        // Loop back to greeter
    }
}

struct SessionInfo {
    username: String,
    cmd: Vec<String>,
}

async fn handle_greeter_auth(
    ipc: &IpcServer,
    auth: &mut AuthSession,
) -> anyhow::Result<SessionInfo> {
    let mut client = ipc.accept().await?;

    loop {
        let request = client.read_request().await?
            .ok_or_else(|| anyhow::anyhow!("Greeter disconnected"))?;

        match request {
            Request::CreateSession { username } => {
                let response = auth.create_session(&username)?;
                client.send_response(&response.into()).await?;
            }
            Request::Authenticate { response } => {
                let result = auth.authenticate(&response)?;
                client.send_response(&result.clone().into()).await?;

                if matches!(result, AuthResponse::Success) {
                    // Read StartSession request
                    if let Some(Request::StartSession { cmd, .. }) =
                        client.read_request().await?
                    {
                        return Ok(SessionInfo {
                            username: auth.username().unwrap().to_string(),
                            cmd,
                        });
                    }
                }
            }
            Request::CancelSession => {
                auth.cancel();
                client.send_response(&Response::Success).await?;
            }
            _ => {
                client.send_response(&Response::Error {
                    message: "Unexpected request".to_string(),
                }).await?;
            }
        }
    }
}
```

## Acceptance Criteria

1. Xorg starts on specified VT
2. Greeter launches as unprivileged gardm user
3. After auth success, greeter is killed and session starts
4. Session runs as authenticated user with correct environment
5. After session logout, greeter restarts
6. X server crash triggers recovery (restart X + greeter)

## Pitfalls to Avoid

1. **Don't forget VT permissions** - Xorg needs access to /dev/tty*
2. **X server takes time to start** - must wait for it to be ready
3. **Environment inheritance** - don't leak root environment to session
4. **Group membership** - user needs video, audio groups for hardware access
5. **Home directory** - chdir to $HOME before starting session
6. **Don't block on X** - use async/spawn for process management

## Testing

```bash
# Test X server startup (need to be root, on real TTY)
sudo ./target/release/gardmd

# Alternative: Test in Xephyr (nested X)
Xephyr -br -ac -noreset -screen 1280x720 :1 &
DISPLAY=:1 ./target/release/gardm-greeter

# Create gardm user for testing
sudo useradd -r -s /usr/bin/nologin -d /var/lib/gardm gardm
sudo mkdir -p /var/lib/gardm
sudo chown gardm:gardm /var/lib/gardm
```

## Dependencies for This Sprint

```toml
# gardmd/Cargo.toml
[dependencies]
x11rb = "0.13"
libc = "0.2"
nix = { version = "0.27", features = ["user", "process", "ioctl"] }
```

## Next Sprint

Sprint 3 will build the graphical greeter UI with Cairo/Pango.

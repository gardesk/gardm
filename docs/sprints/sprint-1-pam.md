# Sprint 1: PAM Authentication

**Goal:** Implement PAM-based user authentication in the daemon with proper conversation handling.

## Objectives

- Integrate with PAM for user authentication
- Handle PAM conversation (prompts, messages)
- Create IPC server for greeter communication
- Implement session state machine
- Test authentication flow end-to-end

## Background: How PAM Works

PAM (Pluggable Authentication Modules) uses a "conversation" model:

1. Application calls `pam_authenticate()`
2. PAM modules may request information via conversation callback
3. Callback returns user input (password, OTP, etc.)
4. PAM returns success/failure

For a display manager, we need to bridge this conversation to the greeter UI:

```
Greeter ←→ IPC ←→ Daemon ←→ PAM ←→ System
```

## Tasks

### 1.1 Add PAM Dependency

The `pam` crate provides safe Rust bindings:

```toml
# gardmd/Cargo.toml
[dependencies]
pam = "0.8"
```

### 1.2 Create PAM Configuration

`/etc/pam.d/gardm`:

```
#%PAM-1.0
auth       required     pam_securetty.so
auth       requisite    pam_nologin.so
auth       include      system-local-login
account    include      system-local-login
session    include      system-local-login
password   include      system-local-login
```

### 1.3 Implement Authentication State Machine

```rust
// gardmd/src/auth.rs

use pam::Authenticator;
use std::ffi::CString;

pub enum AuthState {
    /// No active authentication
    Idle,
    /// Waiting for username
    AwaitingUsername,
    /// PAM conversation in progress
    Authenticating {
        username: String,
        authenticator: Authenticator<'static, PasswordConv>,
    },
    /// Authentication succeeded, ready to start session
    Authenticated {
        username: String,
    },
}

pub struct AuthSession {
    state: AuthState,
}

impl AuthSession {
    pub fn new() -> Self {
        Self { state: AuthState::Idle }
    }

    pub fn create_session(&mut self, username: &str) -> Result<AuthResponse> {
        let service = CString::new("gardm")?;
        let username_c = CString::new(username)?;

        // Create PAM authenticator
        let mut auth = Authenticator::with_password(&service)?;
        auth.get_handler().set_credentials(username_c, /* password later */);

        self.state = AuthState::Authenticating {
            username: username.to_string(),
            authenticator: auth,
        };

        // PAM will ask for password
        Ok(AuthResponse::Prompt {
            prompt: "Password:".to_string(),
            echo: false,
        })
    }

    pub fn authenticate(&mut self, response: &str) -> Result<AuthResponse> {
        match &mut self.state {
            AuthState::Authenticating { username, authenticator } => {
                // Provide password to PAM
                authenticator.get_handler().set_credentials(
                    CString::new(username.as_str())?,
                    CString::new(response)?,
                );

                // Attempt authentication
                match authenticator.authenticate() {
                    Ok(()) => {
                        let username = username.clone();
                        self.state = AuthState::Authenticated { username };
                        Ok(AuthResponse::Success)
                    }
                    Err(e) => {
                        self.state = AuthState::Idle;
                        Ok(AuthResponse::Error {
                            message: format!("Authentication failed: {}", e),
                        })
                    }
                }
            }
            _ => Ok(AuthResponse::Error {
                message: "No authentication in progress".to_string(),
            }),
        }
    }

    pub fn cancel(&mut self) {
        self.state = AuthState::Idle;
    }
}
```

### 1.4 Implement IPC Server

```rust
// gardmd/src/ipc.rs

use gardm_ipc::{Request, Response};
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct IpcServer {
    listener: UnixListener,
}

impl IpcServer {
    pub async fn new(path: &str) -> anyhow::Result<Self> {
        // Remove stale socket
        let _ = std::fs::remove_file(path);

        let listener = UnixListener::bind(path)?;

        // Set permissions (only gardm user can connect)
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;

        Ok(Self { listener })
    }

    pub async fn accept(&self) -> anyhow::Result<IpcClient> {
        let (stream, _) = self.listener.accept().await?;
        Ok(IpcClient::new(stream))
    }
}

pub struct IpcClient {
    reader: BufReader<tokio::net::unix::OwnedReadHalf>,
    writer: tokio::net::unix::OwnedWriteHalf,
}

impl IpcClient {
    pub fn new(stream: UnixStream) -> Self {
        let (read, write) = stream.into_split();
        Self {
            reader: BufReader::new(read),
            writer: write,
        }
    }

    pub async fn read_request(&mut self) -> anyhow::Result<Option<Request>> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line).await?;
        if n == 0 {
            return Ok(None);
        }
        Ok(Some(serde_json::from_str(&line)?))
    }

    pub async fn send_response(&mut self, response: &Response) -> anyhow::Result<()> {
        let json = serde_json::to_string(response)?;
        self.writer.write_all(json.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;
        Ok(())
    }
}
```

### 1.5 Wire Up Main Loop

```rust
// gardmd/src/main.rs

async fn run() -> anyhow::Result<()> {
    let config = Config::load()?;
    let ipc = IpcServer::new("/run/gardm.sock").await?;
    let mut auth = AuthSession::new();

    // Notify systemd we're ready
    sd_notify::notify(true, &[sd_notify::NotifyState::Ready])?;

    tracing::info!("gardmd ready, listening for connections");

    loop {
        let mut client = ipc.accept().await?;

        // Handle single client (greeter)
        while let Some(request) = client.read_request().await? {
            let response = match request {
                Request::CreateSession { username } => {
                    auth.create_session(&username)?
                }
                Request::Authenticate { response } => {
                    auth.authenticate(&response)?
                }
                Request::CancelSession => {
                    auth.cancel();
                    Response::Success
                }
                // Power actions handled in later sprint
                _ => Response::Error {
                    message: "Not implemented".to_string(),
                },
            };

            client.send_response(&response).await?;
        }
    }
}
```

### 1.6 Create Test Client

For testing without the full greeter:

```rust
// gardm-greeter/src/bin/test-auth.rs

use gardm_ipc::{Request, Response};
use std::io::{self, BufRead, Write};
use std::os::unix::net::UnixStream;

fn main() -> anyhow::Result<()> {
    let mut stream = UnixStream::connect("/run/gardm.sock")?;

    // Get username
    print!("Username: ");
    io::stdout().flush()?;
    let mut username = String::new();
    io::stdin().lock().read_line(&mut username)?;

    // Send create_session
    let req = Request::CreateSession {
        username: username.trim().to_string(),
    };
    writeln!(stream, "{}", serde_json::to_string(&req)?)?;

    // Read response
    let mut response = String::new();
    let mut reader = io::BufReader::new(&stream);
    reader.read_line(&mut response)?;
    println!("Response: {}", response);

    // Get password
    print!("Password: ");
    io::stdout().flush()?;
    let password = rpassword::read_password()?;

    // Send authenticate
    let req = Request::Authenticate { response: password };
    writeln!(stream, "{}", serde_json::to_string(&req)?)?;

    // Read response
    response.clear();
    reader.read_line(&mut response)?;
    println!("Response: {}", response);

    Ok(())
}
```

## Acceptance Criteria

1. Daemon accepts IPC connections from greeter
2. `CreateSession` initiates PAM conversation
3. `Authenticate` with correct password returns `Success`
4. `Authenticate` with wrong password returns `AuthError`
5. `CancelSession` resets state cleanly
6. Multiple auth attempts work without daemon restart
7. PAM config file is properly loaded

## Pitfalls to Avoid

1. **PAM is synchronous** - don't block the async runtime. Use `spawn_blocking` for PAM calls.
2. **Memory safety with passwords** - zero memory after use (pam crate should handle this).
3. **Don't store passwords** - only pass through to PAM immediately.
4. **Handle PAM errors gracefully** - account locked, expired password, etc. have specific error types.
5. **Test with real PAM** - mock tests are useful but test against real system too.
6. **Remember PAM is stateful** - can't call authenticate twice on same session.

## Testing

```bash
# Build and run daemon as root (required for PAM)
sudo RUST_LOG=debug ./target/release/gardmd &

# Test with CLI client
./target/release/test-auth

# Test wrong password
# Test account lockout (if configured)
# Test non-existent user
```

## Security Considerations

- Daemon must run as root for PAM access
- IPC socket must be protected (0600 permissions)
- Failed auth attempts should be rate-limited (PAM may do this)
- Log auth attempts but NOT passwords

## Dependencies for This Sprint

```toml
# gardmd/Cargo.toml
[dependencies]
pam = "0.8"
nix = { version = "0.27", features = ["user"] }

# gardm-greeter/Cargo.toml (for test client)
[dependencies]
rpassword = "7.0"
```

## Next Sprint

Sprint 2 will add X11 server management - starting Xorg and the greeter.

# Sprint 0: Project Setup

**Goal:** Establish project structure, build system, and minimal daemon that can start and stop cleanly.

## Objectives

- Initialize Rust workspace with two crates (gardmd, gardm-greeter)
- Set up shared library for IPC protocol
- Create systemd service file
- Establish logging infrastructure
- Basic daemon lifecycle (start, signal handling, shutdown)

## Tasks

### 0.1 Initialize Cargo Workspace

```
gardm/
├── Cargo.toml              # Workspace root
├── gardmd/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         # Daemon entry point
│       └── lib.rs
├── gardm-greeter/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         # Greeter entry point
│       └── lib.rs
└── gardm-ipc/
    ├── Cargo.toml
    └── src/
        └── lib.rs          # Shared IPC types
```

**Root Cargo.toml:**
```toml
[workspace]
members = ["gardmd", "gardm-greeter", "gardm-ipc"]
resolver = "2"

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1.0"
thiserror = "1.0"
```

### 0.2 Define IPC Protocol Types

In `gardm-ipc/src/lib.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    CreateSession { username: String },
    Authenticate { response: String },
    StartSession { cmd: Vec<String>, env: Vec<String> },
    CancelSession,
    Shutdown,
    Reboot,
    Suspend,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    Success,
    AuthPrompt { prompt: String, echo: bool },
    AuthInfo { message: String },
    AuthError { message: String },
    Error { message: String },
}
```

### 0.3 Basic Daemon Skeleton

In `gardmd/src/main.rs`:

```rust
use tokio::signal::unix::{signal, SignalKind};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging to journald
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    tracing::info!("gardmd starting");

    // Signal handling
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;

    tokio::select! {
        _ = sigterm.recv() => {
            tracing::info!("Received SIGTERM, shutting down");
        }
        _ = sigint.recv() => {
            tracing::info!("Received SIGINT, shutting down");
        }
    }

    tracing::info!("gardmd stopped");
    Ok(())
}
```

### 0.4 systemd Service File

Create `gardm.service`:

```ini
[Unit]
Description=gar Display Manager
Documentation=man:gardmd(8)
After=systemd-user-sessions.service getty@tty1.service plymouth-quit.service
Conflicts=getty@tty1.service

[Service]
Type=notify
ExecStart=/usr/bin/gardmd
Restart=always
RestartSec=1

# Security hardening
ProtectSystem=strict
ProtectHome=read-only
PrivateTmp=true
NoNewPrivileges=false  # Required for PAM
ReadWritePaths=/run

[Install]
Alias=display-manager.service
```

### 0.5 Configuration Scaffolding

Create `gardmd/src/config.rs`:

```rust
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub greeter: GreeterConfig,
}

#[derive(Debug, Deserialize, Default)]
pub struct GeneralConfig {
    #[serde(default = "default_session")]
    pub default_session: String,
    #[serde(default = "default_greeter")]
    pub greeter: PathBuf,
    #[serde(default)]
    pub vt: u32,
}

#[derive(Debug, Deserialize, Default)]
pub struct GreeterConfig {
    #[serde(default = "default_blur_radius")]
    pub blur_radius: u32,
}

fn default_session() -> String { "gar".to_string() }
fn default_greeter() -> PathBuf { "/usr/bin/gardm-greeter".into() }
fn default_blur_radius() -> u32 { 20 }

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let path = PathBuf::from("/etc/gardm/config.toml");
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Config::default())
        }
    }
}
```

## Acceptance Criteria

1. `cargo build --release` succeeds for all workspace members
2. `gardmd` starts, logs to stdout, and exits cleanly on SIGTERM
3. IPC types serialize/deserialize correctly (unit test)
4. Config loads from file or uses defaults
5. Service file passes `systemd-analyze verify`

## Pitfalls to Avoid

1. **Don't start X11 yet** - that's Sprint 2. Keep this sprint minimal.
2. **Don't implement PAM yet** - that's Sprint 1. Focus on structure.
3. **Avoid premature optimization** - get it working first.
4. **Don't forget `sd_notify`** - systemd needs to know when daemon is ready.
5. **Test without root first** - use `RUST_LOG=debug cargo run` for development.

## Testing

```bash
# Build
cargo build --release

# Run daemon in foreground (Ctrl+C to stop)
RUST_LOG=debug ./target/release/gardmd

# Verify signal handling
kill -TERM $(pidof gardmd)

# Test config loading
mkdir -p /tmp/gardm-test
echo '[general]' > /tmp/gardm-test/config.toml
echo 'default_session = "test"' >> /tmp/gardm-test/config.toml
```

## Dependencies for This Sprint

```toml
# gardmd/Cargo.toml
[dependencies]
gardm-ipc = { path = "../gardm-ipc" }
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
anyhow = { workspace = true }
serde = { workspace = true }
toml = "0.8"
sd-notify = "0.4"
```

## Next Sprint

Sprint 1 will add PAM authentication to the daemon.

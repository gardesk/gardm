# gardm Architecture

## Overview

gardm is the display manager for the gar desktop suite. It follows a **daemon + greeter** architecture similar to [greetd](https://github.com/kennylevinsen/greetd), providing clean separation between authentication/session management and the user interface.

## Components

```
┌─────────────────────────────────────────────────────────────┐
│                         gardmd                               │
│                   (Display Manager Daemon)                   │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │    PAM      │  │   X11/Xorg  │  │  Session Manager    │  │
│  │ Integration │  │   Control   │  │  (systemd-logind)   │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
│                          │                                   │
│                    Unix Socket IPC                           │
│                          │                                   │
└──────────────────────────┼──────────────────────────────────┘
                           │
┌──────────────────────────┼──────────────────────────────────┐
│                          ▼                                   │
│                   gardm-greeter                              │
│                   (Graphical UI)                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Cairo/    │  │   garbg     │  │    User Input       │  │
│  │   Pango     │  │ Integration │  │   (login form)      │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### gardmd (Daemon)

The privileged daemon that runs as root. Responsibilities:

1. **X11 Server Management**: Start/stop Xorg server on appropriate VT
2. **PAM Authentication**: Verify user credentials via `/etc/pam.d/gardm`
3. **Session Launching**: Start selected session (gar, other WMs/DEs)
4. **systemd-logind Integration**: Register sessions, handle multi-seat
5. **IPC Server**: Listen on Unix socket for greeter commands

### gardm-greeter (Frontend)

The unprivileged graphical interface. Runs as dedicated `gardm` user. Responsibilities:

1. **User Interface**: Render login form, session selector, power buttons
2. **garbg Integration**: Display same wallpaper that gar session will use
3. **Visual Effects**: Blur background, animations, theming
4. **IPC Client**: Send auth requests to daemon, receive responses

## IPC Protocol

JSON-based protocol over Unix socket at `/run/gardm.sock`:

### Requests (Greeter → Daemon)

```json
// Create authentication session
{ "type": "create_session", "username": "user" }

// Attempt authentication
{ "type": "authenticate", "response": "password123" }

// Start session after successful auth
{ "type": "start_session", "cmd": ["gar-session.sh"], "env": [] }

// Cancel current auth attempt
{ "type": "cancel_session" }

// System actions
{ "type": "shutdown" }
{ "type": "reboot" }
{ "type": "suspend" }
```

### Responses (Daemon → Greeter)

```json
// Success
{ "type": "success" }

// Auth prompt (PAM asking for input)
{ "type": "auth_prompt", "prompt": "Password:", "echo": false }

// Auth info message
{ "type": "auth_info", "message": "..." }

// Auth error
{ "type": "auth_error", "message": "Authentication failed" }

// General error
{ "type": "error", "message": "..." }
```

## Session Discovery

Sessions are discovered from standard locations:

- `/usr/share/xsessions/*.desktop` - X11 sessions
- `/usr/share/wayland-sessions/*.desktop` - Wayland sessions (future)

gar session file example (`/usr/share/xsessions/gar.desktop`):
```ini
[Desktop Entry]
Name=gar
Comment=Tiling window manager with smart splits
Exec=/usr/local/bin/gar-session.sh
Type=XSession
DesktopNames=gar
```

## User Discovery

Users are enumerated from:

- `/etc/passwd` (filter by UID range, shell, home directory existence)
- AccountsService D-Bus interface (if available)

Hidden users (UID < 1000, nologin shell, system accounts) are filtered out.

## garbg Integration

The greeter reads garbg's config to display the same wallpaper:

1. Read `~/.config/garbg/config.toml` for default wallpaper source
2. Or read playlist state from `$XDG_RUNTIME_DIR/garbg-state.json`
3. Apply blur effect for greeter aesthetic
4. On successful login, gar session starts garbg which shows same image (seamless transition)

## Security Model

- **gardmd**: Runs as root, handles PAM, minimal attack surface
- **gardm-greeter**: Runs as unprivileged `gardm` user
- **IPC**: Socket permissions restrict access to gardm user
- **X11**: Greeter runs on isolated X server started by daemon

## Configuration

`/etc/gardm/config.toml`:

```toml
[general]
# Default session if user hasn't selected one
default_session = "gar"

# Greeter command
greeter = "/usr/bin/gardm-greeter"

# VT to use (0 = auto-select)
vt = 0

[greeter]
# Theme settings
blur_radius = 20
blur_brightness = 0.7

# Show/hide elements
show_power_buttons = true
show_session_selector = true

# garbg integration
use_garbg_wallpaper = true
fallback_wallpaper = "/usr/share/gardm/backgrounds/default.jpg"

[security]
# Allow empty passwords
allow_empty_password = false

# Lock after N failed attempts (0 = disabled)
lockout_attempts = 5
lockout_duration = 300
```

## PAM Configuration

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

## systemd Integration

`/usr/lib/systemd/system/gardm.service`:

```ini
[Unit]
Description=gar Display Manager
After=systemd-user-sessions.service getty@tty1.service plymouth-quit.service
Conflicts=getty@tty1.service

[Service]
ExecStart=/usr/bin/gardmd
Restart=always

[Install]
Alias=display-manager.service
```

## Key Dependencies

| Component | Crates |
|-----------|--------|
| gardmd | pam, nix, sd-notify, serde, tokio |
| gardm-greeter | x11rb, cairo-rs, pango, image, serde |

## References

- [freedesktop.org: Writing Display Managers](https://systemd.io/WRITING_DISPLAY_MANAGERS/)
- [greetd](https://github.com/kennylevinsen/greetd) - Architecture inspiration
- [SDDM](https://github.com/sddm/sddm) - Feature reference
- [LightDM](https://github.com/canonical/lightdm) - Protocol reference

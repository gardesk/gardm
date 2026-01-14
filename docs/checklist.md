# gardm Feature Checklist

## Core Functionality

### Authentication
- [ ] PAM integration for password authentication
- [ ] Multi-factor auth support (PAM handles this transparently)
- [ ] Account lockout after failed attempts
- [ ] "Remember last user" functionality
- [ ] Guest session support (optional)

### Session Management
- [ ] Discover X11 sessions from `/usr/share/xsessions/`
- [ ] Parse .desktop files for session metadata
- [ ] Remember user's last session choice
- [ ] Support custom session commands
- [ ] Proper session environment setup (DISPLAY, XDG_*, etc.)
- [ ] systemd-logind session registration

### X11 Server Management
- [ ] Start Xorg on appropriate VT
- [ ] Handle X server crashes gracefully
- [ ] Support multi-seat (seat0 initially)
- [ ] Pass correct display/VT to sessions
- [ ] Clean X server shutdown on logout

### Power Management
- [ ] Shutdown button (requires polkit or root)
- [ ] Reboot button
- [ ] Suspend button
- [ ] Hibernate button (if available)
- [ ] Scheduled actions (shutdown in 5 min, etc.) - optional

## User Interface

### Login Form
- [ ] Username input (with dropdown of available users)
- [ ] Password input (masked)
- [ ] Session selector dropdown
- [ ] Login button
- [ ] Clear error messages on auth failure
- [ ] Loading/spinner during authentication

### Visual Design
- [ ] Centered login form
- [ ] Blurred background image
- [ ] Configurable blur radius/brightness
- [ ] Smooth fade-in on greeter start
- [ ] Smooth transition to session (fade-out)
- [ ] Clock display
- [ ] Current date display

### User List
- [ ] Show available users with avatars
- [ ] Filter system accounts (UID < 1000)
- [ ] Filter accounts with nologin shell
- [ ] Support user icons from AccountsService
- [ ] Keyboard navigation through user list

### Theming
- [ ] Configurable colors/fonts via config file
- [ ] Support custom background images
- [ ] garbg wallpaper integration
- [ ] Logo/branding support
- [ ] Dark/light theme variants

### Accessibility
- [ ] Keyboard-only navigation (Tab, Enter, Escape)
- [ ] High contrast mode (optional)
- [ ] Screen reader support (optional, complex)
- [ ] On-screen keyboard (optional, complex)

## gar Integration

### garbg Wallpaper Sync
- [ ] Read garbg config for default wallpaper
- [ ] Read garbg playlist state for current wallpaper
- [ ] Apply consistent blur to match gar lock screen
- [ ] Seamless visual transition to gar session

### Shared Configuration
- [ ] Read gar color scheme for consistent theming
- [ ] Use same fonts as garbar
- [ ] Respect gar's corner radius settings

## System Integration

### systemd
- [ ] Proper service file
- [ ] Socket activation (optional)
- [ ] sd_notify for readiness
- [ ] Proper shutdown handling (SIGTERM)
- [ ] Alias as display-manager.service

### D-Bus
- [ ] systemd-logind integration for session management
- [ ] AccountsService for user info (optional)
- [ ] polkit for power actions (optional)

### Files and Paths
- [ ] Config file: `/etc/gardm/config.toml`
- [ ] PAM config: `/etc/pam.d/gardm`
- [ ] Socket: `/run/gardm.sock`
- [ ] PID file: `/run/gardm.pid`
- [ ] Log file: via journald
- [ ] State dir: `/var/lib/gardm/`

## Security

### Hardening
- [ ] Greeter runs as unprivileged user
- [ ] Minimal daemon attack surface
- [ ] Socket permissions (0600, gardm:gardm)
- [ ] No credentials in logs
- [ ] Secure memory handling for passwords

### Input Validation
- [ ] Sanitize username input
- [ ] Prevent path traversal in session selection
- [ ] Rate limiting on auth attempts
- [ ] Timeout for idle greeter

## Error Handling

- [ ] X server fail to start
- [ ] PAM errors (account locked, expired, etc.)
- [ ] Session fail to start
- [ ] IPC communication errors
- [ ] Missing configuration graceful fallback
- [ ] Greeter crash recovery (restart greeter)

## Testing

- [ ] Unit tests for IPC protocol
- [ ] Unit tests for session discovery
- [ ] Integration tests with mock PAM
- [ ] Manual testing checklist
- [ ] Xephyr-based testing for greeter UI

## Documentation

- [ ] README with quick start
- [ ] Configuration reference
- [ ] Theming guide
- [ ] Troubleshooting guide
- [ ] Man pages (gardmd.8, gardm-greeter.1)

## Nice-to-Have (Future)

- [ ] Wayland greeter support
- [ ] Network login (LDAP, etc.)
- [ ] Fingerprint authentication
- [ ] Auto-login configuration
- [ ] Remote desktop support
- [ ] Multi-monitor greeter
- [ ] Animated backgrounds
- [ ] Weather/calendar widgets

# Sprint 5: Power Buttons and Session Selector

**Goal:** Add power control buttons (shutdown, reboot, suspend) and a session selector dropdown to the greeter UI.

## Completed Features

### Power Buttons
- Custom Cairo-drawn icons (no external dependencies):
  - Power symbol (circle with vertical line gap at top)
  - Reboot (circular arrows)
  - Suspend (crescent moon)
- Bottom-right corner positioning
- Hover effects with color-coded highlights:
  - Shutdown: Red tint
  - Reboot: Blue tint
  - Suspend: Gold tint
- Semi-transparent backgrounds that glow on hover
- IPC integration for power actions

### Session Selector
- Dropdown positioned below login form
- Lists sessions from daemon via `ListSessions` IPC
- Dropdown opens upward above the button
- Hover highlighting on items
- Checkmark indicator on selected session
- Selected session's exec command used for login

### Supporting Changes
- Added `POINTER_MOTION` to X11 event mask for hover tracking
- Mouse position tracking in main loop
- Click handling for interactive elements

## Files Added/Modified
- `gardm-greeter/src/icons.rs` - Custom icon drawing functions
- `gardm-greeter/src/widgets/power_buttons.rs` - Power button widget
- `gardm-greeter/src/widgets/session_selector.rs` - Session dropdown widget
- `gardm-greeter/src/main.rs` - Event handling integration
- `gardm-greeter/src/window.rs` - Mouse motion events

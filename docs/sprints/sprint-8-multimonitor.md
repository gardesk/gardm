# Sprint 8: Multi-Monitor Support

**Goal:** Properly handle multi-monitor setups by detecting all monitors via RandR and positioning the UI appropriately.

## Objectives

1. Detect all connected monitors using XRandR
2. Create window spanning entire virtual screen
3. Render background on all monitors
4. Center login UI on primary monitor
5. Handle monitor hotplug events (optional)

## Implementation

### Monitor Detection
```rust
// Using x11rb's randr extension
use x11rb::protocol::randr::{self, ConnectionExt as _};

struct Monitor {
    x: i16,
    y: i16,
    width: u16,
    height: u16,
    primary: bool,
    name: String,
}

fn get_monitors(conn: &RustConnection, root: Window) -> Result<Vec<Monitor>> {
    let resources = conn.randr_get_screen_resources(root)?.reply()?;
    let mut monitors = Vec::new();

    for output in resources.outputs {
        let output_info = conn.randr_get_output_info(output, 0)?.reply()?;
        if output_info.connection == randr::Connection::CONNECTED {
            if let Some(crtc) = output_info.crtc {
                let crtc_info = conn.randr_get_crtc_info(crtc, 0)?.reply()?;
                monitors.push(Monitor {
                    x: crtc_info.x,
                    y: crtc_info.y,
                    width: crtc_info.width,
                    height: crtc_info.height,
                    primary: false, // Check via randr_get_output_primary
                    name: String::from_utf8_lossy(&output_info.name).to_string(),
                });
            }
        }
    }

    // Mark primary
    let primary = conn.randr_get_output_primary(root)?.reply()?;
    // ... mark the matching monitor as primary

    Ok(monitors)
}
```

### Window Spanning
- Window covers entire virtual screen (union of all monitors)
- Use root window dimensions for total size
- Background rendered per-monitor with proper offsets

### UI Centering
- Find primary monitor (or largest if no primary set)
- Calculate center point of primary monitor
- Offset all UI element positions relative to primary center
- Login form, user list, session selector all centered on primary

### Background Rendering
- Load background image once
- For each monitor:
  - Scale/crop background to monitor dimensions
  - Apply blur and brightness
  - Render at monitor's x,y offset in the virtual screen

### Example Layout
```
┌─────────────────┬─────────────────────────┐
│                 │                         │
│   Monitor 2     │      Monitor 1          │
│   1920x1080     │      2560x1440          │
│   (secondary)   │      (primary)          │
│                 │                         │
│   [background]  │   [background + UI]     │
│                 │                         │
└─────────────────┴─────────────────────────┘
```

## Testing
```bash
# Test with Xephyr multi-head (limited)
# Better: test on actual multi-monitor setup

# Check detected monitors
xrandr --query

# Verify greeter spans all monitors
DISPLAY=:0 ./target/release/gardm-greeter
```

## Files to Modify
- `gardm-greeter/src/window.rs` - Add RandR detection
- `gardm-greeter/src/main.rs` - Pass monitor info to widgets
- `gardm-greeter/src/background.rs` - Per-monitor rendering
- Widget files - Accept center offset parameter

# Sprint 7: Accessibility and Theming

**Goal:** Improve accessibility features and add theming support for visual customization.

## Planned Features

### Accessibility
- [ ] High contrast mode
- [ ] Larger text option
- [ ] Keyboard navigation indicators (visible focus rings)
- [ ] Screen reader hints (if feasible with X11)
- [ ] Reduced motion option (disable fade transitions)

### Theming
- [ ] Theme configuration in `/etc/gardm/theme.toml`
- [ ] Customizable colors:
  - Background overlay color/opacity
  - Panel background color
  - Text colors (primary, secondary, error, info)
  - Accent color for focus/selection
  - Button colors
- [ ] Custom font selection
- [ ] Custom logo/branding image
- [ ] Corner radius customization

### Configuration Options
```toml
[theme]
# Colors (CSS-style hex or rgba)
background_overlay = "rgba(0, 0, 0, 0.6)"
panel_background = "rgba(20, 20, 20, 0.9)"
text_primary = "#ffffff"
text_secondary = "#cccccc"
text_error = "#ff6b6b"
text_info = "#6bff6b"
accent = "#4a90d9"

# Typography
font_family = "Sans"
font_size_normal = 14
font_size_large = 18
font_size_title = 24

# Layout
corner_radius = 16
panel_padding = 24

[accessibility]
high_contrast = false
large_text = false
reduce_motion = false
```

### Implementation Notes
- Theme loaded at startup from config
- Fallback to defaults if theme file missing
- Colors parsed from hex (#RRGGBB) or rgba() format
- High contrast mode overrides theme colors with maximum contrast values

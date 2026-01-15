# Sprint 6: User List with Avatars

**Goal:** Display available users with circular avatars for quick selection, allowing click-to-login workflow.

## Completed Features

### Avatar Loading
- Searches standard avatar locations:
  - `~/.face` (freedesktop standard)
  - `~/.face.icon`
  - `~/.face.png`
  - `~/.config/face.png`
  - `/var/lib/AccountsService/icons/<username>`
- Image processing:
  - Center crop to square
  - Scale with Lanczos3 filter for quality
  - RGBA to BGRA conversion for Cairo
- Caching to avoid reloading

### Fallback Avatars
- Colored circle with user initials
- Hue derived from username hash for consistency
- First + last initial for full names, single initial for usernames

### User List Widget
- Horizontal row positioned above login form
- Circular avatar rendering with clipping
- Username/first name displayed below avatar
- Hover effect: semi-transparent highlight background
- Selection effect: blue ring around avatar
- Click behavior:
  - Populates username field
  - Clears password field
  - Focuses password field
  - Clears any error messages

## Files Added
- `gardm-greeter/src/avatar.rs` - Avatar loading and rendering
- `gardm-greeter/src/widgets/user_list.rs` - User list widget

## IPC Integration
- Uses `ListUsers` request to get available users
- User info includes: name, full_name, home directory, avatar path

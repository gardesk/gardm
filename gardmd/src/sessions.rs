//! Session and user enumeration
//!
//! Discovers available X11/Wayland sessions and system users.

use gardm_ipc::{SessionInfo, UserInfo};
use std::fs;
use std::path::{Path, PathBuf};

/// Directories to search for X11 session .desktop files
const XSESSION_DIRS: &[&str] = &[
    "/usr/share/xsessions",
    "/usr/local/share/xsessions",
];

/// Directories to search for Wayland session .desktop files
const WAYLAND_SESSION_DIRS: &[&str] = &[
    "/usr/share/wayland-sessions",
    "/usr/local/share/wayland-sessions",
];

/// Minimum UID for regular users (typically 1000 on most systems)
const MIN_UID: u32 = 1000;

/// Maximum UID for regular users
const MAX_UID: u32 = 60000;

/// Enumerate available sessions from .desktop files
/// Returns both X11 and Wayland sessions. The daemon handles launching
/// each session type appropriately (X11 on existing server, Wayland directly on VT).
pub fn list_sessions() -> Vec<SessionInfo> {
    let mut sessions = Vec::new();

    // X11 sessions
    for dir in XSESSION_DIRS {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Some(session) = parse_desktop_file(&entry.path(), "x11") {
                    sessions.push(session);
                }
            }
        }
    }

    // Wayland sessions
    for dir in WAYLAND_SESSION_DIRS {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Some(session) = parse_desktop_file(&entry.path(), "wayland") {
                    sessions.push(session);
                }
            }
        }
    }

    // Sort by name for consistent ordering
    sessions.sort_by(|a, b| a.name.cmp(&b.name));

    sessions
}

/// Parse a .desktop file into SessionInfo
fn parse_desktop_file(path: &Path, session_type: &str) -> Option<SessionInfo> {
    let filename = path.file_name()?.to_str()?;
    if !filename.ends_with(".desktop") {
        return None;
    }

    let content = fs::read_to_string(path).ok()?;
    let mut name = None;
    let mut comment = None;
    let mut exec = None;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("Name=") {
            name = Some(line.strip_prefix("Name=")?.to_string());
        } else if line.starts_with("Comment=") {
            comment = Some(line.strip_prefix("Comment=")?.to_string());
        } else if line.starts_with("Exec=") {
            exec = Some(line.strip_prefix("Exec=")?.to_string());
        }
    }

    let id = filename.strip_suffix(".desktop")?.to_string();
    let name = name.unwrap_or_else(|| id.clone());
    let exec = exec?;

    Some(SessionInfo {
        id,
        name,
        comment,
        exec,
        session_type: session_type.to_string(),
    })
}

/// Enumerate users suitable for login
pub fn list_users() -> Vec<UserInfo> {
    let mut users = Vec::new();

    // Read /etc/passwd
    if let Ok(content) = fs::read_to_string("/etc/passwd") {
        for line in content.lines() {
            if let Some(user) = parse_passwd_line(line) {
                users.push(user);
            }
        }
    }

    // Sort by username
    users.sort_by(|a, b| a.name.cmp(&b.name));

    users
}

/// Parse a line from /etc/passwd
fn parse_passwd_line(line: &str) -> Option<UserInfo> {
    let fields: Vec<&str> = line.split(':').collect();
    if fields.len() < 7 {
        return None;
    }

    let name = fields[0].to_string();
    let uid: u32 = fields[2].parse().ok()?;
    let gecos = fields[4];
    let home = PathBuf::from(fields[5]);
    let shell = fields[6];

    // Filter: only regular users (UID in range, valid shell)
    if uid < MIN_UID || uid > MAX_UID {
        return None;
    }

    // Skip users with nologin shell
    if shell.contains("nologin") || shell.contains("false") {
        return None;
    }

    // Parse GECOS field (full name is first comma-separated field)
    let full_name = if gecos.is_empty() {
        None
    } else {
        Some(gecos.split(',').next()?.to_string())
    };

    // Check for user avatar
    let avatar = find_user_avatar(&name, &home);

    Some(UserInfo {
        name,
        full_name,
        home,
        avatar,
    })
}

/// Find user avatar image if available
fn find_user_avatar(username: &str, home: &Path) -> Option<PathBuf> {
    // Check common avatar locations
    let candidates = [
        home.join(".face"),
        home.join(".face.icon"),
        PathBuf::from(format!("/var/lib/AccountsService/icons/{}", username)),
    ];

    for path in &candidates {
        if path.exists() {
            return Some(path.clone());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_sessions() {
        // This test may return empty on systems without xsessions
        let sessions = list_sessions();
        // Just verify it doesn't panic
        println!("Found {} sessions", sessions.len());
    }

    #[test]
    fn test_list_users() {
        let users = list_users();
        // Should find at least the current user on most systems
        println!("Found {} users", users.len());
    }
}

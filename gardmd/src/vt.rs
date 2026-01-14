//! Virtual Terminal management
//!
//! Allocates and switches VTs for X server and sessions.

use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;

// VT ioctl constants
const VT_OPENQRY: libc::c_ulong = 0x5600;
const VT_ACTIVATE: libc::c_ulong = 0x5606;
const VT_WAITACTIVE: libc::c_ulong = 0x5607;

/// Find an unused VT
pub fn find_unused_vt() -> Result<u32> {
    let console = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty0")
        .context("Failed to open /dev/tty0")?;

    let mut vt: libc::c_int = 0;
    let result = unsafe { libc::ioctl(console.as_raw_fd(), VT_OPENQRY, &mut vt) };

    if result < 0 {
        anyhow::bail!(
            "VT_OPENQRY ioctl failed: {}",
            std::io::Error::last_os_error()
        );
    }

    if vt <= 0 {
        anyhow::bail!("No unused VT available");
    }

    tracing::debug!("Found unused VT: {}", vt);
    Ok(vt as u32)
}

/// Switch to a specific VT
pub fn switch_to_vt(vt: u32) -> Result<()> {
    let console = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty0")
        .context("Failed to open /dev/tty0")?;

    let fd = console.as_raw_fd();

    // Activate the VT
    let result = unsafe { libc::ioctl(fd, VT_ACTIVATE, vt as libc::c_int) };
    if result < 0 {
        anyhow::bail!(
            "VT_ACTIVATE failed for VT {}: {}",
            vt,
            std::io::Error::last_os_error()
        );
    }

    // Wait for it to become active
    let result = unsafe { libc::ioctl(fd, VT_WAITACTIVE, vt as libc::c_int) };
    if result < 0 {
        anyhow::bail!(
            "VT_WAITACTIVE failed for VT {}: {}",
            vt,
            std::io::Error::last_os_error()
        );
    }

    tracing::info!("Switched to VT {}", vt);
    Ok(())
}

/// Get the currently active VT
pub fn get_active_vt() -> Result<u32> {
    #[repr(C)]
    struct VtStat {
        v_active: u16,
        v_signal: u16,
        v_state: u16,
    }

    const VT_GETSTATE: libc::c_ulong = 0x5603;

    let console = OpenOptions::new()
        .read(true)
        .open("/dev/tty0")
        .context("Failed to open /dev/tty0")?;

    let mut stat = VtStat {
        v_active: 0,
        v_signal: 0,
        v_state: 0,
    };

    let result = unsafe { libc::ioctl(console.as_raw_fd(), VT_GETSTATE, &mut stat) };

    if result < 0 {
        anyhow::bail!(
            "VT_GETSTATE failed: {}",
            std::io::Error::last_os_error()
        );
    }

    Ok(stat.v_active as u32)
}

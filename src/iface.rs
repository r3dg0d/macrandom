//! Network interface discovery and MAC read/write via sysfs + ip link.

use crate::classify::{classify, InterfaceKind};
use crate::error::{MacrandomError, Result};
use crate::mac::MacAddress;
use crate::skip::{should_skip, skip_reason};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use tracing::{debug, warn};

const SYS_NET: &str = "/sys/class/net";

/// Snapshot of a network interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interface {
    pub name: String,
    pub kind: InterfaceKind,
    pub current_mac: Option<MacAddress>,
    pub permanent_mac: Option<MacAddress>,
    pub operstate: Option<String>,
    pub is_up: bool,
    pub skipped_by_default: bool,
    pub skip_reason: Option<String>,
    #[serde(skip)]
    pub sys_path: PathBuf,
}

impl Interface {
    pub fn from_sys(name: &str) -> Result<Self> {
        let sys_path = PathBuf::from(SYS_NET).join(name);
        if !sys_path.exists() {
            return Err(MacrandomError::InterfaceNotFound(name.to_string()));
        }
        let kind = classify(name, Some(&sys_path));
        let current_mac = read_sys_mac(&sys_path.join("address")).ok();
        let permanent_mac = discover_permanent_mac(name, &sys_path, current_mac.as_ref());
        let operstate = fs::read_to_string(sys_path.join("operstate"))
            .ok()
            .map(|s| s.trim().to_string());
        let flags = fs::read_to_string(sys_path.join("flags"))
            .ok()
            .and_then(|s| u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok());
        // IFF_UP = 0x1
        let is_up = flags.map(|f| f & 0x1 != 0).unwrap_or(false);
        let skipped = should_skip(name, kind);
        let reason = skip_reason(name, kind).map(|s| s.to_string());

        Ok(Self {
            name: name.to_string(),
            kind,
            current_mac,
            permanent_mac,
            operstate,
            is_up,
            skipped_by_default: skipped,
            skip_reason: reason,
            sys_path,
        })
    }

    /// Merge config `extra_skip` prefixes into the skip decision.
    pub fn with_extra_skip(mut self, extra: &[String]) -> Self {
        use crate::skip::should_skip_with_extra;
        if !self.skipped_by_default && should_skip_with_extra(&self.name, self.kind, extra) {
            self.skipped_by_default = true;
            self.skip_reason = Some("matches config extra_skip".into());
        }
        self
    }
}

fn read_sys_mac(path: &Path) -> Result<MacAddress> {
    let s = fs::read_to_string(path).map_err(|e| MacrandomError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    MacAddress::from_str(s.trim())
}

/// Try ethtool -P, then sysfs `address` for devices that expose permanent via ethtool.
fn discover_permanent_mac(
    name: &str,
    sys_path: &Path,
    current: Option<&MacAddress>,
) -> Option<MacAddress> {
    // ethtool -P <iface>
    if let Some(mac) = ethtool_permanent(name) {
        return Some(mac);
    }
    // Some drivers expose permanent address differently; fall back to current
    // only when we have nothing better — callers treat permanent==current as unknown HW.
    let _ = sys_path;
    current.copied()
}

fn ethtool_permanent(name: &str) -> Option<MacAddress> {
    let output = Command::new("ethtool").args(["-P", name]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    // "Permanent address: aa:bb:cc:dd:ee:ff"
    for line in stdout.lines() {
        if let Some(rest) = line
            .strip_prefix("Permanent address:")
            .or_else(|| line.strip_prefix("permanent address:"))
        {
            if let Ok(mac) = MacAddress::from_str(rest.trim()) {
                // ethtool returns 00:00:00:00:00:00 when unknown
                if mac.as_bytes() != &[0, 0, 0, 0, 0, 0] {
                    return Some(mac);
                }
            }
        }
    }
    None
}

/// List all interfaces under /sys/class/net.
pub fn list_interfaces() -> Result<Vec<Interface>> {
    let mut ifaces = Vec::new();
    let entries = fs::read_dir(SYS_NET).map_err(|e| MacrandomError::Io {
        path: PathBuf::from(SYS_NET),
        source: e,
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| MacrandomError::Io {
            path: PathBuf::from(SYS_NET),
            source: e,
        })?;
        let name = entry.file_name().to_string_lossy().to_string();
        match Interface::from_sys(&name) {
            Ok(iface) => ifaces.push(iface),
            Err(e) => warn!("skipping interface {name}: {e}"),
        }
    }
    ifaces.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(ifaces)
}

/// Bring interface down via `ip link set dev X down`.
pub fn link_set_down(name: &str, dry_run: bool) -> Result<()> {
    run_ip(&["link", "set", "dev", name, "down"], dry_run)
}

/// Bring interface up.
pub fn link_set_up(name: &str, dry_run: bool) -> Result<()> {
    run_ip(&["link", "set", "dev", name, "up"], dry_run)
}

/// Set MAC address via `ip link set dev X address Y`.
pub fn link_set_address(name: &str, mac: &MacAddress, dry_run: bool) -> Result<()> {
    let mac_s = mac.to_string_canonical();
    run_ip(&["link", "set", "dev", name, "address", &mac_s], dry_run)
}

fn run_ip(args: &[&str], dry_run: bool) -> Result<()> {
    let cmdline = format!("ip {}", args.join(" "));
    if dry_run {
        tracing::info!("[dry-run] would run: {cmdline}");
        return Ok(());
    }
    debug!("running: {cmdline}");
    let output = Command::new("ip").args(args).output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            MacrandomError::PermissionDenied(args.get(3).unwrap_or(&"?").to_string())
        } else {
            MacrandomError::CommandFailed {
                cmd: cmdline.clone(),
                source: e,
            }
        }
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let status = output.status.code().unwrap_or(1);
        // Common: Operation not permitted
        if stderr.to_lowercase().contains("not permitted")
            || stderr.to_lowercase().contains("permission")
        {
            return Err(MacrandomError::PermissionDenied(
                args.iter()
                    .position(|&a| a == "dev")
                    .and_then(|i| args.get(i + 1))
                    .unwrap_or(&"?")
                    .to_string(),
            ));
        }
        return Err(MacrandomError::CommandExit {
            cmd: cmdline,
            status,
            stderr,
        });
    }
    Ok(())
}

/// Change MAC: down → set address → up (if was up). Respects dry_run.
pub fn change_mac(iface: &Interface, new_mac: &MacAddress, dry_run: bool) -> Result<()> {
    let was_up = iface.is_up;
    if was_up {
        link_set_down(&iface.name, dry_run)?;
    }
    let result = link_set_address(&iface.name, new_mac, dry_run);
    if was_up {
        // Always try to restore up state even if set failed.
        if let Err(e) = link_set_up(&iface.name, dry_run) {
            warn!("failed to bring {} back up: {e}", iface.name);
            if result.is_ok() {
                return Err(e);
            }
        }
    }
    result
}

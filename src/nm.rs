//! NetworkManager integration via nmcli.

use crate::error::{MacrandomError, Result};
use crate::mac::MacAddress;
use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkBackend {
    NetworkManager,
    SystemdNetworkd,
    Neither,
    Both,
}

/// Detect which network management backends appear active.
pub fn detect_backend() -> NetworkBackend {
    let nm = is_nm_running();
    let nd = is_networkd_running();
    match (nm, nd) {
        (true, true) => NetworkBackend::Both,
        (true, false) => NetworkBackend::NetworkManager,
        (false, true) => NetworkBackend::SystemdNetworkd,
        (false, false) => NetworkBackend::Neither,
    }
}

pub fn is_nm_running() -> bool {
    // Prefer nmcli general status
    if let Ok(output) = Command::new("nmcli")
        .args(["-t", "-f", "RUNNING", "general"])
        .output()
    {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            if s.trim().eq_ignore_ascii_case("running") {
                return true;
            }
        }
    }
    // Fallback: systemd unit
    systemctl_is_active("NetworkManager")
}

pub fn is_networkd_running() -> bool {
    systemctl_is_active("systemd-networkd")
}

fn systemctl_is_active(unit: &str) -> bool {
    Command::new("systemctl")
        .args(["is-active", "--quiet", unit])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Connection UUID/name associated with a device, if any.
pub fn nm_connection_for_device(device: &str) -> Option<String> {
    let output = Command::new("nmcli")
        .args(["-t", "-f", "GENERAL.CONNECTION", "device", "show", device])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout);
    for line in s.lines() {
        if let Some(val) = line.strip_prefix("GENERAL.CONNECTION:").or_else(|| {
            // -t mode may just print the value
            if !line.contains(':') {
                Some(line)
            } else {
                line.split_once(':').map(|(_, v)| v)
            }
        }) {
            let v = val.trim();
            if !v.is_empty() && v != "--" {
                return Some(v.to_string());
            }
        }
    }
    // Simpler parse: entire trimmed stdout
    let t = s.trim();
    if !t.is_empty() && t != "--" && !t.contains('\n') {
        return Some(t.to_string());
    }
    None
}

/// Detect if device is wifi or ethernet via nmcli.
pub fn nm_device_type(device: &str) -> Option<&'static str> {
    let output = Command::new("nmcli")
        .args(["-t", "-f", "GENERAL.TYPE", "device", "show", device])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout).to_lowercase();
    if s.contains("wifi") || s.contains("802-11-wireless") {
        Some("wifi")
    } else if s.contains("ethernet") || s.contains("802-3-ethernet") {
        Some("ethernet")
    } else {
        None
    }
}

/// Set cloned-mac-address on the NM connection for persistent randomization.
pub fn nm_set_cloned_mac(device: &str, mac: &MacAddress, dry_run: bool) -> Result<bool> {
    if !is_nm_running() {
        debug!("NetworkManager not running; skip persistent clone");
        return Ok(false);
    }
    let conn = match nm_connection_for_device(device) {
        Some(c) => c,
        None => {
            warn!("no NM connection for device {device}; runtime-only change");
            return Ok(false);
        }
    };
    let dtype = nm_device_type(device).unwrap_or("ethernet");
    let key = if dtype == "wifi" {
        "wifi.cloned-mac-address"
    } else {
        "ethernet.cloned-mac-address"
    };
    let mac_s = mac.to_string_canonical();
    let args = ["connection", "modify", &conn, key, &mac_s];
    let cmdline = format!("nmcli {}", args.join(" "));
    if dry_run {
        info!("[dry-run] would run: {cmdline}");
        return Ok(true);
    }
    debug!("running: {cmdline}");
    let output =
        Command::new("nmcli")
            .args(args)
            .output()
            .map_err(|e| MacrandomError::CommandFailed {
                cmd: cmdline.clone(),
                source: e,
            })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        warn!("nmcli modify failed: {stderr}");
        return Ok(false);
    }
    info!("set NM {key}={mac_s} on connection '{conn}'");
    Ok(true)
}

/// Reset cloned MAC to permanent / default on NM connection.
pub fn nm_clear_cloned_mac(device: &str, dry_run: bool) -> Result<bool> {
    if !is_nm_running() {
        return Ok(false);
    }
    let conn = match nm_connection_for_device(device) {
        Some(c) => c,
        None => return Ok(false),
    };
    let dtype = nm_device_type(device).unwrap_or("ethernet");
    let key = if dtype == "wifi" {
        "wifi.cloned-mac-address"
    } else {
        "ethernet.cloned-mac-address"
    };
    // Empty / "permanent" restores hardware MAC in NM.
    let args = ["connection", "modify", &conn, key, "permanent"];
    let cmdline = format!("nmcli {}", args.join(" "));
    if dry_run {
        info!("[dry-run] would run: {cmdline}");
        return Ok(true);
    }
    let output =
        Command::new("nmcli")
            .args(args)
            .output()
            .map_err(|e| MacrandomError::CommandFailed {
                cmd: cmdline.clone(),
                source: e,
            })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        warn!("nmcli clear cloned-mac failed: {stderr}");
        return Ok(false);
    }
    info!("cleared NM cloned MAC on connection '{conn}'");
    Ok(true)
}

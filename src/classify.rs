//! Interface type classification.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;

/// High-level network interface kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceKind {
    Loopback,
    Ethernet,
    Wifi,
    Bridge,
    Bond,
    Vlan,
    Tunnel,
    WireGuard,
    Vpn,
    Container,
    Virtual,
    Unknown,
}

impl fmt::Display for InterfaceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Loopback => write!(f, "loopback"),
            Self::Ethernet => write!(f, "ethernet"),
            Self::Wifi => write!(f, "wifi"),
            Self::Bridge => write!(f, "bridge"),
            Self::Bond => write!(f, "bond"),
            Self::Vlan => write!(f, "vlan"),
            Self::Tunnel => write!(f, "tunnel"),
            Self::WireGuard => write!(f, "wireguard"),
            Self::Vpn => write!(f, "vpn"),
            Self::Container => write!(f, "container"),
            Self::Virtual => write!(f, "virtual"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Classify an interface by name and sysfs clues.
pub fn classify(name: &str, sys_path: Option<&Path>) -> InterfaceKind {
    let lower = name.to_lowercase();

    // Name-pattern first (most reliable for virtual/VPN/container).
    if lower == "lo" || lower.starts_with("lo:") {
        return InterfaceKind::Loopback;
    }
    if lower.starts_with("wg") {
        return InterfaceKind::WireGuard;
    }
    if lower.starts_with("mullvad")
        || lower.starts_with("nordlynx")
        || lower.starts_with("lokinet")
        || lower.starts_with("tun")
        || lower.starts_with("tap")
        || lower.starts_with("ppp")
        || lower.starts_with("zt")
        || lower.starts_with("ovpn")
        || lower.starts_with("openvpn")
    {
        return InterfaceKind::Vpn;
    }
    if lower.starts_with("tailscale") {
        return InterfaceKind::Vpn;
    }
    if lower.starts_with("br-")
        || lower.starts_with("virbr")
        || lower == "docker0"
        || lower.starts_with("bridge")
    {
        return InterfaceKind::Bridge;
    }
    if lower.starts_with("docker")
        || lower.starts_with("veth")
        || lower.starts_with("cni")
        || lower.starts_with("flannel")
        || lower.starts_with("podman")
        || lower.starts_with("lxc")
    {
        return InterfaceKind::Container;
    }
    if lower.starts_with("bond") {
        return InterfaceKind::Bond;
    }
    if lower.contains('.') && looks_like_vlan(&lower) {
        return InterfaceKind::Vlan;
    }
    if lower.starts_with("ip6tnl")
        || lower.starts_with("sit")
        || lower.starts_with("gre")
        || lower.starts_with("gretap")
        || lower.starts_with("erspan")
        || lower.starts_with("vti")
    {
        return InterfaceKind::Tunnel;
    }

    // Sysfs type / wireless detection.
    if let Some(base) = sys_path {
        if base.join("wireless").exists() || base.join("phy80211").exists() {
            return InterfaceKind::Wifi;
        }
        if let Ok(ty) = std::fs::read_to_string(base.join("type")) {
            // ARPHRD_ETHER = 1, ARPHRD_LOOPBACK = 772
            let t = ty.trim();
            if t == "772" {
                return InterfaceKind::Loopback;
            }
            if t == "1" {
                // Could still be virt — check device symlink.
                let device = base.join("device");
                if !device.exists() {
                    // No underlying device → likely virtual ethernet (virtio etc. may have device)
                    // Check for bridge/bond dirs.
                    if base.join("bridge").exists() {
                        return InterfaceKind::Bridge;
                    }
                    if base.join("bonding").exists() {
                        return InterfaceKind::Bond;
                    }
                    // virtio / qemu often still have device; without it treat as virtual.
                    return InterfaceKind::Virtual;
                }
                return InterfaceKind::Ethernet;
            }
        }
        if base.join("bridge").exists() {
            return InterfaceKind::Bridge;
        }
        if base.join("bonding").exists() {
            return InterfaceKind::Bond;
        }
    }

    // Heuristic Wi-Fi names.
    if lower.starts_with("wl") || lower.starts_with("wlan") || lower.starts_with("wifi") {
        return InterfaceKind::Wifi;
    }
    if lower.starts_with("en") || lower.starts_with("eth") {
        return InterfaceKind::Ethernet;
    }

    InterfaceKind::Unknown
}

fn looks_like_vlan(name: &str) -> bool {
    // e.g. eth0.100, enp1s0.20
    if let Some((_, right)) = name.rsplit_once('.') {
        return right.chars().all(|c| c.is_ascii_digit());
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_common_names() {
        assert_eq!(classify("lo", None), InterfaceKind::Loopback);
        assert_eq!(classify("eth0", None), InterfaceKind::Ethernet);
        assert_eq!(classify("enp0s3", None), InterfaceKind::Ethernet);
        assert_eq!(classify("wlan0", None), InterfaceKind::Wifi);
        assert_eq!(classify("wlp2s0", None), InterfaceKind::Wifi);
        assert_eq!(classify("wg0", None), InterfaceKind::WireGuard);
        assert_eq!(classify("mullvad-0", None), InterfaceKind::Vpn);
        assert_eq!(classify("nordlynx", None), InterfaceKind::Vpn);
        assert_eq!(classify("tailscale0", None), InterfaceKind::Vpn);
        assert_eq!(classify("docker0", None), InterfaceKind::Bridge);
        assert_eq!(classify("br-abc123", None), InterfaceKind::Bridge);
        assert_eq!(classify("veth1a2b", None), InterfaceKind::Container);
        assert_eq!(classify("virbr0", None), InterfaceKind::Bridge);
        assert_eq!(classify("tun0", None), InterfaceKind::Vpn);
        assert_eq!(classify("tap0", None), InterfaceKind::Vpn);
        assert_eq!(classify("zt0", None), InterfaceKind::Vpn);
        assert_eq!(classify("cni0", None), InterfaceKind::Container);
        assert_eq!(classify("flannel.1", None), InterfaceKind::Container);
        assert_eq!(classify("eth0.100", None), InterfaceKind::Vlan);
    }
}

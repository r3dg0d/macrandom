//! Default skip-list for sensitive / virtual interfaces.

use crate::classify::InterfaceKind;
use once_cell::sync::Lazy;
use regex::Regex;

static SKIP_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    [
        r"^lo$",
        r"^lo:",
        r"^wg\d*$",
        r"^wg-",
        r"^mullvad",
        r"^nordlynx",
        r"^lokinet",
        r"^ovpn",
        r"^openvpn",
        r"^tailscale",
        r"^docker",
        r"^br-",
        r"^veth",
        r"^virbr",
        r"^tun",
        r"^tap",
        r"^zt",
        r"^cni",
        r"^flannel",
        r"^podman",
        r"^lxc",
        r"^ip6tnl",
        r"^sit\d*$",
        r"^gre",
        r"^gretap",
        r"^erspan",
        r"^vti",
        r"^ppp",
    ]
    .iter()
    .map(|p| Regex::new(p).expect("valid skip regex"))
    .collect()
});

/// Kinds that are skipped by default even if name doesn't match.
const SKIP_KINDS: &[InterfaceKind] = &[
    InterfaceKind::Loopback,
    InterfaceKind::WireGuard,
    InterfaceKind::Vpn,
    InterfaceKind::Container,
    InterfaceKind::Bridge,
    InterfaceKind::Tunnel,
];

/// Returns true if this interface should be skipped unless `--force`.
pub fn should_skip(name: &str, kind: InterfaceKind) -> bool {
    if SKIP_KINDS.contains(&kind) {
        return true;
    }
    let lower = name.to_lowercase();
    SKIP_PATTERNS.iter().any(|re| re.is_match(&lower))
}

/// Human-readable reason, if skipped.
pub fn skip_reason(name: &str, kind: InterfaceKind) -> Option<&'static str> {
    if !should_skip(name, kind) {
        return None;
    }
    Some(match kind {
        InterfaceKind::Loopback => "loopback interface",
        InterfaceKind::WireGuard => "WireGuard interface",
        InterfaceKind::Vpn => "VPN / tunnel interface",
        InterfaceKind::Container => "container virtual interface",
        InterfaceKind::Bridge => "bridge interface",
        InterfaceKind::Tunnel => "tunnel interface",
        _ => "matches default skip pattern",
    })
}

/// Like [`should_skip`], also matching configured extra name prefixes/patterns.
pub fn should_skip_with_extra(name: &str, kind: InterfaceKind, extra: &[String]) -> bool {
    if should_skip(name, kind) {
        return true;
    }
    let lower = name.to_lowercase();
    extra.iter().any(|p| {
        let p = p.to_lowercase();
        if p.is_empty() {
            return false;
        }
        lower == p || lower.starts_with(&p)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::classify;

    #[test]
    fn skips_known_dangerous() {
        let cases = [
            "lo",
            "wg0",
            "mullvad0",
            "nordlynx",
            "lokinet0",
            "tailscale0",
            "docker0",
            "br-deadbeef",
            "veth0abc",
            "virbr0",
            "tun0",
            "tap1",
            "zt0",
            "cni0",
            "flannel.1",
        ];
        for name in cases {
            let kind = classify(name, None);
            assert!(should_skip(name, kind), "expected skip for {name} ({kind})");
        }
    }

    #[test]
    fn allows_normal_nics() {
        for name in ["eth0", "enp0s3", "wlan0", "wlp2s0", "eno1"] {
            let kind = classify(name, None);
            assert!(!should_skip(name, kind), "should NOT skip {name} ({kind})");
        }
    }
}

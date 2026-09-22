//! MAC address types and generation.

use crate::error::{MacrandomError, Result};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// A 6-byte IEEE 802 MAC address.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MacAddress([u8; 6]);

impl MacAddress {
    pub fn new(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 6] {
        &self.0
    }

    /// Organizationally Unique Identifier (first 3 octets).
    pub fn oui(&self) -> [u8; 3] {
        [self.0[0], self.0[1], self.0[2]]
    }

    /// True if the locally administered bit (U/L) is set.
    pub fn is_locally_administered(&self) -> bool {
        self.0[0] & 0x02 != 0
    }

    /// True if the multicast/group bit (I/G) is set.
    pub fn is_multicast(&self) -> bool {
        self.0[0] & 0x01 != 0
    }

    /// Fully random unicast MAC (clears I/G, sets U/L).
    pub fn random_local() -> Self {
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 6];
        rng.fill(&mut bytes);
        // Unicast + locally administered
        bytes[0] = (bytes[0] & 0xFC) | 0x02;
        Self(bytes)
    }

    /// Fully random unicast MAC without forcing U/L bit.
    /// Still clears multicast bit. Optionally force locally administered.
    pub fn random(force_local: bool) -> Self {
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 6];
        rng.fill(&mut bytes);
        bytes[0] &= 0xFE; // clear multicast
        if force_local {
            bytes[0] |= 0x02;
        }
        Self(bytes)
    }

    /// Preserve OUI from `original`, randomize the last 3 octets.
    /// Forces unicast; U/L bit follows the original OUI byte.
    pub fn preserve_oui(original: &MacAddress) -> Self {
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 6];
        bytes[0] = original.0[0] & 0xFE; // keep OUI, clear multicast
        bytes[1] = original.0[1];
        bytes[2] = original.0[2];
        bytes[3] = rng.gen();
        bytes[4] = rng.gen();
        bytes[5] = rng.gen();
        Self(bytes)
    }

    /// Canonical lowercase colon-separated form: `aa:bb:cc:dd:ee:ff`.
    pub fn to_string_canonical(&self) -> String {
        format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string_canonical())
    }
}

impl fmt::Debug for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MacAddress({})", self)
    }
}

impl FromStr for MacAddress {
    type Err = MacrandomError;

    fn from_str(s: &str) -> Result<Self> {
        let cleaned: String = s
            .trim()
            .to_lowercase()
            .chars()
            .filter(|c| c.is_ascii_hexdigit())
            .collect();
        if cleaned.len() != 12 {
            return Err(MacrandomError::InvalidMac(s.to_string()));
        }
        let mut bytes = [0u8; 6];
        for i in 0..6 {
            bytes[i] = u8::from_str_radix(&cleaned[i * 2..i * 2 + 2], 16)
                .map_err(|_| MacrandomError::InvalidMac(s.to_string()))?;
        }
        Ok(Self(bytes))
    }
}

/// MAC randomization mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RandomizeMode {
    /// Fully random unicast + locally administered.
    #[default]
    FullyRandom,
    /// Same as fully random but explicitly documents U/L bit set.
    LocallyAdministered,
    /// Keep first 3 octets (OUI), randomize NIC-specific bytes.
    VendorPreserving,
}

impl FromStr for RandomizeMode {
    type Err = MacrandomError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "random" | "fully-random" | "full" => Ok(Self::FullyRandom),
            "local" | "locally-administered" | "laa" => Ok(Self::LocallyAdministered),
            "vendor" | "vendor-preserving" | "oui" => Ok(Self::VendorPreserving),
            other => Err(MacrandomError::Config(format!(
                "unknown randomize mode '{other}' (expected: random, local, vendor)"
            ))),
        }
    }
}

impl fmt::Display for RandomizeMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FullyRandom => write!(f, "fully-random"),
            Self::LocallyAdministered => write!(f, "locally-administered"),
            Self::VendorPreserving => write!(f, "vendor-preserving"),
        }
    }
}

impl RandomizeMode {
    pub fn generate(&self, current: &MacAddress) -> MacAddress {
        match self {
            Self::FullyRandom | Self::LocallyAdministered => MacAddress::random_local(),
            Self::VendorPreserving => MacAddress::preserve_oui(current),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_colon_mac() {
        let m: MacAddress = "aa:bb:cc:dd:ee:ff".parse().unwrap();
        assert_eq!(m.as_bytes(), &[0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
    }

    #[test]
    fn parse_dash_mac() {
        let m: MacAddress = "AA-BB-CC-DD-EE-FF".parse().unwrap();
        assert_eq!(m.to_string_canonical(), "aa:bb:cc:dd:ee:ff");
    }

    #[test]
    fn parse_bare_mac() {
        let m: MacAddress = "aabbccddeeff".parse().unwrap();
        assert_eq!(m.to_string(), "aa:bb:cc:dd:ee:ff");
    }

    #[test]
    fn reject_bad_mac() {
        assert!("gg:hh:ii:jj:kk:ll".parse::<MacAddress>().is_err());
        assert!("aa:bb".parse::<MacAddress>().is_err());
    }

    #[test]
    fn random_local_sets_ul_clears_ig() {
        for _ in 0..50 {
            let m = MacAddress::random_local();
            assert!(m.is_locally_administered());
            assert!(!m.is_multicast());
        }
    }

    #[test]
    fn preserve_oui_keeps_first_three() {
        let orig: MacAddress = "00:1a:2b:33:44:55".parse().unwrap();
        for _ in 0..20 {
            let m = MacAddress::preserve_oui(&orig);
            assert_eq!(m.oui(), [0x00, 0x1a, 0x2b]);
            assert!(!m.is_multicast());
            // NIC bytes should differ almost always
            assert_ne!(m.as_bytes()[3..], orig.as_bytes()[3..]);
        }
    }

    #[test]
    fn mode_generate_vendor() {
        let orig: MacAddress = "52:54:00:12:34:56".parse().unwrap();
        let mode = RandomizeMode::VendorPreserving;
        let n = mode.generate(&orig);
        assert_eq!(n.oui(), orig.oui());
    }

    #[test]
    fn mode_parse() {
        assert_eq!(
            "vendor".parse::<RandomizeMode>().unwrap(),
            RandomizeMode::VendorPreserving
        );
        assert_eq!(
            "local".parse::<RandomizeMode>().unwrap(),
            RandomizeMode::LocallyAdministered
        );
    }
}

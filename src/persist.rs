//! Persist original MACs under XDG data dir.

use crate::error::{MacrandomError, Result};
use crate::mac::MacAddress;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedMac {
    pub interface: String,
    pub original_mac: String,
    pub permanent_mac: Option<String>,
    pub saved_at: DateTime<Utc>,
    pub last_randomized_to: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SavedStore {
    pub interfaces: HashMap<String, SavedMac>,
}

impl SavedStore {
    pub fn path(data_dir: &Path) -> PathBuf {
        data_dir.join("original_macs.json")
    }

    pub fn load(data_dir: &Path) -> Result<Self> {
        let path = Self::path(data_dir);
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(&path).map_err(|e| MacrandomError::Io {
            path: path.clone(),
            source: e,
        })?;
        Ok(serde_json::from_str(&text)?)
    }

    pub fn save(&self, data_dir: &Path) -> Result<()> {
        fs::create_dir_all(data_dir).map_err(|e| MacrandomError::Io {
            path: data_dir.to_path_buf(),
            source: e,
        })?;
        let path = Self::path(data_dir);
        let text = serde_json::to_string_pretty(self)?;
        fs::write(&path, text).map_err(|e| MacrandomError::Io { path, source: e })?;
        Ok(())
    }

    /// Save original if not already recorded (first randomization wins).
    pub fn remember_original(
        &mut self,
        iface: &str,
        current: &MacAddress,
        permanent: Option<&MacAddress>,
        dry_run: bool,
    ) {
        if self.interfaces.contains_key(iface) {
            debug!("original MAC for {iface} already saved");
            return;
        }
        if dry_run {
            tracing::info!("[dry-run] would save original MAC {} for {iface}", current);
            return;
        }
        self.interfaces.insert(
            iface.to_string(),
            SavedMac {
                interface: iface.to_string(),
                original_mac: current.to_string_canonical(),
                permanent_mac: permanent.map(|m| m.to_string_canonical()),
                saved_at: Utc::now(),
                last_randomized_to: None,
            },
        );
    }

    pub fn record_randomized(&mut self, iface: &str, new_mac: &MacAddress) {
        if let Some(entry) = self.interfaces.get_mut(iface) {
            entry.last_randomized_to = Some(new_mac.to_string_canonical());
        }
    }

    pub fn get_original(&self, iface: &str) -> Result<MacAddress> {
        let entry = self
            .interfaces
            .get(iface)
            .ok_or_else(|| MacrandomError::NoSavedOriginal(iface.to_string()))?;
        MacAddress::from_str(&entry.original_mac)
    }
}

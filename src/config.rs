//! Configuration and XDG paths.

use crate::error::{MacrandomError, Result};
use crate::mac::RandomizeMode;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Default randomization mode.
    #[serde(default)]
    pub mode: RandomizeMode,
    /// Also apply NM persistent cloned-mac when available.
    #[serde(default = "default_true")]
    pub persistent_nm: bool,
    /// Extra interface name prefixes to always skip.
    #[serde(default)]
    pub extra_skip: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: RandomizeMode::FullyRandom,
            persistent_nm: true,
            extra_skip: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Paths {
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub data_dir: PathBuf,
    pub config_file: PathBuf,
}

impl Paths {
    pub fn xdg() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| MacrandomError::Config("no XDG config dir".into()))?
            .join("macrandom");
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| MacrandomError::Config("no XDG cache dir".into()))?
            .join("macrandom");
        let data_dir = dirs::data_local_dir()
            .ok_or_else(|| MacrandomError::Config("no XDG data dir".into()))?
            .join("macrandom");
        let config_file = config_dir.join("config.json");
        Ok(Self {
            config_dir,
            cache_dir,
            data_dir,
            config_file,
        })
    }

    pub fn ensure(&self) -> Result<()> {
        for d in [&self.config_dir, &self.cache_dir, &self.data_dir] {
            fs::create_dir_all(d).map_err(|e| MacrandomError::Io {
                path: d.clone(),
                source: e,
            })?;
        }
        Ok(())
    }
}

impl Config {
    pub fn load(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            debug!("no config at {}, using defaults", path.display());
            return Ok(Self::default());
        }
        let text = fs::read_to_string(path).map_err(|e| MacrandomError::Io {
            path: path.clone(),
            source: e,
        })?;
        let cfg: Config = serde_json::from_str(&text)?;
        Ok(cfg)
    }

    pub fn save(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| MacrandomError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
        let text = serde_json::to_string_pretty(self)?;
        fs::write(path, text).map_err(|e| MacrandomError::Io {
            path: path.clone(),
            source: e,
        })?;
        Ok(())
    }
}

//! Error types for macrandom.

use std::path::PathBuf;
use thiserror::Error;

/// Application-level errors with actionable messages.
#[derive(Debug, Error)]
pub enum MacrandomError {
    #[error("interface not found: {0}")]
    InterfaceNotFound(String),

    #[error("interface '{0}' is in the default skip list (use --force to override)")]
    SkippedByDefault(String),

    #[error("permission denied changing MAC on '{0}' — need CAP_NET_ADMIN (try sudo)")]
    PermissionDenied(String),

    #[error("failed to run '{cmd}': {source}")]
    CommandFailed {
        cmd: String,
        #[source]
        source: std::io::Error,
    },

    #[error("command '{cmd}' exited {status}: {stderr}")]
    CommandExit {
        cmd: String,
        status: i32,
        stderr: String,
    },

    #[error("invalid MAC address: {0}")]
    InvalidMac(String),

    #[error("no saved original MAC for interface '{0}'")]
    NoSavedOriginal(String),

    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("config error: {0}")]
    Config(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, MacrandomError>;

impl MacrandomError {
    /// Suggested process exit code.
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::InterfaceNotFound(_) => 2,
            Self::SkippedByDefault(_) => 3,
            Self::PermissionDenied(_) => 4,
            Self::NoSavedOriginal(_) => 5,
            Self::InvalidMac(_) | Self::Config(_) => 6,
            Self::CommandFailed { .. } | Self::CommandExit { .. } | Self::Io { .. } => 7,
            Self::Json(_) | Self::Other(_) => 1,
        }
    }
}

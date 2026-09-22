//! Clap CLI definition.

use crate::mac::RandomizeMode;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "macrandom",
    author = "r3dg0d",
    version,
    about = "Linux MAC-address randomizer with NetworkManager awareness",
    long_about = "Honest OPSEC tooling for randomizing NIC MAC addresses on Linux.\n\
                  Does not claim anonymity — see README for limitations.\n\n\
                  Changing MAC addresses typically requires CAP_NET_ADMIN (e.g. sudo).\n\
                  Read-only commands (list, status, --help) work without elevated privileges."
)]
pub struct Cli {
    /// Emit machine-readable JSON on stdout
    #[arg(long, global = true)]
    pub json: bool,

    /// Increase logging verbosity (-v, -vv)
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress non-error human output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Path to config file (default: ~/.config/macrandom/config.json)
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Print planned actions without applying changes
    #[arg(long, global = true)]
    pub dry_run: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// List network interfaces and their MAC addresses
    List {
        /// Include interfaces that are skipped by default
        #[arg(long)]
        all: bool,
    },

    /// Show backend status, paths, and saved originals
    Status,

    /// Randomize MAC address on one or all eligible interfaces
    Randomize {
        /// Interface name (omit with --all)
        iface: Option<String>,

        /// Randomize all non-skipped interfaces
        #[arg(long)]
        all: bool,

        /// Randomization mode
        #[arg(long, short = 'm', value_enum, default_value_t = ModeArg::Random)]
        mode: ModeArg,

        /// Override default skip list
        #[arg(long)]
        force: bool,

        /// Also set NetworkManager cloned-mac-address for persistence
        #[arg(long)]
        persistent: bool,

        /// Do not touch NetworkManager settings
        #[arg(long)]
        no_persistent: bool,
    },

    /// Restore previously saved original MAC
    Restore {
        /// Interface name
        iface: String,

        /// Override skip list
        #[arg(long)]
        force: bool,

        /// Do not clear NetworkManager cloned-mac-address
        #[arg(long)]
        no_clear_nm: bool,
    },

    /// Randomize while preserving the vendor OUI (first 3 octets)
    Vendor {
        /// Interface name
        iface: String,

        /// Override skip list
        #[arg(long)]
        force: bool,

        /// Also set NM cloned-mac-address
        #[arg(long)]
        persistent: bool,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: CompletionShell,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum ModeArg {
    #[default]
    Random,
    Local,
    Vendor,
}

impl From<ModeArg> for RandomizeMode {
    fn from(m: ModeArg) -> Self {
        match m {
            ModeArg::Random => RandomizeMode::FullyRandom,
            ModeArg::Local => RandomizeMode::LocallyAdministered,
            ModeArg::Vendor => RandomizeMode::VendorPreserving,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CompletionShell {
    Bash,
    Zsh,
    Fish,
}

impl From<CompletionShell> for clap_complete::Shell {
    fn from(s: CompletionShell) -> Self {
        match s {
            CompletionShell::Bash => clap_complete::Shell::Bash,
            CompletionShell::Zsh => clap_complete::Shell::Zsh,
            CompletionShell::Fish => clap_complete::Shell::Fish,
        }
    }
}

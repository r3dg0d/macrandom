//! macrandom — Linux MAC-address randomizer with NetworkManager awareness.
//!
//! Library surface used by the CLI binary and integration tests.

pub mod classify;
pub mod cli;
pub mod commands;
pub mod config;
pub mod error;
pub mod iface;
pub mod mac;
pub mod nm;
pub mod output;
pub mod persist;
pub mod skip;

pub use error::{MacrandomError, Result};
pub use mac::{MacAddress, RandomizeMode};

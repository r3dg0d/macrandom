use crate::commands::randomize::{cmd_randomize, RandomizeArgs};
use crate::config::{Config, Paths};
use crate::error::Result;
use crate::mac::RandomizeMode;
use crate::output::OutputOpts;

pub fn cmd_vendor(
    opts: &OutputOpts,
    paths: &Paths,
    cfg: &Config,
    iface: &str,
    force: bool,
    persistent: bool,
    dry_run: bool,
) -> Result<()> {
    cmd_randomize(
        opts,
        paths,
        cfg,
        RandomizeArgs {
            iface: Some(iface),
            all: false,
            mode: RandomizeMode::VendorPreserving,
            force,
            persistent,
            dry_run,
        },
    )
}

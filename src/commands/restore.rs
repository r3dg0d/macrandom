use crate::config::Paths;
use crate::error::{MacrandomError, Result};
use crate::iface::{change_mac, Interface};
use crate::nm::nm_clear_cloned_mac;
use crate::output::OutputOpts;
use crate::persist::SavedStore;
use serde::Serialize;
use tracing::info;

#[derive(Serialize)]
struct RestoreResult {
    interface: String,
    restored_mac: String,
    nm_cleared: bool,
    dry_run: bool,
}

pub fn cmd_restore(
    opts: &OutputOpts,
    paths: &Paths,
    cfg: &crate::config::Config,
    iface_name: &str,
    force: bool,
    clear_nm: bool,
    dry_run: bool,
) -> Result<()> {
    let iface = Interface::from_sys(iface_name)?.with_extra_skip(&cfg.extra_skip);
    if iface.skipped_by_default && !force {
        return Err(MacrandomError::SkippedByDefault(iface_name.to_string()));
    }

    let store = SavedStore::load(&paths.data_dir)?;
    let original = store.get_original(iface_name)?;

    info!("restoring {} to {}", iface_name, original);
    change_mac(&iface, &original, dry_run)?;

    let mut nm_cleared = false;
    if clear_nm {
        nm_cleared = nm_clear_cloned_mac(iface_name, dry_run)?;
    }

    let result = RestoreResult {
        interface: iface_name.to_string(),
        restored_mac: original.to_string(),
        nm_cleared,
        dry_run,
    };

    let human = format!(
        "{}{}: restored to {}{}",
        if dry_run { "[dry-run] " } else { "" },
        iface_name,
        original,
        if nm_cleared { " [NM cleared]" } else { "" }
    );
    opts.emit(&human, &result)?;
    Ok(())
}

use crate::config::{Config, Paths};
use crate::error::{MacrandomError, Result};
use crate::iface::{change_mac, list_interfaces, Interface};
use crate::mac::{MacAddress, RandomizeMode};
use crate::nm::nm_set_cloned_mac;
use crate::output::OutputOpts;
use crate::persist::SavedStore;
use serde::Serialize;
use tracing::info;

#[derive(Serialize)]
struct RandomizeReport {
    results: Vec<ChangeResult>,
}

#[derive(Serialize)]
struct ChangeResult {
    interface: String,
    old_mac: Option<String>,
    new_mac: String,
    mode: String,
    persistent_nm: bool,
    dry_run: bool,
}

pub struct RandomizeArgs<'a> {
    pub iface: Option<&'a str>,
    pub all: bool,
    pub mode: RandomizeMode,
    pub force: bool,
    pub persistent: bool,
    pub dry_run: bool,
}

pub fn cmd_randomize(
    opts: &OutputOpts,
    paths: &Paths,
    cfg: &Config,
    args: RandomizeArgs<'_>,
) -> Result<()> {
    if args.iface.is_none() && !args.all {
        return Err(MacrandomError::Config(
            "specify an interface or --all".into(),
        ));
    }
    if args.iface.is_some() && args.all {
        return Err(MacrandomError::Config(
            "specify either an interface or --all, not both".into(),
        ));
    }

    paths.ensure()?;
    let mut store = SavedStore::load(&paths.data_dir)?;

    let targets: Vec<Interface> = if args.all {
        list_interfaces()?
            .into_iter()
            .map(|i| i.with_extra_skip(&cfg.extra_skip))
            .filter(|i| args.force || !i.skipped_by_default)
            .collect()
    } else {
        let name = args.iface.unwrap();
        let iface = Interface::from_sys(name)?.with_extra_skip(&cfg.extra_skip);
        if iface.skipped_by_default && !args.force {
            return Err(MacrandomError::SkippedByDefault(name.to_string()));
        }
        vec![iface]
    };

    if targets.is_empty() {
        opts.print_human("No eligible interfaces to randomize.");
        if opts.json {
            opts.print_json(&RandomizeReport { results: vec![] })?;
        }
        return Ok(());
    }

    let mode = args.mode;
    let use_nm = args.persistent && cfg.persistent_nm;
    let mut results = Vec::new();

    for iface in &targets {
        let current = iface.current_mac.unwrap_or_else(MacAddress::random_local);
        let new_mac = mode.generate(&current);

        store.remember_original(
            &iface.name,
            &current,
            iface.permanent_mac.as_ref(),
            args.dry_run,
        );

        info!(
            "randomizing {} : {} -> {} ({mode})",
            iface.name, current, new_mac
        );

        change_mac(iface, &new_mac, args.dry_run)?;

        let mut nm_ok = false;
        if use_nm {
            nm_ok = nm_set_cloned_mac(&iface.name, &new_mac, args.dry_run)?;
        }

        if !args.dry_run {
            store.record_randomized(&iface.name, &new_mac);
        }

        results.push(ChangeResult {
            interface: iface.name.clone(),
            old_mac: Some(current.to_string()),
            new_mac: new_mac.to_string(),
            mode: mode.to_string(),
            persistent_nm: nm_ok,
            dry_run: args.dry_run,
        });
    }

    if !args.dry_run {
        store.save(&paths.data_dir)?;
    }

    let report = RandomizeReport { results };
    if opts.json {
        opts.print_json(&report)?;
    } else if !opts.quiet {
        for r in &report.results {
            let tag = if r.dry_run { "[dry-run] " } else { "" };
            println!(
                "{tag}{}: {} -> {} ({}){}",
                r.interface,
                r.old_mac.as_deref().unwrap_or("?"),
                r.new_mac,
                r.mode,
                if r.persistent_nm {
                    " [NM persistent]"
                } else {
                    ""
                }
            );
        }
    }
    Ok(())
}

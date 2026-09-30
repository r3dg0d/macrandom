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

    let use_nm = args.persistent && cfg.persistent_nm;
    let results = randomize_targets(
        paths,
        &targets,
        &args,
        use_nm,
        change_mac,
        nm_set_cloned_mac,
    )?;

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

fn randomize_targets<C, P>(
    paths: &Paths,
    targets: &[Interface],
    args: &RandomizeArgs<'_>,
    use_nm: bool,
    mut change: C,
    mut persistent: P,
) -> Result<Vec<ChangeResult>>
where
    C: FnMut(&Interface, &MacAddress, bool) -> Result<()>,
    P: FnMut(&str, &MacAddress, bool) -> Result<bool>,
{
    let mode = args.mode;
    let mut store = SavedStore::load(&paths.data_dir)?;
    let mut results = Vec::new();

    for iface in targets {
        let current = iface.current_mac.ok_or_else(|| {
            MacrandomError::Other(format!(
                "cannot read current MAC for {}; refusing to change it",
                iface.name
            ))
        })?;
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

        if !args.dry_run {
            // Recovery data must reach disk before any network mutation.
            store.save(&paths.data_dir)?;
        }
        change(iface, &new_mac, args.dry_run)?;
        if !args.dry_run {
            store.record_randomized(&iface.name, &new_mac);
            store.save(&paths.data_dir)?;
        }

        let mut nm_ok = false;
        if use_nm {
            nm_ok = persistent(&iface.name, &new_mac, args.dry_run)?;
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

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::InterfaceKind;
    use std::path::PathBuf;

    fn fixture() -> (
        tempfile::TempDir,
        Paths,
        Vec<Interface>,
        RandomizeArgs<'static>,
    ) {
        let root = tempfile::tempdir().unwrap();
        let paths = Paths {
            config_dir: root.path().join("config"),
            cache_dir: root.path().join("cache"),
            data_dir: root.path().join("data"),
            config_file: root.path().join("config/config.json"),
        };
        let targets = (1..=2)
            .map(|n| Interface {
                name: format!("fixture{n}"),
                kind: InterfaceKind::Ethernet,
                current_mac: Some(MacAddress::new([2, 0, 0, 0, 0, n])),
                permanent_mac: None,
                operstate: Some("up".into()),
                is_up: true,
                skipped_by_default: false,
                skip_reason: None,
                sys_path: PathBuf::new(),
            })
            .collect();
        let args = RandomizeArgs {
            iface: None,
            all: true,
            mode: RandomizeMode::FullyRandom,
            force: false,
            persistent: true,
            dry_run: false,
        };
        (root, paths, targets, args)
    }

    #[test]
    fn recovery_is_saved_before_change_and_survives_later_failure() {
        let (_root, paths, targets, args) = fixture();
        let result = randomize_targets(
            &paths,
            &targets,
            &args,
            false,
            |iface, mac, _| {
                let saved = SavedStore::load(&paths.data_dir).unwrap();
                assert_eq!(
                    saved.get_original(&iface.name).unwrap().to_string(),
                    iface.current_mac.unwrap().to_string()
                );
                if iface.name == "fixture2" {
                    return Err(MacrandomError::Other(
                        "simulated later change failure".into(),
                    ));
                }
                assert!(mac.is_locally_administered());
                Ok(())
            },
            |_, _, _| unreachable!(),
        );
        assert!(result.is_err());
        let saved = SavedStore::load(&paths.data_dir).unwrap();
        assert!(saved.interfaces["fixture1"].last_randomized_to.is_some());
        assert!(saved.interfaces["fixture2"].last_randomized_to.is_none());
        assert_eq!(saved.interfaces.len(), 2);
    }

    #[test]
    fn persistent_failure_keeps_original_and_applied_mac() {
        let (_root, paths, targets, args) = fixture();
        let mut changed = String::new();
        let result = randomize_targets(
            &paths,
            &targets,
            &args,
            true,
            |_, mac, _| {
                changed = mac.to_string();
                Ok(())
            },
            |_, _, _| Err(MacrandomError::Other("simulated NM failure".into())),
        );
        assert!(result.is_err());
        let saved = SavedStore::load(&paths.data_dir).unwrap();
        assert_eq!(saved.interfaces.len(), 1);
        assert_eq!(
            saved.interfaces["fixture1"].last_randomized_to.as_deref(),
            Some(changed.as_str())
        );
        assert_eq!(
            saved.get_original("fixture1").unwrap().to_string(),
            targets[0].current_mac.unwrap().to_string()
        );
    }

    #[test]
    fn unwritable_recovery_path_prevents_change() {
        let (_root, mut paths, targets, args) = fixture();
        std::fs::write(&paths.data_dir, b"not a directory").unwrap();
        paths.data_dir = paths.data_dir.join("blocked");
        let result = randomize_targets(
            &paths,
            &targets,
            &args,
            true,
            |_, _, _| panic!("must not change without saved recovery data"),
            |_, _, _| panic!("must not persist NM settings"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn missing_current_mac_prevents_change_and_fabricated_original() {
        let (_root, paths, mut targets, args) = fixture();
        targets[0].current_mac = None;
        let result = randomize_targets(
            &paths,
            &targets,
            &args,
            true,
            |_, _, _| panic!("must not change an unknown MAC"),
            |_, _, _| unreachable!(),
        );
        assert!(result
            .err()
            .unwrap()
            .to_string()
            .contains("cannot read current MAC"));
        assert!(!SavedStore::path(&paths.data_dir).exists());
    }

    #[test]
    fn dry_run_does_not_write_recovery_store() {
        let (_root, paths, targets, mut args) = fixture();
        args.dry_run = true;
        let results = randomize_targets(
            &paths,
            &targets,
            &args,
            true,
            |_, _, dry| {
                assert!(dry);
                Ok(())
            },
            |_, _, dry| {
                assert!(dry);
                Ok(true)
            },
        )
        .unwrap();
        assert_eq!(results.len(), 2);
        assert!(!SavedStore::path(&paths.data_dir).exists());
    }

    #[test]
    fn repeated_changes_keep_first_original_and_private_store() {
        use std::os::unix::fs::PermissionsExt;
        let (_root, paths, mut targets, args) = fixture();
        let first = targets[0].current_mac.unwrap().to_string();
        for _ in 0..2 {
            let results = randomize_targets(
                &paths,
                &targets[..1],
                &args,
                false,
                |_, _, _| Ok(()),
                |_, _, _| unreachable!(),
            )
            .unwrap();
            let saved = SavedStore::load(&paths.data_dir).unwrap();
            assert_eq!(saved.get_original("fixture1").unwrap().to_string(), first);
            assert_eq!(
                saved.interfaces["fixture1"].last_randomized_to.as_deref(),
                Some(results[0].new_mac.as_str())
            );
            targets[0].current_mac = Some(MacAddress::new([2, 0, 0, 0, 0, 99]));
        }
        assert_eq!(
            std::fs::metadata(SavedStore::path(&paths.data_dir))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

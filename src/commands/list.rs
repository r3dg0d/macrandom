use crate::error::Result;
use crate::iface::list_interfaces;
use crate::output::OutputOpts;
use serde::Serialize;

#[derive(Serialize)]
struct ListJson {
    interfaces: Vec<IfaceJson>,
}

#[derive(Serialize)]
struct IfaceJson {
    name: String,
    kind: String,
    current_mac: Option<String>,
    permanent_mac: Option<String>,
    operstate: Option<String>,
    is_up: bool,
    skipped_by_default: bool,
    skip_reason: Option<String>,
}

pub fn cmd_list(opts: &OutputOpts, show_all: bool, extra_skip: &[String]) -> Result<()> {
    let ifaces: Vec<_> = list_interfaces()?
        .into_iter()
        .map(|i| i.with_extra_skip(extra_skip))
        .collect();
    let filtered: Vec<_> = ifaces
        .into_iter()
        .filter(|i| show_all || !i.skipped_by_default)
        .collect();

    let json = ListJson {
        interfaces: filtered
            .iter()
            .map(|i| IfaceJson {
                name: i.name.clone(),
                kind: i.kind.to_string(),
                current_mac: i.current_mac.map(|m| m.to_string()),
                permanent_mac: i.permanent_mac.map(|m| m.to_string()),
                operstate: i.operstate.clone(),
                is_up: i.is_up,
                skipped_by_default: i.skipped_by_default,
                skip_reason: i.skip_reason.clone(),
            })
            .collect(),
    };

    if opts.json {
        opts.print_json(&json)?;
        return Ok(());
    }

    if filtered.is_empty() {
        opts.print_human("No interfaces to show (try --all).");
        return Ok(());
    }

    let mut lines = vec![format!(
        "{:<16} {:<12} {:<18} {:<18} {:<8} {}",
        "INTERFACE", "KIND", "CURRENT", "PERMANENT", "STATE", "NOTES"
    )];
    for i in &filtered {
        let cur = i
            .current_mac
            .map(|m| m.to_string())
            .unwrap_or_else(|| "-".into());
        let perm = i
            .permanent_mac
            .map(|m| m.to_string())
            .unwrap_or_else(|| "-".into());
        let state =
            i.operstate.clone().unwrap_or_else(
                || {
                    if i.is_up {
                        "UP".into()
                    } else {
                        "DOWN".into()
                    }
                },
            );
        let notes = if i.skipped_by_default {
            i.skip_reason.clone().unwrap_or_else(|| "skipped".into())
        } else {
            String::new()
        };
        lines.push(format!(
            "{:<16} {:<12} {:<18} {:<18} {:<8} {}",
            i.name, i.kind, cur, perm, state, notes
        ));
    }
    opts.emit_multiline(&lines, &json)?;
    Ok(())
}

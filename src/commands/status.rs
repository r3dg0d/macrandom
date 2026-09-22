use crate::config::Paths;
use crate::error::Result;
use crate::nm::{detect_backend, is_networkd_running, is_nm_running, NetworkBackend};
use crate::output::OutputOpts;
use crate::persist::SavedStore;
use serde::Serialize;

#[derive(Serialize)]
struct StatusJson {
    backend: NetworkBackend,
    network_manager: bool,
    systemd_networkd: bool,
    paths: PathsJson,
    saved_originals: Vec<SavedJson>,
}

#[derive(Serialize)]
struct PathsJson {
    config_dir: String,
    cache_dir: String,
    data_dir: String,
    config_file: String,
}

#[derive(Serialize)]
struct SavedJson {
    interface: String,
    original_mac: String,
    permanent_mac: Option<String>,
    saved_at: String,
    last_randomized_to: Option<String>,
}

pub fn cmd_status(opts: &OutputOpts, paths: &Paths) -> Result<()> {
    let backend = detect_backend();
    let store = SavedStore::load(&paths.data_dir).unwrap_or_default();

    let saved: Vec<SavedJson> = store
        .interfaces
        .values()
        .map(|s| SavedJson {
            interface: s.interface.clone(),
            original_mac: s.original_mac.clone(),
            permanent_mac: s.permanent_mac.clone(),
            saved_at: s.saved_at.to_rfc3339(),
            last_randomized_to: s.last_randomized_to.clone(),
        })
        .collect();

    let json = StatusJson {
        backend,
        network_manager: is_nm_running(),
        systemd_networkd: is_networkd_running(),
        paths: PathsJson {
            config_dir: paths.config_dir.display().to_string(),
            cache_dir: paths.cache_dir.display().to_string(),
            data_dir: paths.data_dir.display().to_string(),
            config_file: paths.config_file.display().to_string(),
        },
        saved_originals: saved,
    };

    if opts.json {
        opts.print_json(&json)?;
        return Ok(());
    }

    let backend_s = match backend {
        NetworkBackend::NetworkManager => "NetworkManager",
        NetworkBackend::SystemdNetworkd => "systemd-networkd",
        NetworkBackend::Both => "NetworkManager + systemd-networkd",
        NetworkBackend::Neither => "neither (manual / other)",
    };

    let mut lines = vec![
        format!("Backend:            {backend_s}"),
        format!(
            "NetworkManager:     {}",
            if json.network_manager { "yes" } else { "no" }
        ),
        format!(
            "systemd-networkd:   {}",
            if json.systemd_networkd { "yes" } else { "no" }
        ),
        String::new(),
        format!("Config dir:         {}", json.paths.config_dir),
        format!("Cache dir:          {}", json.paths.cache_dir),
        format!("Data dir:           {}", json.paths.data_dir),
        format!("Config file:        {}", json.paths.config_file),
        String::new(),
        format!("Saved originals:    {}", json.saved_originals.len()),
    ];
    for s in &json.saved_originals {
        lines.push(format!(
            "  {}  original={}  last={}",
            s.interface,
            s.original_mac,
            s.last_randomized_to.as_deref().unwrap_or("-")
        ));
    }
    opts.emit_multiline(&lines, &json)?;
    Ok(())
}

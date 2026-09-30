# macrandom

**Linux MAC-address randomizer with NetworkManager awareness.**

Honest OPSEC tooling. It changes the link-layer address your NIC presents on the
wire. It does **not** make you anonymous.

[![CI](https://github.com/r3dg0d/macrandom/actions/workflows/ci.yml/badge.svg)](https://github.com/r3dg0d/macrandom/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## What this is

`macrandom` is a small Rust CLI that:

- Lists interfaces and their current / permanent MACs
- Randomizes addresses (fully random LAA, or vendor-preserving OUI)
- Saves originals under XDG data so you can restore
- Optionally sets NetworkManager `wifi.cloned-mac-address` /
  `ethernet.cloned-mac-address` for per-connection persistence
- Detects NetworkManager vs systemd-networkd
- Skips VPN / container / bridge / loopback interfaces by default

## What this is not

Changing a MAC address **does not** hide you from:

| Vector | Why MAC randomization does not help |
| ------ | ----------------------------------- |
| Wi-Fi SSID / BSSID association | Access points and probe history still identify devices |
| DHCP client identifiers / lease history | Often stable across MAC changes unless also randomized |
| TLS / browser fingerprinting | Completely orthogonal to L2 addressing |
| Account logins, cookies, apps | Identity lives above the MAC |
| Traffic correlation / timing | Still the same host and routes |
| ISP / upstream Layer-3 identity | Your IP and account are unchanged |
| Hardware radiometrics | Some research can fingerprint radios regardless of MAC |

If you need stronger privacy, use a well-configured VPN or Tor **and** understand
their threat models. `macrandom` is one small L2 hygiene tool among many.

## Requirements

- Linux (uses `/sys/class/net` and `ip`)
- Optional: `nmcli` (NetworkManager), `ethtool` (permanent MAC discovery)
- **Privilege:** `list`, `status`, `--help` work without root. Changing a MAC
  needs `CAP_NET_ADMIN` (commonly `sudo`).

## Install

### From source

```bash
cargo install --path .
# or
cargo build --release
sudo install -Dm755 target/release/macrandom /usr/local/bin/macrandom
```

### Nix

```bash
nix build
./result/bin/macrandom --help
# or
nix develop   # rustc/cargo toolchain shell
```

## Quick start

```bash
# See interfaces (no root)
macrandom list
macrandom list --all
macrandom --json list --all

# Backend + saved originals
macrandom status

# Dry-run a change
macrandom --dry-run randomize wlan0

# Actually randomize (needs privileges)
sudo macrandom randomize wlan0
sudo macrandom randomize --all
sudo macrandom randomize wlan0 --mode vendor
sudo macrandom vendor wlan0

# Persist via NetworkManager cloned-mac (when NM manages the device)
sudo macrandom randomize wlan0 --persistent

# Restore saved original
sudo macrandom restore wlan0
```

## Commands

| Command | Description |
| ------- | ----------- |
| `list [--all]` | Show interfaces, MACs, classification, skip notes |
| `status` | NM / networkd detection, XDG paths, saved originals |
| `randomize <iface>` | Randomize one interface |
| `randomize --all` | Randomize all non-skipped interfaces |
| `restore <iface>` | Restore previously saved original MAC |
| `vendor <iface>` | Shortcut for vendor-preserving (OUI keep) mode |
| `completions <bash\|zsh\|fish>` | Print shell completions |

### Global flags

Available on every command:

- `--help` / `--version`
- `--json` — machine-readable stdout
- `--verbose` / `-v` (repeatable), `--quiet` / `-q`
- `--config <path>` — override config file
- `--dry-run` — print planned actions only

### Randomization modes (`--mode`)

| Mode | Behavior |
| ---- | -------- |
| `random` (default) | Unicast + locally administered (U/L bit set) |
| `local` | Same as `random` (explicit LAA) |
| `vendor` | Keep OUI (first 3 octets), randomize the rest |

## Default skip list

Unless you pass `--force`, these are skipped (by kind and/or name pattern):

`lo`, `wg*`, `mullvad*`, `nordlynx*`, `tailscale*`, `docker*`, `br-*`,
`veth*`, `virbr*`, `tun*`, `tap*`, `zt*`, `cni*`, `flannel*`, plus related
container / tunnel / bridge kinds.

Randomizing those often breaks VPNs, containers, or the host loopback — don't
`--force` unless you know why.

## Persistence & paths (XDG)

| Path | Purpose |
| ---- | ------- |
| `~/.config/macrandom/config.json` | Optional defaults |
| `~/.cache/macrandom/` | Reserved for cache |
| `~/.local/share/macrandom/original_macs.json` | Saved originals before first change |

NetworkManager persistence uses connection properties:

- `wifi.cloned-mac-address`
- `ethernet.cloned-mac-address`

Runtime changes use `ip link set dev … address …` (with down/up when needed).

Example config:

```json
{
  "mode": "fully_random",
  "persistent_nm": true,
  "extra_skip": ["mycustom0"]
}
```

## Exit codes

| Code | Meaning |
| ---- | ------- |
| 0 | Success |
| 1 | Generic / other error |
| 2 | Interface not found |
| 3 | Skipped by default (use `--force`) |
| 4 | Permission denied (`CAP_NET_ADMIN`) |
| 5 | No saved original to restore |
| 6 | Invalid input / config |
| 7 | External command / I/O failure |
| 130 | Interrupted (Ctrl+C) |

## Completions

```bash
macrandom completions bash | sudo tee /usr/share/bash-completion/completions/macrandom
macrandom completions zsh  > ~/.zsh/completions/_macrandom
macrandom completions fish > ~/.config/fish/completions/macrandom.fish
```

## Library use

```rust
use macrandom::{MacAddress, RandomizeMode};

let orig: MacAddress = "00:1a:2b:33:44:55".parse()?;
let next = RandomizeMode::VendorPreserving.generate(&orig);
```

## Limitations (read this)

1. **Not anonymity.** See the table above.
2. **Drivers differ.** Some NICs / firmware ignore or reset MAC changes;
   permanent address discovery via `ethtool -P` may be unavailable.
3. **NM vs networkd.** Persistent clone settings are implemented for
   NetworkManager. systemd-networkd is detected but not fully configured yet
   (runtime `ip link` still works).
4. **Race with reconnects.** Wi-Fi roam / NM reconnect can re-apply connection
   settings — use `--persistent` when you want NM to remember the clone.
5. **Legal / ToS.** Some networks prohibit MAC changes. You are responsible for
   local policy compliance.

## Development

CLI smoke tests use isolated temporary XDG directories. On Linux hosts they
check interface discovery; in build sandboxes without sysfs they verify explicit
I/O errors instead. Loopback randomization tests use `--dry-run` only.

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
./target/release/macrandom --help
```

## License

MIT © r3dg0d — see [LICENSE](LICENSE).

## See also

- [SECURITY.md](SECURITY.md) — vulnerability reporting
- [CONTRIBUTING.md](CONTRIBUTING.md) — development norms
- [CHANGELOG.md](CHANGELOG.md) — release history

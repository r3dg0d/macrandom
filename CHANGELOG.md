# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-09-21

### Added

- Initial release of `macrandom`
- Commands: `list`, `status`, `randomize`, `restore`, `vendor`, `completions`
- MAC modes: fully random (locally administered), vendor-preserving (OUI keep)
- Interface classification (Wi-Fi, Ethernet, VPN, container, bridge, …)
- Default skip list for loopback / WireGuard / Mullvad / Tailscale / Docker / etc.
- NetworkManager awareness (`nmcli` cloned-mac-address for persistence)
- systemd-networkd detection
- XDG paths for config / cache / data; originals saved under data dir
- Global flags: `--json`, `--verbose`, `--quiet`, `--config`, `--dry-run`
- Bash / Zsh / Fish completions via `clap_complete`
- Nix flake (`package` + `devShell`)
- GitHub Actions CI (fmt, clippy, test, build)

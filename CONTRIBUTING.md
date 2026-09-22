# Contributing to macrandom

Thanks for helping improve honest OPSEC tooling.

## Development setup

```bash
git clone https://github.com/r3dg0d/macrandom
cd macrandom
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
```

With Nix:

```bash
nix develop
cargo test
```

## Guidelines

1. **No false anonymity claims** — documentation must stay honest about DHCP,
   SSID, browser fingerprinting, and timing correlation.
2. **Skip list is sacred** — changing default skip patterns needs a clear
   rationale (breaking users' VPN/container setups is easy).
3. **Prefer sysfs/`ip`** — avoid new runtime dependencies when possible.
4. **Tests** — unit tests for MAC generation, classification, and skip logic;
   CLI smoke tests must pass without root.
5. **Style** — `cargo fmt`; clippy clean on CI.

## Pull requests

- Target `main`
- Include a changelog entry under `## [Unreleased]` when user-visible
- Keep PRs focused

## Code of conduct

Be respectful. Harassment or advocating harm will result in bans.

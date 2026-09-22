# Security Policy

## Supported versions

| Version | Supported |
| ------- | --------- |
| 0.1.x   | Yes       |

## Reporting a vulnerability

Please open a **private** security advisory on GitHub:

https://github.com/r3dg0d/macrandom/security/advisories/new

Or email the maintainer via the address on the GitHub profile **r3dg0d**.

Do **not** file public issues for vulnerabilities that could let an unprivileged
user change another user's interface state or escalate privileges.

## Scope notes

- `macrandom` must be run with `CAP_NET_ADMIN` (typically via `sudo`) to change
  MAC addresses. That is expected Linux capability model, not a bug.
- The tool writes original MACs under `~/.local/share/macrandom/`. Protect that
  directory if MAC history is sensitive in your threat model.
- This project does **not** provide anonymity. See the README limitations.

## Hardening expectations

- No network listeners
- No phone-home
- Prefer `/sys` + `ip` + optional `nmcli`/`ethtool`; no proprietary blobs

# Changelog

All notable changes to AgenticBox are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Audit log crate (`audit-log`)** — append-only, hash-chained log (each entry carries `prev_hash`/`self_hash`); rotation by size/age/file-count; corrupted-log recovery with backup
- **Trust-delta scoring** — `compute_trust_delta()` in the agent loop derives allow/deny trust signals from policy decisions
- **`agenticbox audit` CLI** — view recent entries, filter by agent, chain verification (`--verify`, tamper detection), summary counts, JSON output for SIEM integration, and manual/auto rotation flags

---

## [0.1.0] — 2026-06-20

### Added
- **Container-based sandbox execution** — Docker + Podman auto-detected, full lifecycle management via bollard
- **Scoped permissions** — terminal, filesystem (RO/RW), network (allowlist/localhost/offline), browser (planned)
- **Filesystem governance** — path resolution with escape prevention via symlinks and `../` traversal
- **Network policy enforcement** — domain allowlist, localhost-only, and offline modes
- **Agent packages** — TOML manifests (`agent.toml`) with runtime install via `[image].setup` commands
- **Ad-hoc command execution** — `agenticbox run -- <cmd>` wraps any command in a sandbox
- **Named agent profiles** — `agenticbox run <agent>` resolves manifests and deploys to sandbox
- **TTY support** — interactive agents get a real PTY via crossterm raw mode
- **Built-in permission demo** — `agenticbox run demo` showcases ALLOWED/BLOCKED in real-time
- **Session management** — SQLite-backed session storage with model config and permissions
- **Daemon** — Axum REST API + WebSocket for persistent session management
- **Desktop console** — Tauri v2 + React UI for managing agents and viewing sessions
- **Multi-runtime support** — Docker and Podman both auto-detected at startup
- **Install scripts** — `install.sh` (macOS/Linux) and `install.ps1` (Windows)

### Changed
- N/A — initial release

### Security
- Filesystem guard canonicalizes all paths to prevent sandbox escapes
- Protected paths block access to SSH keys, AWS credentials, and other sensitive files
- Network allowlist enforced at the container level

### Known Limitations
- Daemon does not create containers yet (only `run` is fully functional)
- Desktop console needs integration with the container runtime
- ACP permission interception not yet implemented
- Agent identity (own credentials, accounts) is planned for a future release

---

[0.1.0]: https://github.com/morpheus-sh/agenticbox/releases/tag/v0.1.0

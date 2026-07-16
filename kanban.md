# AgenticBox — Kanban

## Positioning
**Run autonomous agents that can actually touch your systems.**
AgenticBox gives AI agents real power — terminal, filesystem, browser, network — and shows you exactly what they tried to do.

---

## 🔴 NOW

### 1. Merge sprint → main
**What:** Squash merge `strategy/ai-cofounder-sprint` into `main`. Close stale PR #6.

**Done when:**
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy -p daemon -p agenticbox-cli -p audit-log -p fs-guard -p model-router -p network-control -p policy-engine -p sandbox-core -p session-manager -p shared-types -p tool-protocol -p agent-loop -- -D warnings` passes
- [ ] `cargo test --workspace` — all pass, zero failures
- [ ] `uv run ruff check src/ tests/` passes (agent-runtime)
- [ ] `uv run python -m pytest tests/ -v` — all pass
- [ ] No cost governance references remain in code or docs
- [ ] Demo (`agenticbox run demo`) runs clean and shows 3 BLOCKED + ALLOWED decisions
- [ ] No stale demo references (SQL injection) in any public-facing file
- [ ] Squash merged to main, tagged `v0.2.0`
- [ ] PR #6 closed with comment "superseded by sprint merge"

### 2. Record 30-second demo video
**What:** Screen capture of the full enterprise flow: create identity, run agent with identity, watch BLOCKED decisions, verify audit trail.

**Done when:**
- [ ] Recorded at 1080p, 30 seconds or less
- [ ] Shows: `agenticbox identity create`, `agenticbox run demo`, BLOCKED/ALLOWED output, `agenticbox audit --summary`
- [ ] Posted to X/Twitter
- [ ] Embedded in README

### 3. Launch on GitHub + X + HN
**What:** Coordinated release of v0.2.0.

**Done when:**
- [ ] GitHub README has the demo at the top
- [ ] X thread posted with video
- [ ] HN "Show HN" submitted
- [ ] Landing page reflects shipped features only (no cost governance)

---

## 🟡 NEXT — SPACE Feature Parity

### 4. Persistent workspaces
**What:** Agents get durable workspaces tied to their identity. Work survives across sessions. No more temp dirs.

**Done when:**
- [ ] Workspace path derived from identity: `~/.local/share/agenticbox/workspaces/<identity>/`
- [ ] `agenticbox run <agent> --identity <name>` reuses existing workspace if it exists
- [ ] `agenticbox workspace list` shows all workspaces with size + last modified
- [ ] `agenticbox workspace clean <identity>` removes workspace
- [ ] Tests: workspace persists across two consecutive `run` invocations
- [ ] Tests: workspace is scoped to identity (agent A can't see agent B's workspace)

### 5. Deploy mode (long-running sessions)
**What:** `agenticbox deploy <agent> --identity <name>` starts a persistent session via the daemon. Supports pause/resume.

**Done when:**
- [ ] `agenticbox deploy` starts agent in background via daemon
- [ ] `agenticbox sessions list` shows active sessions
- [ ] `agenticbox sessions pause <id>` suspends a session
- [ ] `agenticbox sessions resume <id>` resumes from saved state
- [ ] `agenticbox sessions stop <id>` terminates
- [ ] Session state (workspace + metadata) survives daemon restart
- [ ] Tests: deploy → pause → resume cycle preserves workspace files
- [ ] Tests: session appears in audit log with correct identity attribution

### 6. Workspace snapshots
**What:** Snapshot the full workspace state. Restore on demand. Poor man's rolling snapshots.

**Done when:**
- [ ] `agenticbox snapshot create <identity>` creates timestamped tar of workspace
- [ ] `agenticbox snapshot list <identity>` shows available snapshots
- [ ] `agenticbox snapshot restore <identity> <timestamp>` restores workspace state
- [ ] Snapshots stored in `~/.local/share/agenticbox/snapshots/<identity>/`
- [ ] Tests: create snapshot → modify files → restore → files match snapshot
- [ ] Tests: snapshot captures files + metadata, not running processes

---

## 🔵 LATER — Isolation & Scale

### 7. Runtime trait abstraction
**What:** Abstract `sandbox-core` behind a `Runtime` trait. Docker becomes one implementation.

**Done when:**
- [ ] `Runtime` trait defined: `create`, `start`, `stop`, `exec`, `logs`, `remove`
- [ ] `DockerRuntime` implements the trait (existing code, refactored)
- [ ] `sandbox-core` dispatches to runtime based on config/feature flag
- [ ] All existing tests pass against `DockerRuntime`
- [ ] Trait is documented with clear contract for new backends

### 8. Firecracker microVM backend
**What:** `FirecrackerRuntime` implements the `Runtime` trait. Real VM isolation for untrusted agents.

**What this requires (Linux only):**
- KVM access (`/dev/kvm`)
- Root or kvm group membership
- Firecracker binary installed
- Minimal rootfs (Alpine or custom)

**Done when:**
- [ ] `FirecrackerRuntime` implements `Runtime` trait
- [ ] Boot/config tested on Linux with KVM
- [ ] `agenticbox run <agent> --runtime firecracker` selects the backend
- [ ] CI runs Firecracker tests on Ubuntu runner (if KVM available)
- [ ] Docker remains default on Windows/macOS
- [ ] Credential injection works through Firecracker (env vars passed via Space Daemon equivalent)
- [ ] Network controls enforced at VM level (not just container)
- [ ] Documented: when to use Firecracker vs Docker

### 9. Linux-first deployment
**What:** Production AgenticBox runs on Linux with Firecracker. Windows/macOS stay as dev environments with Docker.

**Done when:**
- [ ] Install script detects Linux → installs Firecracker
- [ ] Install script detects Windows/macOS → uses Docker
- [ ] Docs clearly state: "Linux for production, any OS for development"
- [ ] Docker path fully supported and tested on all platforms

---

## Design Principles (how we avoid sprint-quality issues)

1. **No feature ships without tests** — every "Done when" has explicit test criteria
2. **No design doc describes unbuilt features as if they exist** — if it's a draft, it says "Draft"
3. **No enforcement gap** — if there's a CLI command, it must actually do what it says
4. **One PR per feature** — not 65 commits in one squash. Each feature is reviewable independently.
5. **Verify before claiming done** — run the test, paste the output, then check the box

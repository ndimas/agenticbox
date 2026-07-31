# AgenticBox — Kanban

## Positioning
**Run autonomous agents that can actually touch your systems.**
AgenticBox gives AI agents real power — terminal, filesystem, browser, network — and shows you exactly what they tried to do.

---

## 🔴 NOW — Launch

### 1. Firecracker microVM isolation (top priority)
**What:** Replace/augment the Docker/Podman sandbox backend (`crates/sandbox-core`) with Firecracker microVMs for stronger isolation. Boot a microVM per agent with a rootfs + kernel image, talk over a vsock/REST API socket instead of bollard.

**Done when:**
- [ ] sandbox-core has a Firecracker runtime backend (in addition to or replacing bollard)
- [ ] Agent package format updated (rootfs images, not Docker images) OR an adapter bridges existing `agent.toml` `[image].base` to a rootfs build step
- [ ] `exec_interactive` + PTY support works over the Firecracker API
- [ ] Tests pass on a Linux host with KVM

**Notes / blockers (read before starting):**
- ❌ **macOS unsupported.** Firecracker requires Linux KVM. It does not run on macOS at all — there is no Colima-style workaround. The core feature becomes undevelopable on the current dev machine. Plan for a Linux dev box or a remote KVM host before starting.
- ❌ **Near-total rewrite of `sandbox-core`.** Current implementation is bollard-only (Docker/Podman API). Firecracker has no `docker run`, no image registry, no shared primitives. `exec_interactive`, PTY, the `[image] base = "..."` pattern are all Docker-specific.
- ⚠️ **Strategy tension.** `docs/competitive-analysis.md` and `docs/enterprise-readiness.md` explicitly concluded "don't compete on isolation, compete on governance; Docker is sufficient for the deterministic floor; Firecracker is a later upgrade, not a blocker." Revisit the positioning docs before committing to this as top priority — the moat thesis may need updating.

---

### 2. Record the 30-second demo video
**What:** Screen capture of `agenticbox run demo` catching an agent trying to read SSH keys, exfiltrate data, and write to readonly paths. Every attempt BLOCKED in real-time with clean colored output.

**Done when:**
- [ ] Recorded at 1080p, 30 seconds or less
- [ ] Posted to X/Twitter with caption: "Watch what your AI agent does when you're not looking. Every attempt caught. Open source."
- [ ] Embedded in README and landing page

**Why now:** The demo works, the output is screenshot-worthy, the launch assets are drafted. The video is the missing piece that makes it spread.

---

### 3. Launch on GitHub + X — the "permission log" drop
**What:** Coordinated release. GitHub repo public with the demo in the README. X thread with the video. Hacker News submission. The pitch: "Your agent tried to read SSH keys. AgenticBox caught it."

**Done when:**
- [ ] GitHub README has the permission log demo at the top (before the fold) — ✅ done
- [ ] One-line install works (or honest prerequisites clearly stated) — ✅ done
- [ ] X thread posted: video + 3-4 tweets explaining the permission model
- [ ] Hacker News "Show HN" submitted with title focused on the demo
- [ ] Landing page updated with the video embedded
- [ ] Launch assets updated to reflect current shipped features (audit log, identity, rotation, dry-run, auto-fetch, verticals)

**Why now:** All P0/P1 features are shipped. The demo is real, the output is screenshot-worthy, the launch assets are updated. The only missing piece is the video recording and the actual launch execution.

---

## 🟡 NEXT

- Browser automation (Playwright) — agents that browse with network guardrails
- Persistent sessions — `agenticbox deploy` for long-running agents
- Waitlist → beta onboarding for managed cloud
- Secret governance — inject API keys without exposing them to the agent

## 🔵 LATER

- Policy engine (OPA-style audit logging)
- Multi-agent coordination
- Managed cloud with SSO, RBAC, VPC
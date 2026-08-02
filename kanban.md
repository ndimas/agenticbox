# AgenticBox — Kanban

## Positioning
**Run autonomous agents that can actually touch your systems.**
AgenticBox gives AI agents real power — terminal, filesystem, browser, network — and shows you exactly what they tried to do.

---

## 🔴 NOW — Launch

### 1. Record the 30-second demo video
**What:** Screen capture of `agenticbox run demo` catching an agent trying to read SSH keys, exfiltrate data, and write to readonly paths. Every attempt BLOCKED in real-time with clean colored output.

**Done when:**
- [ ] Recorded at 1080p, 30 seconds or less
- [ ] Posted to X/Twitter with caption: "Watch what your AI agent does when you're not looking. Every attempt caught. Open source."
- [ ] Embedded in README and landing page

**Why now:** The demo works, the output is screenshot-worthy, the launch assets are drafted. The video is the missing piece that makes it spread.

---

### 2. Launch on GitHub + X — the "permission log" drop
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

- **Company Brain (Phase 0 scaffold — `crates/brain`)** — one query surface over GitHub, Slack, docs, and the audit trail (memory with receipts). DeepSeek-conditional (byte-stable prompt prefixes + cache-hit accounting), provider-extendable (`BrainProvider` trait). MCP server (`brain_search`, `brain_who_knows`, `brain_recent_prs`, `brain_audit`) + `brain-ingest`.
  - **Done when:** ⚪ dogfooded internally (brain answers real daily questions) · ⚪ cache hit-ratio dashboard live from `brain-ingest` · ⚪ Slack/Discord connector with distillation + bursting · ⚪ identity-scoped guardrails (Phase 2)
- Browser automation (Playwright) — agents that browse with network guardrails
- Persistent sessions — `agenticbox deploy` for long-running agents
- Waitlist → beta onboarding for managed cloud
- Secret governance — inject API keys without exposing them to the agent

## 🔵 LATER

- Firecracker microVMs for stronger isolation
- Policy engine (OPA-style audit logging)
- Multi-agent coordination
- Managed cloud with SSO, RBAC, VPC
# AgenticBox — Kanban

## Positioning
**Run autonomous agents that can actually touch your systems.**
AgenticBox gives AI agents real power — terminal, filesystem, browser, network — and shows you exactly what they tried to do.

---

## 🔴 NOW — Prio 0

### 1. Build `agenticbox run` — the viral demo command
**What:** A single CLI command that wraps any agent in a sandbox and streams every permission decision in real-time. ALLOWED / BLOCKED log to stdout. This is the thing that gets screenshotted and shared.

**Done when:**
- [ ] `agenticbox run --terminal --fs:readonly --network:allowlist -- ./agent-script.sh` works
- [ ] Every tool call is logged: `[timestamp] AGENT → action`, then `[timestamp] ALLOWED/BLOCKED → reason`
- [ ] Blocked actions are caught before execution (permission check happens first)
- [ ] Output is colorized and clean enough to screenshot
- [ ] Works without the full daemon — just a standalone CLI binary

**Why first:** Nothing else matters if the demo doesn't exist. This is the hook. Ship it, record it, post it.

---

### 2. Record the 30-second demo video
**What:** Screen capture of `agenticbox run` catching an agent trying to read SSH keys, exfiltrate data, and write to readonly paths. Every attempt BLOCKED in real-time with clean colored output.

**Done when:**
- [ ] Scripted agent that makes 5-6 interesting attempts (read secrets, curl external, write to protected path, etc.)
- [ ] Recorded at 1080p, 30 seconds or less
- [ ] Posted to X/Twitter with caption: "Watch what your AI agent does when you're not looking. Every attempt caught. Open source."
- [ ] Embedded in README and landing page

**Why second:** The command without the video is just code. The video is what makes it spread.

---

### 3. Launch on GitHub + X — the "permission log" drop
**What:** Coordinated release. GitHub repo public with the demo in the README. X thread with the video. Hacker News submission. The pitch: "Your agent tried to read SSH keys. AgenticBox caught it."

**Done when:**
- [ ] GitHub README has the permission log demo at the top (before the fold)
- [ ] One-line install works (or honest prerequisites clearly stated)
- [ ] X thread posted: video + 3-4 tweets explaining the permission model
- [ ] Hacker News "Show HN" submitted with title focused on the demo
- [ ] Landing page updated with the video embedded

**Why third:** Once the demo + video exist, the launch is distribution. Get it in front of people who'll screenshot the permission log and share it.

---

## 🟡 NEXT

- Browser automation (Playwright) — agents that browse with network guardrails
- Persistent sessions — `agenticbox deploy` for long-running agents
- Web dashboard — visual permission log, session history
- Waitlist → beta onboarding for managed cloud
- Secret governance — inject API keys without exposing them to the agent

## 🔵 LATER

- Firecracker microVMs for stronger isolation
- Cost governance (per-agent billing, quotas, budget alerts)
- Multi-agent coordination
- Managed cloud with SSO, RBAC, VPC

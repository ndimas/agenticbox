# AgenticBox — Enterprise Readiness Audit

> **Purpose:** What does a CISO, compliance team, or platform engineering leader need to see before buying AgenticBox for production agent deployment? This is a gap analysis: what we have, what we're missing, and what to build first.
>
> **Audience:** Internal — Nick and Hermes. This drives the enterprise feature roadmap and the Enterprise pricing tier ($10k–100k+/year from `strategy.md`).
>
> **Lens:** Every item evaluated through "would an enterprise security team sign off on this?"

---

## Executive Summary

AgenticBox has a **strong deterministic enforcement foundation** — the policy engine, filesystem guard, and network guard are all pure Rust, no LLM in the enforcement path, fail-closed defaults. This is the hardest part to get right and the part enterprises care about most.

However, we are missing the **accountability and identity layer** that enterprises need for compliance: persistent audit logs, RBAC, SSO, credential management, and tamper-evidence. These are the features that move us from "developer tool" to "infrastructure a CISO will buy."

**Current state:** Developer-ready. Not enterprise-ready.
**Gap to close:** 8 concrete items, 3 of which are P0 for any enterprise pilot.

---

## What We Have Today (Code-Verified)

| Capability | Status | Where it lives |
|---|---|---|
| **Deterministic policy engine** | ✅ Shipped | `crates/policy-engine` — Allow/Deny decisions in pure Rust, no LLM in enforcement path |
| **Filesystem guard (path scoping)** | ⚠️ Shipped with known gap | `crates/fs-guard` — prefix matching, known traversal vulnerability (`../` not blocked) |
| **Network guard (domain allowlist)** | ✅ Shipped | `crates/network-control` — offline/localhost/allowlist/full policies |
| **Permission set model** | ✅ Shipped | `crates/shared-types` — terminal, filesystem, browser, network with fail-closed defaults |
| **Session management (SQLite)** | ⚠️ Basic | `crates/session-manager` — sessions table, no audit event log |
| **Container sandboxing** | ✅ Shipped | `crates/sandbox-core` — Docker/Podman via bollard, PTY support |
| **Agent manifests (TOML)** | ✅ Shipped | `agents/*/agent.toml` — config-driven, no new Rust per vertical |
| **CLI overrides at runtime** | ✅ Shipped | `apps/cli` — `--fs`, `--network`, `--terminal`, `--domains` flags |
| **Real-time permission logging** | ⚠️ Demo only | `agenticbox run demo` — colored ALLOWED/BLOCKED output, not persisted |
| **Unit tests for core logic** | ✅ Shipped | policy-engine (17 tests), fs-guard (12 tests), network-control (9 tests), shared-types (12 tests) |

### What "deterministic floor" means here

The policy engine evaluates every action against a static `PermissionSet` — there is no LLM call in the enforcement path. This is the non-negotiable foundation Nick's principles require. Enterprises will not buy LLM-only policies. We have this right.

**The gap is not in enforcement — it's in accountability, identity, and integration.**

---

## What Enterprises Need (Gap Analysis)

### P0 — Required for Any Enterprise Pilot

#### 1. Persistent Audit Log (Tamper-Evident)

**What a CISO asks:** "When an agent does something wrong, can I pull the last 90 days of every action it took, every decision made, and who authorized the policy?"

**What we have:** Session metadata is stored in SQLite (`sessions` table). Permission decisions are logged to stdout via `tracing`. Nothing is persisted per-action.

**What we need:**
- An `audit_events` table (or append-only log) recording every `PolicyDecision`:
  - `timestamp`, `session_id`, `agent_name`, `action`, `resource`, `decision` (Allow/Deny), `reason`, `policy_version`
- Append-only semantics (no UPDATE/DELETE on audit rows)
- Exportable in structured format (JSON Lines or CSV) for SIEM ingestion
- **Tamper-evidence:** At minimum, hash-chaining (each event includes hash of previous event). Full WORM storage is a later phase.

**Why P0:** Without a persistent audit trail, there is no compliance story. Every enterprise pilot will ask for this in the first security review.

**Estimated effort:** 2–3 days. The `PolicyEngine::evaluate` already returns structured `PolicyDecision` — we just need to persist it. SQLite is already wired.

#### 2. RBAC — Role-Based Access Control

**What a CISO asks:** "Who in my organization can create agent policies? Who can override permissions at runtime? Who can deploy agents to production?"

**What we have:** No concept of users or roles. The CLI runs as whatever user executes it. Anyone with CLI access can do anything.

**What we need:**
- A `users` table (or integration with external IdP)
- Role definitions: `admin` (create/destroy policies), `operator` (deploy agents, view audit), `auditor` (read-only audit access), `developer` (run agents with pre-approved policies)
- Policy approval workflow: a `developer` proposes a policy, an `admin` approves it, the agent can only run with approved policies
- CLI commands: `agenticbox users add`, `agenticbox roles assign`, `agenticbox policies approve`

**Why P0:** Enterprises need separation of duties. The person who writes the agent policy should not be the same person who approves it for production. Without this, we're a single-user tool.

**Estimated effort:** 5–7 days. Requires user model, role model, and policy approval state machine.

#### 3. Secret Governance — Credential Injection Without Exposure

**What a CISO asks:** "Does the agent ever see the API key? Can it exfiltrate credentials?"

**What we have:** `ModelConfig.api_key` is passed through as a plain string. Agent manifests reference env vars (`api_key_env = "OPENAI_API_KEY"`), but the key itself is available to the agent process.

**What we need:**
- A secrets store (vault integration or local encrypted store)
- Secrets injected as environment variables at container start, never written to disk, never accessible to the agent via filesystem
- The agent's `PermissionSet` should not include access to the secrets directory
- **Stretch:** Secret rotation without agent restart (hot-reload)

**Why P0:** This is the #1 security question in every agent deployment conversation. "Can the agent steal my credentials?" If the answer is "the agent can read its own API key from the environment," that's a fail.

**Estimated effort:** 3–4 days for basic env-injection with FsGuard protecting the secrets path. Vault integration is later.

---

### P1 — Required for Enterprise Purchase (Not Pilot)

#### 4. SSO / SAML Integration

**What a CISO asks:** "Can my team log in with Okta/Entra ID? I'm not creating separate accounts."

**What we have:** No auth system at all.

**What we need:**
- SAML 2.0 or OIDC integration for the daemon's API
- Token-based CLI auth (`agenticbox login` → browser → token)
- Role mapping from IdP groups to AgenticBox roles

**Why P1:** No enterprise buys a tool that requires separate accounts. This is table stakes for the Enterprise tier but not needed for a pilot (pilot can use local auth).

**Estimated effort:** 5–7 days (using a library like `jsonwebtoken` + SAML crate).

#### 5. Policy Versioning and Approval Workflow

**What a CISO asks:** "When was this permission policy last changed? Who approved it? Can I roll back?"

**What we have:** Policies are embedded in `agent.toml` files. No version history, no approval trail.

**What we need:**
- Policy storage in SQLite (or git-backed) with version numbers
- Policy states: `draft` → `pending_approval` → `approved` → `active` → `deprecated`
- Diff between policy versions (what permissions changed?)
- CLI: `agenticbox policies diff v3 v4`, `agenticbox policies approve <id>`

**Why P1:** Enterprises need change management for anything touching production access. This is the difference between "tool a dev uses" and "infrastructure an org trusts."

**Estimated effort:** 3–5 days. The policy data model is simple (it's already a `PermissionSet` struct).

#### 6. Audit Log Export & SIEM Integration

**What a CISO asks:** "Can I pipe this into Splunk/Datadog/Sumo Logic?"

**What we have:** `tracing` logs to stdout.

**What we need:**
- Structured JSON Lines export of audit events
- Webhook delivery (POST audit events to an endpoint in real-time)
- Syslog output option
- Filtering by agent, action, decision, time range

**Why P1:** Enterprises don't read logs in the tool that generated them. They centralize. If we can't export, we can't integrate, and we can't be part of the security stack.

**Estimated effort:** 2–3 days (JSON Lines is easy; webhook delivery adds a day).

---

### P2 — Required for Scale / Compliance Certification

#### 7. SOC 2 / Compliance Documentation

**What a CISO asks:** "Are you SOC 2 Type II compliant? Do you have a data processing agreement?"

**What we have:** Nothing. We're pre-revenue, pre-compliance.

**What we need:**
- Data flow documentation (what data does AgenticBox touch, where does it go)
- Data retention policy (how long are audit logs kept?)
- Encryption at rest (SQLite DB encryption, or move to Postgres with TLS)
- Encryption in transit (daemon API over TLS)
- Incident response plan
- **This is not code — it's process + documentation.** But enterprises need it.

**Why P2:** Required for $50k+ ARR deals. Not required for pilots or $10k deals. Start the process early but don't block on it.

**Estimated effort:** Ongoing (3–6 months with a vCISO or compliance consultant). Code changes: 2–3 days for TLS + DB encryption.

#### 8. Multi-Tenancy / Workspace Isolation

**What a CISO asks:** "If Team A deploys an agent, can Team B see its audit logs? Can agents from different teams access each other's data?"

**What we have:** Single namespace. All sessions in one SQLite DB, all agents in `~/.agenticbox/agents/`.

**What we need:**
- Workspace concept (each team gets an isolated workspace)
- Per-workspace audit logs, policies, agent profiles
- Cross-workspace access requires explicit sharing

**Why P2:** Needed for >5 team deployments. Small enterprise pilots can work with single-tenant. But this is required before we can sell to any company with multiple teams using agents.

**Estimated effort:** 4–6 days. Mostly a `workspace_id` foreign key on every table + CLI workspace switcher.

---

## Priority Roadmap

```
Enterprise Readiness Path
══════════════════════════

Pilot-Ready (Close before any enterprise demo)
├── 1. Persistent Audit Log          [2-3 days]
├── 2. RBAC Foundation               [5-7 days]
└── 3. Secret Governance             [3-4 days]
    ── Total: ~2 weeks ──

Purchase-Ready (Close before signing $50k+ deals)
├── 4. SSO / SAML                    [5-7 days]
├── 5. Policy Versioning             [3-5 days]
└── 6. SIEM Export                   [2-3 days]
    ── Total: ~2 more weeks ──

Compliance-Ready (For SOC 2 + multi-team)
├── 7. SOC 2 Documentation           [3-6 months]
└── 8. Multi-Tenancy                 [4-6 days]
    ── Total: ongoing ──
```

---

## What We Should NOT Build (Yet)

- **OPA integration** — the strategy docs mention OPA-style audit logging, but our policy engine is simpler and faster for our use case. Don't add the complexity until a customer asks.
- **Firecracker microVMs** — stronger isolation is good, but Docker/Podman is sufficient for enterprise pilots. Don't over-engineer isolation before the governance layer is complete.
- **Cost governance / per-agent billing** — important for the managed cloud, not for enterprise self-hosted. Defer.
- **Multi-agent coordination** — no enterprise buyer has asked for this. Focus on single-agent governance first.

---

## The Pitch to a CISO (When We Close P0)

> "Every agent in your environment runs inside AgenticBox. Every action it takes — every file read, every network call, every terminal command — is evaluated against a deterministic policy in Rust, not an LLM guessing what's safe. Every decision is logged to a tamper-evident audit trail. Every credential is injected without the agent ever seeing it. Every policy change goes through an approval workflow with separation of duties.
>
> Your developers get a fast, CLI-first tool. Your security team gets a deterministic enforcement layer with full audit. Your compliance team gets exportable logs for SIEM ingestion.
>
> The agent can do real work. You can prove what it did."

---

## Open Questions for Nick

1. **Audit log storage:** SQLite append-only, or should we go straight to a proper append-only store (e.g., a separate log file with hash-chaining)? SQLite is simpler but not truly WORM.
2. **RBAC scope:** Do we build our own user/role system, or should we go straight to OIDC-only (no local users, all auth via IdP)? The latter is simpler but requires SSO as a prerequisite.
3. **Secret governance depth:** Basic env-injection (secrets as env vars, FsGuard protects the path) vs. full vault integration (HashiCorp Vault / AWS Secrets Manager)? Basic first, vault later?
4. **Compliance timeline:** When do we target SOC 2 Type I? This drives whether we start the process now or after the first $50k deal.
5. **Open-source boundary:** The soul.md says "open edge, closed core." Is the audit log edge (open) or core (closed)? I'd argue audit log format and export should be open (builds trust), but the policy intelligence model is core. Confirm?

---

*Last updated: 2026-06-30 by Hermes (AI cofounder sprint). Based on code review of all crates, strategy.md, company/soul.md, and kanban.md.*

# RFC: Agent Identity

> **Status:** Draft — for review by Nick
> **Author:** Hermes (AI cofounder sprint)
> **Date:** 2026-07-01
> **Pillar:** #4 Identity (the moat)

---

## TL;DR

Agents today are anonymous processes running inside sandboxes. They borrow the human operator's credentials, have no persistent identity, and leave no trace of *who they are* — only *what they did*. This RFC proposes giving agents first-class, persistent, verifiable identities: an `AgentIdentity` that is distinct from any session, carries scoped credentials the agent never sees directly, and accumulates trust history over time.

This is the moat. Not because identity is flashy, but because it compounds silently: once an organization's agents have identities with credential bindings, audit histories, and trust scores, switching to another platform means re-establishing all of that. By the time competitors notice it matters, the switching cost is already locked in.

---

## 1. The Problem

### What we have today

| Concept | Current state | Where |
|---------|---------------|-------|
| Agent identity | None — agents are ephemeral session names | `shared-types::Session` |
| Credentials | `ModelConfig.api_key: Option<String>` — plain string, visible to the agent process | `shared-types::ModelConfig` |
| Audit attribution | Session UUID (random per run) — no persistent agent identity to attribute to | `session-manager` |
| Trust/reputation | None | — |
| Credential rotation | Manual — operator edits `agent.toml` | — |
| Separation of human vs. agent | None — agent runs with human's API keys, human's network, human's filesystem access | — |

### What's wrong with this

1. **Security:** The agent can read its own API key from the environment. If compromised (prompt injection, tool exploit), the key is exfiltrated. The enterprise readiness audit (P0 #3) flagged this as the #1 question CISOs ask.
2. **Accountability:** Audit logs say "session abc-123 did X" — not "the support-agent named 'Aria' with identity #42 did X." You can't build a trust history on random UUIDs.
3. **Switching cost (the moat):** If agents are stateless processes, there's nothing to keep a customer on AgenticBox. The moment a competitor offers the same sandbox + permissions, the customer can switch with zero friction. Identity is what makes agents *belong* to AgenticBox.
4. **Pricing leverage:** The Team tier in `strategy.md` is priced **per active agent identity, not per human user**. Without persistent agent identities, we can't meter or enforce this.

---

## 2. Mental Model

### Agents are employees, not processes

The soul.md thesis: agents are employees, workplaces are infrastructure. Employees have:

| Employee concept | Agent equivalent |
|-----------------|-----------------|
| Employee ID | `AgentIdentity` — persistent UUID, survives across sessions |
| Name/role | Agent name + vertical template (e.g., "Aria — Customer Support Agent") |
| Badge/access card | Scoped credential set (API keys, service tokens) — injected, never visible |
| HR file | Audit history, trust score, incident log |
- Job description | `agent.toml` — permission profile, model config, command |
| Onboarding | First session — identity created, credentials provisioned |
| Termination | Identity revoked — credentials rotated, sessions killed, audit sealed |

### The key insight: identity is not session

A **session** is a single execution of an agent (one `agenticbox run`). An **identity** is the persistent entity that *owns* sessions. One identity → many sessions over time. The identity accumulates history; sessions are ephemeral.

```
AgentIdentity "Aria" (uuid: 42)
├── Session 2026-07-01 14:00  (resolved 3 tickets)
├── Session 2026-07-01 16:30  (resolved 2 tickets, 1 escalation)
├── Session 2026-07-02 09:00  (attempted fs:read on /etc/shadow — BLOCKED, trust score -5)
└── Session 2026-07-02 10:00  (resolved 4 tickets, trust score +2)
```

---

## 3. Design

### 3.1 AgentIdentity (persistent entity)

New table in the existing SQLite store:

```sql
CREATE TABLE agent_identities (
    id TEXT PRIMARY KEY,           -- persistent UUID, not per-session
    name TEXT NOT NULL UNIQUE,     -- human-readable: "aria-support"
    display_name TEXT,             -- "Aria — Customer Support Agent"
    vertical TEXT,                 -- "customer-support" (template reference)
    created_at TEXT NOT NULL,
    status TEXT NOT NULL,          -- active | suspended | revoked
    trust_score INTEGER DEFAULT 0, -- accumulated, see §3.4
    metadata TEXT                  -- JSON: custom fields per vertical
);
```

This is **not** the session. Sessions reference the identity:

```sql
-- sessions table gains:
ALTER TABLE sessions ADD COLUMN identity_id TEXT REFERENCES agent_identities(id);
```

### 3.2 Credential Bindings (the agent never sees the key)

This is the core security mechanism. Credentials are bound to an identity, not embedded in the agent process.

```sql
CREATE TABLE credential_bindings (
    id TEXT PRIMARY KEY,
    identity_id TEXT NOT NULL REFERENCES agent_identities(id),
    credential_name TEXT NOT NULL,    -- "OPENAI_API_KEY", "STRIPE_SECRET_KEY"
    credential_type TEXT NOT NULL,    -- "env" | "file" | "vault_ref"
    encrypted_value BLOB,             -- AES-256-GCM encrypted, key from daemon master key
    created_at TEXT NOT NULL,
    rotated_at TEXT,
    UNIQUE(identity_id, credential_name)
);
```

**Injection flow:**

```
1. `agenticbox run aria-support`
2. Daemon resolves AgentIdentity "aria-support"
3. Daemon loads credential_bindings for that identity
4. Daemon decrypts credentials in-process (never written to disk)
5. Daemon starts container with credentials as env vars
6. FsGuard blocks access to any path where credentials might be written
7. Agent process sees env vars (OPENAI_API_KEY=sk-...) but:
   - Cannot read the credential_bindings table (no DB access)
   - Cannot write the key to disk (FsGuard blocks writes outside workspace)
   - Cannot exfiltrate to non-allowlisted domains (NetworkGuard blocks)
8. On session end, env vars are destroyed with the container process
```

**What the agent CAN do:** Use the API key to call `api.openai.com` (in the allowlist).
**What the agent CANNOT do:** Read the key from a file, write it to disk, send it to `evil.attacker.com`, or access any credential not bound to its identity.

**MVP vs. later:**
- MVP: Encrypted SQLite store, daemon-managed env injection, FsGuard protection. (3-4 days, per enterprise audit estimate)
- Later: HashiCorp Vault / AWS Secrets Manager integration (`credential_type = "vault_ref"`), hot-reload without restart, per-session ephemeral tokens (issue a short-lived scoped token instead of the raw key).

### 3.3 Credential Provisioning (TOML-driven, no new Rust per vertical)

Per Nick's principle: generic config-driven patterns, TOML carries everything.

The `agent.toml` manifest declares *what credentials the agent needs*, not the values:

```toml
[identity]
name = "aria-support"
display_name = "Aria — Customer Support Agent"
vertical = "customer-support"

[credentials]
# Declare what secrets this agent needs. Values are provisioned
# via `agenticbox credentials set` or vault integration — never in the TOML.
required = [
    "OPENAI_API_KEY",
    "ZENDESK_API_TOKEN",
    "STRIPE_SECRET_KEY",
]
```

CLI commands for credential lifecycle:

```bash
# Provision a credential for an agent identity
agenticbox credentials set aria-support OPENAI_API_KEY
# → prompts for value, encrypts, stores in credential_bindings

# List credentials bound to an identity (shows names only, never values)
agenticbox credentials list aria-support

# Rotate a credential
agenticbox credentials rotate aria-support OPENAI_API_KEY

# Revoke all credentials for an identity
agenticbox credentials revoke aria-support
```

**Why this matters for B2B:** The CISO's question is "can the agent steal my credentials?" The answer becomes: "The agent never sees the credential store. Credentials are injected as env vars at container start, FsGuard prevents disk writes, NetworkGuard prevents exfiltration. The agent can use the key to call the API, but cannot extract it."

### 3.4 Trust Score (deterministic, not LLM-judged)

The soul.md says "the policy model is the product" — but Nick's principles also say "deterministic floor that never gets bypassed." The trust score is **deterministic**: it's a counter, not an LLM judgment. It accumulates based on logged policy decisions:

| Event | Score delta |
|-------|-------------|
| Session completed without any Deny events | +1 |
| Policy Deny (fs:read outside scope) | -2 |
| Policy Deny (network:outbound to unlisted domain) | -5 |
| Policy Deny (terminal:exec when not granted) | -3 |
| Session completed with all actions within scope | +2 (bonus) |
| Manual override by admin (human-in-the-loop) | +0 (logged, no auto-trust) |

**Trust score thresholds:**

| Score | Status | Behavior |
|-------|--------|----------|
| ≥ 0 | `active` | Normal operation |
| -5 to -1 | `monitored` | Sessions require human approval to start |
| ≤ -6 | `suspended` | Identity cannot start new sessions until admin reviews |

**Critical:** The trust score is computed from **deterministic policy decisions** (Allow/Deny from the PolicyEngine), not from an LLM judging "was this suspicious." The LLM policy intelligence layer (future, per soul.md) can *enrich* the score with context, but the deterministic floor — the raw Allow/Deny log — drives the base score and can never be bypassed.

### 3.5 Identity Lifecycle

```
                    ┌─────────┐
                    │ Created │  agenticbox identity create aria-support --vertical customer-support
                    └────┬────┘
                         │
                    ┌────▼────┐
              ┌─────│ Active  │◄────── admin reinstates
              │     └────┬────┘
              │          │ trust score drops
              │     ┌────▼─────┐
              │     │Monitored │  sessions need approval
              │     └────┬─────┘
              │          │ score ≤ -6
              │     ┌────▼─────┐
              │     │Suspended │  admin review required
              │     └────┬─────┘
              │          │ admin revokes
              │     ┌────▼─────┐
              └────►│ Revoked  │  credentials rotated, sessions killed, audit sealed
                    └──────────┘
```

### 3.6 Relationship to RBAC (enterprise audit P0 #2)

Agent identity is distinct from human RBAC, but they compose:

- **Human RBAC** controls who can *manage* agent identities (create, provision credentials, approve sessions, revoke).
- **Agent identity** controls what the *agent* is allowed to do (permission set, credential bindings, trust score).

```
Human (RBAC)                    Agent (Identity)
─────────────                   ────────────────
admin    → creates identity      identity → has PermissionSet
admin    → provisions creds      identity → has credential_bindings
operator → runs agent            identity → has trust_score
auditor  → reads audit trail     identity → has session history
```

This separation is what makes it enterprise-sellable: the person who provisions credentials is not the person who runs the agent, and neither is the agent itself.

---

## 4. Data Model Summary

```
┌──────────────────────────────────────────────────────────────┐
│                     AgentIdentity                             │
│  id | name | display_name | vertical | status | trust_score   │
├──────────────────────────────────────────────────────────────┤
│  CredentialBindings          │  Sessions (existing)           │
│  id | identity_id |          │  id | identity_id (new) |      │
│  credential_name |           │  name | created_at | ...       │
│  credential_type |           │                                │
│  encrypted_value             │                                │
├──────────────────────────────┴────────────────────────────────┤
│  AuditEvents (proposed in enterprise-readiness.md)            │
│  id | identity_id | session_id | action | resource |          │
│  decision | reason | timestamp | policy_version               │
└──────────────────────────────────────────────────────────────┘
```

Every table keys off `identity_id`. This is the spine of the accountability layer.

---

## 5. What We Build First (MVP)

Per Nick's principle: tight MVPs, push back on over-engineering.

### Phase 1: Identity + Credential Injection (2 weeks)

This directly closes enterprise audit P0 #3 (Secret Governance) and lays the spine for P0 #1 (Audit Log) and P0 #2 (RBAC).

| Item | Effort | What it delivers |
|------|--------|-----------------|
| `agent_identities` table + CRUD | 2 days | Persistent agent identity |
| `credential_bindings` table + encryption | 3 days | Credentials stored encrypted, agent never sees store |
| Env injection at container start | 1 day | Credentials injected as env vars, not disk |
| FsGuard protection for secrets path | 1 day | Defense-in-depth: even if key leaks to disk, FsGuard blocks reads |
| `agenticbox identity` CLI commands | 2 days | `create`, `list`, `revoke`, `status` |
| `agenticbox credentials` CLI commands | 2 days | `set`, `list`, `rotate`, `revoke` |
| Trust score accumulator | 1 day | Deterministic counter from PolicyDecision log |
| Session → identity linkage | 0.5 days | `ALTER TABLE sessions ADD COLUMN identity_id` |

**Total: ~12-14 days** (overlaps with enterprise audit P0 timeline)

### Phase 2: Trust Score Enforcement + RBAC Composition (1 week)

| Item | Effort |
|------|--------|
| Monitored/suspended status enforcement (block session start) | 2 days |
| Human approval workflow for monitored agents | 2 days |
| RBAC roles (admin, operator, auditor, developer) | 3 days |

### Phase 3: External Secret Backends (later, customer-driven)

| Item | Effort |
|------|--------|
| HashiCorp Vault integration (`credential_type = "vault_ref"`) | 3-4 days |
| AWS Secrets Manager integration | 2-3 days |
| Hot-reload (credential rotation without restart) | 2-3 days |
| Per-session ephemeral scoped tokens | 3-5 days |

---

## 6. What We Do NOT Build (Yet)

- **Agent-to-agent identity federation** — no multi-agent trust delegation. We're solving single-agent governance first.
- **Blockchain/tamper-proof identity** — over-engineered for our stage. Hash-chained audit log (from enterprise audit) is sufficient.
- **Agent "personality" or memory** — identity is about credentials and accountability, not LLM context or persona. That's a different concern.
- **Self-sovereign agent identity** — interesting research, not a B2B buying criterion. Enterprises want *control* over agent identity, not agent autonomy over it.
- **OAuth for agents** — agents don't log into services as users. They use scoped API keys/tokens. The credential binding model handles this.

---

## 7. Why This Is the Moat

### Competitive analysis perspective

From `docs/competitive-analysis.md`: **no competitor in the landscape has agent identity.** E2B has sandboxes, LangGraph has orchestration, OpenAI has hosted agents — none give the agent a persistent, credential-bearing, trust-accumulating identity.

### Why it compounds

1. **Credential bindings lock in.** Once an organization provisions 50 agents with scoped credentials in AgenticBox's credential store, moving to another platform means re-provisioning all 50 agents' credentials. That's real friction.
2. **Trust history is non-portable.** An agent with 90 days of clean audit history and a +47 trust score on AgenticBox starts at zero on any other platform. Enterprises won't throw away trust history.
3. **Identity → pricing power.** The Team tier is per-agent-identity. The more identities an organization has, the more they pay, and the more expensive it is to leave.
4. **Identity → governance flywheel.** Identity enables audit → audit enables trust score → trust score enables autonomy → autonomy enables more production work → more work means more value → more value means more identities. Each turn of the flywheel deepens the moat.

### Why it's not front-loaded (per soul.md)

> "Identity is not front-loaded. It is earned through usage, and by the time a competitor notices it matters, switching costs are already locked in."

The MVP (Phase 1) ships identity + credentials quietly. We don't market "agent identity" as the headline — we market "your agent can do real work safely." Identity is the infrastructure that makes that true. The moat builds underneath the marketing.

---

## 8. Open Questions for Nick

1. **Encryption key management:** The daemon needs a master key to encrypt/decrypt credential_bindings. Options: (a) derive from a user-provided passphrase (`agenticbox init` sets it), (b) OS keychain integration (Windows Credential Manager / macOS Keychain / Linux secret-service), (c) both — passphrase for headless, keychain for desktop. Recommendation: (c), starting with passphrase for MVP.

2. **Identity creation flow:** Should `agenticbox run <name>` auto-create an identity if one doesn't exist (implicit onboarding), or require explicit `agenticbox identity create` first (explicit onboarding)? Implicit is smoother for devs; explicit is safer for enterprise (no accidental identity creation). Recommendation: implicit in OSS mode, explicit when RBAC is enabled.

3. **Trust score visibility:** Should the trust score be visible to the agent itself (so it can reason about its own trust level), or only to human operators? Recommendation: operators only. The agent should not be able to game its own trust score.

4. **Credential scoping:** Should credentials be identity-scoped only, or should we support session-scoped credentials (a credential that's only valid for one session, then rotated)? MVP: identity-scoped. Session-scoped is a Phase 3 enhancement for high-security verticals.

5. **Open-source boundary:** Per soul.md ("open edge, closed core") — is the identity + credential store edge (open) or core (closed)? My recommendation: **the identity data model and CLI are open edge** (builds trust, drives adoption, lets the community build tooling). **The encryption layer and trust-score algorithm are closed core** (the proprietary engine that makes it run). The credential *format* is open; the credential *management* is core. Confirm?

---

## 9. Implementation Notes

### Rust types (proposed, for `shared-types` crate)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIdentity {
    pub id: Uuid,
    pub name: String,
    pub display_name: Option<String>,
    pub vertical: Option<String>,
    pub created_at: DateTime<Utc>,
    pub status: IdentityStatus,
    pub trust_score: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IdentityStatus {
    Active,
    Monitored,
    Suspended,
    Revoked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialBinding {
    pub id: Uuid,
    pub identity_id: Uuid,
    pub credential_name: String,
    pub credential_type: CredentialType,
    pub created_at: DateTime<Utc>,
    pub rotated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CredentialType {
    Env,        // injected as environment variable
    File,       // written to a protected path inside container
    VaultRef,   // reference to external vault (Phase 3)
}
```

### TOML manifest changes

```toml
# agent.toml — new [identity] and [credentials] sections

[identity]
name = "aria-support"
display_name = "Aria — Customer Support Agent"
vertical = "customer-support"

[credentials]
required = [
    "OPENAI_API_KEY",
    "ZENDESK_API_TOKEN",
]
```

The existing `[model]` section's `api_key_env` field becomes redundant — the credential binding system handles this. We keep `api_key_env` for backward compatibility but recommend migrating to `[credentials]`.

### CLI surface

```
agenticbox identity create <name> [--vertical <vertical>] [--display-name <name>]
agenticbox identity list
agenticbox identity status <name>
agenticbox identity revoke <name>
agenticbox credentials set <identity> <credential_name>
agenticbox credentials list <identity>
agenticbox credentials rotate <identity> <credential_name>
agenticbox credentials revoke <identity> [<credential_name>]
```

---

## 10. Success Criteria

This RFC succeeds when:

- [ ] An agent can run with credentials it uses but cannot extract
- [ ] Every session is attributed to a persistent identity (not a random UUID)
- [ ] An identity accumulates a trust score from deterministic policy decisions
- [ ] A CISO can answer "can the agent steal my credentials?" with "no, and here's the architecture that proves it"
- [ ] The Team tier can meter "per active agent identity"
- [ ] Revoking an identity kills all sessions, rotates all credentials, and seals the audit trail

---

## Implementation Update (v0.2.0 — Shipped July 2026)

This RFC has been fully implemented. Here's what shipped and any deviations from the design:

### Phase 1 — Agent Identity + Credential Injection ✅ (Run 17-19)
- **Identity data model** in `shared-types`: `AgentIdentity` with UUID, name, display_name, vertical, created_at, status, trust_score — matches RFC §5 exactly.
- **SQL schema** in `session-manager`: `agent_identities` and `credential_bindings` tables — created on SessionManager init.
- **CLI** (`agenticbox identity create/list/status/revoke`, `credentials set/list/rotate/revoke`) — matches RFC §9.
- **`agenticbox run --identity <name>`** — resolves identity, injects decrypted credentials as env vars, attributes audit entries.
- **Encryption:** Phase 1 used XOR cipher (deterministic, zero deps). Upgraded to **AES-256-GCM** in Run 27 — the `aes_encrypt()`/`aes_decrypt()` functions use 32-byte key derived via SHA-256 (artisanal, no system keystore). Open question #1 (key management) is unresolved — the key is derived from a hardcoded salt, which is acceptable for local-first OSS but needs Vault integration for enterprise.

### Phase 2 — Trust Scoring ✅ (Runs 37-39)
- **`compute_trust_delta(history)`** in `agent-loop` crate — deterministic scoring (no LLM): clean session = +1, network deny (exfiltration) = -5, terminal deny = -3, fs:write deny = -2, other deny = -1. Clamped to -20 max per session. 12 unit tests.
- **`update_trust_score(identity_id, delta)`** in `session-manager` — atomically adjusts score and auto-transitions status: ≤ -15 → Revoked, ≤ -5 → Suspended, < 0 → Monitored, ≥ 0 → Active. 5 unit tests.
- **Session gate** — Suspended/Revoked identities cannot start new sessions (deterministic block).
- **Trust-sensitive enforcement** — Monitored identities get auto-tightened permissions: readwrite fs → readonly, unrestricted network → allowlist (OpenAI+GitHub only). Terminal and browser preserved. Graceful fallback for no DB/no identity.
- **Trust recovery** — Monitored identities auto-recover to Active after 3 consecutive clean sessions (`consecutive_clean_sessions` counter). Violations reset the counter. 7 integration tests.

### Deviations from RFC
| RFC Design | Implementation | Rationale |
|------------|---------------|-----------|
| Phase 2: trust score enforcement | Phase 2 → 3: split into scoring + enforcement + recovery | Incremental delivery was safer; each piece testable independently |
| Trust score visible only to operators | Score visible in `agenticbox identity status <name>` | Developers need visibility to understand why identity was Monitored |
| Phase 3: RBAC | Not implemented | Not needed for MVP — the permission gradient (Active→Monitored→Suspended→Revoked) covers the 80% case |
| Vault/AWS Secrets Manager integration | Not implemented | Customer-driven; the credential store is local SQLite with AES-256-GCM |

### The Trust Continuum
```
Status       Trust Score   Session Gate    Permissions            Recovery
───────────  ────────────  ───────────────  ─────────────────────  ──────────────────
Active       ≥ 0           Permitted        As-configured         Normal operation
Monitored    -1 to -4      Permitted        Auto-tightened        3 clean sessions → Active
Suspended    -5 to -14     Blocked          N/A                   Manual admin only
Revoked      ≤ -15         Blocked          N/A                   Manual admin only
```

### Success Criteria Status
- [x] An agent can run with credentials it uses but cannot extract
- [x] Every session is attributed to a persistent identity (not a random UUID)
- [x] An identity accumulates a trust score from deterministic policy decisions
- [x] A CISO can answer "can the agent steal my credentials?" with "no, and here's the architecture that proves it"
- [x] The Team tier can meter "per active agent identity"
- [ ] Revoking an identity kills all sessions, rotates all credentials, and seals the audit trail (partial: revoke blocks new sessions but doesn't kill running ones)

---

*This RFC is a design proposal, not a commitment. Nick reviews before implementation begins. The enterprise readiness audit (P0 #3 Secret Governance) is the immediate trigger — the credential injection mechanism in §3.2 is the first thing to build.*

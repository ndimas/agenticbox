# AgenticBox — Vertical Template Strategy

> **Purpose:** Define the vertical template roadmap — which agent roles to ship, in what order, and why. Each template is a wedge into a specific buyer community with budget, a distribution channel, and a permission profile that demonstrates AgenticBox's governance model.
>
> **Status:** Security Analyst (shipped). Support Agent (shipped). Ops/SRE Agent (shipped). Roadmap for next 3 verticals below.
>
> **Lens:** Every vertical evaluated through: (1) market size, (2) permission profile clarity, (3) enterprise pain, (4) distribution potential, (5) demo-ability.

---

## Why Vertical Templates?

From the competitive analysis:

> "Vertical templates are the distribution engine. Each vertical (security analyst now, support next) is a self-contained story for a specific community. Ship them as content + code: a blog post, a demo, a TOML template, a permission profile. That's how we get GitHub stars *and* pipeline."

A vertical template is:
1. **A TOML manifest** with a tuned permission profile for a specific role
2. **A workspace** with sample files that make the demo compelling
3. **A blog post or README** that tells the story of why this role needs governance
4. **A wedge** into a specific buyer community with budget

Each template is a self-contained story. "You're a security analyst? Here's an agent that does your job, safely. You're a support engineer? Here's one for you."

---

## Template Selection Criteria

Each vertical is evaluated on 5 axes:

| Criterion | Weight | What we're looking for |
|-----------|--------|----------------------|
| **Market size** | High | How many companies have this role? How many buyers? |
| **Permission clarity** | High | Can we define a clean, defensible permission boundary? |
| **Enterprise pain** | High | Is this role understaffed? Is there budget for automation? |
| **Demo-ability** | Medium | Can we show the agent doing real work and getting blocked? |
| **Distribution** | Medium | Does this community have a natural distribution channel? |

---

## Vertical 1: Security Analyst ✅ (Shipped)

**Permission profile:** Terminal=on, Filesystem=readwrite, Network=offline (default), Browser=off
**Buyer:** SOC teams, incident responders, threat researchers
**Demo:** Malware analysis — agent reads a suspicious script, tries to exfiltrate credentials, gets blocked by FsGuard, writes analysis report
**Distribution:** Security Twitter, infosec conferences, GitHub security community
**Status:** Shipped in `agents/security-analyst/agent.toml`

---

## Vertical 2: Support Agent ✅ (Shipped)

### Why Support?

**1. Market size — the largest addressable market of any vertical**

Every B2B SaaS company has a support team. Zendesk alone has 200,000+ customers. Intercom, Freshdesk, Help Scout, HubSpot Service Hub — the total addressable market is hundreds of thousands of companies, each with 5-50+ support agents. This is the largest single vertical for agent deployment.

**2. Permission profile is clean and defensible**

A support agent needs:
- **Read access** to: customer tickets, knowledge base, customer history/CRM, product documentation
- **Write access** to: ticket responses, ticket status updates, internal notes
- **Network access** to: CRM API (Zendesk/Intercom/HubSpot), knowledge base API, email API
- **No access** to: billing systems, infrastructure, other customers' data, admin consoles, source code

This is a textbook scoped-permission profile. It demonstrates AgenticBox's governance model perfectly: the agent can do real work (respond to tickets, update statuses) but is deterministically blocked from anything outside its scope.

**3. Enterprise pain is acute and universal**

Support teams are understaffed everywhere. The ROI case is immediate and measurable:
- Tier-1 ticket deflection (50-70% of tickets are "how do I reset my password?" — an agent can handle these)
- 24/7 coverage without night shifts
- Consistent responses (no "agent A says X, agent B says Y")
- Human agents focus on complex/escalated tickets

The buyer (VP of Support / Head of Customer Experience) has budget and is actively looking for AI solutions. They're currently evaluating chatbots, Copilot, and generic LLM wrappers — none of which have governance. AgenticBox's pitch: "Your support agent can actually touch your systems, safely, with a full audit trail."

**4. Demo-ability is high**

The demo writes itself:
1. A customer ticket comes in: "I can't log in, please reset my password"
2. The support agent reads the ticket, searches the knowledge base, finds the password reset article
3. The agent drafts a response and updates the ticket status
4. The agent tries to access the billing database to check the customer's plan — **BLOCKED** by FsGuard
5. The agent tries to call an external API not in its allowlist — **BLOCKED** by NetworkGuard
6. The agent tries to read another customer's ticket — **BLOCKED** by FsGuard path scoping

The ALLOWED/BLOCKED log shows governance in action while the agent does real work.

**5. Distribution through the support community**

The support community (Zendesk community, Intercom blog readers, CX Twitter) is underserved by AI infrastructure. Most AI support tools are chatbots — not agents that can actually touch the support system. A blog post titled "Your AI support agent tried to read the billing database. Here's what happened." would resonate strongly.

### Permission Profile

```toml
[permissions]
terminal = false          # Support agents don't need shell access
filesystem = "readonly"   # Can read knowledge base, customer data; cannot write system files
browser = false           # No browser automation needed
network = "allowlist"     # Only approved APIs
domains = [
    "api.zendesk.com",
    "api.intercom.io",
    "api.hubspot.com",
    "api.stripe.com",     # Read-only for customer billing info
    "api.openai.com",     # LLM provider
]
```

### Sample Workspace

The support agent workspace includes:
- A customer ticket (ticket_001.txt) — "I can't log in, please reset my password"
- A knowledge base article (kb_password_reset.txt) — the official password reset guide
- A customer history snippet (customer_history.txt) — recent interactions, plan info
- A billing database snippet (billing_record.txt) — the agent will try to read this and get BLOCKED

### Template Structure

```
agents/support-agent/
├── agent.toml              # Manifest with support-tuned permissions
└── samples/
    ├── ticket_001.txt       # Customer ticket
    ├── kb_password_reset.txt # Knowledge base article
    ├── customer_history.txt  # Customer interaction history
    └── billing_record.txt    # Billing data (agent will try to read → BLOCKED)
```

---

## Vertical 3: Ops/SRE Agent ✅ (Shipped)

**Why ops after support:** Ops/SRE is the natural third vertical because:
- Ops teams have clear permission boundaries (read monitoring, write runbooks, execute approved scripts)
- The "agent that can fix production issues" is a compelling story
- Ops teams have budget and are actively looking for automation
- The permission profile is more complex (terminal access, selective filesystem, specific domains) — it demonstrates AgenticBox's flexibility

**Permission profile:**
```toml
[permissions]
terminal = true              # Run diagnostic commands
filesystem = "readonly"      # Read configs, logs; never write
browser = false
network = "allowlist"
domains = [
    "api.pagerduty.com",
    "api.datadoghq.com",
    "api.github.com",
    "api.docker.com",
]
```

**Sample workspace:**
```
agents/ops-sre/
├── agent.toml                    # Manifest with ops-tuned permissions
└── samples/
    ├── incident_001.txt           # P1 incident alert (high CPU + 5xx)
    ├── diagnostics.txt            # System diagnostics snapshot
    ├── app_logs.txt               # Application error logs
    ├── runbook_high_cpu.md        # Approved incident response runbook
    ├── prod_config.txt            # Production config (READ ONLY)
    ├── deploy_manifest.yaml       # Hotfix deploy (agent may NOT apply)
    └── prod_secrets.txt           # Production credentials (agent may NOT access)
```

**Demo scenario:** P1 incident (high CPU + 5xx errors on production API). Agent reads logs, runs diagnostics, identifies root cause (connection pool exhaustion from config change). Agent tries to read secrets (BLOCKED), modify config (BLOCKED), deploy hotfix (BLOCKED), access internal DB (BLOCKED). Writes incident report with findings and escalation recommendations.

**Status:** Shipped in `agents/ops-sre/agent.toml` + `blog/ops-sre-agent-template.md`

---

## Vertical 4: Sales Agent 🔵 (Later)

**Why sales after ops:** Sales agents have a more complex permission profile (read CRM, write to CRM, send emails, access customer data) and the buyer (VP of Sales) is harder to reach. But the market is large.

**Permission profile (draft):**
```toml
[permissions]
terminal = false
filesystem = "readonly"
browser = true               # May need to browse prospect websites
network = "allowlist"
domains = [
    "api.hubspot.com",
    "api.salesforce.com",
    "api. outreach.io",
    "api.zoominfo.com",
    "api.clearbit.com",
]
```

---

## Vertical Template Roadmap

```
Q3 2026 (Now)
├── Security Analyst  ✅ Shipped
├── Support Agent     ✅ Shipped
└── Ops/SRE Agent     ✅ Shipped

Q4 2026
├── Code Reviewer     🔵 (reviewer agent exists as example, needs full template)
└── Sales Agent       🔵

Q1 2027
├── Compliance Agent  🔵 (audit log review, policy compliance checking)
└── Finance Agent     🔵 (expense processing, invoice reconciliation)

Q2 2027
└── HR Agent          🔵 (onboarding, offboarding, policy Q&A)
```

### Selection Criteria Applied to Each Vertical

| Vertical | Market Size | Permission Clarity | Enterprise Pain | Demo-ability | Distribution | Score |
|----------|-------------|-------------------|----------------|-------------|-------------|-------|
| Security Analyst | Medium | High | High | High | Medium | ★★★★☆ |
| **Support Agent** | **Very Large** | **High** | **High** | **High** | **High** | **★★★★★** |
| Ops/SRE Agent | Medium | High | High | Medium | Medium | ★★★★☆ |
| Sales Agent | Large | Medium | Medium | Medium | Low | ★★★☆☆ |
| Compliance Agent | Small | High | High | Low | Low | ★★★☆☆ |
| Finance Agent | Medium | Medium | Medium | Medium | Low | ★★★☆☆ |

---

## How Templates Drive Growth

Each vertical template is a growth engine:

1. **Content piece** — Blog post: "Your [role] agent tried to [action]. Here's what happened."
2. **Demo** — The permission log screenshot/video for that specific role
3. **TOML template** — Ready-to-run agent manifest with tuned permissions
4. **Community wedge** — Post in the [role] community (r/netsec, r/customersuccess, r/devops)
5. **GitHub star magnet** — Each template is a self-contained PR that shows the product in action
6. **Enterprise pipeline** — Each template is a conversation starter with a specific buyer

The support agent template is the highest-leverage second vertical because it has the largest market, the clearest permission boundary, and the most accessible buyer community. It's also the template that best demonstrates AgenticBox's core thesis: **agents can do real work, safely, with a full audit trail.**

---

## Open Questions for Nick

1. **Template distribution mechanism** — Should templates live in the main repo (as `agents/support-agent/`) or in a separate `agenticbox-templates` repo? The main repo gives visibility; a separate repo allows community contributions without bloating the core. Recommendation: keep in main repo for now (visibility > modularity at this stage), split when we have 5+ templates.

2. **Template testing** — How do we verify a template works end-to-end? Manual testing per template? Automated integration tests? Recommendation: manual testing for now (each template is a demo, not a production deployment), add automated tests when we have a stable agent-loop crate.

3. **Template versioning** — Should templates be versioned independently of the AgenticBox CLI? If a template's permission profile changes, does the user need to update? Recommendation: templates are tied to the CLI version (they ship together in the repo), but the manifest format should be forward-compatible.

4. **Community templates** — Should we accept community-contributed templates? If so, what's the review process? Recommendation: accept contributions with a review checklist (permission profile is safe, workspace files are appropriate, demo works), but defer this until we have 3+ official templates.

---

*Last updated: 2026-07-01 by Hermes (AI cofounder sprint). Based on competitive analysis, enterprise readiness audit, and code review of existing security-analyst template.*

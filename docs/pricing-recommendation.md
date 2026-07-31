# AgenticBox — Pricing Page Recommendation

> **Purpose:** Should the landing page show pricing? If so, what tiers, what prices, and what messaging? This document reconciles `strategy.md` pricing theory with the current landing page implementation and makes a concrete recommendation.
>
> **Audience:** Nick (decision-maker) and Hermes (execution). This is a strategic recommendation, not a final decision.

---

## Executive Summary

**Recommendation: Hold pricing on the public landing page. Remove the pricing table from `public/start/index.html` and replace it with a "Book a Demo" CTA + a transparent "Coming Soon" note.**

The current pricing table is doing more harm than good. It's inconsistent with the strategy, sells a product that doesn't exist (managed cloud), and signals premature maturity. The right time to show pricing is when the managed cloud beta actually exists — not before.

---

## Current State: Three Problems

### Problem 1: The pricing doesn't match the strategy

| Tier | strategy.md | Landing page (current) | Delta |
|------|-----------|----------------------|-------|
| OSS Core | Free, MIT/Apache-2.0, local runtime, basic permissions | **Not shown** | Missing — the free self-hosted option isn't visible |
| Pro | $20-50/mo — individual builders, desktop app, cloud sync | $199/mo — teams & production, 10 agents | 4-10x higher, different buyer |
| Team | $15-30/agent/month — per active agent identity | **Not shown** | Missing — strategy.md calls this "where things get interesting" |
| Enterprise | $10k-100k+/year | $999/mo (~$12k/year) | 5-10x lower ceiling |
| Starter | Not in strategy.md | $49/mo — managed cloud beta | Doesn't exist in strategy |

The landing page has a completely different pricing model than what `strategy.md` defines. The strategy.md pricing is more nuanced and strategically sound:

- **OSS Core** (free) → developer adoption
- **Pro** ($20-50/mo) → individual builders
- **Team** ($15-30/agent/month) → per-agent identity pricing — "where things get interesting"
- **Enterprise** ($10k-100k+/year) → the real business

The landing page has:
- **Starter** ($49/mo) — managed cloud beta, 1 agent
- **Pro** ($199/mo) — teams & production, 10 agents
- **Enterprise** ($999/mo) — custom SLAs, on-prem

Key misalignments:
1. **No OSS tier shown** — the free self-hosted option is invisible
2. **No Team tier** — per-agent identity pricing (strategy.md calls this "where things get interesting") is absent
3. **Prices don't match** — Pro is 4-10x higher than strategy.md, Enterprise is 5-10x lower
4. **Selling managed cloud that doesn't exist** — the pricing table is for "managed cloud beta" but there's no managed cloud
5. **Wrong pricing model** — flat monthly pricing contradicts the strategy thesis of per-agent-identity pricing

### Problem 2: The pricing table sells a product that doesn't exist

The pricing table is titled "Managed cloud beta" but:
- There is no managed cloud. The product is a local CLI tool.
- There is no cloud sync, no hosted package registry, no desktop app.
- The CTA links go to `mailto:hello@agenticbox.co` — there's no signup flow, no billing, no product to buy.

This creates a credibility problem. A visitor who clicks "Get Started" expecting to sign up for a $49/mo service will find... an email address. That's a trust-destroying experience.

### Problem 3: The pricing model contradicts the strategy thesis

`strategy.md` is explicit:

> *"If you price it like a developer tool, you'll compete with sandboxes and open-source runtimes. If you price it like identity/governance infrastructure, you can become part of the stack companies standardize on."*

The landing page prices it like a SaaS tool ($49/$199/$999 per month, flat). The strategy says the real business is:
- **Per-agent-identity pricing** ($15-30/agent/month for Team)
- **Enterprise deals** ($10k-100k+/year)

The current pricing table communicates "we're a SaaS tool" — which is the wrong category signal. It undermines the "Agent Governance Platform" positioning.

---

## Recommendation: Hold Pricing

### What to do now

1. **Remove the pricing table** from `public/start/index.html`
2. **Replace it with a "Book a Demo" CTA** + a transparent note about managed cloud being in development
3. **Keep the OSS core visible** — the free self-hosted option should be the primary call-to-action for developers

### What the replacement should say

```
<section id="pricing" class="section">
  <div class="container">
    <div class="section-header center">
      <span class="section-tag">Pricing</span>
      <h2 class="section-title">Self-hosted today. Managed cloud <em>coming soon</em>.</h2>
      <p class="section-sub" style="margin-left:auto;margin-right:auto;">
        AgenticBox is free and open source (MIT/Apache-2.0). Run it locally, on your own infrastructure, with no restrictions.
      </p>
    </div>
    <div style="text-align:center;margin-top:40px;">
      <a href="https://github.com/morpheus-sh/agenticbox" class="btn btn-primary" style="display:inline-flex;">Get Started — Free & Open Source</a>
      <p style="margin-top:16px;font-size:0.85rem;color:var(--text-muted);">
        Managed cloud with team governance, audit retention, and SSO is in development.
        <br><a href="mailto:hello@agenticbox.co?subject=Pricing%20question" style="color:var(--olive-dark);font-weight:500;">Contact us</a> for early access.
      </p>
    </div>
```

### Problem 2: The pricing table sells a product that doesn't exist

The pricing table is titled "Managed cloud beta" but:
- There is no managed cloud. The product is a local CLI tool.
- There is no cloud sync, no hosted package registry, no desktop app.
- The CTA links go to `mailto:hello@agenticbox.co` — there's no signup flow, no billing, no product to buy.

A visitor who clicks "Get Started" expecting to sign up for a $49/mo service will find an email address. That's a trust-destroying experience.

### Problem 3: The pricing model contradicts the strategy thesis

`strategy.md` is explicit:

> *"If you price it like a developer tool, you'll compete with sandboxes and open-source runtimes. If you price it like identity/governance infrastructure, you can become part of the stack companies standardize on."*

The landing page prices it like a SaaS tool ($49/$199/$999 per month, flat). The strategy says the real business is:
- **Per-agent-identity pricing** ($15-30/agent/month for Team) — "where things get interesting"
- **Enterprise deals** ($10k-100k+/year) — the real business

The current pricing table communicates "we're a SaaS tool" — which is the wrong category signal. It undermines the "Agent Governance Platform" positioning.

---

## Recommendation: Hold Pricing

### What to do now

1. **Remove the pricing table** from `public/start/index.html`
2. **Replace it with a "Book a Demo" CTA** + a transparent note about managed cloud being in development
3. **Keep the OSS core visible** — the free self-hosted option should be the primary call-to-action for developers

### What the replacement should look like

```html
<section id="pricing" class="section">
  <div class="container">
    <div class="section-header center reveal">
      <span class="section-tag">Pricing</span>
      <h2 class="section-title">Self-hosted today. Managed cloud <em>coming soon</em>.</h2>
      <p class="section-sub" style="margin-left:auto;margin-right:auto;">
        AgenticBox is free and open source (MIT/Apache-2.0). Run it locally, on your own infrastructure, with no restrictions.
      </p>
    </div>
    <div style="text-align:center;margin-top:40px;">
      <a href="https://github.com/morpheus-sh/agenticbox" class="btn btn-primary" style="display:inline-flex;">Get Started — Free &amp; Open Source</a>
      <p style="margin-top:16px;font-size:0.85rem;color:var(--text-muted);max-width:480px;margin-left:auto;margin-right:auto;line-height:1.6;">
        Managed cloud with team governance, audit retention, and SSO is in development.
        <br><a href="mailto:hello@agenticbox.co?subject=Early%20access" style="color:var(--olive-dark);font-weight:500;">Contact us</a> for early access or to book a demo.
      </p>
    </div>
  </div>
</section>
```

### Problem 3: The pricing model contradicts the strategy thesis

`strategy.md` is explicit:

> *"If you price it like a developer tool, you'll compete with sandboxes and open-source runtimes. If you price it like identity/governance infrastructure, you can become part of the stack companies standardize on."*

The landing page prices it like a SaaS tool ($49/$199/$999 per month, flat). The strategy says the real business is:
- **Per-agent-identity pricing** ($15-30/agent/month for Team) — "where things get interesting"
- **Enterprise deals** ($10k-100k+/year) — the real business

The current pricing table communicates "we're a SaaS tool" — which is the wrong category signal. It undermines the "Agent Governance Platform" positioning.

---

## Recommendation: Hold Pricing

### What to do now

1. **Remove the pricing table** from `public/start/index.html`
2. **Replace it with a "Book a Demo" CTA** + a transparent note about managed cloud being in development
3. **Keep the OSS core visible** — the free self-hosted option should be the primary call-to-action for developers

### What the replacement should look like

```html
<section id="pricing" class="section">
  <div class="container">
    <div class="section-header center reveal">
      <span class="section-tag">Pricing</span>
      <h2 class="section-title">Self-hosted today. Managed cloud <em>coming soon</em>.</h2>
      <p class="section-sub" style="margin-left:auto;margin-right:auto;">
        AgenticBox is free and open source (MIT/Apache-2.0). Run it locally, on your own infrastructure, with no restrictions.
      </p>
    </div>
    <div style="text-align:center;margin-top:40px;">
      <a href="https://github.com/morpheus-sh/agenticbox" class="btn btn-primary" style="display:inline-flex;">Get Started — Free &amp; Open Source</a>
      <p style="margin-top:16px;font-size:0.85rem;color:var(--text-muted);max-width:480px;margin-left:auto;margin-right:auto;line-height:1.6;">
        Managed cloud with team governance, audit retention, and SSO is in development.
        <br><a href="mailto:hello@agenticbox.co?subject=Early%20access" style="color:var(--olive-dark);font-weight:500;">Contact us</a> for early access or to book a demo.
      </p>
    </div>
  </div>
</section>
```

### What to do when managed cloud is ready

When the managed cloud beta actually exists (cloud sync, session history, hosted package registry), show pricing that aligns with `strategy.md`:

| Tier | Price | Buyer | Key Features | When to Show |
|------|-------|-------|-------------|--------------|
| **OSS Core** | Free (MIT/Apache-2.0) | Developers, individual builders | Local runtime, Docker sandbox, basic permissions, agent packages | **Now** — always visible |
| **Pro** | $20-50/mo | Individual builders | Desktop app, cloud sync, session history, advanced policies, browser profiles, hosted package registry | When managed cloud beta ships |
| **Team** | $15-30/agent/month | Small teams | Per-agent identity pricing, shared workplaces, team governance, approval workflows, audit retention, role templates | When agent identity ships |
| **Enterprise** | $10k-100k+/year | Security/IT/Compliance | Agent identity management, compliance, audit trails, credential provisioning, policy engines, SSO | When P0 enterprise features ship (audit log, RBAC, secret governance) |

### Why hold, not remove entirely

Holding pricing is not the same as hiding it. The strategy is:

1. **Phase 1 (now):** "Free & Open Source" is the only pricing message. This maximizes developer adoption — the goal of Phase 1 per strategy.md.
2. **Phase 2 (managed cloud beta ships):** Show Pro ($20-50/mo) and Team ($15-30/agent/month) pricing. The managed cloud exists, people can sign up.
3. **Phase 3 (enterprise features ship):** Show Enterprise ($10k-100k+/year) pricing. The P0 features (audit log, RBAC, secret governance) exist.

This phased approach:
- Avoids selling vaporware
- Keeps the OSS core as the primary adoption driver
- Lets pricing evolve as the product matures
- Prevents the wrong pricing model from becoming "the price" in people's minds

---

## What the Right Pricing Looks Like (When Ready)

When the managed cloud beta exists, the pricing should align with `strategy.md`:

| Tier | Price | Buyer | Key Features | When to Show |
|------|-------|-------|-------------|-------------|
| **OSS Core** | Free (MIT/Apache-2.0) | Developers, individual builders | Local runtime, Docker sandbox, basic permissions, agent packages | **Now** — always visible |
| **Pro** | $20-50/mo | Individual builders | Desktop app, cloud sync, session history, advanced policies, browser profiles, hosted package registry | When managed cloud beta ships |
| **Team** | $15-30/agent/month | Small teams | Per-agent identity pricing, shared workplaces, team governance, approval workflows, audit retention, role templates | When agent identity ships |
| **Enterprise** | $10k-100k+/year | Security/IT/Compliance | Agent identity management, compliance, audit trails, credential provisioning, policy engines, SSO | When P0 enterprise features ship |

### Why per-agent pricing matters

The Team tier ($15-30/agent/month) is the strategic differentiator. strategy.md calls it "where things get interesting":

> *"Not per user. Per active agent identity. Example: 10 humans, 50 agents, pay for 50 agent employees."*

This pricing model:
1. **Signals the category** — agents are employees, workplaces are infrastructure
2. **Scales naturally** — as companies deploy more agents, revenue grows without adding headcount
3. **Is defensible** — no competitor prices per-agent-identity because no competitor has agent identity
4. **Creates switching costs** — once agents have identities, credentials, and audit histories in AgenticBox, moving is painful

The flat monthly pricing ($49/$199/$999) communicates "we're a SaaS tool." The per-agent pricing communicates "we're identity/governance infrastructure." This is the difference between competing with sandboxes and becoming part of the stack.

---

## Risk Analysis

### Risk: "No pricing = not a real company"

Some visitors will interpret the absence of pricing as "this isn't a real product." Mitigation:
- The OSS core is real and shippable — lead with it
- The "Book a Demo" CTA signals a real company with real customers
- The "Coming Soon" note for managed cloud is honest and transparent
- The GitHub repo, README, and demo are the real product — pricing is secondary at this stage

### Risk: "I can't evaluate the cost"

For developers: the cost is zero (OSS). For enterprises: they'll ask about pricing in a demo call anyway — a pricing table on a landing page doesn't replace a sales conversation.

### Risk: "You're hiding the ball"

Transparency is better than wrong pricing. The current pricing table is wrong (inconsistent with strategy, selling vaporware). Replacing it with honest messaging ("free today, managed cloud coming") is more transparent than showing incorrect pricing.

---

## What NOT to Do

1. **Don't keep the current pricing table.** It's inconsistent with strategy, sells vaporware, and signals the wrong category.
2. **Don't add a "Free" tier to the existing table.** That would legitimize the wrong pricing model. Start fresh when ready.
3. **Don't show per-agent pricing before agent identity exists.** The Team tier ($15-30/agent/month) requires agent identity to be real. Showing it before it ships is a fake demo.
4. **Don't show Enterprise pricing before P0 enterprise features ship.** $10k-100k+/year without audit log, RBAC, or secret governance is a credibility problem.

---

## Open Questions for Nick

1. **Hold vs. show simplified pricing?** My recommendation is to hold entirely. But an alternative is to show only the OSS tier ("Free & Open Source") with a "Managed Cloud — Coming Soon" badge. This signals "we're a real company" without selling vaporware. Which do you prefer?

2. **Per-agent pricing timing.** The Team tier ($15-30/agent/month) is the strategic differentiator, but it requires agent identity to be real. Do we show it as "Coming Soon" on the pricing page, or wait until it ships?

3. **Enterprise pricing range.** strategy.md says $10k-100k+/year. The current landing page says $999/mo (~$12k/year). The real ceiling is higher. Do we want to signal the range ($10k-100k+/year) or keep it vague ("Contact Sales")?

4. **The OSS core on the business page.** The business landing page (`/start`) currently has no mention of the free self-hosted option. Should it? Or is the business page exclusively for managed cloud prospects?

5. **Pricing page URL.** Should we have a dedicated `/pricing` page, or keep pricing on the business landing page? A dedicated page signals maturity but adds maintenance overhead.

---

## Appendix: What Competitors Charge

| Competitor | Pricing Model | Price Range | Notes |
|-----------|--------------|-------------|-------|
| **E2B** | Free (OSS) + Cloud ($20/mo starter, custom enterprise) | $20-???/mo | OSS sandbox, cloud for managed infra. No per-agent pricing. |
| **Modal** | Pay-per-use (compute) | ~$0.0001/sec GPU | Serverless compute, not agent platform. Hard to compare. |
| **Daytona** | Free (OSS) + Cloud ($0-?) | Free + cloud TBD | Dev environments, not agent governance. |
| **OpenAI Assistants** | Per-token + $0.03/assistant/day | Usage-based | Hosted, no governance. |
| **LangGraph Cloud** | $0-? (free tier + usage) | Usage-based | Framework cloud, not governance. |
| **Browserbase** | $20/mo starter + usage | $20-???/mo | Browser infra, not governance. |

**Key insight:** Nobody in the competitive set has per-agent-identity pricing. E2B is OSS + cloud compute. Modal is pay-per-use compute. Daytona is OSS + cloud dev environments. The per-agent-identity model is unique to AgenticBox — and it's only possible because of the agent identity feature (the moat).

---

## Appendix: What the Pricing Communicates

| Pricing model | What it signals | Category | Risk |
|--------------|----------------|----------|------|
| Flat monthly ($49/$199/$999) | "We're a SaaS tool" | Developer tool / SaaS | Competes with sandboxes, easy to compare on price |
| Per-agent-identity ($15-30/agent/mo) | "Agents are employees" | Identity/governance infrastructure | Harder to explain, requires agent identity feature |
| Usage-based (per-compute) | "We're infrastructure" | Compute layer | Commodity pricing, hard to differentiate |
| OSS + Enterprise | "We're open source infra" | Developer tool → infrastructure | The Vercel/Okta playbook |

The per-agent-identity model is the only one that:
1. Signals the category correctly (agents are employees)
2. Is defensible (requires agent identity feature)
3. Scales with value (more agents = more revenue)
4. Creates switching costs (identities + credentials + audit history)

---

*Last updated: 2026-07-01 by Hermes (AI cofounder sprint). Based on strategy.md, code review of landing pages, and competitive pricing analysis.*

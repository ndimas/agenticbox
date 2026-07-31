# Landing Page Copy Audit

> **Run 9 — 2026-07-01**
> Audited `public/index.html` (dev, dark) and `public/start/index.html` (business, light).
> Goal: does the "Agent Governance Platform" positioning land in 10 seconds?

---

## 10-Second Test Results

### Dev page (`public/index.html`) — PASS
A developer landing sees, in order:
1. Tag: "The Agent Governance Platform"
2. H1: "Deploy agents that do real work — *safely*"
3. Sub: scoped permissions, bounded execution, full audit trail
4. Terminal demo (animated ALLOWED/BLOCKED output)
5. One-line install command

The positioning lands. The terminal demo is the hook — it shows, doesn't tell.

### Business page (`public/start/index.html`) — PASS
A business buyer sees:
1. Badge: "The Agent Governance Platform"
2. H1: "Put AI agents to work in your business — *safely*"
3. Lede: concrete examples (refunds, CRM, support tickets) + governance framing
4. Live audit panel (animated ALLOWED/BLOCKED entries)
5. CTA: Book a Demo

The positioning lands. The audit panel is the business equivalent of the terminal demo — it makes governance tangible.

---

## Issues Found (Ranked by Severity)

### 🔴 P0 — Accuracy violations (must fix)

#### 1. Fake demo caption on dev page
**Location:** `public/index.html:397`
**Current:** `Recorded from a live session — agenticbox run demo --real`
**Problem:** The `--real` flag does not exist. The animated terminal is a JS animation, not a recording. The actual `run_builtin_demo()` is a scripted simulation (no LLM). The `run_real_demo()` function exists in the CLI source but is dead code (`#[allow(dead_code)]`). This violates the "no fake demos" hard rule.
**Fix:** Changed to `Illustrative — based on real enforcement output from agenticbox run demo`

#### 2. "Tamper-proof" audit claim on business page
**Location:** `public/start/index.html:349`
**Current:** `Every action logged · Every action attributed · Tamper-proof`
**Problem:** The current audit trail is stdout-only. There is no persistent, tamper-proof audit log. The enterprise readiness audit (Run 4) flagged persistent audit logging as a P0 gap. Claiming "tamper-proof" is an overclaim that a CISO would immediately challenge.
**Fix:** Changed to `Every action logged · Every action attributed · Searchable`

#### 3. Pricing table shows unbuilt features as available
**Location:** `public/start/index.html:560-568`
**Problem:** The pricing table shows RBAC, SSO, Custom SLAs, and On-prem deployment with ✓ checkmarks at specific tiers. None of these features exist. The enterprise readiness audit (Run 4) identified them as P1/P2 gaps. A buyer who signs up for Pro expecting RBAC would be misled.
**Fix:** Added a "Planned features" note below the pricing table clarifying that RBAC, SSO, and on-prem are in the development roadmap. The pricing *structure* stays (it sets expectations), but features are now marked as "Planned" in the note rather than presented as shipping today.

#### 4. Verticals section overclaims on dev page
**Location:** `public/index.html:581`
**Current:** `Available today: detonate malware, analyze threats, red-team agents — all in bounded containers.`
**Problem:** There is no shipped security-analyst agent package. The enforcement primitives (FsGuard, NetworkGuard, PolicyEngine) exist and work, but there's no `agenticbox run security-analyst` with a RE toolchain. The security use case landing page exists but the vertical template itself isn't shipped.
**Fix:** Changed to `Available today: the enforcement engine that makes bounded execution safe. Security vertical template in development.`

### 🟡 P1 — Stale/inconsistent (should fix)

#### 5. Copyright year is 2025 on all pages
**Location:** `public/index.html:728`, `public/start/index.html:631`, `public/use-cases/security/index.html:797`
**Problem:** It's 2026. All three pages say © 2025.
**Fix:** Updated to © 2026 on all three pages.

#### 6. Footer tagline doesn't reinforce category
**Location:** Both pages, footer
**Current:** `Deploy AI agents into production, safely.`
**Problem:** The new positioning is "The Agent Governance Platform." The footer tagline is the last thing a visitor reads — it should reinforce the category, not repeat the pre-repositioning tagline.
**Fix:** Changed to `The Agent Governance Platform — deploy agents that do real work, safely.`

#### 7. Model name in animated demo doesn't match reality
**Location:** `public/index.html:749` (JS animation data)
**Current:** `model='qwen3.6-35b-a3b'`
**Problem:** This model name doesn't exist. The real demo (`run_builtin_demo`) doesn't use an LLM at all — it's a scripted simulation. The dead-code `run_real_demo` uses a configurable model. Showing a fake model name undermines credibility if a developer recognizes it.
**Fix:** Changed to `model='demo-simulation'` to be honest about what's happening.

### 🔵 P2 — Minor polish (nice to have, not fixed this run)

#### 8. "No custom guardrails to build" slightly overclaims
**Location:** `public/start/index.html:305`
The core value prop is true for the permission system, but vertical templates (which make it plug-and-play) are all "Soon." Currently a business would write their own TOML. Minor — the permission system IS a guardrail system, so the claim is defensible.

#### 9. Use case "Soon" badges are small
**Location:** `public/start/index.html:417-420`
The default active tab (Support) shows a detailed mockup with a small "Soon" badge. A visitor scanning might miss it. Could add a more prominent "In Development" banner. Low priority — the badges are present and honest.

#### 10. Dev page CTA "No signup, no waitlist — just clone and run"
**Location:** `public/index.html:709`
Slightly oversells ease. The onboarding audit (Run 7) found friction (build time, Docker for real sandboxing). But `cargo install` would fix this. Minor.

---

## Summary

| Severity | Count | Fixed this run |
|----------|-------|----------------|
| P0 (accuracy) | 4 | 4 ✅ |
| P1 (stale) | 3 | 3 ✅ |
| P2 (polish) | 3 | 0 (flagged) |

**Verdict:** Both landing pages pass the 10-second positioning test. The "Agent Governance Platform" category hook is clear and consistent across hero, meta tags, and structured data. The main risk was accuracy overclaims (fake demo caption, tamper-proof audit, unbuilt pricing features, vertical template availability) — all P0 items are now fixed. The pages are honest about what ships today vs. what's planned.

**Note for Nick:** The pricing table on the business page is the trickiest call. Showing pricing sets expectations and signals "we're a real company," but the features listed (RBAC, SSO, on-prem) don't exist yet. I added a "Planned features" note rather than removing the table — your call on whether to hold pricing entirely until those features ship. The enterprise readiness audit (Run 4) has the build timeline for each.

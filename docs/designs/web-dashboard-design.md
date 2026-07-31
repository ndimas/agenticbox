# AgenticBox — Web Dashboard Design

> **Phase 1: Static Audit Log Viewer**
> **Status:** ✅ Shipped (Run 21)
> **File:** `public/dashboard/index.html`

---

## Overview

The web dashboard provides a visual interface for the AgenticBox audit log — the permission log and session history that's currently CLI-only. It's a single self-contained HTML file (no build step, no server) that reads the JSONL audit log format and renders it as an interactive dashboard.

## Phase 1: Static Viewer (Shipped)

**Single HTML file** — open in any browser, load a `.jsonl` file, see the data.

### Features
- **Summary stats** — total entries, allowed/blocked counts, unique agents, unique sessions, chain integrity status
- **Filterable table** — filter by agent, decision type (allowed/blocked), action type, and free-text search on resources
- **Chain integrity verification** — checks that each entry's `prev_hash` matches the previous entry's `self_hash`
- **Sample data** — 20 entries across 4 sessions (demo, security-analyst, support-agent, ops-sre) showing real governance patterns
- **File loading** — drag-and-drop or click to load any `.jsonl` audit log file
- **Dark theme** — matches the AgenticBox brand (GitHub-dark inspired)

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  public/dashboard/index.html                                  │
│                                                               │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────────┐  │
│  │ Stats Bar   │  │ Filters      │  │ Entry Table        │  │
│  │ (6 cards)   │  │ (agent,      │  │ (seq, timestamp,   │  │
│  │             │  │  decision,   │  │  agent, identity,  │  │
│  │ total,      │  │  action,     │  │  action, resource, │  │
│  │ allowed,    │  │  search)     │  │  decision, hash)   │  │
│  │ denied,     │  │              │  │                    │  │
│  │ agents,     │  │              │  │                    │  │
│  │ sessions,   │  │              │  │                    │  │
│  │ chain)      │  │              │  │                    │  │
│  └─────────────┘  └──────────────┘  └────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

Now let me also create a design doc for the web dashboard that explains the architecture and future phases.

<｜DSML｜tool_calls>
<｜DSML｜invoke name="write_file">
<｜DSML｜parameter name="content" string="true"># AgenticBox — Web Dashboard Design

> **Phase 1: Static Audit Log Viewer**
> **Status:** ✅ Shipped (Run 21)
> **File:** `public/dashboard/index.html`

---

## Overview

The web dashboard provides a visual interface for the AgenticBox audit log — the permission log and session history that's currently CLI-only. It's a single self-contained HTML file (no build step, no server, no dependencies) that reads the JSONL audit log format and renders it as an interactive dashboard.

## Phase 1: Static Viewer (Shipped)

### Features
- **Summary stats** — total entries, allowed/blocked counts, unique agents, unique sessions, chain integrity status
- **Filterable table** — filter by agent, decision type (allowed/blocked), action type, and free-text search on resources
- **Chain integrity verification** — checks that each entry's `prev_hash` matches the previous entry's `self_hash`
- **File loading** — drag-and-drop or click to load any `.jsonl` audit log file
- **Sample data** — 20 entries across 4 sessions (demo, security-analyst, support-agent, ops-sre) showing real governance patterns
- **Dark theme** — matches the AgenticBox brand (GitHub-dark inspired)

### Architecture

```
public/dashboard/index.html
├── Stats bar (6 cards)
│   ├── Total entries
│   ├── Allowed count
│   ├── Blocked count
│   ├── Unique agents
│   ├── Unique sessions
│   └── Chain integrity status
├── Filter controls
│   ├── Agent dropdown
│   ├── Decision dropdown (All/Allowed/Blocked)
│   ├── Action dropdown
│   ├── Search input (resource text)
│   └── Refresh button
├── Entry table
│   ├── Seq, Timestamp, Agent, Identity, Action, Resource, Decision, Hash
│   └── Color-coded ALLOWED/BLOCKED badges
└── Footer
```

## Phase 2: Server-Backed Dashboard (Planned)

The Phase 1 static viewer is useful for local inspection, but a production dashboard needs:

### Backend API
- **`agenticbox dashboard`** — starts a local HTTP server (e.g., `127.0.0.1:8080`) that serves the dashboard HTML and exposes a REST API
- **`GET /api/entries`** — returns audit entries with pagination, filtering, and search
- **`GET /api/stats`** — returns summary statistics
- **`GET /api/sessions`** — returns session summaries with entry counts
- **`GET /api/verify`** — returns chain integrity status
- **WebSocket** — real-time streaming of new audit entries as they're logged

### API Design

```json
GET /api/entries?agent=demo&decision=deny&limit=50&offset=0
→ {
    "entries": [...],
    "total": 142,
    "offset": 0,
    "limit": 50
  }

GET /api/stats
→ {
    "total": 142,
    "allowed": 98,
    "denied": 44,
    "agents": ["demo", "security-analyst", "support-agent", "ops-sre"],
    "sessions": 12,
    "chain_integrity": true
  }

GET /api/sessions
→ {
    "sessions": [
      {
        "session_id": "a1b2c3d4-...",
        "agent_name": "demo",
        "identity_id": null,
        "entry_count": 8,
        "allowed": 5,
        "denied": 3,
        "start_time": "2026-07-12T10:00:00Z",
        "end_time": "2026-07-12T10:00:05Z"
      }
    ]
  }
```

## Phase 2: Server-Backed Dashboard (Planned)

### Backend
- **`agenticbox dashboard`** — starts a local HTTP server (`127.0.0.1:8080`) that serves the dashboard HTML and exposes a REST API
- **WebSocket** — real-time streaming of new audit entries as they're logged
- **Authentication** — optional basic auth for shared environments

### API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/entries` | GET | Paginated audit entries with filters |
| `/api/stats` | GET | Summary statistics |
| `/api/sessions` | GET | Session summaries with entry counts |
| `/api/verify` | GET | Chain integrity verification |
| `/api/ws` | WS | Real-time entry streaming |

### Phase 3: Production Dashboard (Planned)

- **React/TypeScript** — proper SPA with routing, state management, and real-time updates
- **Authentication** — SSO/OIDC integration for enterprise
- **Charts** — time-series of allowed/blocked decisions, agent activity heatmaps
- **Export** — CSV/PDF export of filtered views
- **Alerts** — configurable alerts for blocked actions (e.g., notify when an agent tries to read secrets)
- **Multi-tenant** — organization-level views with role-based access

## Usage

### Phase 1 (current)
```bash
# Open the dashboard
open public/dashboard/index.html

# Click "Load Audit Log" and select your .jsonl file
# Or click "Load Sample Data" to see a demo
```

### Phase 2 (planned)
```bash
# Start the dashboard server
agenticbox dashboard

# Open in browser
open http://127.0.0.1:8080
```

## File Format

The dashboard reads the same JSONL format that `agenticbox audit --json` outputs:

```jsonl
{"seq":1,"timestamp":"2026-07-12T10:00:00Z","session_id":"...","agent_name":"demo","action":"fs:read","resource":"/workspace/deploy.sh","decision":"Allow","prev_hash":"genesis","self_hash":"..."}
{"seq":2,"timestamp":"2026-07-12T10:00:01Z","session_id":"...","agent_name":"demo","action":"fs:read","resource":"~/.ssh/deploy_key","decision":{"Deny":"filesystem: path outside workspace"},"prev_hash":"...","self_hash":"..."}
```

This is the same format produced by `agenticbox audit --json > audit.jsonl`.

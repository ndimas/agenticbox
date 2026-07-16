//! `agenticbox dashboard` — local HTTP server for the audit log web dashboard.
//!
//! Serves the static dashboard HTML and exposes REST API endpoints for
//! querying the audit log. This is Phase 2 of the web dashboard — a
//! self-contained local server that makes the audit log accessible via
//! browser without manual file loading.
//!
//! ## Usage
//!
//! ```bash
//! agenticbox dashboard
//! # → Dashboard running at http://127.0.0.1:8081
//! #   Open in browser to see the audit log viewer
//! ```

use anyhow::Result;
use audit_log::{AuditEntry, AuditLogger};
use axum::{
    extract::{Query, State},
    response::{Html, Json},
    routing::get,
};
use serde::Deserialize;
use std::sync::Arc;

/// Shared state for the dashboard server.
struct DashboardState {
    audit_path: std::path::PathBuf,
}

/// Query parameters for the entries endpoint.
#[derive(Deserialize)]
struct EntriesQuery {
    agent: Option<String>,
    decision: Option<String>,
    action: Option<String>,
    search: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}

/// Summary statistics response.
#[derive(serde::Serialize)]
struct DashboardStats {
    total: u64,
    allowed: u64,
    denied: u64,
    agents: Vec<String>,
    sessions: Vec<String>,
    chain_integrity: bool,
}

/// Paginated entries response.
#[derive(serde::Serialize)]
struct EntriesResponse {
    entries: Vec<audit_log::AuditEntry>,
    total: usize,
    offset: usize,
    limit: usize,
}

/// Session summary.
#[derive(serde::Serialize)]
struct SessionSummary {
    session_id: String,
    agent_name: String,
    identity_id: Option<String>,
    entry_count: usize,
    allowed: usize,
    denied: usize,
    start_time: Option<String>,
    end_time: Option<String>,
}

/// Sessions response.
#[derive(serde::Serialize)]
struct SessionsResponse {
    sessions: Vec<SessionSummary>,
}

/// Chain verification response.
#[derive(serde::Serialize)]
struct VerifyResponse {
    status: String,
    entries: u64,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Start the dashboard HTTP server.
///
/// Serves the static dashboard HTML and exposes REST API endpoints for
/// querying the audit log. Runs on `127.0.0.1:8081` by default.
pub async fn serve_dashboard(_port: u16) -> anyhow::Result<()> {
    let audit_path = audit_log::default_audit_log_path();
    let state = Arc::new(DashboardState { audit_path });

    let app = axum::Router::new()
        .route("/", get(index_handler))
        .route("/api/entries", get(entries_handler))
        .route("/api/stats", get(stats_handler))
        .route("/api/sessions", get(sessions_handler))
        .route("/api/verify", get(verify_handler))
        .layer(tower_http::cors::CorsLayer::permissive())
        .with_state(state);

    let addr = format!("127.0.0.1:{}", 8081);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("✓ Dashboard running at http://{}", addr);
    println!("  Open in your browser to view the audit log.");
    println!("  Press Ctrl+C to stop.\n");

    axum::serve(listener, app).await?;
    Ok(())
}

/// Serve the dashboard HTML.
async fn index_handler() -> Html<String> {
    // Embed the dashboard HTML directly
    Html(DASHBOARD_HTML.to_string())
}

/// GET /api/entries — paginated audit entries with filters.
async fn entries_handler(
    State(state): State<Arc<DashboardState>>,
    Query(query): Query<EntriesQuery>,
) -> Result<Json<EntriesResponse>, (axum::http::StatusCode, String)> {
    let logger = AuditLogger::open(&state.audit_path)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let all = logger
        .read_all()
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let limit = query.limit.unwrap_or(100).min(1000);
    let offset = query.offset.unwrap_or(0);

    let filtered: Vec<AuditEntry> = all
        .into_iter()
        .filter(|e| {
            if let Some(ref agent) = query.agent {
                if e.agent_name != *agent {
                    return false;
                }
            }
            if let Some(ref decision) = query.decision {
                match decision.as_str() {
                    "allow" if !e.decision.is_allowed() => return false,
                    "deny" if e.decision.is_allowed() => return false,
                    _ => {}
                }
            }
            if let Some(ref action) = query.action {
                if e.action != *action {
                    return false;
                }
            }
            if let Some(ref search) = query.search {
                let q = search.to_lowercase();
                if !e.resource.to_lowercase().contains(&q) && !e.action.to_lowercase().contains(&q)
                {
                    return false;
                }
            }
            true
        })
        .collect::<Vec<_>>();

    let total = filtered.len();
    let entries: Vec<AuditEntry> = filtered.into_iter().skip(offset).take(limit).collect();

    Ok::<_, (axum::http::StatusCode, String)>(axum::Json(EntriesResponse {
        entries,
        total,
        offset,
        limit,
    }))
}

/// GET /api/stats — summary statistics.
async fn stats_handler(
    State(state): State<Arc<DashboardState>>,
) -> Result<axum::Json<DashboardStats>, (axum::http::StatusCode, String)> {
    let logger = AuditLogger::open(&state.audit_path)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let counts = logger
        .count_by_decision()
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let all = logger
        .read_all()
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let agents: Vec<String> = {
        let mut a: Vec<String> = all.iter().map(|e| e.agent_name.clone()).collect();
        a.sort();
        a.dedup();
        a
    };
    let sessions: Vec<String> = {
        let mut s: Vec<String> = all.iter().map(|e| e.session_id.to_string()).collect();
        s.sort();
        s.dedup();
        s
    };

    let chain_ok = logger.verify_chain().is_ok();

    Ok(axum::Json(DashboardStats {
        total: counts.total,
        allowed: counts.allowed,
        denied: counts.denied,
        agents,
        sessions,
        chain_integrity: chain_ok,
    }))
}

/// GET /api/sessions — session summaries with entry counts.
async fn sessions_handler(
    State(state): State<Arc<DashboardState>>,
) -> Result<axum::Json<SessionsResponse>, (axum::http::StatusCode, String)> {
    let logger = AuditLogger::open(&state.audit_path)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let all = logger
        .read_all()
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Group entries by session_id
    let mut session_map: std::collections::HashMap<String, Vec<&AuditEntry>> =
        std::collections::HashMap::new();
    for entry in &all {
        session_map
            .entry(entry.session_id.to_string())
            .or_default()
            .push(entry);
    }

    let mut sessions: Vec<SessionSummary> = session_map
        .into_iter()
        .map(|(sid, entries)| {
            let allowed = entries.iter().filter(|e| e.decision.is_allowed()).count();
            let denied = entries.iter().filter(|e| e.decision.is_denied()).count();
            let agent_name = entries
                .first()
                .map(|e| e.agent_name.clone())
                .unwrap_or_default();
            let identity_id = entries
                .first()
                .and_then(|e| e.identity_id.map(|id| id.to_string()));
            let start_time = entries.iter().map(|e| e.timestamp).min();
            let end_time = entries.iter().map(|e| e.timestamp).max();

            SessionSummary {
                session_id: sid,
                agent_name,
                identity_id,
                entry_count: entries.len(),
                allowed,
                denied,
                start_time: start_time.map(|t| t.to_rfc3339()),
                end_time: end_time.map(|t| t.to_rfc3339()),
            }
        })
        .collect();

    sessions.sort_by(|a, b| b.start_time.cmp(&a.start_time));

    Ok(axum::Json(SessionsResponse { sessions }))
}

/// GET /api/verify — chain integrity verification.
async fn verify_handler(
    State(state): State<Arc<DashboardState>>,
) -> Result<axum::Json<VerifyResponse>, (axum::http::StatusCode, String)> {
    let logger = AuditLogger::open(&state.audit_path)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let all = logger
        .read_all()
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let entries_count = all.len() as u64;

    match logger.verify_chain() {
        Ok(()) => Ok(axum::Json(VerifyResponse {
            status: "ok".to_string(),
            entries: entries_count,
            path: state.audit_path.display().to_string(),
            error: None,
        })),
        Err(e) => Ok(axum::Json(VerifyResponse {
            status: "broken".to_string(),
            entries: entries_count,
            path: state.audit_path.display().to_string(),
            error: Some(e.to_string()),
        })),
    }
}

/// The embedded dashboard HTML. This is the same as `public/dashboard/index.html`
/// but with an added "Connect to API" mode that fetches from the local server.
const DASHBOARD_HTML: &str = include_str!("../../../public/dashboard/index.html");

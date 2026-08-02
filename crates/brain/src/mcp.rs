//! The brain's surface: an MCP server over stdio exposing LLM-free retrieval
//! primitives (`brain_search`, `brain_who_knows`, `brain_recent_prs`,
//! `brain_audit`), following the Cerebras design — narrow, structured, stable
//! tools that any MCP client (Hermes, Claude Code, sandboxed agents) uses as
//! its orchestration engine.
//!
//! The [`BrainRuntime`] wires store + provider + connectors into the retrieval
//! pipeline: exact + semantic candidates → recency → RRF fusion → optional
//! rerank → evidence packets with content hashes for provenance.

use crate::connectors::{expand_path, AuditLogConnector, Connector, GitHubConnector};
use crate::provider::{BrainProvider, SYNTHESIZE_SYSTEM_PREFIX};
use crate::retrieval::{normalize, recency_weight, rrf_merge};
use crate::store::KnowledgeStore;
use crate::Evidence;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{info, warn};

/// Default half-life for recency decay (days). Slack answers expire.
const RECENCY_HALF_LIFE_DAYS: f64 = 180.0;
/// RRF smoothing constant (Cerebras: consensus > single top vote).
const RRF_K: f64 = 60.0;

#[derive(Debug, Clone, Default)]
pub struct SearchParams {
    pub query: String,
    pub project: Option<String>,
    pub top_k: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhoEntry {
    pub name: String,
    pub score: f64,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub chain_ok: bool,
    pub entries: Vec<AuditHit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditHit {
    pub seq: u64,
    pub agent: String,
    pub action: String,
    pub resource: String,
    pub decision: String,
    pub timestamp: String,
    pub self_hash: String,
}

/// The composed brain: store + provider + connectors. This is what binaries
/// and tests drive; the MCP server is just a thin JSON-RPC shell over it.
///
/// Typed connector handles (`github`, `audit`) avoid trait-object downcasting;
/// the generic `connectors` list is for the ingest pipeline.
pub struct BrainRuntime {
    pub store: Arc<KnowledgeStore>,
    pub provider: Arc<dyn BrainProvider>,
    pub connectors: Vec<Arc<dyn Connector>>,
    pub github: Option<GitHubConnector>,
    pub audit: Option<AuditLogConnector>,
}

impl BrainRuntime {
    pub fn new(
        store: KnowledgeStore,
        provider: Arc<dyn BrainProvider>,
        github: Option<GitHubConnector>,
        audit: Option<AuditLogConnector>,
        connectors: Vec<Arc<dyn Connector>>,
    ) -> Self {
        Self {
            store: Arc::new(store),
            provider,
            connectors,
            github,
            audit,
        }
    }

    /// Hybrid retrieval pipeline: exact + semantic → recency → RRF → rerank.
    pub async fn search(&self, params: &SearchParams) -> Result<Vec<Evidence>> {
        let now = chrono::Utc::now().timestamp() as f64;
        let project = params.project.as_deref();
        let width = (params.top_k.max(1) * 4).min(200);

        // Per-hit timestamps, fetched once (avoid N queries by fetching ids in
        // bulk later — the recency pass uses a single bulk lookup per leg).
        let mut exact = self
            .store
            .exact_search(&params.query, project, width)
            .await?;
        {
            let rows = self
                .store
                .rows_by_id(&exact.iter().map(|h| h.id).collect::<Vec<_>>())
                .await?;
            let created: std::collections::HashMap<i64, f64> =
                rows.iter().map(|r| (r.id, r.created_at as f64)).collect();
            for h in exact.iter_mut() {
                let age = now - created.get(&h.id).copied().unwrap_or(now);
                h.score *= recency_weight(age, RECENCY_HALF_LIFE_DAYS);
            }
        }
        exact.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let exact_ids: Vec<i64> = exact.iter().map(|h| h.id).collect();

        let mut semantic: Vec<crate::store::RankedHit> = vec![];
        match self.provider.embed(&params.query).await {
            Ok(emb) => {
                semantic = self.store.semantic_search(&emb, project, width).await?;
                let rows = self
                    .store
                    .rows_by_id(&semantic.iter().map(|h| h.id).collect::<Vec<_>>())
                    .await?;
                let created: std::collections::HashMap<i64, f64> =
                    rows.iter().map(|r| (r.id, r.created_at as f64)).collect();
                for h in semantic.iter_mut() {
                    let age = now - created.get(&h.id).copied().unwrap_or(now);
                    h.score *= recency_weight(age, RECENCY_HALF_LIFE_DAYS);
                }
                semantic.sort_by(|a, b| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            Err(e) => warn!(error = %e, "brain: embed failed — semantic leg skipped"),
        }
        let semantic_ids: Vec<i64> = semantic.iter().map(|h| h.id).collect();

        // Rank-based fusion: each leg contributes its *order*, not raw scores
        // (RRF, per Cerebras — consensus across legs beats a single top vote).
        let exact_ranked: Vec<(u64, f64)> = exact_ids.iter().map(|id| (*id as u64, 1.0)).collect();
        let semantic_ranked: Vec<(u64, f64)> =
            semantic_ids.iter().map(|id| (*id as u64, 1.0)).collect();

        let fused = normalize(rrf_merge(&[exact_ranked, semantic_ranked], RRF_K));
        let ids: Vec<i64> = fused.iter().map(|(id, _)| *id as i64).take(width).collect();
        let rows = self.store.rows_by_id(&ids).await?;
        let mut by_id = std::collections::HashMap::new();
        for r in rows {
            by_id.insert(r.id, r);
        }

        let mut candidates: Vec<Evidence> = fused
            .iter()
            .filter_map(|(id, score)| {
                let row = by_id.get(&(*id as i64))?;
                Some(Evidence {
                    id: row.id,
                    source: row.source.clone(),
                    source_id: row.source_id.clone(),
                    kind: row.kind.clone(),
                    title: row.title.clone(),
                    author: row.author.clone(),
                    summary: row.summary.clone(),
                    question: row.question.clone(),
                    resolution: row.resolution.clone(),
                    systems: row
                        .systems
                        .as_ref()
                        .map(|s| s.split(',').map(String::from).collect())
                        .unwrap_or_default(),
                    project: row.project.clone(),
                    created_at: row.created_at,
                    content_hash: row.content_hash.clone(),
                    score: *score,
                })
            })
            .take(params.top_k.max(1))
            .collect();

        // Optional rerank pass; on failure the RRF scores stand.
        if candidates.len() >= 2 {
            match self.provider.rerank(&params.query, &candidates).await {
                Ok(scores) => {
                    for (ev, s) in candidates.iter_mut().zip(scores.iter()) {
                        ev.score = 0.7 * s + 0.3 * ev.score;
                    }
                    candidates.sort_by(|a, b| {
                        b.score
                            .partial_cmp(&a.score)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
                Err(e) => warn!(error = %e, "brain: rerank skipped — RRF scores stand"),
            }
        }
        Ok(candidates)
    }

    /// Who demonstrably knows a topic: authors of matched knowledge rows plus
    /// agents seen touching matching resources in the audit trail.
    pub async fn who_knows(&self, topic: &str, limit: usize) -> Result<Vec<WhoEntry>> {
        let params = SearchParams {
            query: topic.to_string(),
            project: None,
            top_k: 100,
        };
        let mut acc: std::collections::HashMap<String, (f64, std::collections::HashSet<String>)> =
            std::collections::HashMap::new();
        for ev in self.search(&params).await? {
            if let Some(author) = ev.author {
                let entry = acc.entry(author.clone()).or_default();
                entry.0 += ev.score;
                entry.1.insert(ev.source.clone());
            }
        }
        if let Some(audit) = &self.audit {
            if let Ok((entries, _)) = audit.read_verified() {
                let needle = topic.to_lowercase();
                for e in entries {
                    let hay = format!("{} {}", e.action, e.resource).to_lowercase();
                    if hay.contains(&needle) {
                        let entry = acc.entry(e.agent_name.clone()).or_default();
                        entry.0 += 1.0;
                        entry.1.insert("audit".into());
                    }
                }
            }
        }
        let mut out: Vec<WhoEntry> = acc
            .into_iter()
            .map(|(name, (score, sources))| {
                let mut s: Vec<String> = sources.into_iter().collect();
                s.sort();
                WhoEntry {
                    name,
                    score,
                    sources: s,
                }
            })
            .collect();
        out.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        out.truncate(limit);
        Ok(out)
    }

    /// Recent PRs from the GitHub connector (direct REST, no LLM). Scoped by
    /// the connector's configured owner/repo; `owner`/`repo` args are accepted
    /// for forward-compat but ignored when the config is single-repo.
    pub async fn recent_prs(&self, limit: usize) -> Result<Vec<Evidence>> {
        let Some(gh) = &self.github else {
            anyhow::bail!("brain: no github connector configured");
        };
        let docs = gh.fetch().await?;
        let mut hits: Vec<Evidence> = docs
            .into_iter()
            .filter(|d| d.kind == "pr")
            .take(limit)
            .map(|d| Evidence {
                id: 0,
                source: d.source,
                source_id: d.source_id,
                kind: d.kind,
                title: d.title,
                author: d.author,
                summary: None,
                question: None,
                resolution: None,
                systems: vec![],
                project: d.project,
                created_at: d.created_at,
                content_hash: String::new(),
                score: 0.0,
            })
            .collect();
        hits.sort_by_key(|h| std::cmp::Reverse(h.created_at));
        Ok(hits)
    }

    /// Audit lookup: chain verification + filtered entries.
    pub async fn audit(
        &self,
        agent: Option<&str>,
        since_ts: Option<i64>,
        limit: usize,
    ) -> Result<AuditReport> {
        let Some(audit) = &self.audit else {
            anyhow::bail!("brain: no audit connector configured");
        };
        let (entries, chain_ok) = audit.read_verified()?;
        let hits: Vec<AuditHit> = entries
            .into_iter()
            .filter(|e| agent.map(|a| e.agent_name == a).unwrap_or(true))
            .filter(|e| {
                since_ts
                    .map(|s| {
                        chrono::DateTime::parse_from_rfc3339(&e.timestamp)
                            .map(|d| d.timestamp() >= s)
                            .unwrap_or(false)
                    })
                    .unwrap_or(true)
            })
            .rev()
            .take(limit)
            .map(|e| AuditHit {
                decision: match e.decision {
                    crate::connectors::DecisionMirror::Allow => "ALLOW".into(),
                    crate::connectors::DecisionMirror::Deny(r) => format!("DENY({r})"),
                },
                seq: e.seq,
                agent: e.agent_name,
                action: e.action,
                resource: e.resource,
                timestamp: e.timestamp,
                self_hash: e.self_hash,
            })
            .collect();
        Ok(AuditReport {
            chain_ok,
            entries: hits,
        })
    }

    /// Grounded answer: search → synthesize with static-prefix prompt.
    pub async fn answer(&self, query: &str, project: Option<&str>, top_k: usize) -> Result<String> {
        let evidence = self
            .search(&SearchParams {
                query: query.to_string(),
                project: project.map(String::from),
                top_k,
            })
            .await?;
        self.provider
            .synthesize(SYNTHESIZE_SYSTEM_PREFIX, query, &evidence)
            .await
    }

    /// Config-driven runtime (see `brain.toml.example`).
    pub async fn from_config(path: &Path) -> Result<Self> {
        let cfg = BrainConfig::load(path)?;
        let store = KnowledgeStore::connect(&cfg.store.database_url)
            .await
            .context("brain: connect store (config)")?;
        let provider: Arc<dyn BrainProvider> = Arc::new(crate::provider::DeepSeekLiteLLM::new(
            cfg.litellm.base_url,
            cfg.litellm.api_key,
            cfg.models.clone(),
        ));

        let mut github = None;
        let mut audit = None;
        let mut connectors: Vec<Arc<dyn Connector>> = vec![];
        if let Some(g) = &cfg.connectors.github {
            let token = g.token_env.as_ref().and_then(|env| std::env::var(env).ok());
            let gh = GitHubConnector::new(g.owner.clone(), g.repo.clone(), token).with_project(
                g.project
                    .clone()
                    .unwrap_or_else(|| format!("{}/{}", g.owner, g.repo)),
            );
            connectors.push(Arc::new(gh.clone()));
            github = Some(gh);
        }
        if let Some(a) = &cfg.connectors.audit {
            let path = expand_path(&a.path);
            let ac = AuditLogConnector::new(path)
                .with_project(a.project.clone().unwrap_or_else(|| "audit".into()));
            connectors.push(Arc::new(ac.clone()));
            audit = Some(ac);
        }
        info!(
            "brain: runtime ready — store={} connectors={}",
            cfg.store.database_url,
            connectors.len()
        );
        Ok(Self::new(store, provider, github, audit, connectors))
    }
}

impl std::fmt::Debug for BrainRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrainRuntime").finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct BrainConfig {
    #[serde(default)]
    pub store: StoreConfig,
    #[serde(default)]
    pub models: crate::BrainModels,
    pub litellm: LiteLlmConfig,
    #[serde(default)]
    pub connectors: ConnectorsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StoreConfig {
    #[serde(default = "default_database_url")]
    pub database_url: String,
}

fn default_database_url() -> String {
    "sqlite:data/brain.db?mode=rwc".into()
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            database_url: default_database_url(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LiteLlmConfig {
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default = "default_api_key")]
    pub api_key: String,
}

fn default_base_url() -> String {
    "http://192.168.1.13:4000/v1".into()
}

fn default_api_key() -> String {
    "sk-no-key-needed".into()
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ConnectorsConfig {
    pub github: Option<GithubConfig>,
    pub audit: Option<AuditConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GithubConfig {
    pub owner: String,
    pub repo: String,
    pub token_env: Option<String>,
    pub project: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuditConfig {
    #[serde(default = "default_audit_path")]
    pub path: String,
    pub project: Option<String>,
}

fn default_audit_path() -> String {
    "~/.agenticbox/audit/audit.jsonl".into()
}

impl BrainConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("brain: read config {}", path.display()))?;
        let cfg: Self = toml::from_str(&text).context("brain: parse config")?;
        if cfg.litellm.base_url.is_empty() {
            anyhow::bail!("brain: config [litellm] base_url is required");
        }
        Ok(cfg)
    }

    /// `$BRAIN_CONFIG` or `~/.agenticbox/brain.toml`.
    pub fn default_path() -> PathBuf {
        std::env::var("BRAIN_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| expand_path("~/.agenticbox/brain.toml"))
    }
}

impl Default for BrainConfig {
    fn default() -> Self {
        Self {
            store: StoreConfig::default(),
            models: crate::BrainModels::default(),
            litellm: LiteLlmConfig {
                base_url: default_base_url(),
                api_key: default_api_key(),
            },
            connectors: ConnectorsConfig::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// MCP over stdio (JSON-RPC 2.0)
// ---------------------------------------------------------------------------

const PROTOCOL_VERSION: &str = "2025-03-26";

pub async fn serve_stdio(runtime: Arc<BrainRuntime>) -> Result<()> {
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut stdout = tokio::io::stdout();
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break; // EOF
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match handle_line(&runtime, trimmed).await {
            Ok(Some(resp)) => {
                let mut payload = serde_json::to_string(&resp)?;
                payload.push('\n');
                stdout.write_all(payload.as_bytes()).await?;
                stdout.flush().await?;
            }
            Ok(None) => {}
            Err(e) => {
                warn!(error = %e, "brain: mcp request failed");
                // Best-effort error response (notifications get none).
                if let Ok(req) = serde_json::from_str::<Value>(trimmed) {
                    if let Some(id) = req.get("id").cloned() {
                        let resp = json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": { "code": -32603, "message": e.to_string() }
                        });
                        let mut payload = serde_json::to_string(&resp)?;
                        payload.push('\n');
                        stdout.write_all(payload.as_bytes()).await?;
                        stdout.flush().await?;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Handle one JSON-RPC message; `None` for notifications.
pub async fn handle_line(runtime: &BrainRuntime, line: &str) -> Result<Option<Value>> {
    let msg: Value = serde_json::from_str(line).context("brain: invalid JSON-RPC")?;
    let id = msg.get("id").cloned();
    let method = msg
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("brain: missing method"))?
        .to_string();
    let params = msg.get("params").cloned().unwrap_or(Value::Null);

    let result = match method.as_str() {
        "initialize" => Some(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "agenticbox-brain", "version": env!("CARGO_PKG_VERSION") }
        })),
        "notifications/initialized" => None,
        "ping" => Some(json!({})),
        "tools/list" => Some(tools_list()),
        "tools/call" => Some(tools_call(runtime, &params).await?),
        other => anyhow::bail!("brain: unknown method {other}"),
    };

    match (id, result) {
        (Some(id), Some(result)) => Ok(Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        }))),
        (None, _) => Ok(None), // notification
        (Some(id), None) => Ok(Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {}
        }))),
    }
}

fn tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "brain_search",
                "description": "Hybrid retrieval over the company knowledge base (GitHub, Slack, docs, audit). Returns evidence packets with content hashes for provenance.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Question or search phrase" },
                        "project": { "type": "string", "description": "Scope to a project (e.g. agenticbox-core)" },
                        "top_k": { "type": "integer", "default": 5 }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "brain_who_knows",
                "description": "Find people/agents with demonstrated expertise on a topic (authors of matched rows + audit trail evidence).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "topic": { "type": "string" },
                        "limit": { "type": "integer", "default": 5 }
                    },
                    "required": ["topic"]
                }
            },
            {
                "name": "brain_recent_prs",
                "description": "Recent pull requests from the configured repository.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "default": 10 }
                    }
                }
            },
            {
                "name": "brain_audit",
                "description": "Audit trail lookup with SHA-256 chain verification. THE provenance tool: what agents actually did, tamper-evident.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "agent": { "type": "string" },
                        "since_unix": { "type": "integer" },
                        "limit": { "type": "integer", "default": 20 }
                    }
                }
            }
        ]
    })
}

async fn tools_call(runtime: &BrainRuntime, params: &Value) -> Result<Value> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("tools/call missing name"))?;
    let args = params.get("arguments").cloned().unwrap_or(Value::Null);
    tracing::debug!(tool = name, "brain: tool call");

    match name {
        "brain_search" => {
            let query = args
                .get("query")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("brain_search: query required"))?;
            let project = args.get("project").and_then(Value::as_str);
            let top_k = args.get("top_k").and_then(Value::as_u64).unwrap_or(5) as usize;
            let results = runtime
                .search(&SearchParams {
                    query: query.to_string(),
                    project: project.map(String::from),
                    top_k,
                })
                .await?;
            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&results)?
                }],
                "isError": false
            }))
        }
        "brain_who_knows" => {
            let topic = args
                .get("topic")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("brain_who_knows: topic required"))?;
            let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(5) as usize;
            let people = runtime.who_knows(topic, limit).await?;
            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&people)?
                }],
                "isError": false
            }))
        }
        "brain_recent_prs" => {
            let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(10) as usize;
            let prs = runtime.recent_prs(limit).await?;
            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&prs)?
                }],
                "isError": false
            }))
        }
        "brain_audit" => {
            let agent = args.get("agent").and_then(Value::as_str);
            let since = args.get("since_unix").and_then(Value::as_i64);
            let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(20) as usize;
            let report = runtime.audit(agent, since, limit).await?;
            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&report)?
                }],
                "isError": false
            }))
        }
        other => anyhow::bail!("brain: unknown tool {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct NoopProvider {
        models: crate::BrainModels,
    }

    #[async_trait]
    impl BrainProvider for NoopProvider {
        async fn distill(&self, _s: &str, _c: &str) -> Result<crate::DistillOutcome> {
            Ok(crate::DistillOutcome::default())
        }
        async fn rerank(&self, _q: &str, _c: &[Evidence]) -> Result<Vec<f64>> {
            Ok(vec![])
        }
        async fn synthesize(&self, _s: &str, _q: &str, _e: &[Evidence]) -> Result<String> {
            Ok(String::new())
        }
        async fn embed(&self, _t: &str) -> Result<Vec<f32>> {
            anyhow::bail!("no embed model in test")
        }
        fn models(&self) -> &crate::BrainModels {
            &self.models
        }
        fn cache_stats(&self) -> crate::CacheStats {
            crate::CacheStats::default()
        }
    }

    async fn runtime_with_memory_store() -> BrainRuntime {
        let store = KnowledgeStore::connect("sqlite::memory:").await.unwrap();
        let provider: Arc<dyn BrainProvider> = Arc::new(NoopProvider {
            models: crate::BrainModels::default(),
        });
        BrainRuntime::new(store, provider, None, None, vec![])
    }

    #[tokio::test]
    async fn tools_list_is_stable() {
        let v = tools_list();
        let tools = v["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 4);
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert_eq!(
            names,
            vec![
                "brain_search",
                "brain_who_knows",
                "brain_recent_prs",
                "brain_audit"
            ]
        );
    }

    #[tokio::test]
    async fn initialize_handshake() {
        let runtime = runtime_with_memory_store().await;
        let line = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let resp = handle_line(&runtime, line).await.unwrap().unwrap();
        assert_eq!(resp["result"]["serverInfo"]["name"], "agenticbox-brain");
        assert_eq!(resp["id"], 1);
    }

    #[tokio::test]
    async fn notification_gets_no_response() {
        let runtime = runtime_with_memory_store().await;
        let line = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        let resp = handle_line(&runtime, line).await.unwrap();
        assert!(resp.is_none());
    }

    #[tokio::test]
    async fn unknown_method_is_error() {
        let runtime = runtime_with_memory_store().await;
        let line = r#"{"jsonrpc":"2.0","id":2,"method":"bogus"}"#;
        let err = handle_line(&runtime, line).await.unwrap_err();
        assert!(err.to_string().contains("unknown method"));
    }

    #[tokio::test]
    async fn brain_search_requires_query() {
        let runtime = runtime_with_memory_store().await;
        let line = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"brain_search","arguments":{}}}"#;
        let res = handle_line(&runtime, line).await;
        assert!(res.is_err(), "missing query must fail: {res:?}");
    }

    #[tokio::test]
    async fn brain_audit_without_connector_fails_cleanly() {
        let runtime = runtime_with_memory_store().await;
        let line = r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"brain_audit","arguments":{}}}"#;
        let res = handle_line(&runtime, line).await;
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.to_string().contains("no audit connector"));
    }
}

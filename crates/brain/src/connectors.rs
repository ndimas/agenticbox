//! Data connectors: each one emits the same [`RawDoc`] shape into the store
//! (the Cerebras "meet data where it lives" pattern — nothing gets moved into
//! a new system, the brain just indexes it in place).
//!
//! New sources are new `Connector` impls: Slack/Discord (distilled threads),
//! Notion/Linear, docs — and the crown jewel, the AgenticBox audit trail,
//! which gives the brain *receipts*: facts traceable to tamper-evident,
//! chain-hashed records of what agents actually did.

use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A connector-normalized document, ready for distillation + embedding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawDoc {
    pub source: String,
    pub source_id: String,
    pub kind: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub raw_text: String,
    pub created_at: i64,
    pub project: Option<String>,
}

#[async_trait]
pub trait Connector: Send + Sync {
    fn name(&self) -> &str;
    async fn fetch(&self) -> Result<Vec<RawDoc>>;
}

// ---------------------------------------------------------------------------
// GitHub
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct GitHubConnector {
    owner: String,
    repo: String,
    token: Option<String>,
    client: Client,
    project: Option<String>,
}

impl GitHubConnector {
    pub fn new(owner: impl Into<String>, repo: impl Into<String>, token: Option<String>) -> Self {
        Self {
            owner: owner.into(),
            repo: repo.into(),
            token,
            client: Client::new(),
            project: None,
        }
    }

    pub fn with_project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }

    async fn get_json(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("https://api.github.com{path}");
        let mut req = self
            .client
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "agenticbox-brain");
        if let Some(tok) = &self.token {
            req = req.bearer_auth(tok);
        }
        let resp = req.send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("github {path}: HTTP {status}: {text}");
        }
        Ok(serde_json::from_str(&text)?)
    }

    /// Issues and PRs updated in the last `days` days (paginated, 50/page).
    pub async fn fetch_recent(&self, days: u32) -> Result<Vec<RawDoc>> {
        let since = chrono::Utc::now().timestamp() - i64::from(days) * 86_400;
        let mut docs = self.fetch_issues_since(since).await?;
        docs.extend(self.fetch_prs_since(since).await?);
        Ok(docs)
    }

    async fn fetch_issues_since(&self, since_ts: i64) -> Result<Vec<RawDoc>> {
        let mut docs = vec![];
        let mut page = 1;
        loop {
            let since = chrono::DateTime::from_timestamp(since_ts, 0)
                .map(|d| d.to_rfc3339())
                .unwrap_or_default();
            let v = self
                .get_json(&format!(
                    "/repos/{}/{}/issues?state=all&sort=updated&direction=desc&since={}&per_page=50&page={}",
                    self.owner, self.repo, since, page
                ))
                .await?;
            let arr = v.as_array().cloned().unwrap_or_default();
            if arr.is_empty() {
                break;
            }
            for item in &arr {
                // The issues endpoint also returns PRs; skip those here.
                if item.get("pull_request").is_some() {
                    continue;
                }
                if let Some(doc) = self.issue_to_doc(item) {
                    docs.push(doc);
                }
            }
            if arr.len() < 50 {
                break;
            }
            page += 1;
        }
        Ok(docs)
    }

    async fn fetch_prs_since(&self, since_ts: i64) -> Result<Vec<RawDoc>> {
        let mut docs = vec![];
        let mut page = 1;
        loop {
            let since = chrono::DateTime::from_timestamp(since_ts, 0)
                .map(|d| d.to_rfc3339())
                .unwrap_or_default();
            let v = self
                .get_json(&format!(
                    "/repos/{}/{}/pulls?state=all&sort=updated&direction=desc&since={}&per_page=50&page={}",
                    self.owner, self.repo, since, page
                ))
                .await?;
            let arr = v.as_array().cloned().unwrap_or_default();
            if arr.is_empty() {
                break;
            }
            for item in &arr {
                if let Some(doc) = self.pr_to_doc(item) {
                    docs.push(doc);
                }
            }
            if arr.len() < 50 {
                break;
            }
            page += 1;
        }
        Ok(docs)
    }

    fn issue_to_doc(&self, item: &serde_json::Value) -> Option<RawDoc> {
        let number = item.get("number")?.as_u64()?;
        let title = item.get("title")?.as_str()?;
        let body = item.get("body").and_then(|b| b.as_str()).unwrap_or("");
        let author = item
            .get("user")
            .and_then(|u| u.get("login"))
            .and_then(|l| l.as_str());
        let created = item
            .get("created_at")
            .and_then(|t| t.as_str())
            .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
            .map(|d| d.timestamp())?;
        Some(RawDoc {
            source: "github".into(),
            source_id: format!("issue/{number}"),
            kind: "issue".into(),
            title: Some(title.to_string()),
            author: author.map(String::from),
            raw_text: format!("# {title}\n\n{body}"),
            created_at: created,
            project: self.project.clone(),
        })
    }

    fn pr_to_doc(&self, item: &serde_json::Value) -> Option<RawDoc> {
        let number = item.get("number")?.as_u64()?;
        let title = item.get("title")?.as_str()?;
        let body = item.get("body").and_then(|b| b.as_str()).unwrap_or("");
        let author = item
            .get("user")
            .and_then(|u| u.get("login"))
            .and_then(|l| l.as_str());
        let created = item
            .get("created_at")
            .and_then(|t| t.as_str())
            .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
            .map(|d| d.timestamp())?;
        Some(RawDoc {
            source: "github".into(),
            source_id: format!("pr/{number}"),
            kind: "pr".into(),
            title: Some(title.to_string()),
            author: author.map(String::from),
            raw_text: format!("# PR {number}: {title}\n\n{body}"),
            created_at: created,
            project: self.project.clone(),
        })
    }
}

#[async_trait]
impl Connector for GitHubConnector {
    fn name(&self) -> &str {
        "github"
    }

    async fn fetch(&self) -> Result<Vec<RawDoc>> {
        self.fetch_recent(30).await
    }
}

// ---------------------------------------------------------------------------
// Audit log — the crown jewel: memory with receipts.
// ---------------------------------------------------------------------------

/// Mirror of `audit-log::AuditEntry` (serde-compatible; `audit-log` remains
/// the canonical writer). Kept local so the brain has no dependency on the
/// writer crate's internals — only its on-disk JSONL shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntryMirror {
    pub seq: u64,
    pub timestamp: String,
    pub session_id: String,
    pub agent_name: String,
    #[serde(default)]
    pub identity_id: Option<String>,
    pub action: String,
    pub resource: String,
    pub decision: DecisionMirror,
    pub prev_hash: String,
    pub self_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum DecisionMirror {
    Allow,
    Deny(String),
}

#[derive(Debug, Clone)]
pub struct AuditLogConnector {
    path: PathBuf,
    project: Option<String>,
}

impl AuditLogConnector {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            project: None,
        }
    }

    pub fn with_project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }

    /// Read + chain-verify the audit log. Returns entries and whether the
    /// SHA-256 prev/self chain is intact (tamper evidence).
    pub fn read_verified(&self) -> Result<(Vec<AuditEntryMirror>, bool)> {
        let content = std::fs::read_to_string(&self.path)
            .with_context(|| format!("brain: read audit log {}", self.path.display()))?;
        let mut entries = vec![];
        let mut chain_ok = true;
        let mut prev_self_hash: Option<String> = None;
        for (idx, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let entry: AuditEntryMirror = serde_json::from_str(line)
                .with_context(|| format!("brain: audit line {} malformed", idx + 1))?;
            if let Some(expected_prev) = &prev_self_hash {
                if &entry.prev_hash != expected_prev {
                    tracing::warn!(
                        seq = entry.seq,
                        "brain: audit chain broken at entry {} (prev_hash mismatch)",
                        entry.seq
                    );
                    chain_ok = false;
                }
            } else if entry.prev_hash != "genesis" {
                tracing::warn!(seq = entry.seq, "brain: first audit entry not genesis");
                chain_ok = false;
            }
            prev_self_hash = Some(entry.self_hash.clone());
            entries.push(entry);
        }
        Ok((entries, chain_ok))
    }
}

#[async_trait]
impl Connector for AuditLogConnector {
    fn name(&self) -> &str {
        "audit"
    }

    async fn fetch(&self) -> Result<Vec<RawDoc>> {
        let (entries, chain_ok) = self.read_verified()?;
        if !chain_ok {
            tracing::warn!("brain: audit chain verification FAILED — index contents with caution");
        }
        Ok(entries
            .iter()
            .map(|e| {
                let decision = match &e.decision {
                    DecisionMirror::Allow => "ALLOW".to_string(),
                    DecisionMirror::Deny(r) => format!("DENY({r})"),
                };
                let ts = chrono::DateTime::parse_from_rfc3339(&e.timestamp)
                    .map(|d| d.timestamp())
                    .unwrap_or(0);
                RawDoc {
                    source: "audit".into(),
                    source_id: format!("seq/{}", e.seq),
                    kind: "audit".into(),
                    title: Some(format!("{} {} {}", e.agent_name, e.action, e.resource)),
                    author: Some(e.agent_name.clone()),
                    raw_text: format!(
                        "seq={} agent={} action={} resource={} decision={} hash={}",
                        e.seq, e.agent_name, e.action, e.resource, decision, e.self_hash
                    ),
                    created_at: ts,
                    project: self.project.clone(),
                }
            })
            .collect())
    }
}

/// Expand `~` and env vars in a config path (e.g. `$GH_TOKEN`).
pub fn expand_path(p: &str) -> PathBuf {
    let p = p.trim();
    let expanded = if let Some(rest) = p.strip_prefix("~/") {
        std::env::var("HOME").unwrap_or_default() + "/" + rest
    } else {
        p.to_string()
    };
    PathBuf::from(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_chain_detects_tampering() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        std::fs::write(
            &path,
            r#"{"seq":1,"timestamp":"2026-07-30T10:00:00Z","session_id":"s1","agent_name":"demo","action":"fs:read","resource":"/etc/passwd","decision":"Deny(not allowed)","prev_hash":"genesis","self_hash":"abc"}
{"seq":2,"timestamp":"2026-07-30T10:00:01Z","session_id":"s1","agent_name":"demo","action":"fs:write","resource":"/tmp/x","decision":"Allow","prev_hash":"abc","self_hash":"def"}
{"seq":3,"timestamp":"2026-07-30T10:00:02Z","session_id":"s1","agent_name":"demo","action":"network:outbound","resource":"example.com","decision":"Allow","prev_hash":"WRONG","self_hash":"ghi"}
"#,
        )
        .unwrap();
        let c = AuditLogConnector::new(path);
        let (entries, chain_ok) = c.read_verified().unwrap();
        assert_eq!(entries.len(), 3);
        assert!(!chain_ok, "broken prev_hash must be detected");
    }

    #[test]
    fn audit_chain_intact_when_valid() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        std::fs::write(
            &path,
            r#"{"seq":1,"timestamp":"2026-07-30T10:00:00Z","session_id":"s1","agent_name":"demo","action":"fs:read","resource":"/etc/passwd","decision":"Deny(not allowed)","prev_hash":"genesis","self_hash":"abc"}
{"seq":2,"timestamp":"2026-07-30T10:00:01Z","session_id":"s1","agent_name":"demo","action":"fs:write","resource":"/tmp/x","decision":"Allow","prev_hash":"abc","self_hash":"def"}
"#,
        )
        .unwrap();
        let c = AuditLogConnector::new(path);
        let (entries, chain_ok) = c.read_verified().unwrap();
        assert_eq!(entries.len(), 2);
        assert!(chain_ok);
    }

    #[tokio::test]
    async fn audit_connector_emits_raw_docs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        std::fs::write(
            &path,
            r#"{"seq":1,"timestamp":"2026-07-30T10:00:00Z","session_id":"s1","agent_name":"demo","action":"fs:read","resource":"/etc/passwd","decision":"Deny(not allowed)","prev_hash":"genesis","self_hash":"abc"}
"#,
        )
        .unwrap();
        let c = AuditLogConnector::new(path).with_project("agenticbox-core");
        let docs = c.fetch().await.unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].source, "audit");
        assert!(docs[0].raw_text.contains("Deny"));
        assert_eq!(docs[0].author.as_deref(), Some("demo"));
        assert_eq!(docs[0].project.as_deref(), Some("agenticbox-core"));
    }

    #[test]
    fn expand_path_handles_tilde() {
        let p = expand_path("~/data/brain.db");
        assert!(p.to_string_lossy().contains("/data/brain.db"));
    }

    #[test]
    fn expand_path_passthrough() {
        assert_eq!(
            expand_path("sqlite:data/brain.db"),
            PathBuf::from("sqlite:data/brain.db")
        );
    }
}

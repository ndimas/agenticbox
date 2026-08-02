//! AgenticBox Brain — the company knowledge layer.
//!
//! One query surface over GitHub, Slack/Discord, docs, and — the crown jewel —
//! the AgenticBox audit trail. Every answer is an evidence packet whose rows
//! carry a content hash, so provenance can be checked against the audit chain.
//!
//! Model access goes through [`BrainProvider`] (see [`provider`]). The default
//! implementation is DeepSeek-conditional: prompt templates keep a byte-stable
//! static prefix (system instructions, schema, guardrails) with only the
//! variable content appended at the tail, so DeepSeek's prompt cache serves
//! most of every call. New providers (Anthropic, GLM via OpenRouter, local
//! models) implement the same trait — no pipeline changes.
//!
//! Retrieval is hybrid (full-text + embeddings + recency, fused with
//! reciprocal rank fusion) — see [`retrieval`]. Storage is a single SQLite
//! `knowledge` table plus an FTS5 index — see [`store`].
//!
//! Surface: an MCP server over stdio exposing LLM-free retrieval primitives
//! (`brain_search`, `brain_who_knows`, `brain_recent_prs`, `brain_audit`) so
//! Hermes, Claude Code, and sandboxed AgenticBox agents can orchestrate.

pub mod connectors;
pub mod mcp;
pub mod provider;
pub mod retrieval;
pub mod store;

pub use connectors::{AuditLogConnector, Connector, GitHubConnector, RawDoc};
pub use mcp::{serve_stdio, BrainRuntime};
pub use provider::{BrainProvider, CacheStats, DeepSeekLiteLLM, DistillOutcome};
pub use store::{KnowledgeRow, KnowledgeStore, NewKnowledge, StoreStats};

use serde::{Deserialize, Serialize};

/// A single retrieval hit with full provenance, ready for citation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub id: i64,
    pub source: String,
    pub source_id: String,
    pub kind: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub summary: Option<String>,
    pub question: Option<String>,
    pub resolution: Option<String>,
    pub systems: Vec<String>,
    pub project: Option<String>,
    pub created_at: i64,
    pub content_hash: String,
    /// Fusion score from the retrieval pipeline (0..=1-ish, higher = better).
    pub score: f64,
}

impl Evidence {
    /// A compact, citation-ready rendering of this evidence row.
    pub fn snippet(&self) -> String {
        let mut s = format!("[{}/{}] {}", self.source, self.source_id, self.kind);
        if let Some(title) = &self.title {
            s.push_str(&format!(" — {title}"));
        }
        if let Some(summary) = &self.summary {
            s.push_str(&format!("\n  summary: {summary}"));
        }
        if let Some(q) = &self.question {
            s.push_str(&format!("\n  question: {q}"));
        }
        if let Some(r) = &self.resolution {
            s.push_str(&format!("\n  resolution: {r}"));
        }
        if let Some(author) = &self.author {
            s.push_str(&format!("\n  author: {author}"));
        }
        s.push_str(&format!(
            "\n  created: {} hash: {}",
            self.created_at, self.content_hash
        ));
        s
    }
}

/// Named model roles. DeepSeek is the default; every role is configurable so a
/// future provider can take over one or all of them without a rebuild.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainModels {
    /// Model for distill + rerank + synthesize (chat completions).
    pub chat: String,
    /// Model for embeddings. DeepSeek serves no embeddings API, so this is
    /// always pluggable (qwen-embed / bge / etc. via LiteLLM).
    pub embed: String,
}

impl Default for BrainModels {
    fn default() -> Self {
        Self {
            chat: "deepseek-v4".into(),
            embed: "qwen-embed".into(),
        }
    }
}

//! Model access for the brain: the [`BrainProvider`] trait and its default
//! DeepSeek-conditional implementation.
//!
//! **Cache-friendly orchestration is the design.** Every prompt is split into
//! a byte-stable STATIC prefix (system instructions, schema, guardrails —
//! never edited, never reordered) and a DYNAMIC tail (the thread, query, or
//! evidence). DeepSeek v4 caches the longest prefix match, so the marginal
//! cost of the Nth distilled thread or the repeated query collapses. Cache
//! accounting is captured per call into [`CacheStats`] — hit rate is the #1
//! cost lever and the #1 dashboard metric.

use crate::{BrainModels, Evidence};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use model_router::{
    ChatRequest, ChatResponse, EmbeddingProvider, ModelProvider, OpenAIProvider, Usage,
};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// Structured output of distillation: an LLM normalizes a raw thread/doc into
/// the searchable fields (Cerebras: one-line question, summary, resolution,
/// systems mentioned). The original transcript stays in `raw_text`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DistillOutcome {
    pub question: Option<String>,
    pub summary: String,
    pub resolution: Option<String>,
    pub systems: Vec<String>,
    pub title: Option<String>,
}

/// Running cache-accounting across all calls the brain has made. This is the
/// raw material for the "cost per answer" dashboard.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub calls: u64,
    pub hit_tokens: u64,
    pub miss_tokens: u64,
    pub last_hit_ratio: Option<f64>,
    pub last_call_at: Option<DateTime<Utc>>,
}

impl CacheStats {
    pub fn record(&mut self, usage: Option<&Usage>) {
        self.calls += 1;
        self.last_call_at = Some(Utc::now());
        if let Some(u) = usage {
            self.hit_tokens += u.cache_hit_tokens();
            self.miss_tokens += u.cache_miss_tokens();
            self.last_hit_ratio = u.cache_hit_ratio();
        }
    }

    /// Aggregate hit ratio; `None` when no cache accounting has been reported.
    pub fn hit_ratio(&self) -> Option<f64> {
        if self.hit_tokens + self.miss_tokens == 0 {
            None
        } else {
            Some(self.hit_tokens as f64 / (self.hit_tokens + self.miss_tokens) as f64)
        }
    }
}

/// The model contract the brain depends on. New providers (Anthropic, GLM via
/// OpenRouter, local Ollama, ...) implement this trait; nothing else changes.
#[async_trait]
pub trait BrainProvider: Send + Sync {
    /// Normalize a raw content blob into structured knowledge.
    /// `static_prefix` must be byte-identical across calls for cache reuse.
    async fn distill(&self, static_prefix: &str, content: &str) -> Result<DistillOutcome>;
    /// Score `candidates` against `query` (one score per candidate, 0..=1).
    /// The pipeline treats an `Err` as "skip reranking" — RRF scores stand.
    async fn rerank(&self, query: &str, candidates: &[Evidence]) -> Result<Vec<f64>>;
    /// Produce a grounded, citation-bearing answer from query + evidence.
    async fn synthesize(
        &self,
        static_prefix: &str,
        query: &str,
        evidence: &[Evidence],
    ) -> Result<String>;
    /// Embed a single text. DeepSeek serves no embeddings API — this is always
    /// a configurable, pluggable model (qwen-embed / bge via LiteLLM).
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    /// The configured model roles (for logging and the dashboard).
    fn models(&self) -> &BrainModels;
    /// Snapshot of cache accounting.
    fn cache_stats(&self) -> CacheStats;
}

/// Default provider: any OpenAI-compatible endpoint — in practice LiteLLM
/// (`http://192.168.1.13:4000/v1`) fronting DeepSeek models on bonsai-cuda.
/// DeepSeek conditioning = static-prefix prompts + cache accounting; the
/// transport is plain OpenAI-compatible HTTP.
pub struct DeepSeekLiteLLM {
    http: OpenAIProvider,
    models: BrainModels,
    stats: Mutex<CacheStats>,
}

impl DeepSeekLiteLLM {
    pub fn new(base_url: String, api_key: String, models: BrainModels) -> Self {
        Self {
            http: OpenAIProvider::new(api_key, Some(base_url)),
            models,
            stats: Mutex::new(CacheStats::default()),
        }
    }

    async fn chat_text(&self, static_prefix: &str, dynamic_tail: &str) -> Result<String> {
        let req = ChatRequest {
            model: self.models.chat.clone(),
            messages: vec![
                serde_json::json!({ "role": "system", "content": static_prefix }),
                serde_json::json!({ "role": "user", "content": dynamic_tail }),
            ],
            stream: Some(false),
        };
        let resp: ChatResponse = self.http.chat(req).await?;
        self.stats
            .lock()
            .map_err(|_| anyhow::anyhow!("brain: cache stats mutex poisoned"))?
            .record(resp.usage.as_ref());
        let text = resp
            .choices
            .first()
            .and_then(|c| c.message.as_ref())
            .and_then(|m| m.get("content").and_then(serde_json::Value::as_str))
            .ok_or_else(|| anyhow::anyhow!("brain: chat response missing content"))?
            .to_string();
        Ok(text)
    }

    /// Parse a JSON object out of an LLM reply, tolerating code fences and
    /// prose around it (models love to narrate).
    fn extract_json(text: &str) -> Result<serde_json::Value> {
        let start = text
            .find('{')
            .ok_or_else(|| anyhow::anyhow!("no JSON object in reply"))?;
        let end = text
            .rfind('}')
            .ok_or_else(|| anyhow::anyhow!("no JSON object in reply"))?;
        serde_json::from_str(&text[start..=end]).context("brain: parse LLM JSON")
    }
}

#[async_trait]
impl BrainProvider for DeepSeekLiteLLM {
    async fn distill(&self, static_prefix: &str, content: &str) -> Result<DistillOutcome> {
        let text = self.chat_text(static_prefix, content).await?;
        let v = Self::extract_json(&text)?;
        // Lenient field extraction: a partial/malformed reply degrades to
        // defaults instead of failing the whole ingest batch.
        Ok(DistillOutcome {
            question: v.get("question").and_then(|s| s.as_str()).map(String::from),
            summary: v
                .get("summary")
                .and_then(|s| s.as_str())
                .unwrap_or_default()
                .to_string(),
            resolution: v
                .get("resolution")
                .and_then(|s| s.as_str())
                .map(String::from),
            systems: v
                .get("systems")
                .and_then(|s| s.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            title: v.get("title").and_then(|s| s.as_str()).map(String::from),
        })
    }

    async fn rerank(&self, query: &str, candidates: &[Evidence]) -> Result<Vec<f64>> {
        let payload = candidates
            .iter()
            .enumerate()
            .map(|(i, e)| {
                serde_json::json!({
                    "id": i,
                    "title": e.title,
                    "summary": e.summary,
                    "question": e.question,
                    "source": e.source,
                    "created_at": e.created_at,
                })
            })
            .collect::<Vec<_>>();
        let tail = serde_json::json!({
            "query": query,
            "candidates": payload,
        })
        .to_string();
        let text = self.chat_text(RERANK_SYSTEM_PREFIX, &tail).await?;
        let v = Self::extract_json(&text)?;
        let arr = v
            .get("scores")
            .and_then(|s| s.as_array())
            .ok_or_else(|| anyhow::anyhow!("rerank reply missing scores array"))?;
        let mut scores = arr
            .iter()
            .map(|s| s.as_f64().unwrap_or(0.0).clamp(0.0, 1.0))
            .collect::<Vec<f64>>();
        if scores.len() != candidates.len() {
            anyhow::bail!(
                "rerank returned {} scores for {} candidates",
                scores.len(),
                candidates.len()
            );
        }
        // Normalize so the pipeline can blend with RRF scores.
        let max = scores.iter().cloned().fold(0.0f64, f64::max);
        if max > 0.0 {
            for s in scores.iter_mut() {
                *s /= max;
            }
        }
        Ok(scores)
    }

    async fn synthesize(
        &self,
        static_prefix: &str,
        query: &str,
        evidence: &[Evidence],
    ) -> Result<String> {
        let tail = serde_json::json!({
            "query": query,
            "evidence": evidence
                .iter()
                .map(|e| serde_json::json!({
                    "source": e.source, "source_id": e.source_id,
                    "title": e.title, "author": e.author,
                    "summary": e.summary, "question": e.question,
                    "resolution": e.resolution, "systems": e.systems,
                    "created_at": e.created_at, "hash": e.content_hash,
                }))
                .collect::<Vec<_>>(),
        })
        .to_string();
        self.chat_text(static_prefix, &tail).await
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let mut embeds = self
            .http
            .embed(&self.models.embed, &[text.to_string()])
            .await?;
        embeds
            .pop()
            .map(|e| e.embedding)
            .ok_or_else(|| anyhow::anyhow!("brain: embeddings response empty"))
    }

    fn models(&self) -> &BrainModels {
        &self.models
    }

    fn cache_stats(&self) -> CacheStats {
        self.stats.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// Static prompt prefixes — THE cacheable assets.
//
// Rules (DeepSeek byte-stable prefix hash):
//   - Never edit, reorder, reformat, or add placeholders to these.
//   - Never append volatile content (timestamps, ids) to them.
//   - Change the schema/task text only when the task actually changes — every
//     edit busts the cache for every downstream call.
// ---------------------------------------------------------------------------

/// Distillation: normalize a raw thread/doc into searchable structured fields.
pub const DISTILL_SYSTEM_PREFIX: &str = r#"You are the indexing engine of a company knowledge base.
Your job: read an unstructured document or conversation and return STRICT JSON with exactly this schema:
{"question": string|null, "summary": string, "resolution": string|null, "systems": [string], "title": string|null}

Rules:
- "question": the one-line question an engineer would actually search for. null when the content is not a question/answer.
- "summary": 1-3 sentences capturing the durable, reusable knowledge. Never mention "the user said" or "in this thread".
- "resolution": the accepted answer or outcome, if one exists. null otherwise.
- "systems": code references, system names, config flags, hostnames, error-string fragments mentioned.
- "title": a short searchable title for the content.
- Output ONLY the JSON object. No prose, no markdown fences."#;

/// Reranking: score evidence candidates against the query.
pub const RERANK_SYSTEM_PREFIX: &str = r#"You are a retrieval reranker for a company knowledge base.
You receive a query and a list of candidates. Return STRICT JSON:
{"scores": [0.0-1.0, ...]} — one score per candidate, in order.

Rules:
- Score evidence that answers the query close to 1.0.
- Penalize candidates that merely share vocabulary but answer a different question.
- Penalize stale candidates (old created_at) when a fresher one covers the same ground.
- Prefer candidates with explicit resolutions over open questions.
- Output ONLY the JSON object."#;

/// Synthesis: ground an answer in evidence with citations and guardrails.
pub const SYNTHESIZE_SYSTEM_PREFIX: &str = r#"You are the answer engine of a company knowledge base.
Answer the user's question using ONLY the provided evidence. Every factual claim must cite its source.

Rules:
- Cite every claim as [source/source_id] inline.
- If the evidence is insufficient or contradictory, say so explicitly — never invent.
- If evidence is stale, flag it and prefer fresher rows.
- Never combine data from two unrelated contexts in one analysis.
- End with a "Sources:" list of hashes (each: source/source_id, title, hash) so the answer can be provenance-checked against the audit chain.
- Be concise and direct. Use bullets for multi-part answers."#;

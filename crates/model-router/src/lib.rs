use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Value>,
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,
    pub choices: Vec<Choice>,
    #[serde(default)]
    pub usage: Option<Usage>,
}

/// Token accounting from an OpenAI-compatible chat completion response.
///
/// DeepSeek (and LiteLLM pass-through) reports prompt-cache economics as
/// `prompt_cache_hit_tokens` / `prompt_cache_miss_tokens`; OpenAI reports the
/// hit count nested under `prompt_tokens_details.cached_tokens`. Both shapes
/// are parsed defensively so `cache_hit_ratio()` works across providers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub prompt_cache_hit_tokens: Option<u64>,
    pub prompt_cache_miss_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<PromptTokensDetails>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptTokensDetails {
    pub cached_tokens: Option<u64>,
}

impl Usage {
    /// Tokens served from the prompt cache (DeepSeek or OpenAI shape).
    pub fn cache_hit_tokens(&self) -> u64 {
        self.prompt_cache_hit_tokens
            .or_else(|| {
                self.prompt_tokens_details
                    .as_ref()
                    .and_then(|d| d.cached_tokens)
            })
            .unwrap_or(0)
    }

    /// Tokens that had to be recomputed (cache miss).
    pub fn cache_miss_tokens(&self) -> u64 {
        self.prompt_cache_miss_tokens.unwrap_or(0)
    }

    /// Fraction of prompt tokens served from cache. Only meaningful when the
    /// provider reports BOTH hit and miss counters (the DeepSeek/LiteLLM
    /// shape). OpenAI's `cached_tokens` alone carries no miss count, so the
    /// ratio is `None` there — the raw hit count remains available via
    /// [`Usage::cache_hit_tokens`].
    pub fn cache_hit_ratio(&self) -> Option<f64> {
        let hit = self.prompt_cache_hit_tokens?;
        let miss = self.prompt_cache_miss_tokens?;
        if hit + miss == 0 {
            None
        } else {
            Some(hit as f64 / (hit + miss) as f64)
        }
    }
}

/// A single embedding vector returned by an OpenAI-compatible `/embeddings` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub index: usize,
    pub embedding: Vec<f32>,
}

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, model: &str, inputs: &[String]) -> Result<Vec<Embedding>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: usize,
    pub message: Option<Value>,
    pub delta: Option<Value>,
    pub finish_reason: Option<String>,
}

#[async_trait]
pub trait ModelProvider: Send + Sync {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse>;
    async fn models(&self) -> Result<Vec<String>>;
}

pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl OpenAIProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".into()),
        }
    }
}

#[async_trait]
impl ModelProvider for OpenAIProvider {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
        info!("Sending chat request to OpenAI-compatible endpoint");
        let url = format!("{}/chat/completions", self.base_url);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&req)
            .send()
            .await?
            .json::<ChatResponse>()
            .await?;
        Ok(resp)
    }

    async fn models(&self) -> Result<Vec<String>> {
        let url = format!("{}/models", self.base_url);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await?
            .json::<Value>()
            .await?;
        let models = resp["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["id"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        Ok(models)
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAIProvider {
    async fn embed(&self, model: &str, inputs: &[String]) -> Result<Vec<Embedding>> {
        let url = format!("{}/embeddings", self.base_url);
        let body = serde_json::json!({ "model": model, "input": inputs });
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("embeddings request failed: HTTP {status}: {text}");
        }
        let value: Value = serde_json::from_str(&text)?;
        let data = value
            .get("data")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("embeddings response missing `data` array"))?;
        data.iter()
            .map(|item| {
                let index = item.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
                let embedding = item
                    .get("embedding")
                    .and_then(Value::as_array)
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_f64().map(|f| f as f32))
                            .collect()
                    })
                    .ok_or_else(|| anyhow::anyhow!("embedding item missing `embedding` array"))?;
                Ok(Embedding { index, embedding })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_hit_ratio_deepseek_shape() {
        let usage = Usage {
            prompt_cache_hit_tokens: Some(300),
            prompt_cache_miss_tokens: Some(100),
            ..Default::default()
        };
        assert_eq!(usage.cache_hit_tokens(), 300);
        assert_eq!(usage.cache_hit_ratio(), Some(0.75));
    }

    #[test]
    fn cache_hit_ratio_openai_shape() {
        let usage = Usage {
            prompt_tokens_details: Some(PromptTokensDetails {
                cached_tokens: Some(250),
            }),
            ..Default::default()
        };
        assert_eq!(usage.cache_hit_tokens(), 250);
        assert_eq!(usage.cache_hit_ratio(), None); // no miss counter reported
    }

    #[test]
    fn cache_hit_ratio_none_when_no_accounting() {
        let usage = Usage::default();
        assert_eq!(usage.cache_hit_ratio(), None);
    }
}

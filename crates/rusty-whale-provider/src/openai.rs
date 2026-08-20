//! rusty-whale OpenAI provider — real HTTP skeleton via reqwest.
//! Calls `POST {endpoint}/chat/completions` and parses the response into
//! `CompletionResponse`. The skeleton wires up the request shape; real
//! streaming / tool-calling are stubbed behind `supports_*`.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    ChatMessage, CompletionRequest, CompletionResponse, LlmProvider, Usage,
};

/// Configuration for the OpenAI-compatible provider.
#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    /// API key, e.g. `sk-...`. Sent as `Authorization: Bearer <key>`.
    pub api_key: String,
    /// Base URL. Default OpenAI: `https://api.openai.com/v1`.
    /// Point this at LM Studio / vLLM / OpenRouter for drop-in use.
    pub endpoint: String,
    /// Optional request timeout. Defaults to 60s if unset.
    pub timeout_ms: Option<u64>,
}

impl OpenAiConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            endpoint: "https://api.openai.com/v1".into(),
            timeout_ms: Some(60_000),
        }
    }
    pub fn endpoint(mut self, e: impl Into<String>) -> Self {
        self.endpoint = e.into();
        self
    }
}

pub struct OpenAiProvider {
    cfg: OpenAiConfig,
    client: Client,
}

impl OpenAiProvider {
    pub fn new(cfg: OpenAiConfig) -> Result<Self> {
        let mut b = Client::builder().user_agent("rusty-whale/0.1");
        if let Some(ms) = cfg.timeout_ms {
            b = b.timeout(std::time::Duration::from_millis(ms));
        }
        Ok(Self { cfg, client: b.build()? })
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionsReq<'a> {
    model: &'a str,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionsResp {
    choices: Vec<ChatChoice>,
    #[serde(default)]
    usage: Option<ApiUsage>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    #[serde(default)]
    message: Option<ApiMessage>,
    #[serde(default, rename = "finish_reason")]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiMessage {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiUsage {
    #[serde(default, rename = "prompt_tokens")]
    prompt_tokens: u32,
    #[serde(default, rename = "completion_tokens")]
    completion_tokens: u32,
    #[serde(default, rename = "total_tokens")]
    total_tokens: u32,
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    fn name(&self) -> &str { "openai" }
    fn supports_tools(&self) -> bool { true }
    fn supports_streaming(&self) -> bool { true }

    async fn complete(&self, req: &CompletionRequest) -> Result<CompletionResponse> {
        if self.cfg.api_key.is_empty() {
            return Err(anyhow!("openai: api_key is empty"));
        }
        let body = ChatCompletionsReq {
            model: &req.model,
            messages: req.messages.clone(),
            max_tokens: req.max_tokens,
            temperature: req.temperature,
            stream: false,
        };
        let url = format!("{}/chat/completions", self.cfg.endpoint.trim_end_matches('/'));
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.cfg.api_key)
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let txt = resp.text().await.unwrap_or_default();
            return Err(anyhow!("openai: HTTP {status} — {txt}"));
        }
        let parsed: ChatCompletionsResp = resp.json().await?;
        let choice = parsed
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("openai: no choices in response"))?;
        let text = choice
            .message
            .and_then(|m| m.content)
            .unwrap_or_default();
        let usage = parsed.usage.map(|u| Usage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });
        Ok(CompletionResponse {
            text,
            finish_reason: choice.finish_reason,
            usage,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_builder() {
        let c = OpenAiConfig::new("sk-test").endpoint("https://proxy/v1");
        assert_eq!(c.api_key, "sk-test");
        assert_eq!(c.endpoint, "https://proxy/v1");
    }

    #[test]
    fn empty_key_errors() {
        // Construct a provider with an empty key; complete() should error.
        // We don't actually send a request — the error happens before .send().
        let cfg = OpenAiConfig::new("");
        let p = OpenAiProvider::new(cfg).unwrap();
        let r = futures::executor::block_on(p.complete(&CompletionRequest {
            model: "gpt-4o-mini".into(),
            messages: vec![],
            max_tokens: None,
            temperature: None,
        }));
        assert!(r.is_err());
    }

    #[test]
    fn request_body_serializes() {
        // Sanity-check the request shape is what OpenAI expects.
        let body = ChatCompletionsReq {
            model: "gpt-4o-mini",
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "hi".into(),
            }],
            max_tokens: Some(64),
            temperature: Some(0.0),
            stream: false,
        };
        let s = serde_json::to_string(&body).unwrap();
        assert!(s.contains("\"model\":\"gpt-4o-mini\""));
        assert!(s.contains("\"stream\":false"));
    }
}

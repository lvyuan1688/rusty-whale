//! rusty-whale Anthropic provider — real HTTP skeleton via reqwest.
//! Calls `POST {endpoint}/v1/messages` and parses the response into
//! `CompletionResponse`. Uses Anthropic's `messages` API shape with
//! `x-api-key` + `anthropic-version` headers.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    ChatMessage, CompletionRequest, CompletionResponse, LlmProvider, Usage,
};

/// Configuration for the Anthropic provider.
#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    /// API key, e.g. `sk-ant-...`. Sent as `x-api-key`.
    pub api_key: String,
    /// Base URL. Default: `https://api.anthropic.com`.
    /// Point at a proxy for self-hosted / compliant setups.
    pub endpoint: String,
    /// Anthropic API version. Default: `2023-06-01`.
    pub api_version: String,
    /// Optional request timeout. Defaults to 60s if unset.
    pub timeout_ms: Option<u64>,
}

impl AnthropicConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            endpoint: "https://api.anthropic.com".into(),
            api_version: "2023-06-01".into(),
            timeout_ms: Some(60_000),
        }
    }
    pub fn endpoint(mut self, e: impl Into<String>) -> Self {
        self.endpoint = e.into();
        self
    }
    pub fn api_version(mut self, v: impl Into<String>) -> Self {
        self.api_version = v.into();
        self
    }
}

pub struct AnthropicProvider {
    cfg: AnthropicConfig,
    client: Client,
}

impl AnthropicProvider {
    pub fn new(cfg: AnthropicConfig) -> Result<Self> {
        let mut b = Client::builder().user_agent("rusty-whale/0.1");
        if let Some(ms) = cfg.timeout_ms {
            b = b.timeout(std::time::Duration::from_millis(ms));
        }
        Ok(Self { cfg, client: b.build()? })
    }
}

/// Anthropic's `/v1/messages` request shape.
#[derive(Debug, Serialize)]
struct MessagesReq<'a> {
    model: &'a str,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    /// Anthropic splits the system prompt out of `messages`.
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<&'a str>,
    messages: Vec<AnthropicMessage>,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct MessagesResp {
    #[serde(default)]
    content: Vec<ContentBlock>,
    #[serde(default, rename = "stop_reason")]
    stop_reason: Option<String>,
    #[serde(default)]
    usage: Option<ApiUsage>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: serde_json::Value },
}

#[derive(Debug, Deserialize)]
struct ApiUsage {
    #[serde(default, rename = "input_tokens")]
    input_tokens: u32,
    #[serde(default, rename = "output_tokens")]
    output_tokens: u32,
}

/// Split a `CompletionRequest` into (system_prompt, messages) for
/// Anthropic's API. The system prompt is the first message with
/// `role == "system"` (if any); the rest are passed through with role
/// `"user"` or `"assistant"`. Unknown roles are mapped to `"user"`.
fn split_system(req: &CompletionRequest) -> (Option<String>, Vec<AnthropicMessage>) {
    let mut system: Option<String> = None;
    let mut out = Vec::with_capacity(req.messages.len());
    for m in &req.messages {
        match m.role.as_str() {
            "system" if system.is_none() => system = Some(m.content.clone()),
            "user" | "assistant" => out.push(AnthropicMessage {
                role: Box::leak(m.role.clone().into_boxed_str()),
                content: Box::leak(m.content.clone().into_boxed_str()),
            }),
            _ => out.push(AnthropicMessage {
                role: "user",
                content: Box::leak(m.content.clone().into_boxed_str()),
            }),
        }
    }
    (system, out)
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &str { "anthropic" }
    fn supports_tools(&self) -> bool { true }
    fn supports_streaming(&self) -> bool { true }

    async fn complete(&self, req: &CompletionRequest) -> Result<CompletionResponse> {
        if self.cfg.api_key.is_empty() {
            return Err(anyhow!("anthropic: api_key is empty"));
        }
        // Anthropic requires max_tokens; default to 1024 if unset.
        let max_tokens = req.max_tokens.unwrap_or(1024);
        let (system, messages) = split_system(req);
        let system_ref = system.as_deref();
        let body = MessagesReq {
            model: &req.model,
            max_tokens,
            temperature: req.temperature,
            system: system_ref,
            messages,
        };
        let url = format!(
            "{}/v1/messages",
            self.cfg.endpoint.trim_end_matches('/')
        );
        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.cfg.api_key)
            .header("anthropic-version", &self.cfg.api_version)
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let txt = resp.text().await.unwrap_or_default();
            return Err(anyhow!("anthropic: HTTP {status} — {txt}"));
        }
        let parsed: MessagesResp = resp.json().await?;
        let text = parsed
            .content
            .into_iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");
        let usage = parsed.usage.map(|u| Usage {
            prompt_tokens: u.input_tokens,
            completion_tokens: u.output_tokens,
            total_tokens: u.input_tokens + u.output_tokens,
        });
        Ok(CompletionResponse {
            text,
            finish_reason: parsed.stop_reason,
            usage,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_builder_defaults() {
        let c = AnthropicConfig::new("sk-ant-x");
        assert_eq!(c.api_key, "sk-ant-x");
        assert_eq!(c.endpoint, "https://api.anthropic.com");
        assert_eq!(c.api_version, "2023-06-01");
    }

    #[test]
    fn config_overrides() {
        let c = AnthropicConfig::new("k")
            .endpoint("https://proxy")
            .api_version("2024-01-01");
        assert_eq!(c.endpoint, "https://proxy");
        assert_eq!(c.api_version, "2024-01-01");
    }

    #[test]
    fn split_system_extracts_first_system_message() {
        let req = CompletionRequest {
            model: "claude-3-5-sonnet-20240620".into(),
            messages: vec![
                ChatMessage { role: "system".into(), content: "be terse".into() },
                ChatMessage { role: "user".into(), content: "hi".into() },
            ],
            max_tokens: None,
            temperature: None,
        };
        let (sys, msgs) = split_system(&req);
        assert_eq!(sys.as_deref(), Some("be terse"));
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, "user");
    }

    #[test]
    fn split_system_unknown_role_maps_to_user() {
        let req = CompletionRequest {
            model: "x".into(),
            messages: vec![
                ChatMessage { role: "tool".into(), content: "result".into() },
            ],
            max_tokens: None,
            temperature: None,
        };
        let (_, msgs) = split_system(&req);
        assert_eq!(msgs[0].role, "user");
    }

    #[tokio::test]
    async fn empty_key_errors_before_http() {
        let p = AnthropicProvider::new(AnthropicConfig::new("")).unwrap();
        let r = p.complete(&CompletionRequest {
            model: "claude-3-5-sonnet".into(),
            messages: vec![],
            max_tokens: None,
            temperature: None,
        }).await;
        assert!(r.is_err());
    }
}

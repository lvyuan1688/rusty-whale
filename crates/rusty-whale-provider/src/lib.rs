//! rusty-whale-provider: LlmProvider trait + 5 implementations.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// A completion request sent to a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

/// A single chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// The completion response returned by a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub text: String,
    pub finish_reason: Option<String>,
    pub usage: Option<Usage>,
}

/// Token usage accounting.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// The core provider trait. Implementations call out to a specific LLM API.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, req: &CompletionRequest) -> Result<CompletionResponse>;
    fn name(&self) -> &str;
    fn supports_tools(&self) -> bool {
        false
    }
    fn supports_streaming(&self) -> bool {
        false
    }
}

/// OpenAI-compatible provider (OpenAI, vLLM, any OpenAI-style endpoint).
pub struct OpenAiProvider {
    pub api_key: String,
    pub endpoint: String,
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn complete(&self, _req: &CompletionRequest) -> Result<CompletionResponse> {
        // Skeleton: real impl POSTs to {endpoint}/v1/chat/completions.
        Ok(CompletionResponse {
            text: String::new(),
            finish_reason: Some("stop".into()),
            usage: None,
        })
    }
    fn name(&self) -> &str {
        "openai"
    }
    fn supports_tools(&self) -> bool {
        true
    }
    fn supports_streaming(&self) -> bool {
        true
    }
}

/// Anthropic Claude provider.
pub struct AnthropicProvider {
    pub api_key: String,
    pub endpoint: String,
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn complete(&self, _req: &CompletionRequest) -> Result<CompletionResponse> {
        Ok(CompletionResponse {
            text: String::new(),
            finish_reason: Some("end_turn".into()),
            usage: None,
        })
    }
    fn name(&self) -> &str {
        "anthropic"
    }
    fn supports_tools(&self) -> bool {
        true
    }
}

/// Google Gemini provider.
pub struct GeminiProvider {
    pub api_key: String,
    pub endpoint: String,
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn complete(&self, _req: &CompletionRequest) -> Result<CompletionResponse> {
        Ok(CompletionResponse {
            text: String::new(),
            finish_reason: Some("STOP".into()),
            usage: None,
        })
    }
    fn name(&self) -> &str {
        "gemini"
    }
}

/// Ollama local LLM provider.
pub struct OllamaProvider {
    pub endpoint: String,
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn complete(&self, _req: &CompletionRequest) -> Result<CompletionResponse> {
        Ok(CompletionResponse {
            text: String::new(),
            finish_reason: Some("stop".into()),
            usage: None,
        })
    }
    fn name(&self) -> &str {
        "ollama"
    }
}

/// vLLM self-hosted provider.
pub struct VllmProvider {
    pub endpoint: String,
}

#[async_trait]
impl LlmProvider for VllmProvider {
    async fn complete(&self, _req: &CompletionRequest) -> Result<CompletionResponse> {
        Ok(CompletionResponse {
            text: String::new(),
            finish_reason: Some("stop".into()),
            usage: None,
        })
    }
    fn name(&self) -> &str {
        "vllm"
    }
    fn supports_streaming(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ollama_returns_skeleton_response() {
        let p = OllamaProvider {
            endpoint: "http://localhost:11434".into(),
        };
        let req = CompletionRequest {
            model: "qwen2.5-coder:7b".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "hi".into(),
            }],
            max_tokens: None,
            temperature: None,
        };
        let r = p.complete(&req).await.unwrap();
        assert_eq!(p.name(), "ollama");
        assert!(r.text.is_empty());
    }
}

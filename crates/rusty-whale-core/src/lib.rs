//! rusty-whale-core — agent loop, state machine, tool dispatch
//!
//! Core trait definitions and the agent state machine that drives
//! the Think → Act → Verify loop.

pub mod state;
pub mod tool_registry;
pub mod verify;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single completion request to an LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolDef>,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
    pub stream: bool,
}

/// A chat message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
}

/// Message role.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// A response from the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub finish_reason: FinishReason,
    pub usage: TokenUsage,
}

/// Why the model stopped generating.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    ToolCalls,
    Length,
    ContentFilter,
}

/// Token usage for billing/telemetry.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

/// A tool definition exposed to the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// A tool call requested by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: HashMap<String, serde_json::Value>,
}

/// The LLM provider trait. Implement this to add a new provider.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Complete a request, returning the model's response.
    async fn complete(&self, req: &CompletionRequest) -> Result<CompletionResponse, ProviderError>;

    /// Provider name (e.g. "openai", "anthropic").
    fn name(&self) -> &str;

    /// Whether this provider supports tool calling.
    fn supports_tools(&self) -> bool;

    /// Whether this provider supports streaming responses.
    fn supports_streaming(&self) -> bool;

    /// Maximum context tokens this provider/model accepts.
    fn max_context_tokens(&self) -> usize {
        128_000 // sensible default
    }
}

/// An error returned by a provider.
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("authentication failed: {0}")]
    Auth(String),
    #[error("rate limited, retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },
    #[error("model error: {0}")]
    Model(String),
    #[error("unexpected response: {0}")]
    Unexpected(String),
}

/// A tool that the agent can execute.
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;
    async fn execute(&self, args: &HashMap<String, serde_json::Value>) -> Result<String, ToolError>;
}

/// Tool execution error.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("execution failed: {0}")]
    Execution(String),
    #[error("permission denied: {0}")]
    Permission(String),
}

/// The result of running an agent loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub summary: String,
    pub turns: usize,
    pub total_tokens: usize,
    pub tool_calls: Vec<ToolCall>,
    pub verified: bool,
}

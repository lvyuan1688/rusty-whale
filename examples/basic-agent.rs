//! Minimal rusty-whale agent: build a stub provider, send a prompt, print
//! the response. Run with `cargo run --example basic-agent`.

use rusty_whale_core::{run_loop, AgentState};
use rusty_whale_provider::{
    ChatMessage, CompletionRequest, CompletionResponse, LlmProvider,
};

struct EchoProvider;

#[async_trait::async_trait]
impl LlmProvider for EchoProvider {
    async fn complete(&self, req: &CompletionRequest) -> anyhow::Result<CompletionResponse> {
        let last = req.messages.last().cloned().unwrap_or(ChatMessage {
            role: "user".into(),
            content: "".into(),
        });
        Ok(CompletionResponse {
            text: format!("[echo] {}", last.content),
            finish_reason: Some("stop".into()),
            usage: None,
        })
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = EchoProvider;
    let req = CompletionRequest {
        model: "echo-1".into(),
        messages: vec![ChatMessage {
            role: "user".into(),
            content: "Hello, rusty-whale!".into(),
        }],
        max_tokens: Some(64),
        temperature: Some(0.0),
    };
    let resp = provider.complete(&req).await?;
    println!("{}", resp.text);

    let result = run_loop(|s: AgentState| async move {
        let _ = s;
        (AgentState::Done, vec![])
    })
    .await?;
    println!("loop finished in {} iteration(s)", result.iterations);
    Ok(())
}

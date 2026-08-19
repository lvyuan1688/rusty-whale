//! rusty-whale CLI entry point.
//!
//! Wires the provider, verify, and core crates together into a minimal
//! interactive REPL. The skeleton parses CLI flags, builds a default
//! provider (a no-op stub), and runs the agent loop until Done.

use anyhow::Result;
use clap::{Parser, Subcommand};
use rusty_whale_core::{run_loop, AgentState};
use rusty_whale_provider::{ChatMessage, CompletionRequest, LlmProvider};
use rusty_whale_verify::{FailAction, VerifyConfig};

/// Top-level CLI for rusty-whale.
#[derive(Debug, Parser)]
#[command(name = "rusty-whale", version, about = "Open-source agent harness")]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Command>,
}

/// Subcommands supported by the skeleton.
#[derive(Debug, Subcommand)]
enum Command {
    /// Run the agent loop with a prompt.
    Run {
        #[arg(long, default_value = "stub")]
        provider: String,
        #[arg(long, default_value = "gpt-4o-mini")]
        model: String,
        prompt: String,
    },
    /// Print the resolved configuration and exit.
    Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd.unwrap_or(Command::Config) {
        Command::Run {
            provider,
            model,
            prompt,
        } => {
            let p = build_provider(&provider)?;
            let req = CompletionRequest {
                model,
                messages: vec![ChatMessage {
                    role: "user".into(),
                    content: prompt,
                }],
                max_tokens: Some(1024),
                temperature: Some(0.2),
            };
            let resp = p.complete(&req).await?;
            println!("{}", resp.text);
            let _ = run_loop(|s: AgentState| async move {
                let _ = s;
                (AgentState::Done, vec![])
            })
            .await?;
            Ok(())
        }
        Command::Config => {
            let cfg = VerifyConfig {
                command: "cargo build --release".into(),
                fail_action: FailAction::Retry,
                max_retries: 3,
            };
            println!("verify.command = {}", cfg.command);
            println!("verify.fail_action = {:?}", cfg.fail_action);
            println!("verify.max_retries = {}", cfg.max_retries);
            Ok(())
        }
    }
}

/// Build a provider by name. The skeleton only ships a no-op stub.
fn build_provider(name: &str) -> Result<Box<dyn LlmProvider>> {
    tracing::info!("building provider: {name}");
    Ok(Box::new(StubProvider))
}

/// A no-op provider used by the skeleton CLI.
struct StubProvider;

#[async_trait::async_trait]
impl LlmProvider for StubProvider {
    async fn complete(
        &self,
        _req: &CompletionRequest,
    ) -> Result<rusty_whale_provider::CompletionResponse> {
        Ok(rusty_whale_provider::CompletionResponse {
            text: "[rusty-whale stub] wire a real provider in src/main.rs".into(),
            finish_reason: Some("stop".into()),
            usage: None,
        })
    }
}

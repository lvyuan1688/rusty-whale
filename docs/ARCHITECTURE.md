# rusty-whale Architecture

> v0.1.5 — covers the workspace split, the agent state machine, and the
> real OpenAI provider skeleton.

## Workspace layout

```
rusty-whale/
  Cargo.toml            # workspace root, also a binary crate
  src/main.rs           # CLI entry (clap)
  examples/             # basic-agent.rs, custom-verify.rs
  crates/
    rusty-whale-core/      # state machine + run_loop
    rusty-whale-provider/  # LlmProvider trait + openai/ollama/vllm
    rusty-whale-verify/    # ShellVerifier + VerifyConfig
    rusty-whale-tui/       # ratatui rendering
  docs/
    ARCHITECTURE.md
    demo.png
```

## The agent state machine

`rusty_whale_core::AgentState` models one iteration of the agent loop:

```
Idle ─▶ Thinking ─▶ Acting ─▶ Verifying ─▶ Done|Waiting
                                   │
                                   └─▶ (fail) Acting
```

`run_loop(step)` is the public entry. `step` is a closure that takes the
current state and returns the next state plus any tool calls produced. This
keeps the core loop agnostic to the concrete LLM provider and verify
strategy — you can plug in a stub, a real OpenAI client, or a replay.

## Provider layer

`rusty_whale_provider::LlmProvider` is the trait:

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, req: &CompletionRequest) -> Result<CompletionResponse>;
    fn name(&self) -> &str;
    fn supports_tools(&self) -> bool;
    fn supports_streaming(&self) -> bool;
}
```

Implementations:

| Module | Backend | Status |
|--------|---------|--------|
| `openai.rs` | OpenAI-compatible HTTP (reqwest) | **v0.1.5 skeleton** |
| `ollama.rs` | Ollama HTTP | stub |
| `vllm.rs` | vLLM HTTP | stub |

The OpenAI provider wires up the real request shape:

```rust
POST {endpoint}/chat/completions
Authorization: Bearer <key>
Content-Type: application/json

{
  "model": "...",
  "messages": [...],
  "max_tokens": ...,
  "temperature": ...,
  "stream": false
}
```

Streaming and tool-calling are gated behind `supports_*` and return an
`anyhow` error when called on a backend that doesn't support them.

## Verify loop

`rusty_whale_verify::ShellVerifier` shells out to the configured command
(`cargo build --release` by default) and treats a zero exit status as
success. `VerifyConfig::fail_action` decides what happens on failure:

- `Retry` — re-run up to `max_retries` times
- `Abort` — surface the error to the user
- `Ask` — prompt the user (TUI only)

## TUI

`rusty_whale_tui::run` opens a ratatui terminal and renders the current
`AgentState`. `q` quits. The skeleton doesn't render message history or
tool-call output yet — see `docs/ROADMAP.md`.

## Extension points

- **New provider**: implement `LlmProvider` in
  `crates/rusty-whale-provider/src/<name>.rs`. Wire it into the CLI in
  `src/main.rs::build_provider`.
- **New verifier**: implement `Verifier` in
  `crates/rusty-whale-verify/src/<name>.rs`.
- **New tool**: add a variant to `rusty_whale_core::ToolCall` and dispatch
  it inside your `step` closure.

# Providers (v0.1.6)

rusty-whale ships two real HTTP provider skeletons:

| Provider | Endpoint | Auth | Status |
|----------|----------|------|--------|
| `OpenAiProvider` | `POST {endpoint}/chat/completions` | `Authorization: Bearer <key>` | ✅ v0.1.5 |
| `AnthropicProvider` | `POST {endpoint}/v1/messages` | `x-api-key: <key>` + `anthropic-version` | ✅ v0.1.6 |

Both are OpenAI/Anthropic-compatible, so pointing `endpoint` at OpenRouter,
vLLM, LM Studio, or a proxy "just works".

## Anthropic-specific shape

Anthropic's `messages` API differs from OpenAI's in three ways:

1. **`system` is separate.** OpenAI puts it inside `messages` with
   `role:"system"`. Anthropic takes a top-level `system` string.
   `split_system()` extracts the first system-role message and passes
   the rest through.

2. **`max_tokens` is required.** OpenAI defaults it; Anthropic doesn't.
   We default to 1024 if the caller leaves it `None`.

3. **Response is `content: [{type:"text",text:"..."}]`**, not a flat
   `choices[0].message.content` string. We filter to `text` blocks and
   join.

## Usage

```rust
use rusty_whale_provider::{AnthropicProvider, AnthropicConfig};

let cfg = AnthropicConfig::new("sk-ant-...")
    .endpoint("https://api.anthropic.com")
    .api_version("2023-06-01");
let p = AnthropicProvider::new(cfg)?;
let resp = p.complete(&CompletionRequest {
    model: "claude-3-5-sonnet-20240620".into(),
    messages: vec![ChatMessage { role: "user".into(), content: "hi".into() }],
    max_tokens: Some(64),
    temperature: Some(0.0),
}).await?;
```

## Error handling

- Empty `api_key` → `anyhow!("anthropic: api_key is empty")` before HTTP.
- Non-2xx → `anyhow!("anthropic: HTTP {status} — {body}")` with the response body.
- Malformed JSON → `reqwest` returns the serde error.

## Not in v0.1.6

- Streaming (`stream:true` + SSE parsing) — see `open-agent-cli`'s
  `streaming.ts` for the pattern.
- Tool use (`tools: [{name,...}]` + `tool_use` content blocks) — Anthropic's
  tool protocol differs from OpenAI's; we'll add it in v0.2.

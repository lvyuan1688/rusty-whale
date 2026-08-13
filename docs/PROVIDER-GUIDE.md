# Provider Guide

## Supported Providers

| Provider | env var | default model |
|---|---|---|
| OpenAI | `OPENAI_API_KEY` | gpt-5-coder |
| Anthropic | `ANTHROPIC_API_KEY` | claude-sonnet-4-5 |
| Gemini | `GEMINI_API_KEY` | gemini-2.5-pro |
| Ollama | (none, local) | qwen2.5-coder:7b |
| vLLM | `VLLM_BASE_URL` | custom |

## Role-based config

```toml
[roles.codegen]
provider = "openai"
model = "gpt-5-coder"
reasoning_tier = "high"
```

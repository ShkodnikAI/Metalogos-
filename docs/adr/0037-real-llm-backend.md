# ADR-0037: Real LLM Backend

**Status:** Accepted
**Date:** 2026-06-03
**Phase:** 7.1

## Context

The `learnable pattern` construct and VM/interpreter LLM calls used a `MockLlm` that returned the prompt string as-is. The `RealLlm` stub used `curl` via `std::process::Command` which was fragile (no timeout, no retry, no proper error handling) and only supported a single generic endpoint format. To make METALOGOS a genuinely AI-native language, real LLM provider integration was required.

## Decision

Replace the curl-based `RealLlm` with a proper HTTP client (`reqwest` with `rustls-tls`) supporting three providers:

1. **Anthropic (Claude)** — `POST https://api.anthropic.com/v1/messages`, header `x-api-key`, default model `claude-sonnet-4-20250514`
2. **OpenAI** — `POST https://api.openai.com/v1/chat/completions`, header `Authorization: Bearer`, default model `gpt-4o`
3. **Ollama (local)** — `POST http://localhost:11434/api/generate`, no API key, default model `llama3`

### Configuration via environment variables

| Variable | Default | Purpose |
|---|---|---|
| `METALOGOS_LLM_PROVIDER` | `anthropic` | Select provider: `anthropic`, `openai`, `ollama` |
| `METALOGOS_LLM_MODEL` | Provider default | Override model name |
| `METALOGOS_API_KEY` | _(none)_ | API key for Anthropic/OpenAI |
| `METALOGOS_MOCK_LLM` | `true` | Set to `false` for real LLM calls |

### Retry and resilience

- **3 retries** with exponential backoff: 1s, 2s, 4s delays
- **30-second timeout** per attempt, 10-second connect timeout
- **No retry on fatal client errors** (400/401/403/404)
- **Retry on rate limit** (429) and server errors (5xx)

### JSON response auto-parsing

If the LLM response is a JSON object, the interpreter/VM automatically converts it to `Value::Struct` for structured field access (e.g., `result.category`).

### MockLlm preserved for tests

`MockLlm` remains unchanged. Default behavior is mock (`METALOGOS_MOCK_LLM=true`) — no accidental API calls in tests or CI. Three integration tests with real API keys are marked `#[ignore]`.

## Consequences

- `learnable pattern` works with real AI models when `METALOGOS_MOCK_LLM=false`
- No OpenSSL dependency — uses `rustls-tls`
- `reqwest::blocking` keeps `LlmBackend` trait synchronous
- Golden tests continue passing unchanged
- API keys via environment only — never in source code

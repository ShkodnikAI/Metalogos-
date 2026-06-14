# ADR-0048-Smart-LLM-Routing — Наряд №4

## Status
Implemented

## Summary

Smart LLM routing with automatic failover, circuit breaker, per-pattern model override,
token usage tracking, and multi-provider support. Replaces the single-provider
`create_llm_backend()` with a configurable `SmartRouter` that manages multiple
LLM providers with health tracking and automatic recovery.

## Motivation

Before this change, Metalogos used a single LLM provider per runtime, selected
via `METALOGOS_LLM_PROVIDER` env var. If the provider was unavailable, the entire
learnable pattern call failed. This is problematic for:

1. **Cost optimization**: Different patterns need different models (fast/cheap vs. strong/expensive).
2. **Reliability**: Provider outages should not block the application.
3. **Free-tier development**: Multiple free providers (Groq, Cerebras, NVIDIA) exist but
   were not accessible.
4. **Observability**: No usage tracking for token budgeting and latency monitoring.

## Decision

### Part A: `llm {}` Configuration Block

A new top-level declaration configures LLM routing in `.mlog` files:

```mlog
llm {
  providers: [
    { alias: "primary", provider: "anthropic", key: env("ANTHROPIC_KEY") },
    { alias: "fast", provider: "groq", key: env("GROQ_KEY") },
    { alias: "fallback", provider: "openai", key: env("OPENAI_KEY") },
    { alias: "local", provider: "ollama", url: "http://localhost:11434" }
  ]
  default_model: env("METALOGOS_LLM_MODEL")
  failover: auto
  circuit_breaker: 3
  timeout: 30
}
```

If absent, behavior is backward-compatible (env vars, single provider).

### Part B: Supported Providers

| Provider | Endpoint | Format | Free Tier |
|----------|----------|--------|-----------|
| anthropic | api.anthropic.com/v1/messages | Anthropic native | No |
| openai | api.openai.com/v1/chat/completions | OpenAI native | No |
| ollama | localhost:11434/api/generate | Ollama native | Yes (local) |
| groq | api.groq.com/openai/v1/chat/completions | OpenAI-compatible | Yes |
| cerebras | api.cerebras.ai/v1/chat/completions | OpenAI-compatible | Yes |
| nvidia | integrate.api.nvidia.com/v1/chat/completions | OpenAI-compatible | Yes |
| openrouter | openrouter.ai/api/v1/chat/completions | OpenAI-compatible | Aggregator |
| google | generativelanguage.googleapis.com/v1beta | OpenAI-compatible | Yes |
| custom | Any URL with OpenAI-compatible API | OpenAI-compatible | User-defined |

All OpenAI-compatible providers share a single HTTP client implementation.
Only `anthropic` (native format) and `ollama` (native format) have separate handlers.

### Part C: Smart Failover + Circuit Breaker

1. Providers are tried in priority order (list order in `providers`).
2. Circuit breaker: after N consecutive failures (configurable, default 3), the provider
   is skipped for 60 seconds (half-open recovery).
3. Health tracking: last 20 calls per provider. `health_score = success_count / total_count`.
4. On each call, providers are sorted by health_score (best first).
5. Failover modes:
   - `auto` (default): automatically try next provider on failure.
   - `manual` / absent: fail on first provider error (no failover).
6. All providers exhausted → soft failure with empty response + low confidence.

Error classification:
- 4xx (except 429): Fatal client error, no retry on this provider.
- 429: Rate limited, try next provider.
- 5xx / timeout: Transient, try next provider.

### Part D: Per-Pattern Model Override

```mlog
learnable pattern QuickClassify(text: String) -> String {
  prompt: "Classify: question | complaint"
  model: "fast"
}
```

Model resolution order:
1. `METALOGOS_LLM_MODEL_{alias}` env var → use its value
2. Provider alias match → use that provider's model
3. Direct model name → pass as-is

### Part E: Token Usage Tracking

Every LLM call records: provider, prompt_chars (estimated as chars/4 for tokens),
response_tokens, latency_ms, success/failure.

New builtin:
```
llm_usage() -> Struct {
  total_calls: Float,
  total_tokens: Float,
  total_errors: Float,
  providers: List<Struct {
    alias: String,
    calls: Float,
    tokens: Float,
    errors: Float,
    avg_latency_ms: Float,
    health_score: Float
  }>
}
```

## Implementation

### Files Changed
- `src/grammar.pest`: Added `llm_decl` rule with `llm_body`, `llm_providers`,
  `llm_provider_entry`, `llm_default_model`, `llm_failover`, `llm_circuit_breaker`,
  `llm_timeout` sub-rules. Updated `step_ident` negative lookahead.
- `src/ast.rs`: Added `Declaration::LlmConfig(LlmConfigDecl)`, `LlmProviderEntry`,
  `LlmConfigDecl` structs.
- `src/parser.rs`: Added `parse_llm_decl()` function. Handles balanced parsing
  of provider entries with optional key/url fields.
- `src/llm.rs`: Added `SmartRouter`, `LlmUsageTracker`, `ProviderHealth`,
  `LlmUsageReport`, `ProviderUsage`, `GLOBAL_LLM_USAGE` static. SmartRouter
  implements failover, circuit breaker, health-score-based provider selection,
  and multi-format provider calls (OpenAI-compatible, Anthropic, Ollama).
- `src/interpreter.rs`: Added `llm_config` and `smart_router` fields to
  `Interpreter`. LlmConfig handler creates SmartRouter. `invoke_learnable_with_env`
  uses SmartRouter when available, falls back to legacy backend.
- `src/builtins.rs`: Added `llm_usage()` builtin returning usage Struct.
- `src/compiler.rs`: Added `LlmConfig` to no-instruction-needed patterns.
- `Cargo.toml`: Added `once_cell = "1"` dependency.

### Architecture

```
llm {} declaration
    → Interpreter stores LlmConfigDecl + creates SmartRouter
    → invoke_learnable_with_env checks smart_router
        → SmartRouter::call()
            → Sort providers by health_score
            → Check circuit breaker per provider
            → Try best available provider
                → On success: record health, return response
                → On failure: record health, try next (if failover=auto)
            → Record to GLOBAL_LLM_USAGE (for llm_usage() builtin)
```

### Backward Compatibility

- If no `llm {}` block: `smart_router` remains `None`. All LLM calls go through
  the legacy `create_llm_backend()` path. Existing `.mlog` programs continue to work
  unchanged.
- Per-pattern `model:` field (ADR-0048) continues to work via `resolve_model()`.
- `METALOGOS_MOCK_LLM=true` (default) still uses MockLlm for all calls.

## Contracts

1. **Failover**: Broken provider + working provider → automatic fallback, response received.
2. **Circuit breaker**: After N failures, provider skipped without attempt.
3. **Per-pattern model**: Different model aliases → different models in LLM calls.
4. **Token tracking**: 3 calls → `llm_usage().total_calls == 3`.
5. **OpenAI-compatible**: Custom URL with OpenAI-format mock → response received.
6. **Backward compatibility**: No `llm` block → env vars work as before.

## Prior Art

- **LiteLLM**: 100+ providers unified API (python).
- **OpenHands**: Model-agnostic multi-LLM support.
- **free-coding-models**: Smart router with health probes.
- **Resilience4j**: Circuit breaker pattern (Java).

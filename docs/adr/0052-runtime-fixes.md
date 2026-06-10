# ADR-0052: Metalogos Runtime Fixes (Наряд №12)

**Status:** Accepted
**Date:** 2026-06-10
**Branch:** `fix/metalogos-runtime`

## Context

The Metalogos runtime had 4 bugs discovered during production deployment (FOSVED-office-v2):

1. `call_llm()` and builtins did not propagate fully into per-request route handlers
2. `http_post()` lacked support for authorization headers
3. `__replace()` had reported UTF-8/Cyrillic panics (verified safe; added tests)
4. No way to configure a custom OpenAI base URL for proxy/self-hosted deployments

## Decision

### Bug 1: Builtins propagation in route handlers

`Interpreter::clone_definitions_into()` now copies:
- `hooks_before` / `hooks_after` (ADR-0045 advisory hooks)
- `llm_cache` (ADR-0047 LLM response cache)
- `pattern_stats` (ADR-0051 per-pattern statistics)

Previously, per-request interpreters in `execute_route_body()` would have fresh empty caches, losing the benefit of cached LLM responses and breaking hooks.

### Bug 2: http_post() headers parameter

Added optional 4th parameter to `http_post()`:

```
http_post(url, body, content_type)
http_post(url, body, content_type, headers)
```

The `headers` parameter accepts either:
- A JSON string: `'{"Authorization": "Bearer sk-xxx"}'`
- A `Value::Struct`: constructed via `parse_json()` or entity syntax

Backward compatible — 3-arg calls work unchanged.

### Bug 3: __replace() UTF-8 safety

The builtin uses Rust's `String::replace()` which operates on `&str` (UTF-8 slices) and is inherently safe for Cyrillic, CJK, and multi-byte characters. Added 5 contract tests proving safety with Cyrillic text, emojis, empty patterns, and no-match cases.

### Bug 4: METALOGOS_OPENAI_BASE_URL

New environment variable `METALOGOS_OPENAI_BASE_URL` allows routing OpenAI-compatible API calls through a custom proxy:

```bash
export METALOGOS_OPENAI_BASE_URL="https://my-proxy.example.com/v1"
```

The path suffix (`/chat/completions`, `/v1/messages`, `/api/generate`) is automatically extracted from the default endpoint and appended to the custom base URL.

Works for all 3 providers (Anthropic, OpenAI, Ollama).

## Implementation

| File | Change |
|------|--------|
| `src/interpreter.rs` | `clone_definitions_into()` copies hooks, llm_cache, pattern_stats |
| `src/builtins.rs` | `http_post()` accepts optional 4th headers parameter |
| `src/llm.rs` | `RealLlm` gains `base_url` field + `resolve_endpoint()` method |
| `tests/naryad_12_runtime_fixes.rs` | 12 contract tests for all 4 bugs |
| `examples/test_naryad12.mlog` | Integration test for Cyrillic replace |

## Consequences

- Route handlers now share LLM cache and hooks with the global interpreter
- `http_post()` can pass authorization headers for API integrations
- Proxy/self-hosted LLM deployments work via `METALOGOS_OPENAI_BASE_URL`
- No breaking changes — all additions are backward compatible

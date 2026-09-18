# ADR-0137: LLM streaming — `llm_stream_open/next/close` over `reqwest::blocking`

**Status:** Accepted
**Date:** 2026-09-13
**Naryad:** #275 (issue #311, dispatch #316; spike report — `docs/research/naryad-275-streaming-spike.md`)
**Precedent:** ADR-0048 (SmartRouter — the stream sits ON TOP of it, does not duplicate the choice), ADR-0096 (block-in-place single-core — the stream introduces no async/tasks), ADR-0101 (deferred response — no overlap), ADR-0114 (`Value::Reflex` opaque handle — template for `Value::LlmStream`), ADR-0124 (`Value::Vision` — template for the registry in `crate::llm`), ADR-0138 (per-call LLM traces — the stream writes ONE trace line per completed call), #263 (bounded state map — `LLM_STREAM_REGISTRY` is limited).

## Context

All LLM calls in Metalogos today are blocking all-or-nothing: `call_llm` / `SmartRouter::call` wait for the full answer, `resp.text()` in full, timeout 30s (SmartRouter) / 120s (`call_claude`). The runtime is single-core block-in-place (ADR-0096); serve is axum/tokio + `spawn_blocking` (ADR-0096 §2, the same model). For FOSVED UX (Telegram: long department answers, the voice loop) progressive output is a noticeable improvement, but it must not break the concurrency model.

Issue #311 formulates the spike gate SG-3 (a verdict gate, approved by the owner 2026-09-12): the spike decides on its own; ADR-0137 is written post-factum. Go criteria: SSE via `reqwest blocking` without rewriting the backends; the stream ON TOP of SmartRouter; each `llm_stream_next` blocks for ≤ 1 chunk (ADR-0096 holds by construction); TW/VM parity is achievable. No-Go criteria: a backend rewrite or callbacks/tasks are required → honest No-Go, the idea goes to Tier 3 (Python llm_proxy FOSVED, FO-013). Backends without a stream return an explicit `STREAM_UNSUPPORTED` error, not a silent full-answer.

## Decision (spike verdict: GO)

### D1. API — iterator style, no callbacks

```mlog
let s = llm_stream_open(prompt, input?)        // -> Struct { handle: LlmStream, model: String, provider: String }
let chunk = llm_stream_next(s.handle)          // -> String (delta) | "" (keep-alive / end-of-stream marker)
let final = llm_stream_close(s.handle)         // -> Struct { tokens: Float, latency_ms: Float, status: String }
```

- `llm_stream_open(prompt: String, input?: String) -> Struct { handle, model, provider }`.
  Selects the best available provider via the same mechanism as `SmartRouter::call` (candidates sorted by `health_score`, circuit breaker, failover=auto only at the open stage). Returns an opaque handle into `LLM_STREAM_REGISTRY`.
- `llm_stream_next(handle: LlmStream) -> String` — one blocking `Read::read` + parsing of one SSE delta. Returns the `delta` string, or `""` for a keep-alive ping, or the special end-of-stream marker (the end convention is `"__end__"`, chosen after the existing soft-failure EOF patterns; fixed here).
- `llm_stream_close(handle: LlmStream) -> Struct { tokens, latency_ms, status, provider, model }` — drops the response, aggregates usage from the final SSE event, writes **one** JSONL trace line (ADR-0138 §D4 — "one line per completed call, not per chunk"), returns the final metadata.

### D2. Opaque handle — `Value::LlmStream(LlmStreamId)`

`LlmStreamId = u32` (a new type, template: `ReflexId`/`VisionId`). An index into `LLM_STREAM_REGISTRY` (a process-global `Lazy<Mutex<HashMap<u32, LlmStreamState>>>`), lives in `crate::llm` (same as `GLOBAL_SMART_ROUTER`/`GLOBAL_LLM_USAGE` — both backends go through `crate::llm`).

`Value::LlmStream(LlmStreamId)` — a new `Value` variant. `Display` = `[LlmStream#<id>]`. `Debug` — manual, as for `Reflex` (ADR-0114); prints only `provider` and `model`, not the payload. `Serialize`/`Deserialize` — manual, as for `SecretString`/`Reflex`: serializes as the marker `"[LlmStream]"` so that an active stream cannot be dumped into persistent storage. `type_name` = `"LlmStream"`.

### D3. Transport — `reqwest::blocking` + `impl Read` + a hand-rolled SSE parser

**Deviation from the issue #311 wording (recorded loudly):** the issue says "reqwest blocking `chunk()`". `reqwest::blocking::Response` has **no** `chunk()` method (that is a method of the async `Response`). Instead, `impl std::io::Read for Response` is used — a blocking `read(&mut [u8; N])` reads "one buffer-full" from the TCP stream and returns control. This is **semantically equivalent** to "incremental chunked SSE reading" and satisfies the spirit of the Go criterion (no backend rewrite, no callbacks, no async).

One `llm_stream_next` call:
1. One `Read::read(&mut [u8; 8192])` — blocking, ≤ 8192 bytes from the TCP buffer (or fewer).
2. Append into the stream state's `line_buffer`.
3. Parse one complete SSE delta out of the line buffer (format: `data: <json>\n\n` or `event: ...\ndata: <json>\n\n`).
4. If the buffer holds no complete delta — repeat `Read::read` (still "≤ 1 syscall per chunk", but logically "one delta").
5. Return the `delta` string (or `""` for a keep-alive ping; `"__end__"` for the end).

### D4. Stream — a parallel path, not a replacement for single-shot

`SmartRouter::call` (blocking single-shot) **remains unchanged**. `call_llm` / learnables / `call_claude` / `call_llm_schema` — untouched. The stream is a separate API surface:

```rust
impl SmartRouter {
    pub fn stream_open(&self, prompt, input, model_override, timeout) -> Result<LlmStreamState, String>;
    //              ^-- the same candidates/circuit-breaker/resolved_model as call()
    //                  but the POST body has "stream": true (for OpenAI/Anthropic)
    //                  and it returns Response + an incremental SseParser
    pub fn stream_next(state: &mut LlmStreamState) -> Result<String, String>;
    pub fn stream_close(state: LlmStreamState) -> LlmStreamFinal;
}
```

Non-stream providers (mock, future non-SSE) → `STREAM_UNSUPPORTED` at the `stream_open` stage. Not a silent full-answer.

### D5. Failover within a stream — one provider per stream

Failover within a stream is conceptually impossible (one cannot "reconnect to a different provider mid-stream" without losing the chunks already received). `stream_open` picks the best available provider and holds onto it until `close`. If the provider dies mid-stream — `next` returns an error, the user calls `close`; at the next `open` the circuit breaker will mark the provider unhealthy and skip it. This does **not** violate the SmartRouter contract of ADR-0048 (failover applies at the selection stage, not mid-call).

### D6. Concurrent stream limit (lesson of #263)

`LLM_STREAM_REGISTRY` — a `HashMap<u32, LlmStreamState>` with an **explicit upper bound** (64 by default, `METALOGOS_LLM_STREAM_MAX` env override). Exceeding it → a loud `STREAM_LIMIT_REACHED` error. This is the lesson of naryad #263 (unbounded state maps overflow). In serve route bodies the limit protects against resource leaks from unclosed streams.

### D7. Leak elimination

An unclosed stream on scope exit / error — forced close at the interpreter level (template: `Session`/`Conversation` live in the interpreter and are dropped on scope exit). In serve route bodies: the limit (D6) + an explicit drop on exit from the `spawn_blocking` closure (ADR-0096 §2 — the interpreter is cloned into the closure and dropped after `.await`).

### D8. Trace — one line per completed stream (ADR-0138 §D4)

`llm_stream_close` writes **one** `trace_llm_call` line:
- `latency_ms` = open→close (duration of the full stream).
- `gen_ai.usage.input_tokens` / `gen_ai.usage.output_tokens` — aggregated from the final SSE event (Anthropic: `message_delta` with `usage`; OpenAI: the final `data: [DONE]` with `usage` in `stream_options.include_usage`; ollama: in every chunk — we take it from the final one).
- `cache` = `"miss"`.
- `provider_alias` = the alias of the provider the stream was opened on.

`next` calls are **not traced per-chunk** — this is fixed in ADR-0138 §D4: "streams will emit one line per completed call with summarized usage (the issue's contract), not per chunk". Streaming is one LLM call spread over time.

### D9. TW/VM parity

`LLM_STREAM_REGISTRY` lives in `crate::llm` (like `GLOBAL_SMART_ROUTER` / `GLOBAL_LLM_USAGE`). A single builtin body — both backends call `crate::llm::stream_via_smart_router` (following the `call_via_smart_router` pattern). The handle is a `u32` index, not owned data — the `Send` bound is satisfied trivially (as for `ReflexId`/`VisionId`).

## Consequences

- ✅ Single-core block-in-place semantics of ADR-0096 preserved — no async/tasks/callbacks.
- ✅ Single-shot LLM path unchanged — `call_llm`/learnables/`call_claude`/`call_llm_schema` with no regressions.
- ✅ Trace contract of ADR-0138 §D4 honored — one line per stream.
- ✅ SmartRouter contract of ADR-0048 not violated — failover works at the open stage.
- ✅ Opaque-handle pattern of ADR-0114 reused — `Value::LlmStream(LlmStreamId)`, the registry in `crate::llm`.
- ✅ Bounded state map (#263) — `LLM_STREAM_REGISTRY` is limited.
- ⚠️ Deviation from the issue #311 wording: not literally `chunk()`, but `impl Read` + an SSE parser. Recorded loudly in D3.
- ⚠️ The stream is a separate API surface, it does not replace `call_llm`. This is a deliberate choice — it prevents regressions in covered paths.
- ⚠️ Failover mid-stream is conceptually impossible (D5) — the user decides on their own to close the stream on error and open a new one.

## Addendum: Open for future ADRs

- Streaming into the HTTP response (`respond_stream` in serve) — a separate ADR, not part of #275. #275 delivers only the `llm_stream_*` builtins; integration with serve deferred-response (ADR-0101) is a separate task.
- Stream for MCP clients (ADR-0132) — no overlap; MCP is stateless, no stream needed there.
- Stream for `call_llm_schema` (ADR-0133) — not part of #275 v1; schema validation requires the full answer to validate against the JSON schema.

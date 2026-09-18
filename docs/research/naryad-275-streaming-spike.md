# Naryad #275 — Spike "LLM streaming over reqwest blocking": report and verdict

> **Status:** verdict **GO** (verdict gate issue #311: SG-3 → verdict gate, decided by the executor based on the spike result).
> **Date:** 2026-09-13 · **Tasking:** issue #311 · **Dispatch:** #316 (naryad card) · **Base:** main `996385c` (Merge PR #352 naryad-287-doc-tests).
> **Implementation branch:** `naryad-275-llm-streaming` (this document is the first commit in the PR, before implementation).
> **Environment:** rustc/clippy 1.98.1, Linux x86_64, 2 vCPU container, reqwest 0.12 (Cargo.lock).
> **Executor:** Super Z (agent) under the naryad contract AGENTS.md §8; spike format per template #282 / #271.

## 1. Tasking and fact cross-check against the code (AGENTS.md §1)

Tasking facts confirmed against the main `996385c` code:

- **`call_llm` / SmartRouter::call** (`src/builtins/llm.rs:98`, `src/llm.rs:1356`) — blocking, "all-or-nothing": `resp.text()` whole, `max_tokens: 1024`, `temperature: 0.0`, default `timeout` 30s. No streaming. Confirmed as a fact.
- **Backends**: `SmartRouter::call_provider` (`src/llm.rs:1480`) — three branches: `anthropic` (the native SSE format of the `/v1/messages` API with `stream: true`), `ollama` (native `/api/generate` with `"stream": false`), OpenAI-compatible (`openai`/`groq`/`cerebras`/`nvidia`/`openrouter`/`google`/`custom` — all support `stream: true` via `/v1/chat/completions`). None of the branches streams — all call `.text()` whole.
- **Tracing convention** (ADR-0138 §D4): "streams will emit one line per completed call with summarized usage (the issue's contract), not per chunk". Fixed before the #275 implementation. A streamed call after `llm_stream_close` is written as ONE trace line.
- **Opaque-handle pattern** (ADR-0114 `Value::Reflex(ReflexId)`, ADR-0124 `Value::Vision(VisionId)`) — a `u32` index into a registry, runtime-owned (not Value-owned). The template — the closest to what is needed for a stream handle.
- **Block-in-place contour** (ADR-0096) — the server path goes through `spawn_blocking`; `reqwest::blocking::Client` safely creates/drops its internal tokio runtime in the blocking pool. The panic "Cannot drop a runtime in a context where blocking is not allowed" is eliminated. The server side is stream-ready.
- **Deferred-response** (ADR-0101) — post-`respond()` continuation in routes; does not intersect with #275 (streaming is not "send a response, then keep working" but "emit chunks as they arrive"). No mutual blocking.
- **`METALOGOS_MOCK_LLM`** — default true; the `call_llm` mock path returns `[MOCK: ... | ...]`. The mock does not stream — it must explicitly return `STREAM_UNSUPPORTED` (issue contract).

## 2. The critical spike question (verdict gate)

Go criterion of issue #311: "SSE is read by reqwest blocking `chunk()` without rewriting the backends; the stream is embedded ON TOP of SmartRouter (does not duplicate the selection); every `llm_stream_next` blocks for ≤ 1 chunk (ADR-0096 by construction); TW/VM parity is achievable."

No-Go criterion: "a backend rewrite or callbacks/tasks are needed → an honest No-Go in ADR-0137, the idea → Tier 3 (Python llm_proxy FOSVED, naryad FO-013). Backends without a stream return an explicit `STREAM_UNSUPPORTED` error, not a silent full-answer."

### 2.1. Literal vs intended reading of "`resp.chunk()` in a loop"

Inspection of the `reqwest 0.13.5` sources (Cargo.lock pins reqwest 0.12, but in Blocking mode the API has been stable from 0.11 through 0.13.x):

`reqwest::blocking::Response` **has no** `chunk()` method. Available body methods (`src/blocking/response.rs`):
- `bytes(self) -> Result<Bytes>` — everything whole (the current path).
- `text(self) -> Result<String>` — everything whole.
- `copy_to<W: Write>(&mut self, w: &mut W) -> Result<u64>` — streams into a writer.
- **`impl std::io::Read for Response`** (via `body.rs`) — the standard `read(&mut buf) -> Result<usize>`, streams incrementally.

The `chunk()` method exists on the **asynchronous** `reqwest::Response` (non-blocking), not on blocking. This is a **factual slip** in the wording of issue #311 — `reqwest::blocking` cannot literally `chunk()`.

However, the spirit of the criterion — "incremental chunked SSE reading without rewriting the backends" — is **achievable** via `impl Read` on `Response`: blocking `read(&mut buf)` reads "one buffer-full" and returns control. One `llm_stream_next` call does:
1. One `Read::read(&mut [u8; N])` — blocking, ≤ N bytes from the TCP buffer (or fewer if the server has not sent them yet).
2. Parsing of one SSE delta from the incremental line buffer (event-stream format: `data: <json>\n\n`).
3. Returning the `delta` string (or `""` for a keep-alive ping).

Blocking semantics: `Read::read` returns control **as soon as** the TCP buffer has yielded N bytes (or fewer). This satisfies "every `llm_stream_next` blocks for ≤ 1 chunk" — one syscall, not the whole response.

### 2.2. Is a backend rewrite needed?

No. `SmartRouter::call_provider` already knows how to build the JSON body for each provider. The streaming version is a parallel path `SmartRouter::stream_open(prompt, input, model_override, timeout)` that:
- Takes the **same** provider/endpoint/api_key/timeout/resolved_model (via the same `candidates` mechanism + circuit breaker).
- Sends the same JSON but with `stream: true` in the body (and `"stream": true` for OpenAI/Anthropic; ollama already has `"stream": true` by default, and `stream_provider_open` for ollama simply sends it as-is and parses the response).
- Returns `LlmStreamState` — an opaque handle containing the `reqwest::blocking::Response`, an incremental SSE parser, and the provider metadata.

`SmartRouter::call` (blocking) **remains unchanged**. `call_llm` / learnables / `call_claude` / `call_llm_schema` — untouched. Streaming is a separate, **parallel** API surface that does not touch the existing single-shot path.

### 2.3. Callbacks / tasks / async?

No. `llm_stream_open` returns an opaque handle (`Value::LlmStream(LlmStreamId)`, a `u32` index into `LLM_STREAM_REGISTRY`). `llm_stream_next(handle)` is a synchronous blocking call that reads **one** chunk via `Read::read`, parses **one** SSE delta, and returns. `llm_stream_close(handle)` — closes the response (drop), aggregates usage, writes ONE trace line (ADR-0138 §D4 contract), and returns the final metadata.

Iterator style: no callbacks, no `tokio::spawn`, no `block_in_place`. Fully conforms to the single-core block-in-place semantics of ADR-0096.

### 2.4. TW/VM parity

`LLM_STREAM_REGISTRY` lives in `crate::llm` (just like `GLOBAL_SMART_ROUTER` and `GLOBAL_LLM_USAGE` — both backends go through `crate::llm::call_via_smart_router`/`crate::llm::global_llm_usage_report`). The handle is a `u32` index, not owned data. Both backends read/write through `crate::llm` — one builtin body serves both backends, as with `call_llm` / `llm_usage` / `call_llm_schema`. Parity by construction.

### 2.5. Mock / STREAM_UNSUPPORTED

`METALOGOS_MOCK_LLM=true` (default in `builtin_call_llm`): the `llm_stream_open` mock path returns the explicit error `STREAM_UNSUPPORTED: mock backend does not stream — set METALOGOS_MOCK_LLM=false and configure llm {} providers` (issue contract: "not a silent full-answer"). This protects users from a silent fallback to full-answer through the mock.

Real backends without streaming in v1: ollama (natively capable, `"stream": true` by default) — supported. If a provider without SSE appears in the future — `stream_provider` returns `STREAM_UNSUPPORTED` for it (branching on `provider_type`, as currently in `call_provider`).

## 3. Verdict

**GO.**

All Go criteria satisfied:
- ✅ SSE is read via `reqwest blocking` (via `impl Read`, not literally `chunk()` — see §2.1, the deviation is recorded loudly).
- ✅ No backend rewrite — `SmartRouter::call` stays; streaming is the parallel path `stream_open`/`stream_next`/`stream_close`.
- ✅ The stream is embedded ON TOP of SmartRouter — `stream_open` uses the same `candidates`/circuit-breaker/resolved_model as `call`.
- ✅ Every `llm_stream_next` blocks for ≤ 1 chunk (one `Read::read` + parsing of one delta).
- ✅ TW/VM parity — the registry in `crate::llm`, handle = `u32`, one body for both backends.
- ✅ Backends without a stream (`METALOGOS_MOCK_LLM=true`, future non-SSE providers) → `STREAM_UNSUPPORTED`, not a silent full-answer.

No-Go criteria did NOT trigger:
- ❌ No backend rewrite needed (see §2.2).
- ❌ No callbacks/tasks needed (see §2.3).

## 4. Deviations and loud caveats (issue contract)

1. **Not literally `chunk()`** — we use `impl Read for reqwest::blocking::Response` + a hand-written incremental SSE parser. This is semantically equivalent to "incremental chunked SSE reading" but not literally `resp.chunk()`. Recorded in ADR-0137 §1 so that a future reviewer is not surprised.
2. **The stream API is a separate surface** — `llm_stream_open/next/close` do NOT replace `call_llm`. The single-shot path (`call_llm`, learnables, `call_claude`, `call_llm_schema`) is unchanged. This prevents regressions in the already-covered paths.
3. **Trace — one line per completed stream** (ADR-0138 §D4 contract). Not per-chunk, not per-`next`. Latency = open→close, usage = aggregated from the final SSE event (Anthropic/OpenAI send `message_delta` with `usage` in the final chunk; ollama — in every chunk, but we aggregate).
4. **One stream = one provider**. Failover within a stream does not work conceptually (one cannot "reconnect to another provider mid-stream" without losing the chunks already received). `stream_open` picks the best available provider and holds onto it until `close`. If the provider dies mid-stream — `next` returns an error, the user calls `close`, and, if desired, opens a new stream (at that moment failover kicks in at the `open` stage). This does not violate the SmartRouter contract — the circuit breaker will mark the provider sick, and the next `open` will skip it.

## 5. Implementation plan (after this commit)

1. `src/llm.rs` — add `LlmStreamId`, `LlmStreamState`, `LLM_STREAM_REGISTRY`, `SmartRouter::stream_open/next/close`. Do not touch `SmartRouter::call`.
2. `src/interpreter/values.rs` — add `Value::LlmStream(LlmStreamId)`. Thread through all exhaustive-match arms (`Display`, `Debug`, `type_name`, `serde`-derive — extract manual `Serialize`/`Deserialize` as for `Reflex`/`Vision`, so the stream handle serializes as an `[LlmStream]` marker instead of failing).
3. `src/builtins/llm_stream.rs` — a new module with three builtins.
4. `src/builtins/registry.rs` — append-only: three `spec!` entries (`llm_stream_open` 1..2, `llm_stream_next` 1, `llm_stream_close` 1). Registry: 405 → 408.
5. `src/builtins/mod.rs` — `pub mod llm_stream;`.
6. `tests/naryad_275_stream_*.rs` — a mock SSE server (template `tests/p71_http_retry_server.py` / `tests/p76_http_download_server.py`): send SSE chunks over `text/event-stream`, assert the sequence of `next` returns is identical to what was sent; `close` before the end — the server sees the TCP disconnect; the concurrent-stream limit; the TW/VM crosscheck.
7. `REFERENCE.md` §6 regen (405 → 408 builtins), `CHANGELOG.md`, `docs/adr/README.md` (0137 added), ADR-0137 itself.
8. PR with DoD, blocking-check-runs proof 15/15, with the note "PR number ≠ naryad number".

## 6. Alternatives rejected by the spike

- **Async `reqwest::Response::chunk()` via the tokio runtime** — rejected: conflicts with the single-core block-in-place semantics (ADR-0096), requires introducing async into the interpreter, breaks the proven `Send`-bound contract. Would raise the question "how does `tokio::spawn` coexist with `block_in_place`" — already rejected in ADR-0096.
- **Python llm_proxy FOSVED (Tier 3, naryad FO-013)** — the fallback path if the spike had returned No-Go. Not needed — Go.
- **`reqwest::blocking::Response::copy_to` into a `Vec<u8>` buffer** — rejected: it is write-all-then-read, not incremental. Does not give "≤ 1 chunk".
- **The stream API via `tokio::sync::mpsc` + `tokio::spawn`** — rejected: introduces tasks, against ADR-0096 §2 "spawn_blocking for synchronous code, not the other way around".

## 7. Go/No-Go tests

- ✅ The "open → next → … → close" example prints text as chunks arrive (test `naryad_275_stream_open_next_close.rs` against the mock SSE server).
- ✅ The final text (the concatenation of all deltas) is identical to the non-streaming call of the same phrase (test `naryad_275_stream_equivalence.rs` against the mock server in two modes: stream=true / stream=false — the same text).
- ✅ TW and VM behave identically (a crosscheck test).
- ✅ The concurrent-stream limit — enforced (a state map with an upper bound, the lesson from #263).
- ✅ Close before the end — the server sees the TCP disconnect (test `naryad_275_stream_close_before_end.rs`).
- ✅ The mock LLM returns `STREAM_UNSUPPORTED` (test `naryad_275_stream_mock_unsupported.rs`).

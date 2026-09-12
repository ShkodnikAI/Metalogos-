# ADR-0138: Per-call LLM traces — file-based JSONL with OpenTelemetry GenAI field names

**Status:** Accepted
**Date:** 2026-09-12
**Naryad:** #276 (issue #312)
**Precedent:** ADR-0047 (LLM response cache — the `cache` field's "exact" value);
ADR-0048 (SmartRouter — the single instrumented funnel); №4 (`llm_usage()` —
the aggregate view this complements); №156 (per-call timeout/latency precedent);
№259 (env-convention discipline for `METALOGOS_*` variables).

## Context

Observability of LLM usage in Metalogos was aggregate-only: `llm_usage()`
reports total calls/tokens/errors and per-provider health, but there is no
per-call record. The FOSVED contour (backlog item B2) needs a stream of
events — tokens/cost/latency per request — as routing data (FOIP-003) and
input for the darwin cycle (FOIP-002). Post-hoc reconstruction from aggregates
cannot answer "which call was slow/expensive/failed for which model".

The deployment reality constrains the transport: Metalogos is a CLI/library
that runs on a laptop as often as on a server. OTLP export (collector,
async runtime, network) is deliberately deferred to Tier 3. A flat file is
the honest v1: appendable by any process, readable by any tool, zero new
dependencies.

## Decision

### D1. Transport — JSONL file via `METALOGOS_LLM_TRACE`, no dependencies

When the env variable `METALOGOS_LLM_TRACE` is set to a path, every LLM call
appends exactly one JSON line (newline-delimited JSON) to that file. When it
is unset or empty, tracing is fully off. The format is line-per-event so a
crash loses nothing already written; readers process partial files naturally
(the same reasoning as №268's MCP framing). `append + flush` per line is a
deliberate survivability-for-speed trade: one `open(2)` + `write(2)` per call
is negligible against an LLM round-trip measured in seconds.

**No rotation in v1** — documented here so it does not surprise anyone: the
file grows without bound; the operator rotates it (logrotate, or a per-run
path). Automatic rotation would need size-state tracking that adds failure
modes to a component whose contract is "never break the LLM call".

### D2. Field names — OpenTelemetry GenAI semantic conventions, verified against the LIVE spec

Field names follow the OTel GenAI semantic conventions so a future exporter
reads the file **without renames**. Verified 2026-09-12 against the live
specification (open-telemetry/semantic-conventions-genai,
`docs/gen-ai/gen-ai-spans.md`; base semantic-conventions v1.44.0):

| JSONL field | Spec attribute | Notes |
|---|---|---|
| `gen_ai.provider.name` | `gen_ai.provider.name` (Required) | **Deviation from the issue text, on purpose**: the issue (snapshot 2026-09-11) said `gen_ai.system` — the upstream spec has RENAMED this attribute; `gen_ai.system` no longer exists in the live spans doc. Following the stale name would exactly produce the renames the field naming exists to avoid. |
| `gen_ai.request.model` | `gen_ai.request.model` (Conditionally Required if available) | |
| `gen_ai.usage.input_tokens` | `gen_ai.usage.input_tokens` (Recommended) | |
| `gen_ai.usage.output_tokens` | `gen_ai.usage.output_tokens` (Recommended) | |
| `name` | span name convention `{gen_ai.operation.name} {model}` | file-format adaptation: `name` = `gen_ai.<operation>`; v1 has only chat-style calls → always `"gen_ai.chat"` |
| `status` | (file-local) | `"ok"` \| `"error"` |
| `cache` | (file-local) | `"exact"` (ADR-0047 hit) \| `"miss"`; `"semantic"` reserved for №273 |
| `backend` | (file-local) | `"tw"` (tree-walking interpreter) \| `"vm"` (bytecode VM) |
| `provider_alias` | (file-local) | SmartRouter provider alias when routed through `llm {}` config |
| `latency_ms`, `ts` | (file-local) | ts = unix epoch milliseconds |

`anthropic`, `openai`, `groq` are well-known `gen_ai.provider.name` values;
`ollama`/`mock` are honest custom values (custom values are explicitly
allowed by the spec when no well-known value applies).

### D3. Honest data — absent is omitted, never invented

A field the provider did not report is **absent from the line**, not `null`
and not a guess. Mock calls and legacy-backend calls carry no token counts;
cache hits carry no provider/model (the cache entry does not store them);
transport errors carry no usage. Token usage is extracted best-effort from
the raw provider response (OpenAI-compatible `usage.prompt_tokens`/
`completion_tokens`, Anthropic `usage.input_tokens`/`output_tokens`, Ollama
`prompt_eval_count`/`eval_count`); an unparseable or missing usage block
yields nothing. The legacy (non-router) `RealLlm` path reports the model only
when `METALOGOS_LLM_MODEL` is set (the backend resolves aliases internally
and does not expose the resolved value through the type-erased trait object).

### D4. Single point of instrumentation — the funnel, not the call sites

One event per actual LLM invocation; never two. Instrumented exits:

- `SmartRouter::call` — the routed funnel (`call_llm`, `call_llm_schema`,
  learnables, conversation summaries, `human_respond` with `llm {}` config):
  success traces the winning provider (with usage); exhaustion traces
  `"error"` naming the LAST attempted provider; the no-providers fallback
  traces without provider fields (the legacy backend is type-erased at that
  point).
- `call_claude` — refactored into `builtin_call_claude` (trace at the single
  exit) + `call_claude_impl` (HTTP exchange returning text + usage);
  argument-type errors are NOT traced (no request reached any provider).
- Non-router fallbacks of `call_llm` and `call_llm_schema` — traced at the
  fallback (the SmartRouter path traces inside `SmartRouter::call`, so a
  routed call never double-traces). `call_llm_schema` retries call the
  provider closure once per retry — each retry is one trace line, because
  each retry IS one LLM call.
- Legacy (non-router) LLM invocations that bypass builtins entirely — found
  by auditing every `create_llm_backend()` call site — are traced at their
  own points: `Interpreter::call_llm` (TW learnables),
  `Vm`'s learnable evaluator (VM learnables), `summarize_conversation`
  (conversation summaries), and `human_respond` (mock and real arms).
  A site that calls the backend directly but produced no trace line would
  be a silent observability hole — the audit is part of this ADR's contract.
- ADR-0047 cache hit — traced with `cache: "exact"`; `latency_ms` measures
  the cache lookup, not generation.

Future call sites (№273 semantic cache, №275 streams) extend the same
single-point rule: streams will emit one line per completed call with
summarized usage (the issue's contract), not per chunk.

### D5. Failure isolation and overhead

Trace-write errors never fail the LLM call: one warning to stderr, then
silence for the rest of the process (a broken path must not spam a long
running server). Overhead when tracing is off: exactly ONE `env::var` check
per call — verified by construction (`trace_llm_call` returns before any
allocation when the variable is unset).

### D6. Backend tagging — thread-local, set-and-restore

The `backend` field is a thread-local tag: `"tw"` by default; VM entry
points (`Vm::run`, `Vm::execute_route_code`) set `"vm"` for the duration of
a program run and restore the previous value on exit (both success and
error). Restore-on-exit matters because thread pools reuse threads — a
leaked tag would mislabel the next program's calls. Server routes on either
backend are covered: TW routes run on interpreter threads (default `"tw"`),
VM routes go through `execute_route_code`.

## Consequences

- FOSVED B2 can consume per-call cost/latency/token data from day one by
  tailing a file; the OTLP exporter (Tier 3) later reads the SAME field
  names.
- `llm_usage()` aggregates remain unchanged — they answer "how much", the
  trace answers "which call".
- The trace file contains metadata only (no prompts, no responses) — it is
  safe to ship to centralized logging by default; the privacy-sensitive
  content (message bodies) stays out by design.
- Tracing adds no dependencies, no async, no threads; the only new
  synchronization is one `AtomicBool` for warn-once.

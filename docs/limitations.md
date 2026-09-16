# Known Limitations — Metalogos

> **This page is an index. The truth lives in the primary sources (ADR, source files, naryad reports).**
> Each row links directly to the authoritative document. This page does not rephrase — it references. If a limitation is listed here without a working link, that is a bug.

## Static Analysis (audit.rs)

| Limitation | Primary source | Status / condition for removal |
|---|---|---|
| Intraprocedural taint — bounded nesting depth 3 (Наряд №295) | [README §Known boundaries](../README.md#known-boundaries-of-static-analysis) | Bounded to `TAINT_NESTING_MAX_DEPTH=3`; deeper nesting documented as boundary |
| Interprocedural taint — bounded depth 2 (Наряд №292, `TAINT_INTERP`) | [ADR-0137](adr/0137-llm-streaming.md) (references); [README §Known boundaries](../README.md) | Bounded to `TAINT_INTERP_MAX_DEPTH=2`; deeper chains emit `INTERP_DEPTH_LIMIT` warning |
| Persistence taint — file/module scope only (Наряд №141/№157) | [threat-model.md §Known Boundaries](threat-model.md) | `TAINT_PERSISTENCE` check exists (Category-A Error) but bounded to file scope, not cross-module data-flow |
| `{{{ var }}}` raw template substitution bypasses escaping | [threat-model.md §Known Boundaries](threat-model.md) | By design — `template_render` with `raw=true` is trusted-author code only |
| `query(format(...))` — NOT a gap (Наряд №295 truth-up) | [threat-model.md §Known Boundaries](threat-model.md) | `check_sql_dynamic` rejects ALL non-literals in 1st arg of `query()`/`db_execute()` — compile-time error, not a silent gap |

## Bytecode VM (ADR-0105, ADR-0141)

| Limitation | Primary source | Status / condition for removal |
|---|---|---|
| ~~`Match` statement not compiled to VM~~ **CLOSED (№369)** | [ADR-0141](adr/0141-vm-production-readiness.md) Stage 1.1 | Closed: `Statement::Match` compiles to bytecode (`MatchTest` dispatch in both VM loops), TW-identical semantics; crosscheck exception `p_match_switch.mlog` lifted |
| ~~`Expr::BlockIfElse` (if/else as value) not compiled to VM~~ **CLOSED (№370)** | [ADR-0141](adr/0141-vm-production-readiness.md) Stage 1.2 | Closed: the block if/else VALUE compiles to bytecode in ANY expression position (BeginValueExpr/KeepLastValue/EndValueExpr register scheme + jump structure — no new dispatch needed), TW-identical semantics including the statement-block value leak inside branches |
| ~~`match_expr` (`let x = match y {...}`) — TW-only~~ **CLOSED (№369)** | [ADR-0141](adr/0141-vm-production-readiness.md) Stage 1.1 | Closed: `Expr::MatchExpr` is a first-class value on BOTH backends (the value is the last non-Unit expression of the matched arm — REFERENCE §Match); the old lossy №173b parse (arms discarded, raw scrutinee bound) is fixed end-to-end |
| ~~Binop coercion (heterogeneous List+String) — VM strict, TW lenient~~ **CLOSED (№371)** | [ADR-0141](adr/0141-vm-production-readiness.md) Stage 1.3 | Closed: VM `eval_binop` mirrors the TW interpreter exactly — same loud messages for heterogeneous operands (`type mismatch in string concatenation: List + String (use to_string() explicitly)`), same opaque-type restriction on `+`, same `MAX_STRING_LENGTH` (1 MB) limit; crosscheck exclusion `p118_collection_utils.mlog` lifted |
| PRNG state (`random_seed`/`random`) — TW-only, VM has no PRNG | [ADR-0105](adr/0105-vm-experimental-scope.md); crosscheck exclusion `reflex_math.mlog` | Stage 1 plan in [ADR-0141](adr/0141-vm-production-readiness.md) |
| Bool→String formatting: TW="true", VM="1" | [ADR-0105](adr/0105-vm-experimental-scope.md) | Stage 1 plan in [ADR-0141](adr/0141-vm-production-readiness.md) |
| VM is not the default backend for `mlog serve` | [ADR-0088](adr/0088-vm-serve-backend.md) | Default flip gated by [ADR-0141](adr/0141-vm-production-readiness.md) Stage 2-5 (parity + soak + benchmark) |

## Vision Pillar (ADR-0122)

| Limitation | Primary source | Status / condition for removal |
|---|---|---|
| Real-weights run — PARKED (no production PNG generated) | [ADR-0122](adr/0122-vision-pillar-scope.md) map row #237; [Наряд №294 No-Go report](research/naryad-294-vision-realw-no-go.md) | No-Go 2026-09-14 — requires hardware (≥64 GB RAM, ≥40 GB disk, GPU). Revision date: when hardware is allocated |

## Video Pillar (ADR-0147–0151)

| Limitation | Primary source | Status / condition for removal |
|---|---|---|
| Production-weights inference — PARKED (№294 class); `video_fetch_weights` is a loud error, not a fetcher | [ADR-0151 D7](adr/0151-video-i2v-pipeline.md); [Наряд №294 No-Go report](research/naryad-294-vision-realw-no-go.md) | Revision date: when hardware is allocated; the tiny seeded pipeline needs no fetching |
| Prompt embedding is hash-derived (`hash_embedding(seed)`), NOT a learned text encoder; the DiT text path itself is real (projected + added to every token) | [ADR-0153 D1/D2](adr/0153-video-text-path-wired.md) | Learned umT5-class encoders land with the production-weights revision (same №294-class trigger) |
| UNTRUSTED_FRAME taint — let-bound variables holding previously fetched untrusted frames are not tracked; full screen+consent ritual (frame_screen / LikenessToken) is V6 | [ADR-0151 D6](adr/0151-video-i2v-pipeline.md); [ADR-0149 D5/D6](adr/0149-video-security-gates.md) | Lands with the V6 LikenessToken mechanics |
| Interpolation is latent-space linear blending (RIFE-class MVP), not flow-warped synthesis; export payload is raw LE f32 (`MLGV-RAW-F32`), no compressed codec; av_mux supports PCM WAV only | [ADR-0151 D2/D4/D7](adr/0151-video-i2v-pipeline.md) | V7 research items (flow-guided interp, container codec) |

## Adapt / Reflex (ADR-0112, ADR-0117)

| Limitation | Primary source | Status / condition for removal |
|---|---|---|
| `adapt` quality metric is a mock (0.95 fixed) — not a real function | [ADR-0112](adr/0112-adapt-mock-metric.md) | Deferred until real case; revisit when a real metric is demonstrated |
| Text generation — out of scope for initial Reflex stages | [ADR-0117](adr/0117-distillation-semantics.md) §3 | Amended by [ADR-0120](adr/0120-opening-text-generation.md) — opening text generation accepted |
| VM parity for Reflex — VM-owned state, not shared `RuntimeContext` | [ADR-0121](adr/0121-vm-reflex-parity.md) | Accepted (closed gap) |

## Self-Hosting (ADR-0023)

| Limitation | Primary source | Status / condition for removal |
|---|---|---|
| Only the lexer is self-hosted (Phase 4.4); full self-hosting not pursued | [ADR-0023](adr/0023-self-hosting.md) | Accepted scope — full self-hosting is not a goal |

## JIT (ADR-0073)

| Limitation | Primary source | Status / condition for removal |
|---|---|---|
| JIT compilation is experimental scaffold — not production | [ADR-0073](adr/0073-jit-experimental.md) | Declared experimental; no production claim |

## Error Protocol (ADR-0142)

| Limitation | Primary source | Status / condition for removal |
|---|---|---|
| `try` returns `Unit` on error — loses error information (code/message) | [Наряд №91](adr/0142-error-protocol.md); [ADR-0142](adr/0142-error-protocol.md) | ADR-0142 accepted candidate (б) — `try` → Struct{ok,value,error}; implementation is a separate naryad |

## LLM Streaming (ADR-0137)

| Limitation | Primary source | Status / condition for removal |
|---|---|---|
| Stream API is separate from single-shot `call_llm` — not a replacement | [ADR-0137](adr/0137-llm-streaming.md) §D4 | By design — prevents regressions on single-shot path |
| Failover in mid-stream is impossible — provider chosen at `open` only | [ADR-0137](adr/0137-llm-streaming.md) §D5 | By design — circuit breaker marks provider sick, next `open` skips it |

## MCP Server (ADR-0132, Наряд №297)

| Limitation | Primary source | Status / condition for removal |
|---|---|---|
| MCP server is stdio-only — no HTTP/SSE transport | [ADR-0132](adr/0132-mcp-client.md); [src/mcp_server.rs](../src/mcp_server.rs) | HTTP/SSE is Future in ADR-0132 — separate naryad |
| MCP server is fail-closed — no tools exposed without explicit allowlist | [src/mcp_server.rs](../src/mcp_server.rs) | By design — explicitness principle |

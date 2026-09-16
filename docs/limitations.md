# Known Limitations — Metalogos

> **This page is an index. The truth lives in the primary sources (ADR, source files, naryad reports).**
> Each row links directly to the authoritative document. This page does not rephrase — it references. If a limitation is listed here without a working link, that is a bug.

## Static Analysis (audit.rs)

| Limitation | Primary source | Status / condition for removal |
|---|---|---|
| Intraprocedural taint — bounded nesting depth 3 (Наряд №295) | [README §Known boundaries](../README.md#known-boundaries-of-static-analysis) | Bounded to `TAINT_NESTING_MAX_DEPTH=3`; deeper nesting documented as boundary |
| Interprocedural taint — bounded depth, now CONFIGURABLE (Наряд №292; №376) | [ADR-0137](adr/0137-llm-streaming.md) (references); [README §Known boundaries](../README.md) | Bounded to `METALOGOS_TAINT_DEPTH` (default **4** — chosen by the №376 overhead measurement: depth 2→4 = +14.2% audit time on the 222-file corpus, within the +50% dispatch threshold; range 1..=16, invalid → default); deeper chains emit `INTERP_DEPTH_LIMIT` warning; cross-module summaries cache recomputes only on module change (key = source hash + depth) |
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
| ~~PRNG state (`random_seed`/`random`) — TW-only, VM has no PRNG~~ **CLOSED (№372, truth-up)** | [ADR-0141](adr/0141-vm-production-readiness.md) Stage 1.4 | Closed: the claim was STALE — `random_seed`/`random` route through the SHARED registry (thread-local xorshift64 in `src/builtins/math.rs`) on BOTH backends; identical seed → identical sequences, asserted by `tests/naryad_372_prng_bool_format.rs`; crosscheck exclusion `reflex_math.mlog` lifted |
| ~~Bool→String formatting: TW="true", VM="1"~~ **CLOSED (№372)** | [ADR-0141](adr/0141-vm-production-readiness.md) Stage 1.4 | Closed: VM comparisons (`eval_cmp`), `CmpNe`, `Contains`, `StartsWith`, MatchTest predicates now produce `Value::Bool` (TW encoding) — `to_string(a == b)` prints "true"/"false" on BOTH backends; truthiness unchanged (JumpIfNot is Bool-aware) |
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
| ~~`adapt` quality metric is a mock (0.95 fixed) — not a real function~~ **CLOSED for real mode (№375)** | [ADR-0112](adr/0112-mock-accuracy-metric.md) addendum 2026-09-16 | Real mode: golden-task battery (eval datasets + pre-mutation few-shot), held-out split, deterministic seeded order, real LLM answer path; 0.95 stub remains ONLY in mock mode (`METALOGOS_MOCK_LLM`, default-on test mode, loudly documented); battery < 20 tasks → loud BELOW MINIMUM warning; no held-out evidence → accuracy 0.0 |
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

## Backends — real-weights path (№333/№334, ADR-0163)

| Limitation | Primary source | Status / condition for removal |
|---|---|---|
| Real STT/omni/vision-understanding inference — PARKED (№334): the path is turnkey (registry pins, per-file SHA manifests, SSRF-guarded loader, mock-first call surface `stt_transcribe`/`omni_ask`/`vision_understand`), but real inference requires hardware (№294 No-Go: ≥64 GB RAM, ≥40 GB disk, GPU) and weights on disk | [ADR-0163](adr/0163-backend-registry-licenses.md); [src/backends_weights.rs](../src/backends_weights.rs); [Наряд №294 No-Go report](research/naryad-294-vision-realw-no-go.md) | Revision date: when hardware is allocated — fetch via `backends_weights::fetch_weights` (allowlist + SHA-pinned), then `METALOGOS_LLM_MOCK=false` runs the real call; without weights the refusal is loud, never a silent mock |

## Error Protocol (ADR-0142)

| Limitation | Primary source | Status / condition for removal |
|---|---|---|
| ~~`try` returns `Unit` on error — loses error information (code/message)~~ **CLOSED (№374)** | [ADR-0142](adr/0142-error-protocol.md) | Closed: `try` returns `Struct{ok,value,error}` on BOTH backends (shared builder, TW/VM cannot diverge); `code` carries the generic `RUNTIME_ERROR` until runtime errors are promoted to structured ADR-0131/0140 diagnostics — richer per-cause codes are the remaining boundary |

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

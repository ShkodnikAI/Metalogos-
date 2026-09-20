# Known Limitations — Metalogos

> **This page is an index. The truth lives in the primary sources (ADR, source files, naryad reports).**
> Each row links directly to the authoritative document. This page does not rephrase — it references. If a limitation is listed here without a working link, that is a bug.

## Static Analysis (audit.rs)

| Limitation | Primary source | Status / condition for removal |
|---|---|---|
| Intraprocedural taint — bounded nesting depth 3 (Наряд №295) | [README §Known boundaries](../README.md#known-boundaries-of-static-analysis) | Bounded to `TAINT_NESTING_MAX_DEPTH=3`; deeper nesting documented as boundary |
| Interprocedural taint — bounded depth, now CONFIGURABLE (Наряд №292; №376) | [ADR-0137](adr/0137-llm-streaming.md) (references); [README §Known boundaries](../README.md) | Bounded to `METALOGOS_TAINT_DEPTH` (default **4** — chosen by the №376 overhead measurement: depth 2→4 = +14.2% audit time on the 222-file corpus, within the +50% dispatch threshold; range 1..=16, invalid → default); deeper chains emit `INTERP_DEPTH_LIMIT` warning; cross-module summaries cache recomputes only on module change (key = source hash + depth) |
| ~~Persistence taint — file/module scope only~~ **cross-module MVP (Наряд №386)** | [threat-model.md §Known Boundaries](threat-model.md) | `TAINT_PERSISTENCE` (Category-A Error) now covers the cross-module case for literal/prefix memory keys: `PatternSummary` carries tainted-key metadata, a fingerprint-keyed registry matches `recall(<key>)` flows in module B against writes in module A; `METALOGOS_TAINT_STRICT=1` arms a key-less strict mode. №405 (ADR-0170): let-bound key prefixes are now resolved too (`let key = "p:" + id; memorize(key, …)` names `"p:"`), with fail-closed forks (an unresolvable re-assignment keeps the seen prefix; branch bodies share the map; a fresh map per scope). Keys with NO leading literal anywhere (call results, field reads) and full points-to remain the boundary (Phase 7) |
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
| VM is not the default backend for `mlog serve` | [ADR-0088](adr/0088-vm-serve-backend.md) | Default flip gated by [ADR-0141](adr/0141-vm-production-readiness.md) Stage 2-5. Stage 5 re-run 2026-09-20 (№404, 3× pinned, protocol №398): latency criterion met 3/3 (p95 ×2.50–×3.56 ≥ ×1.5 — the №402 divisor elimination confirmed), memory criterion NOT met 0/3 (peak RSS ×1.129–×1.143 > ×1.1) → **NOT flip-ready**; the default remains the interpreter, the VM pool remains opt-in ([ADR-0141](adr/0141-vm-production-readiness.md) Addendum 3). №409 (2026-09-20) compressed the measured per-request footprint — shared builtin registry + lazy per-request db open (db-free requests pay no sqlite; pooled idle VMs rest connection-free, ~236 → ~56 KB each; local bench RSS ratio ×1.08–1.11 → ×0.96–1.03, p95 margin widened) — the fresh re-gate under the SAME thresholds is №410's verdict ([ADR-0141](adr/0141-vm-production-readiness.md) Addendum 4) |

## Vision Pillar (ADR-0122)

| Limitation | Primary source | Status / condition for removal |
|---|---|---|
| Real-weights run — PARKED (no production PNG generated) | [ADR-0122](adr/0122-vision-pillar-scope.md) map row #237; [Наряд №294 No-Go report](research/naryad-294-vision-realw-no-go.md) | No-Go 2026-09-14 — requires hardware (≥64 GB RAM, ≥40 GB disk, GPU). Revision date: when hardware is allocated |

## Video Pillar (ADR-0147–0151)

| Limitation | Primary source | Status / condition for removal |
|---|---|---|
| Production-weights inference — PARKED (№294 class); `video_fetch_weights` is a loud error, not a fetcher | [ADR-0151 D7](adr/0151-video-i2v-pipeline.md); [Наряд №294 No-Go report](research/naryad-294-vision-realw-no-go.md) | Revision date: when hardware is allocated; the tiny seeded pipeline needs no fetching |
| Prompt embedding is hash-derived (`hash_embedding(seed)`), NOT a learned text encoder; the DiT text path itself is real (projected + added to every token) | [ADR-0153 D1/D2](adr/0153-video-text-path-wired.md) | Learned umT5-class encoders land with the production-weights revision (same №294-class trigger) |
| UNTRUSTED_FRAME taint — let-bound variables holding previously fetched untrusted frames are not tracked; the LikenessToken ritual (№387) covers the DECLARED likeness kind statically (presence-based), while the face-identity verification layer (frame_screen / real anti-spoof) is V6 | [ADR-0151 D6](adr/0151-video-i2v-pipeline.md); [ADR-0149 D5/D6](adr/0149-video-security-gates.md) | V6: face-identity verification in the ritual; the №387 token credential (challenge/verify + ledger trace + runtime unseal) is live |
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
| Real OCR inference — PARKED (№407): `ocr_extract` joins the same turnkey path (the canon `trocr-base-printed` registry entry with a real HF LFS pin and a per-file SHA manifest, mock-first surface), but real inference requires hardware (№294 No-Go) and weights on disk | [ADR-0163](adr/0163-backend-registry-licenses.md); [src/vision/ocr.rs](../src/vision/ocr.rs); [src/backends_weights.rs](../src/backends_weights.rs) | Revision date: the same №294 hardware gate — fetch via `backends_weights::fetch_weights("trocr-base-printed")`, then `METALOGOS_LLM_MOCK=false`; without weights the refusal is loud, never a silent mock |
| Real video-understanding inference — PARKED (№408): `video_understand` joins the №407 turnkey shape (three canon donors — qwen2.5-vl-7b-instruct / llava-video-7b-qwen2 / internvl3-8b — real per-shard HF LFS pins, mock-first surface; real-mode frame sampling is deterministic: fixed stride + first/last anchors, no randomness or time) | [ADR-0163](adr/0163-backend-registry-licenses.md); [src/video/understand.rs](../src/video/understand.rs); [src/backends_weights.rs](../src/backends_weights.rs) | Revision date: the №294 hardware gate — fetch via `backends_weights::fetch_weights(...)`, then `METALOGOS_LLM_MOCK=false`; without weights the refusal is loud, never a silent mock |

## Error Protocol (ADR-0142)

| Limitation | Primary source | Status / condition for removal |
|---|---|---|
| ~~`try` returns `Unit` on error — loses error information (code/message)~~ **CLOSED (№374)**; ~~`code` is a generic `RUNTIME_ERROR`~~ **CLOSED (№385)** | [ADR-0142](adr/0142-error-protocol.md); [ADR-0169](adr/0169-try-stable-error-codes.md) | Closed: `try` returns `Struct{ok,value,error}` on BOTH backends (shared builder, TW/VM cannot diverge). The `code` field carries a frozen origin-stamped diagnostic (№385, ADR-0169 §3.1): `LLM_TIMEOUT`, `LLM_PROVIDER_UNAVAILABLE`, `SQL_ERROR`, `SANDBOX_VIOLATION`, `SINK_CLEARANCE_RUNTIME`, `MEDIA_SEALED_EGRESS`, `BACKEND_DEGRADED` — classified ONCE per caught error by the origin stamp at position 0, never by message text. Remaining honest boundary: an error whose origin carries no stamp falls back to the generic `RUNTIME_ERROR` (API-arity refusals, lock poisoning, generic HTTP status answers); widening the stamp surface is incremental and explicitly not a blocker (audit P2-4, dispatch gh#529) |

## LLM Streaming (ADR-0137)

| Limitation | Primary source | Status / condition for removal |
|---|---|---|
| Stream API is separate from single-shot `call_llm` — not a replacement | [ADR-0137](adr/0137-llm-streaming.md) §D4 | By design — prevents regressions on single-shot path |
| Failover in mid-stream is impossible — provider chosen at `open` only | [ADR-0137](adr/0137-llm-streaming.md) §D5 | By design — circuit breaker marks provider sick, next `open` skips it |

## MCP Server (ADR-0132, ADR-0168, Наряды №297/№394)

| Limitation | Primary source | Status / condition for removal |
|---|---|---|
| ~~MCP server is stdio-only — no HTTP/SSE transport~~ **CLOSED (№394)** | [ADR-0168](adr/0168-mcp-server-transports.md); [src/mcp_server.rs](../src/mcp_server.rs) | Closed: `mlog mcp-serve --transport stdio\|http\|sse` (`src/main.rs`); http/sse run on the axum stack (feature `server`, default-on). First-stage boundary (live external verification, naryad #401, gh#488): Bearer auth per request (`--auth-token` / `METALOGOS_MCP_AUTH_TOKEN`, 401 on mismatch) or a token-less localhost-only bind with a loud posture line; a `0.0.0.0` bind without a token is a loud WARNING, never a silent open port. No built-in TLS and no multi-tenant identity yet — terminate TLS and scope identities at a reverse proxy for remote exposure |
| MCP server is fail-closed — no tools exposed without explicit allowlist | [src/mcp_server.rs](../src/mcp_server.rs) | By design — explicitness principle (the server refuses to start on EVERY transport without `--allowlist`) |
| Tool methods execute on the TW interpreter; the runtime sink backstop (`SINK_CLEARANCE_RUNTIME`) is VM-side, and the №259 env gate is serve-route-scoped — `env()` is readable inside a tool method | [src/mcp_server.rs](../src/mcp_server.rs); [src/vm.rs](../src/vm.rs); [src/builtins/io.rs](../src/builtins/io.rs) | The compile-time gates are the main line (`mlog check`/`mlog serve` refuse destructive SQL with `IRREVERSIBLE_NO_GRANT` and dynamic SQL with `SQL_DYNAMIC` inside tool bodies), the `tools/list` policy block discloses `irreversible`/`sink_calls` per tool before any call, and the VM path carries the runtime backstop. Runtime TW backstop for tool methods and a tool-method env posture are the open decision (gh#536) |

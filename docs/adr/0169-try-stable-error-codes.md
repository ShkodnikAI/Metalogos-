# ADR-0169: Stable `try` error codes — origin-stamped classification

**Status:** Accepted
**Date:** 2026-09-18
**Naryad:** #385 (issue #479; wave: post-dispatch #491)
**Pillar:** language / error protocol; extends ADR-0131 (code is a contract, text may change) onto the `try` result shape of ADR-0142 (№374)
**Consumers:** agent-office scenarios (retry/fallback/hard-fail branching), naryad #395 dogfood office, MCP tool surfaces that surface failure kinds

## 1. Context

Since №374 (ADR-0142), `try expr` returns a structured result
`Struct { ok: Bool, value: Value, error: Unit | Struct { code, message } }`.
But the `code` field was a constant: all three sewing points (TW
`Expr::Try` in `src/interpreter/execution.rs`; VM `Instruction::TryEval`
in both dispatch arms of `src/vm.rs`) stitched the generic
`"RUNTIME_ERROR"` into every caught error. An agent that must branch on
the KIND of failure — retry a timed-out LLM call, hard-fail a sandbox
violation, fall back on a dead provider — was forced into fragile
substring matching over `error.message`.

A stable code vocabulary already exists in the codebase as loud string
conventions: `[SANDBOX_VIOLATION] …` (№254, `src/builtins/io.rs`),
`[SINK_CLEARANCE_RUNTIME] …` (№325 runtime twin, `src/vm.rs`),
`MEDIA_SEALED_EGRESS: …` (№325/ADR-0162 §2.5, `src/builtins/media.rs`),
`BACKEND_DEGRADED` (№336/ADR-0165, typed `Degraded(t)` result). What was
missing is (a) a frozen name set, (b) a single classification point, and
(c) origin stamps for the subsystems that had none.

## 2. Decision Drivers

1. **Branching by code, never by message text.** ADR-0131 fixes the
   convention: the code is a contract, the message may change. The
   `try.error.code` set is the same contract extended to the runtime
   error channel; consumers branch on codes only.
2. **One classifier, three sewing points.** TW and VM MUST agree on the
   code for the same error — parity is part of the №373 parity gate. The
   only way that cannot drift is a shared function
   (`values::stable_try_error_code`) called by every sewing point.
3. **Classification by origin, not by prose.** The failing subsystem
   stamps the error WHERE IT IS BORN (where the subsystem knows what
   failed); the classifier only reads a strict whitelist of stamps at
   position 0. No subsystem is inferred from message substrings anywhere.
4. **Honest fallback.** Errors whose origin carries no stamp — API-arity
   refusals, lock poisoning, HTTP status answers, anything unclassified —
   stay `RUNTIME_ERROR`. A wrong specific code is worse than an honest
   generic one; inventing codes beyond the frozen set requires a test and
   an ADR.
5. **Zero new crates, zero new builtins.** The mechanism is string-level
   on the existing error channel (`Result<_, String>`); the builtin count
   (455) is unchanged.
6. **The typed path stays typed.** `Degraded(t)` (№336) is not converted
   into an exception; its `error.code` reuses the same frozen constant so
   the typed result and the String-channel whitelist cannot diverge.

## 3. Decision

### 3.1 The frozen code set

| Code | Origin subsystem | When it fires |
|---|---|---|
| `RUNTIME_ERROR` | (fallback) | the error's origin carries no stamp |
| `LLM_TIMEOUT` | `call_llm` contour | deadline / provider timeout (`reqwest` `is_timeout`, deadline-aware backend contours №248/№156) |
| `LLM_PROVIDER_UNAVAILABLE` | `call_llm` contour | connect failure (`is_connect`), SmartRouter circuit-open exhaustion (all rungs skipped) |
| `SQL_ERROR` | `db_*` builtins | a `rusqlite::Error` raised by the SQL layer itself (prepare/execute/row iteration) |
| `SANDBOX_VIOLATION` | io/exec sandbox | path/traversal/symlink refusals (№254 loud format) |
| `SINK_CLEARANCE_RUNTIME` | VM `SinkCheck` | the runtime twin of the №325 static sink gate |
| `MEDIA_SEALED_EGRESS` | `media_save` | sealed-at-rest media refused materialization (№325/ADR-0162 §2.5) |
| `BACKEND_DEGRADED` | `backend_select` | ladder exhaustion — TYPED result (№336); the String-channel whitelist reuses the same constant |

Names are frozen contracts (ADR-0131). New codes are appended together
with a test and an ADR — never inferred ad hoc.

### 3.2 The origin-stamp mechanism

`values::coded_error(code, msg)` produces the unified loud format
`[<CODE>] <msg>` at the origin (generalizing №254's
`[SANDBOX_VIOLATION] …`; `MEDIA_SEALED_EGRESS: …` is reformatted into the
same bracket form). `values::split_origin_stamp` recognizes a whitelisted
stamp ONLY at position 0 — a `[CODE]`-looking substring mid-message is
content, so a program cannot forge a classification by echoing a marker
into its own strings. `values::wrap_error_preserving_code` keeps the
stamp at the front when a wrapper layer prepends context
(`call_llm() failed: …`, `All LLM providers failed. …`, the №248 deadline
wording) — a naive wrap would bury the stamp mid-message and demote the
classification to `RUNTIME_ERROR`.

Classification is `values::stable_try_error_code(err) -> &'static str`:
whitelisted stamp → that code; anything else → `RUNTIME_ERROR`. It is
called by ALL THREE sewing points; `message` keeps the full original
text (stamp included), so existing message-reading consumers are
unaffected.

### 3.3 Honest non-stamps (explicit)

The following stay UNSTAMPED and classify as `RUNTIME_ERROR`: db
lock-poisoning and db API-validation refusals (not SQL-layer failures),
HTTP `>= 400` status answers (the provider IS available and answered),
other transport failures (TLS, request build — provider state unknown),
Telegram/TTS/whisper transport failures (different subsystems; their
codes are future contracts, not silent additions), stream-body read
errors after a successful open.

### 3.4 Deterministic fault seam (test infrastructure)

`METALOGOS_MOCK_LLM_FAULT=timeout|unavailable` (mock mode, legacy path)
makes `call_llm` fail with the corresponding stamped error — the loud
mock-mode twin of `MockLlm::set_delay_ms` (№126). Invalid values fail
CLOSED (a loud error naming the variable), never a silent green mock
answer. Golden examples declare their seam via an optional per-example
sidecar `examples/X.env` (KEY=VALUE), applied and removed around the run
by BOTH the golden runner and the TW↔VM crosscheck — so the parity gate
compares the fault-injected run on both backends.

### 3.5 Parity contract

TW and VM agree by construction (one classifier, shared origin stamps —
the VM's duplicated SQL error sites route through the SAME `sql_err`
helper as the interpreter). Structural guard: the crosscheck compares the
new try-code goldens between backends; `tests/naryad_385_try_codes.rs`
asserts exact codes on both backends for every stamped subsystem, the
honest fallback, the fail-closed seam, and the position-0 stamp format of
the runtime-sink twin (bytecode-level, the 328 trigger shape).

## 4. Consequences

- Agent scenarios branch on `r.error.code` (retry on `LLM_TIMEOUT`,
  hard-fail on `SANDBOX_VIOLATION`) without substring matching; the
  №327 integrity gate still applies to decisions over LLM-derived data —
  the sanctioned `redact(…, "hash_only")` lift makes code-branching
  legal (the golden office example demonstrates the pattern).
- Stamps are VISIBLE in messages (`[CODE] reason`) — nothing is hidden;
  top-level (uncaught) errors keep their loud stamps, unchanged.
- `try` around a Source-call failure yields an untrusted-derived result
  struct; branching on it goes through the sanctioned redact seam. This
  is the №327 lattice applied verbatim — no try-specific taint exemptions
  were introduced.
- Future codes (whisper/TTS transport, vision) append to the whitelist +
  this table with tests; the fallback keeps every unlisted failure honest.

## 5. Prior Art

- ADR-0131 (diagnostic code convention), ADR-0142/№374 (try shape),
  №254 (loud sandbox codes), №325 (sink clearance + runtime twin),
  №336/ADR-0165 (typed Degraded), №327 (integrity decisions),
  №126/№248/№156 (LLM deadline contours). Take/leave: Go `errors.As`/
  sentinel errors (typed channel — rejected here: the language's error
  channel is `String`-based by №106; the stamp protocol achieves the same
  discriminability without re-typing the channel); HTTP `Retry-After` /
  gRPC status codes (the retryable/unavailable split mirrors
  `UNAVAILABLE` vs `DEADLINE_EXCEEDED` semantics).

# ADR-0142: Error protocol — structural errors through try-extended semantics (candidate B)

**Status:** Accepted + IMPLEMENTED (owner decision 2026-09-14 — GO for research + ADR; priority candidate (b) — extending try-semantics; the final candidate choice — after the ADR). **Implemented in naryad #374** (2026-09-16): `try` → `Struct { ok, value, error }` on both backends via the shared builder `try_result_struct` (`src/interpreter/values.rs`); migration `== Unit` → `.ok == false`; golden examples migrated with no output changes.
**Date:** 2026-09-14
**Naryad:** #298 (issue #362, P1/adr — ERR_PROTOCOL)
**Amends:** ADR-0106 (Option/Result — not introduced, soft-failure remains the error model) — the error-struct is not the Option/Result types; ADR-0106 is not revisited, only annotated with a note.
**Precedent:** Naryad #91 (TryEval — try expr → Unit on error), ADR-0131/0140 (stable diagnostic codes — UPPER_SNAKE_CASE, no-reuse rule), ADR-0106 (soft-failure model).

## Context

Metalogos uses loud runtime errors (`Result<Value, String>`) — a deliberate default (ADR-0106). `try` (Naryad #91) returns `Unit` on error, losing the error information. For large agentic applications (Fosved: dept-handlers, chain patterns) this is a real pain — there is no way to handle an error in a structured way (code, message, position) without losing information.

Option/Result types are EXPLICITLY rejected (ADR-0106, Rejected). The naryad is NOT "add Result" (forbidden without a supersede) but research + ADR: a standardized error protocol via extending existing constructs (`try` #91).

Research doc: `docs/research/error-protocol-prior-art.md` — prior art (Go error values, Lua pcall, Erlang tagged tuples, Elm Result), the current state of Metalogos, 3 candidates, Fosved scenarios, breaking-surface assessment.

## Decision (3 candidates, priority (b))

### Candidate (a) — a standard error-struct + the `?` early-return operator
```mlog
let r = call_llm("...") ?  # early return on error
```
- A new `?` operator (grammar).
- A new error-struct shape: `{ code: String, message: String, span: Option<Struct> }`.
- Changed builtin signatures — returning Struct instead of String on error.
- **Breaking surface**: grammar + builtins + old code.
- **Rejected** by the owner as the priority (but not ruled out — the final choice comes after the ADR).

### Candidate (b) — extending try-semantics to structural errors (PRIORITY)
```mlog
let r = try call_llm("...")
# r = Struct { ok: Bool, value: String, error: Option<Struct> }
if r.ok then { respond("200", r.value) } else { respond("500", r.error.message) }
```
- `try` returns `Struct { ok: Bool, value: Value, error: Option<Struct{code, message}> }` instead of `Unit` on error.
- Grammar — UNCHANGED (try is already in the language).
- Builtins — UNCHANGED.
- Old code: `if try_result == Unit` — breaks (Unit ≠ Struct). Backward incompatibility.
- **Compatibility**: one could keep `try` as is (returns Unit) and introduce `try_struct` (or `try?`) as a new variant (returns Struct). But that = candidate (a).
- **Alternative**: change `try` to return a Struct, with backward-compat: `if result == Unit` → `Unit` is NOT `Struct { ok: false }` → old code breaks. Migration: `if result.ok` replaces `if result == Unit`.
- **Breaking surface**: only old code that checks `try x == Unit`. Minimal.
- **Synergy**: `code` field = ADR-0131/0140 stable codes (UPPER_SNAKE_CASE).
- **Effort**: ~1 naryad to implement (TryEval → TryEvalStruct opcode, VM dispatch, interpreter try-eval → Struct, tests).

### Candidate (c) — status quo + a documented pattern
- `try` stays as is (Unit on error).
- The error-handling pattern is documented.
- **Breaking surface**: zero.
- **Rejected** by the owner (does not solve the problem).

### D1. Selected candidate: (b) — extending try-semantics
- `try expr` → on error returns `Struct { ok: false, value: Unit, error: Some(Struct{ code: "RUNTIME_ERROR", message: "..." }) }`; on success — `Struct { ok: true, value: <result>, error: None }`.
- `code` field — stable per the ADR-0131/0140 convention (UPPER_SNAKE_CASE, no-reuse). For runtime errors — a new namespace category `"runtime"` (vs the existing `"security"`/`"semantic"`).
- `message` field — human-readable, may change freely (the code is the contract, not the prose — ADR-0131 principle).
- `span` field — optional, for source-position (future; not in v1).

### D2. Backward compatibility
- Old code: `if try_result == Unit` → breaks (Unit ≠ Struct).
- Migration path: `if result.ok` (or `if not result.ok` for the error branch).
- ADR-0106 is annotated: the error-struct is not the Option/Result types; the soft-failure model remains (empty string / Unit / false for optional paths); `try` with Struct is for critical-path error-handling, not a replacement for soft-failure.

### D3. Implementation — a SEPARATE naryad
- This ADR fixes the decision; implementation (opcode TryEvalStruct, VM dispatch, interpreter, tests) is a separate naryad after the ADR is accepted.
- Do not implement in this naryad (contract: research + ADR only).

## Consequences

- `try` stops returning `Unit` on error — it returns a `Struct`. Old code checking `try x == Unit` breaks. Migration: `if result.ok`.
- The error-struct with the `code` field — synergy with ADR-0131/0140 stable codes. Codes for runtime errors are new (namespace `"runtime"`) but follow the same UPPER_SNAKE_CASE + no-reuse discipline.
- The soft-failure model (ADR-0106) remains for optional paths (`kv_get`/`recall`/`query_param` → `""`). `try` with Struct is for critical-path error-handling.
- ADR-0106 is not canceled, only annotated (error-struct ≠ Option/Result; soft-failure remains; `try` is an extension, not a replacement).
- Implementation — a separate naryad (~1 naryad, per precedent #91).

## Addendum: ADR-0106 annotation

ADR-0106 §Decision (Option/Result — Rejected) remains in force. The error-struct (candidate (b)) is not an Option/Result type; it is:
- An extension of the existing `try` mechanics (Naryad #91) to return structured error information.
- A Struct value (already in the language), not a new type.
- The soft-failure model (empty string / Unit / false) remains for optional paths.
- Critical-path error-handling via `try` → Struct — a separate pattern, not a replacement for soft-failure.
- ADR-0106 "soft-failure remains the error model" — true for optional paths; for critical-path errors — `try` → Struct adds structured error-handling without introducing Result.

## Addendum (2026-09-22, issue #602): the STRING projection of a TryResult

The structural contract above is unchanged: `try expr` still returns
`Struct { ok: Bool, value: Value, error: Unit | Struct { code, message } }`,
and `.ok` / `.value` / `.error.code` / `.error.message` keep working on both
backends. What changes is the STRING projection only — the `Value` Display
impl shared by `to_string`, `print`, and string interpolation:

- **Success** (`ok = true`): the projection renders the `value` field —
  `to_string(try trim("hello"))` is `"hello"`, exactly the 0.19
  transparency on the success path.
- **Failure** (`ok = false`): the `value` field is `Unit`, so the projection
  renders `"()"` — the 0.19 failure probe idiom
  (`if to_string(x) == "()" { fallback }`) works again in both branches.

Rationale (issue #602, office migration 0.19 → 0.21): the office's verified
workaround `TryVal(x) = x.value` reproduced exactly this projection on both
paths; ~570 try-sites made a per-site migration the higher-risk option. The
projection does not weaken the stable-code contract (№385/ADR-0169): the
codes live in the `error` field, which remains structurally accessible, and
branching on `r.error.code` — not on the rendered string — stays the
sanctioned pattern. A generic struct dump remains available by rendering
individual fields explicitly.

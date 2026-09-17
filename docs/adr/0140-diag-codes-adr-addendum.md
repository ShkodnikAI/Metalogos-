# ADR-0140: Diagnostic codes — addendum (no-reuse rule + SSOT-registry discipline)

**Status:** Accepted
**Date:** 2026-09-14
**Naryad:** #288 (issue #302; ADR-only — no code, addendum to ADR-0131)
**Amends:** ADR-0131 (`Stable diagnostic codes for mlog check`) — accepted 2026-09-10, naryad #255
**Precedent:** ADR-0131 §Decision (format, a single convention, JSON shape), `BUILTIN_REGISTRY` (SSOT registry, naryad #170), ADR-0114 (`ReflexId` opaque handle — the template for "registry owns the data, caller carries an index")

## Context

ADR-0131 (accepted in naryad #255) fixed the core convention: `UPPER_SNAKE_CASE` codes, a single convention for `audit.rs` + `semantic.rs`, JSON output `{code, message, span, severity}`. The intake funnel (issue #302, verified by the formulator on main @ `e5eb2c8` — see the collision block in the issue) ruled questions 1–3 (format, a single registry, JSON output) **not open for reopening** — the answer was given in ADR-0131.

However, ADR-0131 does **not** fix two operational rules, without which the convention remains vulnerable to silent degradation over time:

1. **No-reuse rule (codes are reserved forever).** A code, once assigned to an error/finding, is **never reused** for a different meaning — even after the error is removed from the compiler. External tools (agent parsers, `zero fix --plan --json`-style repair tools, CI gates) may rely on the code silently; reusing it for a different meaning would break them with no visible regression in the compiler itself.
2. **SSOT-registry discipline (where the registry lives, how collisions are caught, how it changes).** ADR-0131 §Consequences mentioned a "registry of all diagnostic codes" as a principle but fixed neither its storage location nor its maintenance discipline. Without this, the registry either never appears (the convention exists, enforcement does not) or appears as scattered `match` arms across the whole compiler — exactly the "second system" ADR-0131 avoided.

This addendum closes both gaps — without reopening questions 1–3 and without superseding ADR-0131.

## Decision

### D1. No-reuse rule — codes are reserved forever

A code, once assigned to a diagnostic (via `check_id: "CODE"` in `audit.rs` or a future `diagnostic_code` mechanism in `semantic.rs`), is **never reused for a different meaning**. Removing an error from the compiler does **not** free the code — it stays in the registry marked `deprecated: true` (or `removed: true` with the removal version), and is not reassigned.

**Rationale.** External tools have no way to learn that code X now means something else — especially if the tool runs in CI against different language versions simultaneously. A silent reassignment → silent false positives (or real errors silently let through). The best case — the tool breaks loudly on unfamiliar code; the worst — it silently misbehaves.

**Precedent.** Rust (`E0XXX`), Clang (`-Wxxxx`), TypeScript (`TSXXXX`) — all follow this rule; Rust, for example, explicitly keeps the `E0XXX` codes of removed errors in `rustc_error_codes` marked as removed.

**Retirement procedure.** Removing an error from the compiler is accompanied by:
- A registry entry (see D2) marked `removed_in: <version>` + `replaced_by: Option<code>` (if the error was replaced by a new one with a different code — e.g., split in two).
- An explicit CHANGELOG entry: `diagnostic code X removed (replaced by Y | deprecated with no replacement)`.
- No reassignment of the same string to a new meaning.

### D2. SSOT-registry discipline — where it lives, how collisions are caught, how it changes

**Storage location.** The registry of all diagnostic codes is a separate module `src/diag_codes.rs` (new, not part of `audit.rs` — that one holds only Category A/B codes; the registry lives separately, just as `BUILTIN_REGISTRY` lives in `registry.rs`, not in `core.rs`). The registry is `const DIAG_CODES: &[DiagCodeSpec]` (template: `BUILTIN_REGISTRY`, naryad #170):
```rust
pub struct DiagCodeSpec {
    pub code: &'static str,         // UPPER_SNAKE_CASE
    pub message_template: &'static str, // human-readable, may evolve freely
    pub severity: Severity,         // Error | Warning | Info
    pub category: &'static str,     // "security" | "semantic" | "vm" | ...
    pub removed_in: Option<&'static str>, // None = active; Some("v0.20") = removed
    pub replaced_by: Option<&'static str>, // Some("NEW_CODE") if renamed/split
}
```
Same SSOT principle: a `spec!` macro or a `const &[]` literal, **append-only** (like `BUILTIN_REGISTRY`) — codes are not reordered; removal goes through `removed_in`/`replaced_by`, not by dropping from the list.

**How collisions are caught.** A test `tests/diag_codes_registry_check.rs` (to be created in the implementation naryad, not here):
- **Code uniqueness** — no duplicate `code` in the registry.
- **Cross-source consistency** — every `check_id: "X"` in `audit.rs` and every future `diagnostic_code: "Y"` in `semantic.rs`/`compiler.rs`/`vm.rs` must have a matching entry in `DIAG_CODES`. A collision (a code used in source but not in the registry, or in the registry but not in source) — a loud CI error.
- **No-reuse enforcement** — no `code` with status `removed_in: Some(_)` is used in active source code (a search via `grep -r "check_id:\s*\"$REMOVED_CODE\""` must return 0).

**How it changes.** Adding a code — append-only + the commit-message convention `feat(diag-codes): add CODE for <description>`. Removal — `removed_in` + `replaced_by` + a CHANGELOG entry. **Renaming** (allowed, but expensive) — `replaced_by: Some("NEW_CODE")`, both codes in the registry, the old one with `removed_in`, the new one active; the CHANGELOG records it.

### D3. Code categories — a single namespace, but a category label

Question 2 of the intake ("a single registry or separate namespaces for `semantic.rs`/`audit.rs`") is **not reopened**. ADR-0131 chose a single convention; this addendum confirms it but introduces a `category` field (`"security" | "semantic" | "vm" | "reflex" | "vision" | "voice" | ...`) in `DiagCodeSpec` for machine-readable category distinction within one registry. This addresses the original intake rationale ("semantically different categories — separate namespaces may be justified") via **subcategorization inside a single registry**, not via separate registries.

**A revisit toward "separate namespaces"** — only via an explicit supersede of ADR-0131 by the owner's decision, not the implementer's. Current position: a single registry + the category field is enough; separation would introduce exactly the "second system" ADR-0131 avoided.

### D4. Registry of known codes as of the addendum (snapshot)

17 unique `check_id` values in `audit.rs` on main `d3a1de5` (naryad #283 merge):

| Code | Category | Status |
|---|---|---|
| `CANARY_LEAK` | security (LLM) | active |
| `CSRF` | security (web) | active |
| `HTML_INJECTION` | security (web) | active |
| `MODEL_WEIGHTS_UNSAFE` | security (reflex) | active |
| `OPEN_REDIRECT` | security (web) | active |
| `RATE_LIMIT` | security (web) | active |
| `SANDBOX_COVERAGE` | security (sandbox) | active |
| `SECRETS` | security (sandbox) | active |
| `SECRET_LEAK` | security (audit) | active |
| `SQL_DYNAMIC` | security (audit) | active |
| `TAINT_PASSTHROUGH` | security (audit) | active |
| `TAINT_PERSISTENCE` | security (audit) | active |
| `UNTRUSTED_TRAINING_DATA` | security (reflex) | active |
| `VISION_POLICY_MISSING` | security (vision) | active |
| `VISION_PROMPT_USER_INPUT` | security (vision) | active |
| `VISION_UNSIGNED_EXPORT` | security (vision) | active |
| `VISION_UNSIGNED_EXPORT_RAW` | security (vision) | active |

All 17 are Category A/B security; ADR-0131 extends the convention to general `semantic.rs` diagnostics. The registry (D2), when created, automatically includes these 17 as the base; new `semantic.rs` codes are added append-only.

## Consequences

- **External tools** (agent parsers, repair tools, CI gates, linters) can rely on a diagnostic code as an **eternal contract** — the code is never reassigned to a different meaning; removing an error does not free the code for reuse.
- **The `DIAG_CODES` registry** — the single source of truth; collisions are caught on CI (implementation naryad, not this ADR).
- **Registry maintenance** — append-only + `removed_in`/`replaced_by` + a CHANGELOG entry, identical to the `BUILTIN_REGISTRY` discipline.
- **Categories within the registry** — via the `category` field, not separate namespaces (confirms ADR-0131 §Decision).
- **Implementation** (applying the codes to all `semantic.rs` errors + creating `src/diag_codes.rs` + `tests/diag_codes_registry_check.rs` + the `mlog check --json` flag) — a separate, next naryad after this ADR is accepted. This ADR fixes the convention, not the implementation.
- **ADR-0131 remains in force** — the addendum only adds two operational rules (no-reuse, SSOT-registry discipline); it does not revisit format / single-convention / JSON shape.

## Addendum-specific precedents

- **Rust `rustc_error_codes`** — a registry of all error codes; removed codes are kept with a mark, never reassigned. Template for D1.
- **`BUILTIN_REGISTRY` (naryad #170)** — an SSOT registry of the `spec!` macro, append-only, cross-source consistency via `registry_sync_check.rs`. Template for D2.
- **`BUILTIN_REGISTRY` reserved numbers (ADR-README.md §Reserved)** — `ADR-0073`/`ADR-0075`/`ADR-0076` reserved to avoid reassignment. Template for D1 (numbers reserved forever).

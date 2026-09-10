# ADR-0131: Stable diagnostic codes for `mlog check` — extending the existing `audit.rs` convention, not a new one

**Status:** Accepted
**Date:** 2026-09-10
**Naryad:** #255 (blocks on this ADR)
**Precedent:** `audit.rs`'s existing check-ids (`SECRET_LEAK`,
`VISION_UNSIGNED_EXPORT`, `MODEL_WEIGHTS_UNSAFE`, etc.) — already a
real, working, descriptive-name diagnostic-code system, just scoped
only to Category A/B security audit checks, not general
`semantic.rs` errors (undefined variable, type mismatch, arity
mismatch).

## Context

External research (verified against real repos, not taken on faith)
confirms structured, stable diagnostic codes are a converging
best practice for agent-legible compilers — Zero's `zero fix --plan
--json` cited as a working example: an agent can programmatically
parse and repair errors by code, not by scraping prose.

Metalogos already has exactly this pattern, just narrowly scoped:
every `audit.rs` finding carries a stable, descriptive SNAKE_CASE
identifier (`SECRET_LEAK`, `SQL_DYNAMIC`, `VISION_PROMPT_USER_INPUT`).
`semantic.rs`'s general compile errors (the much more common path —
every `mlog check` failure on ordinary code, not just security
findings) do not have this — they are prose strings, unstable across
wording changes, not machine-parseable by code.

## Decision

**Extend the existing descriptive-name convention to all of
`semantic.rs`'s diagnostics — do not introduce a second, competing
numeric scheme (Rust-style `E0001`) alongside it.** Two different
diagnostic-code styles in one compiler would genuinely confuse both
human readers and agent tooling about which convention applies where.

Format: `UPPER_SNAKE_CASE`, matching `audit.rs` exactly (e.g.
`UNDEFINED_VARIABLE`, `TYPE_MISMATCH`, `ARITY_MISMATCH`,
`DUPLICATE_DECLARATION`). Each diagnostic carries: the stable code,
a human-readable message (may still change wording freely — the
code is the stable contract, not the prose), and a source span
(naryад #111's `AST span tracking` already provides this — reused,
not reinvented).

**JSON output mode** for `mlog check` (`mlog check --json` or
similar flag, exact interface decided in naряд #255): a machine-
readable array of `{code, message, span, severity}` — the same
shape Zero's `--plan --json` demonstrates, adapted to Metalogos's
existing span-tracking infrastructure.

## Consequences

- `audit.rs`'s Category A/B codes and `semantic.rs`'s new general
  codes share one naming convention — an agent parsing Metalogos
  diagnostics does not need to learn two different systems.
- Existing prose error messages may be reworded freely without
  breaking anything that depends on the stable code — the code is
  the contract, not the exact sentence.
- A registry of all diagnostic codes (mirroring `BUILTIN_REGISTRY`'s
  SSOT discipline) prevents silent duplication of codes across
  `semantic.rs`/`audit.rs`/future pillars (`Reflex`/`Vision`/`Voice`
  each already emit their own domain-specific codes — `SECRET_LEAK`-
  class findings must not collide in meaning with a future Voice-
  specific code of the same name).
- Scope for naряд #255 is `semantic.rs` only — `compiler.rs`/`vm.rs`
  runtime errors are a separate, later naряд if genuinely needed, not
  bundled in here.

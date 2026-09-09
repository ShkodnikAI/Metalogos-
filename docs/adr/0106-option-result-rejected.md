# ADR-0106: `Option`/`Result` — not introduced, soft-failure remains the error model

> **Status:** Rejected  
> **Date:** 2026-08-21  
> **Decision by:** owner  
> **Precedent:** ADR-0105 (VM experimental scope — same "not without
> demonstrated need" principle)

## Context

An external audit (`Metalogos_Audit_and_Roadmap.md`) proposed
introducing `Option`/`Result` as a "modern, idiomatic" error-handling
model, without accounting for what was already decided in the
codebase.

`ADR-0006` (Fluid Types) directly quotes the language's principle:
*"soft-failure instead of exceptions"*. This is not a gap — it is a
deliberately chosen model, carried consistently through the entire
language: `to_float("abc")` → `0.0`, `read_file` on a non-existent
path → an empty string, `recall` with no match → `Unit`.

The real question is not "add `Result`", but one of three options:

**Option A** — replace soft-failure with `Result` system-wide. Cost:
revisiting the signatures of all 349 builtins, every golden example,
and reversing the `ADR-0006` principle.

**Option B** — add `Option`/`Result` alongside, soft-failure remains
for existing code. Cost: two incompatible ways to report an error at
the same time — confusion for whoever writes `.mlog` code.

**Option C** — do not introduce a new type system, resolve specific
cases point-by-point where indistinguishability between "not found"
and "execution error" genuinely gets in the way (for example, in
`recall` or `query_row`).

## Decision

**Neither Option A nor Option B is accepted.** There is no
demonstrated pain that justifies the cost of a system-wide
replacement of the error model, or of two models coexisting at once.

**Option C remains open**, but is not activated by this ADR — only
when a concrete, reproducible case is found where the current
indistinguishability genuinely causes a problem. At that point — a
separate, targeted naryad, not a general type system.

## Consequences

- Soft-failure remains the language's sole error-handling model.
- `Option`/`Result` do not appear in the grammar/AST/`Value` under
  this decision.
- Revisit only on a concrete case (not the abstract "modern languages
  need `Result`"), the same principle ADR-0105 applied to the VM.

## Related

- ADR-0006 — Fluid Types, the source of the soft-failure quote
- ADR-0105 — the same "don't fix without demonstrated demand" principle

# ADR-0107: A separate `Int` type — not introduced without functional necessity

> **Status:** Rejected  
> **Date:** 2026-08-21  
> **Decision by:** owner  
> **Precedent:** ADR-0105 (VM experimental scope)

## Context

An external audit flagged the absence of an integer type as a "primitive"
trait of the type system. Confirmed: `Value` has a single numeric
variant, `Float`; `to_int()` truncates the fractional part but still
returns a `Float`. `REFERENCE.md` states this explicitly as a decision,
not an oversight.

`Value::Float` is the sole numeric type across all 349 builtins,
every arithmetic operation in both the interpreter and the VM, and
every golden example. Introducing `Int` is not adding an enum
variant — it is revisiting every point where a number is created or
used: literals, builtin return types (does `len()` return `Int` or
`Float`?), JSON serialization, comparison in `rule`/`match`.

## Decision

**Do not introduce it.** The real cost of not having `Int` today is
stylistic (`42.0` in output instead of `42`), not functional: the
language does not operate on numbers requiring precision beyond `f64`
(cryptography is a separate layer, not routed through `Value`).

Revisit only on a concrete functional need (not "modern languages
need `Int`"). If it is decided to introduce it — not as adding a
variant, but as a separate ADR with an explicit choice of the
`Int`↔`Float` coercion model (three ready prior-art models:
Lua/JS-style single type, Python/Ruby-style auto-coercion,
Rust/OCaml-style with no auto-coercion).

## Consequences

- `Value` remains with a single numeric variant, `Float`.
- Revisit when a real functional case is demonstrated, not out of
  general completeness considerations.

## Related

- ADR-0105 — the same "don't fix without demonstrated demand" principle

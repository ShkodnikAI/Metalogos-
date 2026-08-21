# ADR-0105: Bytecode VM — experimental scope (not full-language equivalent)

> **Status:** Accepted  
> **Date:** 2026-08-21  
> **Naryads:** №108 (decision), №109 (documentation)  
> **Precedent:** ADR-0073 (JIT experimental scaffold)

## Context

Two confirmed gaps in the bytecode VM (verified on current `main`):

1. **`Match` is not compiled to VM at all.** The only handling in
   `compiler.rs` is an unconditional error. Any `.mlog` program using
   `match` inside a pattern cannot run under the VM — only under the
   tree-walking interpreter (TW).
2. **`BlockIfElse` silently degrades to `Unit`.** It compiles to
   `Const(Value::Unit)` regardless of the intended branch. The program
   does not fail; it produces the wrong result. Same class of defect as
   the pre-№91 `try` path (comment: «Наряд №14, deferred to tree-walking»),
   but `BlockIfElse` was never fixed.

Naryad №91 (fixing `try` via `TryEval`) showed the real cost of closing
one such gap: a new bytecode instruction, handlers in both VM dispatch
loops, success-path contracts, and regression coverage. Closing `Match`
and `BlockIfElse` would be at least two naryads of that calibre, with no
guarantee that further deferred constructs are not still hidden in the
compiler.

There is no confirmed production demand for full VM language coverage
today: FOSVED (the only real consumer) runs on TW; `METALOGOS_SERVE_BACKEND=vm`
is an explicit opt-in and is not the default path.

## Decision

1. **Tree-walking interpreter is the guaranteed, full-language backend.**
2. **Bytecode VM is experimental** for full-language use: it covers a large
   subset of the language and remains valuable for performance experiments
   and `crosscheck_backends` where both backends can run the program.
3. **Do not implement `Match` / `BlockIfElse` in the VM** under this ADR.
4. **Revisit only** under a real strategic need (e.g. demonstrated load where
   VM throughput matters and TW is the bottleneck) — **not** because leaving
   the gaps feels incomplete.

## Consequences

- `mlog serve` defaults to TW; `METALOGOS_SERVE_BACKEND=vm` prints an
  explicit warning about known limitations (naryad №109).
- Claims of “two backends with identical semantics for the whole language”
  are incorrect and must be restated (README, REFERENCE, crosscheck docs).
- `examples/p_match_switch.mlog` is a TW-visible contract (`.expected`) and
  is **explicitly excluded** from `crosscheck_backends` with a pointer to
  this ADR — not left silently unpaired.
- `crosscheck_backends` remains a blocking test for programs both backends
  can execute; it is not a claim of full-language parity.

## Related

- ADR-0073 — JIT experimental (same honesty principle)
- ADR-0086 — VM microbenchmark (arithmetic only; not full-server cost)
- Naryad №91 — cost precedent for one deferred-construct fix

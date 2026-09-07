# ADR-0121: Closing the VM-parity gap for `Reflex` — VM-owned state, not shared `RuntimeContext`

**Status:** Accepted
**Date:** 2026-09-06
**Naryad:** #199 (blocks on this ADR)
**Amends:** `ADR-0105` (accepted no confirmed case for full VM
coverage; that reasoning predates `Reflex` entirely — superseded for
`Reflex` specifically, not for the language's other TW/VM gaps, which
remain as `ADR-0105` left them)

## Context

15+ examples are excluded from `crosscheck_backends` — the entire
`Reflex` pillar (наряды №177–196). `builtin_reflex_*_stub` functions
in `src/vm.rs`'s dispatch explicitly refuse with "VM backend does not
yet support Reflex (ADR-0114)". The owner has explicitly decided to
close this gap — this ADR records **how**, not **whether**.

## The real technical question — and the answer already exists in the codebase

`Value::Reflex`'s state (`ReflexRegistry`) lives on `RuntimeContext`,
which the tree-walking interpreter threads through its own execution.
`src/vm.rs` has zero references to `RuntimeContext` — not "not wired
up," structurally absent as a concept.

**This is not a new problem.** `memorize`/`recall` (наряд №67/72) hit
the identical shape of question — memory state lives on the
interpreter's `MemoryStore`, and VM needed the same capability. The
answer that shipped: `Vm` struct owns its **own**, separate field
(`memory: Vec<VmMemoryEntry>`), with VM-side handlers achieving
parity with the interpreter's logic through independent implementation,
not through sharing the same instance. This pattern is proven,
shipped, and has run in production CI (`crosscheck`) for the entire
duration of naряды №67 onward.

## Decision

Follow the naряд №67/72 pattern exactly, not a new architecture:

```rust
pub struct Vm {
    // ... existing fields ...
    reflex_registry: crate::nn::ReflexRegistry,  // VM's own instance
}
```

VM's bytecode compiler gains real handling for `reflex_train`/
`reflex_predict`/`reflex_save`/`reflex_load`/`reflex_generate`/
`reflex_tokenize`/`reflex_detokenize`/`reflex_metrics`/`reflex_list`
(and BPE variants once наряд №195 lands) — replacing the `_stub`
dispatch with real calls into `Vm::reflex_registry`, using the exact
same underlying `src/nn/*` functions the interpreter already calls
(`ReflexModel::train`, `compute_accuracy`, etc.) — **the neural-network
logic itself is not reimplemented**, only the VM-side plumbing that
routes to it.

## Staging — mirrors how `Reflex` itself was built, not one naряд

| Stage | Scope | Naряд |
|---|---|---|
| 1 | `Vm.reflex_registry` field, `reflex` (classification) declaration + `reflex_train`/`reflex_predict` compiled and dispatched | №199 |
| 2 | `reflex_save`/`reflex_load` (persistence, наряд №180) | №200 |
| 3 | `reflex_seq` (attention/GQA/transformer stack, наряды №183–192) | №201 |
| 4 | `reflex_gen` (generation, KV-cache, наряд №193) | №202 |
| 5 | Tokenization builtins (наряд №194, and BPE from №195 once it lands) | №203 |
| 6 | Full `crosscheck_backends` re-enablement — remove all 15+ exclusions, confirm TW/VM parity on every one | №204 |

Each stage's exclusions in `crosscheck_backends` are removed **only**
when that stage's examples genuinely pass on both backends — not
removed in bulk at the end pending stage 6 alone.

**Update (2026-09-07, on merging №205):** the original stage→naryad
reservation above drifted: slots №200–203 were consumed by blocking
non-VM work (№200 candle-CI job, №201 learnable taint, №202 release
0.19.0, №203 consolidated P3), so the VM-parity stages landed as
**№204 (stages 2–5, PR #215)** and **№205 (stage 6, PR #216)**.
All six stages are merged; the 15+ `crosscheck_backends` exclusions
for Reflex are removed (residual skips are the pre-existing, non-Reflex
`match`/JIT gaps `ADR-0121`'s Consequences deliberately leave open).
The table above is preserved as the historical reservation record —
correction is recorded here, not by rewriting it.

## Consequences

- `ADR-0105`'s general reasoning (no confirmed case, FOSVED runs on
  TW) still holds for the language's *other* pre-existing TW/VM gaps
  (`match` expression, наряд №118's collection-utils case) — this ADR
  narrows the exception to `Reflex` specifically, not a blanket
  reversal.
- `candle`'s determinism (наряд №176's finding, load-bearing since
  наряд №177) must hold identically when invoked from VM's own
  `reflex_registry` — same seed, same weights, regardless of which
  backend trained the model. This becomes a new, explicit crosscheck
  requirement, not assumed.
- Six naряды, not one — matches the actual scope, avoids the наряд
  №179-style trap of claiming completion before the language-level
  plumbing is proven end-to-end on both backends.

# ADR-0114: `Value::Reflex` as an opaque handle, not a tensor type

**Status:** Accepted
**Date:** 2026-09-05
**Naryad:** #178 (blocks on this ADR)
**Pillar:** `Reflex` (eighth semantic pillar)

## Context

The `Reflex` pillar requires the interpreter to hold trained model
state (weights, layer shapes, an embedding-dimension contract)
between calls, addressable from `.mlog` source. `Value` currently has
no representation for mutable, stateful, non-serializable-by-default
data — every existing variant is either a plain scalar/collection or
an explicitly opaque, zeroizing handle (`Secret`, `Encrypted`,
`Hash`, `Session`).

Two representations were considered.

### Option A — a `Tensor` variant in `Value`

Weights live directly in `Value::Tensor(Vec<f32>, Vec<usize>)` (data
+ shape), passed and returned like any other value.

**Rejected.** This would require:
- A dtype/shape-checking layer that does not exist anywhere else in
  the type system — every other `Value` variant is checked at the
  point of use, not carried as a runtime contract that must be
  verified on every operation.
- Duplication across both backends (`src/interpreter/execution.rs`
  and `src/vm.rs`) for every tensor operation, doubling the
  TW/VM-parity surface that `crosscheck_backends` already has to
  cover.
- Serialization/`Debug`/`Display` handling for a type an order of
  magnitude larger than any existing `Value` payload — a single
  embedding is already 384–1536 `f32`s; a multi-layer model's
  weights are larger still. Every place that pattern-matches on
  `Value` today would need to account for a variant that can be
  megabytes wide.

### Option B — an opaque handle, `Value::Reflex(ReflexId)`

`ReflexId` is a `u32` index into a registry (`ReflexRegistry`) owned
by the runtime, not by `Value` itself. Weights, shapes, and training
metadata live in the registry; `Value` carries only the index.

**Accepted.** This mirrors the pattern already established for
`Secret`/`Encrypted`/`Hash`/`Session` — the value the language
manipulates is a reference, not the payload. The blast radius of a
mistake (wrong index, missing entry) is a single, well-defined
runtime error, not a type-system hole.

## Decision

`Value::Reflex(ReflexId)`. `ReflexId = u32`. Weights never enter
`Value`. `ReflexRegistry` lives on the runtime context shared between
both backends (see the `RuntimeContext` refactor tracked alongside
this pillar's staged rollout), so `predict`/`train` builtins have one
body, not two.

`Display` for `Value::Reflex` prints `[Reflex <name>]`. `Debug` is a
**manual implementation** printing only the model's declared name and
its last-measured metric (accuracy/loss), never weight contents —
matching the existing precedent set for `SecretString` (`наряд №19`
era: derived `Debug` on a type holding sensitive/bulky payload leaks
it into error text before any explicit sink check runs).

## Consequences

- Every builtin operating on a `Reflex` (`reflex_train`,
  `reflex_predict`, persistence) takes a `ReflexId` and resolves it
  against the shared registry — never receives or returns raw
  weights through `Value`.
- No new match arms are needed anywhere `Value` is exhaustively
  matched for size/serialization purposes; the pattern already
  exists for opaque types.
- A model's weights are represented at exactly one point in the
  codebase (the registry), not smeared across every function that
  happens to touch a `Value`.
- This does **not** by itself provide GPU acceleration, arbitrary
  tensor shapes, or a general autograd graph. Those remain out of
  scope for the pillar's initial stages (see the staged
  implementation plan) and would require their own ADR if pursued.

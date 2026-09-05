# ADR-0119: Extending the layer abstraction for sequence-processing blocks

**Status:** Accepted
**Date:** 2026-09-05
**Naryad:** #183 (blocks on this ADR)
**Pillar:** `Reflex` (stage 6+ — architecture blocks)
**Relates to:** `ADR-0114` (opaque handle, originally scoped to
single-vector classifier/regressor heads), `ADR-0118` (candle)

## Context

`src/nn/layer.rs`'s `Layer` trait (наряд №178):
```rust
fn forward(&self, input: &[f64]) -> Vec<f64>;
```
operates on one flat feature vector — correct and sufficient for
`Dense`-based classification (наряды №177–182, already shipped,
already load-bearing for `ADR-0117`'s backward-compatibility
guarantee). Attention fundamentally operates on a **sequence** of
vectors (shape `[seq_len, hidden_dim]`) — self-attention computes
relationships *between* positions, which this signature cannot
express without a breaking change.

Two options were considered for accommodating this.

### Option A — generalize `Layer` itself to always operate on sequences

Change the trait to `forward(&self, input: &[Vec<f64>]) -> Vec<Vec<f64>>`
uniformly; a classification head becomes the degenerate case of a
sequence with length 1. This is closer to how production ML
frameworks represent things internally, but it means touching the
signature every existing `Dense`-based test (наряды №177–182)
already depends on — real risk to a guarantee (`ADR-0117`'s
backward-compatibility contract) that is not this stage's to spend.

**Rejected** for this reason specifically: the project has a
demonstrated, repeated pattern today (наряд №182's own principle —
"не менять численный алгоритм... только структурно раздельны") of
not touching already-shipped, correct code to accommodate a new,
separate need. Generalizing `Layer` risks exactly that.

### Option B — a separate trait for sequence-processing blocks

`Layer` (single-vector) stays untouched. A new trait,
`SequenceLayer`, is introduced specifically for attention/transformer-
style blocks:
```rust
pub trait SequenceLayer: Send + Sync + std::any::Any {
    fn forward(&self, input: &candle_core::Tensor) -> Result<candle_core::Tensor, String>;
    fn input_dim(&self) -> usize;
}
```
using `candle_core::Tensor` directly (per `ADR-0118`) rather than a
hand-rolled `Vec<Vec<f64>>`, since sequence-shaped computation is
exactly what `candle` exists to do efficiently and correctly — naryад
№176 already confirmed `candle-transformers`' reference
implementations use this exact representation internally.

**Accepted.**

## Decision

`SequenceLayer` (Option B). `Value::Reflex(ReflexId)` (`ADR-0114`)
remains the single opaque-handle representation for *both* kinds of
model — the registry entry itself carries whichever internal form
(`Vec<Box<dyn Layer>>` or `Vec<Box<dyn SequenceLayer>>`) matches how
it was declared. `LAYER_REGISTRY` (`ADR-0114`'s addendum) gains a
parallel `SEQUENCE_LAYER_REGISTRY` for attention-family blocks — the
same registry-not-grammar extensibility principle, applied to the new
category, not a special case invented for it.

A `reflex` declaration is either a classification/regression head
(`Dense`-only layers, naряды №177–182's existing path, unchanged) or
a sequence model (`SequenceLayer`-only), determined by which registry
its first `layers` entry resolves against — **mixing the two
categories in one declaration is a compile-time error** in this
stage, not silently permitted. Whether a future stage should support
mixing them is an open question for a later ADR, not decided here.

## Consequences

- Naряды №177–182's `Dense`/`Layer` path is provably untouched — no
  existing test's signature changes.
- Sequence models genuinely depend on `candle` (`ADR-0118`); pure
  classification `Reflex` declarations still do not.
- `reflex_predict`/`reflex_train` (наряд №179/179b) need a dispatch
  branch on which category a given `ReflexId` holds — this is new
  code in the builtins, not a change to the existing branch's logic.
- `ADR-0117`'s "generation is a structural non-goal" statement was
  about the *initial rollout's* `Reflex` (classification-only,
  `Dense`). This ADR does not contradict it — it defines a
  genuinely separate model category within the same pillar, gated
  behind the owner's explicit stage-6+ authorization, exactly as
  `ADR-0117` itself anticipated ("any future work in that direction
  is наряд №176's domain... requires its own, separate owner
  decision").

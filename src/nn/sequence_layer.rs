//! Sequence-processing layers for architecture-block extension (Наряд №183).
//!
//! Implements `ADR-0119`: a separate trait `SequenceLayer` for
//! attention/transformer-family blocks that operate on **sequences**
//! of vectors (shape `[seq_len, hidden_dim]`), distinct from `Layer`
//! (наряд №178) which operates on single flat feature vectors.
//!
//! ## Why a separate trait (not generalizing `Layer`)
//!
//! Per `ADR-0119` Option B: generalizing `Layer` to always operate on
//! sequences would touch every existing `Dense`-based test (наряды
//! №177–182) and risk `ADR-0117`'s backward-compatibility guarantee.
//! The project's repeated pattern (наряд №182's own principle — "не
//! менять численный алгоритм, только структурно раздельны") is not to
//! touch already-shipped, correct code for a new, separate need.
//!
//! ## Why `candle_core::Tensor` directly
//!
//! Per `ADR-0118`: sequence-shaped computation is what `candle` exists
//! to do efficiently. Using `Vec<Vec<f64>>` would re-implement what
//! `candle` already provides (BLAS-backed matmul, broadcasting,
//! autograd). Наряд №176 confirmed `candle-transformers`' reference
//! implementations use this exact representation internally.
//!
//! ## Registry (ADR-0114 addendum principle)
//!
//! `SEQUENCE_LAYER_REGISTRY` is parallel to `LAYER_REGISTRY` (наряд
//! №178) — same registry-not-grammar extensibility principle. New
//! sequence-layer types (attention in this naryad; future: RMSNorm,
//! SwiGLU, full transformer block) are added by:
//!
//!   1. Implementing `SequenceLayer` trait in a new module.
//!   2. Adding a `SequenceLayerSpec` entry to `SEQUENCE_LAYER_REGISTRY`.
//!
//! No grammar change needed — `layer_spec = { IDENT ~ "(" ~ args ")" }`
//! (наряд №178) resolves the layer name at `mlog check` time via this
//! registry.
//!
//! ## Feature gating
//!
//! The entire module is gated behind the `candle` feature flag (off by
//! default). When `candle` is not enabled, `reflex_seq` declarations
//! produce a clean "feature not enabled" error at parse/check time,
//! rather than failing to compile the binary.
//!
//! Наряд №183 ships only `attention` — the first, self-contained
//! `SequenceLayer`. The naryad spec explicitly defers: full transformer
//! block (RmsNorm/SwiGLU/residual), GQA, and integration with
//! `reflex_train`/`reflex_predict`.

// The whole module is feature-gated. When `candle` is off, this file
// compiles to nothing — `reflex_seq` declarations fail at check time
// with a clean error message naming the missing feature.
#![cfg(feature = "candle")]

use crate::interpreter::Value;

/// Type alias for the build function signature.
/// Takes `&[Value]` (the same shape as `LayerSpec::build` from наряд №178)
/// so the grammar's `layer_spec = { IDENT ~ "(" ~ args ")" }` works
/// uniformly for both registries.
pub type SequenceLayerBuildFn =
    fn(args: &[Value], seed: u64) -> Result<Box<dyn SequenceLayer>, String>;

/// A sequence-processing layer — operates on `[seq_len, hidden_dim]`
/// tensors (per `ADR-0119`).
///
/// Distinct from `Layer` (наряд №178, single `&[f64]` input):
/// attention fundamentally operates on a *sequence* of vectors —
/// self-attention computes relationships *between* positions, which
/// the single-vector signature cannot express.
pub trait SequenceLayer: Send + Sync + std::any::Any {
    /// Forward pass: input tensor → output tensor.
    ///
    /// Input shape: `[seq_len, input_dim]`.
    /// Output shape: `[seq_len, output_dim]` (typically `output_dim == input_dim`
    /// for attention blocks; the residual stream is preserved across layers).
    ///
    /// Returns `Result<Tensor, String>` (not `Tensor` directly) because
    /// `candle`'s tensor operations are fallible (shape mismatches,
    /// device errors) — propagating errors as Strings matches the
    /// project-wide convention used by `Layer`'s callers and by the
    /// builtins (`reflex_train` etc.).
    fn forward(&self, input: &candle_core::Tensor) -> Result<candle_core::Tensor, String>;

    /// Input dimension (the `hidden_dim` the layer expects).
    /// Used for shape validation at construction time.
    fn input_dim(&self) -> usize;

    /// Output dimension. For most attention blocks this equals `input_dim`
    /// (residual stream is preserved), but the trait allows for projections.
    fn output_dim(&self) -> usize;

    /// Layer type name (e.g. "attention") for debugging and error messages.
    fn name(&self) -> &str;

    /// Upcast to `Any` for downcasting to concrete types (Attention, etc.)
    /// — same pattern as `Layer::as_any` (наряд №178).
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Specification for a sequence-layer type — analogous to `LayerSpec`
/// (наряд №178). The registry is the single source of truth for
/// available sequence-layer types.
pub struct SequenceLayerSpec {
    /// Layer type name (e.g. "attention", "rmsnorm" in future naryads).
    pub name: &'static str,
    /// Parameter names in order (e.g. `&["heads", "dim"]`).
    /// Used for error messages and documentation.
    pub param_names: &'static [&'static str],
    /// Build function: takes Value args + seed, returns a boxed SequenceLayer.
    /// The seed is for deterministic weight init (xorshift64 from наряд №177,
    /// reused so the same declaration with the same seed always produces
    /// the same forward-pass result — Наряд №183 Contract 5).
    pub build: SequenceLayerBuildFn,
}

/// The sequence-layer registry — extensible without grammar changes
/// (ADR-0114 addendum principle, applied to the new category).
///
/// Наряд №183 ships only `attention`. Future naryads add more entries
/// here (rmsnorm, swiglu, full transformer block, GQA if/when authorized).
pub static SEQUENCE_LAYER_REGISTRY: &[SequenceLayerSpec] = &[SequenceLayerSpec {
    name: "attention",
    param_names: &["heads", "dim"],
    build: crate::nn::attention::build_attention,
}];

/// Look up a sequence-layer spec by name. Returns None if not found.
pub fn find_sequence_layer_spec(name: &str) -> Option<&'static SequenceLayerSpec> {
    SEQUENCE_LAYER_REGISTRY.iter().find(|s| s.name == name)
}

/// List all registered sequence-layer names (for error messages).
pub fn sequence_layer_names() -> Vec<&'static str> {
    SEQUENCE_LAYER_REGISTRY.iter().map(|s| s.name).collect()
}

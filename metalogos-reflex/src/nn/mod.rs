//! The reflex domain's NN machinery — the generative contour's home
//! (ADR-0178 made physical; naryad №545 (в), issue #883).
//!
//! The ten modules below moved 1:1 from the language crate
//! (`src/nn/*`); the №463 stop-list manifest entries relocated in the
//! same PR (the baseline does not grow — the movement is the split
//! itself, ADR-0178 §4 needs no expansion patch). The language crate
//! re-exports these modules so every `crate::nn::*` consumer path is
//! preserved (the shell, `metalogos` nn/mod.rs).
//!
//! Gating mirrors the language crate 1:1: `bpe` and `metric` are
//! ungated (pure std), the machinery is behind `candle` (off by
//! default; the workspace feature unification keeps the
//! default/portable/full profiles coherent).

pub mod bpe;
pub mod metric;

#[cfg(feature = "candle")]
pub mod attention;
#[cfg(feature = "candle")]
pub mod gen_model;
#[cfg(feature = "candle")]
pub mod rmsnorm;
#[cfg(feature = "candle")]
pub mod seq_model;
#[cfg(feature = "candle")]
pub mod sequence_layer;
#[cfg(feature = "candle")]
pub mod swiglu;
#[cfg(feature = "candle")]
pub mod trainable_attention;
#[cfg(feature = "candle")]
pub mod trainable_transformer_block;
#[cfg(feature = "candle")]
pub mod transformer_block;

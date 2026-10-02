//! The reflex model HANDLE CONTRACTS (№545 б, sub-step б of the crate
//! split) — the DbAccess №484 precedent applied at the crate seam.
//!
//! The language crate (the registry in `nn/mod.rs`, the builtins in
//! `builtins/reflex.rs`) programs against these two traits, NOT against
//! the concrete machinery structs. The physical move (№545 (в)) relocates
//! the machinery (seq_model/gen_model and the layer family) into the
//! `metalogos-reflex` crate while THIS FILE STAYS in the language crate:
//! the contract belongs to the consumer. The impls below delegate to the
//! concrete types; after the move they resolve through the re-export
//! shell (`crate::nn::seq_model` → the reflex crate) UNCHANGED.
//!
//! Object safety: the registry holds `Box<dyn SequenceModelHandle>` /
//! `Box<dyn GenModelHandle>` as the `ModelKind` payloads, so the
//! language-side match arms never name the concrete candle-backed
//! structs beyond the construction boundary (`build_reflex_*_model`).

#![cfg(feature = "candle")]

use crate::nn::gen_model::ReflexGenModel;
use crate::nn::seq_model::ReflexSeqModel;
use candle_core::Tensor;

/// The handle contract for the sequence-classification model
/// (`reflex_seq`). The consumers: the `ModelKind::Sequence` payload
/// users — `reflex_train`/`reflex_predict`/`reflex_info` dispatch and
/// the registry's Debug.
pub trait SequenceModelHandle: Send + Sync {
    fn name(&self) -> &str;
    fn seq_len(&self) -> usize;
    fn input_dim(&self) -> usize;
    fn labels(&self) -> &[String];
    fn seq_layer_count(&self) -> usize;
    fn last_metric(&self) -> Option<f64>;
    fn train(
        &mut self,
        inputs: &[Tensor],
        target_classes: &[usize],
        epochs: usize,
        learning_rate: f64,
    ) -> Result<(f64, f64), String>;
    fn predict_probs(&self, input: &Tensor) -> Result<Vec<f32>, String>;
}

impl SequenceModelHandle for ReflexSeqModel {
    fn name(&self) -> &str {
        &self.name
    }
    fn seq_len(&self) -> usize {
        self.seq_len
    }
    fn input_dim(&self) -> usize {
        self.input_dim
    }
    fn labels(&self) -> &[String] {
        &self.labels
    }
    fn seq_layer_count(&self) -> usize {
        self.seq_layers.len()
    }
    fn last_metric(&self) -> Option<f64> {
        self.last_metric
    }
    fn train(
        &mut self,
        inputs: &[Tensor],
        target_classes: &[usize],
        epochs: usize,
        learning_rate: f64,
    ) -> Result<(f64, f64), String> {
        ReflexSeqModel::train(self, inputs, target_classes, epochs, learning_rate)
    }
    fn predict_probs(&self, input: &Tensor) -> Result<Vec<f32>, String> {
        ReflexSeqModel::predict_probs(self, input)
    }
}

/// The handle contract for the text-generation model (`reflex_gen`).
/// The consumers: the `ModelKind::Gen` payload users —
/// `reflex_generate`/`reflex_info` dispatch and the registry's Debug.
pub trait GenModelHandle: Send + Sync {
    fn name(&self) -> &str;
    fn input_dim(&self) -> usize;
    fn seq_layer_count(&self) -> usize;
    fn vocab_size(&self) -> usize;
    fn generate_greedy(&self, prompt: &[u32], max_tokens: usize) -> Result<Vec<u32>, String>;
    fn generate_with_temperature(
        &self,
        prompt: &[u32],
        max_tokens: usize,
        temperature: f64,
    ) -> Result<Vec<u32>, String>;
}

impl GenModelHandle for ReflexGenModel {
    fn name(&self) -> &str {
        &self.name
    }
    fn input_dim(&self) -> usize {
        self.input_dim
    }
    fn seq_layer_count(&self) -> usize {
        self.seq_layers.len()
    }
    fn vocab_size(&self) -> usize {
        self.vocab_size
    }
    fn generate_greedy(&self, prompt: &[u32], max_tokens: usize) -> Result<Vec<u32>, String> {
        ReflexGenModel::generate_greedy(self, prompt, max_tokens)
    }
    fn generate_with_temperature(
        &self,
        prompt: &[u32],
        max_tokens: usize,
        temperature: f64,
    ) -> Result<Vec<u32>, String> {
        ReflexGenModel::generate_with_temperature(self, prompt, max_tokens, temperature)
    }
}

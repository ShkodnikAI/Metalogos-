//! `ReflexSeqModel` — sequence-classification model (Наряд №185).
//!
//! Extends the Reflex pillar to sequence inputs: classifies a whole
//! sequence `[seq_len, dim]` into one label from a closed set (per
//! ADR-0117 §3, symmetric to plain `reflex`).
//!
//! ## Architecture
//!
//! ```text
//! input [seq_len, dim]
//!   ↓
//! SequenceLayer::forward (attention, etc.) — autograd-tracked
//!   ↓
//! output [seq_len, dim]
//!   ↓
//! Pooling (mean over positions) → [dim]
//!   ↓
//! Linear classifier (dim → labels.len()) → logits
//!   ↓
//! softmax (in cross-entropy loss)
//! ```
//!
//! ## Pooling choice: mean (not CLS)
//!
//! Mean pooling is used (not CLS-style first-position) for these reasons:
//!
//! 1. **No special CLS token** in the `embedding(dim)` declaration —
//!    the grammar doesn't expose a way to mark one position as special.
//!    CLS-style requires either (a) prepending a learned CLS embedding
//!    (extra parameter, extra grammar surface) or (b) treating
//!    position 0 as implicitly special (unjustified — position 0 is
//!    arbitrary under RoPE).
//!
//! 2. **Symmetric over positions** — mean pooling gives every position
//!    equal weight, which matches the task: classify the whole sequence.
//!    CLS-style asks the model to compress everything into one position
//!    via attention, which is harder to learn and requires more data.
//!
//! 3. **Standard for sentence-transformers** — Reimers & Gurevych 2019
//!    (SBERT) showed mean pooling outperforms CLS-style for
//!    classification of sentence-level inputs. The same principle applies
//!    here: classification of the whole sequence, not generation.
//!
//! 4. **No grammar change** — mean pooling is parameter-free; CLS-style
//!    would require either a new field `cls_token: true/false` or
//!    implicit convention. Mean is the simpler default.
//!
//! ## Training (Block 3 — candle autograd)
//!
//! Per the naryad spec: "не реализовывать backward вручную, использовать
//! candle's autograd напрямую" (ADR-0118). All weights are `Var`s
//! (candle's gradient-tracked tensors), registered in a `VarMap`. The
//! forward pass builds a computation graph; `loss.backward()` produces
//! gradients; SGD step applies them via `var.set(&var.as_tensor() - &(grad * lr)?)`.
//!
//! This is the standard candle-nn training pattern — no manual Jacobian
//! computation, no manual chain-rule. The same pattern candle-transformers
//! uses internally (verified in Наряд №176's reference analysis).

#![cfg(feature = "candle")]

use crate::nn::metric::compute_accuracy;
use crate::nn::sequence_layer::SequenceLayer;

use candle_core::{DType, Device, Tensor};
use candle_nn::{VarBuilder, VarMap};

/// Pooling strategy for converting `[seq_len, dim]` → `[dim]`.
///
/// Mean is the default (see module docs for justification). CLS-style
/// (first position) is included for completeness/future experimentation
/// but not exposed in the grammar yet — the choice is internal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pooling {
    Mean,
    First,
}

/// A sequence-classification model.
///
/// Holds:
///   - `var_map`: the trainable parameters (Q/K/V/O of attention, classifier W/b)
///   - `seq_layers`: boxed SequenceLayer trait objects (attention, etc.)
///   - `labels`: closed-set classification targets
///   - `seq_len`, `input_dim`: shape invariants
///   - `pooling`: mean (default) or first
///
/// Note: `classifier_w` and `classifier_b` are stored as `Tensor` (the
/// autograd-tracked view returned by `VarBuilder::get`). The underlying
/// `Var`s live in `var_map` — accessed via `var_map.all_vars()` during
/// the SGD step.
pub struct ReflexSeqModel {
    pub name: String,
    pub var_map: VarMap,
    pub seq_layers: Vec<Box<dyn SequenceLayer>>,
    /// Classifier weights: `[input_dim, labels.len()]`. Tensor view of
    /// the Var registered in `var_map` under name "classifier_w".
    pub classifier_w: Tensor,
    /// Classifier bias: `[labels.len()]`. Tensor view of the Var
    /// registered in `var_map` under name "classifier_b".
    pub classifier_b: Tensor,
    pub labels: Vec<String>,
    pub seed: u64,
    pub seq_len: usize,
    pub input_dim: usize,
    pub pooling: Pooling,
    pub last_metric: Option<f64>,
}

impl std::fmt::Debug for ReflexSeqModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReflexSeqModel")
            .field("name", &self.name)
            .field("seq_len", &self.seq_len)
            .field("input_dim", &self.input_dim)
            .field("labels", &self.labels)
            .field("seed", &self.seed)
            .field("pooling", &self.pooling)
            .field("last_metric", &self.last_metric)
            .field("num_seq_layers", &self.seq_layers.len())
            .finish_non_exhaustive()
    }
}

impl ReflexSeqModel {
    /// Construct from a `ReflexSeqDecl`-equivalent spec.
    ///
    /// `var_map` and `vb` are passed in so that the trainable sequence
    /// layers (e.g. TrainableAttention) and the classifier weights all
    /// live in the SAME VarMap — this is required for `backward()` to
    /// find all gradients.
    ///
    /// Weights are initialized deterministically from `seed` via the
    /// project's xorshift64 PRNG (same algorithm as Dense / Attention).
    pub fn new(
        name: String,
        input_dim: usize,
        seq_len: usize,
        labels: Vec<String>,
        seed: u64,
        seq_layers: Vec<Box<dyn SequenceLayer>>,
        var_map: VarMap,
        vb: &VarBuilder,
    ) -> Result<Self, String> {
        if labels.is_empty() {
            return Err(format!(
                "reflex_seq '{}': labels cannot be empty (ADR-0117 §3)",
                name
            ));
        }
        if seq_layers.is_empty() {
            return Err(format!(
                "reflex_seq '{}': at least one sequence layer is required",
                name
            ));
        }

        // Classifier weights: [input_dim, num_labels]
        // Init: deterministic via xorshift64 (NOT VarBuilder's Init::Uniform,
        // which uses candle's non-seedable CPU RNG — breaks determinism).
        // Same pattern as TrainableAttention: build Tensor manually,
        // convert to Var, insert into var_map.
        let bound = 1.0 / (input_dim as f64).sqrt();
        let num_labels = labels.len();
        let total_classifier_w = input_dim * num_labels;
        // Different seed offset so classifier RNG stream doesn't collide
        // with attention weights' stream.
        let classifier_seed = seed ^ 0x4C415353; // "LASS"
        let w_init_values = crate::nn::attention::generate_uniform_f32(
            classifier_seed,
            total_classifier_w,
            -bound,
            bound,
        );
        let b_init_values = vec![0.0f32; num_labels];

        let device = Device::Cpu;
        let w_tensor = Tensor::from_slice(&w_init_values, (input_dim, num_labels), &device)
            .and_then(|t| t.to_dtype(DType::F32))
            .map_err(|e| format!("reflex_seq classifier_w tensor: {}", e))?;
        let b_tensor = Tensor::from_slice(&b_init_values, num_labels, &device)
            .and_then(|t| t.to_dtype(DType::F32))
            .map_err(|e| format!("reflex_seq classifier_b tensor: {}", e))?;

        // Convert to Var and insert into var_map so backward() populates
        // gradients for these weights too. Use map_err to avoid unwrap
        // (clippy: would be `unwrap()` on a `Result`).
        let w_var = candle_core::Var::from_tensor(&w_tensor)
            .map_err(|e| format!("reflex_seq classifier_w Var: {}", e))?;
        let b_var = candle_core::Var::from_tensor(&b_tensor)
            .map_err(|e| format!("reflex_seq classifier_b Var: {}", e))?;
        if let Err(e) = var_map.data().lock() {
            return Err(format!("reflex_seq var_map lock (w): {}", e));
        }
        {
            let mut guard = var_map
                .data()
                .lock()
                .map_err(|e| format!("reflex_seq var_map lock (w): {}", e))?;
            guard.insert("classifier_w".to_string(), w_var);
        }
        {
            let mut guard = var_map
                .data()
                .lock()
                .map_err(|e| format!("reflex_seq var_map lock (b): {}", e))?;
            guard.insert("classifier_b".to_string(), b_var);
        }

        // Get the Tensor views (these track gradients via the Var).
        let classifier_w = vb
            .get((input_dim, num_labels), "classifier_w")
            .map_err(|e| format!("reflex_seq classifier_w get: {}", e))?;
        let classifier_b = vb
            .get(num_labels, "classifier_b")
            .map_err(|e| format!("reflex_seq classifier_b get: {}", e))?;

        Ok(Self {
            name,
            var_map,
            seq_layers,
            classifier_w,
            classifier_b,
            labels,
            seed,
            seq_len,
            input_dim,
            pooling: Pooling::Mean,
            last_metric: None,
        })
    }

    /// Forward pass: `[seq_len, dim]` → logits `[num_labels]`.
    ///
    /// This is autograd-tracked — calling `.backward()` on a downstream
    /// loss will populate gradients in `self.var_map`.
    pub fn forward(&self, input: &Tensor) -> Result<Tensor, String> {
        let (_seq, in_dim) = input
            .dims2()
            .map_err(|e| format!("seq forward: dims: {}", e))?;
        if in_dim != self.input_dim {
            return Err(format!(
                "reflex_seq '{}': input dim {} != model dim {}",
                self.name, in_dim, self.input_dim
            ));
        }

        // Run through all sequence layers (attention, etc.)
        let mut current = input.clone();
        for layer in &self.seq_layers {
            current = layer.forward(&current)?;
        }

        // Pool: [seq_len, dim] → [dim]
        let pooled = match self.pooling {
            Pooling::Mean => {
                // mean over axis 0
                current
                    .mean(0)
                    .map_err(|e| format!("seq pool mean: {}", e))?
            }
            Pooling::First => {
                // take position 0
                current
                    .narrow(0, 0, 1)
                    .map_err(|e| format!("seq pool first: {}", e))?
                    .squeeze(0)
                    .map_err(|e| format!("seq pool first squeeze: {}", e))?
            }
        };

        // Classifier: pooled @ W + b → [num_labels]
        // pooled: [dim], W: [dim, num_labels] → need pooled unsqueezed to [1, dim]
        let pooled_row = pooled
            .unsqueeze(0)
            .map_err(|e| format!("seq forward unsqueeze: {}", e))?;
        let logits = pooled_row
            .matmul(&self.classifier_w)
            .map_err(|e| format!("seq forward matmul: {}", e))?;
        let logits = logits
            .broadcast_add(&self.classifier_b)
            .map_err(|e| format!("seq forward bias: {}", e))?;

        // Squeeze the batch dim: [1, num_labels] → [num_labels]
        logits
            .squeeze(0)
            .map_err(|e| format!("seq forward squeeze: {}", e))
    }

    /// Train the model on the given dataset.
    ///
    /// Returns `(train_loss, holdout_accuracy)`.
    ///
    /// Per Наряд №185 Block 3: uses candle autograd directly —
    /// `loss.backward()` populates gradients in `var_map`, then SGD
    /// applies them. No manual Jacobian.
    ///
    /// ## Algorithm
    ///
    /// 1. Split data 80/20 by deterministic xorshift64 shuffle (same
    ///    algorithm as `ReflexModel::train` — Наряд №179).
    /// 2. For each epoch, for each sample:
    ///    a. Forward pass → logits
    ///    b. Cross-entropy loss
    ///    c. `loss.backward()` → gradients
    ///    d. SGD: `var = var - lr * grad`
    /// 3. Compute holdout accuracy.
    pub fn train(
        &mut self,
        inputs: &[Tensor],
        target_classes: &[usize],
        epochs: usize,
        learning_rate: f64,
    ) -> Result<(f64, f64), String> {
        // Block 4: minimum dataset size (ADR-0115, symmetric to Наряд №179)
        if inputs.len() < 10 {
            return Err(format!(
                "reflex_seq train: need at least 10 examples for meaningful holdout, got {}",
                inputs.len()
            ));
        }

        // Deterministic 80/20 split — same algorithm as ReflexModel::train
        let indices = deterministic_split(inputs.len(), self.seed);
        let train_idx: Vec<usize> = indices
            .iter()
            .filter(|(_, is_train)| *is_train)
            .map(|(i, _)| *i)
            .collect();
        let holdout_idx: Vec<usize> = indices
            .iter()
            .filter(|(_, is_train)| !*is_train)
            .map(|(i, _)| *i)
            .collect();

        let mut last_loss = 0.0f64;
        let lr = learning_rate as f32;

        for _epoch in 0..epochs {
            let mut epoch_loss_sum = 0.0f64;
            let mut epoch_count = 0usize;

            for &idx in &train_idx {
                let input = &inputs[idx];
                let target = target_classes[idx];

                // Forward
                let logits = self.forward(input)?;
                // logits: [num_labels]; need [1, num_labels] for cross-entropy
                let logits_b = logits
                    .unsqueeze(0)
                    .map_err(|e| format!("train unsqueeze: {}", e))?;

                // Cross-entropy loss — returns a scalar Tensor.
                // Convert to f64 for aggregation across the epoch.
                let loss_tensor = cross_entropy_seq_loss(&logits_b, target)?;
                let loss = loss_tensor
                    .to_scalar::<f32>()
                    .map_err(|e| format!("train loss to_scalar: {}", e))?
                    as f64;
                epoch_loss_sum += loss;
                epoch_count += 1;

                // Backward — candle autograd populates gradients in var_map
                let grads = loss_tensor
                    .backward()
                    .map_err(|e| format!("train backward: {}", e))?;

                // SGD step: var = var - lr * grad (for each var in var_map)
                let all_vars = self.var_map.all_vars();
                for var in &all_vars {
                    if let Some(grad) = grads.get(var.as_tensor()) {
                        // lr is f32; broadcast_mul needs matching shapes.
                        // Build a scalar tensor and broadcast to grad's shape.
                        let lr_scalar = Tensor::new(lr, &Device::Cpu)
                            .map_err(|e| format!("train lr_scalar: {}", e))?;
                        let lr_tensor = lr_scalar
                            .broadcast_as(grad.shape())
                            .map_err(|e| format!("train lr broadcast: {}", e))?;
                        let scaled_grad = grad
                            .broadcast_mul(&lr_tensor)
                            .map_err(|e| format!("train scale grad: {}", e))?;
                        let new_val = var
                            .as_tensor()
                            .sub(&scaled_grad)
                            .map_err(|e| format!("train sub: {}", e))?;
                        var.set(&new_val).map_err(|e| format!("train set: {}", e))?;
                    }
                }
            }

            if epoch_count > 0 {
                last_loss = epoch_loss_sum / epoch_count as f64;
            }
        }

        // Compute holdout accuracy
        let holdout_preds: Vec<Vec<f64>> = holdout_idx
            .iter()
            .map(|&idx| {
                let logits = self.forward(&inputs[idx]).unwrap_or_else(|_| {
                    // Fallback: zero vector of correct length
                    Tensor::zeros((self.labels.len(),), DType::F32, &Device::Cpu).unwrap_or_else(
                        |_| {
                            // Last-ditch fallback — should never reach here
                            panic!("reflex_seq train: cannot build fallback tensor")
                        },
                    )
                });
                let v: Vec<f32> = logits
                    .flatten_all()
                    .ok()
                    .and_then(|t| t.to_vec1().ok())
                    .unwrap_or_default();
                v.iter().map(|x| *x as f64).collect()
            })
            .collect();
        let holdout_targets: Vec<usize> =
            holdout_idx.iter().map(|&idx| target_classes[idx]).collect();
        let holdout_accuracy = compute_accuracy(&holdout_preds, &holdout_targets);

        self.last_metric = Some(holdout_accuracy);

        Ok((last_loss, holdout_accuracy))
    }

    /// Predict: returns softmax probabilities for each label.
    /// Used by `reflex_predict_dispatch` for the sequence path.
    pub fn predict_probs(&self, input: &Tensor) -> Result<Vec<f32>, String> {
        let logits = self.forward(input)?;
        let logits_2d = logits
            .unsqueeze(0)
            .map_err(|e| format!("predict unsqueeze: {}", e))?;
        let probs = candle_nn::ops::softmax(&logits_2d, 1)
            .map_err(|e| format!("predict softmax: {}", e))?;
        let v: Vec<f32> = probs
            .flatten_all()
            .map_err(|e| format!("predict flatten: {}", e))?
            .to_vec1()
            .map_err(|e| format!("predict to_vec1: {}", e))?;
        Ok(v)
    }
}

// ── helpers ─────────────────────────────────────────────────────────────

/// Cross-entropy loss for a single sample.
///
/// `logits`: `[1, num_labels]`, `target`: class index.
///
/// Computes: `-log(softmax(logits)[target])` — standard formulation,
/// matches `src/nn/loss.rs::cross_entropy_loss` but for a single
/// sample (per-sample SGD, not batched).
///
/// Uses candle ops directly (softmax, narrow, neg) so the computation
/// graph is built and `backward()` works.
fn cross_entropy_seq_loss(logits: &Tensor, target: usize) -> Result<Tensor, String> {
    // softmax(logits) along last axis
    let probs =
        candle_nn::ops::softmax(logits, 1).map_err(|e| format!("seq loss softmax: {}", e))?;
    // target prob: probs[0, target]
    let target_prob = probs
        .narrow(1, target, 1)
        .map_err(|e| format!("seq loss narrow: {}", e))?;
    // loss = -log(target_prob)
    let log_prob = target_prob
        .log()
        .map_err(|e| format!("seq loss log: {}", e))?;
    let neg_log = log_prob
        .affine(-1.0, 0.0)
        .map_err(|e| format!("seq loss neg: {}", e))?;
    // Squeeze to scalar
    neg_log
        .squeeze(0)
        .map_err(|e| format!("seq loss squeeze 0: {}", e))?
        .squeeze(0)
        .map_err(|e| format!("seq loss squeeze 1: {}", e))
}

/// Deterministic 80/20 split — same algorithm as `ReflexModel::train`
/// (Наряд №179), reused here so the determinism contract applies
/// symmetrically to sequence models.
fn deterministic_split(n: usize, seed: u64) -> Vec<(usize, bool)> {
    let mut indices: Vec<usize> = (0..n).collect();
    let mut state = seed ^ 0x9E3779B97F4A7C15;
    if state == 0 {
        state = 0x9E3779B97F4A7C15;
    }
    for i in (1..n).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let j = (state as usize) % (i + 1);
        indices.swap(i, j);
    }
    let train_count = (n * 4) / 5;
    indices
        .into_iter()
        .enumerate()
        .map(|(pos, idx)| (idx, pos < train_count))
        .collect()
}

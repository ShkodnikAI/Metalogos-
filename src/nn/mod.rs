//! Neural network module — Reflex pillar core (Наряд №178, этап 2/6).
//!
//! Implements the Layer trait, LAYER_REGISTRY (registry-extensible per
//! ADR-0114 addendum), Dense layer, and activation functions that
//! reuse the numerically stable implementations from `src/builtins/math.rs`
//! (Наряд №177) — no duplicate math.
//!
//! `ReflexId` and `ReflexRegistry` provide the opaque handle pattern
//! (ADR-0114): model weights never enter `Value`, only an index.

pub mod activation;
/// Наряд №183: multi-head self-attention with RoPE — first SequenceLayer.
/// Feature-gated behind `candle` (off by default).
#[cfg(feature = "candle")]
pub mod attention;
pub mod dense;
pub mod layer;
pub mod loss;
pub mod metric;
pub mod optim;
/// Наряд №180: persistence (ADR-0116) — save/load trained weights to SQLite.
pub mod persist;
/// Наряд №183 (ADR-0119): sequence-processing layer trait + registry.
/// Feature-gated behind `candle` — separate scope from initial Reflex rollout.
#[cfg(feature = "candle")]
pub mod sequence_layer;
pub mod serde_weights;

pub use activation::{Activation, ActivationKind};
pub use dense::Dense;
pub use layer::{Layer, LayerSpec, LAYER_REGISTRY};
pub use metric::{compute_accuracy, find_metric, metric_names, MetricSpec, METRIC_REGISTRY};
// Наряд №183: re-export SequenceLayer types when candle feature is on.
#[cfg(feature = "candle")]
pub use sequence_layer::{
    find_sequence_layer_spec, sequence_layer_names, SequenceLayer, SequenceLayerSpec,
    SEQUENCE_LAYER_REGISTRY,
};

/// Opaque handle to a Reflex model in the registry.
/// Weights are never accessible through this handle — only through
/// `reflex_predict` (Наряд №179) and `reflex_train` (Наряд №179).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ReflexId(pub usize);

/// A registered Reflex model.
/// Debug implementation prints only `name` and `last_metric` — never weights.
/// Manual Debug impl (not derive) because `Box<dyn Layer>` doesn't impl Debug.
pub struct ReflexModel {
    pub name: String,
    pub layers: Vec<Box<dyn Layer>>,
    pub seed: u64,
    pub last_metric: Option<f64>,
    pub input_size: usize,   // embedding dim (from reflex_decl)
    pub labels: Vec<String>, // label names (from reflex_decl)
}

impl ReflexModel {
    /// Forward pass through all layers.
    /// Input is raw f64 slice, output is raw f64 vec.
    pub fn forward(&self, input: &[f64]) -> Vec<f64> {
        let mut current = input.to_vec();
        for layer in &self.layers {
            current = layer.forward(&current);
        }
        current
    }

    /// Total parameter count (for debugging/metrics).
    pub fn param_count(&self) -> usize {
        self.layers
            .iter()
            .map(|l| l.input_size() * l.output_size() + l.output_size())
            .sum()
    }

    /// Train the model on the given dataset.
    /// Returns (train_loss, holdout_accuracy).
    ///
    /// Наряд №179:
    /// - Splits data 80/20 using deterministic shuffle by seed (ADR-0115)
    /// - Trains for `epochs` epochs using SGD with cross-entropy loss
    /// - Computes accuracy on holdout (20%) after training
    /// - Returns (final_train_loss, holdout_accuracy)
    pub fn train(
        &mut self,
        inputs: &[Vec<f64>],
        target_classes: &[usize],
        epochs: usize,
        learning_rate: f64,
    ) -> Result<(f64, f64), String> {
        use crate::nn::loss::cross_entropy_loss;
        use crate::nn::metric::compute_accuracy;

        // Block 4: minimum dataset size check (ADR-0115)
        if inputs.len() < 10 {
            return Err(format!(
                "reflex_train: need at least 10 examples for meaningful holdout, got {}",
                inputs.len()
            ));
        }

        // Deterministic 80/20 split using seed
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

        // Train
        let mut last_loss = 0.0;
        for _epoch in 0..epochs {
            let mut predictions: Vec<Vec<f64>> = Vec::with_capacity(train_idx.len());
            let mut targets: Vec<usize> = Vec::with_capacity(train_idx.len());

            // Per-sample storage: layer inputs, pre-activations, post-activations
            let mut all_layer_inputs: Vec<Vec<Vec<f64>>> = Vec::with_capacity(train_idx.len());
            let mut all_preacts: Vec<Vec<Vec<f64>>> = Vec::with_capacity(train_idx.len());
            let mut all_postacts: Vec<Vec<Vec<f64>>> = Vec::with_capacity(train_idx.len());

            for &idx in &train_idx {
                let input = &inputs[idx];
                let target = target_classes[idx];

                // Forward through all layers, collecting intermediates
                let mut current = input.clone();
                let mut preacts: Vec<Vec<f64>> = Vec::new();
                let mut postacts: Vec<Vec<f64>> = Vec::new();
                let mut layer_inputs: Vec<Vec<f64>> = Vec::new();

                for layer in &self.layers {
                    layer_inputs.push(current.clone());
                    if let Some(dense) = layer.as_any().downcast_ref::<Dense>() {
                        let (pre, post) = dense.forward_with_preact(&current);
                        preacts.push(pre);
                        postacts.push(post.clone());
                        current = post;
                    } else {
                        let out = layer.forward(&current);
                        preacts.push(out.clone());
                        postacts.push(out.clone());
                        current = out;
                    }
                }
                all_layer_inputs.push(layer_inputs);
                all_preacts.push(preacts);
                all_postacts.push(postacts);
                predictions.push(current.clone());
                targets.push(target);
            }

            // Compute loss and gradients
            let (loss, loss_grads) = cross_entropy_loss(&predictions, &targets);
            last_loss = loss;

            // Backward pass for each sample (online SGD)
            for (i, &_idx) in train_idx.iter().enumerate() {
                let mut grad = loss_grads[i].clone();

                // Backward through layers (reverse order)
                for layer_idx in (0..self.layers.len()).rev() {
                    let (grad_in, grad_w, grad_b) = {
                        let layer = &self.layers[layer_idx];
                        if let Some(dense) = layer.as_any().downcast_ref::<Dense>() {
                            let mut adjusted_grad = grad.clone();
                            // For the LAST layer with softmax + cross-entropy:
                            // the loss gradient is already w.r.t. logits (softmax - one_hot),
                            // so we skip the softmax Jacobian (it would double-apply).
                            let is_last_layer = layer_idx == self.layers.len() - 1;
                            if !(is_last_layer && dense.activation == ActivationKind::Softmax) {
                                let post_act = &all_postacts[i][layer_idx];
                                let pre_act = &all_preacts[i][layer_idx];
                                dense.activation_backward(post_act, pre_act, &mut adjusted_grad);
                            }
                            let layer_input = &all_layer_inputs[i][layer_idx];
                            let result = dense.backward(layer_input, &adjusted_grad);
                            (result.0, result.1, result.2)
                        } else {
                            break;
                        }
                    };
                    if let Some(dense_mut) =
                        self.layers[layer_idx].as_any_mut().downcast_mut::<Dense>()
                    {
                        dense_mut.update_weights(&grad_w, &grad_b, learning_rate);
                    }
                    grad = grad_in;
                }
            }
        }

        // Compute holdout accuracy
        let holdout_preds: Vec<Vec<f64>> = holdout_idx
            .iter()
            .map(|&idx| self.forward(&inputs[idx]))
            .collect();
        let holdout_targets: Vec<usize> =
            holdout_idx.iter().map(|&idx| target_classes[idx]).collect();
        let holdout_accuracy = compute_accuracy(&holdout_preds, &holdout_targets);

        self.last_metric = Some(holdout_accuracy);

        Ok((last_loss, holdout_accuracy))
    }
}

/// Registry of all Reflex models — the runtime store for opaque handles.
/// Index into `models` vec = `ReflexId`.
pub struct ReflexRegistry {
    models: Vec<ReflexModel>,
}

impl Default for ReflexRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ReflexRegistry {
    pub fn new() -> Self {
        Self { models: Vec::new() }
    }

    /// Register a model, return its handle.
    pub fn register(&mut self, model: ReflexModel) -> ReflexId {
        let id = ReflexId(self.models.len());
        self.models.push(model);
        id
    }

    /// Get a model by handle.
    pub fn get(&self, id: ReflexId) -> Option<&ReflexModel> {
        self.models.get(id.0)
    }

    /// Get a mutable model by handle.
    pub fn get_mut(&mut self, id: ReflexId) -> Option<&mut ReflexModel> {
        self.models.get_mut(id.0)
    }

    /// Number of registered models.
    pub fn len(&self) -> usize {
        self.models.len()
    }

    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }
}

impl std::fmt::Debug for ReflexRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ReflexRegistry({} models: [{}])",
            self.models.len(),
            self.models
                .iter()
                .map(|m| format!(
                    "{}({} layers, metric={:?})",
                    m.name,
                    m.layers.len(),
                    m.last_metric
                ))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl std::fmt::Debug for ReflexModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReflexModel")
            .field("name", &self.name)
            .field("num_layers", &self.layers.len())
            .field("seed", &self.seed)
            .field("last_metric", &self.last_metric)
            .field("input_size", &self.input_size)
            .field("labels", &self.labels)
            .finish_non_exhaustive()
    }
}

/// Deterministic 80/20 train/holdout split.
/// Uses xorshift64 (same algorithm as Наряд №177) seeded by model.seed.
/// Returns Vec<(index, is_train)> where is_train=true for 80%, false for 20%.
fn deterministic_split(n: usize, seed: u64) -> Vec<(usize, bool)> {
    // Create shuffled indices using xorshift64
    let mut indices: Vec<usize> = (0..n).collect();
    let mut state = seed ^ 0x9E3779B97F4A7C15;
    if state == 0 {
        state = 0x9E3779B97F4A7C15;
    }

    // Fisher-Yates shuffle with xorshift64
    for i in (1..n).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let j = (state as usize) % (i + 1);
        indices.swap(i, j);
    }

    let train_count = (n * 4) / 5; // 80%
    indices
        .into_iter()
        .enumerate()
        .map(|(pos, idx)| (idx, pos < train_count))
        .collect()
}

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
pub mod dense;
pub mod layer;
pub mod serde_weights;

pub use activation::{Activation, ActivationKind};
pub use dense::Dense;
pub use layer::{Layer, LayerSpec, LAYER_REGISTRY};

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

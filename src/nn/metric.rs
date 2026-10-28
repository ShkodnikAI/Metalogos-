//! Metric registry (Наряд №179 Block 3 — ADR-0114 addendum for metrics).
//!
//! Same pattern as LAYER_REGISTRY: extensible, no grammar change.
//! `metric` field in reflex_train resolves via this registry.
//!
//! №826 (ADR-0216): `macro_f1` and `min_class_recall` join `accuracy` —
//! the W2′ line of the provable tier-lowering roadmap (ADR-0200). The
//! `rollback_if` gate resolves its metric NAME through this registry at
//! the semantic check (loud refusal for unregistered names), and the
//! same registry computes the gate's majority baseline.

/// Specification for a metric — analogous to LayerSpec.
pub struct MetricSpec {
    pub name: &'static str,
    pub compute: fn(predictions: &[Vec<f64>], target_classes: &[usize]) -> f64,
}

/// The metric registry. Order is the reported order (the refusal message
/// and REFERENCE list verbatim) — `accuracy` first (the default gate
/// metric), then the №826 additions.
pub static METRIC_REGISTRY: &[MetricSpec] = &[
    MetricSpec {
        name: "accuracy",
        compute: compute_accuracy,
    },
    MetricSpec {
        name: "macro_f1",
        compute: compute_macro_f1,
    },
    MetricSpec {
        name: "min_class_recall",
        compute: compute_min_class_recall,
    },
];

/// Look up a metric by name.
pub fn find_metric(name: &str) -> Option<&'static MetricSpec> {
    METRIC_REGISTRY.iter().find(|m| m.name == name)
}

/// List all registered metric names.
pub fn metric_names() -> Vec<&'static str> {
    METRIC_REGISTRY.iter().map(|m| m.name).collect()
}

// №545 (в): the compute_accuracy SSOT moved with the generative
// machinery to metalogos-reflex (the reflex-side holdout gate uses
// it); the path below keeps every crate::nn::metric::compute_accuracy
// consumer identical.
pub use metalogos_reflex::nn::metric::compute_accuracy;

// №826 (ADR-0216): the named metrics share the reflex-side SSOT — the
// class-space math (`macro_f1_labels`, `min_class_recall_labels`) is the
// ONE body behind both the registry adapters and the mutate gate's
// label-space conversion.
pub use metalogos_reflex::nn::metric::{
    compute_macro_f1, compute_min_class_recall, macro_f1_labels, min_class_recall_labels,
};

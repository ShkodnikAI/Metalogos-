//! Metric registry (Наряд №179 Block 3 — ADR-0114 addendum for metrics).
//!
//! Same pattern as LAYER_REGISTRY: extensible, no grammar change.
//! `metric` field in reflex_train resolves via this registry.

/// Specification for a metric — analogous to LayerSpec.
pub struct MetricSpec {
    pub name: &'static str,
    pub compute: fn(predictions: &[Vec<f64>], target_classes: &[usize]) -> f64,
}

/// The metric registry.
pub static METRIC_REGISTRY: &[MetricSpec] = &[MetricSpec {
    name: "accuracy",
    compute: compute_accuracy,
}];

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

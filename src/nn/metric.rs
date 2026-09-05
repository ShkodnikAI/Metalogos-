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

/// Compute accuracy: fraction of correct predictions.
/// Uses argmax of predictions (the class with highest probability).
pub fn compute_accuracy(predictions: &[Vec<f64>], target_classes: &[usize]) -> f64 {
    if predictions.is_empty() {
        return 0.0;
    }
    let correct = predictions
        .iter()
        .zip(target_classes.iter())
        .filter(|(pred, &target)| {
            let predicted = pred
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i)
                .unwrap_or(0);
            predicted == target
        })
        .count();
    correct as f64 / predictions.len() as f64
}

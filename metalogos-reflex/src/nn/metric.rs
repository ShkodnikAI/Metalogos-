//! The accuracy SSOT (naryad 545 (v)): the pure math moved here from
//! the language crate's metric.rs so the reflex-side holdout gates
//! (seq_model) and the language-side METRIC_REGISTRY share ONE
//! implementation without a cyclic dependency. Ungated: pure f64 math,
//! no candle.

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

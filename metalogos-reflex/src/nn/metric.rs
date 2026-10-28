//! The metric math SSOT (naryad 545 (v)): the pure math moved here from
//! the language crate's metric.rs so the reflex-side holdout gates
//! (seq_model) and the language-side METRIC_REGISTRY share ONE
//! implementation without a cyclic dependency. Ungated: pure f64 math,
//! no candle.
//!
//! №826 (ADR-0216): the registry grows `macro_f1` and `min_class_recall`
//! beside `accuracy`. The CLASS-SPACE math (`*_labels` over parallel
//! truth/pred class-index slices) is the SSOT; the `compute_*` adapters
//! (the `MetricSpec` signature — probability vectors argmaxed to classes)
//! route through it, and the №826 mutate gate converts its held-out label
//! pairs to the same class space. One math body, three consumers.

/// The argmax of a probability vector (ties → the first/highest index —
/// the pre-№826 `compute_accuracy` behavior, byte-preserved).
fn argmax(probs: &[f64]) -> usize {
    probs
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0)
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
        .filter(|(pred, &target)| argmax(pred) == target)
        .count();
    correct as f64 / predictions.len() as f64
}

/// №826 (ADR-0216): macro-averaged F1 over class indices.
///
/// Per class: F1 = 2·TP / ((TP+FP) + (TP+FN)). Edge semantics (pinned by
/// `tests/naryad_826_metric_registry.rs`):
/// - a class present in NEITHER truth NOR predictions is skipped (it
///   carries no confusion evidence);
/// - a class with truths but no predictions scores F1 = 0 (recall 0);
/// - a class with predictions but no truths scores F1 = 0 (precision 0) —
///   and it STAYS in the average (it is present in the confusion);
/// - unequal slice lengths or empty evidence → 0.0 (no invention).
pub fn macro_f1_labels(truth: &[usize], pred: &[usize], n_classes: usize) -> f64 {
    if truth.is_empty() || truth.len() != pred.len() {
        return 0.0;
    }
    let mut f1_sum = 0.0f64;
    let mut counted = 0usize;
    for c in 0..n_classes {
        let tp = truth
            .iter()
            .zip(pred)
            .filter(|(t, p)| **t == c && **p == c)
            .count();
        let fp = pred
            .iter()
            .zip(truth)
            .filter(|(p, t)| **p == c && **t != c)
            .count();
        let fn_ = truth
            .iter()
            .zip(pred)
            .filter(|(t, p)| **t == c && **p != c)
            .count();
        if tp + fp + fn_ == 0 {
            continue;
        }
        let denom = (tp + fp + tp + fn_) as f64;
        let f1 = if denom > 0.0 {
            2.0 * tp as f64 / denom
        } else {
            0.0
        };
        f1_sum += f1;
        counted += 1;
    }
    if counted == 0 {
        0.0
    } else {
        f1_sum / counted as f64
    }
}

/// №826 (ADR-0216): the minimum per-class recall over class indices.
///
/// Per class: recall = TP / (TP + FN). Edge semantics (pinned):
/// - a class with NO truths is skipped (recall is undefined there, and a
///   skipped class must not drag the minimum to a fabricated 0);
/// - a class with truths and no correct predictions scores 0;
/// - unequal slice lengths or no scored classes → 0.0.
pub fn min_class_recall_labels(truth: &[usize], pred: &[usize], n_classes: usize) -> f64 {
    if truth.is_empty() || truth.len() != pred.len() {
        return 0.0;
    }
    let mut min_recall = f64::MAX;
    let mut counted = 0usize;
    for c in 0..n_classes {
        let truth_n = truth.iter().filter(|&t| *t == c).count();
        if truth_n == 0 {
            continue;
        }
        let tp = truth
            .iter()
            .zip(pred)
            .filter(|(t, p)| **t == c && **p == c)
            .count();
        let recall = tp as f64 / truth_n as f64;
        if recall < min_recall {
            min_recall = recall;
        }
        counted += 1;
    }
    if counted == 0 {
        0.0
    } else {
        min_recall
    }
}

/// №826: the class space implied by the evidence — every class index seen
/// in truth or predictions, plus one. The adapters derive it locally so
/// the registry signature needs no explicit class count.
fn implied_classes(pred_classes: &[usize], truth: &[usize]) -> usize {
    pred_classes
        .iter()
        .chain(truth.iter())
        .copied()
        .max()
        .map_or(0, |m| m + 1)
}

/// №826: the `MetricSpec` adapter for macro F1 — argmaxes the probability
/// vectors to classes and routes into `macro_f1_labels` (the SSOT).
pub fn compute_macro_f1(predictions: &[Vec<f64>], target_classes: &[usize]) -> f64 {
    let pred: Vec<usize> = predictions.iter().map(|p| argmax(p)).collect();
    let n_classes = implied_classes(&pred, target_classes);
    macro_f1_labels(target_classes, &pred, n_classes)
}

/// №826: the `MetricSpec` adapter for the minimum class recall — argmaxes
/// the probability vectors to classes and routes into
/// `min_class_recall_labels` (the SSOT).
pub fn compute_min_class_recall(predictions: &[Vec<f64>], target_classes: &[usize]) -> f64 {
    let pred: Vec<usize> = predictions.iter().map(|p| argmax(p)).collect();
    let n_classes = implied_classes(&pred, target_classes);
    min_class_recall_labels(target_classes, &pred, n_classes)
}

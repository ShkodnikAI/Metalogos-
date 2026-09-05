//! Loss functions for neural network training (Наряд №179).
//!
//! Used by reflex_train to compute loss and its gradient.

/// Compute cross-entropy loss and gradient.
/// predictions: softmax probabilities [batch, num_classes]
/// targets: one-hot or class indices [batch] (as f64 for uniformity)
/// Returns (loss, grad) where grad is dL/d(logits) = softmax - one_hot.
pub fn cross_entropy_loss(
    predictions: &[Vec<f64>],
    target_classes: &[usize],
) -> (f64, Vec<Vec<f64>>) {
    let batch_size = predictions.len();
    if batch_size == 0 {
        return (0.0, vec![]);
    }
    let _num_classes = predictions[0].len();

    let mut total_loss = 0.0;
    let mut grads = Vec::with_capacity(batch_size);

    for (i, probs) in predictions.iter().enumerate() {
        let target = target_classes[i];
        // Clamp prediction to avoid log(0)
        let p = probs[target].max(1e-15);
        total_loss -= p.ln();

        // Gradient of cross-entropy w.r.t. logits = softmax - one_hot
        let mut grad = probs.clone();
        grad[target] -= 1.0;
        grads.push(grad);
    }

    let avg_loss = total_loss / batch_size as f64;
    // Scale gradients by 1/batch
    let scale = 1.0 / batch_size as f64;
    for grad in grads.iter_mut() {
        for v in grad.iter_mut() {
            *v *= scale;
        }
    }

    (avg_loss, grads)
}

/// Compute MSE loss and gradient.
/// predictions: [batch, output_dim]
/// targets: [batch, output_dim]
/// Returns (loss, grad) where grad = 2*(pred - target) / batch.
pub fn mse_loss(predictions: &[Vec<f64>], targets: &[Vec<f64>]) -> (f64, Vec<Vec<f64>>) {
    let batch_size = predictions.len();
    if batch_size == 0 {
        return (0.0, vec![]);
    }
    let output_dim = predictions[0].len();

    let mut total_loss = 0.0;
    let mut grads = Vec::with_capacity(batch_size);

    for (i, pred) in predictions.iter().enumerate() {
        let tgt = &targets[i];
        let mut grad = Vec::with_capacity(output_dim);
        let mut sample_loss = 0.0;
        for j in 0..output_dim {
            let diff = pred[j] - tgt[j];
            sample_loss += diff * diff;
            grad.push(2.0 * diff);
        }
        total_loss += sample_loss / output_dim as f64;
        grads.push(grad);
    }

    let avg_loss = total_loss / batch_size as f64;
    let scale = 1.0 / (batch_size as f64 * output_dim as f64);
    for grad in grads.iter_mut() {
        for v in grad.iter_mut() {
            *v *= scale;
        }
    }

    (avg_loss, grads)
}

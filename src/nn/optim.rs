//! SGD optimizer for Dense layers (Наряд №179).
//!
//! Simple SGD with fixed learning rate. No momentum (sufficient
//! for linear/logistic head — the spec says "SGD sufficient").
//! Adam is optional — can be added later if convergence demands it.

/// SGD optimizer state.
pub struct Sgd {
    pub learning_rate: f64,
}

impl Sgd {
    pub fn new(learning_rate: f64) -> Self {
        Self { learning_rate }
    }

    /// Apply gradient descent step to Dense layer weights/bias.
    /// grad_weights: [output_dim][input_dim] (same shape as weights)
    /// grad_bias: [output_dim]
    pub fn step_dense(
        &self,
        weights: &mut [Vec<f64>],
        bias: &mut [f64],
        grad_weights: &[Vec<f64>],
        grad_bias: &[f64],
    ) {
        let lr = self.learning_rate;
        for (o, w_row) in weights.iter_mut().enumerate() {
            for (i, w) in w_row.iter_mut().enumerate() {
                *w -= lr * grad_weights[o][i];
            }
            bias[o] -= lr * grad_bias[o];
        }
    }
}

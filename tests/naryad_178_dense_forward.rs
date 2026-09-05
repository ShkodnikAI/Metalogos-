// ── Наряд №178: Dense layer forward pass contract ───────────────────
//
// Block 4 contract: forward pass through real Dense layer on known
// weights gives mathematically correct result (not just "doesn't panic").

use metalogos::nn::{Layer, Dense, activation::ActivationKind};

#[test]
fn dense_forward_known_weights() {
    // 2→3 Dense layer, no activation (linear)
    // weights[output][input]:
    //   [[1.0, 2.0],   // output neuron 0: y0 = 1*x0 + 2*x1
    //    [3.0, 4.0],   // output neuron 1: y1 = 3*x0 + 4*x1
    //    [5.0, 6.0]]   // output neuron 2: y2 = 5*x0 + 6*x1
    // bias: [0.1, 0.2, 0.3]
    let layer = Dense::with_weights(
        vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]],
        vec![0.1, 0.2, 0.3],
        ActivationKind::None,
    );

    // Input: [10.0, 1.0]
    // y0 = 1*10 + 2*1 + 0.1 = 12.1
    // y1 = 3*10 + 4*1 + 0.2 = 34.2
    // y2 = 5*10 + 6*1 + 0.3 = 56.3
    let output = layer.forward(&[10.0, 1.0]);

    assert_eq!(output.len(), 3);
    assert!((output[0] - 12.1).abs() < 1e-10, "y0 = {}", output[0]);
    assert!((output[1] - 34.2).abs() < 1e-10, "y1 = {}", output[1]);
    assert!((output[2] - 56.3).abs() < 1e-10, "y2 = {}", output[2]);
}

#[test]
fn dense_forward_with_relu() {
    // Same weights, ReLU activation
    // Negative inputs → 0 after ReLU
    let layer = Dense::with_weights(
        vec![vec![-1.0, 0.0], vec![1.0, 0.0]],
        vec![0.0, 0.0],
        ActivationKind::Relu,
    );

    // Input: [5.0, 0.0]
    // y0 = -1*5 + 0 = -5 → ReLU → 0
    // y1 = 1*5 + 0 = 5 → ReLU → 5
    let output = layer.forward(&[5.0, 0.0]);

    assert!((output[0] - 0.0).abs() < 1e-10, "ReLU(-5) = {}", output[0]);
    assert!((output[1] - 5.0).abs() < 1e-10, "ReLU(5) = {}", output[1]);
}

#[test]
fn dense_forward_with_sigmoid() {
    let layer = Dense::with_weights(
        vec![vec![1.0]],
        vec![0.0],
        ActivationKind::Sigmoid,
    );

    // Input: [0.0] → y = sigmoid(0) = 0.5
    let output = layer.forward(&[0.0]);
    assert!((output[0] - 0.5).abs() < 1e-10, "sigmoid(0) = {}", output[0]);
}

#[test]
fn dense_forward_with_softmax() {
    let layer = Dense::with_weights(
        vec![vec![1.0, 0.0], vec![0.0, 1.0]],
        vec![0.0, 0.0],
        ActivationKind::Softmax,
    );

    // Input: [1.0, 1.0] → logits [1, 1] → softmax = [0.5, 0.5]
    let output = layer.forward(&[1.0, 1.0]);
    assert!((output[0] - 0.5).abs() < 1e-10, "softmax[0] = {}", output[0]);
    assert!((output[1] - 0.5).abs() < 1e-10, "softmax[1] = {}", output[1]);

    // Sum = 1.0
    let sum: f64 = output.iter().sum();
    assert!((sum - 1.0).abs() < 1e-10, "softmax sum = {}", sum);
}

#[test]
fn dense_layer_dimensions() {
    let layer = Dense::new(10, 5, ActivationKind::Relu, 42);
    assert_eq!(layer.input_size(), 10);
    assert_eq!(layer.output_size(), 5);
    assert_eq!(layer.name(), "dense");
}

#[test]
fn dense_serialize_deserialize_roundtrip() {
    let original = Dense::with_weights(
        vec![vec![1.0, 2.0], vec![3.0, 4.0]],
        vec![0.5, 0.6],
        ActivationKind::Sigmoid,
    );

    let data = original.serialize_weights();
    let mut restored = Dense::with_weights(
        vec![vec![0.0, 0.0], vec![0.0, 0.0]],
        vec![0.0, 0.0],
        ActivationKind::None,
    );
    restored.deserialize_weights(&data).unwrap();

    // Verify weights match
    let input = [1.0, 2.0];
    let orig_out = original.forward(&input);
    let rest_out = restored.forward(&input);
    for (a, b) in orig_out.iter().zip(rest_out.iter()) {
        assert!((a - b).abs() < 1e-10, "roundtrip mismatch: {} vs {}", a, b);
    }
}

#[test]
fn dense_deterministic_init() {
    // Same seed → same weights → same output
    let l1 = Dense::new(3, 2, ActivationKind::None, 42);
    let l2 = Dense::new(3, 2, ActivationKind::None, 42);
    let input = [1.0, 2.0, 3.0];
    let o1 = l1.forward(&input);
    let o2 = l2.forward(&input);
    for (a, b) in o1.iter().zip(o2.iter()) {
        assert_eq!(a.to_bits(), b.to_bits(), "deterministic init failed: {} != {}", a, b);
    }
}

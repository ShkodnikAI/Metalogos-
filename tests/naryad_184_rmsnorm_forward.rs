// ── Наряд №184 Contract 1: RmsNorm forward — numerical correctness ──
//
// Per the naryad spec:
//   "численная корректность против независимого NumPy-референса (тот же
//    принцип, что наряд №183: не self-confirming через candle)."
//
// This test gates on the `candle` feature — when candle is off, the
// test is skipped (the whole SequenceLayer module is feature-gated).
//
// ## Reference numbers
//
// The expected output is computed by an independent NumPy script
// (kept in /home/z/my-project/scripts/naryad_184_numpy_reference.py,
// also embedded below for full reproducibility). The script reproduces
// the same RmsNorm algorithm using NumPy's float64 mean/sqrt, then
// downcasts to float32 to match candle's CPU default dtype.
//
// The reference is intentionally NOT a `candle`-internal implementation
// (that would be self-confirming).
//
// ```python
// import numpy as np
// dim = 8; seq = 3; eps = 1e-6
// weight = np.ones(dim, dtype=np.float32)
// x = np.arange(seq * dim, dtype=np.float32).reshape(seq, dim) * 0.1
// ms = np.mean(x.astype(np.float64) ** 2, axis=-1, keepdims=True)
// norm = 1.0 / np.sqrt(ms + eps)
// out = (x * norm * weight).astype(np.float32)
// ```
//
// Output (24 values, row-major [3, 8]):
//   row 0: [0.0, 0.23904504, 0.47809008, 0.71713513, 0.95618016, 1.1952252,
//           1.4342703, 1.6733153]
//   row 1: [0.68224204, 0.76752234, 0.8528025, 0.9380828, 1.0233631,
//           1.1086434, 1.1939236, 1.2792038]
//   row 2: [0.8149064, 0.86583805, 0.91676974, 0.9677013, 1.018633,
//           1.0695647, 1.1204963, 1.171428]

#![cfg(feature = "candle")]

use metalogos::nn::rmsnorm::RmsNorm;
use metalogos::nn::sequence_layer::SequenceLayer;

#[test]
fn rmsnorm_forward_matches_numpy_reference() {
    let dim = 8;
    let seq_len = 3;
    let eps = 1e-6;
    // Default RmsNorm init: weight = ones(dim). Matches NumPy reference.
    let layer = RmsNorm::new(dim, 42, eps).expect("rmsnorm build");

    // Input: a 3x8 deterministic tensor (matches the NumPy reference script's
    // x = np.arange(seq_len * dim) * 0.1).
    let input_values: Vec<f32> = (0..seq_len * dim).map(|i| (i as f32) * 0.1).collect();
    let device = candle_core::Device::Cpu;
    let input = candle_core::Tensor::from_vec(input_values, (seq_len, dim), &device)
        .expect("input tensor")
        .to_dtype(candle_core::DType::F32)
        .expect("input dtype");

    let output = layer.forward(&input).expect("forward");

    let output_vec: Vec<f32> = output
        .flatten_all()
        .expect("flatten")
        .to_vec1()
        .expect("to_vec1");

    // Expected output (computed by the NumPy script in the docstring above).
    let expected: &[f32] = &[
        0.0, 0.23904504, 0.47809008, 0.71713513, 0.95618016, 1.1952252, 1.4342703, 1.6733153,
        0.68224204, 0.76752234, 0.8528025, 0.9380828, 1.0233631, 1.1086434, 1.1939236, 1.2792038,
        0.8149064, 0.86583805, 0.91676974, 0.9677013, 1.018633, 1.0695647, 1.1204963, 1.171428,
    ];

    assert_eq!(output_vec.len(), expected.len(), "output length mismatch");

    let mut max_diff = 0.0f32;
    for (i, (actual, expected)) in output_vec.iter().zip(expected.iter()).enumerate() {
        let diff = (actual - expected).abs();
        if diff > max_diff {
            max_diff = diff;
        }
        assert!(
            diff < 1e-5,
            "rmsnorm output[{}] mismatch: actual={:.8}, expected={:.8}, diff={:.2e}",
            i,
            actual,
            expected,
            diff
        );
    }
    println!(
        "✓ rmsnorm forward matches NumPy reference (max diff = {:.2e}, tolerance = 1e-5)",
        max_diff
    );
}

#[test]
fn rmsnorm_forward_deterministic_same_seed() {
    // Same seed → same output (RmsNorm with ones-init: weight is seed-
    // independent, but the contract still holds — same construction
    // yields the same forward pass).
    let dim = 16;
    let seq_len = 4;
    let eps = 1e-6;

    let l1 = RmsNorm::new(dim, 42, eps).expect("rmsnorm build");
    let l2 = RmsNorm::new(dim, 42, eps).expect("rmsnorm build (same seed)");

    let input_values: Vec<f32> = (0..seq_len * dim).map(|i| (i as f32) * 0.05).collect();
    let device = candle_core::Device::Cpu;
    let input = candle_core::Tensor::from_vec(input_values, (seq_len, dim), &device)
        .expect("input")
        .to_dtype(candle_core::DType::F32)
        .expect("dtype");

    let o1 = l1.forward(&input).expect("forward 1");
    let o2 = l2.forward(&input).expect("forward 2");

    let v1: Vec<f32> = o1.flatten_all().expect("flatten").to_vec1().expect("v1");
    let v2: Vec<f32> = o2.flatten_all().expect("flatten").to_vec1().expect("v2");

    assert_eq!(v1.len(), v2.len());
    let max_diff = v1
        .iter()
        .zip(v2.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    assert!(
        max_diff == 0.0,
        "same construction → bitwise-identical output, got max diff = {:.2e}",
        max_diff
    );
    println!(
        "✓ rmsnorm determinism: bitwise-identical ({} elements)",
        v1.len()
    );
}

#[test]
fn rmsnorm_preserves_shape() {
    // Output shape == input shape (residual stream preserved).
    let dim = 32;
    let seq_len = 5;
    let layer = RmsNorm::new(dim, 1, 1e-6).expect("rmsnorm build");

    let input_values: Vec<f32> = (0..seq_len * dim).map(|i| (i as f32) * 0.01).collect();
    let device = candle_core::Device::Cpu;
    let input = candle_core::Tensor::from_vec(input_values, (seq_len, dim), &device)
        .expect("input")
        .to_dtype(candle_core::DType::F32)
        .expect("dtype");

    let output = layer.forward(&input).expect("forward");
    let out_dims = output.dims();
    assert_eq!(
        out_dims,
        &[seq_len, dim],
        "shape mismatch: input {:?}, output {:?}",
        [seq_len, dim],
        out_dims
    );
    println!(
        "✓ rmsnorm preserves shape: {:?} → {:?}",
        [seq_len, dim],
        out_dims
    );
}

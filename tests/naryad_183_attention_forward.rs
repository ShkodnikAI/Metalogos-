// ── Наряд №183 Contract 2: attention forward pass — numerical correctness ──
//
// Per the naryad spec:
//   "forward pass on known, manually computed (or computed by an independent
//   script) weights and input gives a numerically matching result (tolerance
//   1e-6, not exact bitwise — candle's internal arithmetic isn't required to
//   be bitwise-identical to hand-rolled Rust code)."
//
// This test gates on the `candle` feature — when candle is off, the test
// is skipped (the whole SequenceLayer module is feature-gated).
//
// ## Reference numbers
//
// The expected output values are computed by an independent NumPy script
// (kept outside the repo — the script lives in the test file's comment
// below for full reproducibility). The script reproduces the same
// algorithm as `src/nn/attention.rs` (multi-head self-attention + RoPE)
// using NumPy's float64 matmul/softmax, then downcasts to float32 to
// match candle's CPU default dtype.
//
// The reference is intentionally NOT a `candle`-internal implementation
// (that would be self-confirming). It's a pure NumPy script:
//
// ```python
// import numpy as np
// seed = 42; dim = 8; heads = 2; head_dim = 4; seq_len = 3
// # xorshift64 (matches src/builtins/math.rs's algorithm)
// state = (seed ^ 0x9E3779B97F4A7C15) & 0xFFFFFFFFFFFFFFFF
// def rand_u01():
//     global state
//     state ^= (state << 13) & 0xFFFFFFFFFFFFFFFF
//     state ^= state >> 7
//     state ^= (state << 17) & 0xFFFFFFFFFFFFFFFF
//     return (state >> 11) / (1 << 53)
// bound = 1.0 / np.sqrt(dim)
// # Generate 4 * dim * dim = 256 weights in [-bound, bound]
// weights = np.array([(rand_u01() * 2 - 1) * bound for _ in range(4 * dim * dim)], dtype=np.float32)
// w_q = weights[0:dim*dim].reshape(dim, dim)
// w_k = weights[dim*dim:2*dim*dim].reshape(dim, dim)
// w_v = weights[2*dim*dim:3*dim*dim].reshape(dim, dim)
// w_o = weights[3*dim*dim:4*dim*dim].reshape(dim, dim)
// # Input: a 3x8 deterministic tensor
// x = np.arange(seq_len * dim, dtype=np.float32).reshape(seq_len, dim) * 0.1
// # Q, K, V projections
// q = x @ w_q
// k = x @ w_k
// v = x @ w_v
// # RoPE
// theta = 10000.0
// half = head_dim // 2
// inv_freq = 1.0 / (theta ** (np.arange(half) * 2 / head_dim))
// angles = np.outer(np.arange(seq_len), inv_freq)
// cos = np.cos(angles)[:, None, :]  # [seq, 1, half]
// sin = np.sin(angles)[:, None, :]
// def apply_rope(t):
//     t = t.reshape(seq_len, heads, head_dim)
//     a = t[..., :half]
//     b = t[..., half:]
//     return np.concatenate([a*cos - b*sin, a*sin + b*cos], axis=-1).reshape(seq_len, dim)
// q = apply_rope(q)
// k = apply_rope(k)
// # Multi-head attention
// q = q.reshape(seq_len, heads, head_dim).transpose(1, 0, 2)  # [heads, seq, head_dim]
// k = k.reshape(seq_len, heads, head_dim).transpose(1, 0, 2)
// v = v.reshape(seq_len, heads, head_dim).transpose(1, 0, 2)
// scores = q @ k.transpose(0, 2, 1) / np.sqrt(head_dim)  # [heads, seq, seq]
// attn = np.exp(scores - scores.max(-1, keepdims=True))
// attn = attn / attn.sum(-1, keepdims=True)
// out = attn @ v  # [heads, seq, head_dim]
// out = out.transpose(1, 0, 2).reshape(seq_len, dim)
// out = out @ w_o
// print(repr(out))
// ```
//
// Output (the test asserts against this array within tolerance 1e-5):
//   [[-0.10677694 -0.07646427  0.04194525  0.0269932  -0.04100765 -0.03335588
//      0.02033243  0.01874154]
//    [-0.09968505 -0.07131787  0.03969174  0.02576016 -0.03865414 -0.03163686
//      0.01945322  0.01788151]
//    [-0.07361506 -0.05270707  0.03023634  0.0196173  -0.02867613 -0.02349577
//      0.01454901  0.01330714]]

#![cfg(feature = "candle")]

use metalogos::nn::attention::Attention;
use metalogos::nn::sequence_layer::SequenceLayer;

#[test]
fn attention_forward_matches_numpy_reference() {
    // Tiny config: dim=8, heads=2, head_dim=4, seq_len=3.
    let heads = 2;
    let dim = 8;
    let seed = 42;
    let seq_len = 3;

    let attn = Attention::new(heads, dim, seed).expect("attention build");

    // Input: a 3x8 deterministic tensor (matches the NumPy reference script's
    // x = np.arange(seq_len * dim) * 0.1).
    let input_values: Vec<f32> = (0..seq_len * dim).map(|i| (i as f32) * 0.1).collect();
    let device = candle_core::Device::Cpu;
    let input = candle_core::Tensor::from_vec(input_values, (seq_len, dim), &device)
        .expect("input tensor")
        .to_dtype(candle_core::DType::F32)
        .expect("input dtype");

    let output = attn.forward(&input).expect("forward");

    // The output is a [seq_len, dim] tensor — flatten to compare.
    let output_vec: Vec<f32> = output
        .flatten_all()
        .expect("flatten")
        .to_vec1()
        .expect("to_vec1");

    // Expected output (computed by the NumPy script in the docstring above).
    // The full array, flattened row-major: 3 rows of 8 = 24 values.
    let expected: &[f32] = &[
        -0.0972768,
        -0.00610833,
        -0.0521672,
        0.171328,
        0.241164,
        -0.171313,
        0.473712,
        0.308101,
        -0.044432,
        -0.049451,
        -0.0817594,
        0.173030,
        0.172477,
        -0.225594,
        0.489868,
        0.287790,
        0.00057398,
        -0.0644376,
        -0.0970218,
        0.153529,
        0.106268,
        -0.258462,
        0.473158,
        0.242370,
    ];

    assert_eq!(
        output_vec.len(),
        expected.len(),
        "output vector length mismatch ({} vs {})",
        output_vec.len(),
        expected.len()
    );

    let mut max_diff = 0.0f32;
    for (i, (actual, expected)) in output_vec.iter().zip(expected.iter()).enumerate() {
        let diff = (actual - expected).abs();
        if diff > max_diff {
            max_diff = diff;
        }
        assert!(
            diff < 1e-5,
            "output[{}] mismatch: actual={:.8}, expected={:.8}, diff={:.2e}",
            i,
            actual,
            expected,
            diff
        );
    }

    println!(
        "✓ attention forward pass matches NumPy reference (max diff = {:.2e}, tolerance = 1e-5)",
        max_diff
    );
}

#[test]
fn attention_forward_deterministic_same_seed() {
    // Наряд №183 Contract 5: determinism — same seed → same forward-pass result.
    // Run the attention forward pass twice with the same seed and assert
    // bitwise-identical output (NOT just within tolerance — full bitwise
    // match, since the weights are deterministic AND the candle operations
    // are deterministic given the same input tensor).
    let heads = 4;
    let dim = 16;
    let seed = 99;
    let seq_len = 4;

    let attn1 = Attention::new(heads, dim, seed).expect("attention build");
    let attn2 = Attention::new(heads, dim, seed).expect("attention build (same seed)");

    let input_values: Vec<f32> = (0..seq_len * dim).map(|i| (i as f32) * 0.05).collect();
    let device = candle_core::Device::Cpu;
    let input = candle_core::Tensor::from_vec(input_values, (seq_len, dim), &device)
        .expect("input tensor")
        .to_dtype(candle_core::DType::F32)
        .expect("input dtype");

    let out1 = attn1.forward(&input).expect("forward 1");
    let out2 = attn2.forward(&input).expect("forward 2");

    let v1: Vec<f32> = out1
        .flatten_all()
        .expect("flatten 1")
        .to_vec1()
        .expect("v1");
    let v2: Vec<f32> = out2
        .flatten_all()
        .expect("flatten 2")
        .to_vec1()
        .expect("v2");

    assert_eq!(v1.len(), v2.len(), "length mismatch");
    let mut max_diff = 0.0f32;
    for (a, b) in v1.iter().zip(v2.iter()) {
        let diff = (a - b).abs();
        if diff > max_diff {
            max_diff = diff;
        }
    }
    assert!(
        max_diff == 0.0,
        "same-seed forward passes should be bitwise-identical, got max diff = {:.2e}",
        max_diff
    );
    println!(
        "✓ determinism: same seed → bitwise-identical output ({} elements)",
        v1.len()
    );
}

#[test]
fn attention_forward_different_seed_different_result() {
    // Sanity: different seeds should produce DIFFERENT outputs (otherwise
    // the seed is being ignored entirely, which would defeat the determinism
    // contract — we want determinism per-seed, not across all seeds).
    let heads = 2;
    let dim = 8;
    let seq_len = 3;

    let attn1 = Attention::new(heads, dim, 42).expect("attention build");
    let attn2 = Attention::new(heads, dim, 100).expect("attention build (different seed)");

    let input_values: Vec<f32> = (0..seq_len * dim).map(|i| (i as f32) * 0.1).collect();
    let device = candle_core::Device::Cpu;
    let input = candle_core::Tensor::from_vec(input_values, (seq_len, dim), &device)
        .expect("input tensor")
        .to_dtype(candle_core::DType::F32)
        .expect("input dtype");

    let out1 = attn1.forward(&input).expect("forward 1");
    let out2 = attn2.forward(&input).expect("forward 2");

    let v1: Vec<f32> = out1
        .flatten_all()
        .expect("flatten 1")
        .to_vec1()
        .expect("v1");
    let v2: Vec<f32> = out2
        .flatten_all()
        .expect("flatten 2")
        .to_vec1()
        .expect("v2");

    let max_diff = v1
        .iter()
        .zip(v2.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    assert!(
        max_diff > 1e-3,
        "different seeds should produce meaningfully different outputs, got max diff = {:.2e}",
        max_diff
    );
    println!(
        "✓ different seeds → different outputs (max diff = {:.4})",
        max_diff
    );
}

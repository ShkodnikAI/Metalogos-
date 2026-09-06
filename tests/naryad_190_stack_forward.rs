// ── Наряд №190 Contract 2: stacked transformer_block forward — numerical correctness ──
//
// Per the naryad spec:
//   "forward pass через 2–3 transformer_block подряд, сравнение с
//    независимым NumPy-референсом (тот же принцип, что наряды №183/184)"
//
// Uses the forward-only TransformerBlock (Наряд №184) — not the
// Trainable variant. Two blocks stacked, each with its own seed
// (matching Rust's decl.seed.wrapping_add(i) pattern).
//
// The reference is computed by an independent NumPy script
// (scripts/naryad_190_stack_reference.py) — not self-confirming.

#![cfg(feature = "candle")]

use metalogos::nn::sequence_layer::SequenceLayer;
use metalogos::nn::transformer_block::TransformerBlock;

#[test]
fn stack_forward_matches_numpy_reference() {
    let heads = 2;
    let dim = 8;
    let ff_dim = 16;
    let seq_len = 3;
    let base_seed: u64 = 42;

    // Build two blocks with different seeds (matches Rust's
    // decl.seed.wrapping_add(i) pattern for stacked layers).
    let block0 =
        TransformerBlock::new(heads, dim, ff_dim, base_seed.wrapping_add(0)).expect("block0 build");
    let block1 =
        TransformerBlock::new(heads, dim, ff_dim, base_seed.wrapping_add(1)).expect("block1 build");

    let input_values: Vec<f32> = (0..seq_len * dim).map(|i| (i as f32) * 0.1).collect();
    let device = candle_core::Device::Cpu;
    let input = candle_core::Tensor::from_vec(input_values, (seq_len, dim), &device)
        .expect("input tensor")
        .to_dtype(candle_core::DType::F32)
        .expect("input dtype");

    // Forward through both blocks.
    let out0 = block0.forward(&input).expect("block0 forward");
    let out1 = block1.forward(&out0).expect("block1 forward");

    let output_vec: Vec<f32> = out1
        .flatten_all()
        .expect("flatten")
        .to_vec1()
        .expect("to_vec1");

    // Expected output (computed by scripts/naryad_190_stack_reference.py).
    // 24 values: [3, 8] flattened row-major.
    let expected: &[f32] = &[
        -0.43299732,
        -0.13648918,
        0.30969387,
        0.18327263,
        0.6633129,
        0.36460525,
        0.82777315,
        1.1411319,
        0.47453704,
        0.59039325,
        1.0214727,
        1.0457351,
        1.3807,
        1.0212153,
        1.7214522,
        1.8841603,
        1.344958,
        1.3878195,
        1.7790096,
        1.8148196,
        2.1767404,
        1.8122721,
        2.5206678,
        2.6678925,
    ];

    assert_eq!(output_vec.len(), expected.len(), "output length mismatch");

    let mut max_diff = 0.0f32;
    for (i, (actual, expected)) in output_vec.iter().zip(expected.iter()).enumerate() {
        let diff = (actual - expected).abs();
        if diff > max_diff {
            max_diff = diff;
        }
        assert!(
            diff < 1e-3,
            "stack output[{}] mismatch: actual={:.8}, expected={:.8}, diff={:.2e}",
            i,
            actual,
            expected,
            diff
        );
    }
    println!(
        "✓ stacked transformer_block forward matches NumPy reference (max diff = {:.2e}, tol = 1e-3)",
        max_diff
    );
}

#[test]
fn stack_forward_deterministic_same_seed() {
    let heads = 2;
    let dim = 8;
    let ff_dim = 16;
    let seq_len = 3;
    let base_seed: u64 = 42;

    let b0a = TransformerBlock::new(heads, dim, ff_dim, base_seed.wrapping_add(0)).unwrap();
    let b1a = TransformerBlock::new(heads, dim, ff_dim, base_seed.wrapping_add(1)).unwrap();
    let b0b = TransformerBlock::new(heads, dim, ff_dim, base_seed.wrapping_add(0)).unwrap();
    let b1b = TransformerBlock::new(heads, dim, ff_dim, base_seed.wrapping_add(1)).unwrap();

    let input_values: Vec<f32> = (0..seq_len * dim).map(|i| (i as f32) * 0.1).collect();
    let device = candle_core::Device::Cpu;
    let input = candle_core::Tensor::from_vec(input_values, (seq_len, dim), &device)
        .unwrap()
        .to_dtype(candle_core::DType::F32)
        .unwrap();

    let out_a = b1a.forward(&b0a.forward(&input).unwrap()).unwrap();
    let out_b = b1b.forward(&b0b.forward(&input).unwrap()).unwrap();

    let va: Vec<f32> = out_a.flatten_all().unwrap().to_vec1().unwrap();
    let vb: Vec<f32> = out_b.flatten_all().unwrap().to_vec1().unwrap();

    let max_diff = va
        .iter()
        .zip(vb.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    assert!(
        max_diff == 0.0,
        "same seed → bitwise-identical, got {:.2e}",
        max_diff
    );
    println!(
        "✓ stack determinism: bitwise-identical ({} elements)",
        va.len()
    );
}

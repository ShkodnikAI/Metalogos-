// ── Наряд №188 Contract 1: GQA forward pass — numerical correctness ──
//
// Per the naryad spec:
//   "численная корректность против независимого NumPy-референса (тот же
//    принцип, что наряды №183/184), на реальном GQA-случае
//    (n_kv_heads < n_heads)."
//
// Config: heads=4, n_kv_heads=2, dim=16, head_dim=4, seq_len=3, seed=42.
// n_rep = n_heads / n_kv_heads = 2 — each KV head is shared by 2 Q heads.
//
// The expected output is computed by an independent NumPy script
// (scripts/naryad_188_gqa_reference.py), using the same xorshift64 PRNG
// and the same GQA algorithm (repeat_kv via np.repeat with interleave).
//
// The reference is intentionally NOT a candle-internal implementation.

#![cfg(feature = "candle")]

use metalogos::nn::attention::Attention;
use metalogos::nn::sequence_layer::SequenceLayer;

#[test]
fn gqa_forward_matches_numpy_reference() {
    let heads = 4;
    let n_kv_heads = 2;
    let dim = 16;
    let seed = 42;
    let seq_len = 3;

    let attn = Attention::new_with_kv_heads(heads, n_kv_heads, dim, seed)
        .expect("attention build with GQA");

    let input_values: Vec<f32> = (0..seq_len * dim).map(|i| (i as f32) * 0.1).collect();
    let device = candle_core::Device::Cpu;
    let input = candle_core::Tensor::from_vec(input_values, (seq_len, dim), &device)
        .expect("input tensor")
        .to_dtype(candle_core::DType::F32)
        .expect("input dtype");

    let output = attn.forward(&input).expect("forward");
    let output_vec: Vec<f32> = output
        .flatten_all()
        .expect("flatten")
        .to_vec1()
        .expect("to_vec1");

    // Expected output (computed by scripts/naryad_188_gqa_reference.py).
    // 48 values: [3, 16] flattened row-major.
    let expected: &[f32] = &[
        1.5311989,
        1.0272123,
        1.225504,
        0.32987532,
        -0.45663333,
        0.58710355,
        0.7822695,
        0.41635844,
        1.0664829,
        0.91267824,
        0.33867666,
        -0.5147407,
        0.4195591,
        0.7426854,
        1.2642871,
        -1.8920395,
        2.0258763,
        1.5891633,
        1.2639407,
        0.47965747,
        -0.8140224,
        0.61320376,
        1.2196704,
        0.17679904,
        1.1272203,
        1.1835957,
        0.7221393,
        -0.60482824,
        0.21139652,
        1.1204592,
        1.323615,
        -1.9694889,
        1.837558,
        1.8479205,
        1.0275289,
        0.60222155,
        -0.68658644,
        0.51717764,
        0.94862247,
        0.048101746,
        0.8258162,
        1.1567628,
        0.74937004,
        -0.42181745,
        -0.1603918,
        1.0648143,
        1.2427286,
        -1.8402064,
    ];

    assert_eq!(output_vec.len(), expected.len(), "output length mismatch");

    let mut max_diff = 0.0f32;
    for (i, (actual, expected)) in output_vec.iter().zip(expected.iter()).enumerate() {
        let diff = (actual - expected).abs();
        if diff > max_diff {
            max_diff = diff;
        }
        assert!(
            diff < 1e-4,
            "gqa output[{}] mismatch: actual={:.8}, expected={:.8}, diff={:.2e}",
            i,
            actual,
            expected,
            diff
        );
    }
    println!(
        "✓ GQA forward matches NumPy reference (max diff = {:.2e}, tolerance = 1e-4)",
        max_diff
    );
}

#[test]
fn gqa_forward_deterministic_same_seed() {
    let heads = 4;
    let n_kv_heads = 2;
    let dim = 16;
    let seq_len = 3;

    let a1 = Attention::new_with_kv_heads(heads, n_kv_heads, dim, 42).expect("build 1");
    let a2 = Attention::new_with_kv_heads(heads, n_kv_heads, dim, 42).expect("build 2 (same seed)");

    let input_values: Vec<f32> = (0..seq_len * dim).map(|i| (i as f32) * 0.1).collect();
    let device = candle_core::Device::Cpu;
    let input = candle_core::Tensor::from_vec(input_values, (seq_len, dim), &device)
        .expect("input")
        .to_dtype(candle_core::DType::F32)
        .expect("dtype");

    let o1 = a1.forward(&input).expect("forward 1");
    let o2 = a2.forward(&input).expect("forward 2");

    let v1: Vec<f32> = o1.flatten_all().expect("f1").to_vec1().expect("v1");
    let v2: Vec<f32> = o2.flatten_all().expect("f2").to_vec1().expect("v2");

    let max_diff = v1
        .iter()
        .zip(v2.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    assert!(
        max_diff == 0.0,
        "same seed → bitwise-identical, got {:.2e}",
        max_diff
    );
    println!(
        "✓ GQA determinism: bitwise-identical ({} elements)",
        v1.len()
    );
}

#[test]
fn gqa_validation_errors() {
    // n_kv_heads == 0
    assert!(Attention::new_with_kv_heads(4, 0, 16, 42).is_err());

    // n_kv_heads > n_heads
    assert!(Attention::new_with_kv_heads(2, 4, 16, 42).is_err());

    // n_heads not divisible by n_kv_heads
    assert!(Attention::new_with_kv_heads(4, 3, 16, 42).is_err());

    // dim not divisible by heads
    assert!(Attention::new_with_kv_heads(4, 2, 10, 42).is_err());

    println!("✓ GQA validation: all invalid configs rejected");
}

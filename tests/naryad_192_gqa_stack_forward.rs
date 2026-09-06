// ── Наряд №192 Contract 1: GQA + stack forward — numerical correctness ──
//
// Stack of 2 attention layers:
//   Block 0: attention(2, 8)     — standard MHA (n_kv_heads == n_heads == 2)
//   Block 1: attention(4, 8, 2)  — GQA (n_heads=4, n_kv_heads=2, n_rep=2)
//
// This is the integration test that Наряды №188 (GQA) and №190 (stack)
// did NOT cover together: GQA inside a multi-layer stack.
//
// The expected output is computed by an independent NumPy script
// (scripts/naryad_192_gqa_stack_reference.py).

#![cfg(feature = "candle")]

use metalogos::nn::attention::Attention;
use metalogos::nn::sequence_layer::SequenceLayer;

#[test]
fn gqa_stack_forward_matches_numpy_reference() {
    let dim = 8;
    let seq_len = 3;
    let base_seed: u64 = 42;

    // Block 0: standard MHA — attention(2, 8) → n_heads=2, n_kv_heads=2
    let block0 = Attention::new_with_kv_heads(2, 2, dim, base_seed.wrapping_add(0))
        .expect("block0 (MHA) build");

    // Block 1: GQA — attention(4, 8, 2) → n_heads=4, n_kv_heads=2, n_rep=2
    let block1 = Attention::new_with_kv_heads(4, 2, dim, base_seed.wrapping_add(1))
        .expect("block1 (GQA) build");

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

    // Expected output (computed by scripts/naryad_192_gqa_stack_reference.py).
    let expected: &[f32] = &[
        -0.03695687,
        0.11266726,
        0.06416641,
        0.027132606,
        0.03129952,
        -0.020175625,
        -0.035345383,
        -0.0684315,
        -0.036978144,
        0.1126156,
        0.06405625,
        0.027184162,
        0.03127965,
        -0.020112008,
        -0.035274304,
        -0.06841285,
        -0.036986783,
        0.11258051,
        0.06400356,
        0.027224569,
        0.031260688,
        -0.020071615,
        -0.035235893,
        -0.06842099,
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
            "gqa_stack output[{}] mismatch: actual={:.8}, expected={:.8}, diff={:.2e}",
            i,
            actual,
            expected,
            diff
        );
    }
    println!(
        "✓ GQA+stack forward matches NumPy reference (max diff = {:.2e}, tol = 1e-4)",
        max_diff
    );
}

#[test]
fn gqa_stack_forward_deterministic() {
    let dim = 8;
    let seq_len = 3;
    let base_seed: u64 = 42;

    let b0a = Attention::new_with_kv_heads(2, 2, dim, base_seed.wrapping_add(0)).unwrap();
    let b1a = Attention::new_with_kv_heads(4, 2, dim, base_seed.wrapping_add(1)).unwrap();
    let b0b = Attention::new_with_kv_heads(2, 2, dim, base_seed.wrapping_add(0)).unwrap();
    let b1b = Attention::new_with_kv_heads(4, 2, dim, base_seed.wrapping_add(1)).unwrap();

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
        "✓ GQA+stack determinism: bitwise-identical ({} elements)",
        va.len()
    );
}

// ── Наряд №190 Contract 3: single transformer_block regression ──────
//
// Per the naryad spec:
//   "стек из ОДНОГО transformer_block даёт идентичный результат наряду
//    №184 (побайтовое/численное совпадение с уже существующим
//    naryad_184_transformer_block_forward.rs, не новый тест с нуля)."
//
// This test verifies that the prefix fix (Наряд №190) did NOT change
// the forward-pass behaviour for a single-block model. A single
// transformer_block registered with prefix "block0" must produce the
// same output as the original Наряд №184 test (which had no prefix
// concept — it used the forward-only TransformerBlock directly).
//
// We compare against the SAME expected values as naryad_184:
//   tests/naryad_184_transformer_block_forward.rs
//
// The forward-only TransformerBlock (Наряд №184) is unchanged by this
// naryad — it doesn't use VarMap at all. This test uses that path to
// confirm numerical identity.

#![cfg(feature = "candle")]

use metalogos::nn::sequence_layer::SequenceLayer;
use metalogos::nn::transformer_block::TransformerBlock;

#[test]
fn single_block_matches_naryad_184_values() {
    // Same config as naryad_184_transformer_block_forward.rs:
    let heads = 2;
    let dim = 8;
    let ff_dim = 16;
    let seq_len = 3;
    let seed = 42;

    let block = TransformerBlock::new(heads, dim, ff_dim, seed).expect("single block build");

    let input_values: Vec<f32> = (0..seq_len * dim).map(|i| (i as f32) * 0.1).collect();
    let device = candle_core::Device::Cpu;
    let input = candle_core::Tensor::from_vec(input_values, (seq_len, dim), &device)
        .expect("input tensor")
        .to_dtype(candle_core::DType::F32)
        .expect("input dtype");

    let output = block.forward(&input).expect("forward");
    let output_vec: Vec<f32> = output
        .flatten_all()
        .expect("flatten")
        .to_vec1()
        .expect("to_vec1");

    // Expected output (from naryad_184_transformer_block_forward.rs —
    // the SAME values that test asserts against).
    let expected: &[f32] = &[
        -0.1614139,
        0.084099635,
        0.15010984,
        0.43465626,
        0.5716741,
        0.33933592,
        1.0833374,
        0.9571686,
        0.7869828,
        0.9706281,
        0.8331813,
        1.1275158,
        1.3708284,
        1.033035,
        1.9105866,
        1.6622657,
        1.6382306,
        1.8041263,
        1.588619,
        1.8969606,
        2.172858,
        1.8068917,
        2.7007723,
        2.4416504,
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
            "single block output[{}] mismatch: actual={:.8}, expected={:.8}, diff={:.2e}",
            i,
            actual,
            expected,
            diff
        );
    }
    println!(
        "✓ single transformer_block matches Наряд №184 values (max diff = {:.2e})",
        max_diff
    );
}

#[test]
fn single_block_via_trainable_path_matches_forward_only() {
    // Verify the trainable path (TrainableTransformerBlock with prefix)
    // produces the same forward output as the forward-only path
    // (TransformerBlock from Наряд №184). This confirms the prefix
    // fix didn't change numerical behaviour.
    use candle_nn::VarMap;
    use metalogos::nn::trainable_transformer_block::TrainableTransformerBlock;

    let heads = 2;
    let dim = 8;
    let ff_dim = 16;
    let seq_len = 3;
    let seed = 42;

    // Forward-only block (Наряд №184, no VarMap).
    let fwd_block = TransformerBlock::new(heads, dim, ff_dim, seed).expect("fwd block");

    // Trainable block with prefix "block0" (Наряд №190).
    let var_map = VarMap::new();
    let train_block = TrainableTransformerBlock::new(heads, dim, ff_dim, seed, &var_map, "block0")
        .expect("trainable block");

    let input_values: Vec<f32> = (0..seq_len * dim).map(|i| (i as f32) * 0.1).collect();
    let device = candle_core::Device::Cpu;
    let input = candle_core::Tensor::from_vec(input_values, (seq_len, dim), &device)
        .unwrap()
        .to_dtype(candle_core::DType::F32)
        .unwrap();

    let fwd_out = fwd_block.forward(&input).unwrap();
    let train_out = train_block.forward(&input).unwrap();

    let fwd_v: Vec<f32> = fwd_out.flatten_all().unwrap().to_vec1().unwrap();
    let train_v: Vec<f32> = train_out.flatten_all().unwrap().to_vec1().unwrap();

    assert_eq!(fwd_v.len(), train_v.len());
    let mut max_diff = 0.0f32;
    for (i, (a, b)) in fwd_v.iter().zip(train_v.iter()).enumerate() {
        let diff = (a - b).abs();
        if diff > max_diff {
            max_diff = diff;
        }
        assert!(
            diff < 1e-4,
            "fwd vs trainable output[{}] mismatch: diff={:.2e}",
            i,
            diff
        );
    }
    println!(
        "✓ trainable (with prefix) matches forward-only (max diff = {:.2e})",
        max_diff
    );
}

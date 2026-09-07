// ── Наряд №194 Contract 2: vocab overflow detection ──────────────────
//
// When a token ID ≥ vocab_size is used with reflex_generate, the
// model should produce a clear error — not silent truncation or
// embedding lookup failure.
//
// This test uses the Rust API directly (not `mlog run`) to verify
// the embedding lookup path catches out-of-range token IDs.

#![cfg(feature = "candle")]

use metalogos::nn::gen_model::ReflexGenModel;
use metalogos::nn::sequence_layer::SequenceLayer;

use candle_core::Device;
use candle_nn::{VarBuilder, VarMap};

fn make_small_vocab_model(vocab_size: usize) -> ReflexGenModel {
    let dim = 8;
    let var_map = VarMap::new();
    let device = Device::Cpu;
    let vb = VarBuilder::from_varmap(&var_map, candle_core::DType::F32, &device);

    let block: Box<dyn SequenceLayer> = Box::new(
        metalogos::nn::trainable_transformer_block::TrainableTransformerBlock::new(
            2, dim, 16, 42, &var_map, "block0",
        )
        .expect("block0"),
    );

    ReflexGenModel::new(
        "overflow_test".to_string(),
        dim,
        vocab_size,
        42,
        vec![block],
        var_map,
        &vb,
    )
    .expect("reflex gen model build")
}

#[test]
fn vocab_overflow_generate_errors() {
    // Model with vocab_size=10 — only token IDs 0..9 are valid.
    let model = make_small_vocab_model(10);

    // Prompt includes token 15, which is ≥ vocab_size (10).
    // This should produce a clean error, not silent corruption.
    let prompt: Vec<u32> = vec![1, 2, 15]; // 15 >= 10
    let result = model.generate_greedy(&prompt, 2);

    assert!(
        result.is_err(),
        "generate_greedy with out-of-range token should error, got {:?}",
        result
    );
    let err = result.unwrap_err();
    println!("✓ vocab overflow error: {}", err);

    // The error should be informative — mention the problem (embedding
    // lookup failure, out of range, or similar).
    // We don't assert exact text since the error comes from candle's
    // embedding() — but it should not be an empty string.
    assert!(!err.is_empty(), "error should not be empty");
}

#[test]
fn vocab_boundary_ok() {
    // Token ID = vocab_size - 1 should work (valid, last index).
    let model = make_small_vocab_model(10);
    let prompt: Vec<u32> = vec![9]; // 9 = vocab_size - 1 = valid
    let result = model.generate_greedy(&prompt, 1);
    assert!(
        result.is_ok(),
        "token ID = vocab_size - 1 should be valid, got error: {:?}",
        result.err()
    );
    println!("✓ token ID 9 (vocab_size-1) is valid for vocab_size=10");
}

#[test]
fn vocab_zero_id_ok() {
    // Token ID 0 should always work.
    let model = make_small_vocab_model(10);
    let prompt: Vec<u32> = vec![0];
    let result = model.generate_greedy(&prompt, 1);
    assert!(result.is_ok(), "token ID 0 should be valid");
    println!("✓ token ID 0 is valid");
}

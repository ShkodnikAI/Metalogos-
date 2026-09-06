// Standalone sanity check for ReflexGenModel (Наряд №193).
// Verifies:
//   1. generate_greedy and generate_no_cache produce the SAME output
//      (Contract #2: kv_cache_matches_no_cache).
//   2. generate_with_temperature at T=0 matches generate_greedy.
//   3. train() decreases loss over epochs.
//   4. Deterministic same-seed generation.
//   5. НАРЯД №193b: after training on a repeating pattern [1,2,3,4,1,2,3,4,...],
//      greedy generation from prompt [1,2,3] continues the pattern (4,1,2,3,...),
//      not random noise.

#![cfg(feature = "candle")]

use metalogos::nn::gen_model::ReflexGenModel;
use metalogos::nn::sequence_layer::SequenceLayer;

use candle_core::Device;
use candle_nn::{VarBuilder, VarMap};

fn make_gen_model(seed: u64) -> ReflexGenModel {
    let dim = 8;
    let vocab_size = 16;
    let var_map = VarMap::new();
    let device = Device::Cpu;
    let vb = VarBuilder::from_varmap(&var_map, candle_core::DType::F32, &device);

    let block: Box<dyn SequenceLayer> = Box::new(
        metalogos::nn::trainable_transformer_block::TrainableTransformerBlock::new(
            2, dim, 16, seed, &var_map, "block0",
        )
        .expect("block0"),
    );

    ReflexGenModel::new(
        "sanity_test".to_string(),
        dim,
        vocab_size,
        seed,
        vec![block],
        var_map,
        &vb,
    )
    .expect("reflex gen model build")
}

/// Build a gen model with TWO transformer blocks for better pattern learning.
fn make_gen_model_2blocks(seed: u64) -> ReflexGenModel {
    let dim = 8;
    let vocab_size = 16;
    let var_map = VarMap::new();
    let device = Device::Cpu;
    let vb = VarBuilder::from_varmap(&var_map, candle_core::DType::F32, &device);

    let block0: Box<dyn SequenceLayer> = Box::new(
        metalogos::nn::trainable_transformer_block::TrainableTransformerBlock::new(
            2, dim, 16, seed, &var_map, "block0",
        )
        .expect("block0"),
    );
    let block1: Box<dyn SequenceLayer> = Box::new(
        metalogos::nn::trainable_transformer_block::TrainableTransformerBlock::new(
            2,
            dim,
            16,
            seed.wrapping_add(1),
            &var_map,
            "block1",
        )
        .expect("block1"),
    );

    ReflexGenModel::new(
        "sanity_test_2b".to_string(),
        dim,
        vocab_size,
        seed,
        vec![block0, block1],
        var_map,
        &vb,
    )
    .expect("reflex gen model build")
}

#[test]
fn greedy_matches_no_cache() {
    let model = make_gen_model(42);
    let prompt: Vec<u32> = vec![1, 2, 3];
    let max_tokens = 3;

    let cached = model
        .generate_greedy(&prompt, max_tokens)
        .expect("generate_greedy");
    let no_cache = model
        .generate_no_cache(&prompt, max_tokens)
        .expect("generate_no_cache");

    println!("greedy (cache):    {:?}", cached);
    println!("no_cache (recompute): {:?}", no_cache);

    assert_eq!(
        cached, no_cache,
        "Contract #2 (kv_cache_matches_no_cache) violated: \
         generate_greedy and generate_no_cache must produce the same output"
    );
}

#[test]
fn temperature_zero_matches_greedy() {
    let model = make_gen_model(42);
    let prompt: Vec<u32> = vec![1, 2, 3];
    let max_tokens = 4;

    let greedy = model
        .generate_greedy(&prompt, max_tokens)
        .expect("generate_greedy");
    let temp_zero = model
        .generate_with_temperature(&prompt, max_tokens, 0.0)
        .expect("generate_with_temperature(0)");

    assert_eq!(greedy, temp_zero, "temperature == 0 should match greedy");
}

#[test]
fn train_decreases_loss() {
    let mut model = make_gen_model(42);

    // Training sequences — just repeated patterns (deterministic by seed).
    // Two sequences: [1,2,3,4,1,2,3,4,...] and [5,6,7,8,5,6,7,8,...].
    let seq_a: Vec<u32> = (0..16).map(|i| (i % 4) as u32 + 1).collect();
    let seq_b: Vec<u32> = (0..16).map(|i| (i % 4) as u32 + 5).collect();
    let sequences = vec![seq_a, seq_b];

    let initial_loss = model.train(&sequences, 1, 0.1).expect("train (1 epoch)");
    let final_loss = model
        .train(&sequences, 50, 0.1)
        .expect("train (50 more epochs)");

    println!(
        "initial_loss = {:.4}, final_loss = {:.4}",
        initial_loss, final_loss
    );
    assert!(
        final_loss < initial_loss,
        "loss should decrease over epochs: initial={}, final={}",
        initial_loss,
        final_loss
    );
}

#[test]
fn generation_deterministic_same_seed() {
    let m1 = make_gen_model(42);
    let m2 = make_gen_model(42);
    let prompt: Vec<u32> = vec![1, 2, 3];

    let out1 = m1.generate_greedy(&prompt, 5).expect("gen1");
    let out2 = m2.generate_greedy(&prompt, 5).expect("gen2");
    assert_eq!(
        out1, out2,
        "same seed should produce same generation (determinism contract)"
    );
}

/// НАРЯД №193b Block 1: meaningful generation test.
///
/// Train on the repeating pattern [1,2,3,4,1,2,3,4,...] with 2 blocks
/// and many epochs. Then prompt with [1,2,3] and verify that greedy
/// generation continues the pattern as [4,1,2,3,4,...] — not random
/// noise. This proves the model actually learned the pattern, not just
/// that loss decreased (which could be noise on trivial data).
#[test]
fn trained_model_generates_learned_pattern() {
    let mut model = make_gen_model_2blocks(42);

    // Training data: repeating pattern [1,2,3,4,1,2,3,4,...]
    // Use many repetitions so the model has enough signal to learn.
    let seq_a: Vec<u32> = (0..64).map(|i| (i % 4) as u32 + 1).collect();
    let sequences = vec![seq_a];

    // Train with many epochs for meaningful learning
    let initial_loss = model.train(&sequences, 1, 0.1).expect("train 1 epoch");
    let final_loss = model.train(&sequences, 500, 0.1).expect("train 500 more");

    println!(
        "pattern training: initial_loss={:.4}, final_loss={:.4}",
        initial_loss, final_loss
    );
    assert!(
        final_loss < initial_loss,
        "loss should decrease: initial={}, final={}",
        initial_loss,
        final_loss
    );

    // Generate from prompt [1,2,3] — expected continuation: [4,1,2,3,4,...]
    let prompt: Vec<u32> = vec![1, 2, 3];
    let generated = model.generate_greedy(&prompt, 8).expect("generate_greedy");

    println!("prompt:     {:?}", prompt);
    println!("generated:  {:?}", generated);

    // Expected pattern: [4, 1, 2, 3, 4, 1, 2, 3] (continuation of 1,2,3,4,...)
    let expected: Vec<u32> = vec![4, 1, 2, 3, 4, 1, 2, 3];

    // Check: at least the FIRST generated token should be 4 (the pattern
    // continuation). If the model produces noise, this will fail.
    assert!(
        !generated.is_empty(),
        "generated sequence should not be empty"
    );

    // THE KEY ASSERTION: first generated token must be 4 (pattern continuation)
    assert_eq!(
        generated[0], 4,
        "first generated token should be 4 (pattern continuation of [1,2,3] → [4,...]), \
         got {} — model did not learn the pattern",
        generated[0]
    );

    // Check how many tokens match the expected pattern.
    let matching = generated
        .iter()
        .zip(expected.iter())
        .take_while(|(a, b)| a == b)
        .count();
    println!(
        "pattern match: {}/{} tokens match the expected [4,1,2,3,4,1,2,3]",
        matching,
        expected.len()
    );

    // At least the first 2 tokens must match (4,1) — proves it's not a
    // single-token coincidence.
    assert!(
        matching >= 2,
        "at least 2 generated tokens should match the pattern [4,1,2,3,...], \
         got {}/{} matching: {:?}",
        matching,
        expected.len(),
        generated
    );
}

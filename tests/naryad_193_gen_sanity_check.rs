// Standalone sanity check for ReflexGenModel (Наряд №193).
// Not part of the project test suite — temporary verification only.
// Verifies:
//   1. Model constructs.
//   2. generate_greedy and generate_no_cache produce the SAME output
//      (Contract #2: kv_cache_matches_no_cache).
//   3. generate_with_temperature at T=0 matches generate_greedy.
//   4. train() decreases loss over epochs.
//
// Run with:
//   cargo test --features candle --test naryad_193_gen_sanity_check

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

#[test]
fn greedy_matches_no_cache() {
    let model = make_gen_model(42);
    let prompt: Vec<u32> = vec![1, 3, 5, 7];
    let max_tokens = 6;

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
    let prompt: Vec<u32> = vec![1, 3, 5, 7];
    let max_tokens = 5;

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

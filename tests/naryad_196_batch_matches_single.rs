// ── Наряд №196 Contract 1: batch_size=1 matches single-sequence path ──
//
// train_batch(batch_size=1) must give the same loss as train() (single).
// This proves the batch path doesn't change numerics — only efficiency.

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
        "batch_test".to_string(),
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
fn batch_size_1_matches_single_train() {
    let sequences: Vec<Vec<u32>> = vec![vec![1, 2, 3, 4, 1, 2, 3, 4], vec![5, 6, 7, 8, 5, 6, 7, 8]];

    // Train single (existing path)
    let mut model_single = make_gen_model(42);
    let loss_single = model_single
        .train(&sequences, 10, 0.1)
        .expect("train single");

    // Train batch with batch_size=1 (new path)
    let mut model_batch = make_gen_model(42);
    let loss_batch = model_batch
        .train_batch(&sequences, 10, 0.1, 1)
        .expect("train_batch bs=1");

    println!(
        "single loss = {:.6}, batch(bs=1) loss = {:.6}, diff = {:.2e}",
        loss_single,
        loss_batch,
        (loss_single - loss_batch).abs()
    );

    // Numerical match (not bitwise — f32 can have tiny ordering differences)
    assert!(
        (loss_single - loss_batch).abs() < 1e-4,
        "batch_size=1 should match single-sequence path: single={}, batch={}",
        loss_single,
        loss_batch
    );
    println!("✓ batch_size=1 matches single-sequence training");
}

#[test]
fn batch_size_1_deterministic() {
    let sequences: Vec<Vec<u32>> = vec![vec![1, 2, 3, 4, 1, 2, 3, 4]];

    let mut m1 = make_gen_model(42);
    let mut m2 = make_gen_model(42);

    let l1 = m1.train_batch(&sequences, 5, 0.1, 1).expect("train 1");
    let l2 = m2.train_batch(&sequences, 5, 0.1, 1).expect("train 2");

    assert!((l1 - l2).abs() < 1e-6, "deterministic: {} vs {}", l1, l2);
    println!("✓ batch_size=1 deterministic: loss={:.6}", l1);
}

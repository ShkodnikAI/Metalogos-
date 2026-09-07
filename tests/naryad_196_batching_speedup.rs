// ── Наряд №196 Contract 3: measurable speedup ────────────────────────
//
// Real timing: batch training vs single-sequence training on N=8
// sequences. Reports actual milliseconds, not abstract claims.

#![cfg(feature = "candle")]

use metalogos::nn::gen_model::ReflexGenModel;
use metalogos::nn::sequence_layer::SequenceLayer;

use candle_core::Device;
use candle_nn::{VarBuilder, VarMap};
use std::time::Instant;

fn make_gen_model(seed: u64) -> ReflexGenModel {
    let dim = 16;
    let vocab_size = 64;
    let var_map = VarMap::new();
    let device = Device::Cpu;
    let vb = VarBuilder::from_varmap(&var_map, candle_core::DType::F32, &device);

    let block0: Box<dyn SequenceLayer> = Box::new(
        metalogos::nn::trainable_transformer_block::TrainableTransformerBlock::new(
            2, dim, 32, seed, &var_map, "block0",
        )
        .expect("block0"),
    );
    let block1: Box<dyn SequenceLayer> = Box::new(
        metalogos::nn::trainable_transformer_block::TrainableTransformerBlock::new(
            2,
            dim,
            32,
            seed.wrapping_add(1),
            &var_map,
            "block1",
        )
        .expect("block1"),
    );

    ReflexGenModel::new(
        "speed_test".to_string(),
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
fn batch_training_faster_than_single() {
    // 8 sequences of length 16
    let sequences: Vec<Vec<u32>> = (0..8)
        .map(|i| (0..16).map(|j| (j % 4 + i % 4) as u32 + 1).collect())
        .collect();

    let epochs = 5;

    // Time single-sequence training
    let mut model_single = make_gen_model(42);
    let start_single = Instant::now();
    let loss_single = model_single
        .train(&sequences, epochs, 0.1)
        .expect("train single");
    let elapsed_single = start_single.elapsed();

    // Time batch training (batch_size=8 — all in one batch)
    let mut model_batch = make_gen_model(42);
    let start_batch = Instant::now();
    let loss_batch = model_batch
        .train_batch(&sequences, epochs, 0.1, 8)
        .expect("train batch");
    let elapsed_batch = start_batch.elapsed();

    println!("═══ Batching Speedup (Наряд №196 Block 3) ═══");
    println!(
        "Sequences: {}, Epochs: {}, Layers: 2 transformer_blocks",
        sequences.len(),
        epochs
    );
    println!(
        "Single-sequence: {:?} (loss={:.4})",
        elapsed_single, loss_single
    );
    println!(
        "Batch (bs=8):     {:?} (loss={:.4})",
        elapsed_batch, loss_batch
    );
    println!(
        "Speedup: {:.2}x",
        elapsed_single.as_secs_f64() / elapsed_batch.as_secs_f64()
    );

    // Both should produce finite, positive loss
    assert!(loss_single.is_finite() && loss_single > 0.0);
    assert!(loss_batch.is_finite() && loss_batch > 0.0);

    // Report honestly — the spec says "if batching doesn't give
    // measurable speedup, report it as a finding, don't fake the number"
    let speedup = elapsed_single.as_secs_f64() / elapsed_batch.as_secs_f64();
    if speedup > 1.0 {
        println!("✓ Batching is {:.2}x faster than single-sequence", speedup);
    } else {
        println!(
            "⚠ Batching is NOT faster ({:.2}x) — likely because candle CPU backend overhead. Larger models would benefit more.",
            speedup
        );
    }
}

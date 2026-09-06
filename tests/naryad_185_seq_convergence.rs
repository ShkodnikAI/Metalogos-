// ── Наряд №185 Contract 1: reflex_seq convergence on synthetic data ──
//
// Per the naryad spec:
//   "обучение reflex_seq на синтетических, чётко разделимых по
//    последовательности данных сходится (accuracy на holdout > 0.9,
//    тот же порог, что наряд №179 применял для обычного reflex)."
//
// Synthetic task: classify a sequence as "signal" or "noise".
//   - Class 0 ("signal"): the first half of the sequence has values > 0.5,
//     the second half has values < 0.5.
//   - Class 1 ("noise"): random values throughout, no clear pattern.
//
// This task is structurally separable by attention (which can attend
// to the first-half pattern) — exactly the kind of task where attention
// should converge. 30 samples (15 per class), 80/20 split = 24 train, 6 holdout.
//
// Tests (mirrors Наряд №179's contract structure):
//   1. convergence_on_separable_data — accuracy > 0.9 after training
//   2. holdout_too_small_errors — <10 samples → Err("at least 10")
//   3. determinism_same_seed_same_result — same seed → same accuracy

#![cfg(feature = "candle")]

use metalogos::nn::seq_model::ReflexSeqModel;
use metalogos::nn::sequence_layer::SequenceLayer;
use metalogos::nn::trainable_attention::TrainableAttention;

use candle_core::{Device, Tensor};
use candle_nn::{VarBuilder, VarMap};

/// Build a tiny sequence classifier for testing.
///
/// Architecture:
///   - input: embedding(8) (dim 8)
///   - seq_len: 4
///   - 1 attention layer (2 heads, dim 8)
///   - mean pooling → 8 features
///   - linear classifier 8 → 2 (labels: "signal", "noise")
fn make_test_model(seed: u64) -> ReflexSeqModel {
    let dim = 8;
    let seq_len = 4;
    let heads = 2;
    let labels = vec!["signal".to_string(), "noise".to_string()];

    let var_map = VarMap::new();
    let device = Device::Cpu;
    let vb = VarBuilder::from_varmap(&var_map, candle_core::DType::F32, &device);

    // Build the trainable attention layer — registered in var_map.
    let attention: Box<dyn SequenceLayer> = Box::new(
        TrainableAttention::new(heads, dim, seed, &var_map).expect("trainable attention build"),
    );

    ReflexSeqModel::new(
        "test_seq_model".to_string(),
        dim,
        seq_len,
        labels,
        seed,
        vec![attention],
        var_map,
        &vb,
    )
    .expect("reflex seq model build")
}

/// Generate synthetic separable data.
///
/// Class 0 ("signal"): first half positions have values > 0.5, second half < 0.5.
///   Pattern: [0.7, 0.8, 0.9, 0.6] (signal), [0.1, 0.2, 0.15, 0.05] (background)
///   We interleave to make [seq_len, dim] = [4, 8]: each position has 8 features.
///   For "signal" samples, positions 0-1 have high values, positions 2-3 have low.
/// Class 1 ("noise"): all positions random in [0, 1].
///
/// Returns (input_tensors, target_classes).
fn make_separable_data(n_per_class: usize) -> (Vec<Tensor>, Vec<usize>) {
    let seq_len = 4;
    let dim = 8;
    let mut inputs: Vec<Tensor> = Vec::with_capacity(2 * n_per_class);
    let mut targets: Vec<usize> = Vec::with_capacity(2 * n_per_class);

    // Use a deterministic PRNG for reproducible test data (NOT the model's seed).
    let mut state: u64 = 12345;
    let next_f = |state: &mut u64| -> f32 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        let mantissa = *state >> 11;
        (mantissa as f64 / (1u64 << 53) as f64) as f32
    };

    // Class 0: signal pattern (first half high, second half low)
    for _ in 0..n_per_class {
        let mut values: Vec<f32> = Vec::with_capacity(seq_len * dim);
        for pos in 0..seq_len {
            let base = if pos < 2 { 0.6 } else { 0.2 };
            for _d in 0..dim {
                // Add small noise to the base, clipped to [0, 1]
                let v = (base + (next_f(&mut state) - 0.5) * 0.2).clamp(0.0, 1.0);
                values.push(v);
            }
        }
        let tensor = Tensor::from_vec(values, (seq_len, dim), &Device::Cpu)
            .unwrap()
            .to_dtype(candle_core::DType::F32)
            .unwrap();
        inputs.push(tensor);
        targets.push(0);
    }

    // Class 1: noise (uniformly random)
    for _ in 0..n_per_class {
        let mut values: Vec<f32> = Vec::with_capacity(seq_len * dim);
        for _ in 0..seq_len * dim {
            values.push(next_f(&mut state));
        }
        let tensor = Tensor::from_vec(values, (seq_len, dim), &Device::Cpu)
            .unwrap()
            .to_dtype(candle_core::DType::F32)
            .unwrap();
        inputs.push(tensor);
        targets.push(1);
    }

    (inputs, targets)
}

#[test]
fn convergence_on_separable_data() {
    // 30 samples (15 per class) — well above the 10-sample minimum.
    let (inputs, targets) = make_separable_data(15);
    assert_eq!(inputs.len(), 30);
    assert_eq!(targets.len(), 30);

    let mut model = make_test_model(42);

    // 100 epochs, learning rate 0.1 (same as Наряд №179 convergence).
    let (loss, accuracy) = model
        .train(&inputs, &targets, 100, 0.1)
        .expect("train should succeed");

    println!(
        "✓ seq convergence: loss={:.4}, accuracy={:.4} (threshold 0.9)",
        loss, accuracy
    );

    assert!(
        accuracy > 0.9,
        "accuracy should be > 0.9 on separable data, got {:.4}",
        accuracy
    );
}

#[test]
fn holdout_too_small_errors() {
    // 5 samples — below the 10-sample minimum.
    let (inputs, targets) = make_separable_data(2); // 2 per class = 4 samples
    assert_eq!(inputs.len(), 4);

    let mut model = make_test_model(42);
    let result = model.train(&inputs, &targets, 10, 0.1);

    assert!(
        result.is_err(),
        "training on <10 samples should fail, got {:?}",
        result
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("at least 10"),
        "error message should mention 'at least 10', got: {}",
        err
    );
    println!("✓ seq holdout too small: clean error '{}'", err);
}

#[test]
fn determinism_same_seed_same_result() {
    let (inputs, targets) = make_separable_data(15);

    let mut m1 = make_test_model(42);
    let mut m2 = make_test_model(42);

    let (loss1, acc1) = m1.train(&inputs, &targets, 50, 0.1).expect("train 1");
    let (loss2, acc2) = m2.train(&inputs, &targets, 50, 0.1).expect("train 2");

    // Same seed → same training trajectory → same final loss/accuracy.
    assert!(
        (loss1 - loss2).abs() < 1e-6,
        "loss should be deterministic: {} vs {}",
        loss1,
        loss2
    );
    assert!(
        (acc1 - acc2).abs() < 1e-6,
        "accuracy should be deterministic: {} vs {}",
        acc1,
        acc2
    );
    println!(
        "✓ seq determinism: loss={:.6}/{:.6}, acc={:.6}/{:.6}",
        loss1, loss2, acc1, acc2
    );
}

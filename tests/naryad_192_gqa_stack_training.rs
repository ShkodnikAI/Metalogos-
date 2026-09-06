// ── Наряд №192 Contract 2: GQA + stack training convergence ─────────
//
// Stack of 2 attention layers (block0 = MHA, block1 = GQA), trained
// via reflex_train. Verifies gradients flow through BOTH blocks,
// including the GQA repeat_kv path in block1.
//
// Same convergence threshold as Наряды №179/185/190: accuracy > 0.9
// on separable data.

#![cfg(feature = "candle")]

use metalogos::nn::seq_model::ReflexSeqModel;
use metalogos::nn::sequence_layer::SequenceLayer;
use metalogos::nn::trainable_attention::TrainableAttention;

use candle_core::{Device, Tensor};
use candle_nn::{VarBuilder, VarMap};

fn make_gqa_stack_model(seed: u64) -> ReflexSeqModel {
    let dim = 8;
    let seq_len = 4;
    let labels = vec!["signal".to_string(), "noise".to_string()];

    let var_map = VarMap::new();
    let device = Device::Cpu;
    let vb = VarBuilder::from_varmap(&var_map, candle_core::DType::F32, &device);

    // Block 0: standard MHA — attention(2, 8) → n_heads=2, n_kv_heads=2
    let block0: Box<dyn SequenceLayer> = Box::new(
        TrainableAttention::new_with_kv_heads(2, 2, dim, seed, &var_map, "layer0")
            .expect("block0 (MHA) build"),
    );

    // Block 1: GQA — attention(4, 8, 2) → n_heads=4, n_kv_heads=2, n_rep=2
    let block1: Box<dyn SequenceLayer> = Box::new(
        TrainableAttention::new_with_kv_heads(4, 2, dim, seed.wrapping_add(1), &var_map, "layer1")
            .expect("block1 (GQA) build"),
    );

    ReflexSeqModel::new(
        "gqa_stack_test".to_string(),
        dim,
        seq_len,
        labels,
        seed,
        vec![block0, block1],
        var_map,
        &vb,
    )
    .expect("gqa stack model build")
}

fn make_separable_data(n_per_class: usize) -> (Vec<Tensor>, Vec<usize>) {
    let seq_len = 4;
    let dim = 8;
    let mut inputs: Vec<Tensor> = Vec::with_capacity(2 * n_per_class);
    let mut targets: Vec<usize> = Vec::with_capacity(2 * n_per_class);

    let mut state: u64 = 12345;
    let next_f = |state: &mut u64| -> f32 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        let mantissa = *state >> 11;
        (mantissa as f64 / (1u64 << 53) as f64) as f32
    };

    for _ in 0..n_per_class {
        let mut values: Vec<f32> = Vec::with_capacity(seq_len * dim);
        for pos in 0..seq_len {
            let base = if pos < 2 { 0.6 } else { 0.2 };
            for _d in 0..dim {
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
fn gqa_stack_training_converges() {
    let (inputs, targets) = make_separable_data(15);
    assert_eq!(inputs.len(), 30);

    let mut model = make_gqa_stack_model(42);

    let (loss, accuracy) = model
        .train(&inputs, &targets, 100, 0.1)
        .expect("gqa stack training should succeed");

    println!(
        "✓ GQA+stack training: loss={:.4}, accuracy={:.4} (threshold 0.9)",
        loss, accuracy
    );

    assert!(
        accuracy > 0.9,
        "gqa stack accuracy should be > 0.9 on separable data, got {:.4}",
        accuracy
    );
}

#[test]
fn gqa_stack_training_deterministic() {
    let (inputs, targets) = make_separable_data(15);

    let mut m1 = make_gqa_stack_model(42);
    let mut m2 = make_gqa_stack_model(42);

    let (loss1, acc1) = m1.train(&inputs, &targets, 50, 0.1).expect("train 1");
    let (loss2, acc2) = m2.train(&inputs, &targets, 50, 0.1).expect("train 2");

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
        "✓ GQA+stack determinism: loss={:.6}/{:.6}, acc={:.6}/{:.6}",
        loss1, loss2, acc1, acc2
    );
}

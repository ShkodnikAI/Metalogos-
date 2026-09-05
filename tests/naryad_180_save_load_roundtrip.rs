// ── Наряд №180: Contract 1 — save/load roundtrip preserves predictions ──
//
// Train a model, save it, load the weights into a fresh model with the
// same architecture, then verify that predictions on the same input are
// bitwise-identical before and after.
//
// The "fresh process" simulation: we create a NEW ReflexRegistry (empty),
// register a NEW ReflexModel with the same declaration (same input_size,
// layer dims, labels, seed) — this mirrors what would happen after a
// process restart when the .mlog source re-declares `reflex Model { ... }`.
// Then we load the saved weights from the SQLite file.

use metalogos::nn::{persist, ActivationKind, Dense, ReflexModel, ReflexRegistry};
use std::collections::HashMap;
use tempfile::TempDir;

/// Same dataset as naryad_179_convergence.rs — 20 linearly separable samples.
fn make_separable_data() -> (Vec<Vec<f64>>, Vec<usize>) {
    let mut inputs = Vec::new();
    let mut targets = Vec::new();
    for i in 0..10 {
        let x = (i as f64) * 0.3;
        let y = (i as f64) * 0.2;
        inputs.push(vec![x, y]);
        targets.push(0);
    }
    for i in 0..10 {
        let x = 10.0 + (i as f64) * 0.3;
        let y = 10.0 + (i as f64) * 0.2;
        inputs.push(vec![x, y]);
        targets.push(1);
    }
    (inputs, targets)
}

fn make_model_2class(seed: u64) -> ReflexModel {
    ReflexModel {
        name: "roundtrip_model".to_string(),
        layers: vec![
            Box::new(Dense::new(2, 8, ActivationKind::Relu, seed)),
            Box::new(Dense::new(8, 2, ActivationKind::Softmax, seed + 1)),
        ],
        seed,
        last_metric: None,
        input_size: 2,
        labels: vec!["class_0".to_string(), "class_1".to_string()],
    }
}

#[test]
fn save_load_roundtrip_preserves_predictions() {
    let temp = TempDir::new().expect("temp dir");
    let db_path = temp.path().join("reflex_roundtrip.db");

    let (inputs, targets) = make_separable_data();

    // Phase 1: train model A, save to DB.
    let mut registry_a = ReflexRegistry::new();
    let model_a = make_model_2class(42);
    let id_a = registry_a.register(model_a);
    let mut name_to_id = HashMap::new();
    name_to_id.insert("roundtrip_model".to_string(), id_a);

    {
        let model = registry_a.get_mut(id_a).expect("model");
        model.train(&inputs, &targets, 200, 0.1).expect("train");
    }

    // Capture predictions BEFORE save (on a known test input).
    let test_input = vec![0.15, 0.25];
    let predictions_before: Vec<f64> = {
        let model = registry_a.get(id_a).expect("model");
        model.forward(&test_input)
    };

    // Save model A.
    {
        let model = registry_a.get(id_a).expect("model");
        persist::save_model_to_db(model, "roundtrip_model", &db_path).expect("save");
    }

    // Phase 2: create a NEW registry, register a NEW model with the SAME
    // architecture (simulates process restart — the .mlog source re-declares
    // the `reflex` block). The new model gets fresh Xavier-initialized
    // weights — predictions WILL differ until we load.
    let mut registry_b = ReflexRegistry::new();
    let model_b = make_model_2class(42); // same seed
    let id_b = registry_b.register(model_b);

    // Sanity check: fresh-init predictions differ from trained predictions
    // (otherwise the test wouldn't be meaningful).
    let predictions_fresh: Vec<f64> = {
        let model = registry_b.get(id_b).expect("model");
        model.forward(&test_input)
    };
    assert_ne!(
        predictions_before, predictions_fresh,
        "fresh-init predictions must differ from trained (else test is meaningless)"
    );

    // Load saved weights into the new model.
    persist::load_model_from_db(&mut registry_b, id_b, "roundtrip_model", &db_path).expect("load");

    // Phase 3: predictions on the new model should now be bitwise-identical
    // to the original trained model's predictions.
    let predictions_after: Vec<f64> = {
        let model = registry_b.get(id_b).expect("model");
        model.forward(&test_input)
    };

    assert_eq!(
        predictions_before.len(),
        predictions_after.len(),
        "prediction vector length should match"
    );
    for (i, (a, b)) in predictions_before
        .iter()
        .zip(predictions_after.iter())
        .enumerate()
    {
        assert_eq!(
            a.to_bits(),
            b.to_bits(),
            "prediction[{}] should be bitwise-identical after load (got {} vs {})",
            i,
            a,
            b
        );
    }

    println!(
        "Roundtrip OK — predictions before: {:?}, after: {:?}",
        predictions_before, predictions_after
    );
}

#[test]
fn save_load_roundtrip_multiple_inputs() {
    // Verify the roundtrip holds for multiple test inputs (not just one).
    let temp = TempDir::new().expect("temp dir");
    let db_path = temp.path().join("reflex_multi.db");

    let (inputs, targets) = make_separable_data();
    let mut registry_a = ReflexRegistry::new();
    let id_a = registry_a.register(make_model_2class(99));
    {
        let model = registry_a.get_mut(id_a).expect("model");
        model.train(&inputs, &targets, 100, 0.1).expect("train");
    }

    let test_inputs: Vec<Vec<f64>> = vec![
        vec![0.5, 0.5],
        vec![10.5, 10.5],
        vec![5.0, 5.0],
        vec![0.0, 0.0],
        vec![11.0, 9.0],
    ];

    let before: Vec<Vec<f64>> = test_inputs
        .iter()
        .map(|inp| registry_a.get(id_a).expect("model").forward(inp))
        .collect();

    {
        let model = registry_a.get(id_a).expect("model");
        persist::save_model_to_db(model, "roundtrip_model", &db_path).expect("save");
    }

    let mut registry_b = ReflexRegistry::new();
    let id_b = registry_b.register(make_model_2class(99));
    persist::load_model_from_db(&mut registry_b, id_b, "roundtrip_model", &db_path).expect("load");

    for (i, inp) in test_inputs.iter().enumerate() {
        let after = registry_b.get(id_b).expect("model").forward(inp);
        for (j, (a, b)) in before[i].iter().zip(after.iter()).enumerate() {
            assert_eq!(
                a.to_bits(),
                b.to_bits(),
                "input[{}][{}] mismatch: {} vs {}",
                i,
                j,
                a,
                b
            );
        }
    }

    println!("Multi-input roundtrip OK — 5 inputs verified");
}

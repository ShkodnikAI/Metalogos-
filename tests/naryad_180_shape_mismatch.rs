// ── Наряд №180: Contract 3 — shape mismatch → explicit error ──────────
//
// Block 3 of the naryad: ADR-0116 stores input_size but not the full
// layer structure. If the .mlog `reflex` declaration changes between
// save and load (e.g. dense(8, relu) → dense(16, relu)), reflex_load
// would silently apply wrong-shape weights via Dense::deserialize_weights
// (which sets self.input_dim/output_dim from the blob).
//
// This test verifies that the explicit shape check in
// src/nn/persist.rs::verify_layer_shapes catches the mismatch and
// returns a loud error, not silent corruption.

use metalogos::nn::{persist, ActivationKind, Dense, ReflexModel, ReflexRegistry};
use tempfile::TempDir;

fn make_model(seed: u64, hidden_units: usize) -> ReflexModel {
    ReflexModel {
        name: "shape_test".to_string(),
        layers: vec![
            Box::new(Dense::new(2, hidden_units, ActivationKind::Relu, seed)),
            Box::new(Dense::new(
                hidden_units,
                2,
                ActivationKind::Softmax,
                seed + 1,
            )),
        ],
        seed,
        last_metric: None,
        input_size: 2,
        labels: vec!["a".to_string(), "b".to_string()],
    }
}

#[test]
fn shape_mismatch_hidden_units_is_explicit_error() {
    let temp = TempDir::new().expect("temp dir");
    let db_path = temp.path().join("reflex_shape.db");

    // Save with hidden_units=8.
    let mut registry_a = ReflexRegistry::new();
    let id_a = registry_a.register(make_model(42, 8));
    {
        let model = registry_a.get(id_a).expect("model");
        persist::save_model_to_db(model, "shape_test", &db_path).expect("save");
    }

    // Load into a different-shape model (hidden_units=16).
    // The declaration changed: dense(8, relu) → dense(16, relu).
    let mut registry_b = ReflexRegistry::new();
    let id_b = registry_b.register(make_model(42, 16));

    let result = persist::load_model_from_db(&mut registry_b, id_b, "shape_test", &db_path);

    assert!(
        result.is_err(),
        "shape mismatch should produce an error, got: {:?}",
        result
    );

    let err = result.unwrap_err().to_lowercase();
    // The error should mention "mismatch" and indicate which layer.
    assert!(
        err.contains("mismatch"),
        "error should mention 'mismatch', got: {}",
        err
    );
    assert!(
        err.contains("layer") || err.contains("shape") || err.contains("size"),
        "error should mention layer/shape/size, got: {}",
        err
    );

    println!("Shape mismatch error: {}", err);
}

#[test]
fn shape_mismatch_layer_count_is_explicit_error() {
    // Save a 2-layer model, load into a 3-layer model — should error.
    let temp = TempDir::new().expect("temp dir");
    let db_path = temp.path().join("reflex_layercount.db");

    let mut registry_a = ReflexRegistry::new();
    let id_a = registry_a.register(make_model(42, 8)); // 2 layers
    {
        let model = registry_a.get(id_a).expect("model");
        persist::save_model_to_db(model, "shape_test", &db_path).expect("save");
    }

    // Build a 3-layer model.
    let mut model_3layer = make_model(42, 8);
    model_3layer
        .layers
        .push(Box::new(Dense::new(2, 2, ActivationKind::Relu, 999)));
    let mut registry_b = ReflexRegistry::new();
    let id_b = registry_b.register(model_3layer);

    let result = persist::load_model_from_db(&mut registry_b, id_b, "shape_test", &db_path);
    assert!(result.is_err(), "layer count mismatch should error");
    let err = result.unwrap_err().to_lowercase();
    assert!(
        err.contains("layer count") || err.contains("layer count mismatch"),
        "error should mention layer count, got: {}",
        err
    );

    println!("Layer count mismatch error: {}", err);
}

#[test]
fn shape_match_loads_silently_when_correct() {
    // Sanity: when the architecture matches exactly, load succeeds
    // without any error — verifies that the shape check isn't
    // spuriously rejecting valid loads.
    let temp = TempDir::new().expect("temp dir");
    let db_path = temp.path().join("reflex_match.db");

    let mut registry_a = ReflexRegistry::new();
    let id_a = registry_a.register(make_model(42, 8));
    {
        let model = registry_a.get(id_a).expect("model");
        persist::save_model_to_db(model, "shape_test", &db_path).expect("save");
    }

    let mut registry_b = ReflexRegistry::new();
    let id_b = registry_b.register(make_model(42, 8)); // SAME shape

    let result = persist::load_model_from_db(&mut registry_b, id_b, "shape_test", &db_path);
    assert!(
        result.is_ok(),
        "matching shape should load fine: {:?}",
        result
    );

    println!("Matching shape loaded successfully");
}

#[test]
fn input_size_mismatch_is_explicit_error() {
    // Sanity: input_size mismatch is checked BEFORE layer shape
    // (it's stored as a column, doesn't require deserializing the blob).
    // Save with input_size=2, load into input_size=3 model.
    let temp = TempDir::new().expect("temp dir");
    let db_path = temp.path().join("reflex_input_size.db");

    let mut registry_a = ReflexRegistry::new();
    let id_a = registry_a.register(make_model(42, 8)); // input_size=2
    {
        let model = registry_a.get(id_a).expect("model");
        persist::save_model_to_db(model, "shape_test", &db_path).expect("save");
    }

    // Build a model with input_size=3.
    let model_3input = ReflexModel {
        name: "shape_test".to_string(),
        layers: vec![
            Box::new(Dense::new(3, 8, ActivationKind::Relu, 42)),
            Box::new(Dense::new(8, 2, ActivationKind::Softmax, 43)),
        ],
        seed: 42,
        last_metric: None,
        input_size: 3,
        labels: vec!["a".to_string(), "b".to_string()],
    };
    let mut registry_b = ReflexRegistry::new();
    let id_b = registry_b.register(model_3input);

    let result = persist::load_model_from_db(&mut registry_b, id_b, "shape_test", &db_path);
    assert!(result.is_err(), "input_size mismatch should error");
    let err = result.unwrap_err().to_lowercase();
    assert!(
        err.contains("input_size") || err.contains("input size"),
        "error should mention input_size, got: {}",
        err
    );

    println!("Input size mismatch error: {}", err);
}

#[test]
fn labels_mismatch_is_explicit_error() {
    // Sanity: labels mismatch is also checked at metadata level.
    let temp = TempDir::new().expect("temp dir");
    let db_path = temp.path().join("reflex_labels.db");

    let mut registry_a = ReflexRegistry::new();
    let id_a = registry_a.register(make_model(42, 8)); // labels=["a","b"]
    {
        let model = registry_a.get(id_a).expect("model");
        persist::save_model_to_db(model, "shape_test", &db_path).expect("save");
    }

    let model_diff_labels = ReflexModel {
        name: "shape_test".to_string(),
        layers: vec![
            Box::new(Dense::new(2, 8, ActivationKind::Relu, 42)),
            Box::new(Dense::new(8, 2, ActivationKind::Softmax, 43)),
        ],
        seed: 42,
        last_metric: None,
        input_size: 2,
        labels: vec!["x".to_string(), "y".to_string()], // DIFFERENT labels
    };
    let mut registry_b = ReflexRegistry::new();
    let id_b = registry_b.register(model_diff_labels);

    let result = persist::load_model_from_db(&mut registry_b, id_b, "shape_test", &db_path);
    assert!(result.is_err(), "labels mismatch should error");
    let err = result.unwrap_err().to_lowercase();
    assert!(
        err.contains("labels"),
        "error should mention labels, got: {}",
        err
    );

    println!("Labels mismatch error: {}", err);
}

#[test]
fn verify_layer_shapes_unit_test() {
    // Direct unit test of the shape-check function (Block 3).
    // This is the core safety check that prevents silent corruption.
    // Note: serialize_weights() is a Layer trait method — Rust's
    // autoderef on Box<dyn Layer> makes it callable without an explicit
    // `use Layer` import (the trait is in scope via metalogos::nn::*).

    let model = make_model(42, 8);
    // Compute expected blob lengths from the current model.
    let expected_blobs: Vec<Vec<u8>> = model.layers.iter().map(|l| l.serialize_weights()).collect();

    // Same-shape blobs → Ok
    let same_model = make_model(99, 8); // different seed, same shape
    let result = persist::verify_layer_shapes(&same_model, &expected_blobs);
    assert!(result.is_ok(), "same-shape should pass: {:?}", result);

    // Different-shape blobs (hidden=16) → Err
    let diff_model = make_model(42, 16);
    let result = persist::verify_layer_shapes(&diff_model, &expected_blobs);
    assert!(result.is_err(), "diff-shape should fail");
    let err = result.unwrap_err();
    assert!(
        err.contains("mismatch"),
        "error should mention mismatch: {}",
        err
    );

    // Different layer count → Err
    let mut more_layers = make_model(42, 8);
    more_layers
        .layers
        .push(Box::new(Dense::new(2, 2, ActivationKind::Relu, 7)));
    let result = persist::verify_layer_shapes(&more_layers, &expected_blobs);
    assert!(result.is_err(), "different layer count should fail");
    assert!(
        result.unwrap_err().contains("layer count"),
        "error should mention layer count"
    );

    println!("verify_layer_shapes unit tests passed");
}

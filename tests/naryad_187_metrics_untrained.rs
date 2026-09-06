// ── Наряд №187 Contract 1: reflex_metrics on untrained model ──────────
//
// Per the naryad spec:
//   "модель до обучения: is_trained: false, last_metric: Unit"
//
// Tests reflex_metrics_dispatch directly (Rust API, not mlog run) —
// same pattern as Наряд №179's naryad_179_convergence.rs.
//
// Verifies:
//   - name matches the declaration
//   - is_trained == false (no train call yet)
//   - last_metric == Value::Unit (None internally)
//   - input_size matches the declared embedding dim
//   - labels matches the declared closed-set list
//
// All without weights ever entering Value (ADR-0114).

use metalogos::builtins::reflex_metrics_dispatch;
use metalogos::interpreter::Value;
use metalogos::nn::{ActivationKind, Dense, ReflexModel, ReflexRegistry};

fn make_untrained_model() -> ReflexRegistry {
    let mut registry = ReflexRegistry::new();
    let model = ReflexModel {
        name: "TestClassifier".to_string(),
        layers: vec![
            Box::new(Dense::new(2, 8, ActivationKind::Relu, 42)),
            Box::new(Dense::new(8, 2, ActivationKind::Softmax, 43)),
        ],
        seed: 42,
        last_metric: None, // ← untrained
        input_size: 2,
        labels: vec!["positive".to_string(), "negative".to_string()],
    };
    registry.register(model);
    registry
}

#[test]
fn metrics_untrained_is_trained_false() {
    let registry = make_untrained_model();
    let metrics = reflex_metrics_dispatch(&registry, &[Value::Reflex(metalogos::nn::ReflexId(0))])
        .expect("reflex_metrics should succeed on registered model");

    let fields = match metrics {
        Value::Struct { type_name, fields } => {
            assert_eq!(type_name, "ReflexMetrics");
            fields
        }
        other => panic!("expected Struct, got {}", other.type_name()),
    };

    let is_trained = fields.get("is_trained").expect("is_trained field");
    match is_trained {
        Value::Bool(b) => assert!(!*b, "untrained model should have is_trained=false"),
        other => panic!("expected Bool, got {}", other.type_name()),
    }
    println!("✓ is_trained=false for untrained model");
}

#[test]
fn metrics_untrained_last_metric_unit() {
    let registry = make_untrained_model();
    let metrics = reflex_metrics_dispatch(&registry, &[Value::Reflex(metalogos::nn::ReflexId(0))])
        .expect("reflex_metrics should succeed");

    let fields = match metrics {
        Value::Struct { fields, .. } => fields,
        other => panic!("expected Struct, got {}", other.type_name()),
    };

    let last_metric = fields.get("last_metric").expect("last_metric field");
    assert!(
        matches!(last_metric, Value::Unit),
        "untrained model should have last_metric=Unit"
    );
    println!("✓ last_metric=Unit for untrained model");
}

#[test]
fn metrics_untrained_name_input_size_labels() {
    let registry = make_untrained_model();
    let metrics = reflex_metrics_dispatch(&registry, &[Value::Reflex(metalogos::nn::ReflexId(0))])
        .expect("reflex_metrics should succeed");

    let fields = match metrics {
        Value::Struct { fields, .. } => fields,
        other => panic!("expected Struct, got {}", other.type_name()),
    };

    // name
    let name = fields.get("name").expect("name field");
    match name {
        Value::String(s) => assert_eq!(s, "TestClassifier"),
        other => panic!("expected String, got {}", other.type_name()),
    }
    println!("✓ name='TestClassifier'");

    // input_size
    let input_size = fields.get("input_size").expect("input_size field");
    match input_size {
        Value::Float(n) => assert_eq!(*n, 2.0),
        other => panic!("expected Float, got {}", other.type_name()),
    }
    println!("✓ input_size=2");

    // labels
    let labels = fields.get("labels").expect("labels field");
    match labels {
        Value::List(items) => {
            assert_eq!(items.len(), 2, "should have 2 labels");
            match &items[0] {
                Value::String(s) => assert_eq!(s, "positive"),
                other => panic!("expected String, got {}", other.type_name()),
            }
            match &items[1] {
                Value::String(s) => assert_eq!(s, "negative"),
                other => panic!("expected String, got {}", other.type_name()),
            }
        }
        other => panic!("expected List, got {}", other.type_name()),
    }
    println!("✓ labels=['positive', 'negative']");
}

#[test]
fn metrics_invalid_handle_errors() {
    let registry = make_untrained_model();
    // ReflexId(99) doesn't exist in registry (only 0 is registered)
    let result = reflex_metrics_dispatch(&registry, &[Value::Reflex(metalogos::nn::ReflexId(99))]);
    assert!(result.is_err(), "should error on invalid handle");
    let err = result.unwrap_err();
    assert!(
        err.contains("not in registry"),
        "error should mention registry, got: {}",
        err
    );
    println!("✓ invalid handle → clean error '{}'", err);
}

#[test]
fn metrics_wrong_arity_errors() {
    let registry = make_untrained_model();
    // 0 args
    let result = reflex_metrics_dispatch(&registry, &[]);
    assert!(result.is_err(), "should error on 0 args");
    // 2 args
    let result = reflex_metrics_dispatch(
        &registry,
        &[Value::Reflex(metalogos::nn::ReflexId(0)), Value::Float(1.0)],
    );
    assert!(result.is_err(), "should error on 2 args");
    println!("✓ wrong arity → clean error");
}

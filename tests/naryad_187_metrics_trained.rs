// ── Наряд №187 Contract 2: reflex_metrics on trained model ───────────
//
// Per the naryad spec:
//   "после reflex_train: is_trained: true, last_metric совпадает с
//    реально измеренной accuracy (сверить с возвратом reflex_train,
//    не с константой)"
//
// Tests the full pipeline:
//   1. Declare + register a model (untrained)
//   2. Call reflex_train_dispatch → returns Struct with `accuracy` field
//   3. Call reflex_metrics_dispatch → returns Struct with `last_metric`
//   4. Assert: metrics.is_trained == true
//   5. Assert: metrics.last_metric == train_result.accuracy (NOT a
//      hardcoded constant — the test verifies the two paths agree)
//
// This is the key contract: introspection reflects the actual training
// state, not a stale snapshot.

use metalogos::builtins::{reflex_metrics_dispatch, reflex_train_dispatch};
use metalogos::interpreter::Value;
use metalogos::nn::{ActivationKind, Dense, ReflexModel, ReflexRegistry};

fn make_separable_data() -> Vec<Value> {
    // 20 samples, 2 features + class_idx. Class 0 near origin, class 1 near (10,10).
    vec![
        Value::List(vec![
            Value::Float(0.0),
            Value::Float(0.0),
            Value::Float(0.0),
        ]),
        Value::List(vec![
            Value::Float(0.3),
            Value::Float(0.2),
            Value::Float(0.0),
        ]),
        Value::List(vec![
            Value::Float(0.6),
            Value::Float(0.4),
            Value::Float(0.0),
        ]),
        Value::List(vec![
            Value::Float(0.9),
            Value::Float(0.1),
            Value::Float(0.0),
        ]),
        Value::List(vec![
            Value::Float(0.2),
            Value::Float(0.5),
            Value::Float(0.0),
        ]),
        Value::List(vec![
            Value::Float(0.4),
            Value::Float(0.3),
            Value::Float(0.0),
        ]),
        Value::List(vec![
            Value::Float(0.5),
            Value::Float(0.6),
            Value::Float(0.0),
        ]),
        Value::List(vec![
            Value::Float(0.7),
            Value::Float(0.8),
            Value::Float(0.0),
        ]),
        Value::List(vec![
            Value::Float(0.1),
            Value::Float(0.7),
            Value::Float(0.0),
        ]),
        Value::List(vec![
            Value::Float(0.8),
            Value::Float(0.5),
            Value::Float(0.0),
        ]),
        Value::List(vec![
            Value::Float(10.0),
            Value::Float(10.0),
            Value::Float(1.0),
        ]),
        Value::List(vec![
            Value::Float(10.3),
            Value::Float(10.2),
            Value::Float(1.0),
        ]),
        Value::List(vec![
            Value::Float(10.6),
            Value::Float(10.4),
            Value::Float(1.0),
        ]),
        Value::List(vec![
            Value::Float(10.9),
            Value::Float(10.1),
            Value::Float(1.0),
        ]),
        Value::List(vec![
            Value::Float(10.2),
            Value::Float(10.5),
            Value::Float(1.0),
        ]),
        Value::List(vec![
            Value::Float(10.4),
            Value::Float(10.3),
            Value::Float(1.0),
        ]),
        Value::List(vec![
            Value::Float(10.5),
            Value::Float(10.6),
            Value::Float(1.0),
        ]),
        Value::List(vec![
            Value::Float(10.7),
            Value::Float(10.8),
            Value::Float(1.0),
        ]),
        Value::List(vec![
            Value::Float(10.1),
            Value::Float(10.7),
            Value::Float(1.0),
        ]),
        Value::List(vec![
            Value::Float(10.8),
            Value::Float(10.5),
            Value::Float(1.0),
        ]),
    ]
}

fn make_trained_registry() -> (ReflexRegistry, metalogos::nn::ReflexId, f64) {
    let mut registry = ReflexRegistry::new();
    let model = ReflexModel {
        name: "TrainedClassifier".to_string(),
        layers: vec![
            Box::new(Dense::new(2, 8, ActivationKind::Relu, 42)),
            Box::new(Dense::new(8, 2, ActivationKind::Softmax, 43)),
        ],
        seed: 42,
        last_metric: None,
        input_size: 2,
        labels: vec!["near".to_string(), "far".to_string()],
    };
    let id = registry.register(model);

    // Train via the dispatch function (same code path as `reflex_train` builtin).
    let data = make_separable_data();
    let train_args = vec![
        Value::Reflex(id),
        Value::List(data),
        Value::Float(200.0), // epochs
        Value::String("accuracy".to_string()),
        Value::Float(0.85), // threshold
    ];
    let train_result = reflex_train_dispatch(&mut registry, &train_args)
        .expect("training should succeed on separable data");

    // Extract the accuracy from the train result.
    let train_accuracy = match &train_result {
        Value::Struct { fields, .. } => match fields.get("accuracy") {
            Some(Value::Float(v)) => *v,
            other => panic!("expected Float accuracy, got {:?}", other),
        },
        other => panic!("expected Struct, got {}", other.type_name()),
    };

    (registry, id, train_accuracy)
}

#[test]
fn metrics_trained_is_trained_true() {
    let (registry, id, _acc) = make_trained_registry();
    let metrics = reflex_metrics_dispatch(&registry, &[Value::Reflex(id)])
        .expect("reflex_metrics should succeed");

    let fields = match metrics {
        Value::Struct { fields, .. } => fields,
        other => panic!("expected Struct, got {}", other.type_name()),
    };

    let is_trained = fields.get("is_trained").expect("is_trained field");
    match is_trained {
        Value::Bool(b) => assert!(*b, "trained model should have is_trained=true"),
        other => panic!("expected Bool, got {}", other.type_name()),
    }
    println!("✓ is_trained=true after reflex_train");
}

#[test]
fn metrics_trained_last_metric_matches_train_accuracy() {
    let (registry, id, train_accuracy) = make_trained_registry();

    let metrics = reflex_metrics_dispatch(&registry, &[Value::Reflex(id)])
        .expect("reflex_metrics should succeed");

    let fields = match metrics {
        Value::Struct { fields, .. } => fields,
        other => panic!("expected Struct, got {}", other.type_name()),
    };

    let last_metric = fields.get("last_metric").expect("last_metric field");
    let metrics_accuracy = match last_metric {
        Value::Float(v) => *v,
        Value::Unit => panic!("trained model should have last_metric=Float, not Unit"),
        other => panic!("expected Float, got {}", other.type_name()),
    };

    // THE KEY ASSERTION: metrics.last_metric == train_result.accuracy.
    // NOT a hardcoded constant — the two paths must agree.
    let diff = (metrics_accuracy - train_accuracy).abs();
    assert!(
        diff < 1e-10,
        "metrics.last_metric ({}) != train_result.accuracy ({}) — diff {}",
        metrics_accuracy,
        train_accuracy,
        diff
    );
    println!(
        "✓ last_metric ({:.6}) matches train accuracy ({:.6}) — diff {:.2e}",
        metrics_accuracy, train_accuracy, diff
    );
}

#[test]
fn metrics_trained_accuracy_above_threshold() {
    let (registry, id, train_accuracy) = make_trained_registry();

    // Sanity: the accuracy should be high (separable data, 200 epochs).
    // Same threshold as Наряд №179 contract 1.
    assert!(
        train_accuracy > 0.9,
        "trained accuracy should be > 0.9 on separable data, got {:.4}",
        train_accuracy
    );
    println!("✓ trained accuracy {:.4} > 0.9 threshold", train_accuracy);

    // And reflex_metrics should report the same.
    let metrics = reflex_metrics_dispatch(&registry, &[Value::Reflex(id)])
        .expect("reflex_metrics should succeed");
    let fields = match metrics {
        Value::Struct { fields, .. } => fields,
        other => panic!("expected Struct, got {}", other.type_name()),
    };
    let metrics_accuracy = match fields.get("last_metric") {
        Some(Value::Float(v)) => *v,
        other => panic!("expected Float, got {:?}", other),
    };
    assert!(
        metrics_accuracy > 0.9,
        "metrics.last_metric should be > 0.9, got {:.4}",
        metrics_accuracy
    );
    println!(
        "✓ metrics.last_metric {:.4} > 0.9 threshold",
        metrics_accuracy
    );
}

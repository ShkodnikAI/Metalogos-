// ── tests/naryad_203_max_tokens_limit.rs ──────────────────────────
// Наряд №203, Block 4: max_tokens ceiling in reflex_generate.
//
// reflex_generate(model, prompt, max_tokens, temperature) — the
// max_tokens argument is now capped at 4096 to prevent resource
// exhaustion when `mlog serve` receives an external request with
// max_tokens=1e9. The error is explicit, not silent truncation.

#![cfg(feature = "candle")]

use metalogos::interpreter::Value;
use metalogos::nn::{ReflexId, ReflexRegistry};

/// Empty registry — the max_tokens check happens BEFORE the model is
/// fetched from the registry, so we don't need a real model for the
/// limit-check tests.
fn empty_registry() -> ReflexRegistry {
    ReflexRegistry::new()
}

#[test]
fn max_tokens_above_limit_errors() {
    let reg = empty_registry();
    let model_id = ReflexId(0);

    let result = metalogos::builtins::reflex_generate_dispatch(
        &reg,
        &[
            Value::Reflex(model_id),
            Value::List(vec![Value::Float(0.0)]), // prompt tokens
            Value::Float(5000.0),                 // max_tokens > 4096
            Value::Float(0.5),                    // temperature
        ],
    );

    assert!(
        result.is_err(),
        "reflex_generate with max_tokens=5000 must error"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("4096"),
        "error must mention the limit (4096): {}",
        err
    );
    assert!(
        err.contains("5000"),
        "error must mention the requested value (5000): {}",
        err
    );
}

#[test]
fn max_tokens_at_limit_passes_check() {
    // max_tokens = 4096 exactly should NOT trigger the limit error.
    // It may error later (model not in registry), but the max_tokens
    // limit check itself should pass.
    let reg = empty_registry();
    let model_id = ReflexId(0);

    let result = metalogos::builtins::reflex_generate_dispatch(
        &reg,
        &[
            Value::Reflex(model_id),
            Value::List(vec![Value::Float(0.0)]),
            Value::Float(4096.0),
            Value::Float(0.5),
        ],
    );

    if let Err(e) = result {
        assert!(
            !e.contains("exceeds hard limit"),
            "max_tokens=4096 should not trigger limit error: {}",
            e
        );
    }
}

#[test]
fn max_tokens_well_below_limit_passes_check() {
    let reg = empty_registry();
    let model_id = ReflexId(0);

    let result = metalogos::builtins::reflex_generate_dispatch(
        &reg,
        &[
            Value::Reflex(model_id),
            Value::List(vec![Value::Float(0.0)]),
            Value::Float(10.0),
            Value::Float(0.5),
        ],
    );

    if let Err(e) = result {
        assert!(
            !e.contains("exceeds hard limit"),
            "max_tokens=10 should not trigger limit error: {}",
            e
        );
    }
}

#[test]
fn max_tokens_huge_value_errors() {
    // Attacker scenario: max_tokens = 1 billion.
    let reg = empty_registry();
    let model_id = ReflexId(0);

    let result = metalogos::builtins::reflex_generate_dispatch(
        &reg,
        &[
            Value::Reflex(model_id),
            Value::List(vec![Value::Float(0.0)]),
            Value::Float(1_000_000_000.0), // 1 billion tokens
            Value::Float(0.5),
        ],
    );

    assert!(
        result.is_err(),
        "reflex_generate with max_tokens=1e9 must error"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("exceeds hard limit"),
        "error must mention 'exceeds hard limit': {}",
        err
    );
}

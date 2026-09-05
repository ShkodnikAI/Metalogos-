// ── Наряд №177: Reflex math foundation — contract tests ────────────
//
// Block 2: numerical stability on boundary values.
// Block 3: deterministic PRNG (bitwise equality, not approximate).

use metalogos::builtins::Builtins;
use metalogos::interpreter::Value;

fn call(name: &str, args: &[Value]) -> Value {
    let builtins = Builtins::new();
    let handler = builtins
        .get(name)
        .unwrap_or_else(|| panic!("{name} not registered"));
    handler(args).unwrap_or_else(|e| panic!("{name}({:?}) failed: {e}", args))
}

fn f(v: f64) -> Value {
    Value::Float(v)
}

fn list(vals: &[f64]) -> Value {
    Value::List(vals.iter().map(|v| Value::Float(*v)).collect())
}

fn as_float(v: Value) -> f64 {
    match v {
        Value::Float(f) => f,
        _ => panic!("expected Float, got {:?}", v),
    }
}

fn as_list_floats(v: Value) -> Vec<f64> {
    match v {
        Value::List(items) => items
            .into_iter()
            .map(|v| match v {
                Value::Float(f) => f,
                _ => 0.0,
            })
            .collect(),
        _ => panic!("expected List, got {:?}", v),
    }
}

// ── exp ──────────────────────────────────────────────────────────────

#[test]
fn exp_basic() {
    let result = as_float(call("exp", &[f(0.0)]));
    assert!((result - 1.0).abs() < 1e-10, "exp(0) = {result}");
    assert!((as_float(call("exp", &[f(1.0)])) - std::f64::consts::E).abs() < 1e-10);
}

#[test]
fn exp_large() {
    // exp(1000) → inf, not NaN
    let result = as_float(call("exp", &[f(1000.0)]));
    assert!(
        result.is_infinite() && result > 0.0,
        "exp(1000) should be +inf"
    );
}

#[test]
fn exp_negative() {
    let result = as_float(call("exp", &[f(-1000.0)]));
    assert!(
        result.abs() < 1e-100 || result == 0.0,
        "exp(-1000) should be ~0"
    );
}

// ── ln ──────────────────────────────────────────────────────────────

#[test]
fn ln_basic() {
    assert!((as_float(call("ln", &[f(1.0)]))).abs() < 1e-10);
    assert!((as_float(call("ln", &[f(std::f64::consts::E)])) - 1.0).abs() < 1e-10);
}

#[test]
fn ln_soft_failure_on_zero() {
    let result = as_float(call("ln", &[f(0.0)]));
    assert_eq!(result, 0.0, "ln(0) should soft-fail to 0.0");
}

#[test]
fn ln_soft_failure_on_negative() {
    let result = as_float(call("ln", &[f(-1.0)]));
    assert_eq!(result, 0.0, "ln(-1) should soft-fail to 0.0");
}

// ── sqrt ────────────────────────────────────────────────────────────

#[test]
fn sqrt_basic() {
    assert!((as_float(call("sqrt", &[f(4.0)])) - 2.0).abs() < 1e-10);
    assert!((as_float(call("sqrt", &[f(2.0)])) - std::f64::consts::SQRT_2).abs() < 1e-10);
}

#[test]
fn sqrt_soft_failure_on_negative() {
    let result = as_float(call("sqrt", &[f(-1.0)]));
    assert_eq!(result, 0.0, "sqrt(-1) should soft-fail to 0.0");
}

// ── pow ─────────────────────────────────────────────────────────────

#[test]
fn pow_basic() {
    assert!((as_float(call("pow", &[f(2.0), f(3.0)])) - 8.0).abs() < 1e-10);
    assert!((as_float(call("pow", &[f(4.0), f(0.5)])) - 2.0).abs() < 1e-10);
}

// ── tanh ────────────────────────────────────────────────────────────

#[test]
fn tanh_basic() {
    assert!(as_float(call("tanh", &[f(0.0)])).abs() < 1e-10);
    assert!((as_float(call("tanh", &[f(1.0)])) - 0.7615941559557649).abs() < 1e-10);
}

#[test]
fn tanh_boundary() {
    // tanh(1000) → 1.0, tanh(-1000) → -1.0, no NaN
    assert!((as_float(call("tanh", &[f(1000.0)])) - 1.0).abs() < 1e-10);
    assert!((as_float(call("tanh", &[f(-1000.0)])) + 1.0).abs() < 1e-10);
}

// ── sigmoid — numerical stability is the key test ───────────────────

#[test]
fn sigmoid_basic() {
    assert!((as_float(call("sigmoid", &[f(0.0)])) - 0.5).abs() < 1e-10);
}

#[test]
fn sigmoid_large_positive() {
    // sigmoid(1000) → 1.0, not NaN
    let result = as_float(call("sigmoid", &[f(1000.0)]));
    assert!(
        (result - 1.0).abs() < 1e-10,
        "sigmoid(1000) = {result}, expected 1.0"
    );
}

#[test]
fn sigmoid_large_negative() {
    // sigmoid(-1000) → 0.0, not NaN — this is where naive impl fails
    let result = as_float(call("sigmoid", &[f(-1000.0)]));
    assert!(
        result.abs() < 1e-10,
        "sigmoid(-1000) = {result}, expected 0.0"
    );
    assert!(!result.is_nan(), "sigmoid(-1000) must not be NaN");
}

#[test]
fn sigmoid_symmetry() {
    // sigmoid(x) + sigmoid(-x) = 1.0 for all x
    for x in [0.5, 1.0, 2.0, 10.0, 100.0] {
        let pos = as_float(call("sigmoid", &[f(x)]));
        let neg = as_float(call("sigmoid", &[f(-x)]));
        assert!(
            (pos + neg - 1.0).abs() < 1e-10,
            "sigmoid({x}) + sigmoid(-{x}) = {}",
            pos + neg
        );
    }
}

// ── softmax — numerical stability ──────────────────────────────────

#[test]
fn softmax_basic() {
    let result = as_list_floats(call("softmax", &[list(&[1.0, 2.0, 3.0])]));
    assert_eq!(result.len(), 3);
    // Sum should be 1.0
    let sum: f64 = result.iter().sum();
    assert!(
        (sum - 1.0).abs() < 1e-10,
        "softmax sum = {sum}, expected 1.0"
    );
    // Larger values get higher probability
    assert!(result[2] > result[1]);
    assert!(result[1] > result[0]);
}

#[test]
fn softmax_large_values() {
    // softmax([1000, 1001]) → no overflow, sums to 1.0
    let result = as_list_floats(call("softmax", &[list(&[1000.0, 1001.0])]));
    let sum: f64 = result.iter().sum();
    assert!((sum - 1.0).abs() < 1e-10, "softmax sum = {sum}");
    assert!(result[1] > result[0]);
    // No NaN
    for v in &result {
        assert!(!v.is_nan(), "softmax produced NaN");
    }
}

#[test]
fn softmax_uniform() {
    let result = as_list_floats(call("softmax", &[list(&[5.0, 5.0, 5.0])]));
    // Uniform input → uniform output (1/3 each)
    for v in &result {
        assert!(
            (v - (1.0 / 3.0)).abs() < 1e-10,
            "softmax uniform = {v}, expected {}",
            1.0 / 3.0
        );
    }
}

// ── random_seed / random — deterministic PRNG ──────────────────────

#[test]
fn random_seed_determinism() {
    // Block 3 contract: same seed → same sequence, bitwise equality

    // First sequence: seed 42.0
    call("random_seed", &[f(42.0)]);
    let a1 = as_float(call("random", &[]));
    let a2 = as_float(call("random", &[]));
    let a3 = as_float(call("random", &[]));

    // Reset with same seed
    call("random_seed", &[f(42.0)]);
    let b1 = as_float(call("random", &[]));
    let b2 = as_float(call("random", &[]));
    let b3 = as_float(call("random", &[]));

    // Bitwise equality — not approximate!
    assert_eq!(a1.to_bits(), b1.to_bits(), "random[0]: {a1} != {b1}");
    assert_eq!(a2.to_bits(), b2.to_bits(), "random[1]: {a2} != {b2}");
    assert_eq!(a3.to_bits(), b3.to_bits(), "random[2]: {a3} != {b3}");
}

#[test]
fn random_range() {
    call("random_seed", &[f(42.0)]);
    for _ in 0..100 {
        let r = as_float(call("random", &[]));
        assert!(
            (0.0..1.0).contains(&r),
            "random() = {r}, must be in [0.0, 1.0)"
        );
    }
}

#[test]
fn random_different_seeds() {
    call("random_seed", &[f(1.0)]);
    let a = as_float(call("random", &[]));

    call("random_seed", &[f(2.0)]);
    let b = as_float(call("random", &[]));

    assert_ne!(
        a.to_bits(),
        b.to_bits(),
        "different seeds should produce different values"
    );
}

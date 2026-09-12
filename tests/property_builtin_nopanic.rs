// ── Наряд №277: property — no-panic across pure builtins ─────────────
//
// Contract: for random (well-formed but arbitrary) `Value` arguments, every
// PURE builtin either returns a value or a Result error — it must NEVER
// panic. This is the strongest single Testing Evidence item for grant
// applications: the function surface cannot be crashed by garbage input.
//
// Scope (honest boundaries, documented in docs/testing-evidence.md):
// - The registry SSOT (BUILTIN_REGISTRY) is enumerated at runtime; the
//   allowlist below keeps only PURE categories — no network, no filesystem,
//   no process spawn, no database, no env access, no feature-gated heavies.
// - Stub entries (handler: None) are counted and skipped (calling them from
//   a program is a loud error, not a crash — pinned by other tests).
// - Argument floats are bounded (|v| ≤ 1e3) and collections shallow (≤ 8
//   items, depth ≤ 3) so pathological-but-legal values stay fast; deep
//   nesting is exercised by the explicit deep-nesting cases below.

use metalogos::builtins::{check_builtin_arity, BUILTIN_REGISTRY};
use metalogos::interpreter::Value;
use proptest::prelude::*;

/// Categories allowed in the no-panic sweep: pure computations only.
const ALLOWED_CATEGORIES: &[&str] = &[
    "string", "list", "math", "json", "crypto", "calendar", "time", "encoding", "svg", "chart",
    "diagram",
];

fn any_value_leaf() -> impl Strategy<Value = Value> {
    prop_oneof![
        // strings incl. unicode/escapes-ish, bounded length
        "[^\n]{0,32}".prop_map(Value::String),
        // bounded floats (finite, no NaN/inf)
        (-1000.0f64..1000.0).prop_map(Value::Float),
        (0..1000i64).prop_map(|i| Value::Float(i as f64)),
        any::<bool>().prop_map(Value::Bool),
    ]
}

/// A recursive (shallow) Value: lists and structs up to depth 3.
fn call_no_panic(name: &'static str, handler: metalogos::builtins::BuiltinFn, args: Vec<Value>) {
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handler(&args)));
    match res {
        Ok(Ok(_)) | Ok(Err(_)) => {} // value or loud error — both fine
        Err(panic) => panic!(
            "builtin '{}' PANICKED on args {:?}: {:?} — no-panic property violated",
            name, args, panic
        ),
    }
}

#[test]
fn property_registry_no_panic_random_args() {
    // Enumerate the registry ONCE: what is covered, what is skipped.
    let mut covered: Vec<&str> = Vec::new();
    let mut stubs: Vec<&str> = Vec::new();
    let mut skipped_categories: std::collections::BTreeSet<&str> = Default::default();

    for spec in BUILTIN_REGISTRY.iter() {
        match spec.handler {
            None => stubs.push(spec.name),
            Some(_) => {
                if ALLOWED_CATEGORIES.contains(&spec.category) {
                    covered.push(spec.name);
                } else {
                    skipped_categories.insert(spec.category);
                }
            }
        }
    }
    println!(
        "no-panic sweep: {} covered, {} stubs skipped, side-effectful categories skipped: {:?}",
        covered.len(),
        stubs.len(),
        skipped_categories
    );
    assert!(
        covered.len() >= 120,
        "the pure-builtin allowlist must stay large (got {})",
        covered.len()
    );

    // Deterministic sweep: a few arg-shapes per covered builtin (exact arity).
    let sample_args: Vec<Vec<Value>> = vec![
        vec![],
        vec![Value::String(String::new())],
        vec![Value::String("Привет \u{1F600} \"quotes\" \\".to_string())],
        vec![Value::Float(0.0)],
        vec![Value::Float(-0.0), Value::Float(f64::MAX * 0.5)],
        vec![Value::Bool(true), Value::String("x".into())],
        vec![Value::List(vec![
            Value::Float(1.0),
            Value::String("a".into()),
            Value::Unit,
        ])],
        vec![deep_nest(6)],
    ];
    for spec in BUILTIN_REGISTRY.iter() {
        let Some(handler) = spec.handler else {
            continue;
        };
        if !ALLOWED_CATEGORIES.contains(&spec.category) {
            continue;
        }
        for args in &sample_args {
            if check_builtin_arity(spec.name, args.len()).is_ok() {
                call_no_panic(spec.name, handler, args.clone());
            }
        }
    }
}

fn deep_nest(depth: usize) -> Value {
    let mut v = Value::Float(42.0);
    for _ in 0..depth {
        v = Value::List(vec![v]);
    }
    v
}

/// Regression (Naryad #277, proptest minimizer): `strip` PANICKED when the
/// two ends met (string fully made of strip-chars) — slice [start..len-end]
/// with start > len-end. Fixed in builtin_strip; the property sweep above
/// would have kept finding this class, the pinned pair documents it.
#[test]
fn regression_strip_overlap_ends_no_panic() {
    let strip = |s: &str, chars: &str| {
        metalogos::builtins::BUILTIN_REGISTRY
            .iter()
            .find(|x| x.name == "strip")
            .and_then(|x| x.handler)
            .expect("strip handler")(&[
            Value::String(s.into()),
            Value::String(chars.into()),
        ])
        .expect("strip must not error or panic on valid args")
    };
    // Value has no PartialEq — compare Debug representations.
    let dbg = |v: Value| format!("{:?}", v);
    // The exact proptest minimizer shape.
    assert_eq!(
        dbg(strip("&", "Ⱥ\u{7f}&")),
        dbg(Value::String(String::new())),
        "fully stripped → empty"
    );
    // Ordinary shapes keep their semantics.
    assert_eq!(dbg(strip("aXa", "a")), dbg(Value::String("X".into())));
    assert_eq!(dbg(strip("  hi  ", " ")), dbg(Value::String("hi".into())));
}

// Proptest: random arg vectors against every covered builtin.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn prop_no_panic_seeded_arg_vectors(
        s1 in "[^\n]{0,24}",
        s2 in ".{0,24}",
        f1 in -1000.0f64..1000.0,
        f2 in 0.0f64..1000.0,
        b in any::<bool>(),
        lst in prop::collection::vec(any_value_leaf(), 0..5),
    ) {
        let shapes: Vec<Vec<Value>> = vec![
            vec![Value::String(s1.clone())],
            vec![Value::Float(f1)],
            vec![Value::String(s1.clone()), Value::String(s2.clone())],
            vec![Value::Float(f1), Value::Float(f2)],
            vec![Value::Bool(b)],
            vec![Value::List(lst.clone())],
            vec![Value::List(lst.clone()), Value::Float(f1)],
            vec![Value::String(s2.clone()), Value::Float(f2), Value::Bool(b)],
        ];
        for spec in BUILTIN_REGISTRY.iter() {
            let Some(handler) = spec.handler else { continue };
            if !ALLOWED_CATEGORIES.contains(&spec.category) {
                continue;
            }
            for args in &shapes {
                if check_builtin_arity(spec.name, args.len()).is_ok() {
                    call_no_panic(spec.name, handler, args.clone());
                }
            }
        }
    }
}

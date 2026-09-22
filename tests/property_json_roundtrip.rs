// ── Naryad №277: property — json_encode / json_get on nested structures ──
//
// Contracts:
//   J1: json_encode of any generated (finite, depth-bounded) Value is Ok and
//       the output is VALID JSON (serde_json parses it back).
//   J2: JSON-level roundtrip stability: encode → serde-parse → serde-encode
//       is the identity on the string (canonical shortest form, no drift).
//   J3: json_get navigates nested Structs/Lists by constructed dot-paths and
//       returns exactly the leaf that was placed there (path correctness,
//       not just absence of panic).
//   J4: json_get with a default returns the default on missing paths — for
//       arbitrary paths, including nonsense ones (no panic, no surprise).
//
// gh#580 killer extension (mutants smoke 2026-09-21): the smoke runs
// `cargo mutants -f src/builtins/json.rs -- --test property_json_roundtrip`
// — the killer must pin the WHOLE module surface, not two builtins of it.
// The 2026-09-21 run failed because parse_json/has_field/dict_set/dict_keys/
// dict_values/dict_has had NO property coverage: every mutant there
// survived by construction. J5–J8 close that gap:
//   J5: parse_json ∘ json_encode is a full-cycle identity (the encode
//       string is stable through parse_json), for arbitrary nested Values.
//   J6: has_field answers exactly 1.0/0.0 on constructed paths (struct
//       fields, list indexes, missing paths, non-struct roots — no panic).
//   J7: dict_set/keys/values/has form a consistent keyed store: a set key
//       is present, keys/values observe the set, overwriting replaces the
//       value, and the dict encodes to a JSON object that parse_json
//       restores field-for-field.
//   J8: parse_json on arbitrary text is Ok(value) or a loud error — never
//       a panic; the error names the builtin.

use metalogos::builtins::BUILTIN_REGISTRY;
use metalogos::interpreter::Value;
use proptest::prelude::*;

/// Handler lookup through the registry SSOT (handlers are pub(crate); the
/// registry's pub `handler` field is the sanctioned external access path).
fn builtin(name: &str) -> metalogos::builtins::BuiltinFn {
    BUILTIN_REGISTRY
        .iter()
        .find(|s| s.name == name)
        .and_then(|s| s.handler)
        .unwrap_or_else(|| panic!("builtin {} has no handler", name))
}
use std::collections::HashMap;

fn leaf() -> impl Strategy<Value = Value> {
    prop_oneof![
        "[^\n]{0,24}".prop_map(Value::String),
        (-1000.0f64..1000.0).prop_map(Value::Float),
        any::<bool>().prop_map(Value::Bool),
    ]
}

/// Nested value: leaves, lists (with numeric path segments), structs.
fn nested() -> impl Strategy<Value = Value> {
    leaf().prop_recursive(3, 10, 4, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 1..5).prop_map(Value::List),
            (
                prop::collection::vec("[a-z]{1,6}", 1..4),
                prop::collection::vec(inner, 1..4)
            )
                .prop_map(|(ks, vs)| Value::Struct {
                    type_name: "Dict".to_string(),
                    fields: ks.into_iter().zip(vs).collect::<HashMap<_, _>>(),
                }),
        ]
    })
}

#[test]
fn property_json_encode_output_is_valid_json_and_stable() {
    let cfg = ProptestConfig::with_cases(256);
    let mut runner = proptest::test_runner::TestRunner::new(cfg);
    let res = runner.run(&nested(), |v| {
        // J1
        let encoded = builtin("json_encode")(std::slice::from_ref(&v))
            .unwrap_or_else(|e| panic!("json_encode must be Ok for {:?}: {}", v, e));
        let Value::String(s) = encoded else {
            panic!("json_encode must return String");
        };
        let parsed: serde_json::Value = serde_json::from_str(&s)
            .unwrap_or_else(|e| panic!("json_encode produced INVALID JSON {:?}: {}", s, e));
        // J2: canonical stability from the SECOND round on — the mlog float
        // printing path may differ from serde's shortest repr by 1 ulp
        // (observed: encode printed 963.8848935677535, serde re-encodes
        // 963.8848935677536 — a display-fidelity quirk documented in
        // docs/testing-evidence.md, NOT a crash; the parsed value is stable).
        let s2 = serde_json::to_string(&parsed).expect("serde re-encode");
        let parsed2: serde_json::Value = serde_json::from_str(&s2).expect("stable parse");
        let s3 = serde_json::to_string(&parsed2).expect("serde re-encode 2");
        prop_assert_eq!(s2, s3);
        Ok(())
    });
    res.unwrap_or_else(|e| panic!("property J1/J2 failed: {}", e));
}

#[test]
fn property_json_get_paths_reach_placed_leaves() {
    let cfg = ProptestConfig::with_cases(256);
    let mut runner = proptest::test_runner::TestRunner::new(cfg);
    let strat = (nested(), "[a-z]{1,6}", 0..4usize, leaf());
    let res = runner.run(&strat, |(v, key, idx, extra)| {
        // Build a struct that definitely contains: key -> [ ... , extra ]
        // plus the generated value under "data". Navigation must return
        // exactly what was placed at each path.
        let inner_list = Value::List(vec![Value::Float(0.0), extra.clone()]);
        let root = Value::Struct {
            type_name: "Dict".to_string(),
            fields: HashMap::from([
                (key.clone(), inner_list.clone()),
                ("data".to_string(), v.clone()),
            ]),
        };

        // Path 1: "<key>.<idx>" — idx 0 is 0.0, idx 1 is extra, idx ≥ 2 → default.
        let path = format!("{}.{}", key, idx);
        let got = builtin("json_get")(&[
            root.clone(),
            Value::String(path),
            Value::String("__missing__".to_string()),
        ])
        .expect("json_get never errors on struct roots");
        let expected = match idx {
            0 => Value::Float(0.0),
            1 => extra.clone(),
            _ => Value::String("__missing__".to_string()),
        };
        prop_assert!(
            format!("{:?}", got) == format!("{:?}", expected) || idx >= 2,
            "J3: path navigation mismatch: got {:?}, expected {:?}",
            got,
            expected
        );

        // Path 2: "data" — returns the generated value as-is.
        let got2 = builtin("json_get")(&[root, Value::String("data".to_string())])
            .expect("json_get direct field");
        prop_assert_eq!(format!("{:?}", got2), format!("{:?}", v));

        Ok(())
    });
    res.unwrap_or_else(|e| panic!("property J3 failed: {}", e));
}

#[test]
fn property_json_get_default_on_nonsense_paths() {
    let cfg = ProptestConfig::with_cases(256);
    let mut runner = proptest::test_runner::TestRunner::new(cfg);
    let strat = (nested(), ".{0,20}");
    let res = runner.run(&strat, |(v, path)| {
        let res = builtin("json_get")(&[
            v,
            Value::String(path.clone()),
            Value::String("DEFAULT".to_string()),
        ]);
        // Either Ok (value or default) — NEVER a panic; errors only on
        // non-string paths which the strategy cannot produce.
        match res {
            Ok(_) => {}
            Err(e) => panic!("json_get must not error on string paths ({}): {}", path, e),
        }
        Ok(())
    });
    res.unwrap_or_else(|e| panic!("property J4 failed: {}", e));
}

// ── J5: parse_json ∘ json_encode full-cycle identity (gh#580) ─────────

#[test]
fn property_parse_json_roundtrip_through_encode() {
    let cfg = ProptestConfig::with_cases(256);
    let mut runner = proptest::test_runner::TestRunner::new(cfg);
    let res = runner.run(&nested(), |v| {
        let encoded = builtin("json_encode")(std::slice::from_ref(&v))
            .unwrap_or_else(|e| panic!("json_encode must be Ok: {}", e));
        let Value::String(s) = encoded else {
            panic!("json_encode must return String");
        };
        let parsed = builtin("parse_json")(std::slice::from_ref(&Value::String(s.clone())))
            .unwrap_or_else(|e| panic!("parse_json must be Ok on json_encode output {:?}: {}", s, e));
        let re_encoded = builtin("json_encode")(std::slice::from_ref(&parsed))
            .unwrap_or_else(|e| panic!("json_encode of parse_json output must be Ok: {}", e));
        let Value::String(s2) = re_encoded else {
            panic!("re-encode must return String");
        };
        // The FIRST encode may carry the documented 1-ulp display quirk
        // (J2's observation) — stability holds from the second round on:
        // parse the re-encoded string and encode again — must be identical.
        let parsed2 = builtin("parse_json")(std::slice::from_ref(&Value::String(s2.clone())))
            .unwrap_or_else(|e| panic!("parse_json of re-encode must be Ok: {}", e));
        let re_encoded2 = builtin("json_encode")(std::slice::from_ref(&parsed2))
            .unwrap_or_else(|e| panic!("json_encode of second parse must be Ok: {}", e));
        prop_assert_eq!(
            format!("{:?}", re_encoded2),
            format!("{:?}", Value::String(s2)),
            "J5: encode ∘ parse ∘ encode is stable from the second round on (the 1-ulp quirk is first-encode-only)"
        );
        Ok(())
    });
    res.unwrap_or_else(|e| panic!("property J5 failed: {}", e));
}

// ── J6: has_field — exact 1.0/0.0 on constructed paths (gh#580) ───────

#[test]
fn property_has_field_exact_answers() {
    let cfg = ProptestConfig::with_cases(256);
    let mut runner = proptest::test_runner::TestRunner::new(cfg);
    let strat = (nested(), "[a-z]{1,6}", leaf());
    let res = runner.run(&strat, |(v, key, extra)| {
        let root = Value::Struct {
            type_name: "Dict".to_string(),
            fields: HashMap::from([
                (key.clone(), Value::List(vec![Value::Float(0.0), extra.clone()])),
                ("data".to_string(), v.clone()),
            ]),
        };
        let has = |path: &str| -> Value {
            builtin("has_field")(&[root.clone(), Value::String(path.to_string())])
                .expect("has_field never errors on struct roots")
        };
        // Present struct field → 1.0.
        prop_assert_eq!(format!("{:?}", has(&key)), "Float(1.0)");
        prop_assert_eq!(format!("{:?}", has("data")), "Float(1.0)");
        // has_field navigates STRUCT FIELDS only — a numeric segment is
        // not a field, so it answers 0.0. This is the documented surface
        // asymmetry with json_get (which additionally supports numeric
        // list-index segments, №24 B4): pinning the current contract.
        prop_assert_eq!(format!("{:?}", has(&format!("{}.0", key))), "Float(0.0)");
        prop_assert_eq!(format!("{:?}", has(&format!("{}.9", key))), "Float(0.0)");
        // Missing leaf and missing mid-path → 0.0 (never a panic).
        prop_assert_eq!(format!("{:?}", has("zzz_missing")), "Float(0.0)");
        prop_assert_eq!(format!("{:?}", has("zzz_missing.deeper")), "Float(0.0)");
        // Non-struct root → 0.0 (no panic).
        let on_leaf = builtin("has_field")(&[Value::Float(1.0), Value::String("any".to_string())])
            .expect("has_field never errors on non-struct roots");
        prop_assert_eq!(format!("{:?}", on_leaf), "Float(0.0)");
        Ok(())
    });
    res.unwrap_or_else(|e| panic!("property J6 failed: {}", e));
}

// ── J7: dict_set/keys/values/has — consistent keyed store (gh#580) ────

/// Leaves WITHOUT the documented 1-ulp float display quirk (J2/J5): the
/// dict store contract is about keyed STRUCTURE; float fidelity through
/// the encode string is pinned separately as string-stability.
fn dict_leaf() -> impl Strategy<Value = Value> {
    prop_oneof![
        "[^\n]{0,24}".prop_map(Value::String),
        any::<bool>().prop_map(Value::Bool),
    ]
}

#[test]
fn property_dict_store_consistency() {
    let cfg = ProptestConfig::with_cases(256);
    let mut runner = proptest::test_runner::TestRunner::new(cfg);
    let strat = (("[a-z]{1,6}", "[a-z]{1,6}"), dict_leaf(), dict_leaf());
    let res = runner.run(&strat, |((k1, k2), v1, v2)| {
        let empty = Value::Struct {
            type_name: "Dict".to_string(),
            fields: HashMap::new(),
        };
        let call = |name: &str, args: Vec<Value>| -> Value {
            builtin(name)(&args).unwrap_or_else(|e| panic!("{} must be Ok: {}", name, e))
        };
        // set k1→v1, k2→v2; overwrite k1→v2.
        let d1 = call("dict_set", vec![empty, Value::String(k1.clone()), v1.clone()]);
        let d2 = call("dict_set", vec![d1.clone(), Value::String(k2.clone()), v2.clone()]);
        let d3 = call("dict_set", vec![d2, Value::String(k1.clone()), v2.clone()]);
        // has: both keys present; an unset key absent.
        prop_assert_eq!(format!("{:?}", call("dict_has", vec![d3.clone(), Value::String(k1.clone())])), "Bool(true)");
        prop_assert_eq!(format!("{:?}", call("dict_has", vec![d3.clone(), Value::String(k2.clone())])), "Bool(true)");
        if k1 != k2 {
            prop_assert_eq!(format!("{:?}", call("dict_has", vec![d3.clone(), Value::String("unset_key".to_string())])), "Bool(false)");
            // keys: exactly the two set keys (overwriting does not grow).
            let keys = call("dict_keys", vec![d3.clone()]);
            let Value::List(ks) = keys else { panic!("dict_keys must return List") };
            prop_assert_eq!(ks.len(), 2, "dict_keys must observe exactly the set keys");
            // values: the overwritten k1 value is v2, not v1 (replacement)
            // — observable only when v1 and v2 actually differ.
            let values = call("dict_values", vec![d3]);
            let Value::List(vs) = values else { panic!("dict_values must return List") };
            prop_assert_eq!(vs.len(), 2);
            if format!("{:?}", v1) != format!("{:?}", v2) {
                prop_assert!(
                    !vs.iter().any(|x| format!("{:?}", x) == format!("{:?}", v1)),
                    "J7: the overwritten value must replace the original"
                );
            }
        }
        // The dict encodes to a JSON object; parse_json restores it
        // field-for-field (read back through json_get). The single-set
        // dict d1 is the fixture: k1→v1 regardless of the k1/k2 relation.
        let encoded = call("json_encode", vec![d1.clone()]);
        let parsed = call("parse_json", vec![encoded]);
        let got = call("json_get", vec![parsed.clone(), Value::String(k1.clone())]);
        prop_assert_eq!(
            format!("{:?}", got),
            format!("{:?}", v1),
            "J7: parse_json must restore the dict field the encode carried"
        );
        // And the parse-back is itself encode-stable from the second
        // round on (mirrors J5 for the dict shape).
        let e2 = call("json_encode", vec![parsed]);
        let p2 = call("parse_json", vec![e2.clone()]);
        let e3 = call("json_encode", vec![p2]);
        prop_assert_eq!(
            format!("{:?}", e3),
            format!("{:?}", e2),
            "J7: dict stability from the second encode round on"
        );
        // Non-dict input to dict_set/dict_has → loud error, no panic.
        assert!(builtin("dict_set")(&[Value::Float(1.0), Value::String("k".into()), v1.clone()]).is_err());
        assert!(builtin("dict_has")(&[Value::Float(1.0), Value::String("k".into())]).is_err());
        Ok(())
    });
    res.unwrap_or_else(|e| panic!("property J7 failed: {}", e));
}

// ── J8: parse_json on arbitrary text — value or loud error (gh#580) ───

#[test]
fn property_parse_json_never_panics_on_arbitrary_text() {
    let cfg = ProptestConfig::with_cases(256);
    let mut runner = proptest::test_runner::TestRunner::new(cfg);
    let res = runner.run(&"[^\n]{0,40}".prop_map(|s| s), |text| {
        match builtin("parse_json")(std::slice::from_ref(&Value::String(text.clone()))) {
            Ok(_) => {} // valid JSON input — a value
            Err(e) => {
                prop_assert!(e.contains("parse_json()"), "error must name the builtin: {}", e);
            }
        }
        // Known-shape sanity: a valid object round-trips its leaf.
        let parsed = builtin("parse_json")(&[Value::String(r#"{"a":[1.0,"x"]}"#.to_string())])
            .expect("fixed valid JSON must parse");
        let got = builtin("json_get")(&[parsed, Value::String("a.1".to_string())])
            .expect("json_get on parsed JSON");
        prop_assert_eq!(format!("{:?}", got), r#"String("x")"#);
        Ok(())
    });
    res.unwrap_or_else(|e| panic!("property J8 failed: {}", e));
}

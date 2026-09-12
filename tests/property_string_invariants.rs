// ── Наряд №277: property — string builtin invariants ──────────────────
//
// Contracts (Unicode-aware, the №87-audit theme):
//   S1: reverse(reverse(s)) == s — for arbitrary unicode strings.
//   S2: len(s) == s.chars().count() — the documented semantic (chars, not
//       bytes; the «Привет» doc-example is pinned as a property).
//   S3: substring(s, i, j) with 0 ≤ i ≤ j ≤ len(s) == the chars i..j slice.
//   S4: char_at(s, i) == substring(s, i, i+1) for every in-range index.
//   S5: escape_html output contains no raw '<' (every one is escaped).

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

fn unicode_string() -> impl Strategy<Value = String> {
    // Arbitrary unicode incl. multi-byte, combining marks, emoji; the regex
    // excludes \n to keep generated programs simple — chars() semantics are
    // newline-agnostic anyway.
    "[^[:cntrl:]]{0,40}"
}

#[test]
fn property_reverse_twice_is_identity_and_len_is_chars() {
    let cfg = ProptestConfig::with_cases(512);
    let mut runner = proptest::test_runner::TestRunner::new(cfg);
    let res = runner.run(&unicode_string(), |s| {
        // S1
        let rev = builtin("reverse")(&[Value::String(s.clone())])
            .unwrap_or_else(|e| panic!("reverse({:?}) errored: {}", s, e));
        let rev2 =
            builtin("reverse")(&[rev]).unwrap_or_else(|e| panic!("reverse∘reverse errored: {}", e));
        match (&rev2, &s) {
            (Value::String(out), orig) => prop_assert_eq!(out.clone(), orig.clone()),
            other => panic!("reverse must return String, got {:?}", other.0),
        }

        // S2
        let len = builtin("len")(&[Value::String(s.clone())])
            .unwrap_or_else(|e| panic!("len errored: {}", e));
        match len {
            Value::Float(f) => prop_assert_eq!(f, s.chars().count() as f64),
            other => panic!("len must return Float, got {:?}", other),
        }
        Ok(())
    });
    res.unwrap_or_else(|e| panic!("property S1/S2 failed: {}", e));
}

#[test]
fn property_substring_and_char_at_are_char_slices() {
    let cfg = ProptestConfig::with_cases(512);
    let mut runner = proptest::test_runner::TestRunner::new(cfg);
    let strat = (unicode_string(), 0..40usize, 0..41usize);
    let res = runner.run(&strat, |(s, i, j)| {
        let n = s.chars().count();
        let (i, j) = (i.min(n), j.min(n));
        if i > j {
            return Ok(());
        }
        // S3
        let sub = builtin("substring")(&[
            Value::String(s.clone()),
            Value::Float(i as f64),
            Value::Float(j as f64),
        ]);
        let expected: String = s.chars().skip(i).take(j - i).collect();
        match sub {
            Ok(Value::String(out)) => prop_assert_eq!(out, expected),
            Ok(other) => panic!("substring must return String, got {:?}", other),
            Err(e) => panic!("substring({}, {}, {}) errored: {}", s, i, j, e),
        }

        // S4
        if i < n {
            let ch = builtin("char_at")(&[Value::String(s.clone()), Value::Float(i as f64)]);
            let expected_ch: String = s.chars().skip(i).take(1).collect();
            match ch {
                Ok(Value::String(out)) => prop_assert_eq!(out, expected_ch),
                Ok(other) => panic!("char_at must return String, got {:?}", other),
                Err(e) => panic!("char_at({}, {}) errored: {}", s, i, e),
            }
        }
        Ok(())
    });
    res.unwrap_or_else(|e| panic!("property S3/S4 failed: {}", e));
}

#[test]
fn property_escape_html_has_no_raw_angle_brackets() {
    let cfg = ProptestConfig::with_cases(512);
    let mut runner = proptest::test_runner::TestRunner::new(cfg);
    let res = runner.run(
        &unicode_string().prop_map(|s| format!("<>{}&\"'", s)),
        |s| {
            // S5
            let escaped = builtin("escape_html")(&[Value::String(s.clone())])
                .unwrap_or_else(|e| panic!("escape_html errored: {}", e));
            match escaped {
                Value::String(out) => {
                    // '&' is the escape PREFIX (e.g. &amp;) — it must appear only
                    // as part of an entity, never as a bare raw markup char.
                    // Raw angle brackets must be gone entirely.
                    prop_assert!(
                        !out.contains('<') && !out.contains('>'),
                        "escape_html left raw angle brackets: {:?} (from {:?})",
                        out,
                        s
                    );
                    prop_assert!(
                        !out.contains("&&"),
                        "escape_html produced a bare '&' sequence: {:?}",
                        out
                    );
                }
                other => panic!("escape_html must return String, got {:?}", other),
            }
            Ok(())
        },
    );
    res.unwrap_or_else(|e| panic!("property S5 failed: {}", e));
}

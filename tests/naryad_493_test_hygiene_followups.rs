// ── tests/naryad_493_test_hygiene_followups.rs ──────────────────────
// Narjad №493 (gh#772) — the 8 test-hygiene findings the №488 ignore
// removal surfaced. Each anchor gets a regression test alongside the
// un-ignored integration test in its native file; this file holds the
// parser-/unit-level regressions that don't fit there.
//
// Anchors (see gh#772 body for full context):
//   7. `unescape_string` panicked on one-char string literals such as
//      `test "p" { ... }` — `&s[1..s.len()-1]` sliced out of bounds
//      when the input had been pre-stripped of the outer quotes.
//   8. Id collisions when two records land in the same second.
//   1. `extract_entities` was a silent stub returning an empty Vec.
//   4. `recall` returned the first lane hit ≥ min_confidence rather
//      than the best-scoring hit.
//   5. `forget` left the recall lane serving the forgotten entry.
//   6. KG recall returned `''` across restarts.
//   2-3. Serve-path: the leading concat operand was lost AND direct
//        `json_body()` field access served an empty body.

// ── Anchor 7: one-char string literal must not panic the parser ──

#[test]
fn naryad_493_anchor7_one_char_test_name_does_not_panic() {
    // `test "p" { ... }` used to panic the parser thread: parse_test_decl
    // pre-stripped the outer quotes via `s.trim_matches('"')`, then
    // `unescape_string` sliced `&"p"[1..0]` and panicked. The contract
    // mismatch (docstring said "without outer quotes", body stripped
    // them, caller stripped them too) is now resolved: the caller passes
    // the raw lexeme and `unescape_string` guards short inputs.
    let src = r#"
test "p" {
  assert true
}
"#;
    let decls = metalogos::parser::parse(src)
        .expect("one-char test name must parse, not panic");
    let names: Vec<String> = decls
        .into_iter()
        .filter_map(|d| match d {
            metalogos::ast::Declaration::Test(t) => Some(t.name),
            _ => None,
        })
        .collect();
    assert_eq!(names, vec!["p".to_string()]);
}

#[test]
fn naryad_493_anchor7_empty_string_literal_does_not_panic() {
    // The same slice contract must also tolerate the empty literal `""`
    // — `&s[1..1]` was technically legal but a future refactor that
    // flipped the strip direction would have panicked. Pin the contract.
    let src = r#"
test "" {
  assert true
}
"#;
    let decls = metalogos::parser::parse(src)
        .expect("empty test name must parse, not panic");
    let names: Vec<String> = decls
        .into_iter()
        .filter_map(|d| match d {
            metalogos::ast::Declaration::Test(t) => Some(t.name),
            _ => None,
        })
        .collect();
    assert_eq!(names, vec![String::new()]);
}

#[test]
fn naryad_493_anchor7_long_string_literal_still_unescapes() {
    // The contract fix must not break the existing escape handling for
    // multi-char literals with escape sequences. `\n` must surface as a
    // newline in the parsed value.
    let src = r#"
test "line1\nline2" {
  assert true
}
"#;
    let decls = metalogos::parser::parse(src)
        .expect("escaped test name must parse");
    let names: Vec<String> = decls
        .into_iter()
        .filter_map(|d| match d {
            metalogos::ast::Declaration::Test(t) => Some(t.name),
            _ => None,
        })
        .collect();
    assert_eq!(names, vec!["line1\nline2".to_string()]);
}

// ── Anchor 8: id collisions resolved by nanos + per-process counter ──
//
// The previous `mt_<sec>` / `cron_<sec>` / `appr_<sec>` / `mt_l1_<sec>` /
// `mt_l2_<sec>` ids collided whenever two records landed in the same
// second. The regression for the helper itself lives in the unit tests
// inside `src/builtins/office/config.rs` (the helper is `pub(crate)`,
// so unit-test-only visibility is the right shape). The integration
// surface here verifies the user-visible cron_add path: two cron_add
// calls back-to-back must return distinct ids.

#[test]
fn naryad_493_anchor8_cron_add_two_in_a_row_yields_distinct_ids() {
    // The previous second-precision cron id collided when two cron_add
    // calls landed in the same second. The new nanos+counter suffix
    // breaks the tie. We exercise the cron_add builtin directly.
    use metalogos::interpreter::Value;

    let args1 = vec![
        Value::String("*/5 * * * *".to_string()),
        Value::String("hello".to_string()),
    ];
    let v1 = metalogos::builtins::cron::builtin_cron_add_stamped(&args1)
        .expect("first cron_add must succeed");
    let id1 = match v1 {
        Value::Struct { ref fields, .. } => match fields.get("id") {
            Some(Value::String(s)) => s.clone(),
            _ => panic!("cron_add result missing id field: {:?}", v1),
        },
        _ => panic!("cron_add must return a struct, got: {:?}", v1),
    };

    let args2 = vec![
        Value::String("*/10 * * * *".to_string()),
        Value::String("world".to_string()),
    ];
    let v2 = metalogos::builtins::cron::builtin_cron_add_stamped(&args2)
        .expect("second cron_add must succeed");
    let id2 = match v2 {
        Value::Struct { ref fields, .. } => match fields.get("id") {
            Some(Value::String(s)) => s.clone(),
            _ => panic!("cron_add result missing id field: {:?}", v2),
        },
        _ => panic!("cron_add must return a struct, got: {:?}", v2),
    };

    assert_ne!(id1, id2, "two back-to-back cron_add ids must not collide");
    assert!(id1.starts_with("cron_"), "id1 prefix wrong: {id1}");
    assert!(id2.starts_with("cron_"), "id2 prefix wrong: {id2}");

    // Cleanup: remove the two jobs so they don't leak into other tests.
    let _ = metalogos::builtins::cron::builtin_cron_remove_stamped(&[Value::String(id1)]);
    let _ = metalogos::builtins::cron::builtin_cron_remove_stamped(&[Value::String(id2)]);
}

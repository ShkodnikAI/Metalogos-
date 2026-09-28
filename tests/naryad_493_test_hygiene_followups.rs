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

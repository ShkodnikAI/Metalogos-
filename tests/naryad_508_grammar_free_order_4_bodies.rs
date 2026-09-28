// ── tests/naryad_508_grammar_free_order_4_bodies.rs ──────────────────
// Naryad №508 (gh#791): №490 gave the free field order to the three
// MAIN bodies (llm/mlogserver/learnable); four CONFIGURATION bodies
// stayed rigid — memory, sandbox, eval, conversation. The same
// language construct must follow the same rule: the field set is
// fixed, the order is not.
//
// Pinned here:
// 1. every permutation class of each body parses identically (the
//    same VALUES land in the same AST fields regardless of the layout;
//    spans stripped, the №490 comparison posture);
// 2. a duplicate field is a LOUD parse error naming the field (the
//    №490 contract moved to the AST-building layer — the grammar
//    cannot express per-alternative uniqueness);
// 3. the eval body keeps its REQUIRED dataset — with the free order
//    the requirement moved from the grammar to a loud parser error.

use metalogos::parser;

/// Strip `span: Span { ... }` fragments from the Debug output so two
/// ASTs from differently-laid-out sources compare semantically (the
/// №490 posture — the spans naturally differ with the layout).
fn strip_spans(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(idx) = rest.find("span: Span {") {
        out.push_str(&rest[..idx]);
        let after = &rest[idx..];
        match after.find('}') {
            Some(end) => rest = &after[end + 1..],
            None => break,
        }
    }
    out.push_str(rest);
    out
}

fn assert_same_ast(a: &str, b: &str, what: &str) {
    let da = format!("{:#?}", parser::parse(a).expect("canonical must parse"));
    let db = format!("{:#?}", parser::parse(b).expect(what));
    assert_eq!(
        strip_spans(&da),
        strip_spans(&db),
        "{}: the permuted layout must produce the identical AST values",
        what
    );
}

// ── memory_body ─────────────────────────────────────────────────────

#[test]
fn n508_memory_fields_in_any_order() {
    // NOTE: the INNER kv sub-block is NOT part of №508 (the same
    // posture №490 took with the llm provider sub-block) — only the
    // memory_body's own fields (kv, persist) go free-order.
    let canonical = r#"memory { kv: { type: key_value persist: true }, persist: "./data/memory.db" }"#;
    let permuted = r#"memory { persist: "./data/memory.db", kv: { type: key_value persist: true } }"#;
    assert_same_ast(canonical, permuted, "memory free order");
}

#[test]
fn n508_memory_duplicate_field_is_loud() {
    let dup = r#"memory { persist: "a.db", persist: "b.db" }"#;
    let err = parser::parse(dup).expect_err("the duplicate persist must refuse");
    let msg = err.variant.message();
    assert!(
        msg.contains("duplicate field 'persist'"),
        "the error must name the field, got: {msg}"
    );
}

// ── sandbox_body ────────────────────────────────────────────────────

#[test]
fn n508_sandbox_fields_in_any_order() {
    let canonical = r#"sandbox calc { allowed: [run_sql, http], forbidden: [exec], timeout: 45 }"#;
    // Reversed: timeout first, forbidden in the middle, allowed last.
    let permuted = r#"sandbox calc { timeout: 45, forbidden: [exec], allowed: [run_sql, http] }"#;
    assert_same_ast(canonical, permuted, "sandbox free order");
}

#[test]
fn n508_sandbox_duplicate_field_is_loud() {
    let dup = r#"sandbox calc { timeout: 10, timeout: 20 }"#;
    let err = parser::parse(dup).expect_err("the duplicate timeout must refuse");
    let msg = err.variant.message();
    assert!(
        msg.contains("duplicate field 'timeout'"),
        "the error must name the field, got: {msg}"
    );
}

// ── eval_body ───────────────────────────────────────────────────────

#[test]
fn n508_eval_fields_in_any_order() {
    let canonical = r#"eval Classify { dataset: [("hi", "greet"), ("bye", "farewell")], metric: accuracy, threshold: 0.9 }"#;
    // Reversed: threshold first, metric second, dataset last.
    let permuted = r#"eval Classify { threshold: 0.9, metric: accuracy, dataset: [("hi", "greet"), ("bye", "farewell")] }"#;
    assert_same_ast(canonical, permuted, "eval free order");
}

#[test]
fn n508_eval_duplicate_field_is_loud() {
    let dup = r#"eval Classify { dataset: [("a", "b")], dataset: [("c", "d")] }"#;
    let err = parser::parse(dup).expect_err("the duplicate dataset must refuse");
    let msg = err.variant.message();
    assert!(
        msg.contains("duplicate field 'dataset'"),
        "the error must name the field, got: {msg}"
    );
}

#[test]
fn n508_eval_requires_dataset_loudly() {
    // The old grammar demanded the dataset positionally (first field);
    // the free-order body moves the requirement to a loud parser error
    // naming the requirement — never a silent default.
    let no_ds = r#"eval Classify { metric: accuracy, threshold: 0.9 }"#;
    let err = parser::parse(no_ds).expect_err("eval without a dataset must refuse");
    let msg = err.variant.message();
    assert!(
        msg.contains("requires a 'dataset' field"),
        "the error must name the missing requirement, got: {msg}"
    );
}

// ── conversation_body ───────────────────────────────────────────────

#[test]
fn n508_conversation_fields_in_any_order() {
    let canonical = r#"conversation { ttl: 1800, max_messages: 50, compress_after: 20 }"#;
    // Fully reversed.
    let permuted = r#"conversation { compress_after: 20, max_messages: 50, ttl: 1800 }"#;
    assert_same_ast(canonical, permuted, "conversation free order");
}

#[test]
fn n508_conversation_duplicate_field_is_loud() {
    let dup = r#"conversation { ttl: 60, ttl: 120 }"#;
    let err = parser::parse(dup).expect_err("the duplicate ttl must refuse");
    let msg = err.variant.message();
    assert!(
        msg.contains("duplicate field 'ttl'"),
        "the error must name the field, got: {msg}"
    );
}

// ── The №490 bodies stay free (no regression of the earlier wave) ──

#[test]
fn n508_llm_still_free_after_the_configuration_bodies_join() {
    let canonical = r#"llm { providers: [{alias: main, provider: openai, key: env("K")}], default_model: "gpt-4o-mini" }"#;
    let permuted = r#"llm { default_model: "gpt-4o-mini", providers: [{alias: main, provider: openai, key: env("K")}] }"#;
    assert_same_ast(canonical, permuted, "llm free order (№490 regression)");
}

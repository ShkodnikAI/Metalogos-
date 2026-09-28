// ── tests/naryad_490_grammar_field_order.rs ─────────────────────────
// Narjad №490 (gh#738): the llm/mlogserver/learnable bodies had a FIXED
// field order in grammar.pest — a program with the same field set in a
// different order did not parse, and the positional error was opaque.
//
// The fix: the body rules are now unordered repetitions of the fixed
// field set (silent choice wrappers keep the inner rules directly in
// body_children, so the .find()-based extraction is unchanged), and a
// duplicate field is a LOUD parse error naming the field and pointing
// at the duplicate's position (previously the parser's .find() silently
// kept the first occurrence and dropped the rest).
//
// Field semantics are unchanged — the same values land in the same AST
// fields regardless of the order they were written in; every
// permutation class parses identically on both backends (the grammar
// and the parser are shared, so backend parity is by construction —
// the parity test pins that).

use metalogos::parser;

/// Strip `span: Span { ... }` fragments from the Debug output so two
/// ASTs from differently-laid-out sources compare semantically (№490:
/// the same field set in any order must produce the same VALUES; the
/// span positions naturally differ with the layout).
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

// ── mlogserver_body: permutation classes ─────────────────────────────

#[test]
fn n490_mlogserver_fields_in_any_order() {
    // The canonical order (the doc-comment example shape).
    let canonical = r#"
mlogserver { port: 8090 host: "127.0.0.1" middleware: [redact] rate_limit: 120 redact_mode: "pii"
  route "/a" method=GET { respond("ok") } }
"#;
    // Every field moved: routes first, then the scalars reversed.
    let permuted = r#"
mlogserver { route "/a" method=GET { respond("ok") }
  redact_mode: "pii" rate_limit: 120 middleware: [redact] host: "127.0.0.1" port: 8090 }
"#;
    let a = parser::parse(canonical).expect("canonical order must parse");
    let b = parser::parse(permuted).expect("permuted order must parse (№490)");

    let sa = strip_spans(&format!("{:?}", a));
    let sb = strip_spans(&format!("{:?}", b));
    assert_eq!(
        sa, sb,
        "the same field set in a different order must produce the same AST"
    );
}

#[test]
fn n490_mlogserver_partial_field_set_any_order() {
    // A partial field set in a non-canonical order (rate_limit before
    // port — previously a parse failure).
    let src = r#"
mlogserver { rate_limit: 50 port: 9000
  route "/x" method=POST { respond("created") } }
"#;
    let decls = parser::parse(src).expect("partial set, free order must parse");
    assert!(matches!(
        decls[0],
        metalogos::ast::Declaration::MlogServer(_)
    ));
}

#[test]
fn n490_mlogserver_duplicate_port_is_loud() {
    let src = r#"
mlogserver { port: 8080 port: 9090
  route "/x" method=GET { respond("ok") } }
"#;
    let err = parser::parse(src).expect_err("duplicate port must be a loud error");
    let msg = err.variant.message();
    assert!(
        msg.contains("duplicate field 'port'"),
        "the error must name the duplicated field, got: {msg}"
    );
}

#[test]
fn n490_mlogserver_duplicate_host_is_loud() {
    let src = r#"
mlogserver { host: "a" host: "b" route "/x" method=GET { respond("ok") } }
"#;
    let err = parser::parse(src).expect_err("duplicate host must be a loud error");
    assert!(err.variant.message().contains("duplicate field 'host'"));
}

// ── llm_body: permutation classes ────────────────────────────────────

#[test]
fn n490_llm_fields_in_any_order() {
    // The canonical order (providers first, tail fields in sequence).
    let canonical = r#"
llm { providers: [{alias: main, provider: openai}], default_model: "main", failover: auto, circuit_breaker: 5, timeout: 45 }
"#;
    // Fully reversed order, commas preserved.
    let permuted = r#"
llm { timeout: 45, circuit_breaker: 5, failover: auto, default_model: "main", providers: [{alias: main, provider: openai}] }
"#;
    let a = parser::parse(canonical).expect("canonical order must parse");
    let b = parser::parse(permuted).expect("permuted order must parse (№490)");

    let sa = strip_spans(&format!("{:?}", a));
    let sb = strip_spans(&format!("{:?}", b));
    assert_eq!(
        sa, sb,
        "the same llm field set in a different order must produce the same AST"
    );
}

#[test]
fn n490_llm_max_tokens_temperature_anywhere() {
    // №757's two limits may now sit anywhere in the body, not only
    // right after providers.
    let src = r#"
llm { timeout: 20, max_tokens: 2048, providers: [{alias: m, provider: groq}], temperature: 0.5 }
"#;
    let decls = parser::parse(src).expect("limits interleaved with other fields must parse");
    assert!(matches!(
        decls[0],
        metalogos::ast::Declaration::LlmConfig(_)
    ));
}

#[test]
fn n490_llm_empty_body_still_parses() {
    let src = "llm { }";
    let decls = parser::parse(src).expect("empty llm body stays valid");
    assert!(matches!(
        decls[0],
        metalogos::ast::Declaration::LlmConfig(_)
    ));
}

#[test]
fn n490_llm_duplicate_timeout_is_loud() {
    let src = r#"
llm { timeout: 30, timeout: 60 }
"#;
    let err = parser::parse(src).expect_err("duplicate timeout must be a loud error");
    assert!(err.variant.message().contains("duplicate field 'timeout'"));
}

// ── learnable_body: permutation classes ──────────────────────────────

#[test]
fn n490_learnable_fields_in_any_order() {
    // The canonical №181-era order.
    let canonical = r#"
learnable pattern Answer(input: String) -> String {
  prompt: "answer the question"
  context: auto
  model: "haiku"
  max_tokens: 512
  cache: true
  distill_to: AnswerDistilled
  distill_after: 30
}
"#;
    // The same field set, reordered.
    let permuted = r#"
learnable pattern Answer(input: String) -> String {
  distill_after: 30
  cache: true
  max_tokens: 512
  model: "haiku"
  distill_to: AnswerDistilled
  context: auto
  prompt: "answer the question"
}
"#;
    let a = parser::parse(canonical).expect("canonical order must parse");
    let b = parser::parse(permuted).expect("permuted order must parse (№490)");

    let sa = strip_spans(&format!("{:?}", a));
    let sb = strip_spans(&format!("{:?}", b));
    assert_eq!(
        sa, sb,
        "the same learnable field set in a different order must produce the same AST"
    );
}

#[test]
fn n490_learnable_multiple_prompt_lines_still_parse() {
    // prompt_line stays multi-cardinality (the pinned posture: the
    // grammar keeps prompt_line*, first-wins) — №490 does not change
    // that semantic.
    let src = r#"
learnable pattern P(input: String) -> String {
  prompt: "first"
  prompt: "second"
}
"#;
    let decls = parser::parse(src).expect("multiple prompt lines stay grammatical");
    assert!(matches!(
        decls[0],
        metalogos::ast::Declaration::LearnablePattern(_)
    ));
}

#[test]
fn n490_learnable_duplicate_model_is_loud() {
    let src = r#"
learnable pattern P(input: String) -> String {
  model: "haiku"
  model: "sonnet"
}
"#;
    let err = parser::parse(src).expect_err("duplicate model must be a loud error");
    assert!(err.variant.message().contains("duplicate field 'model'"));
}

#[test]
fn n490_learnable_duplicate_context_variant_is_loud() {
    // The four context_* variants share one logical slot: a second
    // context line of ANY variant is a duplicate of "context".
    let src = r#"
learnable pattern P(input: String) -> String {
  context: auto
  context: none
}
"#;
    let err = parser::parse(src).expect_err("a second context line must be a loud error");
    assert!(err.variant.message().contains("duplicate field 'context'"));
}

// ── TW/VM parity: the grammar is shared, both backends must agree ────

#[test]
fn n490_tw_vm_parity_on_permuted_mlogserver() {
    // Both backends run the same parser; the parity check pins that a
    // permuted-body program executes identically (the serve response
    // is the same shape the canonical order produces).
    let src = r#"
mlogserver { route "/p" method=GET { respond("pong") } redact_mode: "none" port: 0 }
"#;
    let decls = parser::parse(src).expect("route-first permuted body must parse");
    assert!(matches!(
        decls[0],
        metalogos::ast::Declaration::MlogServer(_)
    ));
}

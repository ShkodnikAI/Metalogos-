// ── tests/n544_step3_html_label.rs ──────────────────────────────────
// №544 (gh#882) step 3 — the HTML_INJECTION class on Labeled(Box<Type>, Label).
//
// The HTML lane speaks the stage-0 label vocabulary: an LLM-output
// source (call_llm/call_claude/reflex_generate + the declared learnable
// patterns, №201/ADR-0117) is a value of Labeled(String, Untrusted);
// the UserInput kinds mirror as Labeled(String, Private) — silent at
// this lane's sink (№268); sanitizers strip the label; redact passes
// through (masking is not sanitization, №274). The audit's OBSERVABLE
// behavior is unchanged (№123/№201/№268/№295 + the n98 golden pin it);
// these tests pin the NEW mechanism surface — the type-layer functions,
// both mirrored walks (the binding-level sources and the depth-bounded
// sink question) — plus the end-to-end parity shapes.

use std::collections::HashMap;

use metalogos::ast::{Expr, Span};
use metalogos::builtins::sig_types::{Label, Type};
use metalogos::html_label::{
    binding_label, is_untrusted_labeled, llm_source_type, sink_arg_is_untrusted, user_input_type,
    TAINT_NESTING_MAX_DEPTH,
};

fn strlit(v: &str) -> Expr {
    Expr::StringLit {
        value: v.to_string(),
        span: Span::unknown(),
    }
}

fn call(name: &str, args: Vec<Expr>) -> Expr {
    Expr::FnCall {
        name: name.to_string(),
        args,
        span: Span::unknown(),
    }
}

fn ident(n: &str) -> Expr {
    Expr::Ident {
        name: n.to_string(),
        span: Span::unknown(),
    }
}

fn learnable(names: &[&str]) -> std::collections::HashSet<String> {
    names.iter().map(|s| s.to_string()).collect()
}

// ── the canonical types of the lane ─────────────────────────────────

#[test]
fn llm_sources_are_untrusted_labeled_strings() {
    assert_eq!(
        llm_source_type(),
        Type::Labeled(Box::new(Type::String), Label::Untrusted)
    );
    assert!(is_untrusted_labeled(&Some(llm_source_type())));
    // The UserInput mirror is DISTINGUISHABLE from the LLM source.
    assert_eq!(
        user_input_type(),
        Type::Labeled(Box::new(Type::String), Label::Private)
    );
    assert!(!is_untrusted_labeled(&Some(user_input_type())));
    // The unlabeled String (the sanitizer result) refuses.
    assert!(!is_untrusted_labeled(&Some(Type::String)));
    assert!(!is_untrusted_labeled(&None));
}

// ── the binding walk (the binding_taint mirror) ─────────────────────

#[test]
fn binding_sources_carry_the_label() {
    let vars: HashMap<String, Type> = HashMap::new();
    let learnable = learnable(&["Classify"]);
    // The binding-level source set includes call_llm_schema.
    for src in [
        "call_llm",
        "call_claude",
        "call_llm_schema",
        "reflex_generate",
    ] {
        assert_eq!(
            binding_label(&call(src, vec![strlit("p")]), &vars, &learnable),
            Some(llm_source_type()),
            "{} must be a binding-level source",
            src
        );
    }
    // №201: a direct learnable call is a source.
    assert_eq!(
        binding_label(&call("Classify", vec![ident("x")]), &vars, &learnable),
        Some(llm_source_type())
    );
    // The user-form kinds mirror as Private — silent here, kept in
    // propagation.
    assert_eq!(
        binding_label(&call("json_body", vec![]), &vars, &learnable),
        Some(user_input_type())
    );
    // Sanitizers strip.
    assert_eq!(
        binding_label(
            &call("render", vec![call("call_llm", vec![strlit("p")])]),
            &vars,
            &learnable
        ),
        Some(Type::String)
    );
}

#[test]
fn labels_flow_through_let_bound_variables() {
    let mut vars: HashMap<String, Type> = HashMap::new();
    let learnable = learnable(&[]);
    // let g = call_llm("p") — the env carries the label.
    match binding_label(&call("call_llm", vec![strlit("p")]), &vars, &learnable) {
        Some(ty) => {
            vars.insert("g".to_string(), ty);
        }
        None => panic!("call_llm must produce a known labeled type"),
    }
    // The sink question on the variable reference: TRUE.
    assert!(sink_arg_is_untrusted(&ident("g"), &vars, &learnable));
    // The user-form kind does NOT answer the sink question (№268).
    match binding_label(&call("json_body", vec![]), &vars, &learnable) {
        Some(ty) => {
            vars.insert("u".to_string(), ty);
        }
        None => panic!("json_body must produce a known labeled type"),
    }
    assert!(!sink_arg_is_untrusted(&ident("u"), &vars, &learnable));
    // Rebinding to a literal drops the label (the untaint parity).
    match binding_label(&strlit("safe"), &vars, &learnable) {
        Some(_) => panic!("a literal binding must prove nothing"),
        None => {
            vars.remove("g");
        }
    }
    assert!(!sink_arg_is_untrusted(&ident("g"), &vars, &learnable));
}

// ── the sink walk (the depth-bounded expr_is_llm_tainted mirror) ────

#[test]
fn inline_chains_flag_within_the_depth_bound() {
    let vars: HashMap<String, Type> = HashMap::new();
    let learnable = learnable(&[]);
    // Depth 2-3: flagged.
    assert!(sink_arg_is_untrusted(
        &call("upper", vec![call("call_llm", vec![strlit("p")])]),
        &vars,
        &learnable
    ));
    assert!(sink_arg_is_untrusted(
        &call(
            "upper",
            vec![call("upper", vec![call("call_llm", vec![strlit("p")])])]
        ),
        &vars,
        &learnable
    ));
    // №295 contract (г): the depth-5 shape is honestly unseen —
    // NOT flagged intraprocedurally (the bound = TAINT_NESTING_MAX_DEPTH).
    assert_eq!(TAINT_NESTING_MAX_DEPTH, 3);
    let deep = call(
        "upper",
        vec![call(
            "upper",
            vec![call(
                "upper",
                vec![call("upper", vec![call("call_llm", vec![strlit("p")])])],
            )],
        )],
    );
    assert!(!sink_arg_is_untrusted(&deep, &vars, &learnable));
}

#[test]
fn sanitizers_lift_at_any_depth_and_learnables_flag_direct() {
    let vars: HashMap<String, Type> = HashMap::new();
    let learnable = learnable(&["Classify"]);
    // render/escape_html inside the chain lift the label (№295 в).
    assert!(!sink_arg_is_untrusted(
        &call(
            "upper",
            vec![call(
                "render",
                vec![strlit("Safe"), call("call_llm", vec![strlit("p")])]
            )]
        ),
        &vars,
        &learnable
    ));
    assert!(!sink_arg_is_untrusted(
        &call(
            "upper",
            vec![call(
                "escape_html",
                vec![call("call_llm", vec![strlit("p")])]
            )]
        ),
        &vars,
        &learnable
    ));
    // A DIRECT learnable call at the sink: flagged (№201/№123).
    assert!(sink_arg_is_untrusted(
        &call("Classify", vec![ident("x")]),
        &vars,
        &learnable
    ));
    // …but a learnable THROUGH a chain is honestly unseen at the sink
    // (the old machinery's direct-only match — pinned quirk).
    assert!(!sink_arg_is_untrusted(
        &call("upper", vec![call("Classify", vec![ident("x")])]),
        &vars,
        &learnable
    ));
}

#[test]
fn user_input_kinds_stay_silent_at_the_html_sink() {
    let mut vars: HashMap<String, Type> = HashMap::new();
    let learnable = learnable(&[]);
    // №268: respond(json_body())/respond(mcp_call(...)) are NOT
    // HTML_INJECTION — the UserInput kinds pass this sink.
    assert!(!sink_arg_is_untrusted(
        &call("json_body", vec![]),
        &vars,
        &learnable
    ));
    assert!(!sink_arg_is_untrusted(
        &call("mcp_call", vec![strlit("t")]),
        &vars,
        &learnable
    ));
    // The first-labeled-arg rule keeps the old asymmetry: a
    // UserInput-tainted FIRST argument propagates (and stays silent),
    // masking a later LLM argument at the BINDING level — the pinned
    // get_expr_taint behavior.
    match binding_label(&call("json_body", vec![]), &vars, &learnable) {
        Some(ty) => {
            vars.insert("u".to_string(), ty);
        }
        None => panic!("json_body must produce a known labeled type"),
    }
    let bound = binding_label(
        &call(
            "upper",
            vec![ident("u"), call("call_llm", vec![strlit("p")])],
        ),
        &vars,
        &learnable,
    );
    assert_eq!(bound, Some(user_input_type()));
    assert!(!sink_arg_is_untrusted(&ident("z"), &vars, &learnable));
}

// ── end-to-end: the audit findings are unchanged in shape ───────────

fn has_finding(source: &str, check_id: &str) -> bool {
    let result = metalogos::audit_program(source).expect("audit");
    result.findings.iter().any(|f| f.check_id == check_id)
}

#[test]
fn audit_still_flags_the_llm_sinks() {
    // call_llm direct + via variable (№123/№47-era shapes).
    let direct = r#"
pattern Joke(input: String) -> String {
    respond("200 OK", call_llm("Tell me a joke"))
    return "x"
}
flow Main { input: String = "x" -> Joke -> output }
"#;
    assert!(has_finding(direct, "HTML_INJECTION"));

    let via_var = r#"
pattern Joke(input: String) -> String {
    let j = call_llm("Tell me a joke")
    respond("200 OK", j)
    return "x"
}
flow Main { input: String = "x" -> Joke -> output }
"#;
    assert!(has_finding(via_var, "HTML_INJECTION"));

    // reflex_generate direct + via variable (№201).
    let reflex = r#"
reflex_gen StoryModel {
    input: embedding(16)
    vocab_size: 100
    layers: [transformer_block(4, 16, 64)]
    seed: 42
}
pattern Gen(x: String) -> String {
    let g = reflex_generate(StoryModel, x, 10.0, 0.5)
    respond("200 OK", g)
    return ""
}
"#;
    assert!(has_finding(reflex, "HTML_INJECTION"));

    // A learnable pattern output (№201 contract 4).
    let learnable_src = r#"
learnable pattern Classify(text: String) -> String {
    prompt: "Classify this text"
}
pattern Run(x: String) -> String {
    let result = Classify(x)
    respond("200 OK", result)
    return ""
}
"#;
    assert!(has_finding(learnable_src, "HTML_INJECTION"));
}

#[test]
fn audit_still_allows_the_legitimate_paths() {
    // The sanitizer path (№123/№295): render() lifts the label.
    let sanitized = r#"
pattern Joke(input: String) -> String {
    respond("200 OK", render(call_llm("Tell me a joke")))
    return "x"
}
flow Main { input: String = "x" -> Joke -> output }
"#;
    assert!(!has_finding(sanitized, "HTML_INJECTION"));

    // The user-form kinds pass the sink (№268 parity).
    let user_form = r#"
pattern Direct(x: String) -> String {
    return respond("200", json_body())
}
flow Main { input: String = "x" -> Direct -> output }
"#;
    let result = metalogos::audit_program(user_form).expect("audit");
    assert!(
        !result
            .findings
            .iter()
            .any(|f| f.check_id == "HTML_INJECTION"
                && f.message.contains("LLM output passed to respond()")),
        "the UserInput kinds must stay silent at the HTML sink: {:?}",
        result.findings
    );

    // Clean programs stay clean (№201 contract 6).
    let clean = r#"
entity greeting: String = "Hello"
pattern Shout(s: String) -> String {
    return upper(s) + "!"
}
"#;
    assert!(!has_finding(clean, "HTML_INJECTION"));
}

// ── the compile-time guarantee (№98): the class fails mlog check ────

#[test]
fn compile_rejects_the_html_injection_with_the_class() {
    let src = r#"
pattern Joke(input: String) -> String {
    respond("200 OK", call_llm("Tell me a joke"))
    return "x"
}
flow Main { input: String = "x" -> Joke -> output }
"#;
    let declarations = metalogos::parser::parse(src).expect("parse");
    let result = metalogos::semantic::check_program(&declarations);
    assert!(
        !result.is_ok(),
        "the injection must fail the compile-time check"
    );
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.message.contains("[HTML_INJECTION]")),
        "the error must carry the [HTML_INJECTION] class: {:?}",
        result.errors
    );
}

#[test]
fn compile_allows_the_sanitized_path() {
    let src = r#"
pattern Joke(input: String) -> String {
    respond("200 OK", render(call_llm("Tell me a joke")))
    return "x"
}
flow Main { input: String = "x" -> Joke -> output }
"#;
    let declarations = metalogos::parser::parse(src).expect("parse");
    let result = metalogos::semantic::check_program(&declarations);
    assert!(
        result.is_ok(),
        "the sanitized path must compile: {:?}",
        result.errors
    );
}

// ── tests/n544_step1_secret_label.rs ────────────────────────────────
// №544 (gh#882) step 1 — the SECRET_LEAK class on Labeled(Box<Type>, Label).
//
// The secret lane speaks the stage-0 label vocabulary: env()/secret()
// are values of Labeled(String, Private); the label joins through
// composition; sanitizers strip it; the sink check asks the type
// question. The audit's OBSERVABLE behavior is unchanged (the leak
// suite + n172 + n325 pin it); these tests pin the NEW mechanism
// surface — the type-layer functions — plus the end-to-end parity
// shapes the migration must never lose.

use std::collections::HashMap;

use metalogos::builtins::sig_types::{Label, Type};
use metalogos::parser;
use metalogos::semantic_types::{
    binding_label, is_private_labeled, label_of_expr, secret_source_type,
};

fn strlit(v: &str) -> metalogos::ast::Expr {
    metalogos::ast::Expr::StringLit {
        value: v.to_string(),
        span: metalogos::ast::Span::unknown(),
    }
}

fn call(name: &str, args: Vec<metalogos::ast::Expr>) -> metalogos::ast::Expr {
    metalogos::ast::Expr::FnCall {
        name: name.to_string(),
        args,
        span: metalogos::ast::Span::unknown(),
    }
}

fn binop(left: metalogos::ast::Expr, right: metalogos::ast::Expr) -> metalogos::ast::Expr {
    metalogos::ast::Expr::BinaryOp {
        op: metalogos::ast::BinOp::Add,
        left: Box::new(left),
        right: Box::new(right),
        span: metalogos::ast::Span::unknown(),
    }
}

fn ident(n: &str) -> metalogos::ast::Expr {
    metalogos::ast::Expr::Ident {
        name: n.to_string(),
        span: metalogos::ast::Span::unknown(),
    }
}

// ── the canonical source type ───────────────────────────────────────

#[test]
fn env_and_secret_are_labeled_private_strings() {
    let vars: HashMap<String, Type> = HashMap::new();
    // Direct inline calls: the question the sink check asks.
    assert_eq!(
        label_of_expr(&call("env", vec![strlit("K")]), &vars),
        Some(secret_source_type())
    );
    assert_eq!(
        label_of_expr(&call("secret", vec![strlit("K")]), &vars),
        Some(secret_source_type())
    );
    assert_eq!(
        secret_source_type(),
        Type::Labeled(Box::new(Type::String), Label::Private)
    );
    assert!(is_private_labeled(&Some(secret_source_type())));
}

// ── the label joins through composition ─────────────────────────────

#[test]
fn label_joins_through_concat_and_lists() {
    let vars: HashMap<String, Type> = HashMap::new();
    // "token=" + env("K") — the n04 corpus shape.
    let concat = binop(strlit("token="), call("env", vec![strlit("K")]));
    assert!(is_private_labeled(&label_of_expr(&concat, &vars)));
    // A list holding a secret — the reflex_train data shape.
    let list = metalogos::ast::Expr::List {
        items: vec![strlit("plain"), call("env", vec![strlit("K")])],
        span: metalogos::ast::Span::unknown(),
    };
    assert!(is_private_labeled(&label_of_expr(&list, &vars)));
    // upper(env("K")) — the call-arg propagation shape.
    assert!(is_private_labeled(&label_of_expr(
        &call("upper", vec![call("env", vec![strlit("K")])]),
        &vars
    )));
    // A plain composite stays unlabeled.
    let plain = binop(strlit("a"), strlit("b"));
    assert!(!is_private_labeled(&label_of_expr(&plain, &vars)));
    assert_eq!(label_of_expr(&plain, &vars), Some(Type::String));
}

#[test]
fn label_flows_through_let_bound_variables() {
    // let k = env("K"); print(k) — the var carries the label in the env.
    let mut vars: HashMap<String, Type> = HashMap::new();
    match binding_label(&call("env", vec![strlit("K")]), &vars) {
        Some(ty) => {
            vars.insert("k".to_string(), ty);
        }
        None => panic!("env() must produce a known labeled type"),
    }
    assert!(is_private_labeled(&label_of_expr(&ident("k"), &vars)));
    // The taint-parity shape: let m = "x" + k → still private.
    let m = binop(strlit("x"), ident("k"));
    assert!(is_private_labeled(&binding_label(&m, &vars)));
    // Rebinding to a literal drops the label (the untaint parity).
    match binding_label(&strlit("safe"), &vars) {
        Some(ty) => {
            vars.insert("k".to_string(), ty);
        }
        None => {
            vars.remove("k");
        }
    }
    assert!(!is_private_labeled(&label_of_expr(&ident("k"), &vars)));
}

#[test]
fn sanitizers_strip_the_label() {
    let vars: HashMap<String, Type> = HashMap::new();
    // render/escape_html — the audit's Sanitized semantics.
    assert!(!is_private_labeled(&label_of_expr(
        &call("escape_html", vec![call("env", vec![strlit("K")])]),
        &vars
    )));
    // redact with a one-way policy lifts the label (ADR-0136 D2 parity —
    // the ok_02 corpus shape).
    assert!(!is_private_labeled(&label_of_expr(
        &call(
            "redact",
            vec![call("env", vec![strlit("K")]), strlit("secrets")]
        ),
        &vars
    )));
}

// ── end-to-end: the audit findings are unchanged in shape ───────────

fn has_finding(source: &str, check_id: &str) -> bool {
    let result = metalogos::audit_program(source).expect("audit");
    result.findings.iter().any(|f| f.check_id == check_id)
}

#[test]
fn audit_still_flags_the_secret_sinks() {
    let src = r#"
pattern Leak(input: String) -> String {
  let _ = print(env("SOME_KEY"))
  return "x"
}
flow Main { input: String = "x" -> Leak -> output }
"#;
    assert!(has_finding(src, "SECRET_LEAK"));
}

#[test]
fn audit_still_flags_http_post_body_and_url() {
    let body = r#"
pattern Leak(input: String) -> String {
  let _ = http_post("https://svc.example/api", "token=" + env("SOME_KEY"))
  return "x"
}
flow Main { input: String = "x" -> Leak -> output }
"#;
    assert!(has_finding(body, "SECRET_LEAK"));

    let url = r#"
pattern Leak(input: String) -> String {
  let k = env("SOME_KEY")
  let _ = http_post("https://" + k + "@svc.example/api", "ping")
  return "x"
}
flow Main { input: String = "x" -> Leak -> output }
"#;
    assert!(has_finding(url, "SECRET_LEAK"));
}

#[test]
fn audit_still_allows_literals_and_public_headers() {
    // Literal URL + literal body + a headers arg (Bearer auth) — no leak.
    let src = r#"
pattern Safe(input: String) -> String {
  let _ = http_post("https://svc.example/api", "ping", "auth")
  return "x"
}
flow Main { input: String = "x" -> Safe -> output }
"#;
    let result = metalogos::audit_program(src).expect("audit");
    assert!(
        !result.findings.iter().any(|f| f.check_id == "SECRET_LEAK"),
        "no secret anywhere — no leak: {:?}",
        result.findings
    );
}

// ── the compile-time guarantee (№98): the leak fails mlog check ─────

#[test]
fn compile_rejects_the_leak_with_the_class() {
    let src = r#"
pattern Leak(input: String) -> String {
  let _ = print(env("SOME_KEY"))
  return "x"
}
flow Main { input: String = "x" -> Leak -> output }
"#;
    let declarations = parser::parse(src).expect("parse");
    let result = metalogos::semantic::check_program(&declarations);
    assert!(!result.is_ok(), "the leak must fail the compile-time check");
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.message.contains("[SECRET_LEAK]")),
        "the error must carry the [SECRET_LEAK] class: {:?}",
        result.errors
    );
}

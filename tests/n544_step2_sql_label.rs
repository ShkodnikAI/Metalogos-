// ── tests/n544_step2_sql_label.rs ───────────────────────────────────
// №544 (gh#882) step 2 — the SQL_DYNAMIC class on Labeled(Box<Type>, Label).
//
// The SQL lane speaks the stage-0 label vocabulary: a string literal IS
// the SQL template (the unlabeled String); every derived string carries
// the derivation marker Labeled(String, Internal) — Private when a
// secret part joined in (the shared vocabulary with the secret lane);
// the sink check asks the type question. The audit's OBSERVABLE
// behavior is unchanged (the audit.rs SQL suite + the corpus pin it);
// these tests pin the NEW mechanism surface — the type-layer functions,
// the two parity type rules (concatenation is derivation itself; a
// binding demotes the literal) — plus the end-to-end parity shapes the
// migration must never lose.

use std::collections::HashMap;

use metalogos::ast::{BinOp, Expr, Span};
use metalogos::builtins::sig_types::{Label, Type};
use metalogos::parser;
use metalogos::sql_label::{binding_sql_type, derived_sql_type, is_literal_sql, sql_type_of_expr};

fn strlit(v: &str) -> Expr {
    Expr::StringLit {
        value: v.to_string(),
        span: Span::unknown(),
    }
}

fn floatlit() -> Expr {
    Expr::FloatLit {
        value: 1.0,
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

fn binop(left: Expr, right: Expr) -> Expr {
    Expr::BinaryOp {
        op: BinOp::Add,
        left: Box::new(left),
        right: Box::new(right),
        span: Span::unknown(),
    }
}

fn ident(n: &str) -> Expr {
    Expr::Ident {
        name: n.to_string(),
        span: Span::unknown(),
    }
}

// ── the canonical types of the lane ─────────────────────────────────

#[test]
fn literal_and_derived_types_are_distinct() {
    // The pass answer is EXACTLY the unlabeled literal string.
    assert!(is_literal_sql(&Some(Type::String)));
    // The derivation marker carries the Internal label.
    assert_eq!(
        derived_sql_type(),
        Type::Labeled(Box::new(Type::String), Label::Internal)
    );
    // Every labeled/unknown shape refuses.
    assert!(!is_literal_sql(&Some(derived_sql_type())));
    assert!(!is_literal_sql(&Some(Type::Labeled(
        Box::new(Type::String),
        Label::Private
    ))));
    assert!(!is_literal_sql(&Some(Type::Float)));
    assert!(!is_literal_sql(&None));
}

// ── rule 1: concatenation is derivation itself ──────────────────────

#[test]
fn concatenation_never_yields_the_literal_type() {
    let vars: HashMap<String, Type> = HashMap::new();
    // "a" + "b" — BOTH parts literal, the composite is still derived:
    // the old syntactic verdict flagged every non-StringLit shape, and
    // the join algebra encodes exactly that.
    let concat = binop(strlit("a"), strlit("b"));
    assert_eq!(sql_type_of_expr(&concat, &vars), Some(derived_sql_type()));
    assert!(!is_literal_sql(&sql_type_of_expr(&concat, &vars)));
    // Concat with a secret part raises the marker to Private (the conf
    // join — the shared vocabulary with the secret lane).
    let with_secret = binop(strlit("k="), call("env", vec![strlit("K")]));
    assert_eq!(
        sql_type_of_expr(&with_secret, &vars),
        Some(Type::Labeled(Box::new(Type::String), Label::Private))
    );
}

// ── rule 2: a binding demotes the literal ───────────────────────────

#[test]
fn binding_demotes_the_literal_type() {
    let vars: HashMap<String, Type> = HashMap::new();
    // let sql = "SELECT 1" — the binding NEVER carries the literal type:
    // a variable reference is never a literal (the old verdict flagged
    // query(sql) for the plain variable too).
    let bound = binding_sql_type(&strlit("SELECT 1"), &vars);
    assert_eq!(bound, Some(derived_sql_type()));
    assert!(!is_literal_sql(&bound));
    // A secret binding keeps the Private label through the demotion.
    let secret_bound = binding_sql_type(&call("env", vec![strlit("K")]), &vars);
    assert_eq!(
        secret_bound,
        Some(Type::Labeled(Box::new(Type::String), Label::Private))
    );
    // An unknown initializer proves nothing (the honesty rule).
    assert_eq!(binding_sql_type(&floatlit(), &vars), Some(Type::Float));
}

#[test]
fn variable_references_stay_derived_through_the_env() {
    let mut vars: HashMap<String, Type> = HashMap::new();
    // Bind sql to a literal initializer (the demotion applies)...
    match binding_sql_type(&strlit("SELECT 1"), &vars) {
        Some(ty) => {
            vars.insert("sql".to_string(), ty);
        }
        None => panic!("a literal initializer produces a known type"),
    }
    // ...and the variable reference refuses the sink.
    assert_eq!(
        sql_type_of_expr(&ident("sql"), &vars),
        Some(derived_sql_type())
    );
    // An unbound variable is honestly unknown — and refuses all the same.
    assert_eq!(sql_type_of_expr(&ident("nope"), &vars), None);
    assert!(!is_literal_sql(&sql_type_of_expr(&ident("nope"), &vars)));
}

#[test]
fn calls_are_derived_and_secrets_stay_private() {
    let vars: HashMap<String, Type> = HashMap::new();
    // build_sql() — a call result is derived.
    assert_eq!(
        sql_type_of_expr(&call("build_sql", vec![]), &vars),
        Some(derived_sql_type())
    );
    // env()/secret() keep their canonical Private label (the shared
    // vocabulary with the secret lane) — the verdict is the same
    // refusal, the label is the honest reason.
    assert_eq!(
        sql_type_of_expr(&call("env", vec![strlit("K")]), &vars),
        Some(Type::Labeled(Box::new(Type::String), Label::Private))
    );
    // The secret's label flows through a call argument: upper(env("K")).
    assert_eq!(
        sql_type_of_expr(&call("upper", vec![call("env", vec![strlit("K")])]), &vars),
        Some(Type::Labeled(Box::new(Type::String), Label::Private))
    );
}

// ── end-to-end: the audit findings are unchanged in shape ───────────

fn findings_of(source: &str) -> Vec<(String, String)> {
    use metalogos::audit::Severity;
    let result = metalogos::audit_program(source).expect("audit");
    result
        .findings
        .iter()
        .map(|f| {
            let sev = match f.severity {
                Severity::Error => "Error",
                Severity::Warning => "Warning",
                Severity::Info => "Info",
            };
            (f.check_id.to_string(), sev.to_string())
        })
        .collect()
}

#[test]
fn audit_still_allows_the_literal_template() {
    let src = r#"
pattern GetUsers() -> String {
    let result = query("SELECT * FROM users")
    return result
}
"#;
    let result = metalogos::audit_program(src).expect("audit");
    assert!(result
        .findings
        .iter()
        .any(|f| f.check_id == "SQL_DYNAMIC" && f.severity == metalogos::audit::Severity::Info));
    assert!(!result
        .findings
        .iter()
        .any(|f| f.check_id == "SQL_DYNAMIC" && f.severity == metalogos::audit::Severity::Error));
}

#[test]
fn audit_still_flags_the_variable_and_the_concat() {
    // The variable shape: query(sql) — refused by the demotion rule.
    let variable = r#"
pattern GetUsers(table: String) -> String {
    let sql = "SELECT * FROM users"
    let result = query(sql)
    return result
}
"#;
    assert!(findings_of(variable)
        .iter()
        .any(|(id, sev)| id == "SQL_DYNAMIC" && sev == "Error"));

    // The concat shape: "SELECT * FROM " + table — refused by rule 1.
    let concat = r#"
pattern GetUsers(table: String) -> String {
    let sql = "SELECT * FROM " + table
    let result = query(sql)
    return result
}
"#;
    assert!(findings_of(concat)
        .iter()
        .any(|(id, sev)| id == "SQL_DYNAMIC" && sev == "Error"));
}

#[test]
fn audit_still_flags_db_execute_and_inline_dynamic() {
    // db_execute has no safe non-literal path either.
    let dbx = r#"
pattern Cleanup() -> String {
    let t = db_execute("DELETE FROM " + "staging.events")
    return "ok"
}
"#;
    assert!(findings_of(dbx)
        .iter()
        .any(|(id, sev)| id == "SQL_DYNAMIC" && sev == "Error"));

    // The inline dynamic shape (no binding at all).
    let inline = r#"
pattern GetUsers(table: String) -> String {
    let result = query("SELECT * FROM " + table)
    return result
}
"#;
    assert!(findings_of(inline)
        .iter()
        .any(|(id, sev)| id == "SQL_DYNAMIC" && sev == "Error"));
}

// ── the compile-time guarantee (№98): dynamic SQL fails mlog check ──

#[test]
fn compile_rejects_dynamic_sql_with_the_class() {
    let src = r#"
pattern GetUsers(table: String) -> String {
    let result = query("SELECT * FROM " + table)
    return result
}
flow Main { input: String = "x" -> GetUsers -> output }
"#;
    let declarations = parser::parse(src).expect("parse");
    let result = metalogos::semantic::check_program(&declarations);
    assert!(
        !result.is_ok(),
        "dynamic SQL must fail the compile-time check"
    );
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.message.contains("[SQL_DYNAMIC]")),
        "the error must carry the [SQL_DYNAMIC] class: {:?}",
        result.errors
    );
}

#[test]
fn compile_allows_the_literal_template() {
    let src = r#"
pattern GetUsers() -> String {
    let result = query("SELECT * FROM users")
    return result
}
flow Main { input: String = "x" -> GetUsers -> output }
"#;
    let declarations = parser::parse(src).expect("parse");
    let result = metalogos::semantic::check_program(&declarations);
    assert!(
        result.is_ok(),
        "the literal template must compile: {:?}",
        result.errors
    );
}

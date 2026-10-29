// ── tests/naryad_835_template_lexical_state.rs ──────────────────────
// №835 (wave 70, P1, [correctness][parser]) — gh#1633: the template
// pre-pass (`preprocess_templates`, src/parser/helpers.rs) scans the
// RAW source for the word `template` BEFORE pest runs; the №86
// identifier-boundary guard checks only the NEIGHBORING characters —
// a STANDALONE `template` inside a line comment (or a string literal)
// passes the guard and starts a phantom template-body extraction:
//
//   1. `result[abs_start..].find('{')` — the next `{` anywhere AFTER
//      the comment (typically the FOLLOWING pattern's opening brace);
//   2. the balanced-brace scan replaces that `{…}` block with a
//      `{__TEMPLATE_BODY_N__}` placeholder — the braces STAY, so the
//      husk still parses as a (wrong) single-statement body;
//   3. every body-walking gate (SQL_DYNAMIC, SECRET_LEAK,
//      HTML_INJECTION, TAINT_*) sees no calls — the unit compiles
//      clean with the audit blind.
//
// RED-before evidence (gh#1633, found during №831 on the
// naryad-831-sql-gate branch @ 1de285f; re-verified on main b2d978d5,
// rustc 1.99.1): 5 of the 8 contracts here fail on main — the comment
// repro loses BOTH FnCalls (the husk carries none), the SQL gate goes
// silent on the swallowed bodies; the multiline-string contract passed
// pre-fix only through the weak body-shape witness and was sharpened.
//
// The fix contract: the pre-pass keyword search and the opening-brace
// search walk the SAME lexical state pest uses — the `//`-to-EOL
// comments, the double-quoted string literals with escapes, and the
// triple-quoted multiline form never host the `template` keyword.
// The audit witness uses `query(...)` — the №544 carrier that exists
// on main (the №831 branch widens the carrier set to query_row and
// friends; the structural FnCall assertion is carrier-independent and
// mirrors the gh#1633 repro verbatim).

use metalogos::ast::{Declaration, Expr, Statement};
use metalogos::audit::{audit_category_a, Severity};
use metalogos::parser;

/// Every FnCall/QualifiedCall name inside the NAMED pattern's body
/// (recursive — the structural witness that the body survived the
/// pre-pass).
fn pattern_fn_calls(source: &str, pattern: &str) -> Vec<String> {
    let declarations = parser::parse(source).expect("parse must succeed");
    let mut calls = Vec::new();
    fn walk(e: &Expr, calls: &mut Vec<String>) {
        match e {
            Expr::FnCall { name, args, .. } => {
                calls.push(name.clone());
                for a in args {
                    walk(a, calls);
                }
            }
            Expr::QualifiedCall { function, args, .. } => {
                calls.push(function.clone());
                for a in args {
                    walk(a, calls);
                }
            }
            Expr::FieldAccess { object, .. } => walk(object, calls),
            Expr::BinaryOp { left, right, .. } => {
                walk(left, calls);
                walk(right, calls);
            }
            Expr::IfElse {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                walk(condition, calls);
                walk(then_branch, calls);
                walk(else_branch, calls);
            }
            Expr::List { items, .. } => {
                for i in items {
                    walk(i, calls);
                }
            }
            Expr::IndexAccess { object, index, .. } => {
                walk(object, calls);
                walk(index, calls);
            }
            Expr::Try { expr, .. } => walk(expr, calls),
            _ => {}
        }
    }
    for d in &declarations {
        if let Declaration::Pattern(pd) = d {
            if pd.name != pattern {
                continue;
            }
            for st in &pd.body {
                match st {
                    Statement::LetBinding { value, .. } => walk(value, &mut calls),
                    Statement::Return { value, .. } => walk(value, &mut calls),
                    _ => {}
                }
            }
        }
    }
    calls
}

/// The SQL_DYNAMIC error findings for one program (the audit witness).
fn sql_dynamic_errors(source: &str) -> Vec<String> {
    let declarations = parser::parse(source).expect("parse must succeed");
    audit_category_a(&declarations, "")
        .into_iter()
        .filter(|f| f.check_id == "SQL_DYNAMIC" && f.severity == Severity::Error)
        .map(|f| f.message)
        .collect()
}

// ── the gh#1633 minimal repro (the comment form) ────────────────────

#[test]
fn comment_template_word_does_not_swallow_the_next_pattern() {
    // The verbatim gh#1633 repro: a standalone `template` in the line
    // comment ABOVE the pattern. Pre-fix: the pre-pass mistakes the
    // comment word for a declaration, the pattern's braces become a
    // placeholder husk, the body arrives without its calls — 0
    // FnCalls. Post-fix: the body survives — 2 FnCalls (query_row,
    // query_param).
    let source = "// some template here\npattern P() -> String {\n  let _ = query_row(\"S\" + query_param(\"n\"))\n  return \"done\"\n}\n";
    let calls = pattern_fn_calls(source, "P");
    assert!(
        calls.contains(&"query_row".to_string()) && calls.contains(&"query_param".to_string()),
        "the pattern body must carry BOTH FnCalls (query_row, query_param), got: {:?}",
        calls
    );
}

#[test]
fn comment_template_word_blinding_is_loud_under_the_sql_gate() {
    // The same class, witnessed through the audit: `query` is the №544
    // carrier on main — with the body swallowed the gate sees no calls
    // (the false-negative parse), with the body intact the SQL_DYNAMIC
    // error fires.
    let source = "// some template here\npattern P() -> String {\n  let _ = query(\"SELECT * FROM u WHERE n='\" + query_param(\"n\") + \"'\")\n  return \"done\"\n}\n";
    let calls = pattern_fn_calls(source, "P");
    assert!(
        calls.contains(&"query".to_string()),
        "the pattern body must survive the pre-pass, got calls: {:?}",
        calls
    );
    let errs = sql_dynamic_errors(source);
    assert!(
        errs.iter().any(|m| m.contains("query")),
        "the swallowed-body parse must stay loud under SQL_DYNAMIC, got: {:?}",
        errs
    );
}

// ── the comment inside a PRECEDING pattern's body ───────────────────

#[test]
fn comment_inside_earlier_pattern_does_not_swallow_the_later_one() {
    // The comment sits INSIDE pattern A's body — the next brace in the
    // raw scan is pattern B's opening brace: B's body is swallowed and
    // B's SQL carrier goes blind while A parses clean.
    let source = "pattern A() -> String {\n  // a template here\n  return \"x\"\n}\npattern B() -> String {\n  let _ = query(\"SELECT * FROM u WHERE n='\" + query_param(\"n\") + \"'\")\n  return \"done\"\n}\n";
    let calls = pattern_fn_calls(source, "B");
    assert!(
        calls.contains(&"query".to_string()),
        "pattern B's body must survive the pre-pass (A's comment must not start a phantom extraction), got: {:?}",
        calls
    );
    let errs = sql_dynamic_errors(source);
    assert!(
        errs.iter().any(|m| m.contains("query")),
        "B's dynamic SQL must stay loud, got: {:?}",
        errs
    );
}

// ── the string-literal form ─────────────────────────────────────────

#[test]
fn template_word_in_string_literal_does_not_swallow_the_next_pattern() {
    // The word inside a double-quoted literal: the extractor searches
    // FORWARD for the next brace, so the first brace block after the
    // string (the following pattern's body) gets corrupted.
    let source = "pattern Q() -> String {\n  let m = \"my template here\"\n  return m\n}\npattern P() -> String {\n  let _ = query(\"SELECT * FROM u WHERE n='\" + query_param(\"n\") + \"'\")\n  return \"done\"\n}\n";
    let calls = pattern_fn_calls(source, "P");
    assert!(
        calls.contains(&"query".to_string()),
        "pattern P's body must survive the pre-pass (the string's `template` word must not start a phantom extraction), got: {:?}",
        calls
    );
    let errs = sql_dynamic_errors(source);
    assert!(
        errs.iter().any(|m| m.contains("query")),
        "P's dynamic SQL must stay loud, got: {:?}",
        errs
    );
}

#[test]
fn template_word_in_multiline_string_does_not_swallow_the_next_pattern() {
    // The MULTILINE_STRING form (triple-quoted, no escapes in the
    // grammar) — the walker consumes it to the closing triple-quote.
    let source = "pattern Q() -> String {\n  let doc = \"\"\"a template\nspans lines\"\"\"\n  return doc\n}\npattern P() -> String {\n  let _ = query(\"SELECT * FROM u WHERE n='\" + query_param(\"n\") + \"'\")\n  return \"done\"\n}\n";
    let calls = pattern_fn_calls(source, "P");
    assert!(
        calls.contains(&"query".to_string()),
        "pattern P's body must survive the pre-pass (the multiline string's `template` word must not start a phantom extraction), got: {:?}",
        calls
    );
    let errs = sql_dynamic_errors(source);
    assert!(
        errs.iter().any(|m| m.contains("query")),
        "P's dynamic SQL must stay loud, got: {:?}",
        errs
    );
}

// ── the escaped-quote form (the escape must not end the string) ─────

#[test]
fn template_word_after_escaped_quote_does_not_swallow_the_next_pattern() {
    // `\"` inside the literal — the string state must survive the
    // escape, otherwise the walker thinks the string ended and the
    // rest of the line becomes (phantom) code.
    let source = "pattern Q() -> String {\n  let m = \"say \\\"template\\\" loud\"\n  return m\n}\npattern P() -> String {\n  let _ = query(\"SELECT * FROM u WHERE n='\" + query_param(\"n\") + \"'\")\n  return \"done\"\n}\n";
    let calls = pattern_fn_calls(source, "P");
    assert!(
        calls.contains(&"query".to_string()),
        "pattern P's body must survive the pre-pass (the escaped-quote string must not desync the walker), got: {:?}",
        calls
    );
    let errs = sql_dynamic_errors(source);
    assert!(
        errs.iter().any(|m| m.contains("query")),
        "P's dynamic SQL must stay loud, got: {:?}",
        errs
    );
}

// ── the brace-find must skip the lexemes too ────────────────────────

#[test]
fn brace_inside_comment_between_keyword_and_body_is_skipped() {
    // A comment between the template header and the body brace carries
    // its own brace — the grammar's WHITESPACE includes COMMENT, so
    // the declaration is legal; the brace-find must not mistake the
    // comment's brace for the body opening.
    let source = "template T() -> Html // the body { starts below\n{ <b>x</b> }\npattern P() -> String {\n  let _ = query(\"SELECT * FROM u WHERE n='\" + query_param(\"n\") + \"'\")\n  return \"done\"\n}\n";
    let declarations = parser::parse(source).expect("parse must succeed");
    let mut template_body = String::new();
    for d in &declarations {
        if let Declaration::Template(t) = d {
            template_body = t.body.clone();
        }
    }
    assert!(
        template_body.contains("<b>x</b>"),
        "the template body must be extracted from the REAL brace (the comment's brace must not start the extraction), got: {:?}",
        template_body
    );
    let calls = pattern_fn_calls(source, "P");
    assert!(
        calls.contains(&"query".to_string()),
        "the following pattern must survive untouched, got: {:?}",
        calls
    );
}

// ── the regressions (the honest pre-pass stays working) ─────────────

#[test]
fn genuine_template_declaration_still_extracts_and_restores() {
    // A real template + a pattern after it: the body extraction and the
    // placeholder restore must behave exactly as before the rewrite.
    let source = "template Card() -> Html { <div class=\"card\">{{ title }}</div> }\npattern P() -> String {\n  let _ = query(\"SELECT * FROM u WHERE n='\" + query_param(\"n\") + \"'\")\n  return \"done\"\n}\n";
    let declarations = parser::parse(source).expect("parse must succeed");
    let mut template_body = String::new();
    for d in &declarations {
        if let Declaration::Template(t) = d {
            template_body = t.body.clone();
        }
    }
    assert!(
        template_body.contains("<div"),
        "the template body must be restored from the placeholder, got: {:?}",
        template_body
    );
    let calls = pattern_fn_calls(source, "P");
    assert!(
        calls.contains(&"query".to_string()),
        "the pattern after a real template must keep its body"
    );
    let errs = sql_dynamic_errors(source);
    assert!(
        errs.iter().any(|m| m.contains("query")),
        "the SQL gate must still fire on the pattern after a real template, got: {:?}",
        errs
    );
}

#[test]
fn template_render_identifier_regression_still_holds() {
    // №86: `template_render(...)` — the identifier case. The
    // neighboring-character guard stays; the call's brace arguments
    // must not be extracted.
    let source = "pattern P() -> String {\n  let h = template_render(\"card\", {title: t})\n  let _ = query(\"SELECT * FROM u WHERE n='\" + query_param(\"n\") + \"'\")\n  return h\n}\n";
    let calls = pattern_fn_calls(source, "P");
    assert!(
        calls.contains(&"template_render".to_string()),
        "the template_render call must survive, got: {:?}",
        calls
    );
    let errs = sql_dynamic_errors(source);
    assert!(
        errs.iter().any(|m| m.contains("query")),
        "the SQL gate must still fire, got: {:?}",
        errs
    );
}

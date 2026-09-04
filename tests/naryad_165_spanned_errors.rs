// ── Наряд №165: SpannedError carries real AST positions ───────────────
//
// Contract (from the naryad):
//   A .mlog file with a semantic error on a known line → the resulting
//   `AnalysisResult.errors[0].span.start_line` MUST equal that line
//   (1-indexed). The LSP layer then converts this span to a 0-indexed
//   LSP `Position` — that conversion is verified separately in the
//   mlog-lsp tests.
//
// Block 4 of the naryad requires multiple, distinct error types to be
// checked (not just one) — this proves the span is propagated through
// every error-generation path, not just one or two.
//
// All test sources are written with explicit `\n` line tracking so the
// expected line number is unambiguous and stable across edits. Lines
// are 1-indexed in pest spans (see `src/ast.rs`).

use metalogos::parser::parse;
use metalogos::semantic::{check_program, SpannedError};

/// Run `check_program` on `source` and return the first error's span.
///
/// Panics with a helpful message if no error is produced — the test
/// source was chosen specifically to trigger an error, so an empty
/// error list means the source is no longer exercising the path the
/// naryad intended to verify.
fn first_error(source: &str) -> SpannedError {
    let decls = parse(source).expect("source must parse");
    let result = check_program(&decls);
    assert!(
        !result.errors.is_empty(),
        "test source should produce at least one error, got none. source:\n{}",
        source
    );
    result.errors[0].clone()
}

// ── 1. Duplicate pattern name → decl-level error → span points at the
//    SECOND declaration (the duplicate), not the first.
//
// Source layout (1-indexed):
//   1: pattern P(x: String) -> String { return x }
//   2: pattern P(y: String) -> String { return y }     ← duplicate
//
// The span should point at line 2 (the offending duplicate), not line 1.

#[test]
fn span_for_duplicate_pattern_points_at_duplicate_line() {
    let src =
        "pattern P(x: String) -> String { return x }\npattern P(y: String) -> String { return y }";
    let err = first_error(src);
    assert!(
        err.message.contains("duplicate pattern"),
        "expected 'duplicate pattern' in message, got: {}",
        err.message
    );
    assert_eq!(
        err.span.start_line, 2,
        "duplicate-pattern error should point at the second (duplicate) declaration. \
         Span: {:?}",
        err.span
    );
}

// ── 2. Entity references unknown type → decl-level error.
//
// Source layout (1-indexed):
//   1: (blank leading line so the error is NOT on line 1 — verifies
//      the span is not always 1)
//   2: entity record referencing undeclared type
//
// EntityRecord (entity Foo { ... }) referencing an undeclared EntityType
// produces an ERROR (not a WARNING — the WARNING path is for entity
// simple / primitives). This test exercises the ERROR path.

#[test]
fn span_for_unknown_entity_type_points_at_decl_line() {
    let src = "\nentity bob: NonExistent = { name: \"test\" }";
    let err = first_error(src);
    assert!(
        err.message.contains("unknown type"),
        "expected 'unknown type' in message, got: {}",
        err.message
    );
    assert_eq!(
        err.span.start_line, 2,
        "entity-unknown-type error should point at the entity declaration line. \
         Span: {:?}",
        err.span
    );
}

// ── 3. Rule references undefined entity → decl-level error.
//
// Source layout (1-indexed):
//   1: (blank)
//   2: (blank)
//   3: rule targeting an entity that does not exist
//
// Rule syntax: `rule If(m.field contains "x") then m.field = v with priority=N`
// — requires a target entity 'm' which must exist. An undefined entity
// reference triggers the `rule target ... references undefined entity` error.

#[test]
fn span_for_rule_unknown_entity_points_at_rule_line() {
    let src = "\n\nrule If(m.text contains \"x\") then m.urgency = 0.9 with priority=5";
    let err = first_error(src);
    assert!(
        err.message.contains("undefined entity"),
        "expected 'undefined entity' in message, got: {}",
        err.message
    );
    assert_eq!(
        err.span.start_line, 3,
        "rule-undefined-entity error should point at the rule line. Span: {:?}",
        err.span
    );
}

// ── 4. mlogserver with unknown middleware → decl-level error.
//
// Source layout (1-indexed):
//   1: mlogserver {
//   2:   middleware: [bogus_middleware]
//   3:   route "/" method=GET { return "Hello" }
//   4: }
//
// The mlogserver block starts on line 1; its span points at line 1.

#[test]
fn span_for_unknown_middleware_points_at_mlogserver_block() {
    let src = "mlogserver {\n  middleware: [bogus_middleware]\n  route \"/\" method=GET { return \"Hello\" }\n}";
    let err = first_error(src);
    assert!(
        err.message.contains("unknown middleware"),
        "expected 'unknown middleware' in message, got: {}",
        err.message
    );
    assert_eq!(
        err.span.start_line, 1,
        "unknown-middleware error should point at the mlogserver block. Span: {:?}",
        err.span
    );
}

// ── 5. mlogserver with invalid HTTP method → decl-level error.
//
// Verifies a DIFFERENT mlogserver error path from #4 — proves span
// propagation is uniform across mlogserver sub-checks, not just the
// middleware check.

#[test]
fn span_for_invalid_http_method_points_at_mlogserver_block() {
    let src = "mlogserver {\n  route \"/\" method=INVALID { return \"Hello\" }\n}";
    let err = first_error(src);
    assert!(
        err.message.contains("unknown HTTP method"),
        "expected 'unknown HTTP method' in message, got: {}",
        err.message
    );
    assert_eq!(
        err.span.start_line, 1,
        "invalid-method error should point at the mlogserver block. Span: {:?}",
        err.span
    );
}

// ── 6. Template returning opaque type other than Html → decl-level error.
//
// Verifies yet another decl-level error path. If this span is missing
// while #4 and #5 work, it means the conversion was not uniform.

#[test]
fn span_for_template_wrong_return_type_points_at_template_decl() {
    let src = "\ntemplate Page(title: String) -> Secret {\n  <h1>{{ title }}</h1>\n}";
    let err = first_error(src);
    assert!(
        err.message.contains("only Html is supported"),
        "expected 'only Html is supported' in message, got: {}",
        err.message
    );
    assert_eq!(
        err.span.start_line, 2,
        "template-wrong-return-type error should point at the template declaration. \
         Span: {:?}",
        err.span
    );
}

// ── 7. Adapt with non-existent learnable pattern → decl-level error.
//
// Verifies the adapt/mutate/eval error path (distinct from rule).

#[test]
fn span_for_adapt_unknown_pattern_points_at_adapt_decl() {
    let src = "\n\n\nadapt NonExistent add_example(\"in\", \"out\")";
    let err = first_error(src);
    assert!(
        err.message.contains("not found"),
        "expected 'not found' in message, got: {}",
        err.message
    );
    assert_eq!(
        err.span.start_line, 4,
        "adapt-unknown-pattern error should point at the adapt declaration. Span: {:?}",
        err.span
    );
}

// ── 8. Audit Category A: SQL_DYNAMIC → finding.line path.
//
// `audit_category_a` returns `AuditFinding { line, .. }`. When called
// from `check_program`, `source` is "" (empty) — `find_line(source, ...)`
// then returns 1 (its default fallback). `SpannedError::at_line` builds
// a Span from that line value.
//
// This is a known limitation: audit findings carry only a line (no
// column), and the line is the source-text lookup result, not the AST
// node line. Improving this requires passing source text into
// `check_program` — out of scope for наряд №165. The contract here is
// that the span IS propagated (not silently dropped) — verifying
// that the LSP layer will get a non-`Span::unknown()` range for
// audit findings.

#[test]
fn span_for_sql_dynamic_audit_finding_is_propagated() {
    // SQL_DYNAMIC: query() with a non-literal (variable) SQL string.
    // Mirror the existing fixture at
    // tests/fixtures/n98_audit_invariant/sql_dynamic_rejected.mlog.
    let src = "pattern BadQuery(table: String) -> String {\n    let result = query(table)\n    return result\n}";
    let decls = parse(src).expect("source must parse");
    let result = check_program(&decls);
    let sql_err = result
        .errors
        .iter()
        .find(|e| e.message.contains("SQL_DYNAMIC"))
        .unwrap_or_else(|| {
            panic!(
                "expected SQL_DYNAMIC error, got: {:?}",
                result.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
            )
        });
    // Span is propagated — not Span::unknown() (start_line == 0).
    // The exact line value is `find_line("", ...) == 1` (the default
    // fallback when source text is not passed to check_program). This
    // is the same behavior as before наряд №165 — audit findings got
    // `Range::new(0..0, 0..100)` (line 0) in the LSP. With наряд №165,
    // `at_line(line)` with line=1 produces Span { start_line: 1, ... }
    // → LSP line 0 (after `saturating_sub(1)`). So the LSP output is
    // byte-for-byte identical to the previous hardcoded fallback.
    assert!(
        sql_err.span.start_line >= 1,
        "SQL_DYNAMIC audit finding should have a propagated span (start_line >= 1, \
         not Span::unknown() with start_line == 0). Span: {:?}",
        sql_err.span
    );
    // Sanity: the message still identifies the check_id.
    assert!(
        sql_err.message.contains("SQL_DYNAMIC"),
        "message should contain 'SQL_DYNAMIC', got: {}",
        sql_err.message
    );
}

// ── 9. Warning (not error) also carries a span.
//
// The naryad explicitly says "errors and warnings". Verify a warning
// path too — `mlogserver` without `security_headers` middleware emits
// a warning, and that warning's span should point at the mlogserver
// block (not be missing / zero).

#[test]
fn span_for_security_headers_warning_points_at_mlogserver_block() {
    let src = "mlogserver {\n  route \"/\" method=GET { return \"Hello\" }\n}";
    let decls = parse(src).expect("parse");
    let result = check_program(&decls);
    assert!(
        result.errors.is_empty(),
        "expected no errors, got: {:?}",
        result.errors
    );
    let warn = result
        .warnings
        .iter()
        .find(|w| w.message.contains("security_headers"))
        .unwrap_or_else(|| {
            panic!(
                "expected security_headers warning, got: {:?}",
                result.warnings
            )
        });
    assert_eq!(
        warn.span.start_line, 1,
        "security_headers warning should point at the mlogserver block. Span: {:?}",
        warn.span
    );
}

// ── 10. SpannedError API: .message and .span are accessible.
//
// Smoke test that the new struct fields are public and the type is
// exported from `metalogos::semantic`. If this test fails to compile,
// downstream LSP / programmatic consumers will also fail — this is
// the canary.

#[test]
fn spanned_error_type_is_public_and_has_message_and_span() {
    let src =
        "pattern P(x: String) -> String { return x }\npattern P(y: String) -> String { return y }";
    let decls = parse(src).unwrap();
    let result = check_program(&decls);
    let err: &SpannedError = result
        .errors
        .first()
        .expect("should have at least one error");
    // Touch both fields — proves they're public.
    let _: &String = &err.message;
    let _: &metalogos::ast::Span = &err.span;
}

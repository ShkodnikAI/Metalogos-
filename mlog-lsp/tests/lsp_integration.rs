// ── LSP integration tests ─────────────────────────────────────────────
//
// Contract (Наряд Phase 3.3):
//   Send LSP server textDocument/didOpen with an erroneous program,
//   verify that a diagnostic is returned.
//
// Since tower-lsp doesn't provide a mock transport, we test the
// core code path that didOpen invokes: Backend::parse_and_analyze().
// This exercises the exact same parse → semantic → diagnostic pipeline
// that the LSP server uses.

use metalogos::ast::Span;
use mlog_lsp::{span_to_range, Backend};
use tower_lsp::lsp_types::*;

/// Contract: erroneous program → diagnostic returned.
#[test]
fn lsp_did_open_erroneous_program_returns_diagnostic() {
    // This is the erroneous program from the contract
    let source = r#"entity m: UnknownType = { text: "hi" }"#;

    // Simulate what textDocument/didOpen does:
    let (declarations, diagnostics) = Backend::parse_and_analyze(source);

    // 1. Parse succeeds (declarations are returned)
    assert!(!declarations.is_empty(), "parse should succeed");

    // 2. Semantic analysis finds errors → diagnostics are produced
    assert!(!diagnostics.is_empty(), "diagnostics should be returned");

    // 3. The diagnostic is an Error
    assert_eq!(
        diagnostics[0].severity,
        Some(DiagnosticSeverity::ERROR),
        "diagnostic should be ERROR severity"
    );

    // 4. The message mentions the unknown type
    assert!(
        diagnostics[0].message.contains("unknown type"),
        "diagnostic message should mention 'unknown type': {}",
        diagnostics[0].message
    );

    // 5. The diagnostic has a valid range (non-zero span)
    assert!(
        diagnostics[0].range != Range::default(),
        "diagnostic should have a non-zero range"
    );

    // 6. Source is tagged as "mlog"
    assert_eq!(
        diagnostics[0].source.as_deref(),
        Some("mlog"),
        "diagnostic source should be 'mlog'"
    );
}

/// Contract: clean program → no diagnostics.
#[test]
fn lsp_did_open_clean_program_no_diagnostics() {
    let source = r#"
        entity greeting: String = "Hello, Metalogos!"
        pattern SayHello(text: String) -> String { return text }
        flow Main { input: String = greeting -> SayHello -> output }
    "#;

    let (declarations, diagnostics) = Backend::parse_and_analyze(source);

    assert_eq!(declarations.len(), 3, "should parse 3 declarations");
    assert!(
        diagnostics.is_empty(),
        "clean program should produce no diagnostics"
    );
}

/// Contract: duplicate pattern → error diagnostic.
#[test]
fn lsp_did_open_duplicate_pattern_returns_error() {
    let source = r#"
        pattern Foo(x: String) -> String { return x }
        pattern Foo(y: String) -> String { return y }
    "#;

    let (_declarations, diagnostics) = Backend::parse_and_analyze(source);

    assert!(!diagnostics.is_empty());
    assert!(diagnostics
        .iter()
        .any(|d| d.message.contains("duplicate pattern")));
}

/// Contract: adapt target not found → error diagnostic.
#[test]
fn lsp_did_open_adapt_not_found_returns_error() {
    let source = r#"adapt NonExistent add_example("in", "out")"#;

    let (_declarations, diagnostics) = Backend::parse_and_analyze(source);

    assert!(!diagnostics.is_empty());
    assert!(diagnostics[0].message.contains("not found"));
}

/// Contract: parse error → error diagnostic (not semantic).
#[test]
fn lsp_did_open_parse_error_returns_diagnostic() {
    let source = "entity { this is broken syntax";

    let (declarations, diagnostics) = Backend::parse_and_analyze(source);

    // No declarations parsed
    assert!(declarations.is_empty());

    // But a diagnostic is produced
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("parse error"));
    assert_eq!(diagnostics[0].severity, Some(DiagnosticSeverity::ERROR));
}

/// Contract: go-to-definition finds entity declaration.
#[test]
fn lsp_goto_definition_finds_entity() {
    let source = r#"entity greeting: String = "Hello"
pattern SayHello(text: String) -> String { return text }
flow Main { input: String = greeting -> SayHello -> output }
"#;
    let (declarations, _) = Backend::parse_and_analyze(source);
    let symbols = Backend::build_symbols_with_text(&declarations, source);

    // "greeting" should be found as a symbol with its span
    let greeting = symbols.iter().find(|s| s.name == "greeting");
    assert!(greeting.is_some());
    let greeting = greeting.unwrap();
    let range = span_to_range(&greeting.span);
    // Entity declaration should have a valid range (non-zero span)
    // Text search should find "entity greeting" on line 0
    assert!(
        range.start.line <= 3,
        "entity should be in first few lines: line={}",
        range.start.line
    );
    assert!(
        range.start.character < 20,
        "entity name starts within first 20 cols: col={}",
        range.start.character
    );
    // The span should NOT be unknown (0,0,0,0)
    assert!(
        greeting.span != Span::unknown(),
        "greeting span should be resolved, not unknown"
    );
}

/// Contract: hover returns type info for pattern.
#[test]
fn lsp_hover_returns_type_info() {
    let source = r#"
        entity greeting: String = "Hello"
        pattern SayHello(text: String) -> String { return text }
        flow Main { input: String = greeting -> SayHello -> output }
    "#;
    let (declarations, _) = Backend::parse_and_analyze(source);

    // Find SayHello pattern
    let symbols = Backend::build_symbols(&declarations);
    let say_hello = symbols.iter().find(|s| s.name == "SayHello").unwrap();
    let decl = &declarations[say_hello.decl_index];

    // Verify type_info includes signature
    let info = decl.type_info();
    assert!(info.contains("pattern SayHello"));
    assert!(info.contains("String"));
}

/// Contract: multiple errors → multiple diagnostics.
#[test]
fn lsp_did_open_multiple_errors_multiple_diagnostics() {
    let source = r#"
        entity m: UnknownType = { text: "hi" }
        adapt NonExistent add_example("in", "out")
    "#;

    let (_declarations, diagnostics) = Backend::parse_and_analyze(source);

    assert!(diagnostics.len() >= 2, "should have at least 2 diagnostics");
}

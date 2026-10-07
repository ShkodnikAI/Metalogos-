// ── Naryad №634 (P2, compiler): the stray-keyword Ident artifact ────
//
// The №617 finding (ledger gh#967 §5 / the PR comment 6022497489): the
// expression parser's primary fallback (src/parser/expr.rs) lifts ANY
// unexpected token into a bare `Expr::Ident` — including the reserved
// keywords. The live artifact shape (bisected to self-host/parser.mlog:
// the single-line if-then-block `if c == 1.0 then { p = p + 1.0 }` —
// the statement-level IfThen of Phase 7.7) leaves `if`/`then` tokens
// behind, and the fallback silently accepts them as Idents.
//
// THE DECISION (№634, documented acceptance): the tolerance STAYS —
// parser.mlog depends on it (0 real corpus hits; the grammar of
// self-host does not change without a cause), and the semantic
// checker's variable walk skips exactly these names (the №617
// keyword-ident exemption, `is_reserved_keyword`), so the artifact can
// never mask a REAL undefined-variable defect. This test pins BOTH
// sides of that decision: the artifact exists (an Ident("then") node
// survives the parse) AND the semantic checker stays green on the
// artifact shape (the exemption holds). If the parser one day refuses
// stray keywords loudly, this test must flip red — consciously.

const WHILE_IF_SNIPPET: &str = r#"
pattern P(c: Float) -> Float {
  let mut p = 0.0
  if c == 1.0 then p = p + 1.0
  return p
}
"#;

#[test]
fn stray_keyword_parses_as_bare_ident_and_checker_stays_green() {
    // (1) The parse SUCCEEDS on the tolerated legacy shape (no loud
    // refusal — the documented acceptance decision).
    let decls = metalogos::parser::parse(WHILE_IF_SNIPPET)
        .unwrap_or_else(|e| panic!("the tolerated while-if shape must parse: {}", e));

    // (2) The artifact exists: `if`/`then` survive the parse as bare
    // Expr::Ident nodes (the fallback's documented behavior) — pinned
    // through the AST Debug dump (the walker stays variant-complete by
    // construction: the dump cannot miss a slot).
    let dump = format!("{:?}", decls);
    let hits: Vec<&str> = ["if", "then"]
        .iter()
        .copied()
        .filter(|k| dump.contains(&format!("name: \"{}\"", k)))
        .collect();
    assert!(
        hits.contains(&"if") && hits.contains(&"then"),
        "the stray-keyword artifact is gone from the if-then-block shape — \
         either the parser refuses stray keywords now (flip this test \
         consciously, the №617 checker exemption narrows with it) or the \
         shape changed; hits: {:?}",
        hits
    );

    // (3) The checker side of the decision: the semantic check on the
    // SAME source reports no undefined-variable error for the artifact
    // (the №617 exemption) — the program is check-clean.
    let checked = metalogos::semantic::check_program(&decls);
    let undefined: Vec<_> = checked
        .errors
        .iter()
        .filter(|f| f.message.contains("undefined variable"))
        .collect();
    assert!(
        undefined.is_empty(),
        "the №617 keyword-ident exemption leaked: {:?}",
        undefined
    );
}

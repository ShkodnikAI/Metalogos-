// ── №958 (issue #1925, the audit 12.11 — P1): the AST-walk depth guard ─
//
// The parser pre-scan bounds every SOURCE; the DepthGuard (the semantic
// walkers' second line) bounds the WALK when a form the scanner does not
// model grows deep (a future grammar change, a new carrying expression
// form). This test builds the adversarial AST DIRECTLY (bypassing the
// parser) — a 50 000-deep left `+` chain — and requires the clean
// `[INPUT_TOO_DEEP]` refusal from `semantic::check_program` instead of
// the stack overflow.
//
// The mutation (scripts/mutation_verify_958.sh M2) neuters the guard →
// the walk recurses 50 000 frames → the overflow abort → this test red.
//
// The chain is built iteratively (no recursion while building) and the
// whole declaration is LEAKED deliberately (recursive Drop of a 50k Box
// chain would overflow on its own — a test-scoped leak is the honest way
// to skip the drop).
#![allow(clippy::disallowed_methods)]

use metalogos::ast::{BinOp, Declaration, Expr, PatternDecl, Span, Statement};

/// Build a `b+a+a+...` left chain of `depth` BinaryOp nodes; the chain is
/// moved up iteratively (no clone, no recursion) and ends up leaked
/// inside the leaked declaration — nothing recursive ever drops.
fn build_deep_declaration(depth: usize) -> &'static Declaration {
    let mut current: Box<Expr> = Box::new(Expr::StringLit {
        value: "a".to_string(),
        span: Span::unknown(),
    });
    for _ in 0..depth {
        let left = Box::new(Expr::StringLit {
            value: "b".to_string(),
            span: Span::unknown(),
        });
        let right = current; // move the accumulated chain
        current = Box::new(Expr::BinaryOp {
            left,
            op: BinOp::Add,
            right,
            span: Span::unknown(),
        });
    }
    Box::leak(Box::new(Declaration::Pattern(PatternDecl {
        span: Span::unknown(),
        name: "Deep".to_string(),
        params: Vec::new(),
        return_type: "String".to_string(),
        effects: None,
        body: vec![Statement::ExprStmt {
            expr: *current,
            span: Span::unknown(),
        }],
    })))
}

#[test]
fn n958_walk_guard_refuses_the_deep_ast_cleanly() {
    // The 50k-deep chain, built flat; the declaration is leaked — the
    // drop glue (which would itself recurse 50k frames) never runs.
    let decl = build_deep_declaration(50_000);
    // The check runs on the MAIN thread (the default stack) — with the
    // guard the walk refuses at the input-depth cap (2048 by default);
    // without it the recursion overflows and the test process aborts
    // (the mutation state — the main stack dies at a few thousand debug
    // frames).
    let result = metalogos::semantic::check_program(std::slice::from_ref(decl));
    let rendered = format!("{:?}", result.errors);
    assert!(
        rendered.contains("INPUT_TOO_DEEP"),
        "the walk guard must refuse the deep AST with INPUT_TOO_DEEP, got: {}",
        rendered
    );
}

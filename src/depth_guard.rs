//! №958 (the audit 12.11, M-1 — gh#1925): the AST-walk depth guard.
//!
//! The parser pre-scan (`parser::scan_input_depth`) is the PRIMARY gate —
//! every source passes it, so the walkers see bounded ASTs. This guard is
//! the SECOND line: the recursive walkers (semantic; the LSP rides the
//! same parser) check the thread-local depth on entry and refuse with
//! `[INPUT_TOO_DEEP]` instead of overflowing the stack when a form the
//! scanner does not model grows deep (a future grammar change, a new
//! carrying expression form). The RAII counter keeps the bookkeeping
//! local to the recursion frame.

use std::cell::Cell;

thread_local! {
    static WALK_DEPTH: Cell<usize> = const { Cell::new(0) };
}

/// The RAII depth increment — one per recursive walk frame.
pub struct DepthGuard;

impl DepthGuard {
    pub fn enter() -> DepthGuard {
        WALK_DEPTH.with(|d| d.set(d.get() + 1));
        DepthGuard
    }

    /// True when the CURRENT frame sits deeper than the input-depth cap.
    pub fn exceeded() -> bool {
        WALK_DEPTH.with(|d| d.get() > crate::limits::max_input_depth())
    }
}

impl Drop for DepthGuard {
    fn drop(&mut self) {
        WALK_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
    }
}

/// The stable refusal text (the ONE form; the walkers wrap it into their
/// own error type).
pub const INPUT_TOO_DEEP_MSG: &str =
    "[INPUT_TOO_DEEP] the expression nesting exceeds the analysis depth limit (METALOGOS_MAX_INPUT_DEPTH, №958) — split the expression or raise the limit";

/// №958: the ENTRY guard of `semantic::check_program` — the ITERATIVE
/// (flat-stack) depth measurement over the declarations. No recursion
/// happens during the measurement, so the guard itself cannot overflow;
/// a program whose expression tree is deeper than the input-depth cap
/// refuses with `[INPUT_TOO_DEEP]` BEFORE any recursive walker sees it
/// (the walkers — label, taint, deny, svg, calls — are then guaranteed
/// bounded trees by construction). The parser pre-scan bounds SOURCES;
/// this bounds synthetic/hand-built ASTs and any future scanner-evading
/// form.
pub fn declarations_depth_exceeds(
    declarations: &[crate::ast::Declaration],
) -> Option<(usize, usize)> {
    use crate::ast::{Declaration, Expr, Statement};
    let limit = crate::limits::max_input_depth();
    // BFS over (expr, depth) — the queue holds at most O(width) entries,
    // never deep frames.
    let mut queue: std::collections::VecDeque<(&Expr, usize)> = std::collections::VecDeque::new();
    // The statement walk is FLAT (a worklist of statements); expressions
    // go through the BFS queue with their depth.
    fn walk_statements<'a>(
        stmts: &'a [Statement],
        queue: &mut std::collections::VecDeque<(&'a Expr, usize)>,
    ) {
        for st in stmts {
            match st {
                Statement::ExprStmt { expr, .. } => {
                    queue.push_back((expr, 1));
                }
                Statement::IfElseBlock {
                    condition,
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    queue.push_back((condition, 1));
                    walk_statements(then_body, queue);
                    for (cond, body) in else_ifs {
                        queue.push_back((cond, 1));
                        walk_statements(body, queue);
                    }
                    if let Some(body) = else_body {
                        walk_statements(body, queue);
                    }
                }
                Statement::IfThen {
                    condition, body, ..
                } => {
                    queue.push_back((condition, 1));
                    walk_statements(body, queue);
                }
                Statement::Each { iterable, body, .. } => {
                    queue.push_back((iterable, 1));
                    walk_statements(body, queue);
                }
                Statement::While {
                    condition, body, ..
                } => {
                    queue.push_back((condition, 1));
                    walk_statements(body, queue);
                }
                _ => {}
            }
        }
    }

    for decl in declarations {
        if let Declaration::Pattern(p) = decl {
            walk_statements(&p.body, &mut queue);
        }
        while let Some((e, d)) = queue.pop_front() {
            if d > limit {
                return Some((d, limit));
            }
            match e {
                Expr::BinaryOp { left, right, .. } => {
                    queue.push_back((left, d + 1));
                    queue.push_back((right, d + 1));
                }
                Expr::IfElse {
                    condition,
                    then_branch,
                    else_branch,
                    ..
                } => {
                    queue.push_back((condition, d + 1));
                    queue.push_back((then_branch, d + 1));
                    queue.push_back((else_branch, d + 1));
                }
                Expr::BlockIfElse {
                    condition,
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    queue.push_back((condition, d + 1));
                    walk_statements(then_body, &mut queue);
                    for (cond, body) in else_ifs {
                        queue.push_back((cond, d + 1));
                        walk_statements(body, &mut queue);
                    }
                    if let Some(body) = else_body {
                        walk_statements(body, &mut queue);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

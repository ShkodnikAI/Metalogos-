// ── Naryad #394 (P1, feature/mcp, wave 3): MCP tool-policy compiler ───
//
// The MCP server's tool-policy is COMPILED from the program contour, not
// hand-written YAML: for every exposed tool method the compiler walks the
// method body AST, finds the builtins it calls, and reads their role /
// label / reversibility from the №316 SSOT classification
// (`builtins_classification::classify`) — the same map every security
// gate reads. The policy answers, mechanically:
//   - which calls go to audit (every Role::Sink call, with its sink
//     class from `audit::sink_kind` — the SSOT the deny gates use);
//   - which tool arguments require clearance (params that lexically flow
//     into a sink call's arguments — conservative: any identifier inside
//     a sink argument subtree counts);
//   - whether the method can perform an irreversible effect (any
//     Irreversible-classified builtin);
//   - which sources the method ingests (Role::Source calls).
//
// Publishing stays FAIL-CLOSED: the allowlist decides what is published
// (unchanged №297 posture); the policy only annotates what the published
// tool does — there is no policy knob that can widen anything.

use crate::ast::{Expr, Statement, ToolMethod};

/// One audit-relevant sink call discovered in a method body.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct SinkCall {
    pub builtin: String,
    /// The sink class (`audit::sink_kind` SSOT): exec/vcs/voice/db/
    /// output/file/memory/network.
    pub class: String,
}

/// The compiled policy for one tool method (ADR-0168 §4).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ToolPolicy {
    /// Every Role::Sink call in the method body (dedup, in call order) —
    /// these are the audited effects.
    pub sink_calls: Vec<SinkCall>,
    /// Every Role::Source call (ingress points).
    pub source_calls: Vec<String>,
    /// Tool params that lexically flow into a sink call's arguments —
    /// the arguments a consumer must treat as clearance-relevant.
    pub clearance_args: Vec<String>,
    /// True when the method can perform an irreversible effect.
    pub irreversible: bool,
    /// Builtins without a classification entry (intercepted names the
    /// registry resolves at runtime — e.g. `deny_event`) — honest
    /// visibility instead of silent omission.
    pub unclassified: Vec<String>,
}

impl ToolPolicy {
    /// The `_meta` key the tools/list entries carry.
    pub const META_KEY: &str = "metalogos.dev/policy";
    /// Schema version of the compiled policy.
    pub const VERSION: u32 = 1;
}

/// Collect builtin calls from a method body and compile the policy.
pub fn compile_policy(method: &ToolMethod) -> ToolPolicy {
    let mut usage = Usage::default();
    for stmt in &method.body {
        walk_stmt(stmt, &mut usage);
    }
    let mut sink_calls: Vec<SinkCall> = Vec::new();
    let mut source_calls: Vec<String> = Vec::new();
    let mut clearance_args: Vec<String> = Vec::new();
    let mut irreversible = false;
    let mut unclassified: Vec<String> = Vec::new();
    for name in &usage.calls {
        match crate::builtins_classification::classify(name) {
            Some(class) => {
                match class.role {
                    crate::builtins_classification::Role::Sink => {
                        let sc = SinkCall {
                            builtin: name.clone(),
                            class: crate::audit::sink_kind(name).to_string(),
                        };
                        if !sink_calls.contains(&sc) {
                            sink_calls.push(sc);
                        }
                        // Params flowing into THIS sink's arguments.
                        if let Some(idents) = usage.sink_args.get(name) {
                            for ident in idents {
                                if method.params.iter().any(|p| &p.name == ident)
                                    && !clearance_args.contains(ident)
                                {
                                    clearance_args.push(ident.clone());
                                }
                            }
                        }
                        if class.reversibility
                            == crate::builtins_classification::Reversibility::Irreversible
                        {
                            irreversible = true;
                        }
                    }
                    crate::builtins_classification::Role::Source
                        if !source_calls.contains(name) =>
                    {
                        source_calls.push(name.clone());
                    }
                    crate::builtins_classification::Role::Source => {}
                    _ => {}
                }
                if class.role != crate::builtins_classification::Role::Pure
                    && class.reversibility
                        == crate::builtins_classification::Reversibility::Irreversible
                    && !sink_calls.iter().any(|s| &s.builtin == name)
                {
                    // An irreversible non-sink (e.g. a Lift-classified
                    // ledger_rotate) still flips the irreversible flag.
                    irreversible = true;
                }
            }
            None => {
                if !unclassified.contains(name) {
                    unclassified.push(name.clone());
                }
            }
        }
    }
    ToolPolicy {
        sink_calls,
        source_calls,
        clearance_args,
        irreversible,
        unclassified,
    }
}

/// Raw walk result: ordered builtin call names + the identifier sets
/// passed into each sink call's arguments.
#[derive(Default)]
struct Usage {
    calls: Vec<String>,
    sink_args: std::collections::HashMap<String, Vec<String>>,
}

fn push_call(usage: &mut Usage, name: &str) {
    if !usage.calls.iter().any(|c| c == name) {
        usage.calls.push(name.to_string());
    }
}

fn walk_stmt(stmt: &Statement, usage: &mut Usage) {
    match stmt {
        Statement::LetBinding { value, .. } => walk_expr(value, usage),
        Statement::Assign { value, .. } => walk_expr(value, usage),
        Statement::Each { iterable, body, .. } => {
            walk_expr(iterable, usage);
            walk_stmts(body, usage);
        }
        Statement::EachWithIndex { iterable, body, .. } => {
            walk_expr(iterable, usage);
            walk_stmts(body, usage);
        }
        Statement::While {
            condition, body, ..
        } => {
            walk_expr(condition, usage);
            walk_stmts(body, usage);
        }
        Statement::IfElseBlock {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            walk_expr(condition, usage);
            walk_stmts(then_body, usage);
            for (cond, body) in else_ifs {
                walk_expr(cond, usage);
                walk_stmts(body, usage);
            }
            if let Some(b) = else_body {
                walk_stmts(b, usage);
            }
        }
        Statement::IfThen {
            condition, body, ..
        } => {
            walk_expr(condition, usage);
            walk_stmts(body, usage);
        }
        Statement::Return { value, .. } => walk_expr(value, usage),
        Statement::ExprStmt { expr, .. } => walk_expr(expr, usage),
        Statement::Match {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            walk_expr(scrutinee, usage);
            for arm in arms {
                walk_stmts(arm.body(), usage);
                if let crate::ast::MatchArm::Compare(_, expr, _) = arm {
                    walk_expr(expr, usage);
                }
            }
            if let Some(b) = else_body {
                walk_stmts(b, usage);
            }
        }
        Statement::Memorize(m) => walk_expr(&m.value, usage),
        Statement::Forget(f) => walk_expr(&f.query, usage),
        Statement::Relate(r) => {
            walk_expr(&r.from, usage);
            walk_expr(&r.to, usage);
        }
        Statement::Break | Statement::Continue => {}
    }
}

fn walk_stmts(stmts: &[Statement], usage: &mut Usage) {
    for s in stmts {
        walk_stmt(s, usage);
    }
}

fn walk_expr(expr: &Expr, usage: &mut Usage) {
    match expr {
        Expr::FnCall { name, args, .. } => {
            push_call(usage, name);
            let is_sink = crate::builtins_classification::classify(name)
                .map(|c| c.role == crate::builtins_classification::Role::Sink)
                .unwrap_or(false);
            let mut idents = Vec::new();
            for arg in args {
                if is_sink {
                    collect_idents(arg, &mut idents);
                }
                walk_expr(arg, usage);
            }
            if is_sink {
                let entry = usage.sink_args.entry(name.clone()).or_default();
                for i in idents {
                    if !entry.contains(&i) {
                        entry.push(i);
                    }
                }
            }
        }
        Expr::QualifiedCall { function, args, .. } => {
            push_call(usage, function);
            for arg in args {
                walk_expr(arg, usage);
            }
        }
        Expr::FieldAccess { object, .. } => walk_expr(object, usage),
        Expr::BinaryOp { left, right, .. } => {
            walk_expr(left, usage);
            walk_expr(right, usage);
        }
        Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            walk_expr(condition, usage);
            walk_expr(then_branch, usage);
            walk_expr(else_branch, usage);
        }
        Expr::List { items, .. } => {
            for i in items {
                walk_expr(i, usage);
            }
        }
        Expr::IndexAccess { object, index, .. } => {
            walk_expr(object, usage);
            walk_expr(index, usage);
        }
        Expr::StructLit { fields, .. } => {
            for v in fields.values() {
                walk_expr(v, usage);
            }
        }
        Expr::BlockIfElse {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            walk_expr(condition, usage);
            walk_stmts(then_body, usage);
            for (cond, body) in else_ifs {
                walk_expr(cond, usage);
                walk_stmts(body, usage);
            }
            if let Some(b) = else_body {
                walk_stmts(b, usage);
            }
        }
        Expr::MatchExpr {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            walk_expr(scrutinee, usage);
            for arm in arms {
                walk_stmts(arm.body(), usage);
                if let crate::ast::MatchArm::Compare(_, expr, _) = arm {
                    walk_expr(expr, usage);
                }
            }
            if let Some(b) = else_body {
                walk_stmts(b, usage);
            }
        }
        Expr::Try { expr, .. } => walk_expr(expr, usage),
        Expr::ProvBind { inner, .. } => walk_expr(inner, usage),
        Expr::StringLit { .. }
        | Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::Ident { .. }
        | Expr::HandleSource { .. } => {}
    }
}

/// Conservative identifier collection: every Ident in the subtree counts
/// as flowing into the sink argument (no path sensitivity — the policy is
/// an annotation, never a gate).
fn collect_idents(expr: &Expr, out: &mut Vec<String>) {
    match expr {
        Expr::Ident { name, .. } => {
            if !out.contains(name) {
                out.push(name.clone());
            }
        }
        Expr::FnCall { args, .. } => {
            for a in args {
                collect_idents(a, out);
            }
        }
        Expr::QualifiedCall { args, .. } => {
            for a in args {
                collect_idents(a, out);
            }
        }
        Expr::FieldAccess { object, .. } => collect_idents(object, out),
        Expr::BinaryOp { left, right, .. } => {
            collect_idents(left, out);
            collect_idents(right, out);
        }
        Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            collect_idents(condition, out);
            collect_idents(then_branch, out);
            collect_idents(else_branch, out);
        }
        Expr::List { items, .. } => {
            for i in items {
                collect_idents(i, out);
            }
        }
        Expr::IndexAccess { object, index, .. } => {
            collect_idents(object, out);
            collect_idents(index, out);
        }
        Expr::StructLit { fields, .. } => {
            for v in fields.values() {
                collect_idents(v, out);
            }
        }
        Expr::BlockIfElse {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            collect_idents(condition, out);
            collect_idents_in_stmts(then_body, out);
            for (cond, body) in else_ifs {
                collect_idents(cond, out);
                collect_idents_in_stmts(body, out);
            }
            if let Some(b) = else_body {
                collect_idents_in_stmts(b, out);
            }
        }
        Expr::MatchExpr {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            collect_idents(scrutinee, out);
            for arm in arms {
                collect_idents_in_stmts(arm.body(), out);
                if let crate::ast::MatchArm::Compare(_, expr, _) = arm {
                    collect_idents(expr, out);
                }
            }
            if let Some(b) = else_body {
                collect_idents_in_stmts(b, out);
            }
        }
        Expr::Try { expr, .. } => collect_idents(expr, out),
        Expr::ProvBind { inner, .. } => collect_idents(inner, out),
        Expr::StringLit { .. }
        | Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::HandleSource { .. } => {}
    }
}

fn collect_idents_in_stmts(stmts: &[Statement], out: &mut Vec<String>) {
    for s in stmts {
        collect_idents_in_stmt(s, out);
    }
}

fn collect_idents_in_stmt(stmt: &Statement, out: &mut Vec<String>) {
    match stmt {
        Statement::LetBinding { value, .. } => collect_idents(value, out),
        Statement::Assign { value, .. } => collect_idents(value, out),
        Statement::Each { iterable, body, .. } => {
            collect_idents(iterable, out);
            collect_idents_in_stmts(body, out);
        }
        Statement::EachWithIndex { iterable, body, .. } => {
            collect_idents(iterable, out);
            collect_idents_in_stmts(body, out);
        }
        Statement::While {
            condition, body, ..
        } => {
            collect_idents(condition, out);
            collect_idents_in_stmts(body, out);
        }
        Statement::IfElseBlock {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            collect_idents(condition, out);
            collect_idents_in_stmts(then_body, out);
            for (cond, body) in else_ifs {
                collect_idents(cond, out);
                collect_idents_in_stmts(body, out);
            }
            if let Some(b) = else_body {
                collect_idents_in_stmts(b, out);
            }
        }
        Statement::IfThen {
            condition, body, ..
        } => {
            collect_idents(condition, out);
            collect_idents_in_stmts(body, out);
        }
        Statement::Return { value, .. } => collect_idents(value, out),
        Statement::ExprStmt { expr, .. } => collect_idents(expr, out),
        Statement::Match {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            collect_idents(scrutinee, out);
            for arm in arms {
                collect_idents_in_stmts(arm.body(), out);
                if let crate::ast::MatchArm::Compare(_, expr, _) = arm {
                    collect_idents(expr, out);
                }
            }
            if let Some(b) = else_body {
                collect_idents_in_stmts(b, out);
            }
        }
        Statement::Memorize(m) => collect_idents(&m.value, out),
        Statement::Forget(f) => collect_idents(&f.query, out),
        Statement::Relate(r) => {
            collect_idents(&r.from, out);
            collect_idents(&r.to, out);
        }
        Statement::Break | Statement::Continue => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_meta_contract_is_stable() {
        assert_eq!(ToolPolicy::META_KEY, "metalogos.dev/policy");
        assert_eq!(ToolPolicy::VERSION, 1);
    }
}

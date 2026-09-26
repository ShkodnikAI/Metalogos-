//! №474 (gh#742) — stage 1 of the type system: let-type inference,
//! WARN-ONLY (gate gh#680, decision 3-A, step 2).
//!
//! The canonical stage-1 definition is the №467 body (gh#688): "этап 1
//! (вывод let, warn-only)" — the inference is a DIAGNOSTIC, never a
//! rejection: blocking type checks are out (the №467 boundary — after
//! the migration path, office#372), and stage 2 (the Labeled migration
//! of SECRET_LEAK / SQL_DYNAMIC / HTML_INJECTION) is a separate naryad
//! of a following wave.
//!
//! The machinery (deliberately minimal, per the М1 decomposition):
//! - a per-block, straight-line type environment over `let` bindings;
//! - a binding gets a KNOWN type when its initializer is
//!     (a) a direct builtin call whose `BUILTIN_REGISTRY` row carries a
//!         typed stage-0 `return_type` (not `Unknown`), or
//!     (b) a literal (string / float / bool / list), or
//!     (c) a copy from another let-bound variable of known type;
//!   everything else (binary ops, field/index access, struct literals,
//!   pattern calls, namespaced calls, if/match/try expressions) is
//!   conservatively `Unknown` — and `Unknown` NEVER warns (the honesty
//!   pin, tested);
//! - exactly ONE warn rule: a variable with a known inferred type `T`
//!   reassigned through `=` with an expression of a known DIFFERENT
//!   type `U` produces a warning into `AnalysisResult.warnings`. The
//!   language stays dynamically typed — the program compiles and runs
//!   exactly as before (warn-only proof, tested);
//! - descent: the top level of each block plus `each` bodies (walked
//!   with a CLONED environment — inner bindings do not leak out, since
//!   the loop may run zero times). Other nested blocks (if/match/while)
//!   are not descended in stage 1 — loudly.
//!
//! The warnings carry the `[stage1 types]` prefix (grep-able; the corpus
//! test pins every stage-1 warning to it). Shadowing is respected: a
//! name that resolves to a DECLARED pattern (not a builtin) never takes
//! the registry type.

use std::collections::{HashMap, HashSet};

use crate::ast::{Declaration, Expr, Statement};
use crate::builtins::registry::BUILTIN_REGISTRY;
use crate::builtins::sig_types::Type;
use crate::semantic::SpannedError;

/// The grep-able prefix of every stage-1 warning.
pub const STAGE1_PREFIX: &str = "[stage1 types]";

/// A known type plus the human-readable origin it was inferred from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeOrigin {
    pub ty: Type,
    pub origin: String,
}

/// The per-block inferred environment: variable → (type, origin).
pub type TypeEnv = HashMap<String, TypeOrigin>;

/// Stage 1 entry: infer let-types over every statement-bearing
/// declaration body (patterns, test blocks, tool methods) and produce
/// the warn-only diagnostics. Pure: never touches `errors`, never
/// rejects a program.
pub fn stage1_warnings(declarations: &[Declaration]) -> Vec<SpannedError> {
    let callables = callable_names(declarations);
    let mut warnings = Vec::new();
    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => {
                let mut env = TypeEnv::new();
                walk_block(&p.body, &mut env, &callables, &mut warnings);
            }
            Declaration::Test(t) => {
                let mut env = TypeEnv::new();
                walk_block(&t.body, &mut env, &callables, &mut warnings);
            }
            Declaration::Tool(tool) => {
                for method in &tool.methods {
                    let mut env = TypeEnv::new();
                    walk_block(&method.body, &mut env, &callables, &mut warnings);
                }
            }
            _ => {}
        }
    }
    warnings
}

/// The test-visible variant: the FINAL environment of one block
/// (straight-line; `each` bodies are walked with cloned environments
/// and do not leak). Used by `tests/naryad_474_stage1_types.rs` to pin
/// the inference golden directly.
pub fn infer_block_types(body: &[Statement]) -> TypeEnv {
    let mut env = TypeEnv::new();
    let empty = HashSet::new();
    walk_block_inner(body, &mut env, &empty, &mut Vec::new());
    env
}

/// Names that resolve to DECLARED patterns (a `FnCall` of one of these
/// is a pattern call, not a builtin — no registry type is taken).
fn callable_names(declarations: &[Declaration]) -> HashSet<String> {
    let mut names = HashSet::new();
    for decl in declarations {
        if let Declaration::Pattern(p) = decl {
            names.insert(p.name.clone());
        }
    }
    names
}

fn walk_block(
    stmts: &[Statement],
    env: &mut TypeEnv,
    callables: &HashSet<String>,
    warnings: &mut Vec<SpannedError>,
) {
    walk_block_inner(stmts, env, callables, warnings);
}

fn walk_block_inner(
    stmts: &[Statement],
    env: &mut TypeEnv,
    callables: &HashSet<String>,
    warnings: &mut Vec<SpannedError>,
) {
    for st in stmts {
        match st {
            Statement::LetBinding { name, value, .. } => {
                // A fresh binding (or a re-`let`): the previous knowledge
                // is replaced by whatever the new initializer proves —
                // no conflict warning (the runtime rebinds silently).
                match infer_expr(value, env, callables) {
                    Some(entry) => {
                        env.insert(name.clone(), entry);
                    }
                    None => {
                        env.remove(name);
                    }
                }
            }
            Statement::Assign { name, value, span } => {
                let inferred = infer_expr(value, env, callables);
                if let (Some(prev), Some(next)) = (env.get(name), inferred.as_ref()) {
                    if prev.ty != next.ty {
                        warnings.push(SpannedError::at(
                            format!(
                                "{} type conflict: variable '{}' was inferred {} ({}), \
                                 but is reassigned with {} ({}) — the language stays \
                                 dynamically typed; this is a warn-only diagnostic \
                                 (naryad №474, stage 1)",
                                STAGE1_PREFIX,
                                name,
                                format!("{:?}", prev.ty),
                                prev.origin,
                                format!("{:?}", next.ty),
                                next.origin,
                            ),
                            span.clone(),
                        ));
                    }
                }
                // After the assignment the variable HOLDS the new value:
                // the knowledge follows the runtime (or is dropped when
                // the new type is unknown — honesty over cleverness).
                match inferred {
                    Some(entry) => {
                        env.insert(name.clone(), entry);
                    }
                    None => {
                        env.remove(name);
                    }
                }
            }
            Statement::Each { body, .. } | Statement::EachWithIndex { body, .. } => {
                // The loop may run zero times: inner bindings do not leak.
                let mut inner = env.clone();
                walk_block_inner(body, &mut inner, callables, warnings);
            }
            // Stage-1 boundary (loud): other nested blocks (if/match/
            // while/return/expr-statements) are not descended — their
            // lets stay Unknown, no warnings fire inside them.
            _ => {}
        }
    }
}

/// The best-effort type of one expression: `Some` = known, `None` =
/// Unknown (never stored, never warned).
fn infer_expr(expr: &Expr, env: &TypeEnv, callables: &HashSet<String>) -> Option<TypeOrigin> {
    match expr {
        Expr::StringLit { .. } => Some(TypeOrigin {
            ty: Type::String,
            origin: "a string literal".to_string(),
        }),
        Expr::FloatLit { .. } => Some(TypeOrigin {
            ty: Type::Float,
            origin: "a float literal".to_string(),
        }),
        Expr::BoolLit { .. } => Some(TypeOrigin {
            ty: Type::Bool,
            origin: "a bool literal".to_string(),
        }),
        Expr::List { items, .. } => Some(TypeOrigin {
            ty: Type::List,
            origin: format!("a list literal ({} items)", items.len()),
        }),
        Expr::Ident { name, .. } => env.get(name).cloned(),
        Expr::FnCall { name, .. } => {
            if callables.contains(name) {
                return None; // a declared pattern call — no registry type
            }
            BUILTIN_REGISTRY
                .iter()
                .find(|s| s.name == name)
                .map(|s| TypeOrigin {
                    ty: s.return_type.clone(),
                    origin: format!("the builtin '{}'", name),
                })
                .filter(|entry| entry.ty != Type::Unknown) // Unknown NEVER lies
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_expr_literals_are_typed() {
        let env = TypeEnv::new();
        let empty: HashSet<String> = HashSet::new();
        assert_eq!(
            infer_expr(
                &Expr::StringLit {
                    value: "x".into(),
                    span: crate::ast::Span::unknown()
                },
                &env,
                &empty
            )
            .unwrap()
            .ty,
            Type::String
        );
        assert_eq!(
            infer_expr(
                &Expr::FloatLit {
                    value: 1.0,
                    span: crate::ast::Span::unknown()
                },
                &env,
                &empty
            )
            .unwrap()
            .ty,
            Type::Float
        );
        assert_eq!(
            infer_expr(
                &Expr::BoolLit {
                    value: true,
                    span: crate::ast::Span::unknown()
                },
                &env,
                &empty
            )
            .unwrap()
            .ty,
            Type::Bool
        );
    }

    #[test]
    fn unknown_builtin_never_yields_a_type() {
        let env = TypeEnv::new();
        let empty: HashSet<String> = HashSet::new();
        // A registry name that exists but carries the honest Unknown.
        let untyped = BUILTIN_REGISTRY
            .iter()
            .find(|s| s.return_type == Type::Unknown)
            .expect("stage 0 keeps Unknown rows — the corpus relies on it");
        let got = infer_expr(
            &Expr::FnCall {
                name: untyped.name.to_string(),
                args: vec![],
                span: crate::ast::Span::unknown(),
            },
            &env,
            &empty,
        );
        assert!(got.is_none(), "Unknown must stay unknown");
    }
}

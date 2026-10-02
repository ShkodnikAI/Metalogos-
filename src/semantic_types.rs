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
//!   (a) a direct builtin call whose `BUILTIN_REGISTRY` row carries a
//!   typed stage-0 `return_type` (not `Unknown`), or
//!   (b) a literal (string / float / bool / list), or
//!   (c) a copy from another let-bound variable of known type;
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
//!
//! ── №486 (gh#734) — the declared-type conflict surface ──────────────
//!
//! Stage 0 typed the BUILTIN return signatures (49→50 registry rows);
//! the language ALSO carries declared types as `String` spellings in the
//! AST (`PatternDecl::return_type`, `Param::type_name`,
//! `ToolMethod::return_type`, `LearnablePatternDecl::return_type`).
//! Until now NOTHING compared the two worlds — a pattern declaring
//! `-> Int` that returns a String builtin passed silently (the audit
//! v0.26.1 §3.9 finding). Three warn-only checks close that (same pass,
//! same honesty rules, own grep-able prefix `[n486 types]`):
//!
//! R1 — DECLARED RETURN vs the returned expression: at a `return` whose
//!      expression has a KNOWN type (direct typed builtin call, literal,
//!      typed variable, or a pattern call under its DECLARED return
//!      type), a different comparable declared return type warns.
//! R2 — DECLARED PARAMETERS seed the inference environment: a parameter
//!      declared `x: String` participates like a let of that type, so
//!      the EXISTING stage-1 assign rule flags `x = <Float builtin>`.
//!      The origin text names the declaration, not an inference.
//! R3 — PATTERN CALLS type from their DECLARED return type: `let v =
//!      helper()` where helper declares `-> Float` gives `v` the
//!      declared Float, so a later builtin reassignment of a different
//!      known type conflicts (the №474 rule catches it; the origin names
//!      the declaration).
//!
//! Honesty (pinned by tests): a declared spelling that does not parse
//! into the stage-0 vocabulary (`-> Message`, entity names) is Unknown —
//! it NEVER warns; a declared `Fluid` never warns (Fluid accepts any
//! value by design); labels are ERASED before comparing (`String<private>`
//! compares as `String` — labels are annotations, not runtime types);
//! `Unknown` never warns (the №474 pin, unchanged). Nested blocks other
//! than `each` stay undescented (the №474 stage-1 boundary, loud).

use std::collections::HashMap;

use crate::ast::{Declaration, Expr, Statement};
use crate::builtins::registry::BUILTIN_REGISTRY;
use crate::builtins::sig_types::Type;
use crate::semantic::SpannedError;

/// The grep-able prefix of every stage-1 warning.
pub const STAGE1_PREFIX: &str = "[stage1 types]";

/// The grep-able prefix of every №486 declared-type warning.
pub const N486_PREFIX: &str = "[n486 types]";

/// The declared surface of one callable: the declared return type
/// (already reduced to the comparable form — `None` when the spelling is
/// Unknown/Fluid and must stay silent) plus the declared parameter
/// types. `kind` is the honest noun for the warning text
/// ("pattern" / "learnable pattern").
struct DeclaredSig {
    kind: &'static str,
    return_ty: Option<Type>,
    params: Vec<(String, Option<Type>)>,
}

/// The comparable form of a declared type spelling: labels are erased
/// (annotations, not runtime types), `Fluid` and `Unknown` collapse to
/// `None` (both must stay silent — Fluid accepts anything, Unknown is
/// the honest "not typed").
fn comparable(ty: Type) -> Option<Type> {
    match ty {
        Type::Labeled(inner, _) => comparable(*inner),
        Type::Fluid | Type::Unknown => None,
        other => Some(other),
    }
}

/// Parse one declared surface (`PatternDecl::return_type` /
/// `Param::type_name` / `ToolMethod::return_type`) into its comparable
/// form.
fn declared(return_type: &str) -> Option<Type> {
    comparable(crate::builtins::sig_types::parse_type(return_type))
}

/// Build the declared signature of a pattern-like callable.
fn declared_sig(
    kind: &'static str,
    return_type: &str,
    params: &[crate::ast::Param],
) -> DeclaredSig {
    DeclaredSig {
        kind,
        return_ty: declared(return_type),
        params: params
            .iter()
            .map(|p| (p.name.clone(), declared(&p.type_name)))
            .collect(),
    }
}

/// Seed the environment with the DECLARED parameter types (R2): a
/// parameter declared `x: String` enters the walk exactly like a let of
/// that type, with an origin that names the declaration. Unparseable
/// spellings stay out (the honesty rule).
fn seed_params(sig: &DeclaredSig, env: &mut TypeEnv) {
    for (name, ty) in &sig.params {
        if let Some(ty) = ty {
            let origin = format!("the declared parameter type '{}'", ty);
            env.insert(
                name.clone(),
                TypeOrigin {
                    ty: ty.clone(),
                    origin,
                },
            );
        }
    }
}

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
/// the warn-only diagnostics — the №474 assign rule AND the №486
/// declared-type checks. Pure: never touches `errors`, never rejects a
/// program.
pub fn stage1_warnings(declarations: &[Declaration]) -> Vec<SpannedError> {
    let callables = callable_sigs(declarations);
    let mut warnings = Vec::new();
    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => {
                let sig = declared_sig("pattern", &p.return_type, &p.params);
                let mut env = TypeEnv::new();
                seed_params(&sig, &mut env);
                walk_block(
                    &p.body,
                    &mut env,
                    &callables,
                    &sig.return_ty,
                    "pattern",
                    &p.name,
                    &mut warnings,
                );
            }
            Declaration::Test(t) => {
                let mut env = TypeEnv::new();
                let none: Option<Type> = None;
                walk_block(
                    &t.body,
                    &mut env,
                    &callables,
                    &none,
                    "test block",
                    "",
                    &mut warnings,
                );
            }
            Declaration::Tool(tool) => {
                for method in &tool.methods {
                    // A ToolMethod is structurally a pattern (the AST doc);
                    // the declared return/param surfaces apply the same way.
                    let sig = declared_sig("tool method", &method.return_type, &method.params);
                    let mut env = TypeEnv::new();
                    seed_params(&sig, &mut env);
                    let owner = format!("{}.{}", tool.name, method.name);
                    walk_block(
                        &method.body,
                        &mut env,
                        &callables,
                        &sig.return_ty,
                        "tool method",
                        &owner,
                        &mut warnings,
                    );
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
    let empty: HashMap<String, DeclaredSig> = HashMap::new();
    let none: Option<Type> = None;
    walk_block_inner(body, &mut env, &empty, &none, "", "", &mut Vec::new());
    env
}

/// The DECLARED signatures of the plain-name callables (R3's source):
/// `pattern` and `learnable pattern` declarations. A call of one of
/// these takes its DECLARED return type — never the registry type (the
/// shadowing promise). Tool methods are QUALIFIED calls
/// (`tool.method`), not plain names — their declared surfaces are
/// applied in the `Tool` arm of the walk, not here.
fn callable_sigs(declarations: &[Declaration]) -> HashMap<String, DeclaredSig> {
    let mut map: HashMap<String, DeclaredSig> = HashMap::new();
    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => {
                map.insert(
                    p.name.clone(),
                    declared_sig("pattern", &p.return_type, &p.params),
                );
            }
            Declaration::LearnablePattern(l) => {
                map.insert(
                    l.name.clone(),
                    declared_sig("learnable pattern", &l.return_type, &l.params),
                );
            }
            _ => {}
        }
    }
    map
}

fn walk_block(
    stmts: &[Statement],
    env: &mut TypeEnv,
    callables: &HashMap<String, DeclaredSig>,
    declared_return: &Option<Type>,
    kind: &str,
    owner: &str,
    warnings: &mut Vec<SpannedError>,
) {
    walk_block_inner(
        stmts,
        env,
        callables,
        declared_return,
        kind,
        owner,
        warnings,
    );
}

fn walk_block_inner(
    stmts: &[Statement],
    env: &mut TypeEnv,
    callables: &HashMap<String, DeclaredSig>,
    declared_return: &Option<Type>,
    kind: &str,
    owner: &str,
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
                        // Precompute the Debug spellings — no format! inside
                        // format! args (the clippy -D warnings lint).
                        let prev_ty_dbg = format!("{:?}", prev.ty);
                        let next_ty_dbg = format!("{:?}", next.ty);
                        warnings.push(SpannedError::at(
                            format!(
                                "{} type conflict: variable '{}' was inferred {} ({}), \
                                 but is reassigned with {} ({}) — the language stays \
                                 dynamically typed; this is a warn-only diagnostic \
                                 (naryad №474, stage 1)",
                                STAGE1_PREFIX,
                                name,
                                prev_ty_dbg,
                                prev.origin,
                                next_ty_dbg,
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
            Statement::Return { value, span } => {
                // №486 R1: the DECLARED return type vs the returned
                // expression. Only a KNOWN returned type vs a KNOWN
                // declared type warns — anything unknown stays silent
                // (the honesty rule). Bare `return` never reaches here
                // as a Unit: the grammar requires an expression, and a
                // Unit inference is deliberately not invented.
                if let Some(declared) = declared_return {
                    if let Some(got) = infer_expr(value, env, callables) {
                        if got.ty != *declared {
                            let declared_ty = declared.to_string();
                            let got_ty = got.ty.to_string();
                            warnings.push(SpannedError::at(
                                format!(
                                    "{} declared-return conflict: the {} '{}' declares -> {}, \
                                     but its `return` produces {} (from {}) — the language stays \
                                     dynamically typed; this is a warn-only diagnostic \
                                     (naryad №486)",
                                    N486_PREFIX, kind, owner, declared_ty, got_ty, got.origin,
                                ),
                                span.clone(),
                            ));
                        }
                    }
                }
            }
            Statement::Each { body, .. } | Statement::EachWithIndex { body, .. } => {
                // The loop may run zero times: inner bindings do not leak.
                let mut inner = env.clone();
                walk_block_inner(
                    body,
                    &mut inner,
                    callables,
                    declared_return,
                    kind,
                    owner,
                    warnings,
                );
            }
            // Stage-1 boundary (loud): other nested blocks (if/match/
            // while/expr-statements) are not descended — their
            // lets stay Unknown, no warnings fire inside them.
            _ => {}
        }
    }
}

/// The best-effort type of one expression: `Some` = known, `None` =
/// Unknown (never stored, never warned).
fn infer_expr(
    expr: &Expr,
    env: &TypeEnv,
    callables: &HashMap<String, DeclaredSig>,
) -> Option<TypeOrigin> {
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
            // A declared callable takes its DECLARED return type (№486
            // R3) — never the registry type (the shadowing promise).
            // An unparseable/Fluid declaration is None — silent, like
            // any Unknown.
            if let Some(sig) = callables.get(name) {
                return sig.return_ty.clone().map(|ty| TypeOrigin {
                    ty,
                    origin: format!("the declared return type of the {} '{}'", sig.kind, name),
                });
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
        let empty: HashMap<String, DeclaredSig> = HashMap::new();
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
        let empty: HashMap<String, DeclaredSig> = HashMap::new();
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

// ── №544 (gh#882) step 1 — the secret lane's label semantics ────────
//
//! Stage 2 of the type system (№467 canon): the label-algebra application
//! to the SECRET_LEAK audit class. A LEAF module by design — it imports
//! the AST and the stage-0 signature types, and nothing imports it back;
//! audit.rs's check_secret_leak consumes it (the C4 acyclicity ratchet
//! sees no new intra-cycle edge).
//!
//! A secret source (`env()` / `secret()`) is a value of the type
//! `Labeled(String, Private)` — the confidentiality label of the
//! signature taxonomy (sig_types::Label::Private; the №322 lattice is
//! the algebra behind it — conf=private). The label JOINS through
//! composition (a composite with any private part is private — the
//! ADR-0154 §2 conf join over data), sanitizers strip it
//! (`render` / `escape_html`), `redact` lifts it exactly when the
//! policy is one-way (the redact-policy registry, ADR-0136 D2), and
//! the sink check asks the TYPE question. The lanes of the other audit
//! classes: SQL_DYNAMIC is typed since №544 step 2 (its own leaf
//! `sql_label.rs` — the derivation marker `Labeled(String, Internal)`,
//! the sink demanding the literal template); LlmOutput / UserInput /
//! CanaryLeak stay on the taint machinery until their own migration
//! step (step 3: HTML_INJECTION).

use std::collections::HashMap;

use crate::ast::Expr;
use crate::builtins::sig_types::{Label, Type};

// ── №544 (gh#882) step 1 — the secret lane's label semantics (typed) ────────────────────
//
// Stage 2 of the type system (№467 canon). This is the LABEL-ALGEBRA home
// of the lane: audit.rs's check_secret_leak and semantic.rs's label_source
// consume it — both modules already depend on labels.rs, so the C4
// acyclicity ratchet sees no new intra-cycle edge (the ADR-0154 §5
// lattice is the shared vocabulary by construction).
// (the №467 canon: "этап 2 —
// SECRET_LEAK/SQL_DYNAMIC/HTML_INJECTION на Labeled(Box<Type>, Label)").
// Step 1 of the migration: the SECRET_LEAK class stops speaking a bespoke
// taint kind and speaks the stage-0 label vocabulary instead — a secret
// source (`env()` / `secret()`) is a value of the type
//
//     Labeled(String, Private)
//
// the confidentiality label of the signature taxonomy (sig_types::Label;
// the №322 lattice is the algebra behind it — conf=private). The label
// JOINS through composition (a composite with any private part is
// private — the ADR-0154 §2 conf join over data), sanitizers strip it
// (`render` / `escape_html` — the audit's Sanitized semantics),
// `redact` strips it exactly when the policy is one-way (the
// redact-policy registry, target_conf == "public" — ADR-0136 D2), and
// the sink check asks the TYPE question
// `is_private_labeled(&label_of_expr(...))` instead of a bespoke enum
// comparison. The OBSERVABLE behavior — which programs are flagged,
// severities, messages, line resolution, the leak-suite corpus parity —
// is unchanged and pinned by the existing suites; the MECHANISM is the
// type. The lanes of the other audit classes: SQL_DYNAMIC is typed
// since №544 step 2 (its own leaf `sql_label.rs`); LlmOutput /
// UserInput / CanaryLeak stay on the taint machinery until their own
// migration step (step 3: HTML_INJECTION).

/// The type of a secret source: `env()` / `secret()` produce a private
/// string — the canonical labeled type of the secret lane.
pub fn secret_source_type() -> Type {
    Type::Labeled(Box::new(Type::String), Label::Private)
}

/// True when the type carries the Private confidentiality label at the
/// top (the secret lane's leak question).
pub fn is_private_labeled(ty: &Option<Type>) -> bool {
    matches!(ty, Some(Type::Labeled(_, Label::Private)))
}

/// Join two known expression types for the secret lane: a composite with
/// any private-labeled part is private-labeled (the conf join); the
/// inner type of a synthesized label defaults to String (the secret
/// vocabulary is strings — env/secret return String).
fn join_secret(a: Option<Type>, b: Option<Type>) -> Option<Type> {
    match (a, b) {
        (None, other) => other,
        (other, None) => other,
        (Some(x), Some(y)) => {
            if is_private_labeled(&Some(x.clone())) || is_private_labeled(&Some(y.clone())) {
                Some(secret_source_type())
            } else {
                // Both known, neither private: the composite keeps the
                // left side's known type (the concat parity) — honestly
                // unknown otherwise.
                Some(x)
            }
        }
    }
}

/// The label-typed value of one expression: `Some(ty)` = the type is
/// known; the leak question is [`is_private_labeled`]. Mirrors the
/// propagation shape of the audit's expression walk EXACTLY (binary
/// ops, field/index access, lists, if/else, call arguments — the label
/// flows through; literals are known and unlabeled; anything the walk
/// cannot see stays None — the honesty rule).
pub fn label_of_expr(expr: &Expr, vars: &HashMap<String, Type>) -> Option<Type> {
    match expr {
        Expr::StringLit { .. } => Some(Type::String),
        Expr::FloatLit { .. } => Some(Type::Float),
        Expr::BoolLit { .. } => Some(Type::Bool),
        Expr::Ident { name, .. } => vars.get(name).cloned(),
        Expr::FnCall { name, args, .. } => {
            // Sanitizers override argument labels (the audit's Sanitized
            // semantics: the result is a known, unlabeled String).
            if name == "render" || name == "escape_html" {
                return Some(Type::String);
            }
            // №274 (ADR-0136): redact is the masking sanitizer — the label
            // lifts exactly when the policy is one-way (target public).
            if name == "redact" {
                return redact_result_label(args, vars);
            }
            // The secret sources: the canonical labeled type.
            if name == "env" || name == "secret" {
                return Some(secret_source_type());
            }
            // Any other call: the label joins from the arguments (the
            // taint-propagation parity — upper(env("K")) stays private).
            let mut acc: Option<Type> = None;
            for arg in args {
                acc = join_secret(acc, label_of_expr(arg, vars));
            }
            acc
        }
        Expr::BinaryOp { left, right, .. } => {
            join_secret(label_of_expr(left, vars), label_of_expr(right, vars))
        }
        Expr::FieldAccess { object, .. } => label_of_expr(object, vars),
        Expr::IndexAccess { index, .. } => label_of_expr(index, vars),
        Expr::List { items, .. } => {
            let mut acc: Option<Type> = Some(Type::List);
            for item in items {
                acc = join_secret(acc, label_of_expr(item, vars));
            }
            acc
        }
        Expr::IfElse {
            then_branch,
            else_branch,
            ..
        } => join_secret(
            label_of_expr(then_branch, vars),
            label_of_expr(else_branch, vars),
        ),
        _ => None,
    }
}

/// The label-typed value of a binding's initializer (the binding parity
/// of the audit's `binding_taint`): direct sources first, then the
/// expression walk.
pub fn binding_label(value: &Expr, vars: &HashMap<String, Type>) -> Option<Type> {
    if let Expr::FnCall { name, .. } = value {
        if name == "env" || name == "secret" {
            return Some(secret_source_type());
        }
        if name == "render" || name == "escape_html" {
            return Some(Type::String);
        }
        if name == "redact" {
            if let Expr::FnCall { args, .. } = value {
                return redact_result_label(args, vars);
            }
        }
    }
    label_of_expr(value, vars)
}

/// The label of a `redact(input, mode)` result: a one-way policy (the
/// redact-policy registry, target_conf == "public") DESTROYS the secret
/// data — the Private label lifts; a conservative policy passes the
/// label through (masking is not declassification — ADR-0136 D2 parity).
fn redact_result_label(args: &[Expr], vars: &HashMap<String, Type>) -> Option<Type> {
    let input = args.first().and_then(|a| label_of_expr(a, vars));
    let mode = args.get(1).and_then(|m| match m {
        Expr::StringLit { value, .. } => Some(value.as_str()),
        _ => None,
    });
    let target_public = mode
        .and_then(crate::builtins::string::redact_policy)
        .is_some_and(|p| p.target_conf == "public");
    match (mode, target_public, input.clone()) {
        (Some(_), true, Some(Type::Labeled(inner, _))) => Some(*inner),
        _ => input,
    }
}

// The label-lane value vocabulary (№544 step 1) walks the AST — the
// algebra and its application live side by side here.

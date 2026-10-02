// ── №544 (gh#882) step 2 — the SQL lane's label semantics ───────────
//
//! Stage 2 of the type system (№467 canon): the label-algebra application
//! to the SQL_DYNAMIC audit class. A LEAF module by design — it imports
//! the AST, the stage-0 signature types and the secret lane's canonical
//! type, and nothing imports it back; audit.rs's check_sql_dynamic
//! consumes it (the C4 acyclicity ratchet sees no new intra-cycle edge).
//!
//! The SQL sink (`query` / `db_execute`) demands a LITERAL SQL template —
//! a string whose content is compile-time known (parameterized queries
//! are the documented safe path; the ?/$N placeholders carry the data).
//! In the stage-2 vocabulary the lane speaks:
//!
//! - a string literal IS the template — its type is the unlabeled
//!   `String`;
//! - every DERIVED string (concatenation, a call result, a variable
//!   reference, an index/field read — anything the walk cannot prove
//!   literal) carries the derivation marker `Labeled(String, Internal)`;
//! - when a secret part joined in (`env()` / `secret()` — the shared
//!   vocabulary with the secret lane), the conf join raises the marker
//!   to `Labeled(String, Private)`.
//!
//! The sink check asks the TYPE question [`is_literal_sql`] instead of
//! the bespoke syntactic comparison. Two type RULES keep the observable
//! parity with the old syntactic verdict (which programs are flagged,
//! severities, messages, line resolution — unchanged and pinned by the
//! existing suites):
//!
//! 1. CONCATEENATION IS DERIVATION ITSELF — a `BinaryOp` never yields
//!    the literal type, even `"a" + "b"` (both parts literal): the old
//!    verdict flagged every non-`StringLit` argument shape, and the
//!    join algebra encodes exactly that (the join of a composite is at
//!    least `Internal`).
//! 2. A BINDING DEMOTES THE LITERAL — the literal type never survives
//!    a `let`/assign: a VARIABLE reference is never a literal (the old
//!    verdict flagged `query(sql)` for the plain variable too). Rule 2
//!    is [`binding_sql_type`]: `String` re-enters the environment as
//!    `Labeled(String, Internal)`.
//!
//! The OBSERVABLE behavior is unchanged; the MECHANISM is the type.
//! The LlmOutput / UserInput / CanaryLeak lanes stay on the taint
//! machinery until their own migration step (step 3: HTML_INJECTION).

use std::collections::HashMap;

use crate::ast::Expr;
use crate::builtins::sig_types::{Label, Type};

/// The type of a DERIVED string: the content is runtime-computed, its
/// provenance is not a compile-time literal — the derivation marker the
/// SQL sink refuses.
pub fn derived_sql_type() -> Type {
    Type::Labeled(Box::new(Type::String), Label::Internal)
}

/// The SQL lane's pass answer: true when the type is the unlabeled
/// literal string — the trusted SQL template. Every labeled, derived,
/// non-string or unknown type refuses.
pub fn is_literal_sql(ty: &Option<Type>) -> bool {
    matches!(ty, Some(Type::String))
}

/// Join two known expression types for the SQL lane: a composite with a
/// private part is private (the conf join — the shared vocabulary with
/// the secret lane); otherwise the composite is derived (at least
/// `Internal` — the derivation marker never lifts through composition).
fn join_sql(a: Option<Type>, b: Option<Type>) -> Option<Type> {
    match (a, b) {
        (None, other) => other,
        (other, None) => other,
        (Some(x), Some(y)) => {
            if crate::secret_label::is_private_labeled(&Some(x.clone()))
                || crate::secret_label::is_private_labeled(&Some(y.clone()))
            {
                Some(crate::secret_label::secret_source_type())
            } else {
                Some(derived_sql_type())
            }
        }
    }
}

/// The SQL-lane type of one expression: `Some(ty)` = the type is known;
/// the sink question is [`is_literal_sql`]. Mirrors the audit walk's
/// visibility EXACTLY (the same expression shapes the old syntactic
/// verdict saw): a string literal is the ONLY literal type; env/secret
/// keep their Private label (the shared vocabulary — the verdict is the
/// same refusal, the label is the honest reason); anything else joins
/// as derived; anything the walk cannot see stays None — the honesty
/// rule. No SQL sanitizer exists in the language (parameterized queries
/// are the safe path), so NO call is exempt from the derivation marker.
pub fn sql_type_of_expr(expr: &Expr, vars: &HashMap<String, Type>) -> Option<Type> {
    match expr {
        // The ONLY pass: a string literal IS the SQL template.
        Expr::StringLit { .. } => Some(Type::String),
        Expr::FloatLit { .. } => Some(Type::Float),
        Expr::BoolLit { .. } => Some(Type::Bool),
        Expr::Ident { name, .. } => vars.get(name).cloned(),
        Expr::FnCall { name, args, .. } => {
            // The secret sources keep their canonical label (the shared
            // vocabulary with the secret lane).
            if name == "env" || name == "secret" {
                return Some(crate::secret_label::secret_source_type());
            }
            // Any other call: the result is derived; the arguments'
            // labels join into it (upper(env("K")) stays private).
            let mut acc: Option<Type> = Some(derived_sql_type());
            for arg in args {
                acc = join_sql(acc, sql_type_of_expr(arg, vars));
            }
            acc
        }
        // Concatenation is derivation itself: never the literal type,
        // at least Internal, Private when a secret part joined in.
        Expr::BinaryOp { left, right, .. } => {
            let joined = join_sql(sql_type_of_expr(left, vars), sql_type_of_expr(right, vars));
            join_sql(Some(derived_sql_type()), joined)
        }
        Expr::FieldAccess { object, .. } => {
            join_sql(Some(derived_sql_type()), sql_type_of_expr(object, vars))
        }
        Expr::IndexAccess { index, .. } => {
            join_sql(Some(derived_sql_type()), sql_type_of_expr(index, vars))
        }
        Expr::List { items, .. } => {
            let mut acc: Option<Type> = Some(derived_sql_type());
            for item in items {
                acc = join_sql(acc, sql_type_of_expr(item, vars));
            }
            acc
        }
        Expr::IfElse {
            then_branch,
            else_branch,
            ..
        } => {
            let joined = join_sql(
                sql_type_of_expr(then_branch, vars),
                sql_type_of_expr(else_branch, vars),
            );
            join_sql(Some(derived_sql_type()), joined)
        }
        _ => None,
    }
}

/// The SQL-lane type of a binding's initializer (the binding parity of
/// the audit walk): RULE 2 — the literal never survives a binding, a
/// variable reference is never a literal. `Some(ty)` re-enters the
/// environment; `None` means the initializer proved nothing (the
/// honesty rule — the binding stays untyped in this lane).
pub fn binding_sql_type(value: &Expr, vars: &HashMap<String, Type>) -> Option<Type> {
    match sql_type_of_expr(value, vars) {
        // The demotion: a bound name carries the derivation marker, so
        // a later `query(name)` refuses exactly like the old syntactic
        // verdict refused the Ident shape.
        Some(Type::String) => Some(derived_sql_type()),
        other => other,
    }
}

// The SQL-lane value vocabulary (№544 step 2) walks the AST — the
// algebra and its application live side by side here.

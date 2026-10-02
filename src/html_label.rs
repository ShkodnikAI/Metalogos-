// ── №544 (gh#882) step 3 — the HTML lane's label semantics ──────────
//
//! Stage 2 of the type system (№467 canon): the label-algebra application
//! to the HTML_INJECTION audit class. A LEAF module by design — it imports
//! the AST and the stage-0 signature types, and nothing imports it back;
//! audit.rs's check_html_injection consumes it (the C4 acyclicity ratchet
//! sees no new intra-cycle edge).
//!
//! An LLM-output source (`call_llm` / `call_claude` / `reflex_generate`,
//! and every declared learnable pattern — №201, ADR-0117: the model
//! output is untrusted text) is a value of the type
//!
//!     Labeled(String, Untrusted)
//!
//! — the stage-2 label the HTML lane adds to the stage-0 vocabulary.
//! The UserInput KIND of the old machinery (form_data / json_body /
//! query_param / mcp_call — №268: their policy is NOT the HTML lane's)
//! mirrors as `Labeled(String, Private)`: distinguishable in
//! propagation, silent at this lane's sink. The sanitizers
//! (`render` / `escape_html`) strip the label exactly like the audit's
//! Sanitized semantics; `redact` passes its input through unchanged
//! (masking is not channel sanitization — №274/№326/№284).
//!
//! The lane carries TWO walks, mirroring the two questions the old
//! machinery asked — the parity demands the EXACT mirror, quirks
//! included (they are pinned behavior):
//!
//! - the BINDING walk ([`binding_label`], the mirror of the audit's
//!   `binding_taint` + the №201 direct-learnable arm): the direct
//!   source set at the top of an initializer is WIDER
//!   (call_llm_schema flags via a binding) and the propagation shape
//!   mirrors `get_expr_taint` (unbounded; IfElse branches only; the
//!   first labeled argument wins — Sanitized and unknown skip);
//! - the SINK walk ([`sink_arg_is_untrusted`], the mirror of the
//!   audit's `expr_is_llm_tainted` + `expr_is_learnable_tainted`):
//!   the direct source set is NARROWER (call_llm_schema is NOT a
//!   sink-level source), the recursion is DEPTH-BOUNDED at 3 (№295:
//!   deeper chains are honestly unseen — pinned NOT flagged) and the
//!   IfElse CONDITION participates; the direct-learnable question is
//!   a separate, non-recursive match.
//!
//! The OBSERVABLE behavior — which programs are flagged, severities,
//! messages, line resolution — is unchanged and pinned by the existing
//! suites (№123/№201/№268/№295 + the n98 golden); the MECHANISM is the
//! type. The lanes of the other checks (TAINT_PERSISTENCE,
//! CANARY_LEAK, TAINT_INTERP, the UserInput machinery) stay on the
//! taint tracker — their migration is not in №544's scope.

use std::collections::HashMap;

use crate::ast::Expr;
use crate::builtins::sig_types::{Label, Type};

/// The depth bound of the sink walk — the №295 contract (the mirror of
/// the audit's `TAINT_NESTING_MAX_DEPTH`): at depth 4+ the walk cannot
/// see and the argument is honestly unflagged intraprocedurally.
pub const TAINT_NESTING_MAX_DEPTH: usize = 3;

/// The type of an LLM-output source: model-generated text — the
/// canonical labeled type of the HTML lane.
pub fn llm_source_type() -> Type {
    Type::Labeled(Box::new(Type::String), Label::Untrusted)
}

/// The mirror of the old machinery's UserInput kind: user-form data —
/// distinguishable in propagation (the first-labeled-arg rule), silent
/// at THIS lane's sink (№268: the UserInput kinds pass respond — the
/// policy differentiation is the lattice gate's business, not the
/// legacy HTML check's).
pub fn user_input_type() -> Type {
    Type::Labeled(Box::new(Type::String), Label::Private)
}

/// True when the type carries the Untrusted label at the top (the HTML
/// lane's sink question).
pub fn is_untrusted_labeled(ty: &Option<Type>) -> bool {
    matches!(ty, Some(Type::Labeled(_, Label::Untrusted)))
}

/// The label-typed value of a binding's initializer (the binding parity
/// of the audit's `binding_taint` + the №201 direct-learnable arm):
/// the direct source arms first, then the expression walk.
pub fn binding_label(
    value: &Expr,
    vars: &HashMap<String, Type>,
    learnable: &std::collections::HashSet<String>,
) -> Option<Type> {
    if let Expr::FnCall { name, args, .. } = value {
        // №201: a direct learnable-pattern call produces LLM-output —
        // the output of call_llm outside the pattern body (ADR-0117).
        if learnable.contains(name) {
            return Some(llm_source_type());
        }
        match name.as_str() {
            // The binding-level direct source set (the mirror of the
            // audit's binding_taint arms — call_llm_schema flags via a
            // binding).
            "call_llm" | "call_claude" | "call_llm_schema" | "reflex_generate" => {
                return Some(llm_source_type());
            }
            // Sanitizers override (the Sanitized semantics: the result
            // is a known, unlabeled String).
            "render" | "escape_html" => return Some(Type::String),
            // №274 (ADR-0136): redact passes the input's state through
            // unchanged — LlmOutput/UserInput are not curable by
            // masking (the one-way-policy lift stayed with the Secret
            // lane, which left the taint machinery in step 1).
            "redact" => return redact_label(args, vars, learnable),
            // №268 (ADR-0132 D3): the user-form kinds — the UserInput
            // mirror (Private-labeled: silent here, distinguishable in
            // propagation).
            "form_data" | "json_body" | "query_param" | "mcp_call" => {
                return Some(user_input_type());
            }
            _ => {}
        }
    }
    expr_label(value, vars, learnable)
}

/// The redact mirror (redact_result_taint): the result carries the
/// input's state through unchanged in every mode.
fn redact_label(
    args: &[Expr],
    vars: &HashMap<String, Type>,
    learnable: &std::collections::HashSet<String>,
) -> Option<Type> {
    args.first().and_then(|a| expr_label(a, vars, learnable))
}

/// The propagation walk (the mirror of the audit's `get_expr_taint` —
/// the binding-side question; unbounded, IfElse branches only): an
/// LlmOutput/UserInput state (a LABELED type) wins on the first-come
/// rule; Sanitized (an unlabeled String) and unknown skip to the next
/// argument — exactly the "first non-Sanitized taint" rule.
fn expr_label(
    expr: &Expr,
    vars: &HashMap<String, Type>,
    learnable: &std::collections::HashSet<String>,
) -> Option<Type> {
    match expr {
        Expr::Ident { name, .. } => vars.get(name).cloned(),
        Expr::FnCall { name, args, .. } => {
            // Sanitizers override argument states.
            if name == "render" || name == "escape_html" {
                return Some(Type::String);
            }
            if name == "redact" {
                return redact_label(args, vars, learnable);
            }
            // The ONLY expression-level direct source of the old
            // machinery (reflex_generate; call_llm/call_claude are NOT
            // — the pinned quirk the parity keeps).
            if name == "reflex_generate" {
                return Some(llm_source_type());
            }
            // Propagate from the first labeled argument (Sanitized and
            // unknown skip — the mirror of "first non-Sanitized").
            for arg in args {
                if let Some(ty @ Type::Labeled(..)) = expr_label(arg, vars, learnable) {
                    return Some(ty);
                }
            }
            None
        }
        Expr::BinaryOp { left, right, .. } => {
            expr_label(left, vars, learnable).or_else(|| expr_label(right, vars, learnable))
        }
        Expr::FieldAccess { object: obj, .. } => expr_label(obj, vars, learnable),
        Expr::IndexAccess { index, .. } => expr_label(index, vars, learnable),
        // Literals are always clean.
        Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::StringLit { .. }
        | Expr::StructLit { .. } => None,
        // List literals propagate from their elements (the same
        // first-labeled rule).
        Expr::List { items, .. } => {
            for item in items {
                if let Some(ty @ Type::Labeled(..)) = expr_label(item, vars, learnable) {
                    return Some(ty);
                }
            }
            None
        }
        Expr::IfElse {
            then_branch,
            else_branch,
            ..
        } => expr_label(then_branch, vars, learnable)
            .or_else(|| expr_label(else_branch, vars, learnable)),
        // For other complex expressions, conservatively unknown.
        _ => None,
    }
}

/// The sink question for one respond()/respond_html() argument (the
/// mirror of the audit's `expr_is_llm_tainted(arg) ||
/// expr_is_learnable_tainted(arg)`): the bounded LLM walk PLUS the
/// direct-learnable match (№123/№201: no recursion, no chain).
pub fn sink_arg_is_untrusted(
    expr: &Expr,
    vars: &HashMap<String, Type>,
    learnable: &std::collections::HashSet<String>,
) -> bool {
    if let Expr::FnCall { name, .. } = expr {
        if learnable.contains(name) {
            return true;
        }
    }
    llm_labeled_bounded(expr, vars, 0)
}

fn llm_labeled_bounded(expr: &Expr, vars: &HashMap<String, Type>, depth: usize) -> bool {
    if depth > TAINT_NESTING_MAX_DEPTH {
        // The №295 boundary: deeper chains are honestly unseen
        // intraprocedurally (README "Known boundaries"); the
        // interprocedural check (№292) catches through summaries.
        return false;
    }
    match expr {
        Expr::Ident { name, .. } => is_untrusted_labeled(&vars.get(name).cloned()),
        Expr::FnCall { name, args, .. } => {
            // Direct LLM source — true regardless of depth. The
            // sink-level source set is NARROWER than the binding-level
            // one (call_llm_schema is not here — the pinned quirk).
            if name == "call_llm" || name == "call_claude" || name == "reflex_generate" {
                return true;
            }
            // Sanitizers lift the label at any depth.
            if name == "render" || name == "escape_html" {
                return false;
            }
            // Bounded nesting: any labeled argument chain → true.
            args.iter()
                .any(|arg| llm_labeled_bounded(arg, vars, depth + 1))
        }
        Expr::BinaryOp { left, right, .. } => {
            llm_labeled_bounded(left, vars, depth + 1)
                || llm_labeled_bounded(right, vars, depth + 1)
        }
        Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            // The sink-level walk INCLUDES the condition (the mirror of
            // expr_is_llm_tainted — the binding walk does not).
            llm_labeled_bounded(condition, vars, depth + 1)
                || llm_labeled_bounded(then_branch, vars, depth + 1)
                || llm_labeled_bounded(else_branch, vars, depth + 1)
        }
        Expr::List { items, .. } => items
            .iter()
            .any(|item| llm_labeled_bounded(item, vars, depth + 1)),
        Expr::FieldAccess { object, .. } => llm_labeled_bounded(object, vars, depth + 1),
        Expr::IndexAccess { object, index, .. } => {
            llm_labeled_bounded(object, vars, depth + 1)
                || llm_labeled_bounded(index, vars, depth + 1)
        }
        // Literals, struct literals, etc. — never untrusted directly.
        _ => false,
    }
}

// The HTML-lane value vocabulary (№544 step 3) walks the AST — the
// algebra and its application live side by side here.

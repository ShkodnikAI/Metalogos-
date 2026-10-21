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
//! №568 (gh#932, the stage-3 typing pass): the lane carries ONE walk —
//! [`walk`] — parameterized by [`WalkMode`]. The two questions the old
//! machinery asked (the parity demands the EXACT mirror, quirks
//! included — they are pinned behavior) are the mode CONTRACTS, not
//! duplicated code:
//!
//! The BINDING question ([`binding_label`], modes
//! [`WalkMode::BindingTop`] → [`WalkMode::BindingNested`], the mirror
//! of the audit's `binding_taint` + `get_expr_taint` + the №201
//! direct-learnable arm): the direct source set at the top of an
//! initializer is WIDER (call_llm_schema flags via a binding), the
//! propagation is UNBOUNDED, IfElse branches only (the condition does
//! not participate), the first labeled argument wins — Sanitized and
//! unknown skip.
//!
//! The SINK question ([`sink_arg_is_untrusted`], mode
//! [`WalkMode::Sink`], the mirror of the audit's `expr_is_llm_tainted`
//! and `expr_is_learnable_tainted`): the direct source set is NARROWER
//! (call_llm_schema is NOT a sink-level source), the recursion is
//! DEPTH-BOUNDED at 3 (№295: deeper chains are honestly unseen —
//! pinned NOT flagged), the IfElse CONDITION participates, the redact
//! arm is ABSENT (the sink walks through redact's arguments), the
//! learnable question is top-only, and only the Untrusted label counts
//! (Private/Sanitized skip — the any-argument existence question).
//!
//!   The OBSERVABLE behavior — which programs are flagged, severities,
//!   messages, line resolution — is unchanged and pinned by the existing
//!   suites (№123/№201/№268/№295 + the n98 golden); the MECHANISM is the
//!   type. The lanes of the other checks (TAINT_PERSISTENCE,
//!   CANARY_LEAK, TAINT_INTERP, the UserInput machinery) stay on the
//!   taint tracker — their migration is not in №544's scope.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::ast::Expr;
use crate::builtins::sig_types::{Label, Type};

/// №685 (gh#1252, В44): template name → the ARG indices (args[1..] of a
/// render() call) that land in RAW (`{{{ }}}`) positions — the shared
/// fact the sink walk and the audit taint layer both read. Built from
/// the program's template declarations through
/// `template_scan::raw_param_positions` (the ONE tokenizer — no
/// second parser, the S-1 lesson; the UNGATED home — render() is an
/// ungated builtin).
pub(crate) type RenderRawMap = HashMap<String, Vec<usize>>;

thread_local! {
    static RENDER_RAW: RefCell<Option<RenderRawMap>> = const { RefCell::new(None) };
}

/// №685: seed the raw-position map from the program's declarations —
/// called by the audit entries (`audit_program`, `audit_category_a`)
/// BEFORE the checks run; every walk inside the audit reads the seeded
/// fact. Re-seeding overwrites (each audit invocation is self-contained).
pub(crate) fn seed_render_raw(declarations: &[crate::ast::Declaration]) {
    let mut map: RenderRawMap = HashMap::new();
    for decl in declarations {
        if let crate::ast::Declaration::Template(t) = decl {
            let param_names: Vec<String> = t.params.iter().map(|p| p.name.clone()).collect();
            // №685: the raw triple-brace positions. №695 (ADR-0188 phase
            // 2): the safe stage `{{ param | safe }}` is a RAW position
            // too — the taint CONDUCTS through it (the escape-exempt slot
            // never sanitizes; answering «sanitized» for a safe slot
            // would recreate exactly the S-1 silent-mismatch class the
            // raw gate closed).
            let mut positions = crate::template_scan::raw_param_positions(&t.body, &param_names);
            let (safe_idx, _) = crate::template_scan::pipe_scan(&t.body, &param_names);
            for idx in safe_idx {
                if !positions.contains(&idx) {
                    positions.push(idx);
                }
            }
            // EVERY declared template is recorded — a template with NO
            // raw positions is precisely KNOWN all-escaped (the walk
            // sanitizes it); only a name ABSENT from the map is the
            // conservative unknown. Conflating the two would make fully
            // escaped templates fail closed (a false positive).
            map.insert(t.name.clone(), positions);
        }
    }
    RENDER_RAW.with(|cell| *cell.borrow_mut() = Some(map));
}

/// №685: the seeded snapshot (None = not seeded — the caller treats the
/// template as UNKNOWN, the conservative posture).
pub(crate) fn render_raw_snapshot() -> Option<RenderRawMap> {
    RENDER_RAW.with(|cell| cell.borrow().clone())
}

/// №765 (gh#1418, Wave 59 — the audit S-1b): the shared static-template
/// resolution for the render name argument. A StringLit name is the
/// literal form; a bare Ident is the name-reference form (render is a
/// name-reference builtin — semantic.rs `is_name_reference_builtin` — a
/// bare Ident there is a compile-time template reference, never a
/// variable read). A name that maps to a DECLARED template resolves its
/// raw positions; anything else (a variable holding the name — the
/// №685-E dynamic pin, an expression, an unknown name) is the
/// conservative unknown. ONE resolution, THREE consumers (the walk's
/// two modes + the label engines) — the ONE-scan discipline.
pub(crate) fn resolve_raw_positions<'a>(
    snapshot: &'a Option<RenderRawMap>,
    name_arg: Option<&Expr>,
) -> Option<&'a Vec<usize>> {
    match (snapshot, name_arg) {
        (Some(map), Some(Expr::StringLit { value, .. })) => map.get(value),
        (Some(map), Some(Expr::Ident { name, .. })) => map.get(name),
        _ => None,
    }
}

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

/// The ONE walk of the lane — the mode is the CONTRACT. Each mode
/// reproduces the exact pinned behavior of the question it serves; the
/// quirks below are contract, not duplication (№568):
#[derive(Clone, Copy, PartialEq, Eq)]
enum WalkMode {
    /// The top of a binding initializer: the WIDE direct-source set
    /// (call_llm_schema flags via a binding), the learnable check, the
    /// form-kind mirrors, the redact passthrough. With NO arm matched,
    /// the walk FALLS THROUGH to [`WalkMode::BindingNested`] on the
    /// SAME node (the mirror of `binding_label` → `expr_label`).
    BindingTop,
    /// The nested propagation: UNBOUNDED, IfElse branches only (the
    /// condition does not participate), the expression-level direct
    /// source is `reflex_generate` ONLY (the pinned quirk), the first
    /// labeled argument wins — Sanitized and unknown skip.
    BindingNested,
    /// The sink question: DEPTH-BOUNDED at №295's 3, the IfElse
    /// CONDITION participates, the direct source set NARROWER
    /// (call_llm_schema NOT a sink source), NO redact arm, the
    /// learnable question top-only, and only the Untrusted label
    /// counts — the walk skips Private/Sanitized/unknown entirely.
    Sink,
}

/// The one live walk behind both questions. Returns the label the walk
/// carries up: in the binding modes — the first LABELED value reached
/// (Untrusted or Private alike); in the sink mode — `Some` iff an
/// UNTRUSTED-labeled value is reachable within the depth bound (the
/// Private/Sanitized/unknown nodes skip to the next argument, the
/// any-argument existence question).
fn walk(
    expr: &Expr,
    vars: &HashMap<String, Type>,
    learnable: &std::collections::HashSet<String>,
    mode: WalkMode,
    depth: usize,
) -> Option<Type> {
    if mode == WalkMode::Sink && depth > TAINT_NESTING_MAX_DEPTH {
        // The №295 boundary: deeper chains are honestly unseen
        // intraprocedurally (README "Known boundaries"); the
        // interprocedural check (№292) catches through summaries.
        return None;
    }
    match expr {
        Expr::Ident { name, .. } => {
            let ty = vars.get(name).cloned();
            match mode {
                // The sink counts only the Untrusted label.
                WalkMode::Sink => ty.filter(|t| matches!(t, Type::Labeled(_, Label::Untrusted))),
                // The binding walks carry any label (the first-come rule).
                _ => ty,
            }
        }
        Expr::FnCall { name, args, .. } => {
            // №123/№201: the learnable question — the direct pattern
            // call produces LLM-output. The binding TOP and the SINK
            // TOP only: no recursion, no chain (the pinned contract).
            if (mode == WalkMode::BindingTop || (mode == WalkMode::Sink && depth == 0))
                && learnable.contains(name)
            {
                return Some(llm_source_type());
            }
            // The mode's direct-source set (the parity quirks live here).
            match mode {
                WalkMode::BindingTop => match name.as_str() {
                    // The binding-level direct source set: call_llm_schema
                    // flags via a binding.
                    "call_llm" | "call_claude" | "call_llm_schema" | "reflex_generate" => {
                        return Some(llm_source_type());
                    }
                    // №268 (ADR-0132 D3): the user-form kinds — the
                    // UserInput mirror (Private-labeled: silent here,
                    // distinguishable in propagation).
                    // N2 (gh#1211): request_header — the same lane.
                    // N5 (gh#1215, ADR-0192): the html extraction family —
                    // scraped HTML/text must not reach respond_html silent:
                    // the sink gate asks for the escape/validate step.
                    "form_data" | "json_body" | "query_param" | "request_header" | "mcp_call"
                    | "html_select" | "html_text" | "html_links" | "feed_parse"
                    | "sitemap_parse" | "xml_parse" => {
                        return Some(user_input_type());
                    }
                    _ => {}
                },
                WalkMode::BindingNested => {
                    // The ONLY expression-level direct source of the old
                    // machinery (reflex_generate; call_llm/call_claude are
                    // NOT — the pinned quirk the parity keeps).
                    if name == "reflex_generate" {
                        return Some(llm_source_type());
                    }
                }
                WalkMode::Sink => {
                    // The sink-level source set is NARROWER than the
                    // binding-level one (call_llm_schema is not here —
                    // the pinned quirk).
                    if name == "call_llm" || name == "call_claude" || name == "reflex_generate" {
                        return Some(llm_source_type());
                    }
                }
            }
            // Sanitizers override argument states (the Sanitized
            // semantics): an unlabeled String in the binding walks;
            // never untrusted at the sink.
            //
            // №685 (gh#1252, В44): the SINK walk no longer treats
            // render() as an UNCONDITIONAL lifter — the raw-insert split
            // (the S-1 audit): a raw position (`{{{ }}}`) CONDUCTS the
            // untrusted state of its argument. A statically-known
            // template resolves its raw positions from the seeded map;
            // a dynamic/unknown name walks every argument conservatively
            // (fail-closed). The BINDING walks keep the pinned
            // `Some(Type::String)` — the binding question is what the
            // variable HOLDS (Html), the sink question is what reaches
            // the page THROUGH the raw form.
            if name == "escape_html" {
                return if mode == WalkMode::Sink {
                    None
                } else {
                    Some(Type::String)
                };
            }
            if name == "render" {
                if mode != WalkMode::Sink {
                    // №765 (gh#1418, Wave 59 — the audit S-1b): the
                    // BINDING walk no longer treats render() as an
                    // unconditional String lifter either — a raw slot
                    // (`{{{ }}}` / the safe form) CONDUCTS the untrusted
                    // state of its argument into the value the variable
                    // HOLDS: the audit's let-form (`let page =
                    // render("T", j.author, j.text)` → respond_html(page))
                    // read a clean String here and the page went out
                    // unchecked. The raw positions resolve from the
                    // seeded map exactly like the sink walk; a
                    // dynamic/unknown name walks every argument
                    // conservatively (fail-closed). A labeled argument
                    // at a raw position answers
                    // `Labeled(String, Untrusted)` — the rendered page
                    // carries that value verbatim into the HTML surface
                    // (the raw-slot OUTPUT is untrusted regardless of
                    // the input label kind); only all-escaped positions
                    // keep the pinned clean `Type::String`.
                    let snapshot = render_raw_snapshot();
                    let raw_positions = resolve_raw_positions(&snapshot, args.first());
                    let is_raw = |idx: usize| -> bool {
                        raw_positions.map(|pos| pos.contains(&idx)).unwrap_or(true)
                    };
                    for (idx, arg) in args.iter().skip(1).enumerate() {
                        if !is_raw(idx) {
                            continue;
                        }
                        if walk(arg, vars, learnable, WalkMode::BindingNested, depth + 1).is_some()
                        {
                            return Some(llm_source_type());
                        }
                    }
                    return Some(Type::String);
                }
                let snapshot = render_raw_snapshot();
                let raw_positions = resolve_raw_positions(&snapshot, args.first());
                // The escape positions are skipped either way; the raw
                // positions conduct. Unknown template (no seeded fact
                // for the name — including a literal name that maps to
                // no declared template) → the conservative walk of ALL
                // arguments.
                let skip = |idx: usize| -> bool {
                    raw_positions
                        .map(|pos| !pos.contains(&idx))
                        .unwrap_or(false)
                };
                for (idx, arg) in args.iter().skip(1).enumerate() {
                    if skip(idx) {
                        continue;
                    }
                    if let Some(ty) = walk(arg, vars, learnable, mode, depth + 1) {
                        return Some(ty);
                    }
                }
                return None;
            }
            // N2 (gh#1211): ct_eq — the comparison verdict never walks as
            // untrusted (the integrity is restored; the label engine in
            // semantic.rs carries the conf nuance). The verdict is a Bool.
            if name == "ct_eq" {
                return if mode == WalkMode::Sink {
                    None
                } else {
                    Some(Type::Bool)
                };
            }
            // №274 (ADR-0136): redact passes the input's state through
            // unchanged in every mode — the binding walks mirror
            // redact_result_taint (the first argument, nested mode).
            // The sink walk has NO redact arm (the pinned quirk): the
            // sink sees through redact by walking its arguments below.
            if name == "redact" && mode != WalkMode::Sink {
                return args
                    .first()
                    .and_then(|a| walk(a, vars, learnable, WalkMode::BindingNested, depth));
            }
            // The binding TOP with no arm matched falls through to the
            // nested walk of the SAME node (the mirror of
            // `binding_label` → `expr_label`): the arguments are NOT
            // walked in the top mode.
            if mode == WalkMode::BindingTop {
                return walk(expr, vars, learnable, WalkMode::BindingNested, depth);
            }
            // Propagate: the first labeled argument wins in the nested
            // binding walk; at the sink the walk skips every non-Untrusted
            // node, so the first `Some` is the any-argument existence
            // answer (the mirror of "first non-Sanitized" / `.any()`).
            for arg in args {
                if let Some(ty) = walk(arg, vars, learnable, mode, depth + 1) {
                    return Some(ty);
                }
            }
            None
        }
        Expr::BinaryOp { left, right, .. } => walk(left, vars, learnable, mode, depth + 1)
            .or_else(|| walk(right, vars, learnable, mode, depth + 1)),
        Expr::FieldAccess { object: obj, .. } => walk(obj, vars, learnable, mode, depth + 1),
        Expr::IndexAccess { object, index, .. } => {
            if mode == WalkMode::Sink {
                // The sink walk sees the object AND the index.
                walk(object, vars, learnable, mode, depth + 1)
                    .or_else(|| walk(index, vars, learnable, mode, depth + 1))
            } else {
                // The binding walk keeps its pinned shape: index only.
                walk(index, vars, learnable, mode, depth)
            }
        }
        // Literals are always clean.
        Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::StringLit { .. }
        | Expr::StructLit { .. } => None,
        // List literals propagate from their elements (the same
        // first-labeled rule).
        Expr::List { items, .. } => {
            for item in items {
                if let Some(ty) = walk(item, vars, learnable, mode, depth + 1) {
                    return Some(ty);
                }
            }
            None
        }
        Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            if mode == WalkMode::Sink {
                // The sink-level walk INCLUDES the condition (the mirror
                // of expr_is_llm_tainted — the binding walk does not).
                walk(condition, vars, learnable, mode, depth + 1)
                    .or_else(|| walk(then_branch, vars, learnable, mode, depth + 1))
                    .or_else(|| walk(else_branch, vars, learnable, mode, depth + 1))
            } else {
                walk(then_branch, vars, learnable, mode, depth)
                    .or_else(|| walk(else_branch, vars, learnable, mode, depth))
            }
        }
        // For other complex expressions, conservatively unknown.
        _ => None,
    }
}

/// The label-typed value of a binding's initializer (the binding parity
/// of the audit's `binding_taint` + the №201 direct-learnable arm):
/// the direct source arms first, then the expression walk.
pub fn binding_label(
    value: &Expr,
    vars: &HashMap<String, Type>,
    learnable: &std::collections::HashSet<String>,
) -> Option<Type> {
    walk(value, vars, learnable, WalkMode::BindingTop, 0)
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
    matches!(
        walk(expr, vars, learnable, WalkMode::Sink, 0),
        Some(Type::Labeled(_, Label::Untrusted))
    )
}

// The HTML-lane value vocabulary (№544 step 3) walks the AST — the
// algebra and its application live side by side here. №568: the walk is
// ONE — the modes carry the parity quirks as contracts.

// ── №695 (gh#1274, В46): the safe slot is a RAW position ──────────────
//
// The seeded render-raw map must carry the recognized safe stage
// `{{ param | safe }}` as a RAW (conductor) position — the escape-exempt
// slot never sanitizes. Answering «sanitized» for a safe slot would
// recreate exactly the S-1 silent-mismatch class the №685 raw gate
// closed. The map is pub(crate), so this contract is pinned by a UNIT
// test here (the integration tests cannot reach the seeder).
#[cfg(test)]
mod n695_safe_slot_taint_tests {
    use super::*;

    #[test]
    fn safe_slot_is_a_raw_position_in_the_seeded_map() {
        let src = "template T(h: Html) -> Html {\n  <p>{{ h | safe }}{{ h }}</p>\n}";
        let declarations = crate::parser::parse(src).expect("parse");
        seed_render_raw(&declarations);
        let snap = render_raw_snapshot().expect("seeded");
        let positions = snap.get("T").expect("the template is in the map");
        assert!(
            positions.contains(&0),
            "the safe slot must conduct (position 0 in the raw map), got: {positions:?}"
        );
        // The control: a template with NO raw/safe positions stays in the
        // map as the KNOWN all-escaped fact (empty vec, not absent).
        let src2 = "template E(s: String) -> Html {\n  <p>{{ s }}</p>\n}";
        let declarations2 = crate::parser::parse(src2).expect("parse");
        seed_render_raw(&declarations2);
        let snap2 = render_raw_snapshot().expect("seeded");
        let positions2 = snap2.get("E").expect("the escaped template is recorded");
        assert!(
            positions2.is_empty(),
            "the all-escaped template must record zero raw positions, got: {positions2:?}"
        );
    }
}

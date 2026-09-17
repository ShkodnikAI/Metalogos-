// ── Semantic analysis for METALOGOS ──────────────────────────────
// Validates declarations without execution. Reports errors and warnings.
// Phase 6+: Enforces opaque type constraints (Html, Query, Secret, etc.)
//
// Наряд №165: every diagnostic carries the AST `Span` of the offending
// node. The previous `Vec<String>` API was a flat list — diagnostics
// lost the position that наряд №121 already added to every AST node.
// `SpannedError` restores the link. Callers that previously iterated
// `result.errors.iter()` as `&String` now see `&SpannedError` and must
// read `.message` (or `.span` for LSP / programmatic consumers).

use crate::ast::*;
use crate::audit::{audit_category_a, Severity};
use crate::builtins_classification::{classify, Reversibility, Role};
use crate::labels::{legacy_taint_label, Label};
use std::collections::{BTreeMap, HashMap, HashSet};

/// A diagnostic with the AST `Span` of the offending node.
///
/// `span` is the source position of the AST node that triggered the
/// diagnostic — never computed, always taken from the node via
/// `decl.span()` / `expr.span()`. If a node genuinely has no span
/// (synthetic nodes, programmatic constructions), `Span::unknown()`
/// (all-zero) is used and consumers should treat `start_line == 0` as
/// "no position available".
#[derive(Debug, Clone)]
pub struct SpannedError {
    pub message: String,
    pub span: Span,
}

impl SpannedError {
    /// Build a `SpannedError` from a message and an explicit span.
    pub fn at(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }

    /// Build a `SpannedError` from a message and a declaration's span.
    /// Mirrors the legacy `with_line_prefix` shape — but instead of
    /// prefixing the message with the line number, it carries the span
    /// structurally so LSP / programmatic consumers can read it
    /// directly. The message itself stays clean.
    pub fn at_decl(decl: &Declaration, message: impl Into<String>) -> Self {
        Self::at(message, decl.span().clone())
    }

    /// Build a `SpannedError` from a message and an expression's span.
    pub fn at_expr(expr: &Expr, message: impl Into<String>) -> Self {
        Self::at(message, expr.span().clone())
    }

    /// Build a `SpannedError` with no source position. Used for
    /// diagnostics whose originating node is not in the AST (e.g.
    /// audit-category findings that come back with only a line number).
    /// If `line > 0`, the span is constructed to point at that line so
    /// the LSP can still highlight something; otherwise the span is
    /// `Span::unknown()`.
    pub fn at_line(message: impl Into<String>, line: usize) -> Self {
        if line > 0 {
            Self::at(message, Span::new(line as u32, 0, line as u32, 0))
        } else {
            Self::at(message, Span::unknown())
        }
    }
}

/// Result of semantic analysis: errors prevent execution, warnings are advisory.
///
/// Наряд №165: `errors` and `warnings` now carry `SpannedError` (with
/// `Span`) instead of bare `String`. This is an internal breaking
/// change — every caller inside the crate has been updated to access
/// `.message` for display and `.span` for LSP / programmatic use.
#[derive(Debug, Clone, Default)]
pub struct AnalysisResult {
    pub errors: Vec<SpannedError>,
    pub warnings: Vec<SpannedError>,
}

impl AnalysisResult {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    /// Format analysis result for display.
    ///
    /// Preserves the human-readable form previously produced by
    /// `with_line_prefix` (so `mlog check` output stays stable): each
    /// diagnostic is shown as `строка N: <message>` when a span is
    /// present, or just `<message>` when the span is unknown.
    pub fn format(&self) -> String {
        fn render(err: &SpannedError) -> String {
            if err.span.start_line > 0 {
                format!("строка {}: {}", err.span.start_line, err.message)
            } else {
                err.message.clone()
            }
        }
        let mut lines = Vec::new();
        if !self.errors.is_empty() {
            let n = self.errors.len();
            if n == 1 {
                lines.push("1 error:".to_string());
            } else {
                lines.push(format!("{} errors:", n));
            }
            for (i, e) in self.errors.iter().enumerate() {
                lines.push(format!("  {}: {}", i + 1, render(e)));
            }
        }
        if !self.warnings.is_empty() {
            let n = self.warnings.len();
            if n == 1 {
                lines.push("1 warning:".to_string());
            } else {
                lines.push(format!("{} warnings:", n));
            }
            for (i, w) in self.warnings.iter().enumerate() {
                lines.push(format!("  {}: {}", i + 1, render(w)));
            }
        }
        if self.errors.is_empty() && self.warnings.is_empty() {
            lines.push("OK: no issues found.".to_string());
        }
        lines.join("\n")
    }
}

/// Legacy helper preserved for backward compatibility with call sites
/// that already pass `(decl, format!(...))`. Now returns a `SpannedError`
/// instead of a `String` — same ergonomic shape, but carries the span.
fn with_line_prefix(decl: &Declaration, msg: String) -> SpannedError {
    SpannedError::at_decl(decl, msg)
}

/// Valid middleware names for mlogserver blocks.
const VALID_MIDDLEWARE: &[&str] = &["session", "csrf", "security_headers", "rate_limit", "cors"];

/// Valid HTTP methods for route declarations.
const VALID_METHODS: &[&str] = &["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

// ── Statement-level label inference (Наряд №323, ADR-0154 Appendix A) ──
//
// Static propagation of labels `(conf, integrity, consent-scope)` through
// pattern bodies: every statement kind has a contract (input reading,
// output label, side effects, merge rule) recorded in ADR-0154 Appendix A
// and REFERENCE §2.2. Join at merge points is componentwise:
// if/else-if/else branches, match arms, loop exits.
//
// Sources and sanitizers reuse the audit.rs vocabulary (binding_taint),
// projected onto the lattice via `labels::legacy_taint_label` — the bridge
// table of ADR-0154 §5. This is the machinery №325's sink-gate reads.

/// Bounded-fixpoint pass cap for `while` bodies. The conf lattice has
/// height 4 (`public < consented < private < poisoned`); monotone joins
/// stabilize any var→var loop-carried chain within that many passes —
/// 8 = height × 2 safety factor. Deterministic, always terminates.
const LABEL_FIXPOINT_MAX_PASSES: usize = 8;

/// Result of the statement-level label inference for one pattern.
#[derive(Debug, Clone)]
pub struct LabelInference {
    /// Final label of every variable bound in the pattern body (params
    /// included), after all merge joins.
    pub var_labels: BTreeMap<String, Label>,
    /// Componentwise join of every `Return` value label and `ExprStmt`
    /// result label — the pattern's output label.
    pub output_label: Label,
}

/// Source/sanitizer vocabulary — mirrors audit.rs `binding_taint`, keyed
/// the same way, projected via the ADR-0154 §5 table. `None` = not a
/// source (the caller falls back to argument propagation).
fn label_source(fn_name: &str, args: &[Expr], env: &BTreeMap<String, Label>) -> Option<Label> {
    // Static kind names only — the ADR-0154 §5 table covers them (pinned
    // by the exhaustiveness test in audit.rs); the fallback never fires.
    let kind_label = |kind: &str| legacy_taint_label(kind).unwrap_or_else(Label::bottom);
    match fn_name {
        // Secret sources.
        "env" | "secret" => Some(kind_label("Secret")),
        // LLM-output sources (model output is untrusted — ADR-0117).
        "call_llm" | "call_claude" | "call_llm_schema" | "reflex_generate" => {
            Some(kind_label("LlmOutput"))
        }
        // Untrusted-input sources (№268: MCP tool output reuses UserInput).
        "form_data" | "json_body" | "query_param" | "mcp_call" => Some(kind_label("UserInput")),
        // №325: network ingress and file ingress are untrusted sources —
        // the sink-clearance gate needs their labels to attribute
        // UNTRUSTED_EGRESS_* classes (ADR-0161 §3).
        "http_get" | "read_file" => Some(kind_label("UserInput")),
        // Sanitizers restore trust.
        "render" | "escape_html" => Some(kind_label("Sanitized")),
        // №274 (ADR-0136): redact masks secrets — the ONLY downward move
        // for `private` until №326 formalizes redact/declassify. Semantics:
        // mode "secrets"/"all" maps private → public/trusted; every other
        // label passes through UNCHANGED — quarantine (`poisoned`) is NOT
        // curable by redact (a channel is not a secret; ADR-0136 D2).
        "redact" => {
            let input = args
                .first()
                .map(|a| expr_label(a, env))
                .unwrap_or_else(Label::bottom);
            // №326: the policy is a VALUE — its target conf comes from
            // the registry (the single source of truth shared with the
            // runtime). Unknown/dynamic policies pass the input through
            // (conservative — no silent downward moves).
            let target_conf = args.get(1).and_then(|p| match p {
                Expr::StringLit { value, .. } => {
                    crate::builtins::string::redact_policy(value).map(|pol| pol.target_conf)
                }
                _ => None,
            });
            match target_conf {
                // One-way: the data is destroyed — the result is a
                // compiler-derived value (bottom: public AND trusted).
                // Integrity is restored too (№327: hash_only decisions
                // are legal). QUARANTINE EXCEPTION: poisoned is not
                // curable by any policy (ADR-0154 §2.1 / §10) — the
                // channel is not the data.
                Some("public") => {
                    if input.conf == crate::labels::Conf::Poisoned {
                        Some(input)
                    } else {
                        Some(Label::bottom())
                    }
                }
                _ => Some(input),
            }
        }
        // №335 (spec §7.2 v2): consent_grant extends the value's
        // consent-scope set by the granted scope. Non-literal scope
        // cannot be named statically — conservative no-extension (the
        // redact dynamic-policy posture); the ledger still records the
        // runtime grant.
        "consent_grant" => {
            let input = args
                .first()
                .map(|a| expr_label(a, env))
                .unwrap_or_else(Label::bottom);
            match args.get(1) {
                Some(Expr::StringLit { value, .. }) => {
                    let consent = input
                        .consent
                        .clone()
                        .meet(crate::labels::ConsentScope::from_scopes([value.as_str()]));
                    Some(Label {
                        conf: input.conf,
                        integrity: input.integrity,
                        consent,
                    })
                }
                _ => Some(input),
            }
        }
        // №335: consent_revoke — the FLAT cascade entry point. The
        // revoked value (and every value derived from it) carries the
        // QUARANTINE label: poison is ABSORBING in the lattice
        // (ADR-0154 §2.1) — join with anything stays poisoned, so the
        // cascade is the lattice's own semantics, not a separate
        // analysis. Quarantine clears the consent scope too.
        "consent_revoke" => Some(Label {
            conf: crate::labels::Conf::Poisoned,
            integrity: crate::labels::Integrity::Untrusted,
            consent: Default::default(),
        }),
        _ => None,
    }
}

/// Label of an expression under environment `env`. Join is componentwise;
/// literals and unknown identifiers are `bottom` (ADR-0154 Appendix A:
/// unannotated params and unresolved names start open — the sink-gate
/// №325 reads these labels, it does not trust them).
fn expr_label(expr: &Expr, env: &BTreeMap<String, Label>) -> Label {
    match expr {
        Expr::StringLit { .. } | Expr::FloatLit { .. } | Expr::BoolLit { .. } => Label::bottom(),
        // №332 (ADR-0164): HandleSource/ProvBind labels are resolved at
        // the LetBinding/Assign insertion points (origin declarations are
        // not in scope here). Legal constructions only appear there —
        // this arm is the conservative fallback (bottom) for the general
        // walker; the origin pass refuses illegal positions loudly.
        Expr::HandleSource { .. } => Label::bottom(),
        Expr::ProvBind { inner, .. } => expr_label(inner, env),
        Expr::Ident { name, .. } => env.get(name).cloned().unwrap_or_else(Label::bottom),
        Expr::FieldAccess { object, .. } => expr_label(object, env),
        Expr::FnCall { name, args, .. } => {
            if let Some(l) = label_source(name, args, env) {
                return l;
            }
            // Data flows through ordinary functions: join of the arguments.
            let mut acc = Label::bottom();
            for a in args {
                acc = acc.join(&expr_label(a, env));
            }
            acc
        }
        // Qualified calls (module functions): no source knowledge at this
        // slice — argument propagation only (recorded boundary).
        Expr::QualifiedCall { args, .. } => {
            let mut acc = Label::bottom();
            for a in args {
                acc = acc.join(&expr_label(a, env));
            }
            acc
        }
        Expr::BinaryOp { left, right, .. } => expr_label(left, env).join(&expr_label(right, env)),
        // Conditions do not taint values; branches do.
        Expr::IfElse {
            then_branch,
            else_branch,
            ..
        } => expr_label(then_branch, env).join(&expr_label(else_branch, env)),
        Expr::List { items, .. } => {
            let mut acc = Label::bottom();
            for item in items {
                acc = acc.join(&expr_label(item, env));
            }
            acc
        }
        // `list[index]` yields an ELEMENT of the object; the list label is
        // already the join of its elements, so the object label is the
        // sound answer (the index selects, it does not contribute).
        Expr::IndexAccess { object, .. } => expr_label(object, env),
        Expr::StructLit { fields, .. } => {
            let mut acc = Label::bottom();
            for v in fields.values() {
                acc = acc.join(&expr_label(v, env));
            }
            acc
        }
        // Block-expression branches: join of every expression label that
        // appears in the branch bodies (approximation of the block's value
        // — recorded in Appendix A).
        Expr::BlockIfElse {
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            let mut acc = block_expr_label(then_body, env);
            for (_, body) in else_ifs {
                acc = acc.join(&block_expr_label(body, env));
            }
            if let Some(body) = else_body {
                acc = acc.join(&block_expr_label(body, env));
            }
            acc
        }
        // №369: match-as-expression — the value carries the join of the
        // scrutinee's label (control dependence, REFERENCE §labels: the
        // scrutinee's label joins every variable assigned in any arm) with
        // every arm body's expression labels; else body joins too.
        Expr::MatchExpr {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            let mut acc = expr_label(scrutinee, env);
            for arm in arms {
                acc = acc.join(&block_expr_label(arm.body(), env));
            }
            if let Some(eb) = else_body {
                acc = acc.join(&block_expr_label(eb, env));
            }
            acc
        }
        // `try expr` returns the value or Unit on error — the value label
        // is an upper bound, keep the inner label.
        Expr::Try { expr, .. } => expr_label(expr, env),
    }
}

/// Join of every expression label appearing (top-level-ish) in a block —
/// used for block-expression value approximation only.
fn block_expr_label(stmts: &[Statement], env: &BTreeMap<String, Label>) -> Label {
    let mut acc = Label::bottom();
    for st in stmts {
        match st {
            Statement::LetBinding { value, .. } | Statement::Assign { value, .. } => {
                acc = acc.join(&expr_label(value, env));
            }
            Statement::Return { value, .. } | Statement::ExprStmt { expr: value, .. } => {
                acc = acc.join(&expr_label(value, env));
            }
            Statement::IfThen { body, .. } => {
                acc = acc.join(&block_expr_label(body, env));
            }
            Statement::IfElseBlock {
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                acc = acc.join(&block_expr_label(then_body, env));
                for (_, body) in else_ifs {
                    acc = acc.join(&block_expr_label(body, env));
                }
                if let Some(body) = else_body {
                    acc = acc.join(&block_expr_label(body, env));
                }
            }
            _ => {}
        }
    }
    acc
}

/// Merge rule shared by all merge points: per-variable componentwise join
/// of the entry environment and every branch environment (a branch that
/// did not assign the variable contributes the entry label — this is what
/// makes one-sided assignment conservative).
fn merge_envs(
    entry: &BTreeMap<String, Label>,
    branches: &[&BTreeMap<String, Label>],
) -> BTreeMap<String, Label> {
    let mut names: Vec<&String> = entry.keys().collect();
    for b in branches {
        names.extend(b.keys());
    }
    let mut out = BTreeMap::new();
    for name in names {
        let mut acc = entry.get(name).cloned().unwrap_or_else(Label::bottom);
        for b in branches {
            acc = acc.join(&b.get(name).cloned().unwrap_or_else(Label::bottom));
        }
        out.insert(name.clone(), acc);
    }
    out
}

/// Structurally collect every variable ASSIGNED or LET-BOUND in a
/// statement sequence (recursively through nested blocks) — used by the
/// Match scrutinee rule, which must trigger on assignment SHAPE, not on
/// label changes (a branch can assign the same label it inherited).
fn collect_assigned_vars(stmts: &[Statement], out: &mut std::collections::HashSet<String>) {
    for st in stmts {
        match st {
            Statement::LetBinding { name, .. } | Statement::Assign { name, .. } => {
                out.insert(name.clone());
            }
            Statement::Each { variable, body, .. } => {
                out.insert(variable.clone());
                collect_assigned_vars(body, out);
            }
            Statement::EachWithIndex {
                index_var,
                item_var,
                body,
                ..
            } => {
                out.insert(index_var.clone());
                out.insert(item_var.clone());
                collect_assigned_vars(body, out);
            }
            Statement::While { body, .. } => collect_assigned_vars(body, out),
            Statement::IfThen { body, .. } => collect_assigned_vars(body, out),
            Statement::IfElseBlock {
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                collect_assigned_vars(then_body, out);
                for (_, body) in else_ifs {
                    collect_assigned_vars(body, out);
                }
                if let Some(body) = else_body {
                    collect_assigned_vars(body, out);
                }
            }
            Statement::Match {
                arms, else_body, ..
            } => {
                for arm in arms {
                    let body = match arm {
                        MatchArm::Exact(_, b)
                        | MatchArm::StartsWith(_, b)
                        | MatchArm::Contains(_, b)
                        | MatchArm::Compare(_, _, b) => b,
                    };
                    collect_assigned_vars(body, out);
                }
                if let Some(body) = else_body {
                    collect_assigned_vars(body, out);
                }
            }
            _ => {}
        }
    }
}

/// Infer one statement sequence, mutating `env`; value-producing
/// statements (Return, ExprStmt) join the pattern `output`.
fn infer_block(stmts: &[Statement], env: &mut BTreeMap<String, Label>, output: &mut Label) {
    for st in stmts {
        infer_stmt(st, env, output);
    }
}

fn infer_stmt(st: &Statement, env: &mut BTreeMap<String, Label>, output: &mut Label) {
    match st {
        // LetBinding/Assign — the label comes from the RHS (Assign replaces:
        // reassignment to a safe value lowers the label in straight-line
        // code, mirroring the TaintTracker untaint semantics; merge points
        // re-add the conservatism).
        Statement::LetBinding { name, value, .. } => {
            let l = expr_label(value, env);
            env.insert(name.clone(), l);
        }
        Statement::Assign { name, value, .. } => {
            let l = expr_label(value, env);
            env.insert(name.clone(), l);
        }
        // Each: the iterator takes the ITERABLE's label; the body cannot
        // raise it (restored after the body). Loop exit: join of the entry
        // and post-body environments for every other variable.
        Statement::Each {
            variable,
            iterable,
            body,
            ..
        } => {
            let it_label = expr_label(iterable, env);
            env.insert(variable.clone(), it_label.clone());
            let entry = env.clone();
            infer_block(body, env, output);
            env.insert(variable.clone(), it_label);
            *env = merge_envs(&entry, &[&*env]);
        }
        Statement::EachWithIndex {
            index_var,
            item_var,
            iterable,
            body,
            ..
        } => {
            let it_label = expr_label(iterable, env);
            env.insert(item_var.clone(), it_label.clone());
            // The index is a position, not data: bottom (Appendix A).
            env.insert(index_var.clone(), Label::bottom());
            let entry = env.clone();
            infer_block(body, env, output);
            env.insert(item_var.clone(), it_label);
            env.insert(index_var.clone(), Label::bottom());
            *env = merge_envs(&entry, &[&*env]);
        }
        // While — bounded fixpoint: the body runs until the environment
        // stabilizes (≤ 8 passes). Monotone joins on a finite lattice make
        // this the exact fixpoint; the cap is a termination guard.
        // Condition labels are ignored (conditions do not taint values).
        Statement::While { body, .. } => {
            for _ in 0..LABEL_FIXPOINT_MAX_PASSES {
                let before = env.clone();
                infer_block(body, env, output);
                // join so loop-carried growth accumulates across passes
                for (k, v) in env.iter_mut() {
                    let b = before.get(k).cloned().unwrap_or_else(Label::bottom);
                    *v = b.join(v);
                }
                if *env == before {
                    break;
                }
            }
        }
        // If/else-if/else: every branch is inferred from the entry env;
        // merge is the componentwise join over all branches.
        Statement::IfElseBlock {
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            let entry = env.clone();
            infer_block(then_body, env, output);
            let then_env = env.clone();
            let mut branch_envs: Vec<BTreeMap<String, Label>> = vec![then_env];
            for (_, body) in else_ifs {
                let mut e = entry.clone();
                infer_block(body, &mut e, output);
                branch_envs.push(e);
            }
            if let Some(body) = else_body {
                let mut e = entry.clone();
                infer_block(body, &mut e, output);
                branch_envs.push(e);
            }
            let refs: Vec<&BTreeMap<String, Label>> = branch_envs.iter().collect();
            *env = merge_envs(&entry, &refs);
        }
        // Single-branch if: merge with an implicit empty else.
        Statement::IfThen { body, .. } => {
            let entry = env.clone();
            infer_block(body, env, output);
            *env = merge_envs(&entry, &[&*env]);
        }
        // Return/ExprStmt — the result label joins the pattern output.
        Statement::Return { value, .. } => {
            let l = expr_label(value, env);
            *output = output.join(&l);
        }
        Statement::ExprStmt { expr, .. } => {
            let l = expr_label(expr, env);
            *output = output.join(&l);
        }
        // Match: join over arms; the scrutinee's label additionally joins
        // every variable ASSIGNED in any arm — control dependence on the
        // scrutinee (decisions derived from private data taint outcomes).
        Statement::Match {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            let scrut = expr_label(scrutinee, env);
            let entry = env.clone();
            let mut branch_envs: Vec<BTreeMap<String, Label>> = Vec::new();
            let mut assigned: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut run_branch =
                |body: &Vec<Statement>,
                 branch_envs: &mut Vec<_>,
                 assigned: &mut std::collections::HashSet<String>,
                 entry: &BTreeMap<String, Label>| {
                    collect_assigned_vars(body, assigned);
                    let mut e = entry.clone();
                    let mut out_tmp = Label::bottom();
                    infer_block(body, &mut e, &mut out_tmp);
                    // NOTE: arm outputs join the caller's output too (a Return
                    // inside an arm is still a pattern exit).
                    *output = output.join(&out_tmp);
                    branch_envs.push(e);
                };
            for arm in arms {
                let body = match arm {
                    MatchArm::Exact(_, b)
                    | MatchArm::StartsWith(_, b)
                    | MatchArm::Contains(_, b)
                    | MatchArm::Compare(_, _, b) => b,
                };
                run_branch(body, &mut branch_envs, &mut assigned, &entry);
            }
            if let Some(body) = else_body {
                run_branch(body, &mut branch_envs, &mut assigned, &entry);
            }
            let refs: Vec<&BTreeMap<String, Label>> = branch_envs.iter().collect();
            let mut merged = merge_envs(&entry, &refs);
            for k in &assigned {
                // The variable is structurally assigned in some branch, so
                // every branch env (and the merge union) carries it.
                if let Some(m) = merged.get_mut(k) {
                    *m = m.join(&scrut);
                }
            }
            *env = merged;
        }
        // Loop control: no label effect, no merge contribution (Appendix A).
        Statement::Break | Statement::Continue => {}
        // Memory side-effect statements: the payload label does not enter
        // the value flow here; persistence gating is №325
        // (TAINT_PERSISTENCE class in the leak-suite vocabulary).
        Statement::Memorize(_) | Statement::Forget(_) | Statement::Relate(_) => {}
    }
}

/// Run the statement-level label inference over one pattern (Наряд №323).
/// Parameters: annotated → parsed label; unannotated → `bottom`. The
/// result is what №325's sink-gate will read; see ADR-0154 Appendix A
/// for the per-statement contracts and REFERENCE §2.2 for the table.
pub fn infer_pattern_labels(pattern: &PatternDecl) -> LabelInference {
    let mut env: BTreeMap<String, Label> = BTreeMap::new();
    for prm in &pattern.params {
        let l = match &prm.label {
            Some(ann) => Label::parse(&ann.raw).unwrap_or_else(|_| Label::bottom()),
            None => Label::bottom(),
        };
        env.insert(prm.name.clone(), l);
    }
    let mut output = Label::bottom();
    infer_block(&pattern.body, &mut env, &mut output);
    LabelInference {
        var_labels: env,
        output_label: output,
    }
}

// ── Label annotation validation (Наряд №322, ADR-0154) ───────────

/// Validate one label annotation, reporting a parse failure with the
/// annotation's span. Message follows the existing semantic convention
/// (context first, reason last) — e.g.
/// `label annotation '<private, bogus>' on parameter 's' of pattern 'p': unknown label word 'bogus'`.
fn validate_label_ann(ann: &LabelAnn, context: &str, errors: &mut Vec<SpannedError>) {
    if let Err(e) = crate::labels::Label::parse(&ann.raw) {
        errors.push(SpannedError::at(
            format!("label annotation '<{}>' on {}: {}", ann.raw, context, e),
            ann.span.clone(),
        ));
    }
}

/// Walk a declaration and validate every label annotation it carries.
///
/// Grammar-restricted positions (Наряд №322): pattern / learnable /
/// template / tool-method parameters, entity-type fields, and the type
/// position of entity record/simple declarations. Everywhere else a
/// `<...>` after a type name remains a parse error — annotations cannot
/// appear where the label system does not see them.
fn validate_decl_labels(decl: &Declaration, errors: &mut Vec<SpannedError>) {
    match decl {
        Declaration::Pattern(p) => {
            for prm in &p.params {
                if let Some(ann) = &prm.label {
                    validate_label_ann(
                        ann,
                        &format!("parameter '{}' of pattern '{}'", prm.name, p.name),
                        errors,
                    );
                }
            }
        }
        Declaration::LearnablePattern(lp) => {
            for prm in &lp.params {
                if let Some(ann) = &prm.label {
                    validate_label_ann(
                        ann,
                        &format!(
                            "parameter '{}' of learnable pattern '{}'",
                            prm.name, lp.name
                        ),
                        errors,
                    );
                }
            }
        }
        Declaration::Template(t) => {
            for prm in &t.params {
                if let Some(ann) = &prm.label {
                    validate_label_ann(
                        ann,
                        &format!("parameter '{}' of template '{}'", prm.name, t.name),
                        errors,
                    );
                }
            }
        }
        Declaration::Tool(t) => {
            for m in &t.methods {
                for prm in &m.params {
                    if let Some(ann) = &prm.label {
                        validate_label_ann(
                            ann,
                            &format!(
                                "parameter '{}' of tool method '{}.{}'",
                                prm.name, t.name, m.name
                            ),
                            errors,
                        );
                    }
                }
            }
        }
        Declaration::EntityType(e) => {
            for f in &e.fields {
                if let Some(ann) = &f.label {
                    validate_label_ann(
                        ann,
                        &format!("field '{}' of entity type '{}'", f.name, e.name),
                        errors,
                    );
                }
            }
        }
        Declaration::EntityRecord(e) => {
            if let Some(ann) = &e.label {
                validate_label_ann(ann, &format!("entity '{}'", e.name), errors);
            }
        }
        Declaration::EntitySimple(e) => {
            if let Some(ann) = &e.label {
                validate_label_ann(ann, &format!("entity '{}'", e.name), errors);
            }
        }
        _ => {}
    }
}

// ── Effect trail (Наряд №324, ADR-0154 §9) ───────────────────────────

/// Parse the declared effect-trail words (`"io, audit"`) into a set.
/// Errors (unknown word / duplicate) are loud; `⟨⟩` (empty raw) is the
/// zero-effect declaration.
fn parse_effect_set(ann: &EffectAnn) -> Result<EffectSet, String> {
    let raw = ann.raw.trim();
    if raw.is_empty() {
        return Ok(EffectSet::new());
    }
    let mut set = EffectSet::new();
    for word in raw.split(',') {
        let w = word.trim();
        match Effect::parse_word(w) {
            Some(e) => {
                if !set.insert(e) {
                    return Err(format!("duplicate effect word '{w}'"));
                }
            }
            None => return Err(format!("unknown effect word '{w}'")),
        }
    }
    Ok(set)
}

/// Validate one effect trail, reporting a bad word with the trail's
/// span (same convention as `validate_label_ann`).
fn validate_effect_ann(ann: &EffectAnn, context: &str, errors: &mut Vec<SpannedError>) {
    if let Err(e) = parse_effect_set(ann) {
        errors.push(SpannedError::at(
            format!("effect trail '⟨{}⟩' on {}: {}", ann.raw, context, e),
            ann.span.clone(),
        ));
    }
}

// ── Media handle opacity (Наряд №331, ADR-0114 / ADR-0162 §2.5) ──────
//
// Media handles (`Value::Media`: Image/Audio/VideoFrame/VideoSegment)
// are opaque: bytes NEVER live in `Value`, so any field access on a
// media-typed expression is a COMPILE error (loud, with span) — bytes
// are reachable only through the sanctioned materialization sink
// (`media_save`, №325-gated). The language has no other byte-extraction
// syntax on handles; this pass closes the syntactic surface there is.

/// Builtins that PRODUCE a media handle (the four per-type stores +
/// `media_retain`, which returns the same handle).
fn is_media_producing_builtin(name: &str) -> bool {
    matches!(
        name,
        "media_store_image"
            | "media_store_audio"
            | "media_store_video_frame"
            | "media_store_video_segment"
            | "media_retain"
    )
}

/// `true` when the expression's value is a media handle by direct
/// construction (a producing builtin call).
fn is_media_binding_expr(value: &Expr) -> bool {
    match value {
        Expr::FnCall { name, .. } => is_media_producing_builtin(name),
        // №332 (ADR-0164 §7.4): the perception constructions produce
        // media handles too — a bound Lift (`from <origin>
        // media_store_*(...)`) and a source capture (`source <origin>`)
        // are opaque handles like any other.
        Expr::ProvBind { inner, .. } => {
            matches!(inner.as_ref(), Expr::FnCall { name, .. } if is_media_producing_builtin(name))
        }
        Expr::HandleSource { .. } => true,
        _ => false,
    }
}

/// Is this expression's static bottom a media-typed binding?
fn media_typed_object(object: &Expr, media_vars: &std::collections::HashSet<String>) -> bool {
    match object {
        Expr::Ident { name, .. } => media_vars.contains(name),
        Expr::FnCall { name, .. } => is_media_producing_builtin(name),
        // №332: perception constructions are media-typed bottoms too.
        Expr::HandleSource { .. } => true,
        Expr::ProvBind { inner, .. } => {
            matches!(inner.as_ref(), Expr::FnCall { name, .. } if is_media_producing_builtin(name))
        }
        _ => false,
    }
}

/// Walk one expression for field accesses on media-typed objects and
/// for further media bindings (recursively).
fn check_media_expr(
    expr: &Expr,
    media_vars: &mut std::collections::HashSet<String>,
    container: &str,
    violations: &mut Vec<MediaOpacityViolation>,
) {
    match expr {
        Expr::FieldAccess {
            object,
            field,
            span,
        } => {
            if media_typed_object(object, media_vars) {
                violations.push(MediaOpacityViolation {
                    container: container.to_string(),
                    field: field.clone(),
                    span: span.clone(),
                });
            } else {
                check_media_expr(object, media_vars, container, violations);
            }
        }
        Expr::FnCall { name, args, .. } => {
            // The call NAME may be a media binding (checked by the caller
            // when binding); the arguments still need the field-access walk.
            let _ = name;
            for a in args {
                check_media_expr(a, media_vars, container, violations);
            }
        }
        Expr::QualifiedCall { args, .. } => {
            for a in args {
                check_media_expr(a, media_vars, container, violations);
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            check_media_expr(left, media_vars, container, violations);
            check_media_expr(right, media_vars, container, violations);
        }
        Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            check_media_expr(condition, media_vars, container, violations);
            check_media_expr(then_branch, media_vars, container, violations);
            check_media_expr(else_branch, media_vars, container, violations);
        }
        Expr::List { items, .. } => {
            for i in items {
                check_media_expr(i, media_vars, container, violations);
            }
        }
        Expr::IndexAccess { object, index, .. } => {
            check_media_expr(object, media_vars, container, violations);
            check_media_expr(index, media_vars, container, violations);
        }
        Expr::StructLit { fields, .. } => {
            for v in fields.values() {
                check_media_expr(v, media_vars, container, violations);
            }
        }
        Expr::BlockIfElse {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            check_media_expr(condition, media_vars, container, violations);
            check_media_stmts(then_body, media_vars, container, violations);
            for (cond, body) in else_ifs {
                check_media_expr(cond, media_vars, container, violations);
                check_media_stmts(body, media_vars, container, violations);
            }
            if let Some(eb) = else_body {
                check_media_stmts(eb, media_vars, container, violations);
            }
        }
        Expr::MatchExpr {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            check_media_expr(scrutinee, media_vars, container, violations);
            for arm in arms {
                check_media_stmts(arm.body(), media_vars, container, violations);
            }
            if let Some(eb) = else_body {
                check_media_stmts(eb, media_vars, container, violations);
            }
        }
        Expr::Try { expr, .. } => {
            check_media_expr(expr, media_vars, container, violations);
        }
        _ => {}
    }
}

/// Walk statements, tracking media-typed bindings (direct production
/// calls and one-step aliases) and checking every expression.
fn check_media_stmts(
    stmts: &[Statement],
    media_vars: &mut std::collections::HashSet<String>,
    container: &str,
    violations: &mut Vec<MediaOpacityViolation>,
) {
    for s in stmts {
        match s {
            Statement::LetBinding { name, value, .. } => {
                check_media_expr(value, media_vars, container, violations);
                if is_media_binding_expr(value) {
                    media_vars.insert(name.clone());
                } else if let Expr::Ident { name: src, .. } = value {
                    if media_vars.contains(src) {
                        media_vars.insert(name.clone());
                    }
                }
            }
            Statement::Assign { name, value, .. } => {
                check_media_expr(value, media_vars, container, violations);
                if is_media_binding_expr(value) {
                    media_vars.insert(name.clone());
                }
            }
            Statement::Each {
                variable,
                iterable,
                body,
                ..
            } => {
                let _ = variable;
                check_media_expr(iterable, media_vars, container, violations);
                check_media_stmts(body, media_vars, container, violations);
            }
            Statement::EachWithIndex {
                item_var,
                iterable,
                body,
                ..
            } => {
                let _ = item_var;
                check_media_expr(iterable, media_vars, container, violations);
                check_media_stmts(body, media_vars, container, violations);
            }
            Statement::While {
                condition, body, ..
            } => {
                check_media_expr(condition, media_vars, container, violations);
                check_media_stmts(body, media_vars, container, violations);
            }
            Statement::IfElseBlock {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                check_media_expr(condition, media_vars, container, violations);
                check_media_stmts(then_body, media_vars, container, violations);
                for (cond, body) in else_ifs {
                    check_media_expr(cond, media_vars, container, violations);
                    check_media_stmts(body, media_vars, container, violations);
                }
                if let Some(eb) = else_body {
                    check_media_stmts(eb, media_vars, container, violations);
                }
            }
            Statement::IfThen {
                condition, body, ..
            } => {
                check_media_expr(condition, media_vars, container, violations);
                check_media_stmts(body, media_vars, container, violations);
            }
            Statement::Return { value, .. } => {
                check_media_expr(value, media_vars, container, violations);
            }
            Statement::ExprStmt { expr, .. } => {
                check_media_expr(expr, media_vars, container, violations);
            }
            Statement::Match {
                scrutinee,
                arms,
                else_body,
                ..
            } => {
                check_media_expr(scrutinee, media_vars, container, violations);
                for arm in arms {
                    check_media_stmts(arm.body(), media_vars, container, violations);
                }
                if let Some(eb) = else_body {
                    check_media_stmts(eb, media_vars, container, violations);
                }
            }
            Statement::Memorize(m) => {
                check_media_expr(&m.value, media_vars, container, violations);
            }
            Statement::Forget(f) => {
                check_media_expr(&f.query, media_vars, container, violations);
            }
            Statement::Relate(r) => {
                check_media_expr(&r.from, media_vars, container, violations);
                check_media_expr(&r.to, media_vars, container, violations);
            }
            Statement::Break | Statement::Continue => {}
        }
    }
}

/// One opacity violation (№331): a field access whose object bottoms out
/// at a media-typed binding/call. Collected by `media_opacity_violations`;
/// consumed by `check_program` (semantic errors) and the audit Category-A
/// check `MEDIA_HANDLE_OPAQUE` (the compile_program path).
pub struct MediaOpacityViolation {
    pub container: String,
    pub field: String,
    pub span: Span,
}

impl MediaOpacityViolation {
    /// The stable compile-error text (pinned by examples/w1_handle_opaque).
    pub fn message(&self) -> String {
        format!(
            "media handle is opaque (ADR-0114): field access '.{}' on a media \
             handle in {} — bytes never live in Value; use the sanctioned \
             materialization sink (media_save), gated by №325",
            self.field, self.container
        )
    }
}

/// Public entry point: every field access on a media-typed expression in
/// every statement container (the same containers the №324 effect fixpoint
/// walks). See ADR-0162 §2.5.
pub fn media_opacity_violations(declarations: &[Declaration]) -> Vec<MediaOpacityViolation> {
    let mut violations = Vec::new();
    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => {
                let mut media_vars = std::collections::HashSet::new();
                // Params are typed (name: Type) — no media params exist
                // until №332 lands the perception AST; none to pre-track.
                check_media_stmts(
                    &p.body,
                    &mut media_vars,
                    &format!("pattern '{}'", p.name),
                    &mut violations,
                );
            }
            Declaration::LearnablePattern(_) => {
                // A learnable pattern is prompt-declared (no statement body
                // — the №324 effect walker treats it as an LLM call by
                // construction). Nothing to walk here.
            }
            Declaration::Tool(t) => {
                for m in &t.methods {
                    let mut media_vars = std::collections::HashSet::new();
                    check_media_stmts(
                        &m.body,
                        &mut media_vars,
                        &format!("tool method '{}.{}'", t.name, m.name),
                        &mut violations,
                    );
                }
            }
            _ => {}
        }
    }
    violations
}

// ── Origin chain (Наряд №332, ADR-0164 §7.4) ─────────────────────────
//
// The rule: a media handle without origin is NOT constructed. Every
// handle-producing construction must be traceable to a declared source:
//   - `source <origin>`  (HandleSource) — the origin IS the source;
//   - `from <origin> media_store_*(...)` (ProvBind over the Lift) — the
//     bind attaches the construction to the declared origin;
//   - `vision_generate` handles carry the №241 manifest (generation
//     provenance already exists — out of this rule's scope).
// Illegal positions (a construction nested inside another expression, a
// direct `media_source_capture`/`media_bind_origin` builtin call, an
// unknown origin name) are loud violations. The check runs in BOTH
// compile paths: check_program (semantic errors) and audit_category_a
// (ORIGIN_REQUIRED, Category-A) — mirroring the №331 opacity split.

/// What the origin pass tracks per construction site.
pub struct OriginViolation {
    pub message: String,
    pub span: Span,
}

/// The declared origin kinds (generation handles are Lift+ProvBind /
/// vision_generate manifests — `source` capture is camera|file only,
/// enforced in the runtime dispatch).
const ORIGIN_KINDS: &[&str] = &["camera", "file", "generation"];
const ORIGIN_FIELDS: &[&str] = &["kind", "media", "label", "path"];

fn validate_origin_decls(declarations: &[Declaration], errors: &mut Vec<SpannedError>) {
    for decl in declarations {
        if let Declaration::Origin(o) = decl {
            let compiled = match crate::bytecode::CompiledOriginDecl::from_ast(o) {
                Ok(c) => c,
                Err(e) => {
                    errors.push(SpannedError::at(e, o.span.clone()));
                    continue;
                }
            };
            // kind vocabulary
            if !ORIGIN_KINDS.contains(&compiled.kind.as_str()) {
                errors.push(SpannedError::at(
                    format!(
                        "origin '{}': unknown kind '{}' (expected {})",
                        o.name,
                        compiled.kind,
                        ORIGIN_KINDS.join(" | ")
                    ),
                    o.span.clone(),
                ));
            }
            // media vocabulary
            if crate::media::MediaKind::from_slug(&compiled.media).is_err() {
                // from_slug's message names the expected words; re-anchor it.
                errors.push(SpannedError::at(
                    format!(
                        "origin '{}': {}",
                        o.name,
                        crate::media::MediaKind::from_slug(&compiled.media)
                            .err()
                            .unwrap_or_default()
                    ),
                    o.span.clone(),
                ));
            }
            // label vocabulary — poisoned is NOT constructible via a
            // declaration (quarantine comes only from the taint machinery).
            if crate::media::parse_sensitivity(&compiled.conf).is_err() {
                errors.push(SpannedError::at(
                    format!(
                        "origin '{}': {}",
                        o.name,
                        crate::media::parse_sensitivity(&compiled.conf)
                            .err()
                            .unwrap_or_default()
                    ),
                    o.span.clone(),
                ));
            }
            // unknown fields are loud (a typo must never silently no-op)
            for (k, _) in &o.fields {
                if !ORIGIN_FIELDS.contains(&k.as_str()) {
                    errors.push(SpannedError::at(
                        format!(
                            "origin '{}': unknown field '{}' (expected {})",
                            o.name,
                            k,
                            ORIGIN_FIELDS.join(" | ")
                        ),
                        o.span.clone(),
                    ));
                }
            }
        }
    }
}

/// Public entry point: the origin-chain rule over every statement
/// container (the same containers the №331 opacity pass walks).
pub fn media_origin_violations(declarations: &[Declaration]) -> Vec<OriginViolation> {
    // №337 (ADR-0166 §2.3): name → declared kind — the generation
    // contract message needs the KIND of the bound origin. Declared
    // origins that fail compilation (validate_origin_decls) simply do
    // not join the map — their own loud error already fired.
    let origins: std::collections::HashMap<String, String> = declarations
        .iter()
        .filter_map(|d| match d {
            Declaration::Origin(o) => crate::bytecode::CompiledOriginDecl::from_ast(o)
                .ok()
                .map(|c| (c.name, c.kind)),
            _ => None,
        })
        .collect();
    let mut violations = Vec::new();
    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => {
                let mut media_vars = std::collections::HashSet::new();
                check_origin_stmts(
                    &p.body,
                    &mut media_vars,
                    &origins,
                    &format!("pattern '{}'", p.name),
                    &mut violations,
                );
            }
            Declaration::Tool(t) => {
                for m in &t.methods {
                    let mut media_vars = std::collections::HashSet::new();
                    check_origin_stmts(
                        &m.body,
                        &mut media_vars,
                        &origins,
                        &format!("tool method '{}.{}'", t.name, m.name),
                        &mut violations,
                    );
                }
            }
            _ => {}
        }
    }
    violations
}

fn push_origin(violations: &mut Vec<OriginViolation>, span: &Span, message: String) {
    violations.push(OriginViolation {
        message,
        span: span.clone(),
    });
}

/// The construction forms the origin rule accepts at a binding site.
enum BindingForm {
    /// `source <origin>` — the origin is the source.
    Source(String),
    /// `from <origin> media_store_*(...)` — a bound Lift.
    BoundLift(String),
    /// An existing handle variable (alias) — inherits its binding state.
    Alias(String),
    /// Anything else: not a handle construction (checked for nested
    /// violations by the walker).
    Other,
}

fn classify_binding(value: &Expr) -> BindingForm {
    match value {
        Expr::HandleSource { origin, .. } => BindingForm::Source(origin.clone()),
        Expr::ProvBind { origin, inner, .. } => {
            if matches!(inner.as_ref(), Expr::FnCall { name, .. } if is_media_producing_builtin(name))
            {
                BindingForm::BoundLift(origin.clone())
            } else {
                BindingForm::Other
            }
        }
        Expr::Ident { name, .. } => BindingForm::Alias(name.clone()),
        _ => BindingForm::Other,
    }
}

fn check_origin_stmts(
    stmts: &[Statement],
    media_vars: &mut std::collections::HashSet<String>,
    origins: &std::collections::HashMap<String, String>,
    container: &str,
    violations: &mut Vec<OriginViolation>,
) {
    for s in stmts {
        match s {
            Statement::LetBinding {
                name, value, span, ..
            }
            | Statement::Assign { name, value, span } => {
                match classify_binding(value) {
                    BindingForm::Source(origin) => {
                        if !origins.contains_key(&origin) {
                            push_origin(
                                violations,
                                span,
                                format!(
                                    "origin chain violation (ORIGIN_REQUIRED): source '{}' in {} names no declared origin — declare 'origin {} {{ kind: ..., media: ..., label: ... }}' (ADR-0164)",
                                    origin, container, origin
                                ),
                            );
                        }
                        media_vars.insert(name.clone());
                    }
                    BindingForm::BoundLift(origin) => {
                        if !origins.contains_key(&origin) {
                            push_origin(
                                violations,
                                span,
                                format!(
                                    "origin chain violation (ORIGIN_REQUIRED): from '{}' in {} names no declared origin (ADR-0164)",
                                    origin, container
                                ),
                            );
                        }
                        media_vars.insert(name.clone());
                    }
                    BindingForm::Alias(src) => {
                        if media_vars.contains(&src) {
                            media_vars.insert(name.clone());
                        }
                    }
                    BindingForm::Other => {
                        // Nested constructions (a bare Lift, a stray
                        // source/from, a direct builtin call) are loud.
                        check_origin_expr(value, origins, container, violations);
                    }
                }
            }
            Statement::ExprStmt { expr, .. } => {
                // A construction as a bare statement is a discarded handle —
                // the origin rule still refuses unbound Lifts (bytes enter
                // the store without provenance, even if the value is dropped).
                if let Expr::ProvBind { .. } = expr {
                    // `from o media_store_image(...)` as a statement: legal
                    // shape (bind + discard) — inner construction is bound.
                } else {
                    check_origin_expr(expr, origins, container, violations);
                }
            }
            Statement::Return { value, .. } => {
                if !matches!(value, Expr::HandleSource { .. } | Expr::ProvBind { .. }) {
                    check_origin_expr(value, origins, container, violations);
                }
            }
            Statement::Each { iterable, body, .. } => {
                check_origin_expr(iterable, origins, container, violations);
                check_origin_stmts(body, media_vars, origins, container, violations);
            }
            Statement::EachWithIndex { iterable, body, .. } => {
                check_origin_expr(iterable, origins, container, violations);
                check_origin_stmts(body, media_vars, origins, container, violations);
            }
            Statement::While { body, .. } => {
                check_origin_stmts(body, media_vars, origins, container, violations);
            }
            Statement::IfElseBlock {
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                check_origin_stmts(then_body, media_vars, origins, container, violations);
                for (_, b) in else_ifs {
                    check_origin_stmts(b, media_vars, origins, container, violations);
                }
                if let Some(eb) = else_body {
                    check_origin_stmts(eb, media_vars, origins, container, violations);
                }
            }
            Statement::IfThen { body, .. } => {
                check_origin_stmts(body, media_vars, origins, container, violations);
            }
            Statement::Match {
                arms, else_body, ..
            } => {
                for arm in arms {
                    check_origin_stmts(arm.body(), media_vars, origins, container, violations);
                }
                if let Some(eb) = else_body {
                    check_origin_stmts(eb, media_vars, origins, container, violations);
                }
            }
            _ => {}
        }
    }
}

/// Nested-expression check: handle constructions are illegal outside
/// binding initializers.
fn check_origin_expr(
    expr: &Expr,
    origins: &std::collections::HashMap<String, String>,
    container: &str,
    violations: &mut Vec<OriginViolation>,
) {
    match expr {
        Expr::HandleSource { origin, span } => {
            push_origin(
                violations,
                span,
                format!(
                    "origin chain violation (ORIGIN_REQUIRED): 'source {}' in {} is only legal as a binding initializer (`let h = source {}`) — a source handle must be bound to a variable to be tracked (ADR-0164)",
                    origin, container, origin
                ),
            );
        }
        Expr::ProvBind {
            origin,
            inner,
            span,
        } => {
            if !matches!(inner.as_ref(), Expr::FnCall { name, .. } if is_media_producing_builtin(name))
            {
                // №337 (ADR-0166 §2.3): a GENERATION bind over a
                // non-construction is the "generation lift without
                // synthetic: true" hole — the refusal names the marking
                // contract (an alias bind would either skip the marking
                // or falsify it — captured bytes marked synthetic).
                let msg = if origins.get(origin).map(|k| k.as_str()) == Some("generation") {
                    format!(
                        "origin chain violation (ORIGIN_REQUIRED): 'from {}' in {} must wrap a fresh handle construction (media_store_*) — a GENERATION lift sets synthetic: true on the new store entry (ADR-0166 §2.3); binding an existing handle would skip or falsify the Art. 50 marking",
                        origin, container
                    )
                } else {
                    format!(
                        "origin chain violation (ORIGIN_REQUIRED): 'from {}' in {} must wrap a handle construction (media_store_*) — a bind attaches provenance to a NEW handle (ADR-0164)",
                        origin, container
                    )
                };
                push_origin(violations, span, msg);
            }
            check_origin_expr(inner, origins, container, violations);
        }
        Expr::FnCall { name, args, span } => {
            if matches!(
                name.as_str(),
                "media_store_image"
                    | "media_store_audio"
                    | "media_store_video_frame"
                    | "media_store_video_segment"
            ) {
                push_origin(
                    violations,
                    span,
                    format!(
                        "origin chain violation (ORIGIN_REQUIRED): bare {} in {} — a handle without origin is not constructed (§7.4); wrap it: 'from <origin> {}(...)' with a declared origin (ADR-0164)",
                        name, container, name
                    ),
                );
            }
            if matches!(name.as_str(), "media_source_capture" | "media_bind_origin") {
                push_origin(
                    violations,
                    span,
                    format!(
                        "origin chain violation (ORIGIN_REQUIRED): direct {} call in {} — use the source/from syntax (ADR-0164)",
                        name, container
                    ),
                );
            }
            for a in args {
                check_origin_expr(a, origins, container, violations);
            }
        }
        Expr::QualifiedCall { args, .. } => {
            for a in args {
                check_origin_expr(a, origins, container, violations);
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            check_origin_expr(left, origins, container, violations);
            check_origin_expr(right, origins, container, violations);
        }
        Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            check_origin_expr(condition, origins, container, violations);
            check_origin_expr(then_branch, origins, container, violations);
            check_origin_expr(else_branch, origins, container, violations);
        }
        Expr::List { items, .. } => {
            for i in items {
                check_origin_expr(i, origins, container, violations);
            }
        }
        Expr::IndexAccess { object, index, .. } => {
            check_origin_expr(object, origins, container, violations);
            check_origin_expr(index, origins, container, violations);
        }
        Expr::StructLit { fields, .. } => {
            for v in fields.values() {
                check_origin_expr(v, origins, container, violations);
            }
        }
        Expr::BlockIfElse {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            check_origin_expr(condition, origins, container, violations);
            check_origin_stmts(
                then_body,
                &mut std::collections::HashSet::new(),
                origins,
                container,
                violations,
            );
            for (_, b) in else_ifs {
                check_origin_stmts(
                    b,
                    &mut std::collections::HashSet::new(),
                    origins,
                    container,
                    violations,
                );
            }
            if let Some(eb) = else_body {
                check_origin_stmts(
                    eb,
                    &mut std::collections::HashSet::new(),
                    origins,
                    container,
                    violations,
                );
            }
        }
        Expr::MatchExpr {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            check_origin_expr(scrutinee, origins, container, violations);
            for arm in arms {
                for st in arm.body() {
                    check_origin_expr_stmt(st, origins, container, violations);
                }
            }
            if let Some(eb) = else_body {
                for st in eb {
                    check_origin_expr_stmt(st, origins, container, violations);
                }
            }
        }
        Expr::Try { expr, .. } => {
            check_origin_expr(expr, origins, container, violations);
        }
        _ => {}
    }
}

/// Statement-level helper for nested block/match bodies inside the
/// expression walker (lightweight: only the construction sites matter).
fn check_origin_expr_stmt(
    st: &Statement,
    origins: &std::collections::HashMap<String, String>,
    container: &str,
    violations: &mut Vec<OriginViolation>,
) {
    match st {
        Statement::LetBinding { value, .. } | Statement::Assign { value, .. } => {
            check_origin_expr(value, origins, container, violations);
        }
        Statement::ExprStmt { expr, .. } | Statement::Return { value: expr, .. } => {
            check_origin_expr(expr, origins, container, violations);
        }
        _ => {}
    }
}

/// Semantic-layer wiring: origin violations are loud compile errors.
fn check_media_origin_chain(declarations: &[Declaration], errors: &mut Vec<SpannedError>) {
    validate_origin_decls(declarations, errors);
    for v in media_origin_violations(declarations) {
        errors.push(SpannedError::at(v.message, v.span));
    }
}

/// Public entry for the audit path (№332): declared-origin shape and
/// vocabulary errors — the SAME rules `check_program` applies, so a
/// mis-declared origin is loud on EVERY compile path, not only `mlog
/// check`. An unknown kind/media/label word must never silently no-op:
/// a mis-declared provenance source is a provenance lie (ADR-0164 §3).
pub fn origin_decl_errors(declarations: &[Declaration]) -> Vec<SpannedError> {
    let mut errors = Vec::new();
    validate_origin_decls(declarations, &mut errors);
    errors
}

/// Semantic-layer wiring: the violations are loud compile errors.
fn check_media_handle_opacity(declarations: &[Declaration], errors: &mut Vec<SpannedError>) {
    for v in media_opacity_violations(declarations) {
        errors.push(SpannedError::at(v.message(), v.span));
    }
}

// ── BackendSelect ladder companion check (Наряд №336, ADR-0165 §2.4) ──
//
// A statically-visible `backend_select("class", ["rung", …])` call site
// is verified against the №333 registry SSOT at BUILD time:
//   - the class word must be a §7.6 class;
//   - every rung must resolve in the registry (`find_by_name`);
//   - every rung's class must match the requested class;
//   - rungs must be unique (a rung tried twice is a contract bug);
//   - under `profile device { mode: production }` every rung must be
//     SHA-pinnable — a `ShaPin::PendingNo334` rung is UNVERIFIABLE for
//     the production profile and fails compilation (the §11.2 rule,
//     companion-check precedent).
// Non-literal ladders are not statically verifiable and stay with the
// runtime checks (documented in ADR-0165 §2.4).

fn check_backend_select_ladders(declarations: &[Declaration], errors: &mut Vec<SpannedError>) {
    for v in backend_select_ladder_violations(declarations) {
        errors.push(SpannedError::at(v.message, v.span));
    }
}

/// One statically-verifiable ladder defect. `kind` separates the two
/// Category-A check ids: a malformed ladder (BACKEND_SELECT_INVALID)
/// vs a ladder unverifiable for the production device profile
/// (BACKEND_LADDER_UNVERIFIABLE) — ADR-0165 §2.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LadderViolationKind {
    Invalid,
    UnverifiableForProduction,
}

#[derive(Debug, Clone)]
pub struct LadderViolation {
    pub kind: LadderViolationKind,
    pub message: String,
    pub span: Span,
}

/// Public entry for the audit path (№336): the SAME rules `check_program`
/// applies, so a statically-broken ladder is loud on EVERY compile path
/// (`compile_program`/`run_program_with_dir` call `audit_category_a`, not
/// `check_program` — the №332 origin-chain posture).
pub fn backend_select_ladder_violations(declarations: &[Declaration]) -> Vec<LadderViolation> {
    let device_production = crate::profile::resolve(declarations).device_mode_production;
    let mut violations: Vec<LadderViolation> = Vec::new();
    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => {
                check_backend_stmts(&p.body, device_production, &mut violations)
            }
            Declaration::Tool(t) => {
                for m in &t.methods {
                    check_backend_stmts(&m.body, device_production, &mut violations);
                }
            }
            Declaration::MlogServer(srv) => {
                for r in &srv.routes {
                    check_backend_stmts(&r.body, device_production, &mut violations);
                }
            }
            Declaration::Flow(f) => {
                check_backend_expr(&f.source, device_production, &mut violations)
            }
            _ => {}
        }
    }
    violations
}

fn verify_backend_ladder(
    args: &[Expr],
    span: &Span,
    device_production: bool,
    out: &mut Vec<LadderViolation>,
) {
    let push = |out: &mut Vec<LadderViolation>, kind, message: String| {
        out.push(LadderViolation {
            kind,
            message,
            span: span.clone(),
        });
    };
    // The class argument: verified only when literal.
    let class_word = match args.first() {
        Some(Expr::StringLit { value, .. }) => Some(value.clone()),
        _ => None,
    };
    if let Some(word) = &class_word {
        if crate::backends::BackendClass::parse(word).is_none() {
            push(
                out,
                LadderViolationKind::Invalid,
                format!(
                    "backend_select: unknown backend class '{}' (available: stt, tts, omni, vision-understanding, llm)",
                    word
                ),
            );
        }
    }
    // The ladder argument: verified only when a literal list of strings.
    let rung_names: Option<Vec<String>> = match args.get(1) {
        Some(Expr::List { items, .. }) => {
            let mut names = Vec::with_capacity(items.len());
            let mut literal = true;
            for it in items {
                match it {
                    Expr::StringLit { value, .. } => names.push(value.clone()),
                    _ => {
                        literal = false;
                        break;
                    }
                }
            }
            if literal {
                Some(names)
            } else {
                None
            }
        }
        _ => None,
    };
    let names = match rung_names {
        Some(n) => n,
        None => return, // non-literal ladder: runtime checks own it
    };
    if names.is_empty() {
        push(
            out,
            LadderViolationKind::Invalid,
            "backend_select: ladder is EMPTY — a ladder without rungs cannot \
             fall back (silent fallback forbidden, ADR-0165 §2.1)"
                .to_string(),
        );
        return;
    }
    let mut seen: Vec<&str> = Vec::new();
    for name in &names {
        if seen.contains(&name.as_str()) {
            push(
                out,
                LadderViolationKind::Invalid,
                format!(
                    "backend_select: duplicate ladder rung '{}' — a rung tried \
                     twice is a contract bug, not a fallback",
                    name
                ),
            );
        }
        seen.push(name.as_str());
        let entry = match crate::backends::find_by_name(name) {
            Some(e) => e,
            None => {
                push(
                    out,
                    LadderViolationKind::Invalid,
                    format!(
                        "backend_select: ladder rung '{}' has no registry record \
                         (the №333 registry is the SSOT)",
                        name
                    ),
                );
                continue;
            }
        };
        if let Some(word) = &class_word {
            if let Some(class) = crate::backends::BackendClass::parse(word) {
                if entry.class != class {
                    push(
                        out,
                        LadderViolationKind::Invalid,
                        format!(
                            "backend_select: ladder rung '{}' is class '{}', ladder \
                             serves '{}' (ADR-0165 §2.4)",
                            name,
                            entry.class.as_str(),
                            class.as_str()
                        ),
                    );
                }
            }
        }
        if device_production {
            if let crate::backends::ShaPin::PendingNo334 = entry.pin {
                push(
                    out,
                    LadderViolationKind::UnverifiableForProduction,
                    format!(
                        "backend_select: ladder rung '{}' (weights '{}') is \
                         UNVERIFIABLE for device profile production — the weights \
                         manifest is pending (№334 sha-pin path); a production \
                         ladder may only contain SHA-pinned backends (ADR-0165 §2.4)",
                        name, entry.weights_id
                    ),
                );
            }
        }
    }
}

fn check_backend_stmts(stmts: &[Statement], production: bool, out: &mut Vec<LadderViolation>) {
    for st in stmts {
        match st {
            Statement::LetBinding { value, .. } | Statement::Assign { value, .. } => {
                check_backend_expr(value, production, out)
            }
            Statement::ExprStmt { expr, .. } | Statement::Return { value: expr, .. } => {
                check_backend_expr(expr, production, out)
            }
            Statement::Each { iterable, body, .. }
            | Statement::EachWithIndex { iterable, body, .. } => {
                check_backend_expr(iterable, production, out);
                check_backend_stmts(body, production, out);
            }
            Statement::While {
                condition, body, ..
            } => {
                check_backend_expr(condition, production, out);
                check_backend_stmts(body, production, out);
            }
            Statement::IfElseBlock {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                check_backend_expr(condition, production, out);
                check_backend_stmts(then_body, production, out);
                for (c, b) in else_ifs {
                    check_backend_expr(c, production, out);
                    check_backend_stmts(b, production, out);
                }
                if let Some(eb) = else_body {
                    check_backend_stmts(eb, production, out);
                }
            }
            Statement::IfThen {
                condition, body, ..
            } => {
                check_backend_expr(condition, production, out);
                check_backend_stmts(body, production, out);
            }
            Statement::Match {
                scrutinee,
                arms,
                else_body,
                ..
            } => {
                check_backend_expr(scrutinee, production, out);
                for arm in arms {
                    check_backend_stmts(arm.body(), production, out);
                }
                if let Some(eb) = else_body {
                    check_backend_stmts(eb, production, out);
                }
            }
            _ => {}
        }
    }
}

fn check_backend_expr(expr: &Expr, production: bool, out: &mut Vec<LadderViolation>) {
    match expr {
        Expr::FnCall { name, args, span } => {
            if name == "backend_select" {
                verify_backend_ladder(args, span, production, out);
            }
            for a in args {
                check_backend_expr(a, production, out);
            }
        }
        Expr::QualifiedCall { args, .. } => {
            for a in args {
                check_backend_expr(a, production, out);
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            check_backend_expr(left, production, out);
            check_backend_expr(right, production, out);
        }
        Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            check_backend_expr(condition, production, out);
            check_backend_expr(then_branch, production, out);
            check_backend_expr(else_branch, production, out);
        }
        Expr::List { items, .. } => {
            for i in items {
                check_backend_expr(i, production, out);
            }
        }
        Expr::FieldAccess { object, .. } => check_backend_expr(object, production, out),
        Expr::IndexAccess { object, index, .. } => {
            check_backend_expr(object, production, out);
            check_backend_expr(index, production, out);
        }
        Expr::StructLit { fields, .. } => {
            for v in fields.values() {
                check_backend_expr(v, production, out);
            }
        }
        Expr::BlockIfElse {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            check_backend_expr(condition, production, out);
            check_backend_stmts(then_body, production, out);
            for (c, body) in else_ifs {
                check_backend_expr(c, production, out);
                check_backend_stmts(body, production, out);
            }
            if let Some(eb) = else_body {
                check_backend_stmts(eb, production, out);
            }
        }
        Expr::MatchExpr {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            check_backend_expr(scrutinee, production, out);
            for arm in arms {
                check_backend_stmts(arm.body(), production, out);
            }
            if let Some(eb) = else_body {
                check_backend_stmts(eb, production, out);
            }
        }
        Expr::Try { expr, .. } => check_backend_expr(expr, production, out),
        Expr::ProvBind { inner, .. } => check_backend_expr(inner, production, out),
        Expr::HandleSource { .. }
        | Expr::StringLit { .. }
        | Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::Ident { .. } => {}
    }
}

/// Effects of a builtin call site, read from the №316 SSOT map:
/// `Source` crosses the boundary inwards → `io`; `Sink` crosses
/// outwards → `io`, plus `audit` when the external effect is not
/// undoable-pure (a persistent, auditable write: state/db/file/memory
/// writes, delivery). `Pure`/`Lift` call sites carry no effect.
fn builtin_effects(name: &str) -> EffectSet {
    let mut set = EffectSet::new();
    if let Some(class) = classify(name) {
        match class.role {
            Role::Source => {
                set.insert(Effect::Io);
            }
            Role::Sink => {
                set.insert(Effect::Io);
                if class.reversibility != Reversibility::Pure {
                    set.insert(Effect::Audit);
                }
            }
            Role::Pure | Role::Lift => {}
        }
    }
    set
}

/// Walk one expression collecting effect-bearing call sites.
/// `contract_of` resolves a call name (pattern, `tool.method`, or a
/// builtin) to its effect contract.
fn walk_effects_expr(expr: &Expr, contract_of: &dyn Fn(&str) -> EffectSet, acc: &mut EffectSet) {
    match expr {
        // №332 (ADR-0164): HandleSource carries the source-capture effect
        // (store insert — io by the №316 SSOT); ProvBind delegates to the
        // inner construction.
        Expr::HandleSource { .. } => acc.extend(builtin_effects("media_source_capture")),
        Expr::ProvBind { inner, .. } => walk_effects_expr(inner, contract_of, acc),
        Expr::FnCall { name, args, .. } => {
            acc.extend(contract_of(name));
            for a in args {
                walk_effects_expr(a, contract_of, acc);
            }
        }
        Expr::QualifiedCall {
            module,
            function,
            args,
            ..
        } => {
            let qualified = format!("{module}.{function}");
            acc.extend(contract_of(&qualified));
            for a in args {
                walk_effects_expr(a, contract_of, acc);
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            walk_effects_expr(left, contract_of, acc);
            walk_effects_expr(right, contract_of, acc);
        }
        Expr::IfElse {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            walk_effects_expr(condition, contract_of, acc);
            walk_effects_expr(then_branch, contract_of, acc);
            walk_effects_expr(else_branch, contract_of, acc);
        }
        Expr::BlockIfElse {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            walk_effects_expr(condition, contract_of, acc);
            walk_effects_stmts(then_body, contract_of, acc);
            for (_, body) in else_ifs {
                walk_effects_stmts(body, contract_of, acc);
            }
            if let Some(eb) = else_body {
                walk_effects_stmts(eb, contract_of, acc);
            }
        }
        // №369: match-as-expression — scrutinee + arm bodies + else.
        Expr::MatchExpr {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            walk_effects_expr(scrutinee, contract_of, acc);
            for arm in arms {
                walk_effects_stmts(arm.body(), contract_of, acc);
            }
            if let Some(eb) = else_body {
                walk_effects_stmts(eb, contract_of, acc);
            }
        }
        Expr::Try { expr, .. } => walk_effects_expr(expr, contract_of, acc),
        Expr::List { items, .. } => {
            for i in items {
                walk_effects_expr(i, contract_of, acc);
            }
        }
        Expr::StructLit { fields, .. } => {
            for v in fields.values() {
                walk_effects_expr(v, contract_of, acc);
            }
        }
        Expr::IndexAccess { object, index, .. } => {
            walk_effects_expr(object, contract_of, acc);
            walk_effects_expr(index, contract_of, acc);
        }
        Expr::FieldAccess { object, .. } => walk_effects_expr(object, contract_of, acc),
        Expr::StringLit { .. } | Expr::FloatLit { .. } | Expr::BoolLit { .. } => {}
        Expr::Ident { .. } => {}
    }
}

/// Walk one statement list collecting effects (recurses into nested
/// blocks; memory side-effect statements are `audit` effects).
fn walk_effects_stmts(
    stmts: &[Statement],
    contract_of: &dyn Fn(&str) -> EffectSet,
    acc: &mut EffectSet,
) {
    for stmt in stmts {
        match stmt {
            Statement::LetBinding { value, .. } | Statement::Assign { value, .. } => {
                walk_effects_expr(value, contract_of, acc);
            }
            Statement::Each { iterable, body, .. }
            | Statement::EachWithIndex { iterable, body, .. } => {
                walk_effects_expr(iterable, contract_of, acc);
                walk_effects_stmts(body, contract_of, acc);
            }
            Statement::While {
                condition, body, ..
            } => {
                walk_effects_expr(condition, contract_of, acc);
                walk_effects_stmts(body, contract_of, acc);
            }
            Statement::IfElseBlock {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                walk_effects_expr(condition, contract_of, acc);
                walk_effects_stmts(then_body, contract_of, acc);
                for (_, body) in else_ifs {
                    walk_effects_stmts(body, contract_of, acc);
                }
                if let Some(eb) = else_body {
                    walk_effects_stmts(eb, contract_of, acc);
                }
            }
            Statement::IfThen {
                condition, body, ..
            } => {
                walk_effects_expr(condition, contract_of, acc);
                walk_effects_stmts(body, contract_of, acc);
            }
            Statement::Return { value, .. } | Statement::ExprStmt { expr: value, .. } => {
                walk_effects_expr(value, contract_of, acc);
            }
            Statement::Match {
                scrutinee,
                arms,
                else_body,
                ..
            } => {
                walk_effects_expr(scrutinee, contract_of, acc);
                for arm in arms {
                    let body = match arm {
                        MatchArm::Exact(_, b)
                        | MatchArm::StartsWith(_, b)
                        | MatchArm::Contains(_, b)
                        | MatchArm::Compare(_, _, b) => b,
                    };
                    walk_effects_stmts(body, contract_of, acc);
                }
                if let Some(eb) = else_body {
                    walk_effects_stmts(eb, contract_of, acc);
                }
            }
            Statement::Memorize(m) => {
                // Persistent memory write — an audit effect (ADR-0154 §9).
                acc.insert(Effect::Io);
                acc.insert(Effect::Audit);
                walk_effects_expr(&m.value, contract_of, acc);
            }
            Statement::Forget(f) => {
                acc.insert(Effect::Io);
                acc.insert(Effect::Audit);
                walk_effects_expr(&f.query, contract_of, acc);
            }
            Statement::Relate(r) => {
                acc.insert(Effect::Io);
                acc.insert(Effect::Audit);
                walk_effects_expr(&r.from, contract_of, acc);
                walk_effects_expr(&r.to, contract_of, acc);
            }
            Statement::Break | Statement::Continue => {}
        }
    }
}

/// One effect-trail container: a name key resolvable at call sites
/// (`"P"`, `"tool.method"`, `"LP"`), its optional declared trail, and
/// the body kind (learnables have no statement body — their factual
/// effect is fixed `{io}`: a model call).
struct EffectContainer<'a> {
    key: String,
    declared: Option<&'a EffectAnn>,
    name_for_errors: String,
    kind: ContainerKind<'a>,
}

enum ContainerKind<'a> {
    Statements(&'a [Statement]),
    Learnable,
}

/// Compute the whole-program effect trail: fixpoint over the pattern
/// call graph, then gate every DECLARED trail against the inferred
/// effects (factual ⊑ declared; excess = compile error with the list).
///
/// Recursion (ADR-0154 §9 decision): the effect domain is the
/// 4-element powerset of `{io, audit}` — union-joins over this finite
/// flat lattice converge in ≤ 4 passes, so the inference CONVERGES on
/// recursive patterns without annotations (the dispatcher's No-Go
/// signal does not fire; an explicit trail on recursive patterns is
/// welcome but not required). An unannotated recursive pattern
/// therefore behaves predictably: its effects are inferred, and the
/// gate applies only where a trail is declared.
fn check_effect_trails(declarations: &[Declaration], errors: &mut Vec<SpannedError>) {
    // ── Validate every declared trail (bad words are loud) and collect
    //    containers; a trail that failed validation is NOT registered
    //    in `declared`, so the gate skips it — the word error is the
    //    one the user sees.
    let mut containers: Vec<EffectContainer> = Vec::new();
    let mut declared: HashMap<String, EffectSet> = HashMap::new();

    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => {
                if let Some(ann) = &p.effects {
                    validate_effect_ann(ann, &format!("pattern '{}'", p.name), errors);
                }
            }
            Declaration::LearnablePattern(lp) => {
                if let Some(ann) = &lp.effects {
                    validate_effect_ann(ann, &format!("learnable pattern '{}'", lp.name), errors);
                }
            }
            Declaration::Tool(t) => {
                for m in &t.methods {
                    if let Some(ann) = &m.effects {
                        validate_effect_ann(
                            ann,
                            &format!("tool method '{}.{}'", t.name, m.name),
                            errors,
                        );
                    }
                }
            }
            _ => {}
        }
    }

    fn register<'a>(
        containers: &mut Vec<EffectContainer<'a>>,
        declared: &mut HashMap<String, EffectSet>,
        key: String,
        name_for_errors: String,
        ann: Option<&'a EffectAnn>,
        kind: ContainerKind<'a>,
    ) {
        if let Some(ann) = ann {
            if let Ok(set) = parse_effect_set(ann) {
                declared.insert(key.clone(), set);
            }
        }
        containers.push(EffectContainer {
            key,
            declared: ann,
            name_for_errors,
            kind,
        });
    }

    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => register(
                &mut containers,
                &mut declared,
                p.name.clone(),
                format!("pattern '{}'", p.name),
                p.effects.as_ref(),
                ContainerKind::Statements(&p.body),
            ),
            Declaration::LearnablePattern(lp) => register(
                &mut containers,
                &mut declared,
                lp.name.clone(),
                format!("learnable pattern '{}'", lp.name),
                lp.effects.as_ref(),
                ContainerKind::Learnable,
            ),
            Declaration::Tool(t) => {
                for m in &t.methods {
                    register(
                        &mut containers,
                        &mut declared,
                        format!("{}.{}", t.name, m.name),
                        format!("tool method '{}.{}'", t.name, m.name),
                        m.effects.as_ref(),
                        ContainerKind::Statements(&m.body),
                    );
                }
            }
            _ => {}
        }
    }

    // A DECLARED trail is the call's contract (interface semantics);
    // otherwise the current fixpoint estimate; builtins via the №316
    // SSOT; unknown names carry no effects.
    let contract_of = |name: &str,
                       inferred: &HashMap<String, EffectSet>,
                       declared: &HashMap<String, EffectSet>|
     -> EffectSet {
        if let Some(d) = declared.get(name) {
            return d.clone();
        }
        if let Some(e) = inferred.get(name) {
            return e.clone();
        }
        builtin_effects(name)
    };

    let factual_effects = |kind: &ContainerKind,
                           inferred: &HashMap<String, EffectSet>,
                           declared: &HashMap<String, EffectSet>|
     -> EffectSet {
        match kind {
            ContainerKind::Learnable => {
                // A learnable pattern IS an LLM call — io by
                // construction (№316: call_llm is a Source).
                let mut s = EffectSet::new();
                s.insert(Effect::Io);
                s
            }
            ContainerKind::Statements(body) => {
                let mut acc = EffectSet::new();
                let resolve = |name: &str| contract_of(name, inferred, declared);
                walk_effects_stmts(body, &resolve, &mut acc);
                acc
            }
        }
    };

    // ── Fixpoint over the (possibly recursive) call graph ──
    let mut inferred: HashMap<String, EffectSet> = containers
        .iter()
        .map(|c| (c.key.clone(), EffectSet::new()))
        .collect();
    let max_passes = (containers.len() + 2).clamp(4, 16);
    for _ in 0..max_passes {
        let mut changed = false;
        for c in &containers {
            let new_set = factual_effects(&c.kind, &inferred, &declared);
            if inferred.get(&c.key) != Some(&new_set) {
                inferred.insert(c.key.clone(), new_set);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    // ── Gate every DECLARED trail: factual ⊑ declared ──
    for c in &containers {
        let Some(ann) = c.declared else { continue };
        let Some(declared_set) = declared.get(&c.key) else {
            continue;
        };
        let actual = factual_effects(&c.kind, &inferred, &declared);
        let excess: Vec<Effect> = actual.difference(declared_set).copied().collect();
        if excess.is_empty() {
            continue;
        }
        let fmt = |es: &[Effect]| {
            let words: Vec<&str> = es.iter().map(|e| e.word()).collect();
            if words.is_empty() {
                "⟨⟩".to_string()
            } else {
                format!("⟨{}⟩", words.join(", "))
            }
        };
        let declared_sorted: Vec<Effect> = declared_set.iter().copied().collect();
        let actual_sorted: Vec<Effect> = actual.iter().copied().collect();
        errors.push(SpannedError::at(
            format!(
                "effect trail violation on {}: declared {} but body requires {} (excess: {})",
                c.name_for_errors,
                fmt(&declared_sorted),
                fmt(&actual_sorted),
                excess
                    .iter()
                    .map(|e| e.word())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            ann.span.clone(),
        ));
    }
}

// ── Sink clearance (Наряд №325, ADR-0161) ────────────────────────────

/// One sink call-site whose argument label does not clear the sink.
#[derive(Debug, Clone)]
pub struct SinkViolation {
    /// Container tag: `pattern P`, `tool t.m`, `route GET /x`, ...
    pub container: String,
    /// Sink builtin name (from the №316 classification — never a
    /// hand-written list).
    pub fn_name: String,
    /// 0-based index of the offending argument.
    pub arg_index: usize,
    pub span: Span,
    /// The argument's inferred label (№322/№323 machinery).
    pub label: Label,
    /// Which bridge threshold failed (№391): the clearance reason —
    /// e.g. "untrusted-exec", "secret-exec", "untrusted-egress",
    /// "private-egress", "private-db", "irreversible-content". Consumed
    /// by the deny message (explainable refusal) and by №392 DenyEvent.
    pub reason: &'static str,
}

/// Conservative confidentiality markers for string LITERALS (ADR-0161
/// §3): a literal carrying a personal-data marker (passport / SNILS /
/// diagnosis / confidential wording — the vocabulary is deliberately
/// small and bilingual-safe) or a private-infrastructure URL marker is
/// treated as `private, trusted`. Sound: markers are conservative —
/// a literal without markers stays bottom, an unannotated program
/// keeps passing unless it actually carries the marked data to a sink.
fn literal_confidentiality(text: &str) -> bool {
    let lower = text.to_lowercase();
    const WORD_MARKERS: &[&str] = &[
        "паспорт",
        "снилс",
        "диагноз",
        "конфиденциальн",
        "персональн",
        "секретн",
        "confidential",
        "passport",
        "social security",
    ];
    if WORD_MARKERS.iter().any(|m| lower.contains(m)) {
        return true;
    }
    // Private-infrastructure URL markers (ADR-0161 §3): a literal
    // address/host pointing at internal infrastructure names private
    // data — the destination IS the leak vector.
    const URL_MARKERS: &[&str] = &["internal", "intranet", "corp.", "private", "secret"];
    if URL_MARKERS.iter().any(|m| lower.contains(m)) {
        return true;
    }
    // Structural markers: passport RU (dddd dddddd / dddddddd), SNILS
    // (ddd-ddd-ddd d), card-like 16-digit groups are covered by the
    // word markers above in the corpus; the digit shapes below catch
    // the bare-number forms.
    let digits: Vec<char> = lower.chars().collect();
    let digit_at = |i: usize| digits.get(i).is_some_and(|c| c.is_ascii_digit());
    let pasp = (0..digits.len()).filter(|i| digit_at(*i)).any(|i| {
        // 4 digits, optional space/dash, 6 digits
        (0..4).all(|k| digit_at(i + k))
            && matches!(digits.get(i + 4), None | Some(' ') | Some('-'))
            && (0..6).all(|k| digit_at(i + 5 + k))
            && digits.get(i + 11).is_none_or(|c| !c.is_ascii_digit())
    });
    if pasp {
        return true;
    }
    let snils = (0..digits.len()).filter(|i| digit_at(*i)).any(|i| {
        (0..3).all(|k| digit_at(i + k))
            && digits.get(i + 3) == Some(&'-')
            && (0..3).all(|k| digit_at(i + 4 + k))
            && digits.get(i + 7) == Some(&'-')
            && (0..3).all(|k| digit_at(i + 8 + k))
            && digits.get(i + 11).is_none_or(|c| !c.is_ascii_digit())
    });
    snils
}

/// The seed label of a global entity initializer: a literal carrying
/// confidentiality markers is `private, trusted`; everything else is
/// bottom (the №322 permissive default).
fn entity_seed_label(value: Option<&Expr>) -> Label {
    match value {
        Some(Expr::StringLit { value, .. }) if literal_confidentiality(value) => Label {
            conf: crate::labels::Conf::Private,
            integrity: crate::labels::Integrity::Trusted,
            consent: Default::default(),
        },
        _ => Label::bottom(),
    }
}

/// Label of an expression, extending the №323 expression rules with the
/// literal confidentiality markers (ADR-0161 §3) and the №332 perception
/// forms: `source <origin>` carries the origin's declared label; `from
/// <origin> <construction>` carries the JOIN of the origin label and the
/// construction's data-flow label (conservative — the strongest axis wins).
fn sink_arg_label(
    expr: &Expr,
    env: &BTreeMap<String, Label>,
    origin_labels: &BTreeMap<String, Label>,
) -> Label {
    match expr {
        Expr::StringLit { value, .. } if literal_confidentiality(value) => Label {
            conf: crate::labels::Conf::Private,
            integrity: crate::labels::Integrity::Trusted,
            consent: Default::default(),
        },
        Expr::HandleSource { origin, .. } => origin_labels
            .get(origin)
            .cloned()
            .unwrap_or_else(Label::bottom),
        Expr::ProvBind { origin, inner, .. } => {
            let o = origin_labels
                .get(origin)
                .cloned()
                .unwrap_or_else(Label::bottom);
            o.join(&expr_label(inner, env))
        }
        _ => expr_label(expr, env),
    }
}

type SinkCheck<'a> = &'a dyn Fn(&Expr, &str, &BTreeMap<String, Label>, &mut Vec<SinkViolation>);

/// Collect sink call-sites whose argument labels do not clear the sink
/// (№325, ADR-0161 §2). The sink list comes from the №316 SSOT map
/// (`Role::Sink`) — never a hand-written list. Confidentiality
/// clearance: a sink requires `public` (and `poisoned` clears nothing).
/// The EXEC class additionally refuses untrusted data (an integrity
/// decision gate for command execution; the general integrity gate is
/// №327). The VOICE class additionally refuses anything without a
/// consent scope (consent SOURCES are Phase 2, №335 — until then every
/// voice egress is unconsented by default, loud by design).
/// ── Naryad #391: the data ↔ action bridge — per-sink thresholds ──────
///
/// The bridge rule for ACTION sinks: the decision argument (command /
/// URL / SQL / addressee) must satisfy BOTH axes of the label lattice
/// (ADR-0154):
///   - confidentiality: label.conf ⊑ Public (the action must not leak
///     secrets into its own trace);
///   - integrity: label.integrity ≥ Trusted (untrusted data must not
///     drive an irreversible action).
///
/// This is the SYSTEMATIC rule the point classes (UNTRUSTED_EXEC_DECISION,
/// SECRET_TO_EXEC, SECRET_EGRESS_VCS, SECRET_EGRESS_NETWORK, PII_EGRESS_*)
/// were hand-expressing; the classes stay (leak-suite vocabulary, DoD в —
/// nothing is weakened), the table documents the thresholds per sink.
///
/// Orthogonality with grants (№390): the grant check authorizes the
/// ACTION (scope/TTL/quota, runtime); the bridge gates the DATA that
/// feeds it (labels, compile time). `db_execute_with_grant` is not a
/// №325 sink — its grant gates are runtime-only; the bridge does not
/// duplicate them.
pub struct ActionBridgeThreshold {
    pub sink: &'static str,
    /// Human-readable decision-argument description.
    pub decision_arg: &'static str,
    /// The confidentiality threshold (always Public today — the action
    /// trace must not carry secrets).
    pub max_conf: &'static str,
    /// The integrity threshold (always Trusted today — untrusted data
    /// must not drive the action).
    pub min_integrity: &'static str,
    /// How the integrity half is enforced for this sink.
    pub integrity_enforcement: &'static str,
}

pub const ACTION_BRIDGE: &[ActionBridgeThreshold] = &[
    ActionBridgeThreshold {
        sink: "exec",
        decision_arg: "arg 0 — command",
        max_conf: "public",
        min_integrity: "trusted",
        integrity_enforcement: "clearance (UNTRUSTED_EXEC_DECISION)",
    },
    ActionBridgeThreshold {
        sink: "exec_argv",
        decision_arg: "arg 0 — binary",
        max_conf: "public",
        min_integrity: "trusted",
        integrity_enforcement: "clearance (UNTRUSTED_EXEC_DECISION)",
    },
    ActionBridgeThreshold {
        sink: "git_push",
        decision_arg: "arg 0 — remote URL/ref",
        max_conf: "public",
        min_integrity: "trusted",
        integrity_enforcement: "clearance (UNTRUSTED_EGRESS_NETWORK; №391 adds the integrity half)",
    },
    ActionBridgeThreshold {
        sink: "http_post",
        decision_arg: "arg 0 — URL",
        max_conf: "public",
        min_integrity: "trusted",
        integrity_enforcement: "clearance (UNTRUSTED_EGRESS_NETWORK; body args = egress classes)",
    },
    ActionBridgeThreshold {
        sink: "send_message",
        decision_arg: "arg 0 — chat/addressee",
        max_conf: "public",
        min_integrity: "trusted",
        integrity_enforcement: "clearance (UNTRUSTED_EGRESS_NETWORK; body args = egress classes)",
    },
    ActionBridgeThreshold {
        sink: "db_execute",
        decision_arg: "arg 0 — SQL",
        max_conf: "public",
        min_integrity: "trusted",
        integrity_enforcement: "clearance for confidentiality (private-db); integrity via SQL_DYNAMIC (non-literal SQL is refused before the bridge can see it)",
    },
];

pub fn sink_clearance_violations(declarations: &[Declaration]) -> Vec<SinkViolation> {
    let mut violations = Vec::new();

    // №332 (ADR-0164): declared origin labels — `source <origin>` /
    // `from <origin> ...` constructions carry their origin's declared
    // conf into the flow (joined with the data-flow label).
    let origin_labels: BTreeMap<String, Label> = declarations
        .iter()
        .filter_map(|d| match d {
            Declaration::Origin(o) => {
                let conf = crate::media::parse_sensitivity(
                    &o.fields
                        .iter()
                        .find(|(k, _)| k == "label")
                        .map(|(_, v)| v.clone())
                        .unwrap_or_default(),
                )
                .ok()?;
                Some((
                    o.name.clone(),
                    Label {
                        conf,
                        integrity: crate::labels::Integrity::Trusted,
                        consent: Default::default(),
                    },
                ))
            }
            _ => None,
        })
        .collect();

    // Seed environment: global entities (initializers with confidentiality
    // markers are private — ADR-0161 §3), all other names bottom.
    let mut seed: BTreeMap<String, Label> = BTreeMap::new();
    for decl in declarations {
        match decl {
            Declaration::EntitySimple(e) => {
                seed.insert(e.name.clone(), entity_seed_label(Some(&e.value)));
            }
            Declaration::EntityRecord(e) => {
                // Record entities are maps; the entity name itself seeds
                // with the strongest field marker.
                let strongest = e
                    .fields
                    .iter()
                    .map(|f| entity_seed_label(Some(&f.value)))
                    .fold(Label::bottom(), |a, b| a.join(&b));
                seed.insert(e.name.clone(), strongest);
            }
            _ => {}
        }
    }

    fn is_sink(name: &str) -> bool {
        matches!(
            classify(name).map(|c| c.role),
            Some(crate::builtins_classification::Role::Sink)
        )
    }

    fn sink_kind(name: &str) -> &'static str {
        // №331 boundary (loud): media GENERATION sinks (vision_*/video_*)
        // produce synthetic content — their egress is gated by the
        // marking machinery (MEDIA_SYNTHETIC_UNMARKED, №320) and their
        // label-side gating is Phase 2; they are outside №325.
        if name.starts_with("vision_") || name.starts_with("video_") || name == "tts_synthesize" {
            return "media";
        }
        match name {
            "exec" | "exec_argv" => "exec",
            "git_push" => "vcs",
            "tts_send" => "voice",
            "db_execute" => "db",
            "print" | "respond" | "respond_html" | "html_response" => "output",
            "write_file" | "append_file" | "delete_file" => "file",
            // Persistent memory writes: untrusted data must not persist
            // (the TAINT_PERSISTENCE vocabulary; №266 statement forms are
            // covered separately).
            "memorize" | "mem_set" | "mtree_store" | "kv_set" => "memory",
            // №331 (ADR-0162): the sanctioned media materialization sink
            // is FILE egress — private-labelled handles fail the
            // default clearance (`private-egress`) at compile time; the
            // runtime backstop (MEDIA_SEALED_EGRESS) refuses non-public
            // entries even if the static layer was bypassed.
            "media_save" => "file",
            _ => "network",
        }
    }

    /// Clearance check for one sink argument. Returns the reason the
    /// argument fails, if any.
    fn clearance_failure(fn_name: &str, label: &Label) -> Option<&'static str> {
        // №335: the quarantine sink is the ONLY legal egress for a
        // poisoned value (unconditional audit event; №326 posture) —
        // exempt from every clearance class here. Every OTHER sink
        // still refuses poisoned below (quarantine absorbs them).
        if fn_name == "quarantine_write" {
            return None;
        }
        // Quarantine clears nothing, anywhere (ADR-0154 §2.1).
        if label.conf == crate::labels::Conf::Poisoned {
            return Some("poisoned");
        }
        match sink_kind(fn_name) {
            // Command execution: untrusted data must not drive it, and
            // secrets must never enter it.
            "exec" => {
                if label.integrity == crate::labels::Integrity::Untrusted {
                    Some("untrusted-exec")
                } else if label.conf != crate::labels::Conf::Public {
                    Some("secret-exec")
                } else {
                    None
                }
            }
            // Media generation: №331 Phase-2 boundary — the marking
            // machinery (MEDIA_SYNTHETIC_UNMARKED, №320) owns it here.
            "media" => None,
            // Voice egress requires a consent scope (Phase-2 sources, №335;
            // until then the scope is empty by default — loud by design).
            "voice" => {
                if label.consent.is_empty() {
                    Some("voice-unconsented")
                } else {
                    None
                }
            }
            // Irreversible DB writes with destructive literals are gated
            // regardless of label (grant algebra is Phase 3, №339).
            // NOTE: only schema-destroying forms (DROP/TRUNCATE) —
            // DELETE/ALTER are parameterized CRUD, gated by SQL_DYNAMIC.
            "db" => {
                if label.conf != crate::labels::Conf::Public {
                    Some("private-db")
                } else {
                    None
                }
            }
            // Public-output and memory sinks also refuse UNTRUSTED data
            // (the HTML-injection and taint-persistence vocabularies —
            // the leak-suite classes; the general integrity gate is
            // №327, these two special cases start here). Everything else:
            // confidentiality clearance (public only); network sinks
            // additionally refuse untrusted data (UNTRUSTED_EGRESS_*).
            _ => {
                let kind = sink_kind(fn_name);
                if label.conf != crate::labels::Conf::Public {
                    Some("private-egress")
                } else if label.integrity == crate::labels::Integrity::Untrusted
                    // №391 bridge: "vcs" joins the integrity-gated kinds —
                    // an untrusted URL must not drive git_push (the
                    // systematization of the point rules; the class stays
                    // UNTRUSTED_EGRESS_NETWORK, the leak-suite vocabulary).
                    && matches!(kind, "network" | "vcs" | "output" | "memory")
                {
                    Some("untrusted-egress")
                } else {
                    None
                }
            }
        }
    }

    let check_calls = |expr: &Expr,
                       container: &str,
                       env: &BTreeMap<String, Label>,
                       violations: &mut Vec<SinkViolation>| {
        if let Expr::FnCall { name, args, .. } = expr {
            if is_sink(name) {
                for (i, a) in args.iter().enumerate() {
                    // Destructive DB literals are gated by CONTENT (a
                    // DROP needs no tainted data to destroy state):
                    if name == "db_execute"
                        && matches!(
                            a,
                            Expr::StringLit { value, .. }
                                if ["drop table", "drop database", "drop index", "truncate"].iter().any(|w| value.to_lowercase().contains(w))
                        )
                    {
                        violations.push(SinkViolation {
                            container: container.to_string(),
                            fn_name: name.clone(),
                            arg_index: i,
                            span: expr.span().clone(),
                            label: Label::bottom(),
                            reason: "irreversible-content",
                        });
                        continue;
                    }
                    let label = sink_arg_label(a, env, &origin_labels);
                    if let Some(reason) = clearance_failure(name, &label) {
                        violations.push(SinkViolation {
                            container: container.to_string(),
                            fn_name: name.clone(),
                            arg_index: i,
                            span: expr.span().clone(),
                            label,
                            reason,
                        });
                    }
                }
            }
        }
    };

    fn walk_expr(
        expr: &Expr,
        container: &str,
        env: &BTreeMap<String, Label>,
        origin_labels: &BTreeMap<String, Label>,
        check: SinkCheck,
        violations: &mut Vec<SinkViolation>,
    ) {
        check(expr, container, env, violations);
        match expr {
            Expr::FnCall { args, .. } | Expr::QualifiedCall { args, .. } => {
                for a in args {
                    walk_expr(a, container, env, origin_labels, check, violations);
                }
            }
            Expr::BinaryOp { left, right, .. } => {
                walk_expr(left, container, env, origin_labels, check, violations);
                walk_expr(right, container, env, origin_labels, check, violations);
            }
            Expr::IfElse {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                walk_expr(condition, container, env, origin_labels, check, violations);
                walk_expr(
                    then_branch,
                    container,
                    env,
                    origin_labels,
                    check,
                    violations,
                );
                walk_expr(
                    else_branch,
                    container,
                    env,
                    origin_labels,
                    check,
                    violations,
                );
            }
            Expr::BlockIfElse {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                // Expression position: env side effects of the branches do
                // not escape (№323 D5) — analyze the branches on forks.
                walk_expr(condition, container, env, origin_labels, check, violations);
                let mut env_t = env.clone();
                walk_stmts(
                    then_body,
                    container,
                    &mut env_t,
                    origin_labels,
                    check,
                    violations,
                );
                for (_, b) in else_ifs {
                    let mut env_b = env.clone();
                    walk_stmts(b, container, &mut env_b, origin_labels, check, violations);
                }
                if let Some(eb) = else_body {
                    let mut env_e = env.clone();
                    walk_stmts(eb, container, &mut env_e, origin_labels, check, violations);
                }
            }
            Expr::Try { expr, .. } => {
                walk_expr(expr, container, env, origin_labels, check, violations)
            }
            Expr::List { items, .. } => {
                for i in items {
                    walk_expr(i, container, env, origin_labels, check, violations);
                }
            }
            Expr::StructLit { fields, .. } => {
                for v in fields.values() {
                    walk_expr(v, container, env, origin_labels, check, violations);
                }
            }
            Expr::IndexAccess { object, index, .. } => {
                walk_expr(object, container, env, origin_labels, check, violations);
                walk_expr(index, container, env, origin_labels, check, violations);
            }
            Expr::FieldAccess { object, .. } => {
                walk_expr(object, container, env, origin_labels, check, violations)
            }
            _ => {}
        }
    }

    fn walk_stmts(
        stmts: &[Statement],
        container: &str,
        env: &mut BTreeMap<String, Label>,
        origin_labels: &BTreeMap<String, Label>,
        check: SinkCheck,
        violations: &mut Vec<SinkViolation>,
    ) {
        for stmt in stmts {
            match stmt {
                Statement::LetBinding { name, value, .. }
                | Statement::Assign { name, value, .. } => {
                    walk_expr(value, container, env, origin_labels, check, violations);
                    // Track the binding (flow-sensitive overwrite, №323
                    // LetBinding/Assign contract) so later sink calls see
                    // the carried label.
                    let label = sink_arg_label(value, env, origin_labels);
                    env.insert(name.clone(), label);
                }
                Statement::Each {
                    variable,
                    iterable,
                    body,
                    ..
                }
                | Statement::EachWithIndex {
                    item_var: variable,
                    iterable,
                    body,
                    ..
                } => {
                    walk_expr(iterable, container, env, origin_labels, check, violations);
                    // The iterant is bound for the body only (scope-local,
                    // №323 Each contract) — analyze the body with a fork.
                    let mut env_body = env.clone();
                    env_body.insert(
                        variable.clone(),
                        sink_arg_label(iterable, env, origin_labels),
                    );
                    walk_stmts(
                        body,
                        container,
                        &mut env_body,
                        origin_labels,
                        check,
                        violations,
                    );
                }
                Statement::While {
                    condition, body, ..
                } => {
                    walk_expr(condition, container, env, origin_labels, check, violations);
                    // Conservative single-pass join (ADR-0154 §8 D1): the
                    // body may raise labels; fold the raises back.
                    let mut env_body = env.clone();
                    walk_stmts(
                        body,
                        container,
                        &mut env_body,
                        origin_labels,
                        check,
                        violations,
                    );
                    for (k, v) in env_body {
                        let merged = env.get(&k).cloned().unwrap_or_else(Label::bottom).join(&v);
                        env.insert(k, merged);
                    }
                }
                Statement::IfElseBlock {
                    condition,
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    walk_expr(condition, container, env, origin_labels, check, violations);
                    // Branches fork from the entry env; merge on exit
                    // (componentwise, №323 rules). A closed merge (else
                    // present) drops the pre-branch value; an open merge
                    // keeps the entry env in the join.
                    let mut env_then = env.clone();
                    walk_stmts(
                        then_body,
                        container,
                        &mut env_then,
                        origin_labels,
                        check,
                        violations,
                    );
                    let mut merged = env_then;
                    for (_, b) in else_ifs {
                        let mut env_b = env.clone();
                        walk_stmts(b, container, &mut env_b, origin_labels, check, violations);
                        for (k, v) in env_b {
                            let m = merged
                                .get(&k)
                                .cloned()
                                .unwrap_or_else(Label::bottom)
                                .join(&v);
                            merged.insert(k, m);
                        }
                    }
                    if let Some(eb) = else_body {
                        let mut env_e = env.clone();
                        walk_stmts(eb, container, &mut env_e, origin_labels, check, violations);
                        for (k, v) in env_e {
                            let m = merged
                                .get(&k)
                                .cloned()
                                .unwrap_or_else(Label::bottom)
                                .join(&v);
                            merged.insert(k, m);
                        }
                    } else {
                        // Open merge: the entry env survives.
                        for (k, v) in env.clone() {
                            let m = merged
                                .get(&k)
                                .cloned()
                                .unwrap_or_else(Label::bottom)
                                .join(&v);
                            merged.insert(k, m);
                        }
                    }
                    *env = merged;
                }
                Statement::IfThen {
                    condition, body, ..
                } => {
                    walk_expr(condition, container, env, origin_labels, check, violations);
                    let mut env_t = env.clone();
                    walk_stmts(
                        body,
                        container,
                        &mut env_t,
                        origin_labels,
                        check,
                        violations,
                    );
                    for (k, v) in env_t {
                        let m = env.get(&k).cloned().unwrap_or_else(Label::bottom).join(&v);
                        env.insert(k, m);
                    }
                }
                Statement::Return { value, .. } | Statement::ExprStmt { expr: value, .. } => {
                    walk_expr(value, container, env, origin_labels, check, violations);
                }
                Statement::Match {
                    scrutinee,
                    arms,
                    else_body,
                    ..
                } => {
                    walk_expr(scrutinee, container, env, origin_labels, check, violations);
                    let mut merged = env.clone();
                    for arm in arms {
                        let body = match arm {
                            MatchArm::Exact(_, b)
                            | MatchArm::StartsWith(_, b)
                            | MatchArm::Contains(_, b)
                            | MatchArm::Compare(_, _, b) => b,
                        };
                        let mut env_b = env.clone();
                        walk_stmts(
                            body,
                            container,
                            &mut env_b,
                            origin_labels,
                            check,
                            violations,
                        );
                        for (k, v) in env_b {
                            let m = merged
                                .get(&k)
                                .cloned()
                                .unwrap_or_else(Label::bottom)
                                .join(&v);
                            merged.insert(k, m);
                        }
                    }
                    if let Some(eb) = else_body {
                        let mut env_e = env.clone();
                        walk_stmts(eb, container, &mut env_e, origin_labels, check, violations);
                        for (k, v) in env_e {
                            let m = merged
                                .get(&k)
                                .cloned()
                                .unwrap_or_else(Label::bottom)
                                .join(&v);
                            merged.insert(k, m);
                        }
                    } else {
                        for (k, v) in env.clone() {
                            let m = merged
                                .get(&k)
                                .cloned()
                                .unwrap_or_else(Label::bottom)
                                .join(&v);
                            merged.insert(k, m);
                        }
                    }
                    *env = merged;
                }
                Statement::Memorize(m) => {
                    walk_expr(&m.value, container, env, origin_labels, check, violations)
                }
                Statement::Forget(f) => {
                    walk_expr(&f.query, container, env, origin_labels, check, violations)
                }
                Statement::Relate(r) => {
                    walk_expr(&r.from, container, env, origin_labels, check, violations);
                    walk_expr(&r.to, container, env, origin_labels, check, violations);
                }
                Statement::Break | Statement::Continue => {}
            }
        }
    }

    let walk_container = |stmts: &[Statement],
                          container: &str,
                          params: &[crate::ast::Param],
                          violations: &mut Vec<SinkViolation>| {
        let mut env = seed.clone();
        for p in params {
            let l = match &p.label {
                Some(ann) => Label::parse(&ann.raw).unwrap_or_default(),
                None => Label::bottom(),
            };
            env.insert(p.name.clone(), l);
        }
        walk_stmts(
            stmts,
            container,
            &mut env,
            &origin_labels,
            &check_calls,
            violations,
        );
    };

    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => {
                walk_container(
                    &p.body,
                    &format!("pattern {}", p.name),
                    &p.params,
                    &mut violations,
                );
            }
            Declaration::Tool(t) => {
                for m in &t.methods {
                    walk_container(
                        &m.body,
                        &format!("tool {}.{}", t.name, m.name),
                        &m.params,
                        &mut violations,
                    );
                }
            }
            Declaration::MlogServer(srv) => {
                for r in &srv.routes {
                    walk_container(
                        &r.body,
                        &format!("route {} {}", r.method, r.path),
                        &[],
                        &mut violations,
                    );
                }
            }
            Declaration::Hook(h) => {
                walk_container(
                    &h.body,
                    &format!("hook {:?}", h.phase),
                    &[],
                    &mut violations,
                );
            }
            Declaration::Test(t) => {
                walk_container(
                    &t.body,
                    &format!("test \"{}\"", t.name),
                    &[],
                    &mut violations,
                );
            }
            _ => {}
        }
    }

    violations
}

// ── Integrity: anti-injection decision gate (Наряд №327) ─────────────

/// One control-flow decision point whose deciding expression carries an
/// untrusted label (integrity axis — ADR-0154 §2.2).
#[derive(Debug, Clone)]
pub struct DecisionViolation {
    pub container: String,
    /// `"if" | "while" | "match"` — the kind of the decision point.
    pub kind: String,
    pub span: Span,
    /// The deciding expression's label.
    pub label: Label,
    /// The name of the untrusted source, when the deciding expression is
    /// a direct Source call; `<derived>` otherwise.
    pub source_name: String,
}

/// Name of the untrusted source behind a deciding expression: a direct
/// call of a №316 Source builtin names itself; everything else is
/// derived (the join poisoned the integrity — the exact origin is not
/// tracked at this precision; the boundary is documented).
fn decision_source_name(expr: &Expr, prov: &BTreeMap<String, String>) -> String {
    // Descend into the deciding expression to name the direct Source
    // call behind the untrusted label (the naryad: the message names
    // the untrusted SOURCE and the decision point). Variables bound to
    // a Source call carry their provenance through the bindings map.
    if let Expr::FnCall { name, .. } = expr {
        if matches!(
            classify(name).map(|c| c.role),
            Some(crate::builtins_classification::Role::Source)
        ) {
            return name.clone();
        }
    }
    if let Expr::Ident { name, .. } = expr {
        if let Some(origin) = prov.get(name) {
            return origin.clone();
        }
    }
    for sub in expr_operands(expr) {
        let found = decision_source_name(sub, prov);
        if found != "<derived>" {
            return found;
        }
    }
    "<derived>".to_string()
}

/// Direct sub-expressions of `expr` (one level).
fn expr_operands(expr: &Expr) -> Vec<&Expr> {
    match expr {
        Expr::BinaryOp { left, right, .. } => vec![left.as_ref(), right.as_ref()],
        Expr::IfElse {
            then_branch,
            else_branch,
            ..
        } => vec![then_branch.as_ref(), else_branch.as_ref()],
        Expr::FieldAccess { object, .. } => vec![object.as_ref()],
        Expr::IndexAccess { object, index, .. } => vec![object.as_ref(), index.as_ref()],
        Expr::Try { expr, .. } => vec![expr.as_ref()],
        Expr::List { items, .. } => items.iter().collect(),
        Expr::FnCall { args, .. } | Expr::QualifiedCall { args, .. } => args.iter().collect(),
        _ => vec![],
    }
}

/// The №327 anti-injection rule: data that DECIDES control flow must be
/// `trusted`. Untrusted data as DATA is legal — the gate fires only on
/// decision positions: `if`/`else if` conditions, `while` conditions,
/// `match` scrutinees. (Sink-target decisions are the №325 classes:
/// UNTRUSTED_EXEC_DECISION / UNTRUSTED_EGRESS_NETWORK.)
pub fn integrity_decision_violations(declarations: &[Declaration]) -> Vec<DecisionViolation> {
    let mut violations = Vec::new();

    // Seed environment mirrors the sink-clearance pass (№322 annotations
    // on params; entity initializers via the marker lексикон; otherwise
    // bottom).
    let mut seed: BTreeMap<String, Label> = BTreeMap::new();
    for decl in declarations {
        match decl {
            Declaration::EntitySimple(e) => {
                seed.insert(e.name.clone(), entity_seed_label(Some(&e.value)));
            }
            Declaration::EntityRecord(e) => {
                let strongest = e
                    .fields
                    .iter()
                    .map(|f| entity_seed_label(Some(&f.value)))
                    .fold(Label::bottom(), |a, b| a.join(&b));
                seed.insert(e.name.clone(), strongest);
            }
            _ => {}
        }
    }

    fn check_decision(
        expr: &Expr,
        kind: &str,
        container: &str,
        env: &BTreeMap<String, Label>,
        prov: &BTreeMap<String, String>,
        violations: &mut Vec<DecisionViolation>,
    ) {
        let label = sink_arg_label(expr, env, &BTreeMap::new());
        if label.integrity == crate::labels::Integrity::Untrusted {
            violations.push(DecisionViolation {
                container: container.to_string(),
                kind: kind.to_string(),
                span: expr.span().clone(),
                label,
                source_name: decision_source_name(expr, prov),
            });
        }
    }

    fn walk_stmts(
        stmts: &[Statement],
        container: &str,
        env: &mut BTreeMap<String, Label>,
        prov: &mut BTreeMap<String, String>,
        violations: &mut Vec<DecisionViolation>,
    ) {
        // Provenance: a variable bound to a DIRECT Source call keeps the
        // call's name; bindings derived from such a variable inherit it.
        fn prov_of(value: &Expr, prov: &BTreeMap<String, String>) -> Option<String> {
            match value {
                Expr::FnCall { name, .. } => {
                    if matches!(
                        classify(name).map(|c| c.role),
                        Some(crate::builtins_classification::Role::Source)
                    ) {
                        return Some(name.clone());
                    }
                    // A wrapper call derives from its args' provenance.
                    for a in args_of(value) {
                        if let Some(o) = prov_of(a, prov) {
                            return Some(format!("{o} (via {name})"));
                        }
                    }
                    None
                }
                Expr::Ident { name, .. } => prov.get(name).cloned(),
                Expr::BinaryOp { left, right, .. } => {
                    prov_of(left, prov).or_else(|| prov_of(right, prov))
                }
                Expr::FieldAccess { object, .. } => {
                    if let Expr::Ident { name, .. } = object.as_ref() {
                        return prov.get(name).cloned();
                    }
                    prov_of(object, prov)
                }
                _ => None,
            }
        }
        fn args_of(value: &Expr) -> Vec<&Expr> {
            match value {
                Expr::FnCall { args, .. } | Expr::QualifiedCall { args, .. } => {
                    args.iter().collect()
                }
                _ => vec![],
            }
        }
        for stmt in stmts {
            match stmt {
                Statement::LetBinding { name, value, .. }
                | Statement::Assign { name, value, .. } => {
                    // №327 integrity pass: origin labels are Trusted, so
                    // the empty origins map is sound here (integrity is
                    // unaffected by the conf-bearing origin labels).
                    let label = sink_arg_label(value, env, &BTreeMap::new());
                    env.insert(name.clone(), label);
                    if let Some(origin) = prov_of(value, prov) {
                        prov.insert(name.clone(), origin);
                    } else {
                        prov.remove(name);
                    }
                }
                Statement::Each {
                    variable,
                    iterable,
                    body,
                    ..
                }
                | Statement::EachWithIndex {
                    item_var: variable,
                    iterable,
                    body,
                    ..
                } => {
                    let it = sink_arg_label(iterable, env, &BTreeMap::new());
                    let mut env_body = env.clone();
                    env_body.insert(variable.clone(), it);
                    let mut prov_body = prov.clone();
                    if let Some(origin) = prov_of(iterable, prov) {
                        prov_body.insert(variable.clone(), origin);
                    }
                    walk_stmts(body, container, &mut env_body, &mut prov_body, violations);
                }
                Statement::While {
                    condition, body, ..
                } => {
                    check_decision(condition, "while", container, env, prov, violations);
                    let mut env_body = env.clone();
                    let mut prov_body = prov.clone();
                    walk_stmts(body, container, &mut env_body, &mut prov_body, violations);
                    for (k, v) in env_body {
                        let m = env.get(&k).cloned().unwrap_or_else(Label::bottom).join(&v);
                        env.insert(k, m);
                    }
                }
                Statement::IfElseBlock {
                    condition,
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    check_decision(condition, "if", container, env, prov, violations);
                    let mut env_then = env.clone();
                    let mut prov_then = prov.clone();
                    walk_stmts(
                        then_body,
                        container,
                        &mut env_then,
                        &mut prov_then,
                        violations,
                    );
                    let mut merged = env_then;
                    for (cond, b) in else_ifs {
                        check_decision(cond, "if", container, env, prov, violations);
                        let mut env_b = env.clone();
                        let mut prov_b = prov.clone();
                        walk_stmts(b, container, &mut env_b, &mut prov_b, violations);
                        for (k, v) in env_b {
                            let m = merged
                                .get(&k)
                                .cloned()
                                .unwrap_or_else(Label::bottom)
                                .join(&v);
                            merged.insert(k, m);
                        }
                    }
                    if let Some(eb) = else_body {
                        let mut env_e = env.clone();
                        let mut prov_e = prov.clone();
                        walk_stmts(eb, container, &mut env_e, &mut prov_e, violations);
                        for (k, v) in env_e {
                            let m = merged
                                .get(&k)
                                .cloned()
                                .unwrap_or_else(Label::bottom)
                                .join(&v);
                            merged.insert(k, m);
                        }
                    } else {
                        for (k, v) in env.clone() {
                            let m = merged
                                .get(&k)
                                .cloned()
                                .unwrap_or_else(Label::bottom)
                                .join(&v);
                            merged.insert(k, m);
                        }
                    }
                    *env = merged;
                }
                Statement::IfThen {
                    condition, body, ..
                } => {
                    check_decision(condition, "if", container, env, prov, violations);
                    let mut env_t = env.clone();
                    let mut prov_t = prov.clone();
                    walk_stmts(body, container, &mut env_t, &mut prov_t, violations);
                    for (k, v) in env_t {
                        let m = env.get(&k).cloned().unwrap_or_else(Label::bottom).join(&v);
                        env.insert(k, m);
                    }
                }
                Statement::Return { .. } | Statement::ExprStmt { .. } => {}
                Statement::Match {
                    scrutinee,
                    arms,
                    else_body,
                    ..
                } => {
                    check_decision(scrutinee, "match", container, env, prov, violations);
                    let mut merged = env.clone();
                    for arm in arms {
                        let b = match arm {
                            MatchArm::Exact(_, b)
                            | MatchArm::StartsWith(_, b)
                            | MatchArm::Contains(_, b)
                            | MatchArm::Compare(_, _, b) => b,
                        };
                        let mut env_b = env.clone();
                        let mut prov_b = prov.clone();
                        walk_stmts(b, container, &mut env_b, &mut prov_b, violations);
                        for (k, v) in env_b {
                            let m = merged
                                .get(&k)
                                .cloned()
                                .unwrap_or_else(Label::bottom)
                                .join(&v);
                            merged.insert(k, m);
                        }
                    }
                    if let Some(eb) = else_body {
                        let mut env_e = env.clone();
                        let mut prov_e = prov.clone();
                        walk_stmts(eb, container, &mut env_e, &mut prov_e, violations);
                        for (k, v) in env_e {
                            let m = merged
                                .get(&k)
                                .cloned()
                                .unwrap_or_else(Label::bottom)
                                .join(&v);
                            merged.insert(k, m);
                        }
                    } else {
                        for (k, v) in env.clone() {
                            let m = merged
                                .get(&k)
                                .cloned()
                                .unwrap_or_else(Label::bottom)
                                .join(&v);
                            merged.insert(k, m);
                        }
                    }
                    *env = merged;
                }
                Statement::Memorize(_) | Statement::Forget(_) | Statement::Relate(_) => {}
                Statement::Break | Statement::Continue => {}
            }
        }
    }

    let walk_container = |stmts: &[Statement],
                          container: &str,
                          params: &[crate::ast::Param],
                          violations: &mut Vec<DecisionViolation>| {
        let mut env = seed.clone();
        let mut prov: BTreeMap<String, String> = BTreeMap::new();
        for p in params {
            let l = match &p.label {
                Some(ann) => Label::parse(&ann.raw).unwrap_or_default(),
                None => Label::bottom(),
            };
            env.insert(p.name.clone(), l);
        }
        walk_stmts(stmts, container, &mut env, &mut prov, violations);
    };

    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => {
                walk_container(
                    &p.body,
                    &format!("pattern {}", p.name),
                    &p.params,
                    &mut violations,
                );
            }
            Declaration::Tool(t) => {
                for m in &t.methods {
                    walk_container(
                        &m.body,
                        &format!("tool {}.{}", t.name, m.name),
                        &m.params,
                        &mut violations,
                    );
                }
            }
            Declaration::MlogServer(srv) => {
                for r in &srv.routes {
                    walk_container(
                        &r.body,
                        &format!("route {} {}", r.method, r.path),
                        &[],
                        &mut violations,
                    );
                }
            }
            Declaration::Hook(h) => {
                walk_container(
                    &h.body,
                    &format!("hook {:?}", h.phase),
                    &[],
                    &mut violations,
                );
            }
            Declaration::Test(t) => {
                walk_container(
                    &t.body,
                    &format!("test \"{}\"", t.name),
                    &[],
                    &mut violations,
                );
            }
            _ => {}
        }
    }

    violations
}

/// Perform semantic analysis on a list of declarations (without executing them).
/// Validates:
///   - Entity types referenced in records exist
///   - Field initializers reference valid fields
///   - Patterns/learnables invoked in flows exist
///   - Flow branch targets are known patterns
///   - Duplicate entity/pattern/flow names
///   - Rule targets reference existing entities
///   - Adapt/mutate targets reference existing learnable patterns
///   - Relate/sandbox declarations are well-formed
///   - MlogServer middleware names are valid (Phase 6.1)
///   - Route methods are valid HTTP methods (Phase 6.1)
///   - Template return type is Html (Phase 6.2)
///   - Opaque types used in correct contexts (Phase 6.2–6.5)
pub fn check_program(declarations: &[Declaration]) -> AnalysisResult {
    let mut result = AnalysisResult::default();
    let mut entity_types: HashSet<String> = HashSet::new();
    let mut entity_names: HashSet<String> = HashSet::new();
    let mut pattern_names: HashSet<String> = HashSet::new();
    let mut learnable_names: HashSet<String> = HashSet::new();
    let mut flow_names: HashSet<String> = HashSet::new();
    // Наряд №238 (Vision R4.1): vision declaration names (duplicate check).
    let mut vision_names: HashSet<String> = HashSet::new();
    let builtin_names = crate::builtins::builtin_name_set();
    let mut role_names: HashSet<String> = HashSet::new();
    let mut pattern_param_counts: HashSet<(String, usize)> = HashSet::new();
    // Наряд №181 (ADR-0117): reflex declarations — name → labels set.
    // Used to validate distill_to references and labels-non-empty rule.
    let mut reflex_decls: HashMap<String, Vec<String>> = HashMap::new();

    // Наряд №119: build type alias map and collect errors
    let (type_alias_map, alias_errors) = build_type_alias_map(declarations);
    for e in alias_errors {
        // build_type_alias_map returns Vec<String> (defined in ast.rs,
        // also consumed by interpreter/execution.rs). The errors don't
        // carry a span — `SpannedError::at_line(e, 0)` produces a
        // Span::unknown() which the LSP layer maps to line 0 (the
        // previous hardcoded fallback behavior).
        result.errors.push(SpannedError::at_line(e, 0));
    }
    let alias_names: HashSet<String> = type_alias_map.keys().cloned().collect();

    // Наряд №322 (ADR-0154): validate label annotations everywhere the
    // grammar allows them. This is what makes the annotation "visible to
    // semantics" — the label carrier (`ast::LabelAnn`) is parsed with a
    // span, and unknown words fail here, not silently later.
    for decl in declarations {
        validate_decl_labels(decl, &mut result.errors);
    }

    // Наряд №324 (ADR-0154 §9): effect trail in pattern signatures.
    // Validates the trail words (io|audit), computes the factual body
    // effects of every pattern/tool-method/learnable (interprocedural
    // fixpoint over the call graph — converges on recursion), and gates
    // every DECLARED trail: factual ⊑ declared, excess = loud error.
    // Patterns without a declared trail are ungated (zero delta).
    check_effect_trails(declarations, &mut result.errors);

    // Наряд №331 (ADR-0162 §2.5): media handles are opaque — any field
    // access on a media-typed expression is a COMPILE error; bytes are
    // reachable only through the sanctioned materialization sink
    // (media_save, №325-gated).
    check_media_handle_opacity(declarations, &mut result.errors);

    // Наряд №332 (ADR-0164): the origin chain — a media handle without
    // origin is not constructed (§7.4 rule); origin declarations are
    // validated loudly (unknown kinds/fields/labels).
    check_media_origin_chain(declarations, &mut result.errors);

    // Наряд №325 (ADR-0161): validate the compatibility profile shape —
    // unknown profile names / options / egress modes are loud errors
    // (a compat-profile mistake must never be a silent no-op).
    for decl in declarations {
        if let Declaration::Profile(p) = decl {
            if let Err(e) = crate::profile::validate(p) {
                result.errors.push(SpannedError::at(e, p.span.clone()));
            }
        }
    }

    // Наряд №336 (ADR-0165 §2.4): the BackendSelect ladder companion
    // check — statically-visible ladders are verified against the №333
    // registry SSOT (unknown rung / class mismatch / duplicates), and a
    // `device { mode: production }` profile refuses unverifiable
    // (PendingNo334) rungs at BUILD time.
    check_backend_select_ladders(declarations, &mut result.errors);

    // First pass: collect all declarations (names)
    for decl in declarations {
        match decl {
            Declaration::EntityType(e) => {
                if !entity_types.insert(e.name.clone()) {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!("duplicate entity type: {}", e.name),
                    ));
                }
            }
            Declaration::EntityRecord(e) => {
                if !entity_names.insert(e.name.clone()) {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!("duplicate entity: {}", e.name),
                    ));
                }
            }
            Declaration::EntitySimple(e) => {
                if !entity_names.insert(e.name.clone()) {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!("duplicate entity: {}", e.name),
                    ));
                }
            }
            Declaration::Pattern(p) => {
                if !pattern_names.insert(p.name.clone()) {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!("duplicate pattern: {}", p.name),
                    ));
                }
                pattern_param_counts.insert((p.name.clone(), p.params.len()));
            }
            Declaration::LearnablePattern(lp) => {
                if !learnable_names.insert(lp.name.clone()) {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!("duplicate learnable pattern: {}", lp.name),
                    ));
                }
                pattern_param_counts.insert((lp.name.clone(), lp.params.len()));
            }
            // Наряд №181: collect reflex declarations for cross-reference check.
            Declaration::Reflex(r) => {
                if reflex_decls
                    .insert(r.name.clone(), r.labels.clone())
                    .is_some()
                {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!("duplicate reflex declaration: {}", r.name),
                    ));
                }
            }
            Declaration::Flow(f) => {
                if !flow_names.insert(f.name.clone()) {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!("duplicate flow: {}", f.name),
                    ));
                }
            }
            // Наряд №238 (Vision R4.1): duplicate vision declaration names
            // are a semantic error (same лекал as reflex/pattern/flow).
            Declaration::Vision(v) => {
                if !vision_names.insert(v.name.clone()) {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!("duplicate vision declaration: {}", v.name),
                    ));
                }
            }
            Declaration::Template(t) => {
                // Templates are also callable as render targets
                pattern_names.insert(t.name.clone());
                if is_opaque_type(&t.return_type) && t.return_type != "Html" {
                    result.errors.push(with_line_prefix(decl, format!(
                        "template '{}' returns opaque type '{}' — only Html is supported as template return type",
                        t.name, t.return_type
                    )));
                }
            }
            _ => {}
        }
    }

    // Second pass: cross-reference validation
    for decl in declarations {
        match decl {
            // Наряд №181 (ADR-0117) Block 2: validate distill_to reference.
            // `distill_to: X` requires:
            //   1. X is declared as `reflex X { ... }` in the same scope
            //   2. If the pattern returns String, the reflex must have a
            //      non-empty `labels` list (closed-label enforcement —
            //      ADR-0117 §3 explicitly excludes free-form generation).
            Declaration::LearnablePattern(lp) => {
                if let Some(distill_target) = &lp.distill_to {
                    match reflex_decls.get(distill_target) {
                        None => {
                            result.errors.push(with_line_prefix(
                                decl,
                                format!(
                                    "learnable pattern '{}': distill_to references '{}' \
                                     which is not declared as `reflex {} {{ ... }}`",
                                    lp.name, distill_target, distill_target
                                ),
                            ));
                        }
                        Some(labels) => {
                            // ADR-0117 §3 enforcement: String-returning patterns
                            // distilling to a reflex with empty labels is rejected
                            // (free-form generation is permanently out of scope).
                            if lp.return_type == "String" && labels.is_empty() {
                                result.errors.push(with_line_prefix(
                                    decl,
                                    format!(
                                        "learnable pattern '{}': distill_to '{}' has empty labels list. \
                                         Free-form text distillation is out of scope (ADR-0117 §3) — \
                                         reflex must declare a closed label set: \
                                         `reflex {} {{ labels: [\"a\", \"b\", ...] ... }}`",
                                        lp.name, distill_target, distill_target
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
            Declaration::EntityRecord(e) => {
                if !entity_types.contains(&e.type_name) {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!(
                            "entity '{}' references unknown type '{}'",
                            e.name, e.type_name
                        ),
                    ));
                }
                if let Some(fields) = get_type_fields(declarations, &e.type_name) {
                    for init in &e.fields {
                        if !fields.contains(&init.name.as_str()) {
                            result.errors.push(with_line_prefix(
                                decl,
                                format!(
                                    "entity '{}' initializes unknown field '{}' on type '{}'",
                                    e.name, init.name, e.type_name
                                ),
                            ));
                        }
                    }
                }
            }
            Declaration::EntitySimple(e) => {
                let known_primitives = [
                    "String",
                    "Float",
                    "Bool",
                    "Html",
                    "Query",
                    "Secret",
                    "Encrypted",
                    "Hash",
                    "Session",
                ];
                // Наряд №119: resolve type alias before checking
                let resolved = type_alias_map
                    .get(&e.type_name)
                    .map(|s| s.as_str())
                    .unwrap_or(&e.type_name);
                if !known_primitives.contains(&resolved)
                    && !entity_types.contains(resolved)
                    && !alias_names.contains(&e.type_name)
                {
                    result.warnings.push(with_line_prefix(
                        decl,
                        format!(
                            "entity '{}' uses undeclared type '{}' (may be a forward reference)",
                            e.name, e.type_name
                        ),
                    ));
                }
            }
            Declaration::Rule(r) => {
                if let Expr::Ident { name, .. } = &r.target {
                    if !entity_names.contains(name) {
                        result.errors.push(with_line_prefix(
                            decl,
                            format!("rule target '{}' references undefined entity", name),
                        ));
                    }
                }
            }
            Declaration::Adapt(a) => {
                if !learnable_names.contains(&a.pattern_name) {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!("adapt: learnable pattern '{}' not found", a.pattern_name),
                    ));
                }
            }
            Declaration::Mutate(m) => {
                if !learnable_names.contains(&m.pattern_name) {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!("mutate: learnable pattern '{}' not found", m.pattern_name),
                    ));
                }
            }
            Declaration::Eval(e) => {
                if !learnable_names.contains(&e.pattern_name) {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!("eval: learnable pattern '{}' not found", e.pattern_name),
                    ));
                }
                if e.dataset.is_empty() {
                    result.warnings.push(with_line_prefix(
                        decl,
                        format!(
                            "eval '{}': dataset is empty — eval will trivially pass",
                            e.pattern_name
                        ),
                    ));
                }
            }
            Declaration::Flow(f) => {
                for step in &f.pipeline {
                    let known = pattern_names.contains(step)
                        || learnable_names.contains(step)
                        || builtin_names.contains(step)
                        || step == "recall";
                    if !known {
                        let has_branch_def = f.branch_defs.iter().any(|(name, _)| name == step);
                        if !has_branch_def {
                            result.errors.push(with_line_prefix(decl, format!(
                                "flow '{}': pipeline step '{}' is not a known pattern, builtin, or branch definition",
                                f.name, step
                            )));
                        }
                    }
                }
                for (_, branches) in &f.branch_defs {
                    for branch in branches {
                        if !pattern_names.contains(&branch.target)
                            && !learnable_names.contains(&branch.target)
                            && !builtin_names.contains(&branch.target)
                        {
                            result.errors.push(with_line_prefix(
                                decl,
                                format!(
                                    "flow '{}': branch '{}' target '{}' is not a known pattern",
                                    f.name, branch.label, branch.target
                                ),
                            ));
                        }
                    }
                }
            }
            Declaration::Pattern(p) => {
                // Walk expression tree: check arity and undefined functions
                for stmt in &p.body {
                    check_stmt_exprs(
                        stmt,
                        &builtin_names,
                        &pattern_param_counts,
                        &learnable_names,
                        &mut result.errors,
                    );
                }
                // Наряд №264: static immutability enforcement — assignment to
                // a variable not bound with `let mut` is an error here, so
                // `mlog check` catches before any backend runs it. Previously
                // the tree-walking interpreter rejected such assignments at
                // RUNTIME (contract since №14, examples/p30_assign_immutable)
                // while `mlog check` passed and the VM compiled the
                // assignment silently — three backends, three answers.
                check_pattern_mutability(&p.body, &mut result.errors);
            }
            // Phase 6.1: Validate mlogserver block
            Declaration::MlogServer(srv) => {
                // Validate middleware names
                for mw in &srv.middleware {
                    if !VALID_MIDDLEWARE.contains(&mw.as_str()) {
                        result.errors.push(with_line_prefix(
                            decl,
                            format!(
                                "mlogserver: unknown middleware '{}'. Valid: {:?}",
                                mw, VALID_MIDDLEWARE
                            ),
                        ));
                    }
                }
                // Validate route methods and role references
                for route in &srv.routes {
                    if !VALID_METHODS.contains(&route.method.as_str()) {
                        result.errors.push(with_line_prefix(
                            decl,
                            format!(
                                "route '{}': unknown HTTP method '{}'. Valid: {:?}",
                                route.path, route.method, VALID_METHODS
                            ),
                        ));
                    }
                    for role in &route.requires {
                        // Collect role names for cross-reference
                        role_names.insert(role.clone());
                    }
                }
                // Warn if no security_headers middleware
                if !srv.middleware.contains(&"security_headers".to_string()) {
                    result.warnings.push(
                        with_line_prefix(decl, "mlogserver: no 'security_headers' middleware — recommend adding it for OWASP compliance".to_string())
                    );
                }
                // Warn if POST routes but no csrf middleware
                let has_post = srv
                    .routes
                    .iter()
                    .any(|r| r.method == "POST" || r.method == "PUT" || r.method == "DELETE");
                if has_post && !srv.middleware.contains(&"csrf".to_string()) {
                    result.warnings.push(
                        with_line_prefix(decl, "mlogserver: has mutating routes but no 'csrf' middleware — recommend adding it".to_string())
                    );
                }
                // Наряд №264: route bodies get the same immutability walk as
                // pattern bodies — TW executes route bodies through
                // eval_statements (server.rs), so the `let mut` contract
                // applies there identically; the static pass now catches the
                // violation at `mlog check` time for both serve backends.
                for route in &srv.routes {
                    check_pattern_mutability(&route.body, &mut result.errors);
                }
            }
            // Test declarations: no semantic checks needed (statements inside are checked by patterns)
            Declaration::Test(_) => {}
            // Наряд №238 (Vision R4.1): vision declaration validation
            // (Block 2). Parser already validated the field set and the
            // policy/profile enums — semantic does NOT re-validate enums
            // (наряд Block 1.3), it checks the model SSOT list and the
            // numeric contracts. All errors carry the declaration span and
            // name the field (Block 2.4 — no silent corrections).
            Declaration::Vision(v) => {
                // Block 2.1: model must be in the SSOT list
                // (`crate::vision::KNOWN_VISION_MODELS`, R4.1 = exactly
                // ["z-image-turbo"]).
                if !crate::vision::KNOWN_VISION_MODELS.contains(&v.model.as_str()) {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!(
                            "vision '{}': unknown model '{}' (known models: {})",
                            v.name,
                            v.model,
                            crate::vision::KNOWN_VISION_MODELS.join(", ")
                        ),
                    ));
                }
                // Block 2.2: steps >= 1. The value 8 is the recommended
                // distilled-NFE count for the z-image-turbo wedge (ADR-0122);
                // steps != 8 is an audit-warning, NOT an error.
                if v.steps < 1 {
                    result.errors.push(with_line_prefix(
                        decl,
                        format!("vision '{}': steps must be >= 1, got {}", v.name, v.steps),
                    ));
                } else if v.steps != 8 {
                    result.warnings.push(with_line_prefix(
                        decl,
                        format!(
                            "vision '{}': steps = {} != 8 (recommended distilled-NFE for z-image-turbo)",
                            v.name, v.steps
                        ),
                    ));
                }
                // Block 2.2: width/height — multiples of 16, range
                // 256..=4096 (VAE latent constraint).
                for (field, value) in [("width", v.width), ("height", v.height)] {
                    if value % 16 != 0 || !(256..=4096).contains(&value) {
                        result.errors.push(with_line_prefix(
                            decl,
                            format!(
                                "vision '{}': {} = {} must be a multiple of 16 in range 256..=4096 (VAE latent constraint)",
                                v.name, field, value
                            ),
                        ));
                    }
                }
                // Block 2.2: seed — any u64 is valid (parser guarantees the
                // range); no further check.
            }
            _ => {}
        }
    }

    // ── Наряд №98: Category A audit → compiler errors ──
    // SQL_DYNAMIC, SECRET_LEAK, HTML_INJECTION are structural security
    // invariants — a finding is never a legitimate false positive.
    // These run in `mlog check`/`run`/`serve`/`compile`, not just `mlog audit`.
    // Source text is not available here (we only have AST), so we pass ""
    // for line-number resolution; the check_id and message are sufficient.
    let cat_a_findings = audit_category_a(declarations, "");
    for finding in &cat_a_findings {
        match finding.severity {
            Severity::Error => {
                result.errors.push(SpannedError::at_line(
                    format!("[{}] {}", finding.check_id, finding.message),
                    finding.line,
                ));
            }
            Severity::Warning => {
                // check_html_injection currently emits Warning;
                // promote to Error per Наряд №98 Block 1 classification.
                result.errors.push(SpannedError::at_line(
                    format!("[{}] {}", finding.check_id, finding.message),
                    finding.line,
                ));
            }
            Severity::Info => {}
        }
    }

    // ── Нарjad №74: SVG/HTML security lint (ADR-0102) ──
    // AST-level analysis: detect potential injection vectors that could
    // bypass runtime escaping. See `svg_security_lint` docstring below.
    svg_security_lint(declarations, &mut result);

    result
}

// ── Наряд №74: SVG/HTML Security Lint ────────────────────────────────
//
// Walks the AST of every declaration and inspects all `Expr::FnCall`
// nodes. For each call to an SVG/HTML-emitting builtin (svg_text,
// svg_callout, svg_path, svg_canvas, svg_group, chart_*, diagram_*,
// html_response, escape_html), it inspects the string-literal arguments.
//
// Findings:
//
//   ERROR (potential bypass):
//     A string literal containing `<script`, `javascript:`, or `on\w+=`
//     is passed to a builtin that does NOT auto-escape that argument
//     (e.g. svg_path's `d` argument is structural and not escaped).
//     Also: a `<script` literal appearing in any string concatenation
//     that ends up in an HTML context.
//
//   WARNING (suspicious but auto-escaped):
//     A string literal containing `<script>`, `on\w+=`, etc. passed to
//     a builtin that DOES auto-escape (svg_text content, svg_callout
//     text). Runtime will escape it correctly, but the source intent
//     looks like an attempted injection — worth flagging for review.
//
// This is defense-in-depth: runtime escaping (escape_html_chars in
// svg_text, svg_callout) is the primary barrier. The lint catches the
// case where an attacker could bypass escaping by passing the payload
// to a non-escaping argument (svg_path d, svg_canvas viewbox, etc.).
//
// Whitelist: Google Fonts URLs ("https://fonts.googleapis.com/...")
// are explicitly permitted in href/src contexts — the only external
// resource allowed (matches the source repo's self_check.py rule).

/// Builtins whose string arguments are auto-escaped at runtime.
/// String literals with `<script>` here generate a WARNING (suspicious
/// but safe — runtime will escape).
///
/// chart_bar / chart_donut / chart_line / chart_area accept user labels
/// inside `data: List<Struct{label, value}>` at arg 0. Their labels are
/// escaped via escape_html_chars at runtime (defense-in-depth).
/// chart_scatter uses `List<Struct{x, y, label?}>` — same `label` key,
/// but optional and at a different struct position. The walker has a
/// special case (scan_chart_labels) that scans the list-of-structs
/// pattern by field NAME, so it works uniformly across all five shapes.
/// chart_boxplot uses `List<Struct{label, values}>` — same `label` key
/// in a list-of-structs, so scan_chart_labels covers it without changes.
/// chart_radar uses a DIFFERENT top-level shape (Struct{axes, series},
/// not List<Struct>), so it has its own scanner (scan_radar_labels).
/// chart_heatmap is intentionally NOT in this list — its data is purely
/// numeric (List<List<Float>>), there is no user text to scan.
const SVG_AUTO_ESCAPE_BUILTINS: &[&str] = &[
    "svg_text",
    "svg_callout",
    "chart_bar",
    "chart_donut",
    "chart_line",
    "chart_scatter",
    "chart_area",
    "chart_radar",
    "chart_boxplot",
    // Наряд №81 Block 6: diagrams accept user text (label/title/description).
    //   diagram_tree / diagram_org_chart — recursive Struct{label, children}.
    //   diagram_flowchart — Struct{nodes: [{id,label}], edges: [{from,to,label?}]}.
    //   diagram_layers — List<Struct{label, description?}>.
    // All four escape text via escape_html_chars at runtime (svg.rs).
    // AST lint scans label literals as WARNINGs (defense-in-depth).
    "diagram_tree",
    "diagram_org_chart",
    "diagram_flowchart",
    "diagram_layers",
    // Наряд №82 Block 6: temporal & process diagrams.
    //   diagram_sequence  — Struct{actors: List<String>, messages: [{from,to,label?}]}.
    //     actors[] is a List<String> (NOT List<Struct>) — special scanner.
    //     messages[].label is the rendered text (from/to are idents, scanned
    //     defensively).
    //   diagram_timeline  — List<Struct{date, label, description?}> — flat
    //     list pattern, same shape as diagram_layers (3 string fields, not 2).
    //   diagram_gantt     — List<Struct{task, start, duration}> — task is
    //     the only string field; start/duration are floats, never rendered.
    //   diagram_process   — List<Struct{label, description?}> — identical
    //     shape to diagram_layers (reuses scan_layers_labels logic by name).
    //   diagram_loop      — List<Struct{label, description?}> — same shape
    //     as diagram_layers/diagram_process.
    // All five escape text via escape_html_chars at runtime (svg.rs).
    "diagram_sequence",
    "diagram_timeline",
    "diagram_gantt",
    "diagram_process",
    "diagram_loop",
    // Наряд №83 Block 6: sets & comparison diagrams.
    //   diagram_venn     — Struct{circles: [{label, value?}], overlap_label?}.
    //     circles[].label is rendered; overlap_label is a TOP-LEVEL field
    //     (NOT inside the list) — easy to forget, called out explicitly in
    //     the spec. Special scanner scan_venn_labels covers both.
    //   diagram_quadrant — Struct{x_axis_label, y_axis_label, items: [{label, x, y}]}.
    //     BOTH axis labels are TOP-LEVEL fields (not in items[]) — same
    //     "easy to forget" category as overlap_label. items[].label is the
    //     only rendered text in the list (x/y are floats). Special scanner
    //     scan_quadrant_labels covers all three.
    //   diagram_pyramid  — List<Struct{label, value?}> — same flat shape
    //     as diagram_layers (label rendered, value is float). REUSES
    //     scan_layers_labels (no new scanner for an identical shape).
    //   diagram_nested   — List<Struct{label, value?}> — identical shape
    //     to diagram_pyramid. REUSES scan_layers_labels.
    //   diagram_medallion — List<Struct{icon?, label, value?}>. label is
    //     rendered; icon is a controlled enum (validated against the 10
    //     svg_icon names at runtime), NOT user free-form text — explicitly
    //     NOT scanned per the spec. value is float. New scanner
    //     scan_medallion_labels checks only `label`.
    "diagram_venn",
    "diagram_quadrant",
    "diagram_pyramid",
    "diagram_nested",
    "diagram_medallion",
    // Наряд №84 Block 7: data & state diagrams.
    //   diagram_er — Struct{entities: [{name, fields: List<String>}], relations: [{from,to,label?}]}.
    //     entities[].name and relations[].label are rendered as text.
    //     entities[].fields is a List<String> NESTED INSIDE a struct field —
    //     this is the THIRD nesting form encountered in the SVG suite
    //     (1st: top-level List<String> like diagram_sequence.actors in Н82;
    //      2nd: List<Struct> like diagram_layers in Н81;
    //      3rd: List<String> inside a struct field, here). The scanner
    //     scan_er_labels walks both levels: per-entity name + per-field string.
    //   diagram_state — Struct{states: List<String>, transitions: [{from,to,label?}], initial?}.
    //     states[] is List<String> (same as diagram_sequence.actors — scan
    //     each StringLit directly). transitions[].label is rendered.
    //     `initial` is a TOP-LEVEL String? field (like diagram_venn.overlap_label).
    //   diagram_swimlane — Struct{lanes: List<String>, steps: [{lane,label,order}]}.
    //     lanes[] is List<String>. steps[].label is rendered (lane is an
    //     identifier, scanned defensively). steps[].order is Float, skipped.
    //   diagram_data_flow / diagram_high_level / diagram_architecture —
    //     Struct{nodes:[{id,label,icon?}], edges:[{from,to,label?}]}.
    //     Same shape as diagram_flowchart (Н81) — REUSES scan_flowchart_labels
    //     (no new scanner for an identical shape, per spec: "переиспользовать
    //     сканер, не писать заново для каждой из трёх").
    //     For diagram_architecture, the `icon` field is a controlled enum
    //     (validated against svg_icon's 10 names at runtime) — NOT scanned,
    //     same decision as diagram_medallion.
    "diagram_er",
    "diagram_state",
    "diagram_swimlane",
    "diagram_data_flow",
    "diagram_high_level",
    "diagram_architecture",
];

/// Builtins whose string arguments are NOT auto-escaped (structural).
/// String literals with `<script>` here generate an ERROR (injection
/// vector — runtime cannot catch it).
///
/// [Наряд №92 Category 2] These 4 builtins are covered by this separate,
/// stricter lint path (ERROR not WARNING). Each has runtime validation
/// that rejects dangerous inputs:
///   svg_path          — d argument validated: rejects < and > chars
///   svg_canvas        — viewbox validated: must be exactly 4 numbers
///   svg_group         — transform escaped via escape_attr(); children
///                        are trusted prior-builtin outputs (not user text)
///   svg_sketchy_filter — id escaped via escape_attr()
const SVG_NO_ESCAPE_BUILTINS: &[&str] = &[
    "svg_path",           // d is path-data mini-language, not escaped
    "svg_canvas",         // viewbox is structural, not escaped
    "svg_group",          // transform is structural, not escaped
    "svg_sketchy_filter", // id is structural, validated but not escaped as text
];

// [Наряд №92 Category 1] Builtins intentionally NOT in either
// SVG_AUTO_ESCAPE_BUILTINS or SVG_NO_ESCAPE_BUILTINS.
//
// None of these accept free-form user text that is inserted as-is
// into SVG markup. Each is excluded for a documented reason:
//
//   svg_rect / svg_circle / svg_line — attribute values only (fill,
//     stroke, color strings), escaped at runtime via escape_attr().
//     No text *content* inserted between XML tags.
//
//   svg_icon — name is a validated enum (10 known icon names, error
//     on unknown); color is an escaped attribute. No free-form text
//     injection surface.
//
//   diagram_style — returns a DiagramStyle Struct, NOT SVG markup.
//     Color strings are consumed and escaped by downstream builtins
//     (chart_*/diagram_*), which are in SVG_AUTO_ESCAPE_BUILTINS.
//
//   color_palette — returns a DiagramStyle Struct, NOT SVG markup.
//     intent/mode are validated enums; no SVG output at all.
//
//   svg_generate — kind/intent are validated enums. Procedural
//     background generation with no free-form text output.
//
//   svg_canvas_preset — preset_name is a validated enum; viewbox
//     validated as 4 numbers; children are trusted prior-builtin
//     outputs (same trust model as svg_group/svg_canvas children).
//
//   chart_heatmap — pure numeric grid (List<List<Float>>). No user
//     text at all. Documented exclusion per Наряд №79.
//
//   infographic_qa — read-only analysis of an SVG string, produces
//     no new markup. Documented exclusion per Наряд №89.
//
//   template_render — template is trusted code; data values escaped
//     at runtime via escape_html_chars. Documented exclusion per
//     Наряд №86.
//
//   НАРЯД №117 string builtins (trim_start, trim_end, truncate,
//     slugify, word_wrap, capitalize, title_case) — pure string
//     transformations. No SVG markup produced, no user text
//     inserted into HTML/SVG. Not applicable to SVG escape
//     classification.
//
//   НАРЯД №118 collection builtins (unique, chunk, sort) — pure
//     list transformations. No SVG/HTML markup produced.
//     Not applicable to SVG escape classification.

// Argument indices that are auto-escaped within SVG_AUTO_ESCAPE_BUILTINS.
// For svg_text: arg 2 (content). For svg_callout: arg 0 (text).
fn auto_escaped_arg_index(builtin: &str) -> Option<usize> {
    match builtin {
        "svg_text" => Some(2),
        "svg_callout" => Some(0),
        _ => None,
    }
}

/// Detect potential XSS/injection payload in a string literal.
/// Returns Some(reason) if the string looks dangerous.
fn detect_xss_payload(s: &str) -> Option<&'static str> {
    let lower = s.to_lowercase();
    if lower.contains("<script") {
        return Some("contains <script> tag");
    }
    if lower.contains("javascript:") {
        return Some("contains javascript: URL");
    }
    // on<event>= attributes: onclick=, onload=, onerror=, etc.
    // Match `on` followed by 2+ letters followed by `=`
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 4 < bytes.len() {
        if bytes[i] == b'o' && bytes[i + 1] == b'n' {
            // Check that what follows is letters then '='
            let mut j = i + 2;
            let mut letter_count = 0;
            while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
                letter_count += 1;
                j += 1;
            }
            if letter_count >= 2 && j < bytes.len() && bytes[j] == b'=' {
                return Some("contains onX event handler attribute");
            }
        }
        i += 1;
    }
    None
}

/// Check if a URL is on the whitelist (Google Fonts only).
fn is_whitelisted_url(url: &str) -> bool {
    let trimmed = url.trim();
    // Allow Google Fonts CSS and font files
    if trimmed.starts_with("https://fonts.googleapis.com/") {
        return true;
    }
    if trimmed.starts_with("https://fonts.gstatic.com/") {
        return true;
    }
    // Allow data: URIs for SVG inline (common for icons)
    if trimmed.starts_with("data:image/svg+xml") {
        return true;
    }
    false
}

/// Scan chart_* `data` arg (arg 0) for XSS payloads in label string
/// literals. The data arg is a List literal of Struct literals:
///   chart_bar / chart_donut / chart_line / chart_area:
///     [{label: "...", value: 10.0}, ...]
///   chart_scatter:
///     [{x: 1.0, y: 2.0, label: "..."}, ...]   (label OPTIONAL)
///   chart_boxplot:
///     [{label: "...", values: [1.0, 2.0, ...]}, ...]
/// For each struct, we look up the `label` field BY NAME (not position).
/// This makes the scanner shape-agnostic: it catches <script> in label
/// regardless of whether label is the first, second, or third field, and
/// regardless of whether other fields (x, y, value, values) are present.
/// If the struct has no `label` field at all (legal for chart_scatter),
/// the scanner simply skips it — no label, no injection vector.
///
/// If the `label` value is a StringLit with an XSS payload, we emit a
/// WARNING (runtime escapes label text via escape_html_chars — this is
/// a defense-in-depth review hint, not a hard error).
fn scan_chart_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::List { items, .. }) = args.first() {
        for item in items {
            if let Expr::StructLit { fields, .. } = item {
                if let Some(label_expr) = fields.get("label") {
                    if let Expr::StringLit { value: s, .. } = label_expr {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(SpannedError::at_expr(
                                label_expr,
                                format!(
                                    "security: {} data[].label string literal {} — runtime will escape, but review intent",
                                    fn_name, reason
                                ),
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Scan chart_radar `data` arg (arg 0) for XSS payloads. Radar has a
/// DIFFERENT top-level shape from other chart_* builtins — it's a
/// Struct with two List fields, not a List of Structs:
///   Struct {
///     axes:  List<String>,                  // scan each StringLit
///     series: List<Struct{name, values}>,   // scan each series.name
///   }
///
/// We look up `axes` and `series` by field name on the top-level
/// StructLit. For `axes`, the elements are StringLits directly (no
/// struct wrapping) — so we scan them in place. For `series`, each
/// element is a StructLit and we look up its `name` field by key
/// (same approach as scan_chart_labels, just one struct level deeper).
///
/// If any string literal carries an XSS payload, we emit a WARNING
/// (runtime escapes via escape_html_chars — defense-in-depth, not a
/// hard error). The warning identifies WHICH field is suspicious
/// (axes vs series[].name) so the caller can locate the input.
fn scan_radar_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::StructLit { fields, .. }) = args.first() {
        // axes: List<String> — scan each StringLit directly
        if let Some(Expr::List {
            items: axes_items, ..
        }) = fields.get("axes")
        {
            for axis in axes_items {
                if let Expr::StringLit { value: s, .. } = axis {
                    if let Some(reason) = detect_xss_payload(s) {
                        result.warnings.push(SpannedError::at_expr(
                            axis,
                            format!(
                                "security: {} data.axes string literal {} — runtime will escape, but review intent",
                                fn_name, reason
                            ),
                        ));
                    }
                }
            }
        }
        // series: List<Struct{name, values}> — scan each series.name
        if let Some(Expr::List {
            items: series_items,
            ..
        }) = fields.get("series")
        {
            for item in series_items {
                if let Expr::StructLit {
                    fields: series_fields,
                    ..
                } = item
                {
                    if let Some(name_expr) = series_fields.get("name") {
                        if let Expr::StringLit { value: s, .. } = name_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    name_expr,
                                    format!(
                                        "security: {} data.series[].name string literal {} — runtime will escape, but review intent",
                                        fn_name, reason
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Наряд №81 Block 6 — recursive scanner for `diagram_tree` /
/// `diagram_org_chart` data shapes.
///
/// Both functions accept `Struct { label, title?, children }` where
/// `children` is `List<Struct>` of the SAME shape — i.e. the nesting
/// depth is unbounded. The existing `scan_chart_labels` only scans
/// ONE level (it iterates a List<Struct> literal and looks up `label`
/// on each element). It does NOT recurse into a `children` field —
/// which means an injection at depth ≥ 2 would slip past it.
///
/// This function walks the recursive struct: at each level we look up
/// `label` (and `title` if present) as StringLit fields and check for
/// XSS payloads. We then descend into the `children` List and recurse
/// on each child Struct.
///
/// `path` is used in the warning message so the caller can locate the
/// offending node (e.g. "root.children[1].children[2].label") — this
/// is essential because the recursive structure can be deeply nested
/// and a generic "label contains <script>" warning would be useless.
///
/// `allow_title` controls whether `title` is scanned (diagram_org_chart
/// allows it; diagram_tree ignores the field even if present, but we
/// still scan it defensively — better to over-warn than miss a payload
/// at the call site that uses diagram_tree but actually provides title).
fn scan_tree_labels_recursive(
    fn_name: &str,
    arg: &Expr,
    path: &str,
    allow_title: bool,
    result: &mut AnalysisResult,
) {
    if let Expr::StructLit { fields, .. } = arg {
        // Scan `label` (always present per spec — required field)
        if let Some(label_expr) = fields.get("label") {
            if let Expr::StringLit { value: s, .. } = label_expr {
                if let Some(reason) = detect_xss_payload(s) {
                    result.warnings.push(SpannedError::at_expr(
                        label_expr,
                        format!(
                            "security: {} data.{}.label string literal {} — runtime will escape, but review intent",
                            fn_name, path, reason
                        ),
                    ));
                }
            }
        }
        // Scan `title` (org chart only — but scan defensively regardless
        // of allow_title, since a malicious caller could supply title to
        // diagram_tree too; the runtime still escapes it via svg_text if
        // it ends up in the output. Better to over-warn.)
        if allow_title {
            if let Some(title_expr) = fields.get("title") {
                if let Expr::StringLit { value: s, .. } = title_expr {
                    if let Some(reason) = detect_xss_payload(s) {
                        result.warnings.push(SpannedError::at_expr(
                            title_expr,
                            format!(
                                "security: {} data.{}.title string literal {} — runtime will escape, but review intent",
                                fn_name, path, reason
                            ),
                        ));
                    }
                }
            }
        } else {
            // Even for diagram_tree (allow_title=false), if a title is
            // present we should warn — it's not used, but its presence
            // in literal form is suspicious intent.
            if let Some(title_expr) = fields.get("title") {
                if let Expr::StringLit { value: s, .. } = title_expr {
                    if let Some(reason) = detect_xss_payload(s) {
                        result.warnings.push(SpannedError::at_expr(
                            title_expr,
                            format!(
                                "security: {} data.{}.title string literal {} — field not used by diagram_tree but review intent",
                                fn_name, path, reason
                            ),
                        ));
                    }
                }
            }
        }
        // Recurse into children (if present)
        if let Some(Expr::List {
            items: child_items, ..
        }) = fields.get("children")
        {
            for (i, child) in child_items.iter().enumerate() {
                let child_path = format!("{}.children[{}]", path, i);
                scan_tree_labels_recursive(fn_name, child, &child_path, allow_title, result);
            }
        }
    }
}

/// Наряд №81 Block 6 — scanner for `diagram_flowchart` data shape.
///
/// Flowchart data has TWO independent lists, each containing user text:
///   Struct {
///     nodes: List<Struct{id, label}>,
///     edges: List<Struct{from, to, label?}>,
///   }
///
/// We scan `nodes[].label` and `edges[].label` separately (the `id`,
/// `from`, `to` fields are identifiers used for graph topology, not
/// rendered text — but we scan them defensively too, since a literal
/// `<script>` in an id field is still suspicious intent even if it
/// wouldn't reach the output).
///
/// Both lists are scanned to WARNINGs (runtime escapes label text via
/// escape_html_chars — defense-in-depth, same as chart_*).
fn scan_flowchart_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::StructLit { fields, .. }) = args.first() {
        // nodes: List<Struct{id, label}>
        if let Some(Expr::List {
            items: node_items, ..
        }) = fields.get("nodes")
        {
            for (i, item) in node_items.iter().enumerate() {
                if let Expr::StructLit {
                    fields: node_fields,
                    ..
                } = item
                {
                    // Scan id (defensive — not rendered, but suspicious if payload)
                    if let Some(id_expr) = node_fields.get("id") {
                        if let Expr::StringLit { value: s, .. } = id_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    id_expr,
                                    format!(
                                        "security: {} data.nodes[{}].id string literal {} — field not rendered but review intent",
                                        fn_name, i, reason
                                    ),
                                ));
                            }
                        }
                    }
                    // Scan label (rendered — primary injection vector)
                    if let Some(label_expr) = node_fields.get("label") {
                        if let Expr::StringLit { value: s, .. } = label_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    label_expr,
                                    format!(
                                        "security: {} data.nodes[{}].label string literal {} — runtime will escape, but review intent",
                                        fn_name, i, reason
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }
        // edges: List<Struct{from, to, label?}>
        if let Some(Expr::List {
            items: edge_items, ..
        }) = fields.get("edges")
        {
            for (i, item) in edge_items.iter().enumerate() {
                if let Expr::StructLit {
                    fields: edge_fields,
                    ..
                } = item
                {
                    // from/to are identifiers (defensive scan)
                    for ident_key in &["from", "to"] {
                        if let Some(ident_expr) = edge_fields.get(*ident_key) {
                            if let Expr::StringLit { value: s, .. } = ident_expr {
                                if let Some(reason) = detect_xss_payload(s) {
                                    result.warnings.push(SpannedError::at_expr(
                                        ident_expr,
                                        format!(
                                            "security: {} data.edges[{}].{} string literal {} — field not rendered but review intent",
                                            fn_name, i, ident_key, reason
                                        ),
                                    ));
                                }
                            }
                        }
                    }
                    // label is rendered as edge midpoint text
                    if let Some(label_expr) = edge_fields.get("label") {
                        if let Expr::StringLit { value: s, .. } = label_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    label_expr,
                                    format!(
                                        "security: {} data.edges[{}].label string literal {} — runtime will escape, but review intent",
                                        fn_name, i, reason
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Наряд №81 Block 6 — scanner for `diagram_layers` data shape.
///
/// Flat list of structs:
///   List<Struct{label, description?}>
///
/// Both `label` and `description` are rendered as text — both must be
/// scanned. Uses the same per-struct pattern as scan_chart_labels but
/// checks TWO fields instead of one.
fn scan_layers_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::List { items, .. }) = args.first() {
        for (i, item) in items.iter().enumerate() {
            if let Expr::StructLit { fields, .. } = item {
                // label (always rendered)
                if let Some(label_expr) = fields.get("label") {
                    if let Expr::StringLit { value: s, .. } = label_expr {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(SpannedError::at_expr(
                                label_expr,
                                format!(
                                    "security: {} data[{}].label string literal {} — runtime will escape, but review intent",
                                    fn_name, i, reason
                                ),
                            ));
                        }
                    }
                }
                // description (rendered, optional)
                if let Some(description_expr) = fields.get("description") {
                    if let Expr::StringLit { value: s, .. } = description_expr {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(SpannedError::at_expr(
                                description_expr,
                                format!(
                                    "security: {} data[{}].description string literal {} — runtime will escape, but review intent",
                                    fn_name, i, reason
                                ),
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Наряд №82 Block 6 — scanner for `diagram_sequence` data shape.
///
/// Sequence has TWO independent lists, each containing user text:
///   Struct {
///     actors:   List<String>,                         // scan each StringLit directly
///     messages: List<Struct{from, to, label?}>,       // scan messages[].label
///   }
///
/// `actors` is a `List<String>` (NOT `List<Struct>`) — this is the
/// special case called out in the narazd spec: "список строк — не забыть,
/// это другая форма, чем везде остальные". The scanner walks each
/// StringLit in actors[] directly (no struct unwrap), unlike
/// scan_chart_labels/scan_layers_labels which expect StructLit elements.
///
/// `messages[].label` is the only rendered text field in messages;
/// `from`/`to` are identifier strings used for actor lookup, not
/// rendered — but we scan them defensively (same pattern as
/// scan_flowchart_labels for edges[].from/to).
///
/// All findings are WARNINGs (runtime escapes via escape_html_chars —
/// defense-in-depth, not a hard error).
fn scan_sequence_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::StructLit { fields, .. }) = args.first() {
        // actors: List<String> — scan each StringLit directly (no struct
        // unwrap, this is the spec's "list of strings, not list of structs"
        // special case).
        if let Some(Expr::List {
            items: actor_items, ..
        }) = fields.get("actors")
        {
            for (i, actor) in actor_items.iter().enumerate() {
                if let Expr::StringLit { value: s, .. } = actor {
                    if let Some(reason) = detect_xss_payload(s) {
                        result.warnings.push(SpannedError::at_expr(
                            actor,
                            format!(
                                "security: {} data.actors[{}] string literal {} — runtime will escape, but review intent",
                                fn_name, i, reason
                            ),
                        ));
                    }
                }
            }
        }
        // messages: List<Struct{from, to, label?}> — scan from/to
        // defensively (identifiers, not rendered) and label as primary
        // (rendered edge label at midpoint).
        if let Some(Expr::List {
            items: msg_items, ..
        }) = fields.get("messages")
        {
            for (i, item) in msg_items.iter().enumerate() {
                if let Expr::StructLit {
                    fields: msg_fields, ..
                } = item
                {
                    // from/to (defensive — identifiers, not rendered)
                    for ident_key in &["from", "to"] {
                        if let Some(ident_expr) = msg_fields.get(*ident_key) {
                            if let Expr::StringLit { value: s, .. } = ident_expr {
                                if let Some(reason) = detect_xss_payload(s) {
                                    result.warnings.push(SpannedError::at_expr(
                                        ident_expr,
                                        format!(
                                            "security: {} data.messages[{}].{} string literal {} — field not rendered but review intent",
                                            fn_name, i, ident_key, reason
                                        ),
                                    ));
                                }
                            }
                        }
                    }
                    // label (rendered as edge midpoint text)
                    if let Some(label_expr) = msg_fields.get("label") {
                        if let Expr::StringLit { value: s, .. } = label_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    label_expr,
                                    format!(
                                        "security: {} data.messages[{}].label string literal {} — runtime will escape, but review intent",
                                        fn_name, i, reason
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Наряд №82 Block 6 — scanner for `diagram_gantt` data shape.
///
/// Gantt is a flat list of structs with exactly ONE string field:
///   List<Struct{task: String, start: Float, duration: Float}>
///
/// Only `task` is rendered (as the bar label). `start`/`duration` are
/// floats used for geometry — never reach SVG output. We scan `task`
/// as a WARNING (runtime escapes via escape_html_chars — same as
/// scan_chart_labels scans `label`, just under a different field name).
///
/// Implementation note: this is essentially scan_chart_labels with the
/// field name changed from "label" to "task". We could parametrize
/// scan_chart_labels to take a field name, but the existing call sites
/// all use "label" — keeping a separate function preserves the
/// self-documenting nature of each scanner and avoids changing
/// behavior for the five existing chart_* builtins.
fn scan_gantt_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::List { items, .. }) = args.first() {
        for (i, item) in items.iter().enumerate() {
            if let Expr::StructLit { fields, .. } = item {
                // task is the only string field; start/duration are floats
                if let Some(task_expr) = fields.get("task") {
                    if let Expr::StringLit { value: s, .. } = task_expr {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(SpannedError::at_expr(
                                task_expr,
                                format!(
                                    "security: {} data[{}].task string literal {} — runtime will escape, but review intent",
                                    fn_name, i, reason
                                ),
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Наряд №82 Block 6 — scanner for `diagram_timeline` data shape.
///
/// Timeline is a flat list of structs with THREE string fields:
///   List<Struct{date: String, label: String, description?: String}>
///
/// All three are rendered as text (`date` and `label` always, `description`
/// when present). All three must be scanned. Same per-struct walk pattern
/// as scan_layers_labels — extended to check `date` as the first field.
fn scan_timeline_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::List { items, .. }) = args.first() {
        for (i, item) in items.iter().enumerate() {
            if let Expr::StructLit { fields, .. } = item {
                // date (always rendered, above/below the dot)
                if let Some(date_expr) = fields.get("date") {
                    if let Expr::StringLit { value: s, .. } = date_expr {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(SpannedError::at_expr(
                                date_expr,
                                format!(
                                    "security: {} data[{}].date string literal {} — runtime will escape, but review intent",
                                    fn_name, i, reason
                                ),
                            ));
                        }
                    }
                }
                // label (always rendered)
                if let Some(label_expr) = fields.get("label") {
                    if let Expr::StringLit { value: s, .. } = label_expr {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(SpannedError::at_expr(
                                label_expr,
                                format!(
                                    "security: {} data[{}].label string literal {} — runtime will escape, but review intent",
                                    fn_name, i, reason
                                ),
                            ));
                        }
                    }
                }
                // description (rendered, optional)
                if let Some(description_expr) = fields.get("description") {
                    if let Expr::StringLit { value: s, .. } = description_expr {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(SpannedError::at_expr(
                                description_expr,
                                format!(
                                    "security: {} data[{}].description string literal {} — runtime will escape, but review intent",
                                    fn_name, i, reason
                                ),
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Наряд №83 Block 6 — scanner for `diagram_venn` data shape.
///
/// Venn has a nested structure with a TOP-LEVEL string field:
///   Struct {
///     circles:      List<Struct{label: String, value?: Float}>,
///     overlap_label: String?,                          ← TOP-LEVEL, not in list
///   }
///
/// `overlap_label` is the spec-called-out "easy to forget" case: it's a
/// field on the outer Struct, not an element of `circles[]`. A scanner
/// that only walks List elements would miss it entirely. We scan it
/// separately as a top-level StringLit field.
///
/// `circles[].label` is rendered inside each circle (offset from center).
/// `circles[].value` is a Float — never rendered as text, skipped.
///
/// All findings are WARNINGs (runtime escapes via escape_html_chars —
/// defense-in-depth, not a hard error).
fn scan_venn_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::StructLit { fields, .. }) = args.first() {
        // circles: List<Struct{label, value?}> — scan each circle's label
        if let Some(Expr::List {
            items: circle_items,
            ..
        }) = fields.get("circles")
        {
            for (i, item) in circle_items.iter().enumerate() {
                if let Expr::StructLit {
                    fields: circle_fields,
                    ..
                } = item
                {
                    if let Some(label_expr) = circle_fields.get("label") {
                        if let Expr::StringLit { value: s, .. } = label_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    label_expr,
                                    format!(
                                        "security: {} data.circles[{}].label string literal {} — runtime will escape, but review intent",
                                        fn_name, i, reason
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }
        // overlap_label — TOP-LEVEL field (the spec's "don't forget" case).
        // This is NOT inside the circles list; a scanner that only walks
        // list elements would miss it.
        if let Some(overlap_expr) = fields.get("overlap_label") {
            if let Expr::StringLit { value: s, .. } = overlap_expr {
                if let Some(reason) = detect_xss_payload(s) {
                    result.warnings.push(SpannedError::at_expr(
                        overlap_expr,
                        format!(
                            "security: {} data.overlap_label string literal {} — runtime will escape, but review intent",
                            fn_name, reason
                        ),
                    ));
                }
            }
        }
    }
}

/// Наряд №83 Block 6 — scanner for `diagram_quadrant` data shape.
///
/// Quadrant has TWO top-level string fields PLUS a nested list:
///   Struct {
///     x_axis_label: String,                            ← TOP-LEVEL
///     y_axis_label: String,                            ← TOP-LEVEL
///     items: List<Struct{label: String, x: Float, y: Float}>,
///   }
///
/// Both `x_axis_label` and `y_axis_label` are TOP-LEVEL fields (the spec
/// calls them out as "поля верхнего уровня, не элементы списка, легко
/// забыть"). A scanner that only walks List elements would miss BOTH.
///
/// `items[].label` is rendered next to each point marker. `items[].x`
/// and `items[].y` are Floats — geometry, never rendered as text, skipped.
///
/// All findings are WARNINGs (runtime escapes via escape_html_chars —
/// defense-in-depth, not a hard error).
fn scan_quadrant_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::StructLit { fields, .. }) = args.first() {
        // x_axis_label — top-level, easy to forget
        if let Some(x_axis_expr) = fields.get("x_axis_label") {
            if let Expr::StringLit { value: s, .. } = x_axis_expr {
                if let Some(reason) = detect_xss_payload(s) {
                    result.warnings.push(SpannedError::at_expr(
                        x_axis_expr,
                        format!(
                            "security: {} data.x_axis_label string literal {} — runtime will escape, but review intent",
                            fn_name, reason
                        ),
                    ));
                }
            }
        }
        // y_axis_label — top-level, easy to forget
        if let Some(y_axis_expr) = fields.get("y_axis_label") {
            if let Expr::StringLit { value: s, .. } = y_axis_expr {
                if let Some(reason) = detect_xss_payload(s) {
                    result.warnings.push(SpannedError::at_expr(
                        y_axis_expr,
                        format!(
                            "security: {} data.y_axis_label string literal {} — runtime will escape, but review intent",
                            fn_name, reason
                        ),
                    ));
                }
            }
        }
        // items: List<Struct{label, x, y}> — scan each item's label
        if let Some(Expr::List {
            items: item_list, ..
        }) = fields.get("items")
        {
            for (i, item) in item_list.iter().enumerate() {
                if let Expr::StructLit {
                    fields: item_fields,
                    ..
                } = item
                {
                    if let Some(label_expr) = item_fields.get("label") {
                        if let Expr::StringLit { value: s, .. } = label_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    label_expr,
                                    format!(
                                        "security: {} data.items[{}].label string literal {} — runtime will escape, but review intent",
                                        fn_name, i, reason
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Наряд №83 Block 6 — scanner for `diagram_medallion` data shape.
///
/// Medallion is a flat list of structs:
///   List<Struct{icon: String?, label: String, value?: Float}>
///
/// `label` is rendered below each medallion — primary scan target.
/// `icon` is a CONTROLLED ENUM (validated at runtime against the 10
/// known svg_icon names) — NOT user free-form text, explicitly NOT
/// scanned per the spec ("icon не сканировать как текст — это controlled
/// enum-подобное значение"). If a user passes `<script>` as an icon name,
/// the runtime rejects it with an "unknown icon name" error before any
/// SVG output is produced.
/// `value` is a Float — never rendered as text, skipped.
///
/// This scanner is essentially scan_chart_labels (single `label` field
/// per struct). We keep it as a separate function for self-documenting
/// naming and to leave room for medallion-specific extensions later.
fn scan_medallion_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::List { items, .. }) = args.first() {
        for (i, item) in items.iter().enumerate() {
            if let Expr::StructLit { fields, .. } = item {
                // label — rendered below the medallion (primary target)
                if let Some(label_expr) = fields.get("label") {
                    if let Expr::StringLit { value: s, .. } = label_expr {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.warnings.push(SpannedError::at_expr(
                                label_expr,
                                format!(
                                    "security: {} data[{}].label string literal {} — runtime will escape, but review intent",
                                    fn_name, i, reason
                                ),
                            ));
                        }
                    }
                }
                // NOTE: `icon` is intentionally NOT scanned here. It is a
                // controlled enum (one of 10 known svg_icon names), validated
                // at runtime via icon_path_data() before any SVG output.
                // Scanning it as text would produce false positives for
                // legitimate icon names that happen to contain angle brackets
                // (none currently do, but the principle holds).
            }
        }
    }
}

/// Наряд №84 Block 7 — scanner for `diagram_er` data shape.
///
/// ER has two lists, each with rendered text:
///   Struct {
///     entities:  List<Struct{name: String, fields: List<String>}>,
///     relations: List<Struct{from: String, to: String, label?: String}>,
///   }
///
/// The novel case here is `entities[].fields` — a `List<String>` NESTED
/// INSIDE a struct field. This is the THIRD nesting form in the SVG
/// suite (after top-level `List<String>` in diagram_sequence.actors,
/// and `List<Struct>` everywhere else). A scanner that only walks one
/// level of struct fields would miss `fields[]` entirely — each field
/// name is rendered as a separate line inside the entity box, so an
/// injection in `fields[2]` would reach the SVG output.
///
/// We walk per-entity: scan `name` (rendered in the header bar), then
/// iterate `fields[]` and scan each `StringLit` element directly (no
/// struct unwrap — same approach as scan_sequence_labels.actors).
///
/// `relations[].label` is rendered at the connector midpoint (same as
/// flowchart edges). `relations[].from` and `.to` are entity-name
/// identifiers, scanned defensively (not rendered, but suspicious if
/// they contain a payload).
///
/// All findings are WARNINGs (runtime escapes via escape_html_chars —
/// defense-in-depth, not a hard error).
fn scan_er_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::StructLit { fields, .. }) = args.first() {
        // entities: List<Struct{name, fields: List<String>}>
        if let Some(Expr::List {
            items: entity_items,
            ..
        }) = fields.get("entities")
        {
            for (i, item) in entity_items.iter().enumerate() {
                if let Expr::StructLit {
                    fields: entity_fields,
                    ..
                } = item
                {
                    // name — rendered in the entity header bar (primary target)
                    if let Some(name_expr) = entity_fields.get("name") {
                        if let Expr::StringLit { value: s, .. } = name_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    name_expr,
                                    format!(
                                        "security: {} data.entities[{}].name string literal {} — runtime will escape, but review intent",
                                        fn_name, i, reason
                                    ),
                                ));
                            }
                        }
                    }
                    // fields: List<String> NESTED inside a struct field —
                    // the third nesting form. Each StringLit is rendered
                    // as a separate line inside the entity box.
                    if let Some(Expr::List {
                        items: field_items, ..
                    }) = entity_fields.get("fields")
                    {
                        for (j, f_item) in field_items.iter().enumerate() {
                            if let Expr::StringLit { value: s, .. } = f_item {
                                if let Some(reason) = detect_xss_payload(s) {
                                    result.warnings.push(SpannedError::at_expr(
                                        f_item,
                                        format!(
                                            "security: {} data.entities[{}].fields[{}] string literal {} — runtime will escape, but review intent",
                                            fn_name, i, j, reason
                                        ),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        // relations: List<Struct{from, to, label?}> — same shape as
        // diagram_flowchart.edges. Scan from/to defensively, label as primary.
        if let Some(Expr::List {
            items: rel_items, ..
        }) = fields.get("relations")
        {
            for (i, item) in rel_items.iter().enumerate() {
                if let Expr::StructLit {
                    fields: rel_fields, ..
                } = item
                {
                    for ident_key in &["from", "to"] {
                        if let Some(ident_expr) = rel_fields.get(*ident_key) {
                            if let Expr::StringLit { value: s, .. } = ident_expr {
                                if let Some(reason) = detect_xss_payload(s) {
                                    result.warnings.push(SpannedError::at_expr(
                                        ident_expr,
                                        format!(
                                            "security: {} data.relations[{}].{} string literal {} — field not rendered but review intent",
                                            fn_name, i, ident_key, reason
                                        ),
                                    ));
                                }
                            }
                        }
                    }
                    if let Some(label_expr) = rel_fields.get("label") {
                        if let Expr::StringLit { value: s, .. } = label_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    label_expr,
                                    format!(
                                        "security: {} data.relations[{}].label string literal {} — runtime will escape, but review intent",
                                        fn_name, i, reason
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Наряд №84 Block 7 — scanner for `diagram_state` data shape.
///
/// State has TWO independent text sources plus a top-level optional:
///   Struct {
///     states:      List<String>,                              // scan each StringLit
///     transitions: List<Struct{from, to, label?}>,            // scan transitions[].label
///     initial:     String?,                                   // TOP-LEVEL field, like venn.overlap_label
///   }
///
/// `states[]` is `List<String>` (same form as diagram_sequence.actors —
/// scan each StringLit directly, no struct unwrap).
/// `transitions[].label` is rendered at the connector midpoint (self-
/// loops render the label above the loop arc — same scan rule applies).
/// `transitions[].from`/`.to` are state-name identifiers, scanned
/// defensively (not rendered).
/// `initial` is a TOP-LEVEL `String?` field — easy to forget, called
/// out explicitly in the spec. We scan it separately at the outer
/// StructLit level (not inside any list).
///
/// All findings are WARNINGs (runtime escapes via escape_html_chars —
/// defense-in-depth, not a hard error).
fn scan_state_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::StructLit { fields, .. }) = args.first() {
        // states: List<String> — scan each StringLit directly
        if let Some(Expr::List {
            items: state_items, ..
        }) = fields.get("states")
        {
            for (i, state) in state_items.iter().enumerate() {
                if let Expr::StringLit { value: s, .. } = state {
                    if let Some(reason) = detect_xss_payload(s) {
                        result.warnings.push(SpannedError::at_expr(
                            state,
                            format!(
                                "security: {} data.states[{}] string literal {} — runtime will escape, but review intent",
                                fn_name, i, reason
                            ),
                        ));
                    }
                }
            }
        }
        // transitions: List<Struct{from, to, label?}> — same pattern as
        // diagram_flowchart.edges / diagram_sequence.messages.
        if let Some(Expr::List {
            items: trans_items, ..
        }) = fields.get("transitions")
        {
            for (i, item) in trans_items.iter().enumerate() {
                if let Expr::StructLit {
                    fields: trans_fields,
                    ..
                } = item
                {
                    for ident_key in &["from", "to"] {
                        if let Some(ident_expr) = trans_fields.get(*ident_key) {
                            if let Expr::StringLit { value: s, .. } = ident_expr {
                                if let Some(reason) = detect_xss_payload(s) {
                                    result.warnings.push(SpannedError::at_expr(
                                        ident_expr,
                                        format!(
                                            "security: {} data.transitions[{}].{} string literal {} — field not rendered but review intent",
                                            fn_name, i, ident_key, reason
                                        ),
                                    ));
                                }
                            }
                        }
                    }
                    if let Some(label_expr) = trans_fields.get("label") {
                        if let Expr::StringLit { value: s, .. } = label_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    label_expr,
                                    format!(
                                        "security: {} data.transitions[{}].label string literal {} — runtime will escape, but review intent",
                                        fn_name, i, reason
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }
        // initial — TOP-LEVEL String? field (the spec's "easy to forget" case,
        // same category as diagram_venn.overlap_label). It's rendered as the
        // entry-arrow target state name, but ALSO drawn near the initial node.
        if let Some(initial_expr) = fields.get("initial") {
            if let Expr::StringLit { value: s, .. } = initial_expr {
                if let Some(reason) = detect_xss_payload(s) {
                    result.warnings.push(SpannedError::at_expr(
                        initial_expr,
                        format!(
                            "security: {} data.initial string literal {} — runtime will escape, but review intent",
                            fn_name, reason
                        ),
                    ));
                }
            }
        }
    }
}

/// Наряд №84 Block 7 — scanner for `diagram_swimlane` data shape.
///
/// Swimlane has TWO lists, each containing user text:
///   Struct {
///     lanes: List<String>,                              // scan each StringLit
///     steps: List<Struct{lane, label, order}>,          // scan steps[].label
///   }
///
/// `lanes[]` is `List<String>` (same form as diagram_sequence.actors
/// and diagram_state.states — scan each StringLit directly).
/// `steps[].label` is rendered inside each step pill (primary target).
/// `steps[].lane` is a lane-name identifier (defensive scan — not
/// rendered, but suspicious if it carries a payload).
/// `steps[].order` is a Float — geometry, never rendered as text, skipped.
///
/// All findings are WARNINGs (runtime escapes via escape_html_chars —
/// defense-in-depth, not a hard error).
fn scan_swimlane_labels(fn_name: &str, args: &[Expr], result: &mut AnalysisResult) {
    if let Some(Expr::StructLit { fields, .. }) = args.first() {
        // lanes: List<String> — scan each StringLit directly
        if let Some(Expr::List {
            items: lane_items, ..
        }) = fields.get("lanes")
        {
            for (i, lane) in lane_items.iter().enumerate() {
                if let Expr::StringLit { value: s, .. } = lane {
                    if let Some(reason) = detect_xss_payload(s) {
                        result.warnings.push(SpannedError::at_expr(
                            lane,
                            format!(
                                "security: {} data.lanes[{}] string literal {} — runtime will escape, but review intent",
                                fn_name, i, reason
                            ),
                        ));
                    }
                }
            }
        }
        // steps: List<Struct{lane, label, order}> — scan lane defensively,
        // label as primary. order is Float, skipped.
        if let Some(Expr::List {
            items: step_items, ..
        }) = fields.get("steps")
        {
            for (i, item) in step_items.iter().enumerate() {
                if let Expr::StructLit {
                    fields: step_fields,
                    ..
                } = item
                {
                    // lane — defensive (identifier, not rendered as free text)
                    if let Some(lane_expr) = step_fields.get("lane") {
                        if let Expr::StringLit { value: s, .. } = lane_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    lane_expr,
                                    format!(
                                        "security: {} data.steps[{}].lane string literal {} — field not rendered but review intent",
                                        fn_name, i, reason
                                    ),
                                ));
                            }
                        }
                    }
                    // label — rendered inside the step pill (primary target)
                    if let Some(label_expr) = step_fields.get("label") {
                        if let Expr::StringLit { value: s, .. } = label_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    label_expr,
                                    format!(
                                        "security: {} data.steps[{}].label string literal {} — runtime will escape, but review intent",
                                        fn_name, i, reason
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Walk an expression and run the security check on every FnCall node.
fn walk_expr_for_svg_security(expr: &Expr, result: &mut AnalysisResult, ctx: &str) {
    match expr {
        Expr::FnCall { name, args, .. } => {
            // Check string-literal arguments to SVG builtins
            if SVG_AUTO_ESCAPE_BUILTINS.contains(&name.as_str()) {
                if let Some(content_idx) = auto_escaped_arg_index(name) {
                    if let Some(content_expr) = args.get(content_idx) {
                        if let Expr::StringLit { value: s, .. } = content_expr {
                            if let Some(reason) = detect_xss_payload(s) {
                                result.warnings.push(SpannedError::at_expr(
                                    content_expr,
                                    format!(
                                        "security: {}({} arg) string literal {} — runtime will escape, but review intent",
                                        name, content_idx + 1, reason
                                    ),
                                ));
                            }
                        }
                    }
                }
                // Special case for chart_* builtins: their `label` field
                // lives inside `data: List<Struct{...}>` at arg 0. The list
                // element shape varies:
                //   chart_bar / chart_donut / chart_line / chart_area:
                //     {label, value}  — label first
                //   chart_scatter:
                //     {x, y, label?}  — label third, optional
                //   chart_boxplot:
                //     {label, values} — label first, but values is a List
                // scan_chart_labels looks up `label` BY NAME, so it works
                // uniformly across all these list-of-struct shapes.
                //
                // chart_radar has a DIFFERENT top-level shape
                // (Struct{axes, series}, not List<Struct>), so it gets
                // its own scanner (scan_radar_labels) that walks both
                // the `axes` List<String> and the `series[].name` field.
                //
                // chart_heatmap is intentionally NOT scanned — its data
                // is `List<List<Float>>` (pure numeric, no user text).
                // Runtime escapes label text via escape_html_chars, so
                // these are WARNINGs (suspicious intent) not ERRORs.
                if name == "chart_bar"
                    || name == "chart_donut"
                    || name == "chart_line"
                    || name == "chart_area"
                    || name == "chart_scatter"
                    || name == "chart_boxplot"
                {
                    scan_chart_labels(name, args, result);
                }
                if name == "chart_radar" {
                    scan_radar_labels(name, args, result);
                }
                // Наряд №81 Block 6: diagram scanners.
                //   diagram_tree / diagram_org_chart — recursive Struct,
                //     need scan_tree_labels_recursive (NOT the flat
                //     scan_chart_labels — that one only goes one level
                //     deep and would miss injections at children[2].label
                //     or deeper).
                //   diagram_flowchart — two independent lists (nodes,
                //     edges), each with its own label field. Scanned
                //     separately by scan_flowchart_labels.
                //   diagram_layers — flat list, label + description.
                //     scan_layers_labels checks both fields.
                if name == "diagram_tree" {
                    if let Some(arg0) = args.first() {
                        scan_tree_labels_recursive(name, arg0, "root", false, result);
                    }
                }
                if name == "diagram_org_chart" {
                    if let Some(arg0) = args.first() {
                        scan_tree_labels_recursive(name, arg0, "root", true, result);
                    }
                }
                if name == "diagram_flowchart" {
                    scan_flowchart_labels(name, args, result);
                }
                if name == "diagram_layers" {
                    scan_layers_labels(name, args, result);
                }
                // Наряд №82 Block 6: temporal & process diagram scanners.
                //   diagram_sequence — Struct{actors: List<String>, messages: ...}.
                //     actors is List<String> (special case — direct StringLit
                //     scan, no StructLit unwrap). messages[].label is rendered.
                //     Scanned by scan_sequence_labels.
                //   diagram_timeline — flat List<Struct{date, label, description?}>.
                //     3 string fields — scan_timeline_labels checks all three.
                //   diagram_gantt — flat List<Struct{task, start, duration}>.
                //     Only `task` is rendered — scan_gantt_labels checks it.
                //   diagram_process / diagram_loop — same shape as diagram_layers
                //     (List<Struct{label, description?}>) — REUSES
                //     scan_layers_labels (no need to write a new scanner for
                //     an identical shape — "не писать заново" per spec).
                if name == "diagram_sequence" {
                    scan_sequence_labels(name, args, result);
                }
                if name == "diagram_timeline" {
                    scan_timeline_labels(name, args, result);
                }
                if name == "diagram_gantt" {
                    scan_gantt_labels(name, args, result);
                }
                if name == "diagram_process" || name == "diagram_loop" {
                    scan_layers_labels(name, args, result);
                }
                // Наряд №83 Block 6: sets & comparison diagram scanners.
                //   diagram_venn — Struct{circles: [{label, value?}], overlap_label?}.
                //     Special scanner: overlap_label is a TOP-LEVEL field
                //     (not in circles[]), easy to forget — scan_venn_labels
                //     checks both the nested circles[].label and the top-level
                //     overlap_label in one pass.
                //   diagram_quadrant — Struct{x_axis_label, y_axis_label, items: [...]}.
                //     BOTH axis labels are top-level fields. scan_quadrant_labels
                //     checks both axis labels + items[].label.
                //   diagram_pyramid / diagram_nested — same flat shape as
                //     diagram_layers (List<Struct{label, value?}>) — REUSES
                //     scan_layers_labels (no new scanner for identical shape).
                //   diagram_medallion — List<Struct{icon?, label, value?}>.
                //     Only `label` is scanned; `icon` is a controlled enum
                //     (validated at runtime against 10 known svg_icon names),
                //     NOT free-form text — explicitly NOT scanned per spec.
                if name == "diagram_venn" {
                    scan_venn_labels(name, args, result);
                }
                if name == "diagram_quadrant" {
                    scan_quadrant_labels(name, args, result);
                }
                if name == "diagram_pyramid" || name == "diagram_nested" {
                    scan_layers_labels(name, args, result);
                }
                if name == "diagram_medallion" {
                    scan_medallion_labels(name, args, result);
                }
                // Наряд №84 Block 7: data & state diagram scanners.
                //   diagram_er — Struct{entities:[{name, fields:[String]}],
                //     relations:[{from,to,label?}]}. Special scanner
                //     scan_er_labels walks the nested List<String> inside
                //     each entity (the third nesting form — see comment
                //     on the scanner for why this needs bespoke handling).
                //   diagram_state — Struct{states:[String], transitions:[...],
                //     initial?}. scan_state_labels scans states[] as a direct
                //     List<String>, transitions[].label as rendered text, and
                //     `initial` as a TOP-LEVEL String? field (overlap_label form).
                //   diagram_swimlane — Struct{lanes:[String], steps:[{lane,label,order}]}.
                //     scan_swimlane_labels scans lanes[] directly + steps[].label.
                //   diagram_data_flow / diagram_high_level / diagram_architecture —
                //     Struct{nodes:[{id,label,icon?}], edges:[{from,to,label?}]}.
                //     Same shape as diagram_flowchart (Н81) — REUSES
                //     scan_flowchart_labels (no new scanner for an identical
                //     shape, per spec). The `icon` field on diagram_architecture
                //     nodes is a controlled enum (validated against svg_icon's
                //     10 names at runtime), NOT free-form text — explicitly
                //     NOT scanned, same decision as diagram_medallion.
                if name == "diagram_er" {
                    scan_er_labels(name, args, result);
                }
                if name == "diagram_state" {
                    scan_state_labels(name, args, result);
                }
                if name == "diagram_swimlane" {
                    scan_swimlane_labels(name, args, result);
                }
                if name == "diagram_data_flow"
                    || name == "diagram_high_level"
                    || name == "diagram_architecture"
                {
                    scan_flowchart_labels(name, args, result);
                }
            }
            if SVG_NO_ESCAPE_BUILTINS.contains(&name.as_str()) {
                // Check ALL string-literal args — any of them could be an injection vector
                for (i, arg) in args.iter().enumerate() {
                    if let Expr::StringLit { value: s, .. } = arg {
                        if let Some(reason) = detect_xss_payload(s) {
                            result.errors.push(SpannedError::at_expr(
                                arg,
                                format!(
                                    "security: {}({} arg) string literal {} — this builtin does NOT auto-escape this argument; potential injection vector",
                                    name, i + 1, reason
                                ),
                            ));
                        }
                        // For svg_canvas, check viewbox arg specifically
                        if name == "svg_canvas" && i == 2 {
                            // viewbox should be 4 numbers — any other format is suspicious
                            let parts: Vec<&str> = s.split_whitespace().collect();
                            if parts.len() != 4 || parts.iter().any(|p| p.parse::<f64>().is_err()) {
                                result.warnings.push(SpannedError::at_expr(
                                    arg,
                                    format!(
                                        "security: svg_canvas viewbox argument should be 4 numbers, got {:?}",
                                        s
                                    ),
                                ));
                            }
                        }
                    } else {
                        // Recurse into non-literal expressions (concat, list, etc.)
                        // to catch <script> hidden inside e.g. "M 10 10 " + "<script>"
                        walk_expr_for_svg_security(arg, result, ctx);
                    }
                }
            }
            // Check for external URLs in non-whitelisted contexts
            // (e.g. svg_icon color, svg_text fill) — these are colors, not URLs,
            // but if someone passes "javascript:..." as a color, that's suspicious
            for (i, arg) in args.iter().enumerate() {
                if let Expr::StringLit { value: s, .. } = arg {
                    if s.starts_with("javascript:") || s.starts_with("data:text/html") {
                        result.errors.push(SpannedError::at_expr(
                            arg,
                            format!(
                                "security: {}({} arg) contains potentially dangerous URL scheme: {:?}",
                                name,
                                i + 1,
                                s
                            ),
                        ));
                    }
                }
            }
            // Recurse into arguments
            for arg in args {
                walk_expr_for_svg_security(arg, result, ctx);
            }
        }
        Expr::QualifiedCall { function, args, .. } => {
            for arg in args {
                walk_expr_for_svg_security(arg, result, ctx);
            }
            let _ = function;
        }
        Expr::BinaryOp {
            left: lhs,
            right: rhs,
            ..
        } => {
            // String concatenation: walk BOTH sides to scan all string literals.
            // If a StringLit appears inside a concat that's an arg to an SVG
            // no-escape builtin, the surrounding FnCall walker has already
            // recursed into us (via the `else` branch). Here we additionally
            // scan the immediate StringLit children of BinaryOp for XSS payloads
            // — this catches concat expressions where the FnCall walker didn't
            // flag them because the payload is buried inside a BinaryOp.
            if let Expr::StringLit { value: s, .. } = lhs.as_ref() {
                if let Some(reason) = detect_xss_payload(s) {
                    // This is a WARNING only — we don't know if this concat
                    // feeds an SVG context. The FnCall walker will escalate
                    // to ERROR if appropriate.
                    result.warnings.push(SpannedError::at_expr(
                        lhs.as_ref(),
                        format!(
                            "security: string literal in concatenation {} — review usage",
                            reason
                        ),
                    ));
                }
            }
            if let Expr::StringLit { value: s, .. } = rhs.as_ref() {
                if let Some(reason) = detect_xss_payload(s) {
                    result.warnings.push(SpannedError::at_expr(
                        rhs.as_ref(),
                        format!(
                            "security: string literal in concatenation {} — review usage",
                            reason
                        ),
                    ));
                }
            }
            walk_expr_for_svg_security(lhs, result, ctx);
            walk_expr_for_svg_security(rhs, result, ctx);
        }
        Expr::IfElse {
            condition: cond,
            then_branch: then_e,
            else_branch: else_e,
            ..
        } => {
            walk_expr_for_svg_security(cond, result, ctx);
            walk_expr_for_svg_security(then_e, result, ctx);
            walk_expr_for_svg_security(else_e, result, ctx);
        }
        Expr::List { items, .. } => {
            for item in items {
                walk_expr_for_svg_security(item, result, ctx);
            }
        }
        Expr::StructLit { fields, .. } => {
            for v in fields.values() {
                walk_expr_for_svg_security(v, result, ctx);
            }
        }
        Expr::FieldAccess { object: inner, .. } => {
            walk_expr_for_svg_security(inner, result, ctx);
        }
        Expr::IndexAccess {
            object: inner,
            index: idx,
            ..
        } => {
            walk_expr_for_svg_security(inner, result, ctx);
            walk_expr_for_svg_security(idx, result, ctx);
        }
        Expr::Try { expr: inner, .. } => {
            walk_expr_for_svg_security(inner, result, ctx);
        }
        Expr::BlockIfElse {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            walk_expr_for_svg_security(condition, result, ctx);
            for s in then_body {
                walk_stmt_for_svg_security(s, result, ctx);
            }
            for (cond, body) in else_ifs {
                walk_expr_for_svg_security(cond, result, ctx);
                for s in body {
                    walk_stmt_for_svg_security(s, result, ctx);
                }
            }
            if let Some(body) = else_body {
                for s in body {
                    walk_stmt_for_svg_security(s, result, ctx);
                }
            }
        }
        // №369: match-as-expression — walk scrutinee + arm bodies + else.
        Expr::MatchExpr {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            walk_expr_for_svg_security(scrutinee, result, ctx);
            for arm in arms {
                for s in arm.body() {
                    walk_stmt_for_svg_security(s, result, ctx);
                }
            }
            if let Some(eb) = else_body {
                for s in eb {
                    walk_stmt_for_svg_security(s, result, ctx);
                }
            }
        }
        // №332 (ADR-0164): perception constructions — no SVG surface;
        // ProvBind delegates to the inner construction.
        Expr::HandleSource { .. } => {}
        Expr::ProvBind { inner, .. } => walk_expr_for_svg_security(inner, result, ctx),
        Expr::StringLit { .. }
        | Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::Ident { .. } => {}
    }
    let _ = ctx;
}

/// Walk a statement and run security check on every expression it contains.
fn walk_stmt_for_svg_security(stmt: &Statement, result: &mut AnalysisResult, ctx: &str) {
    match stmt {
        Statement::LetBinding { value, .. } | Statement::Assign { value, .. } => {
            walk_expr_for_svg_security(value, result, ctx);
        }
        Statement::Return { value: e, .. } => walk_expr_for_svg_security(e, result, ctx),
        Statement::ExprStmt { expr: e, .. } => walk_expr_for_svg_security(e, result, ctx),
        Statement::Each { iterable, body, .. } => {
            walk_expr_for_svg_security(iterable, result, ctx);
            for s in body {
                walk_stmt_for_svg_security(s, result, ctx);
            }
        }
        Statement::EachWithIndex { iterable, body, .. } => {
            walk_expr_for_svg_security(iterable, result, ctx);
            for s in body {
                walk_stmt_for_svg_security(s, result, ctx);
            }
        }
        Statement::While {
            condition, body, ..
        } => {
            walk_expr_for_svg_security(condition, result, ctx);
            for s in body {
                walk_stmt_for_svg_security(s, result, ctx);
            }
        }
        Statement::IfElseBlock {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            walk_expr_for_svg_security(condition, result, ctx);
            for s in then_body {
                walk_stmt_for_svg_security(s, result, ctx);
            }
            for (cond, body) in else_ifs {
                walk_expr_for_svg_security(cond, result, ctx);
                for s in body {
                    walk_stmt_for_svg_security(s, result, ctx);
                }
            }
            if let Some(body) = else_body {
                for s in body {
                    walk_stmt_for_svg_security(s, result, ctx);
                }
            }
        }
        Statement::IfThen {
            condition: cond,
            body,
            ..
        } => {
            walk_expr_for_svg_security(cond, result, ctx);
            for s in body {
                walk_stmt_for_svg_security(s, result, ctx);
            }
        }
        Statement::Match {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            walk_expr_for_svg_security(scrutinee, result, ctx);
            for arm in arms {
                match arm {
                    MatchArm::Compare(_, e, body) => {
                        walk_expr_for_svg_security(e, result, ctx);
                        for s in body {
                            walk_stmt_for_svg_security(s, result, ctx);
                        }
                    }
                    MatchArm::Exact(_, body)
                    | MatchArm::StartsWith(_, body)
                    | MatchArm::Contains(_, body) => {
                        for s in body {
                            walk_stmt_for_svg_security(s, result, ctx);
                        }
                    }
                }
            }
            if let Some(body) = else_body {
                for s in body {
                    walk_stmt_for_svg_security(s, result, ctx);
                }
            }
        }
        Statement::Break | Statement::Continue => {}
        // Наряд №266: memory statements carry user-controlled expressions —
        // scan them like any other expression carrier.
        Statement::Memorize(m) => walk_expr_for_svg_security(&m.value, result, ctx),
        Statement::Forget(f) => walk_expr_for_svg_security(&f.query, result, ctx),
        Statement::Relate(r) => {
            walk_expr_for_svg_security(&r.from, result, ctx);
            walk_expr_for_svg_security(&r.to, result, ctx);
        }
    }
}

/// Walk all declarations and run the SVG/HTML security lint.
fn svg_security_lint(declarations: &[Declaration], result: &mut AnalysisResult) {
    for decl in declarations {
        match decl {
            Declaration::Pattern(p) => {
                let ctx = format!("pattern {}", p.name);
                for s in &p.body {
                    walk_stmt_for_svg_security(s, result, &ctx);
                }
            }
            Declaration::Flow(f) => {
                let ctx = format!("flow {}", f.name);
                walk_expr_for_svg_security(&f.source, result, &ctx);
                for (_, branches) in &f.branch_defs {
                    for branch in branches {
                        // Branch bodies can contain expressions in pipeline steps
                        let _ = branch;
                    }
                }
            }
            Declaration::EntitySimple(e) => {
                let ctx = format!("entity {}", e.name);
                walk_expr_for_svg_security(&e.value, result, &ctx);
            }
            Declaration::EntityRecord(e) => {
                let ctx = format!("entity {}", e.name);
                for f in &e.fields {
                    walk_expr_for_svg_security(&f.value, result, &ctx);
                }
            }
            Declaration::MlogServer(srv) => {
                let ctx = "mlogserver".to_string();
                for route in &srv.routes {
                    for s in &route.body {
                        walk_stmt_for_svg_security(s, result, &ctx);
                    }
                }
            }
            Declaration::Template(t) => {
                // Template body is a raw string with {{ var }} placeholders,
                // not parsed statements. We do a simple text scan for XSS payloads.
                let ctx = format!("template {}", t.name);
                if let Some(reason) = detect_xss_payload(&t.body) {
                    result.errors.push(SpannedError::at_decl(
                        decl,
                        format!(
                            "security: template '{}' body {} — templates are raw HTML, no auto-escaping",
                            t.name, reason
                        ),
                    ));
                }
                let _ = ctx;
            }
            _ => {}
        }
    }
    // Suppress unused warning for is_whitelisted_url (kept for future URL-context checks)
    let _ = is_whitelisted_url("https://fonts.googleapis.com/css");
}

/// Helper: extract field names from an EntityType declaration.
fn get_type_fields<'a>(declarations: &'a [Declaration], type_name: &str) -> Option<Vec<&'a str>> {
    for decl in declarations {
        if let Declaration::EntityType(e) = decl {
            if e.name == type_name {
                return Some(e.fields.iter().map(|f| f.name.as_str()).collect());
            }
        }
    }
    None
}

/// Walk an expression tree, checking FnCall arity and detecting undefined functions.
fn check_expr_calls(
    expr: &Expr,
    builtin_names: &HashSet<String>,
    pattern_param_counts: &HashSet<(String, usize)>,
    learnable_names: &HashSet<String>,
    errors: &mut Vec<SpannedError>,
) {
    if let Expr::FnCall { name, args, .. } = expr {
        let is_known = builtin_names.contains(name)
            || pattern_param_counts.iter().any(|(n, _)| n == name)
            || learnable_names.contains(name);

        const INTERCEPTED_FUNCTIONS: &[&str] = &["recall_top_k"];

        if !is_known && !INTERCEPTED_FUNCTIONS.contains(&name.as_str()) {
            // Наряд №165: span taken from the FnCall expression itself —
            // the call site is the natural place to point the squiggle.
            errors.push(SpannedError::at_expr(
                expr,
                format!(
                    "undefined: function '{}' is not a builtin, pattern, or learnable",
                    name
                ),
            ));
        }

        // Check builtin arity
        if builtin_names.contains(name) {
            if let Err(e) = crate::builtins::check_builtin_arity(name, args.len()) {
                errors.push(SpannedError::at_expr(expr, e));
            }
        }

        // Check pattern param count
        for (pname, pcount) in pattern_param_counts {
            if *pname == *name && !builtin_names.contains(name) {
                if args.len() != *pcount {
                    errors.push(SpannedError::at_expr(
                        expr,
                        format!(
                            "function '{}' expects {} argument(s), got {}",
                            name,
                            pcount,
                            args.len()
                        ),
                    ));
                }
                break;
            }
        }

        // Recurse into arguments
        for arg in args {
            check_expr_calls(
                arg,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
    } else if let Expr::BinaryOp {
        left,
        op: _op,
        right,
        ..
    } = expr
    {
        check_expr_calls(
            left,
            builtin_names,
            pattern_param_counts,
            learnable_names,
            errors,
        );
        check_expr_calls(
            right,
            builtin_names,
            pattern_param_counts,
            learnable_names,
            errors,
        );
    } else if let Expr::IfElse {
        condition: cond,
        then_branch: then_br,
        else_branch: else_br,
        ..
    } = expr
    {
        check_expr_calls(
            cond,
            builtin_names,
            pattern_param_counts,
            learnable_names,
            errors,
        );
        check_expr_calls(
            then_br,
            builtin_names,
            pattern_param_counts,
            learnable_names,
            errors,
        );
        check_expr_calls(
            else_br,
            builtin_names,
            pattern_param_counts,
            learnable_names,
            errors,
        );
    } else if let Expr::List { items, .. } = expr {
        for item in items {
            check_expr_calls(
                item,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
    } else if let Expr::IndexAccess {
        object: inner,
        index: idx,
        ..
    } = expr
    {
        check_expr_calls(
            inner,
            builtin_names,
            pattern_param_counts,
            learnable_names,
            errors,
        );
        check_expr_calls(
            idx,
            builtin_names,
            pattern_param_counts,
            learnable_names,
            errors,
        );
    } else if let Expr::StructLit { fields, .. } = expr {
        for v in fields.values() {
            check_expr_calls(
                v,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
    }
}

/// Extract expressions from a statement and walk them for arity/undefined checks.
fn check_stmt_exprs(
    stmt: &Statement,
    builtin_names: &HashSet<String>,
    pattern_param_counts: &HashSet<(String, usize)>,
    learnable_names: &HashSet<String>,
    errors: &mut Vec<SpannedError>,
) {
    match stmt {
        Statement::LetBinding { value, .. } => {
            check_expr_calls(
                value,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
        Statement::Assign { value, .. } => {
            check_expr_calls(
                value,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
        Statement::Return { value: expr, .. } => {
            check_expr_calls(
                expr,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
        Statement::ExprStmt { expr, .. } => {
            check_expr_calls(
                expr,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
        Statement::Each { iterable, body, .. } => {
            check_expr_calls(
                iterable,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
            for s in body {
                check_stmt_exprs(
                    s,
                    builtin_names,
                    pattern_param_counts,
                    learnable_names,
                    errors,
                );
            }
        }
        Statement::EachWithIndex { iterable, body, .. } => {
            check_expr_calls(
                iterable,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
            for s in body {
                check_stmt_exprs(
                    s,
                    builtin_names,
                    pattern_param_counts,
                    learnable_names,
                    errors,
                );
            }
        }
        Statement::While {
            condition, body, ..
        } => {
            check_expr_calls(
                condition,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
            for s in body {
                check_stmt_exprs(
                    s,
                    builtin_names,
                    pattern_param_counts,
                    learnable_names,
                    errors,
                );
            }
        }
        Statement::IfElseBlock {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            check_expr_calls(
                condition,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
            for s in then_body {
                check_stmt_exprs(
                    s,
                    builtin_names,
                    pattern_param_counts,
                    learnable_names,
                    errors,
                );
            }
            for (cond, body) in else_ifs {
                check_expr_calls(
                    cond,
                    builtin_names,
                    pattern_param_counts,
                    learnable_names,
                    errors,
                );
                for s in body {
                    check_stmt_exprs(
                        s,
                        builtin_names,
                        pattern_param_counts,
                        learnable_names,
                        errors,
                    );
                }
            }
            if let Some(else_body) = else_body {
                for s in else_body {
                    check_stmt_exprs(
                        s,
                        builtin_names,
                        pattern_param_counts,
                        learnable_names,
                        errors,
                    );
                }
            }
        }
        Statement::IfThen {
            condition: cond,
            body,
            ..
        } => {
            check_expr_calls(
                cond,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
            for s in body {
                check_stmt_exprs(
                    s,
                    builtin_names,
                    pattern_param_counts,
                    learnable_names,
                    errors,
                );
            }
        }
        Statement::Match {
            scrutinee,
            arms,
            else_body,
            ..
        } => {
            check_expr_calls(
                scrutinee,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
            for arm in arms {
                match arm {
                    MatchArm::Exact(_, body)
                    | MatchArm::StartsWith(_, body)
                    | MatchArm::Contains(_, body) => {
                        for s in body {
                            check_stmt_exprs(
                                s,
                                builtin_names,
                                pattern_param_counts,
                                learnable_names,
                                errors,
                            );
                        }
                    }
                    MatchArm::Compare(_, expr, body) => {
                        check_expr_calls(
                            expr,
                            builtin_names,
                            pattern_param_counts,
                            learnable_names,
                            errors,
                        );
                        for s in body {
                            check_stmt_exprs(
                                s,
                                builtin_names,
                                pattern_param_counts,
                                learnable_names,
                                errors,
                            );
                        }
                    }
                }
            }
            if let Some(else_body) = else_body {
                for s in else_body {
                    check_stmt_exprs(
                        s,
                        builtin_names,
                        pattern_param_counts,
                        learnable_names,
                        errors,
                    );
                }
            }
        }
        Statement::Break | Statement::Continue => {}
        // Наряд №266: memory statements' expressions go through the same
        // call/arity/undefined-function checks as every other statement.
        Statement::Memorize(m) => {
            check_expr_calls(
                &m.value,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
        Statement::Forget(f) => {
            check_expr_calls(
                &f.query,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
        Statement::Relate(r) => {
            check_expr_calls(
                &r.from,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
            check_expr_calls(
                &r.to,
                builtin_names,
                pattern_param_counts,
                learnable_names,
                errors,
            );
        }
    }
}

// ── Наряд №264: static immutability enforcement ─────────────────────
//
// Contract (№14, REFERENCE.md §3.2, examples/p30_assign_*): a variable is
// immutable unless bound with `let mut`; assignment to an immutable
// variable is an error "cannot assign to immutable variable: x (use
// 'let mut x' to make it mutable)". Until this pass the contract lived
// ONLY in the tree-walking interpreter's runtime (execution.rs
// eval_statements_cf) — `mlog check` passed violating programs and the
// VM compiled them silently.
//
// The walk mirrors TW's EXACT mutability model so `mlog check` never
// diverges from `mlog run`:
//   - a flat `HashSet<String>` of mutable names is threaded through ALL
//     nested blocks and never popped (execution.rs:946 passes the same
//     `mutable_vars` down into if/while/each/match bodies — a `let mut`
//     inside a block stays in effect for the rest of the function);
//   - pattern params are NOT mutable (they never enter mutable_vars);
//   - `each` / `each i, item` loop variables are NOT mutable (they are
//     env.insert-ed per iteration, never added to mutable_vars);
//   - the mutability check happens BEFORE the variable is resolved, so an
//     assignment to a never-declared name reports the same immutability
//     message (TW behavior, execution.rs:989-992) — not a separate
//     "undefined variable" diagnostic.

/// The error text is the TW лекало verbatim (execution.rs:991) —
/// examples/p30_assign_immutable.error is matched against the TW channel
/// and must keep matching whichever backend surfaces the contract.
pub(crate) fn immutability_error_text(name: &str) -> String {
    format!(
        "cannot assign to immutable variable: {} (use 'let mut {}' to make it mutable)",
        name, name
    )
}

/// Entry point: check one statement list (pattern body / route body) with
/// an initially-empty mutable set (params are immutable; route bodies have
/// no params at all).
fn check_pattern_mutability(body: &[Statement], errors: &mut Vec<SpannedError>) {
    let mut mutable: HashSet<String> = HashSet::new();
    check_stmts_mutability(body, &mut mutable, errors);
}

fn check_stmts_mutability(
    stmts: &[Statement],
    mutable: &mut HashSet<String>,
    errors: &mut Vec<SpannedError>,
) {
    for stmt in stmts {
        check_stmt_mutability(stmt, mutable, errors);
    }
}

fn check_stmt_mutability(
    stmt: &Statement,
    mutable: &mut HashSet<String>,
    errors: &mut Vec<SpannedError>,
) {
    match stmt {
        Statement::LetBinding {
            name,
            mutable: is_mut,
            ..
        } => {
            // `let` (mutable or not) rebinds the name in the flat env;
            // only `let mut` makes it assignable. TW: execution.rs:977-988.
            if *is_mut {
                mutable.insert(name.clone());
            }
        }
        Statement::Assign { name, span, .. } => {
            if !mutable.contains(name) {
                errors.push(SpannedError::at(
                    immutability_error_text(name),
                    span.clone(),
                ));
            }
            // The assigned value expression is walked by check_stmt_exprs.
        }
        // Recurse into every nested block with the SAME set — flat
        // function-level model, mirrors eval_statements_cf exactly.
        Statement::Each { body, .. } => check_stmts_mutability(body, mutable, errors),
        Statement::EachWithIndex { body, .. } => check_stmts_mutability(body, mutable, errors),
        Statement::While { body, .. } => check_stmts_mutability(body, mutable, errors),
        Statement::IfThen { body, .. } => check_stmts_mutability(body, mutable, errors),
        Statement::IfElseBlock {
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            check_stmts_mutability(then_body, mutable, errors);
            for (_, body) in else_ifs {
                check_stmts_mutability(body, mutable, errors);
            }
            if let Some(else_body) = else_body {
                check_stmts_mutability(else_body, mutable, errors);
            }
        }
        Statement::Match {
            arms, else_body, ..
        } => {
            for arm in arms {
                match arm {
                    MatchArm::Exact(_, body)
                    | MatchArm::StartsWith(_, body)
                    | MatchArm::Contains(_, body) => {
                        check_stmts_mutability(body, mutable, errors);
                    }
                    MatchArm::Compare(_, _, body) => {
                        check_stmts_mutability(body, mutable, errors);
                    }
                }
            }
            if let Some(else_body) = else_body {
                check_stmts_mutability(else_body, mutable, errors);
            }
        }
        Statement::Return { .. } | Statement::ExprStmt { .. } => {}
        // Наряд №266: memory statements never assign to variables — no
        // mutability implications.
        Statement::Memorize(_) | Statement::Forget(_) | Statement::Relate(_) => {}
        Statement::Break | Statement::Continue => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ok_program() {
        let source = r#"
            entity greeting: String = "Hello, Metalogos!"
            pattern SayHello(text: String) -> String { return text }
            flow Main { input: String = greeting -> SayHello -> output }
        "#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(result.is_ok());
        assert_eq!(result.error_count(), 0);
    }

    #[test]
    fn test_undefined_type() {
        let source = r#"
            entity m: UnknownType = { text: "hi" }
        "#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("unknown type")));
    }

    #[test]
    fn test_adapt_target_not_found() {
        let source = r#"
            adapt NonExistent add_example("in", "out")
        "#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("not found")));
    }

    #[test]
    fn test_duplicate_pattern() {
        let source = r#"
            pattern Foo(x: String) -> String { return x }
            pattern Foo(y: String) -> String { return y }
        "#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("duplicate pattern")));
    }

    // ── Phase 6 semantic tests ────────────────────────────────

    #[test]
    fn test_mlogserver_valid() {
        let source = r#"
mlogserver {
  port: 8080
  middleware: [session, csrf, security_headers]
  route "/" method=GET { return "Hello" }
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(result.is_ok(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_mlogserver_unknown_middleware() {
        let source = r#"
mlogserver {
  middleware: [bogus_middleware]
  route "/" method=GET { return "Hello" }
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("unknown middleware")));
    }

    #[test]
    fn test_mlogserver_invalid_method() {
        let source = r#"
mlogserver {
  route "/" method=INVALID { return "Hello" }
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("unknown HTTP method")));
    }

    #[test]
    fn test_mlogserver_warns_no_security_headers() {
        let source = r#"
mlogserver {
  route "/" method=GET { return "Hello" }
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(result.is_ok()); // Only warning, not error
        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("security_headers")));
    }

    #[test]
    fn test_mlogserver_warns_no_csrf_with_post() {
        let source = r#"
mlogserver {
  middleware: [session, security_headers]
  route "/login" method=POST { return "OK" }
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(result.is_ok());
        assert!(result.warnings.iter().any(|w| w.message.contains("csrf")));
    }

    #[test]
    fn test_template_valid() {
        let source = r#"
template Page(title: String) -> Html {
  <h1>{{ title }}</h1>
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(result.is_ok());
    }

    #[test]
    fn test_template_wrong_return_type() {
        let source = r#"
template Page(title: String) -> Secret {
  <h1>{{ title }}</h1>
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("only Html is supported")));
    }

    // ── Наряд №238 (Vision R4.1): vision declaration semantic ─────────

    /// Valid plan-§3 example — no errors, no warnings (no false positives).
    #[test]
    fn test_vision_valid_program_ok() {
        let source = r#"
vision "poster" {
  model: "z-image-turbo"
  steps: 8
  width: 1024
  height: 1024
  seed: 42
  policy: safe
  profile: fp16
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(result.is_ok(), "errors: {:?}", result.errors);
        assert!(
            result.warnings.is_empty(),
            "unexpected warnings: {:?}",
            result.warnings
        );
    }

    /// Block 3.2 negative: unknown model — semantic error naming the model
    /// and the known set.
    #[test]
    fn test_vision_unknown_model_is_semantic_error() {
        let source = r#"
vision "poster" {
  model: "flux-2"
  steps: 8
  width: 1024
  height: 1024
  seed: 42
  policy: safe
  profile: fp16
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        let hit = result
            .errors
            .iter()
            .find(|e| e.message.contains("unknown model 'flux-2'"))
            .expect("expected unknown-model error");
        assert!(
            hit.message.contains("z-image-turbo"),
            "got: {}",
            hit.message
        );
    }

    /// Block 3.2 negative: duplicate vision declaration name.
    #[test]
    fn test_vision_duplicate_name_is_semantic_error() {
        let source = r#"
vision "poster" { model: "z-image-turbo" steps: 8 width: 1024 height: 1024 seed: 1 policy: safe profile: fp16 }
vision "poster" { model: "z-image-turbo" steps: 8 width: 512 height: 512 seed: 2 policy: safe profile: fp8 }
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("duplicate vision declaration: poster")));
    }

    /// Block 3.2 negative: width not a multiple of 16.
    #[test]
    fn test_vision_width_not_multiple_of_16_is_error() {
        let source = r#"
vision "poster" {
  model: "z-image-turbo"
  steps: 8
  width: 1025
  height: 1024
  seed: 42
  policy: safe
  profile: fp16
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        let hit = result
            .errors
            .iter()
            .find(|e| e.message.contains("width = 1025 must be a multiple of 16"))
            .expect("expected width-contract error");
        assert!(
            hit.message.contains("vision 'poster'"),
            "got: {}",
            hit.message
        );
    }

    /// Block 3.2 negative: steps = 0 — semantic error (steps >= 1).
    #[test]
    fn test_vision_steps_zero_is_error() {
        let source = r#"
vision "poster" {
  model: "z-image-turbo"
  steps: 0
  width: 1024
  height: 1024
  seed: 42
  policy: safe
  profile: fp16
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(!result.is_ok());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("steps must be >= 1, got 0")));
    }

    /// Block 2.2: steps != 8 (but >= 1) is an audit-WARNING, not an error.
    #[test]
    fn test_vision_steps_not_8_is_warning_not_error() {
        let source = r#"
vision "poster" {
  model: "z-image-turbo"
  steps: 12
  width: 1024
  height: 1024
  seed: 42
  policy: safe
  profile: fp16
}
"#;
        let decls = crate::parser::parse(source).unwrap();
        let result = check_program(&decls);
        assert!(
            result.is_ok(),
            "steps=12 must not be an error; got: {:?}",
            result.errors
        );
        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("steps = 12 != 8")));
    }
}

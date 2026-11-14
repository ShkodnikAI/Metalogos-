// ── №952 (the audit 12.11, P0): the TW value-label line ──────────────
//
//! The runtime twin of the static lattice for the tree-walking backend.
//! The label of an EXPRESSION is computed by a pure walk
//! (`label_of_expr`) — no side effects, no re-evaluation — mirroring the
//! static `semantic::expr_label` arm-for-arm with ONE runtime-specific
//! replacement: the inter-procedural summary comes from the LIVE pattern
//! bodies (the runtime walks the actual `return` sites of the callee,
//! depth-capped) instead of the seeded static cache, so the line keeps
//! working when the static analysis is silenced entirely
//! (`METALOGOS_STATIC_LABEL_GATE_OFF=1`, the №952 "static off" test).
//!
//! The name-keyed env (`Interpreter::label_env`, VM parity) is updated at
//! the binding sites (LetBinding/Assign/entity writes/param seeding) and
//! read by the walk; the sink gate (`runtime_sink_gate`) refuses with the
//! SAME clearance rule the static gate applies
//! (`semantic::clearance_failure` — one rule, two lanes).

use super::Interpreter;
use crate::ast::{Expr, Statement};
use crate::interpreter::values::Value;
use crate::labels::Label;
use std::collections::BTreeMap;

/// The depth cap for the runtime pattern-return summary walk — the same
/// bound the static lane uses (`METALOGOS_TAINT_DEPTH` default 4, the
/// №376 overhead measurement).
const RUNTIME_TAINT_DEPTH: usize = 4;

impl Interpreter {
    /// The runtime label of an expression (the pure walk). `depth` bounds
    /// the pattern-summary recursion.
    pub(super) fn label_of_expr(&self, expr: &Expr, depth: usize) -> Label {
        self.label_of_expr_env(expr, depth, &BTreeMap::new())
    }

    /// The walk with an overlay env (the pattern-summary walk seeds
    /// parameter labels on top of the ambient `label_env`).
    pub(super) fn label_of_expr_env(
        &self,
        expr: &Expr,
        depth: usize,
        overlay: &BTreeMap<String, Label>,
    ) -> Label {
        let bottom = Label::bottom();
        match expr {
            Expr::StringLit { .. } | Expr::FloatLit { .. } | Expr::BoolLit { .. } => bottom,
            Expr::HandleSource { .. } => bottom,
            Expr::ProvBind { inner, .. } => self.label_of_expr_env(inner, depth, overlay),
            Expr::Ident { name, .. } => overlay
                .get(name)
                .cloned()
                .or_else(|| self.label_env.borrow().get(name).cloned())
                .unwrap_or_else(Label::bottom),
            Expr::FieldAccess { object, .. } => self.label_of_expr_env(object, depth, overlay),
            Expr::FnCall { name, args, .. } => self.label_of_call(name, args, depth, overlay),
            Expr::QualifiedCall {
                module,
                function,
                args,
                ..
            } => {
                // The tool-namespace summary mirrors the static lane: the
                // qualified name summarizes like a pattern.
                let mut acc = Label::bottom();
                for a in args {
                    acc = acc.join(&self.label_of_expr_env(a, depth, overlay));
                }
                if depth < RUNTIME_TAINT_DEPTH {
                    if let Some(qualified) = self.patterns.get(&format!("{module}.{function}")) {
                        let param_names: Vec<String> =
                            qualified.params.iter().map(|p| p.name.clone()).collect();
                        acc = acc.join(&self.pattern_return_summary(
                            &param_names,
                            args,
                            &qualified.body,
                            depth + 1,
                            overlay,
                        ));
                    }
                }
                acc
            }
            Expr::BinaryOp { left, right, .. } => self
                .label_of_expr_env(left, depth, overlay)
                .join(&self.label_of_expr_env(right, depth, overlay)),
            // Conditions do not taint values; branches do (the static
            // lane's rule — mirrored).
            Expr::IfElse {
                then_branch,
                else_branch,
                ..
            } => self
                .label_of_expr_env(then_branch, depth, overlay)
                .join(&self.label_of_expr_env(else_branch, depth, overlay)),
            Expr::List { items, .. } => {
                let mut acc = Label::bottom();
                for i in items {
                    acc = acc.join(&self.label_of_expr_env(i, depth, overlay));
                }
                acc
            }
            // Index access: the container's label rides into the element
            // (conservative: the container is the join of its contents).
            Expr::IndexAccess { object, .. } => self.label_of_expr_env(object, depth, overlay),
            Expr::StructLit { fields, .. } => {
                let mut acc = Label::bottom();
                for v in fields.values() {
                    acc = acc.join(&self.label_of_expr_env(v, depth, overlay));
                }
                acc
            }
            Expr::BlockIfElse {
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                let mut acc = Label::bottom();
                acc = acc.join(&self.block_tail_label(then_body, depth, overlay));
                for (_, body) in else_ifs {
                    acc = acc.join(&self.block_tail_label(body, depth, overlay));
                }
                if let Some(body) = else_body {
                    acc = acc.join(&self.block_tail_label(body, depth, overlay));
                }
                acc
            }
            Expr::MatchExpr {
                arms, else_body, ..
            } => {
                let mut acc = Label::bottom();
                for arm in arms {
                    acc = acc.join(&self.block_tail_label(arm.body(), depth, overlay));
                }
                if let Some(body) = else_body {
                    acc = acc.join(&self.block_tail_label(body, depth, overlay));
                }
                acc
            }
            Expr::Try { expr: inner, .. } => self.label_of_expr_env(inner, depth, overlay),
        }
    }

    /// The label of a CALL: the source/sanitizer floor from the static
    /// SSOT (`label_source` — one rule), the №689 private-field bridge,
    /// the №964 sink receipt, then the argument join LIFTED by the
    /// runtime pattern summary (the inter-procedural half the seeded
    /// static cache provided — computed live from the callee bodies).
    fn label_of_call(
        &self,
        name: &str,
        args: &[Expr],
        depth: usize,
        overlay: &BTreeMap<String, Label>,
    ) -> Label {
        // render: the position-aware №685 logic — mirrored verbatim from
        // the static walk (escaped slot: integrity restored only; raw
        // slot: conducts everything; unknown template: fail-closed join).
        if name == "render" {
            let snapshot = crate::html_label::render_raw_snapshot();
            let raw_positions = crate::html_label::resolve_raw_positions(&snapshot, args.first());
            let is_raw = |idx: usize| raw_positions.map(|p| p.contains(&idx)).unwrap_or(true);
            let mut acc = Label::bottom();
            for (idx, a) in args.iter().skip(1).enumerate() {
                let l = self.label_of_expr_env(a, depth, overlay);
                if is_raw(idx) {
                    acc = acc.join(&l);
                } else {
                    acc = acc.join(&Label {
                        conf: l.conf,
                        integrity: crate::labels::Integrity::Trusted,
                        consent: l.consent,
                    });
                }
            }
            return acc;
        }
        if let Some(l) = crate::semantic::label_source(name, args, &self.label_env_for(overlay)) {
            return l;
        }
        if let Some(l) = crate::semantic::registry_private_flow_label(name) {
            return l;
        }
        if crate::sink_receipt::is_sink_receipt(name) {
            return Label::bottom();
        }
        // Data flows through ordinary calls: the join of the arguments.
        let mut acc = Label::bottom();
        for a in args {
            acc = acc.join(&self.label_of_expr_env(a, depth, overlay));
        }
        // ── the runtime inter-procedural half (the №952 replacement of
        // the static summary): walk the LIVE callee body's return sites.
        if depth < RUNTIME_TAINT_DEPTH {
            if let Some(qualified) = self.patterns.get(name) {
                let param_names: Vec<String> =
                    qualified.params.iter().map(|p| p.name.clone()).collect();
                acc = acc.join(&self.pattern_return_summary(
                    &param_names,
                    args,
                    &qualified.body,
                    depth + 1,
                    overlay,
                ));
            }
        }
        acc
    }

    /// The overlay env for `label_source`: the ambient `label_env` plus
    /// the walk's parameter overlay.
    fn label_env_for(&self, overlay: &BTreeMap<String, Label>) -> BTreeMap<String, Label> {
        if overlay.is_empty() {
            return self.label_env.borrow().clone();
        }
        let mut env = self.label_env.borrow().clone();
        for (k, v) in overlay {
            env.insert(k.clone(), v.clone());
        }
        env
    }

    /// The runtime pattern summary: the join of the labels of ALL `return`
    /// sites in the body (recursively through the block shapes), with the
    /// parameters seeded from the CALL SITE's argument labels. This is the
    /// live replacement for the seeded static summary — the second line
    /// must work with the static gate OFF.
    pub(super) fn pattern_return_summary(
        &self,
        param_names: &[String],
        args: &[Expr],
        body: &[Statement],
        depth: usize,
        caller_overlay: &BTreeMap<String, Label>,
    ) -> Label {
        // Seed the parameter labels from the call site's argument labels.
        let mut overlay = caller_overlay.clone();
        for (param, arg) in param_names.iter().zip(args.iter()) {
            let l = self.label_of_expr_env(arg, depth, caller_overlay);
            overlay.insert(param.clone(), l);
        }
        self.summary_return_label(body, depth, &overlay)
    }

    fn summary_return_label(
        &self,
        body: &[Statement],
        depth: usize,
        overlay: &BTreeMap<String, Label>,
    ) -> Label {
        let mut acc = Label::bottom();
        for stmt in body {
            match stmt {
                Statement::Return { value, .. } => {
                    acc = acc.join(&self.label_of_expr_env(value, depth, overlay));
                }
                Statement::IfElseBlock {
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    acc = acc.join(&self.summary_return_label(then_body, depth, overlay));
                    for (_, body) in else_ifs {
                        acc = acc.join(&self.summary_return_label(body, depth, overlay));
                    }
                    if let Some(body) = else_body {
                        acc = acc.join(&self.summary_return_label(body, depth, overlay));
                    }
                }
                Statement::IfThen { body, .. } => {
                    acc = acc.join(&self.summary_return_label(body, depth, overlay));
                }
                Statement::Each { body, .. } | Statement::EachWithIndex { body, .. } => {
                    acc = acc.join(&self.summary_return_label(body, depth, overlay));
                }
                Statement::While { body, .. } => {
                    acc = acc.join(&self.summary_return_label(body, depth, overlay));
                }
                _ => {}
            }
        }
        acc
    }

    /// The last expression-value label of a block (for BlockIfElse /
    /// MatchExpr tails): the join of the block's tail expressions.
    fn block_tail_label(
        &self,
        body: &[Statement],
        depth: usize,
        overlay: &BTreeMap<String, Label>,
    ) -> Label {
        let mut acc = Label::bottom();
        for stmt in body {
            match stmt {
                Statement::Return { value, .. } => {
                    acc = acc.join(&self.label_of_expr_env(value, depth, overlay));
                }
                Statement::ExprStmt { expr, .. } => {
                    acc = acc.join(&self.label_of_expr_env(expr, depth, overlay));
                }
                _ => {}
            }
        }
        acc
    }

    /// The value-variant floor: a typed `Secret`/`WorldState` value
    /// carries its own secrecy regardless of the walk (the opaque
    /// variants' contracts, Phase 6.4 / №355).
    pub(super) fn value_variant_label(v: &Value) -> Label {
        crate::runtime_labels::value_variant_floor(v)
    }

    /// The №952 runtime sink gate: every classified Sink checks its
    /// arguments' runtime labels against THE SAME clearance rule the
    /// static gate applies. `arg_labels` are the walk's per-argument
    /// labels (may be empty when the site has only values — the variant
    /// floor still applies). The kv sidecar records BEFORE the check so
    /// the store always reflects what was attempted.
    pub(super) fn runtime_sink_gate(
        &mut self,
        name: &str,
        arg_values: &[Value],
        arg_labels: &[Label],
        line: u32,
    ) -> Result<(), String> {
        // №952 gate = SINKS + the №951 B6 request-egress family
        // (http_get et al. check confidentiality on the request side).
        if !crate::runtime_labels::is_gated_dispatch(name) {
            return Ok(());
        }
        // The kv round-trip contract: record the label the value carried
        // INTO the store (the ADR-0156 §5 «kv в пределах процесса
        // сохраняют» clause).
        if name == "kv_set" {
            if let (Some(crate::interpreter::values::Value::String(key_str)), Some(label)) =
                (arg_values.first(), arg_labels.get(1))
            {
                crate::runtime_labels::kv_label_record(key_str, label);
            }
        }
        for (i, value) in arg_values.iter().enumerate() {
            let walk_label = arg_labels.get(i).cloned().unwrap_or_else(Label::bottom);
            let label = walk_label.join(&Self::value_variant_label(value));
            crate::runtime_labels::runtime_sink_check(name, i, &label, line)?;
        }
        Ok(())
    }
}

use super::*;

impl Interpreter {
    pub(super) fn fire_on_write_hooks(&self, target: &str, args: &[Value]) {
        if self.hooks_on_write.is_empty() {
            return;
        }
        let mut hook_env = HashMap::new();
        hook_env.insert("target".to_string(), Value::String(target.to_string()));
        hook_env.insert("args".to_string(), Value::List(args.to_vec()));
        for hook in &self.hooks_on_write {
            // Ignore hook errors — hooks are advisory, not blocking
            let _ = self.eval_statements(&hook.body, &mut hook_env);
        }
    }

    /// Write-builtin names that trigger on_write hooks.
    pub(super) const WRITE_BUILTINS: &'static [&'static str] = &[
        "mem_set",
        "mtree_store",
        "db_execute",
        "write_file",
        "append_file",
    ];

    pub(super) fn is_write_builtin(name: &str) -> bool {
        Self::WRITE_BUILTINS.contains(&name)
    }

    /// Handle a mutate declaration: replace few-shot examples, compute mock accuracy, decide keep/rollback.
    pub(super) fn handle_mutate(&mut self, m: &MutateDecl) -> Result<String, String> {
        // Evaluate new examples first (before borrowing learnable mutably)
        let mut evaluated_examples: Vec<(String, String)> = Vec::new();
        for (input_expr, output_expr) in &m.new_examples {
            let input_str = match self.eval_expr(input_expr)? {
                Value::String(s) => s,
                other => format!("{}", other),
            };
            let output_str = match self.eval_expr(output_expr)? {
                Value::String(s) => s,
                other => format!("{}", other),
            };
            evaluated_examples.push((input_str, output_str));
        }

        let num_examples = evaluated_examples.len();

        // Now borrow the learnable pattern mutably
        let learnable = self
            .learnable_patterns
            .get_mut(&m.pattern_name)
            .ok_or_else(|| format!("mutate: learnable pattern '{}' not found", m.pattern_name))?;

        // Save original few-shot for rollback
        let original_few_shot = std::mem::take(&mut learnable.few_shot);

        // Replace with new examples
        learnable.few_shot = evaluated_examples;

        // Compute mock accuracy (always 0.95 for MockLlm)
        let accuracy: f64 = 0.95;

        // Check against threshold
        let kept = match (&m.rollback_op, &m.rollback_threshold) {
            (Some(op), Some(threshold)) => {
                match op {
                    CompareOp::Lt => accuracy >= *threshold,
                    CompareOp::Le => accuracy > *threshold,
                    CompareOp::Gt => false, // accuracy >= threshold is the "kept" condition
                    CompareOp::Ge => false,
                    CompareOp::Eq => (accuracy - threshold).abs() < 1e-9,
                    CompareOp::Ne => (accuracy - threshold).abs() >= 1e-9,
                }
            }
            _ => true, // No rollback condition → always keep
        };

        if kept {
            // Keep the new examples (already in place)
            let msg = Ok(format!(
                "[MUTATE] {}: accuracy={}, kept (>= {:.1})",
                m.pattern_name,
                accuracy,
                m.rollback_threshold.unwrap_or(0.0)
            ));
            // Phase 7.5: Audit log for mutate operations (after releasing mutable borrow)
            self.push_audit(format!(
                "[AUDIT] mutate {}: {} examples, accuracy={}",
                m.pattern_name, num_examples, accuracy
            ));
            msg
        } else {
            // Rollback: restore original few-shot
            let learnable = self
                .learnable_patterns
                .get_mut(&m.pattern_name)
                .ok_or_else(|| {
                    format!("mutate: learnable pattern '{}' not found", m.pattern_name)
                })?;
            learnable.few_shot = original_few_shot;
            let msg = Ok(format!(
                "[MUTATE] {}: accuracy={}, rolled back (below {:.1})",
                m.pattern_name,
                accuracy,
                m.rollback_threshold.unwrap_or(0.0)
            ));
            // Phase 7.5: Audit log for mutate operations (rolled back)
            self.push_audit(format!(
                "[AUDIT] mutate {}: {} examples, accuracy={} (rolled back)",
                m.pattern_name, num_examples, accuracy
            ));
            msg
        }
    }

    /// Take the mutate log messages (consuming them).
    pub fn take_mutate_log(&mut self) -> Vec<String> {
        std::mem::take(&mut self.mutate_log)
    }

    // ── Eval Harness (ADR-0050) ──────────────────────────────────────────

    /// Run all collected eval blocks and return results.
    /// Called after `run()` has registered learnable patterns (and adapt examples).
    pub fn run_eval_blocks(&self) -> Result<Vec<EvalResult>, String> {
        let mut results = Vec::new();
        for eval_decl in &self.eval_blocks {
            let result = self.run_single_eval(eval_decl)?;
            results.push(result);
        }
        Ok(results)
    }

    /// Run a single eval block: invoke learnable pattern on each dataset example,
    /// compare with expected, compute accuracy and confusion matrix.
    fn run_single_eval(&self, eval_decl: &EvalDecl) -> Result<EvalResult, String> {
        let learnable = self
            .learnable_patterns
            .get(&eval_decl.pattern_name)
            .ok_or_else(|| {
                format!(
                    "eval: learnable pattern '{}' not found",
                    eval_decl.pattern_name
                )
            })?;

        let mut correct = 0usize;
        let mut confusion: HashMap<String, HashMap<String, usize>> = HashMap::new();
        let mut failures: Vec<(String, String, String)> = Vec::new(); // (input, expected, actual)

        for (input_str, expected_label) in &eval_decl.dataset {
            // Build args from dataset input (single String argument)
            let args = vec![Value::String(input_str.clone())];

            // Invoke the learnable pattern
            let actual_value =
                self.invoke_learnable_with_env(&eval_decl.pattern_name, learnable, &args)?;
            let actual_label = match actual_value {
                Value::String(s) => s.trim().to_string(),
                other => format!("{}", other),
            };

            // Record in confusion matrix
            let pred_entry = confusion
                .entry(expected_label.clone())
                .or_default()
                .entry(actual_label.clone())
                .or_insert(0);
            *pred_entry += 1;

            if actual_label == *expected_label {
                correct += 1;
            } else {
                failures.push((input_str.clone(), expected_label.clone(), actual_label));
            }
        }

        let total = eval_decl.dataset.len();
        let accuracy = if total > 0 {
            correct as f64 / total as f64
        } else {
            1.0 // empty dataset → perfect by convention
        };

        Ok(EvalResult {
            pattern_name: eval_decl.pattern_name.clone(),
            metric: eval_decl.metric.clone(),
            total,
            correct,
            accuracy,
            threshold: eval_decl.threshold,
            passed: accuracy >= eval_decl.threshold,
            confusion,
            failures,
        })
    }
}

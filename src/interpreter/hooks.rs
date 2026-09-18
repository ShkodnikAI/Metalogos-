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

    /// Наряд №392: fire the on_deny handler for a refused action.
    ///
    /// Selection: an exact sink-class match wins over the `*` fallback;
    /// no covering handler → `Ok(false)` and the caller keeps the loud
    /// default error (deny behavior by default is unchanged).
    ///
    /// While the handler body runs, `current_deny_event` holds the typed
    /// event — `deny_event()` / `deny_reason()` read it, everything else
    /// sees loud errors. The handler runs AFTER the gate has already
    /// refused the action and its verdict is final: the handler can log,
    /// notify or degrade, it can never re-allow the refused action.
    pub(super) fn fire_on_deny(
        &self,
        event_args: crate::deny::DenyEventArgs,
    ) -> Result<bool, String> {
        let crate::deny::DenyEventArgs {
            reason,
            class,
            sink,
            argument,
            label,
            line,
            human,
        } = event_args;
        // ── Naryad #393 (ADR-0167 §3.4): the deny HAPPENED regardless of
        // whether a handler covers it — the ledger record is written
        // BEFORE handler selection, as a side effect of the refusal path
        // itself. Best-effort (loud stderr on failure, outcome unchanged).
        crate::ledger::record(
            &format!("deny.{}", reason),
            "runtime",
            &class,
            &format!("{}|{}|{}|{}|{}", sink, argument, label, line, human),
        );
        let handler = crate::deny::select_handler(
            &self
                .deny_handlers
                .iter()
                .map(|d| (d.class.clone(), ()))
                .collect::<Vec<_>>(),
            class.as_str(),
        )
        .and_then(|idx| self.deny_handlers.get(idx).cloned());
        let Some(handler) = handler else {
            return Ok(false);
        };
        // Observability parity with the runtime gate twin: the event is
        // on stderr with the same audit-event convention (№325/№328).
        eprintln!(
            "[DENY_EVENT][audit-event] {} refused {} (class {}, reason {}, line {}) — handled by on_deny({})",
            sink, argument, class, reason, line, handler.class
        );
        let event =
            crate::deny::make_event(&reason, &sink, &class, &argument, &label, line, &human);
        *self
            .current_deny_event
            .lock()
            .unwrap_or_else(|p| p.into_inner()) = Some(event);
        let mut handler_env = HashMap::new();
        let result = self.eval_statements(&handler.body, &mut handler_env);
        *self
            .current_deny_event
            .lock()
            .unwrap_or_else(|p| p.into_inner()) = None;
        // A failing handler is loud — a broken degradation path must not
        // masquerade as a handled refusal.
        result?;
        Ok(true)
    }

    /// Наряд №392: the live deny event (Some exactly while an on_deny
    /// body runs). Outside a handler this is a loud error — the event is
    /// runtime-constructed and cannot be forged or stale-read.
    pub(super) fn take_deny_event(&self) -> Result<Value, String> {
        self.current_deny_event
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
            .ok_or_else(|| "deny_event() is only available inside an on_deny handler".to_string())
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

        // Replace with new examples (cloned — the build-set inputs are
        // needed below for the held-out split)
        learnable.few_shot = evaluated_examples.clone();

        // ── Accuracy: REAL golden-task battery (№375, ADR-0112 addendum) ──
        //
        // Mock mode (METALOGOS_MOCK_LLM, default-on — the test-mode
        // convention used across the codebase): the 0.95 stub stays,
        // loudly documented here and in ADR-0112 — the rollback MECHANISM
        // is exercised, not a real quality signal.
        //
        // Real mode: the mutated pattern is measured on a golden-task
        // battery — the eval-block datasets registered for this pattern
        // (ADR-0050) plus the pre-mutation few-shot (the pattern's
        // established Q→A behavior) — and accuracy is measured ONLY on
        // held-out tasks (inputs that are NOT the mutation's own new
        // examples). The answer path is the pattern's real LLM call.
        let mock_mode = std::env::var("METALOGOS_MOCK_LLM")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(true);
        let pattern_name = m.pattern_name.clone();
        let (accuracy, battery_note) = if mock_mode {
            // Compute mock accuracy (always 0.95 for MockLlm) — test mode only.
            (0.95, String::new())
        } else {
            // Battery: eval-block datasets for this pattern first, then the
            // pre-mutation few-shot; dedup by input (first wins).
            let mut battery: Vec<(String, String)> = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for ed in &self.eval_blocks {
                if ed.pattern_name == pattern_name {
                    for (input, expected) in &ed.dataset {
                        if seen.insert(input.clone()) {
                            battery.push((input.clone(), expected.clone()));
                        }
                    }
                }
            }
            for (input, expected) in &original_few_shot {
                if seen.insert(input.clone()) {
                    battery.push((input.clone(), expected.clone()));
                }
            }
            let build_inputs: std::collections::HashSet<String> = evaluated_examples
                .iter()
                .map(|(input, _)| input.clone())
                .collect();
            let report = crate::interpreter::learnable::measure_battery_accuracy(
                &battery,
                &build_inputs,
                |input| self.call_llm_for_battery(&pattern_name, input),
            );
            let note = format!(
                " (battery: {} tasks, held-out {}, correct {}{})",
                report.battery_size,
                report.held_out,
                report.correct,
                if report.below_minimum {
                    ", BELOW MINIMUM 20"
                } else {
                    ""
                }
            );
            if report.held_out == 0 {
                eprintln!(
                    "[MUTATE] WARNING: no held-out battery tasks for '{}' — accuracy counts as 0.0 (no evidence, no keep)",
                    pattern_name
                );
            }
            (report.accuracy, note)
        };

        // Check against threshold
        // "kept = true" means the mutation is KEPT (not rolled back).
        // rollback_if: accuracy OP threshold → roll back if condition is TRUE.
        // So kept = NOT (accuracy OP threshold).
        let kept = match (&m.rollback_op, &m.rollback_threshold) {
            (Some(op), Some(threshold)) => match op {
                CompareOp::Lt => accuracy >= *threshold, // rollback if <, keep if >=
                CompareOp::Le => accuracy > *threshold,  // rollback if <=, keep if >
                CompareOp::Gt => accuracy <= *threshold, // rollback if >, keep if <=
                CompareOp::Ge => accuracy < *threshold,  // rollback if >=, keep if <
                // Наряд №206: Eq/Ne were inverted (bug since n148).
                // rollback_if: accuracy == 0.95 → roll back if equal, keep if different.
                CompareOp::Eq => (accuracy - threshold).abs() >= 1e-9,
                // rollback_if: accuracy != 0.95 → roll back if different, keep if equal.
                CompareOp::Ne => (accuracy - threshold).abs() < 1e-9,
            },
            _ => true, // No rollback condition → always keep
        };

        if kept {
            // Keep the new examples (already in place)
            let msg = Ok(format!(
                "[MUTATE] {}: accuracy={}, kept (>= {:.1}){}",
                m.pattern_name,
                accuracy,
                m.rollback_threshold.unwrap_or(0.0),
                battery_note
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
                "[MUTATE] {}: accuracy={}, rolled back (below {:.1}){}",
                m.pattern_name,
                accuracy,
                m.rollback_threshold.unwrap_or(0.0),
                battery_note
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

    // ── Test Blocks (Наряд №120) ──────────────────────────────────────

    /// Run all collected test blocks. Each test runs in isolation:
    /// assertion errors are caught per-test, other tests continue.
    pub fn run_test_blocks(&self) -> Vec<TestResult> {
        self.test_blocks
            .iter()
            .map(|td| self.run_single_test(td))
            .collect()
    }

    /// Run a single test block. Assertion errors (from assert_eq / assert_contains)
    /// are caught and reported as test failures. Panics are also caught.
    fn run_single_test(&self, test_decl: &TestDecl) -> TestResult {
        let mut scope: HashMap<String, Value> = HashMap::new();
        let result = self.eval_statements(&test_decl.body, &mut scope);
        match result {
            Ok(_) => TestResult {
                name: test_decl.name.clone(),
                passed: true,
                error: None,
            },
            Err(e) => TestResult {
                name: test_decl.name.clone(),
                passed: false,
                error: Some(e),
            },
        }
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

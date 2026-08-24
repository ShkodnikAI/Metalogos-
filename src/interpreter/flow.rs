use super::*;

impl Interpreter {
    pub(super) fn execute_rules(&mut self) -> Result<(), String> {
        // Sort by priority descending; stable sort preserves declaration order for ties.
        // ADR-0090: priority-ordered, first-wins semantics.
        // Rules are evaluated in priority-descending order. For each (entity, field)
        // pair, only the first matching rule (highest priority, earliest declared)
        // writes the field. Subsequent rules targeting the same field are skipped.
        // Rules targeting *different* fields of the same entity all fire.
        let mut sorted_rules: Vec<&RuleDecl> = self.rules.iter().collect();
        sorted_rules.sort_by_key(|b| std::cmp::Reverse(b.priority));

        // Track which (entity_name, field_name) pairs have already been written
        let mut written: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();

        for rule in sorted_rules {
            // Extract target name for dedup tracking
            let target_name = match &rule.target {
                Expr::Ident { name: name, .. } => name.clone(),
                _ => continue, // non-ident targets are not supported by rules
            };

            // Skip if this field was already written by a higher-priority rule
            if written.contains(&(target_name.clone(), rule.field.clone())) {
                continue;
            }

            let condition_met = self.eval_condition(&rule.condition, &self.variables)?;
            if condition_met {
                // Evaluate the value before mutation
                let _target_val = self.eval_expr(&rule.target)?;
                let value_val = self.eval_expr(&rule.value)?;

                if let Expr::Ident { name: name, .. } = &rule.target {
                    let entity = self
                        .variables
                        .get_mut(name)
                        .ok_or_else(|| format!("rule target '{}' not found", name))?;
                    entity.set_field(&rule.field, value_val)?;

                    // Mark this (entity, field) as written — first-wins
                    written.insert((name.clone(), rule.field.clone()));
                }
            }
        }
        Ok(())
    }

    /// Evaluate a rule condition.
    fn eval_condition(
        &self,
        cond: &Condition,
        env: &HashMap<String, Value>,
    ) -> Result<bool, String> {
        match cond {
            Condition::Contains { left, right } => {
                let lv = self.eval_expr_with_env(left, env)?;
                let rv = self.eval_expr_with_env(right, env)?;
                let ls = match &lv {
                    Value::String(s) => s.clone(),
                    other => {
                        return Err(format!(
                            "contains: left must be String, got {}",
                            other.type_name()
                        ))
                    }
                };
                let rs = match &rv {
                    Value::String(s) => s.clone(),
                    other => {
                        return Err(format!(
                            "contains: right must be String, got {}",
                            other.type_name()
                        ))
                    }
                };
                Ok(ls.contains(&rs))
            }
            Condition::Compare { left, op, right } => {
                let lv = self.eval_expr_with_env(left, env)?;
                let rv = self.eval_expr_with_env(right, env)?;
                let lf = lv.as_float()?;
                let rf = rv.as_float()?;
                Ok(match op {
                    CompareOp::Gt => lf > rf,
                    CompareOp::Lt => lf < rf,
                    CompareOp::Ge => lf >= rf,
                    CompareOp::Le => lf <= rf,
                    CompareOp::Eq => lf == rf,
                    CompareOp::Ne => lf != rf,
                })
            }
        }
    }

    /// ADR-0056: Save a checkpoint for a flow at a given pipeline step.
    fn save_checkpoint(
        &self,
        flow_name: &str,
        checkpoint_name: &str,
        step_index: usize,
        current_value: &Value,
    ) -> Result<(), String> {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        let data = CheckpointData {
            flow_name: flow_name.to_string(),
            checkpoint_name: checkpoint_name.to_string(),
            step_index,
            current_value: current_value.clone(),
            variables: self.variables.clone(),
            created_at: ts,
        };

        let state_json = serde_json::to_string(&data)
            .map_err(|e| format!("checkpoint serialization error: {}", e))?;

        // Try SQLite first
        if let Some(ref conn) = *self
            .checkpoint_db
            .lock()
            .map_err(|e| format!("checkpoint lock: {}", e))?
        {
            conn.execute(
                "INSERT OR REPLACE INTO checkpoints (flow_name, checkpoint_name, step_index, state_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![flow_name, checkpoint_name, step_index as i64, state_json, ts],
            ).map_err(|e| format!("checkpoint save error: {}", e))?;
        } else {
            // Fallback: in-memory
            let key = format!("{}:{}", flow_name, checkpoint_name);
            self.checkpoint_mem
                .lock()
                .map_err(|e| format!("checkpoint lock: {}", e))?
                .insert(key, data);
        }

        Ok(())
    }

    /// ADR-0056: Load a checkpoint for a flow. Returns None if not found.
    fn load_checkpoint(
        &self,
        flow_name: &str,
        checkpoint_name: &str,
    ) -> Result<Option<CheckpointData>, String> {
        // Try SQLite first
        if let Some(ref conn) = *self
            .checkpoint_db
            .lock()
            .map_err(|e| format!("checkpoint lock: {}", e))?
        {
            let mut stmt = conn.prepare(
                "SELECT state_json FROM checkpoints WHERE flow_name = ?1 AND checkpoint_name = ?2"
            ).map_err(|e| format!("checkpoint load error: {}", e))?;

            let result: Result<String, _> = stmt
                .query_row(rusqlite::params![flow_name, checkpoint_name], |row| {
                    row.get(0)
                });

            match result {
                Ok(state_json) => {
                    let data: CheckpointData = serde_json::from_str(&state_json)
                        .map_err(|e| format!("checkpoint deserialization error: {}", e))?;
                    Ok(Some(data))
                }
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(format!("checkpoint load error: {}", e)),
            }
        } else {
            // Fallback: in-memory
            let key = format!("{}:{}", flow_name, checkpoint_name);
            Ok(self
                .checkpoint_mem
                .lock()
                .map_err(|e| format!("checkpoint lock: {}", e))?
                .get(&key)
                .cloned())
        }
    }

    /// ADR-0056: Set the resume target for a specific flow and checkpoint.
    /// Must be called before `run()` to take effect.
    pub fn set_resume_target(&mut self, flow_name: &str, checkpoint_name: &str) {
        self.resume_target = Some((flow_name.to_string(), checkpoint_name.to_string()));
    }

    /// ADR-0056: List all checkpoints for a flow (public for tests and CLI).
    /// Returns Vec of (checkpoint_name, step_index, created_at).
    pub fn list_checkpoints(&self, flow_name: &str) -> Result<Vec<(String, usize, i64)>, String> {
        if let Some(ref conn) = *self
            .checkpoint_db
            .lock()
            .map_err(|e| format!("checkpoint lock: {}", e))?
        {
            let mut stmt = conn.prepare(
                "SELECT checkpoint_name, step_index, created_at FROM checkpoints WHERE flow_name = ?1 ORDER BY step_index"
            ).map_err(|e| format!("checkpoint list error: {}", e))?;

            let rows = stmt
                .query_map(rusqlite::params![flow_name], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)? as usize,
                        row.get::<_, i64>(2)?,
                    ))
                })
                .map_err(|e| format!("checkpoint list error: {}", e))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("checkpoint list error: {}", e))?;

            Ok(rows)
        } else {
            // In-memory fallback
            let mem = self
                .checkpoint_mem
                .lock()
                .map_err(|e| format!("checkpoint lock: {}", e))?;
            let prefix = format!("{}:", flow_name);
            let mut results: Vec<(String, usize, i64)> = mem
                .iter()
                .filter(|(k, _)| k.starts_with(&prefix))
                .map(|(_k, v)| (v.checkpoint_name.clone(), v.step_index, v.created_at))
                .collect();
            results.sort_by_key(|(_, idx, _)| *idx);
            Ok(results)
        }
    }

    /// ADR-0056: Delete a specific checkpoint (public for tests and cleanup).
    pub fn delete_checkpoint(&self, flow_name: &str, checkpoint_name: &str) -> Result<(), String> {
        if let Some(ref conn) = *self
            .checkpoint_db
            .lock()
            .map_err(|e| format!("checkpoint lock: {}", e))?
        {
            conn.execute(
                "DELETE FROM checkpoints WHERE flow_name = ?1 AND checkpoint_name = ?2",
                rusqlite::params![flow_name, checkpoint_name],
            )
            .map_err(|e| format!("checkpoint delete error: {}", e))?;
        } else {
            let key = format!("{}:{}", flow_name, checkpoint_name);
            self.checkpoint_mem
                .lock()
                .map_err(|e| format!("checkpoint lock: {}", e))?
                .remove(&key);
        }
        Ok(())
    }

    /// ADR-0056: Reset all in-memory checkpoints (for test isolation).
    pub fn reset_checkpoints(&self) {
        if let Ok(mut mem) = self.checkpoint_mem.lock() {
            mem.clear();
        }
    }

    /// Execute a flow: evaluate source, thread through pipeline steps.
    /// ADR-0056: After each step, check if a checkpoint follows. If so, save state.
    /// If resume_target is set, skip steps until we reach the checkpoint, then resume.
    pub(super) fn run_flow(&mut self, flow: &FlowDecl) -> Result<String, String> {
        // Register branch definitions for this flow
        self.branch_defs.clear();
        for (step_name, branches) in &flow.branch_defs {
            self.branch_defs.insert(step_name.clone(), branches.clone());
        }

        // ADR-0056: Determine resume start position
        let mut start_idx: usize = 0;
        let mut current: Option<Value> = None;

        if let Some((ref target_flow, ref target_cp)) = self.resume_target {
            if target_flow == &flow.name {
                // Try to load checkpoint
                if let Some(data) = self.load_checkpoint(&flow.name, target_cp)? {
                    // Restore variables from checkpoint
                    for (k, v) in data.variables {
                        self.variables.insert(k, v);
                    }
                    // Start from the step AFTER the checkpoint
                    start_idx = data.step_index + 1;
                    current = Some(data.current_value);
                } else {
                    return Err(format!(
                        "checkpoint '{}' not found for flow '{}'",
                        target_cp, flow.name
                    ));
                }
                // Clear resume target (one-shot)
                self.resume_target = None;
            }
        }

        // If no resume, evaluate the source expression
        let mut current = match current {
            Some(v) => v,
            None => self.eval_expr(&flow.source)?,
        };

        // ADR-0056: Build reverse map: step_index -> checkpoint names at that position
        let mut checkpoint_at: HashMap<usize, Vec<String>> = HashMap::new();
        for (cp_name, &step_idx) in &flow.checkpoints {
            checkpoint_at
                .entry(step_idx)
                .or_default()
                .push(cp_name.clone());
        }

        // Execute pipeline steps, starting from start_idx (0 for fresh run)
        for (i, step_name) in flow.pipeline.iter().enumerate() {
            if i < start_idx {
                continue; // Skip steps before resume point
            }
            current = self.run_flow_step(step_name, current)?;

            // Check if a checkpoint follows this step
            if let Some(cp_names) = checkpoint_at.get(&i) {
                for cp_name in cp_names {
                    self.save_checkpoint(&flow.name, cp_name, i, &current)?;
                }
            }
        }

        Ok(format!("{}", current))
    }

    /// Execute a single flow step: check branch_defs first, else invoke as pattern/builtin.
    fn run_flow_step(&mut self, step_name: &str, current: Value) -> Result<Value, String> {
        if let Some(branches) = self.branch_defs.get(step_name).cloned() {
            // Step has branch definitions — evaluate conditions against current value
            for branch in &branches {
                if self.eval_branch_condition(&branch.condition, &current)? {
                    return self.invoke(&branch.target, vec![current]);
                }
            }
            Err(format!("no branch matched in step '{}'", step_name))
        } else {
            // No branch definitions — invoke as pattern/builtin directly
            self.invoke(step_name, vec![current])
        }
    }

    /// Evaluate a branch condition: `target.field op threshold`
    fn eval_branch_condition(
        &self,
        cond: &BranchCondition,
        current: &Value,
    ) -> Result<bool, String> {
        // The target in branch_condition is the flow input value (current)
        let field_val = current
            .get_field(&cond.field)
            .map_err(|e| format!("branch condition: {}", e))?
            .clone();
        let threshold = self.eval_expr(&cond.threshold)?;
        let fv = field_val.as_float()?;
        let tv = threshold.as_float()?;
        Ok(match cond.op {
            CompareOp::Gt => fv > tv,
            CompareOp::Lt => fv < tv,
            CompareOp::Ge => fv >= tv,
            CompareOp::Le => fv <= tv,
            CompareOp::Eq => fv == tv,
            CompareOp::Ne => fv != tv,
        })
    }
}

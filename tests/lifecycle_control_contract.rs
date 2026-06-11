// ── ADR-0056 Contract Tests: Lifecycle Control — checkpoint/resume ──────────
//
// Contracts:
//   C1: checkpoint("name") in flow pipeline saves state after the step
//   C2: resume from checkpoint restores variables and continues from next step
//   C3: Multiple checkpoints in one flow work independently
//   C4: list_checkpoints returns all saved checkpoints for a flow
//   C5: delete_checkpoint removes a checkpoint
//   C6: Resume from nonexistent checkpoint returns error
//   C7: Flow without checkpoints runs normally (backward compatibility)
//   C8: Checkpoint captures current value correctly
//   C9: Resume restores variable scope

use metalogos::interpreter::Interpreter;
use metalogos::parser;

/// Helper: parse + run source, return interpreter.
fn run_source(source: &str) -> Result<Interpreter, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut interp = Interpreter::new();
    interp.run(declarations)?;
    Ok(interp)
}

/// Helper: run source on an existing interpreter (re-parse and re-run).
/// Used for resume: the interpreter retains checkpoint_mem from the first run.
fn rerun_on(interp: &mut Interpreter, source: &str) -> Result<Option<String>, String> {
    let declarations = parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    interp.run(declarations)
}

// ── C1: checkpoint saves state after step ──────────────────────────────

#[test]
fn test_checkpoint_saves_state() {
    let source = r#"
        pattern Step1(x: String) -> String {
            return "Step1:" + x
        }
        pattern Step2(x: String) -> String {
            return "Step2:" + x
        }
        entity n: String = "hello"
        flow TestFlow { input: String = n -> Step1 -> checkpoint("mid") -> Step2 -> output }
    "#;

    let interp = run_source(source).unwrap();
    let cps = interp.list_checkpoints("TestFlow").unwrap();
    assert_eq!(cps.len(), 1, "C1: should have 1 checkpoint");
    assert_eq!(cps[0].0, "mid", "C1: checkpoint name should be 'mid'");
    assert_eq!(cps[0].1, 0, "C1: checkpoint at step index 0 (after Step1)");
}

// ── C2: resume from checkpoint continues from next step ──────────────────

#[test]
fn test_resume_continues_from_checkpoint() {
    let source = r#"
        pattern Step1(x: String) -> String {
            return "S1:" + x
        }
        pattern Step2(x: String) -> String {
            return "S2:" + x
        }
        entity n: String = "data"
        flow TestFlow { input: String = n -> Step1 -> checkpoint("mid") -> Step2 -> output }
    "#;

    // Step 1: Run normally — checkpoint saved after Step1
    let mut interp = run_source(source).unwrap();

    // Step 2: Resume from "mid" on same interpreter (retains checkpoint_mem)
    interp.set_resume_target("TestFlow", "mid");
    let output = rerun_on(&mut interp, source).unwrap();
    // After Step1, value is "S1:data". Resume skips Step1, runs Step2 → "S2:S1:data"
    assert_eq!(output, Some("S2:S1:data".to_string()), "C2: resume should continue from Step2, getting 'S1:data' as input from checkpoint");
}

// ── C3: Multiple checkpoints in one flow ───────────────────────────────

#[test]
fn test_multiple_checkpoints() {
    let source = r#"
        pattern Step1(x: String) -> String {
            return "A:" + x
        }
        pattern Step2(x: String) -> String {
            return "B:" + x
        }
        pattern Step3(x: String) -> String {
            return "C:" + x
        }
        entity n: String = "x"
        flow TestFlow { input: String = n -> Step1 -> checkpoint("cp1") -> Step2 -> checkpoint("cp2") -> Step3 -> output }
    "#;

    let interp = run_source(source).unwrap();
    let cps = interp.list_checkpoints("TestFlow").unwrap();
    assert_eq!(cps.len(), 2, "C3: should have 2 checkpoints");
    assert_eq!(cps[0].0, "cp1");
    assert_eq!(cps[1].0, "cp2");
}

// ── C4: list_checkpoints returns all checkpoints ────────────────────────

#[test]
fn test_list_checkpoints() {
    let source = r#"
        pattern S(x: String) -> String { return x }
        pattern T(x: String) -> String { return x }
        entity n: String = "v"
        flow F { input: String = n -> S -> checkpoint("alpha") -> T -> checkpoint("beta") -> S -> output }
    "#;

    let interp = run_source(source).unwrap();
    let cps = interp.list_checkpoints("F").unwrap();
    assert_eq!(cps.len(), 2);
    assert_eq!(cps[0].0, "alpha");
    assert_eq!(cps[0].1, 0); // after first step (index 0)
    assert_eq!(cps[1].0, "beta");
    assert_eq!(cps[1].1, 1); // after second step (index 1)
}

// ── C5: delete_checkpoint removes a checkpoint ───────────────────────────

#[test]
fn test_delete_checkpoint() {
    let source = r#"
        pattern S(x: String) -> String { return x }
        entity n: String = "v"
        flow F { input: String = n -> S -> checkpoint("save") -> S -> output }
    "#;

    let interp = run_source(source).unwrap();
    assert_eq!(interp.list_checkpoints("F").unwrap().len(), 1);

    interp.delete_checkpoint("F", "save").unwrap();
    assert_eq!(interp.list_checkpoints("F").unwrap().len(), 0, "C5: checkpoint should be deleted");
}

// ── C6: Resume from nonexistent checkpoint returns error ──────────────

#[test]
fn test_resume_nonexistent_checkpoint() {
    let source = r#"
        pattern S(x: String) -> String { return x }
        entity n: String = "v"
        flow F { input: String = n -> S -> output }
    "#;

    let declarations = parser::parse(source).unwrap();
    let mut interp = Interpreter::new();
    interp.set_resume_target("F", "nonexistent");
    let result = interp.run(declarations);
    assert!(result.is_err(), "C6: resume from nonexistent checkpoint should fail");
    assert!(result.unwrap_err().contains("nonexistent"));
}

// ── C7: Flow without checkpoints runs normally ─────────────────────────

#[test]
fn test_flow_without_checkpoints_backward_compat() {
    let source = r#"
        pattern Double(x: String) -> String {
            return x + x
        }
        pattern Upper(x: String) -> String {
            return upper(x)
        }
        entity n: String = "hi"
        flow F { input: String = n -> Double -> Upper -> output }
    "#;

    // Should parse and run without errors; no checkpoints saved
    let interp = run_source(source).unwrap();
    assert_eq!(interp.list_checkpoints("F").unwrap().len(), 0, "C7: no checkpoints for flow without checkpoint()");
}

// ── C8: Checkpoint captures current value correctly ───────────────────

#[test]
fn test_checkpoint_captures_value() {
    let source = r#"
        pattern Prefix(x: String) -> String {
            return "PRE:" + x
        }
        pattern Suffix(x: String) -> String {
            return x + ":POST"
        }
        entity n: String = "test"
        flow F { input: String = n -> Prefix -> checkpoint("after_prefix") -> Suffix -> output }
    "#;

    // Run normally — checkpoint saved after Prefix
    let mut interp = run_source(source).unwrap();

    // Resume from "after_prefix"
    interp.set_resume_target("F", "after_prefix");
    let output = rerun_on(&mut interp, source).unwrap();
    // After Prefix, value is "PRE:test". Suffix adds ":POST" → "PRE:test:POST"
    assert_eq!(output, Some("PRE:test:POST".to_string()), "C8: resumed flow should get correct value from checkpoint");
}

// ── C9: Resume restores variable scope ─────────────────────────────────

#[test]
fn test_resume_restores_variables() {
    let source = r#"
        pattern Step1(x: String) -> String {
            mem_set("flow_state", "running")
            return x
        }
        pattern Step2(x: String) -> String {
            let state = mem_get("flow_state")
            return x + ":" + state
        }
        entity n: String = "data"
        flow F { input: String = n -> Step1 -> checkpoint("after_s1") -> Step2 -> output }
    "#;

    // Run normally — mem_set happens in Step1, then checkpoint saves variables
    let mut interp = run_source(source).unwrap();

    // Resume from "after_s1" — variables restored, Step2 reads mem_get("flow_state")
    interp.set_resume_target("F", "after_s1");
    let output = rerun_on(&mut interp, source).unwrap();
    assert_eq!(output, Some("data:running".to_string()), "C9: resumed flow should have variables restored from checkpoint");
}

// ── C10: Reset checkpoints clears all ──────────────────────────────────

#[test]
fn test_reset_checkpoints() {
    let source = r#"
        pattern S(x: String) -> String { return x }
        entity n: String = "v"
        flow F { input: String = n -> S -> checkpoint("a") -> S -> checkpoint("b") -> output }
    "#;

    let interp = run_source(source).unwrap();
    assert_eq!(interp.list_checkpoints("F").unwrap().len(), 2);

    interp.reset_checkpoints();
    assert_eq!(interp.list_checkpoints("F").unwrap().len(), 0, "C10: reset should clear all checkpoints");
}

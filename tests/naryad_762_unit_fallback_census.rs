// ── №762 (gh#1415, Wave 58): the census/classification of the 129 quiet
// `unwrap_or(Value::Unit)` fallbacks in src/vm.rs — the loud operand
// contract (ADR-0194 package A = (c) RATIFIED) ─────────────────────────
//
// The census (the r29 anchor list was the INPUT, the live handlers are
// the verdict — the counts below are the classification record):
//
//   (a) 47 sites PINNED, semantics unchanged (no fix, no pin comment in
//       source — the battery carries the verification, the ADR-0194 (c)
//       form "the batteries, not a dead-branch unit test"):
//     (a1) 39 loud-through-result — the fallback value feeds an
//          evaluator that rejects Unit loudly on EVERY path: the binop
//          family Add/Sub/Mul/Div (eval_binop catch-all), Contains on
//          the run loop (eval_contains), CmpGt/Lt/Ge/Le (the eval_cmp
//          catch-all — ONLY CmpEq/CmpNe answer Unit silently, so the Eq/
//          Ne pair is class (b)), GetField (get_field — the №756
//          contract), the eval_rule_value FieldAccess read (get_field),
//          and LoadGlobalByName — every valid-program emission is a
//          name-reference position (compiler.rs Expr::Ident fallback +
//          the semantic name-reference walk) whose miss lands the
//          builtin's own loud "model not declared" — the semantic.rs
//          "unchanged" contract forbids touching that text;
//     (a2) 6 documented-soft language semantics — the IndexAccess
//          in-value lookups (`items.get(i)`, `fields.get(key)`): an
//          out-of-range index / a missing struct key is a VALID-program
//          event with the documented Unit answer — NOT an underflow;
//     (a3) 2 internal register-stack pops (EndValueExpr over
//          value_registers — paired by BeginValueExpr inside the same
//          loop, not bytecode-stream-controlled);
//   (b) 82 reachable-masking — THIS landing: the .mbc delivery path
//       (deserialize, bytecode.rs — no version/integrity gate) makes
//       stack underflow and slot-bounds misses REACHABLE (a desynced or
//       corrupt program), and the silent Unit fallback flows into
//       results, stores, branches and memory as the №604 class-4
//       silently-wrong shape. Every class-(b) site now fails LOUDLY
//       naming the instruction ("VM: stack underflow at …"). Zero
//       observable change on valid bytecode: the compiler guarantees
//       every operand (Statement::Return compiles to `expr; Return` in
//       every context — compiler.rs; the body epilogue leaves the value,
//       №582 PushUnit; the local slots are constructor-paired).
//
// The pins below craft the bytecode DIRECTLY (the underflow is
// unreachable from the source grammar by construction — that is the
// point), drive BOTH VM loops (run() = execute_main_code;
// execute_code_inner() via a pattern body), and hold the happy-path
// negative controls (valid programs unchanged).
#![allow(clippy::disallowed_methods)]

use metalogos::bytecode::{Instruction, Program};
use metalogos::interpreter::Value;
use std::path::PathBuf;
use std::sync::Arc;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn bare_program(main_code: Vec<Instruction>) -> Program {
    Program {
        globals: vec![],
        patterns: Arc::new(vec![]),
        learnables: vec![],
        rules: vec![],
        skill_indices: vec![],
        reflex_decls: vec![],
        reflex_seq_decls: vec![],
        reflex_gen_decls: vec![],
        vision_decls: vec![],
        origin_decls: vec![],
        deny_handlers: vec![],
        memory_persist_path: None,
        conversation_config: Default::default(),
        db_url: None,
        db_url_env: None,
        schema_ddl: vec![],
        main_code,
        collections_loaded: false,
        shared_cache: metalogos::bytecode::ProgramSharedCache::new(),
    }
}

fn underflow_program(instrs: Vec<Instruction>) -> Program {
    let mut p = bare_program(instrs);
    p.main_code.push(Instruction::Halt);
    p
}

fn pattern_program(body: Vec<Instruction>) -> Program {
    let mut p = bare_program(vec![
        Instruction::RegisterPatternRef(0),
        Instruction::CallPattern(0, 0),
    ]);
    p.main_code.push(Instruction::Halt);
    p.patterns = Arc::new(vec![metalogos::bytecode::CompiledFn {
        name: "P".to_string(),
        param_count: 0,
        param_types: vec![],
        code: body,
        is_pure: true,
    }]);
    p
}

fn run_vm(p: Program) -> Result<Option<String>, String> {
    let mut vm = metalogos::vm::Vm::new();
    vm.run(p)
}

fn compile_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source.trim()).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(repo_root());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

// ── Class (b) pins — the RUN loop (execute_main_code) ────────────────

#[test]
fn n762_run_loop_underflow_pins() {
    // CmpEq — the ONLY binop family whose evaluator answers Unit silently
    // (eval_cmp Eq: Unit==Unit → Ok(true)) — the strongest masking shape.
    let err = run_vm(underflow_program(vec![Instruction::CmpEq])).unwrap_err();
    assert!(
        err.contains("stack underflow at CmpEq (right)"),
        "CmpEq must refuse naming the instruction; got: {err}"
    );
    // Stores — the underflowed Unit used to overwrite the slot silently.
    for (prog, name) in [
        (vec![Instruction::StoreGlobal(0)], "StoreGlobal"),
        (vec![Instruction::StoreLocal(0)], "StoreLocal"),
    ] {
        let err = run_vm(underflow_program(prog)).unwrap_err();
        assert!(
            err.contains(&format!("stack underflow at {name}")),
            "{name} must refuse loudly; got: {err}"
        );
    }
    // Branches — the underflowed Unit used to take the falsy arm silently.
    for (prog, name) in [
        (vec![Instruction::JumpIfNot(9)], "JumpIfNot"),
        (vec![Instruction::JumpIfNotCond(9)], "JumpIfNotCond"),
    ] {
        let err = run_vm(underflow_program(prog)).unwrap_err();
        assert!(
            err.contains(&format!("stack underflow at {name}")),
            "{name} must refuse loudly; got: {err}"
        );
    }
    // MakeList — the count comes from the (corruptible) bytecode stream.
    let err = run_vm(underflow_program(vec![Instruction::MakeList(3)])).unwrap_err();
    assert!(
        err.contains("stack underflow at MakeList (element)"),
        "MakeList must refuse loudly; got: {err}"
    );
    // Memory ops — the underflowed Unit used to enter memory as "()" text.
    for (prog, name) in [
        (vec![Instruction::Memorize(1.0)], "Memorize"),
        (vec![Instruction::Recall], "Recall"),
    ] {
        let err = run_vm(underflow_program(prog)).unwrap_err();
        assert!(
            err.contains(&format!("stack underflow at {name}")),
            "{name} must refuse loudly; got: {err}"
        );
    }
}

#[test]
fn n762_run_loop_slot_bounds_pins() {
    // LoadGlobal with a slot the globals table does not have — the
    // underflowed Unit used to flow into the computation silently.
    let err = run_vm(underflow_program(vec![Instruction::LoadGlobal(7)])).unwrap_err();
    assert!(
        err.contains("stack underflow at LoadGlobal (global slot 7 out of range)"),
        "LoadGlobal must name the slot; got: {err}"
    );
    // CallBuiltin arity-pops name the callee (the std table pre-registers
    // names, so the exact callee at index 0 is environment-dependent —
    // the pin is the NAME + the operand fact).
    let err = run_vm(underflow_program(vec![Instruction::CallBuiltin(0, 2)])).unwrap_err();
    assert!(
        err.contains("stack underflow at CallBuiltin '") && err.contains("(operand)"),
        "CallBuiltin must name the callee; got: {err}"
    );
    // Return — the main-flow return on an empty stack (the compiler emits
    // `expr; Return` in every context, so the pop is constructor-guaranteed).
    let err = run_vm(underflow_program(vec![Instruction::Return])).unwrap_err();
    assert!(
        err.contains("stack underflow at Return"),
        "Return must refuse loudly; got: {err}"
    );
}

// ── Class (b) pins — the INNER loop (execute_code_inner) ─────────────

#[test]
fn n762_inner_loop_underflow_pins() {
    // A pattern body running CmpEq on the (empty) frame stack — the
    // execute_code_inner site of the run-loop pin.
    let err = run_vm(pattern_program(vec![Instruction::CmpEq])).unwrap_err();
    assert!(
        err.contains("stack underflow at CmpEq (right)"),
        "inner CmpEq must refuse naming the instruction; got: {err}"
    );
    // Inner LoadGlobal slot bounds.
    let err = run_vm(pattern_program(vec![Instruction::LoadGlobal(7)])).unwrap_err();
    assert!(
        err.contains("stack underflow at LoadGlobal (global slot 7 out of range)"),
        "inner LoadGlobal must name the slot; got: {err}"
    );
    // Inner Return — the body return on an empty stack (was: Ok(Unit)
    // silently — the silent body value).
    let err = run_vm(pattern_program(vec![Instruction::Return])).unwrap_err();
    assert!(
        err.contains("stack underflow at Return"),
        "inner Return must refuse loudly; got: {err}"
    );
    // The body-epilogue tail pop — an empty body leaves no value (was:
    // Ok(Unit) silently — the desync swallowed as a "successful Unit").
    let err = run_vm(pattern_program(vec![])).unwrap_err();
    assert!(
        err.contains("stack underflow at the body epilogue (the return value)"),
        "the epilogue tail must refuse loudly; got: {err}"
    );
}

// ── Class (a1) pins — loud-through-result (the classification record) ─

#[test]
fn n762_class_a1_loud_through_result_pins() {
    // Add on an empty stack: the Unit fallback flows into eval_binop,
    // which refuses LOUDLY with the type-mismatch text — the masking is
    // impossible by construction (the (a1) pin: the error is the
    // evaluator's, NOT the underflow text).
    let err = run_vm(underflow_program(vec![Instruction::Add])).unwrap_err();
    assert!(
        err.contains("type mismatch"),
        "Add underflow must stay loud-through-result (eval_binop); got: {err}"
    );
    assert!(
        !err.contains("stack underflow"),
        "Add is class (a1) — no underflow text expected; got: {err}"
    );
    // CmpGt — the eval_cmp catch-all (the Eq/Ne arms are the silent
    // exception; Gt refuses loudly).
    let err = run_vm(underflow_program(vec![Instruction::CmpGt])).unwrap_err();
    assert!(
        err.contains("type mismatch") && !err.contains("stack underflow"),
        "CmpGt must stay loud-through-result; got: {err}"
    );
    // GetField on the underflowed Unit — get_field refuses loudly (the
    // №756 contract; the pop fallback stays — the (a1) pin).
    let err = run_vm(underflow_program(vec![Instruction::GetField("f".into())])).unwrap_err();
    assert!(
        err.contains("cannot access field 'f'"),
        "GetField must stay loud-through-result (get_field); got: {err}"
    );
}

// ── Class (a2)/(a1) pins — the documented soft semantics hold ────────

#[test]
fn n762_class_a2_documented_soft_pins() {
    // IndexAccess out-of-range on a VALID program — the documented soft
    // Unit answer (the (a2) pin: NOT an underflow, no loud refusal).
    let out = compile_vm(
        r#"
pattern P(_x: String) -> String {
  let l = [10.0, 20.0]
  return str(l[9])
}
flow Main { input: String = "x" -> P -> output }
"#,
    )
    .expect("the valid program must run");
    assert_eq!(
        out.as_deref(),
        Some("()"),
        "the soft Unit answer is documented"
    );
    // str() of a List<String> element path — the struct-key miss twin:
    let out = compile_vm(
        r#"
pattern P(_x: String) -> String {
  let s = {a: 1.0}
  return str(s["zz"])
}
flow Main { input: String = "x" -> P -> output }
"#,
    )
    .expect("the valid program must run");
    assert_eq!(
        out.as_deref(),
        Some("()"),
        "the missing-key soft answer is documented"
    );
}

#[test]
fn n762_class_a1_loadglobalbyname_soft_miss_pin() {
    // LoadGlobalByName on a name nothing resolves — the handler keeps the
    // Unit fallback (the (a1) classification: every valid-program emission
    // is a name-reference position whose miss lands the builtin's loud
    // error; the handler itself must NOT grow an underflow refusal — the
    // semantic.rs "unchanged" contract). The crafted stream runs to Halt.
    let p = underflow_program(vec![Instruction::LoadGlobalByName("zzz".into())]);
    let out = run_vm(p).expect("the soft name-miss must not refuse");
    assert_eq!(out, None);
}

// ── Happy-path negative controls — valid programs unchanged ──────────

#[test]
fn n762_happy_path_controls_unchanged() {
    // Comparison + branches (the CmpEq/JumpIfNotCond handler family) +
    // the CmpGt path through a Float local (LoadLocal + CmpGt).
    let out = compile_vm(
        r#"
pattern P(x: String) -> String {
  if x == "5" {
    return "five"
  }
  return "other"
}
flow Main { input: String = "5" -> P -> output }
"#,
    )
    .expect("valid comparison/branch program must run");
    assert_eq!(out.as_deref(), Some("five"));
    let out = compile_vm(
        r#"
pattern P(_x: String) -> String {
  let a = 3.0
  if a > 2.0 {
    return "big"
  }
  return "small"
}
flow Main { input: String = "z" -> P -> output }
"#,
    )
    .expect("valid CmpGt/branch program must run");
    assert_eq!(out.as_deref(), Some("big"));

    // Globals (StoreGlobal/LoadGlobal — the exact handlers the class-(b)
    // fixes touched): a crafted constructor-guaranteed sequence — store,
    // load back, return — the slot roundtrip is unchanged.
    let mut p = bare_program(vec![
        Instruction::const_(Value::Float(7.0)),
        Instruction::StoreGlobal(0),
        Instruction::LoadGlobal(0),
        Instruction::Return,
    ]);
    p.globals = vec!["g".to_string()];
    let out = run_vm(p).expect("the crafted global roundtrip must run");
    assert_eq!(out.as_deref(), Some("7"));

    // Locals, struct make/read, list make/len — the exact handlers the
    // class-(b) fixes touched.
    let out = compile_vm(
        r#"
pattern P(_x: String) -> String {
  let l = [1.0, 2.0, 3.0]
  let s = {name: "m", weight: 2.0}
  let n = len(l)
  let local = n + 7.0
  return str(local) + "/" + str(len(l)) + "/" + s.name
}
flow Main { input: String = "x" -> P -> output }
"#,
    )
    .expect("valid locals/struct/list program must run");
    assert_eq!(out.as_deref(), Some("10/3/m"));

    // CmpEq/Ne on real values — the silent-Unit arms stay value-driven.
    let out = compile_vm(
        r#"
pattern P(_x: String) -> String {
  if 1.0 == 1.0 {
    return "eq"
  }
  return "ne"
}
flow Main { input: String = "x" -> P -> output }
"#,
    )
    .expect("valid CmpEq program must run");
    assert_eq!(out.as_deref(), Some("eq"));

    // Memory ops on real values — the format path is unchanged for
    // Strings (the Memorize/Recall pair).
    let out = compile_vm(
        r#"
pattern P(_x: String) -> String {
  memorize("alpha-bravo")
  return recall("alpha")
}
flow Main { input: String = "x" -> P -> output }
"#,
    )
    .expect("valid memory program must run");
    assert_eq!(out.as_deref(), Some("alpha-bravo"));

    // Dup/value channels — the KeepLastValue/SetValueReg/Dup family.
    let out = compile_vm(
        r#"
pattern P(_x: String) -> String {
  return str(len([1.0, 2.0]) * 2.0)
}
flow Main { input: String = "x" -> P -> output }
"#,
    )
    .expect("valid value-expr program must run");
    assert_eq!(out.as_deref(), Some("4"));
}

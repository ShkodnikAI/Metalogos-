// ── Наряд №264 (issue #280): immutability enforced across ALL backends ──
//
// Fact (verified on main @ 0d356dd, probe pattern+flow with
// `let x = 10` / `x = 20`): three backends, three answers —
//   * `mlog check`  → "OK: no issues found." exit 0  (static analysis SKIPPED it);
//   * `mlog run`    → error: cannot assign to immutable variable: x ... exit 1
//                     (the contract — №14, REFERENCE.md §3.2, examples/p30_assign_*);
//   * `mlog compile` + `mlog run x.mbc` (VM) → silently printed 20, exit 0
//                     (contract broken SILENTLY).
//
// Fix roots:
//   1. semantic.rs — new static mutability pass over pattern/route bodies;
//      `mlog check` now rejects before any backend runs (TW-mirroring model:
//      flat never-popped mutable set, params/each-vars immutable).
//   2. compiler.rs — the compiler knows mut-ness at compile time: assignment
//      to a non-`let mut` name is a COMPILE ERROR (before serialization).
//   3. vm.rs / bytecode.rs — VM backstop: assignments travel as
//      `StoreAssignLocal { slot, name, mutable }` (instruction metadata;
//      Program schema untouched); `mutable: false` on the wire fails LOUDLY.
//
// Done-when (наряд): the probe-fact is inverted — check errors; bytecode
// produced past the check never silently assigns on the VM; `let mut`
// programs keep working on every backend; corpus stays green.

use metalogos::bytecode::{Instruction, Program};
use metalogos::interpreter::Value;

const TEXT: &str = "cannot assign to immutable variable: x (use 'let mut x' to make it mutable)";

/// The probe from the наряд fact (pattern + flow, `let x = 10` then `x = 20`).
const VIOLATING: &str = r#"
pattern Probe(dummy: String) -> String {
  let x = 10.0
  x = 20.0
  return to_string(x)
}
flow Main { input: String = "x" -> Probe -> output }
"#;

/// Negative control: the same program with `let mut` must keep working
/// on every backend.
const MUT_CONTROL: &str = r#"
pattern Probe(dummy: String) -> String {
  let mut x = 10.0
  x = 20.0
  return to_string(x)
}
flow Main { input: String = "x" -> Probe -> output }
"#;

/// Fact part 1 inverted: `mlog check` is no longer silent — the static
/// pass rejects the assignment before any backend runs.
#[test]
fn n264_check_rejects_non_mut_assign() {
    let result = metalogos::check_program(VIOLATING).unwrap();
    assert!(
        !result.is_ok(),
        "mlog check must reject assignment to a non-mut variable (was: silent OK)"
    );
    assert!(
        result.errors.iter().any(|e| e.message == TEXT),
        "check error must carry the TW-parity text {:?}, got: {:?}",
        TEXT,
        result.errors
    );
}

/// Contract preserved (наряд #14): the TW interpreter keeps rejecting the
/// same source at runtime — `mlog check` did not weaken the runtime check.
#[test]
fn n264_tw_contract_unchanged() {
    let err = metalogos::run_program(VIOLATING).unwrap_err();
    assert!(
        err.contains(TEXT),
        "TW runtime error must keep the contract text, got: {}",
        err
    );
}

/// Fact part 3 root closed on the compile side: the compiler knows mut-ness
/// at compile time, so a violating program never serializes into an .mbc.
#[test]
fn n264_compile_rejects_non_mut_assign() {
    let err = metalogos::compile_program(VIOLATING).unwrap_err();
    assert!(
        err.contains(TEXT),
        "compile must reject non-mut assignment with the TW-parity text, got: {}",
        err
    );
}

/// Backend parity (наряд task 3): one source → check rejects it before both
/// backends, and BOTH backends answer loudly with the SAME contract text
/// (TW at runtime, compiler before serialization) — no third answer.
#[test]
fn n264_backend_parity_same_source_both_loud() {
    let chk = metalogos::check_program(VIOLATING).unwrap();
    assert!(!chk.is_ok(), "check must reject first");

    let tw = metalogos::run_program(VIOLATING);
    let vmc = metalogos::compile_program(VIOLATING);
    assert!(tw.is_err(), "TW must be loud");
    assert!(vmc.is_err(), "VM path must be loud");
    let tw_err = tw.unwrap_err();
    let vm_err = vmc.unwrap_err();
    assert!(
        tw_err.contains(TEXT) && vm_err.contains(TEXT),
        "both backends must carry the same contract text:\n  TW: {}\n  VM: {}",
        tw_err,
        vm_err
    );
}

/// The VM backstop: bytecode produced PAST the check (a faithful compiler
/// that wrongly allowed the assignment encodes `mutable: false`) must fail
/// loudly on the VM — it must never silently print 20 again.
#[test]
fn n264_vm_backstop_loud_on_past_check_mbc() {
    // Compile the LEGIT mut program (the compiler only ever emits
    // `mutable: true` since non-mut assigns are rejected at compile time).
    let mut program = metalogos::compile_program(MUT_CONTROL).unwrap();

    // Simulate "compiled past-check": flip the immutability fact the
    // compiler recorded on every assignment instruction. BOTH copies must
    // be flipped — the VM pre-registers patterns from main_code's
    // RegisterPattern instructions (№250) and re-registers them during
    // execute_main_code, so main_code's clone is what actually executes.
    for pat in program.patterns.iter_mut() {
        for instr in pat.code.iter_mut() {
            if let Instruction::StoreAssignLocal { mutable, .. } = instr {
                *mutable = false;
            }
        }
    }
    for instr in program.main_code.iter_mut() {
        if let Instruction::RegisterPattern(fn_def) = instr {
            for code_instr in fn_def.code.iter_mut() {
                if let Instruction::StoreAssignLocal { mutable, .. } = code_instr {
                    *mutable = false;
                }
            }
        }
    }

    // Full .mbc round-trip, exactly like `mlog run file.mbc`.
    let bytes = program.serialize().unwrap();
    let reloaded = Program::deserialize(&bytes).unwrap();
    let result = metalogos::run_bytecode(reloaded);

    match result {
        Ok(output) => panic!(
            "VM must NOT silently execute an immutable assignment (output: {:?} — the fact printed 20)",
            output
        ),
        Err(e) => assert!(
            e.contains(TEXT),
            "VM backstop must fail with the TW-parity text, got: {}",
            e
        ),
    }
}

/// Negative control (не сломать легитимный mut): the `let mut` program is
/// OK on `mlog check`, TW and VM — the contract only ever covered non-mut
/// assignments.
#[test]
fn n264_let_mut_control_all_backends_green() {
    let chk = metalogos::check_program(MUT_CONTROL).unwrap();
    assert!(
        chk.is_ok(),
        "let mut must pass check, got: {:?}",
        chk.errors
    );

    let tw = metalogos::run_program(MUT_CONTROL).unwrap();
    assert_eq!(
        tw.as_deref(),
        Some("20"),
        "TW must still assign through let mut"
    );

    let program = metalogos::compile_program(MUT_CONTROL).unwrap();
    let vm = metalogos::run_bytecode(program).unwrap();
    assert_eq!(
        vm.as_deref(),
        Some("20"),
        "VM must still assign through let mut"
    );
}

/// TW-model mirror (no false positives): `each` loop variables are
/// immutable (TW env.insert-es them per iteration, never into
/// mutable_vars), while a `let mut` declared inside a nested block stays
/// in effect AFTER the block (TW threads the flat mutable_vars through
/// every block without popping) — `mlog check` mirrors both exactly.
#[test]
fn n264_each_var_immutable_and_mut_leak_mirrors_tw() {
    // each-loop variable assignment: rejected, same as `mlog run`.
    let each_violation = r#"
pattern Sum(items: Float) -> Float {
  let total = 0.0
  each it in [1.0, 2.0] {
    it = 99.0
    total = total + it
  }
  return total
}
flow Main { input: String = "x" -> Sum -> output }
"#;
    let chk = metalogos::check_program(each_violation).unwrap();
    assert!(
        !chk.is_ok()
            && chk.errors.iter().any(|e| e
                .message
                .contains("cannot assign to immutable variable: it")),
        "each-loop variable must be immutable for check (TW parity), got: {:?}",
        chk.errors
    );
    let err = metalogos::run_program(each_violation).unwrap_err();
    assert!(err.contains("cannot assign to immutable variable: it"));

    // let-mut leak: TW accepts, so check must accept too.
    let leak_control = r#"
pattern Leaky(flag: Float) -> Float {
  let x = 1.0
  if flag > 0.0 {
    let mut x = 5.0
    x = 6.0
  }
  x = 7.0
  return x
}
flow Main { input: Float = 1.0 -> Leaky -> output }
"#;
    let chk = metalogos::check_program(leak_control).unwrap();
    assert!(
        chk.is_ok(),
        "flat TW mutability model (leak through blocks) must not be flagged: {:?}",
        chk.errors
    );
    // And the program behaves identically on both backends.
    let tw = metalogos::run_program(leak_control).unwrap();
    let vm = metalogos::run_bytecode(metalogos::compile_program(leak_control).unwrap()).unwrap();
    assert_eq!(tw, vm, "backend parity for the leak control");
}

/// Pattern parameters are immutable (TW: mutable_vars starts empty per
/// invocation) — assigning to a param is rejected by check AND compile.
#[test]
fn n264_param_assignment_rejected() {
    let source = r#"
pattern Shadow(name: String) -> String {
  name = "other"
  return name
}
flow Main { input: String = "x" -> Shadow -> output }
"#;
    let chk = metalogos::check_program(source).unwrap();
    assert!(
        !chk.is_ok()
            && chk.errors.iter().any(|e| e
                .message
                .contains("cannot assign to immutable variable: name")),
        "param must be immutable for check (TW parity), got: {:?}",
        chk.errors
    );
    let err = metalogos::compile_program(source).unwrap_err();
    assert!(
        err.contains("cannot assign to immutable variable: name"),
        "param must be immutable for compile too, got: {}",
        err
    );
}

/// Route bodies get the same enforcement on both the static pass and the
/// VM compile path (TW serves route bodies through eval_statements, so the
/// contract applies there identically).
#[test]
fn n264_route_body_non_mut_assign_rejected() {
    let source = r#"
mlogserver {
  port: 8080
  route "/inc" method=GET {
    let n = 1.0
    n = n + 1.0
    respond(to_string(n))
  }
}
"#;
    let chk = metalogos::check_program(source).unwrap();
    assert!(
        !chk.is_ok()
            && chk
                .errors
                .iter()
                .any(|e| e.message.contains("cannot assign to immutable variable: n")),
        "route-body non-mut assign must fail check, got: {:?}",
        chk.errors
    );

    // VM compile path (server compiles routes at startup).
    let decls = metalogos::parser::parse(source).unwrap();
    let mut route = None;
    for decl in &decls {
        if let metalogos::ast::Declaration::MlogServer(srv) = decl {
            route = Some(srv.routes[0].clone());
        }
    }
    let route = route.expect("mlogserver declaration must parse");
    let comp = metalogos::compiler::Compiler::new();
    let err = comp
        .compile_routes(std::slice::from_ref(&route))
        .unwrap_err();
    assert!(
        err.contains("cannot assign to immutable variable: n"),
        "route compile must be loud, got: {}",
        err
    );
}

/// Honest residual (documented in the PR): PRE-№264 .mbc artifacts encode
/// assignments as plain `StoreLocal` — byte-identical to a second `let`
/// binding, so a sound VM-side detection is impossible without breaking
/// legitimate legacy bytecode (double-let reuses the same slot by design).
/// This test pins the compat decision: plain StoreLocal is NEVER rejected.
#[test]
fn n264_legacy_storelocal_shape_still_runs() {
    // Any valid program serves as the Program shell for execute_code.
    let program = metalogos::compile_program("entity g: Float = 0.0").unwrap();
    let mut vm = metalogos::vm::Vm::new();
    let mut stack = vec![Value::String("dummy".into())]; // param slot 0 (bp = 0)
    let mut call_stack = Vec::new();
    let code = vec![
        Instruction::Const(Value::Float(1.0)),
        Instruction::StoreLocal(1), // let x = 1.0 (binding)
        Instruction::Const(Value::Float(2.0)),
        Instruction::StoreLocal(1), // pre-№264 assign encoding — indistinguishable
        Instruction::LoadLocal(1),
        Instruction::Return,
    ];
    let v = vm
        .execute_code(&code, &mut stack, &mut call_stack, &program)
        .unwrap();
    assert!(
        matches!(v, Value::Float(f) if f == 2.0),
        "legacy StoreLocal shape must stay allowed, got {:?}",
        v
    );
}

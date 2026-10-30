// Naryad №780 (issue #1491, wave 62, P0) — the audit-23.10 A1: the
// runtime label gates (LabelJoin/SinkCheck — the №325/ADR-0156 runtime
// twin) execute in the PATTERN/ROUTE bodies too. The inner loop
// (execute_code_inner) previously dropped both into the wildcard tail:
// the label env stayed unseeded in the bodies and the runtime sink
// clearance verdict (the `[SINK_CLEARANCE_RUNTIME]` class, the №392
// deny path included) never fired in the server code — the serve lanes
// compile route bodies into exactly that loop.
//
// The n328 pins cover the MAIN loop; this file covers the INNER loop:
// the compiler-level shape (the pattern body), the bytecode-direct
// shape (stable), the main-only loud skew, and the route-body lane.

use metalogos::bytecode::{Instruction, Program};
use metalogos::interpreter::Value;
use metalogos::vm::Vm;

fn empty_program() -> Program {
    Program {
        globals: vec![],
        patterns: std::sync::Arc::new(vec![]),
        learnables: vec![],
        rules: vec![],
        skill_indices: vec![],
        reflex_decls: vec![],
        reflex_seq_decls: vec![],
        reflex_gen_decls: vec![],
        vision_decls: vec![],
        origin_decls: vec![],
        deny_handlers: vec![],
        db_url: None,
        db_url_env: None,
        memory_persist_path: None,
        conversation_config: Default::default(),
        schema_ddl: vec![],
        main_code: vec![],
        collections_loaded: false,
        shared_cache: metalogos::bytecode::ProgramSharedCache::new(),
        capability_permits: Vec::new(), // №845: the standard posture (empty roster)
    }
}

fn label_join(dst: &str, src: &str) -> Instruction {
    Instruction::LabelJoin(Box::new(metalogos::bytecode::LabelJoinData {
        dst: dst.to_string(),
        src: src.to_string(),
    }))
}

fn sink_check(fn_name: &str, arg: &str) -> Instruction {
    Instruction::SinkCheck(Box::new(metalogos::bytecode::SinkCheckData {
        fn_name: fn_name.to_string(),
        arg: arg.to_string(),
        line: 3,
        arg_index: 0,
        deny: None,
        slot: None,
    }))
}

// ── The compiler-level shape: the pattern body carries the gates ─────

#[test]
fn n780_compiler_emits_label_gates_into_pattern_bodies() {
    // Anti-drift: a source-backed let of a Source builtin seeds the
    // LabelJoin and a sink call arms the SinkCheck INSIDE the pattern
    // body (compiler.rs emit sites). If the compiler stops emitting
    // them there, this test fails before the VM ever runs.
    let source = r#"
pattern leak(_: String) -> Any {
  let k = env_or("N780_ABSENT", "placeholder")
  print(k)
  return 1
}

flow Main {
  input: String = "" -> leak -> output
}
"#;
    let declarations = metalogos::parser::parse(source.trim()).expect("parse");
    let mut comp = metalogos::compiler::Compiler::with_std_root(std::path::PathBuf::from("."));
    let program = comp.compile(declarations).expect("compile");
    let body = &program.patterns[0].code;
    assert!(
        body.iter().any(|c| matches!(c, Instruction::LabelJoin(_))),
        "the pattern body must carry a LabelJoin (the source-backed let): {body:?}"
    );
    assert!(
        body.iter().any(|c| matches!(c, Instruction::SinkCheck(_))),
        "the pattern body must carry an armed SinkCheck (the print call): {body:?}"
    );
}

// ── THE RED-TO-GREEN pin: the SinkCheck verdict fires in the body ────

#[test]
fn n780_sink_check_in_pattern_body_rejects_private_at_runtime() {
    std::env::set_var("N780_TEST_SECRET", "s3cret-value");
    let source = r#"
pattern leak(_: String) -> Any {
  let k = env("N780_TEST_SECRET")
  print(k)
  return 1
}

flow Main {
  input: String = "" -> leak -> output
}
"#;
    let declarations = metalogos::parser::parse(source.trim()).expect("parse");
    let mut comp = metalogos::compiler::Compiler::with_std_root(std::path::PathBuf::from("."));
    let program = comp.compile(declarations).expect("compile");
    let result = Vm::new().run(program);
    let err = result.expect_err(
        "the private secret routed to a sink inside the pattern body must be refused loudly (the audit-23.10 A1)",
    );
    assert!(
        err.contains("SINK_CLEARANCE_RUNTIME"),
        "the runtime verdict is loud and distinct: {err}"
    );
    assert!(
        err.contains("private"),
        "the verdict names the label it refused: {err}"
    );
}

#[test]
fn n780_label_gates_in_pattern_body_bytecode_direct() {
    // The stable bytecode pin: the same shape WITHOUT the compiler and
    // the environment — LabelJoin seeds private from @env, the
    // SinkCheck on print must refuse; before №780 both instructions
    // drowned in the inner loop's wildcard tail and the body executed
    // silently.
    let mut program = empty_program();
    program.patterns = std::sync::Arc::new(vec![metalogos::bytecode::CompiledFn {
        name: "leak".to_string(),
        param_count: 0,
        param_types: vec![],
        code: vec![
            label_join("k", "@env"),
            sink_check("print", "k"),
            Instruction::Const(Box::new(Value::Float(0.0))),
        ],
        is_pure: false,
    }]);
    program.main_code = vec![Instruction::CallPattern(0, 0)];
    let result = Vm::new().run(program);
    let err = result.expect_err("the body-level SinkCheck must refuse the private label");
    assert!(err.contains("SINK_CLEARANCE_RUNTIME"), "{err}");
}

// ── The main-only set is loud in a body (the compiler/VM skew) ───────

#[test]
fn n780_main_only_instruction_in_body_is_loud() {
    let mut program = empty_program();
    program.globals = vec!["g".to_string()];
    program.patterns = std::sync::Arc::new(vec![metalogos::bytecode::CompiledFn {
        name: "weird".to_string(),
        param_count: 0,
        param_types: vec![],
        code: vec![
            Instruction::StoreGlobal(0),
            Instruction::Const(Box::new(Value::Float(0.0))),
        ],
        is_pure: false,
    }]);
    program.main_code = vec![Instruction::CallPattern(0, 0)];
    let result = Vm::new().run(program);
    let err = result.expect_err("a main-only instruction in a body is a compiler/VM skew");
    assert!(
        err.contains("is main-only"),
        "the skew error names the class: {err}"
    );
    assert!(
        err.contains("StoreGlobal"),
        "the skew error names the instruction: {err}"
    );
}

// ── The main loop must stay loud (the n328 regression, both loops) ───

#[test]
fn n780_main_loop_sink_check_still_rejects() {
    let mut program = empty_program();
    program.main_code = vec![
        label_join("k", "@env"),
        sink_check("write_file", "k"),
        Instruction::Const(Box::new(Value::Float(0.0))),
    ];
    let err = Vm::new().run(program).expect_err("main still loud");
    assert!(err.contains("SINK_CLEARANCE_RUNTIME"), "{err}");
}

// ── The route-body lane (the serve path) ─────────────────────────────

#[test]
fn n780_route_body_sink_check_rejects_private_at_runtime() {
    std::env::set_var("N780_TEST_SECRET", "s3cret-value");
    let source = r#"
mlogserver {
  port: 8095
  route "/leak" method=GET {
    let k = env("N780_TEST_SECRET")
    print(k)
    respond("done")
  }
}
"#;
    let declarations = metalogos::parser::parse(source.trim()).expect("parse");
    let mut comp = metalogos::compiler::Compiler::with_std_root(std::path::PathBuf::from("."));
    let program = comp.compile(declarations.clone()).expect("compile");
    let config = declarations
        .iter()
        .find_map(|d| match d {
            metalogos::ast::Declaration::MlogServer(s) => Some(s.clone()),
            _ => None,
        })
        .expect("mlogserver block");
    let compiled_routes = comp.compile_routes(&config.routes).expect("compile_routes");
    let mut vm = Vm::new();
    vm.load_program(&program).expect("load_program");
    let result = vm.execute_route_code(&compiled_routes[0], &program);
    let err = result.expect_err(
        "the route body runs through the SAME inner loop — the private label at a sink must refuse (the audit-23.10 A1 serve lane)",
    );
    assert!(err.contains("SINK_CLEARANCE_RUNTIME"), "{err}");
}

// ── The typed materialization canon (№355/№691) survives the twin ────

#[test]
fn n780_world_state_in_body_keeps_the_typed_code() {
    // The runtime twin fires BEFORE the shared builtin leg now — the
    // WorldState canon keeps its typed code (`WORLD_STATE_PRIVATE`,
    // the conformance pair sst_001): the compiler tracks the argument
    // slot, the VM reads the value and specializes the verdict.
    let mut program = empty_program();
    program.patterns = std::sync::Arc::new(vec![metalogos::bytecode::CompiledFn {
        name: "snap".to_string(),
        param_count: 0,
        param_types: vec![],
        code: vec![
            label_join("w", "@world_state"),
            Instruction::Const(Box::new(Value::WorldState(Default::default()))),
            Instruction::StoreLocal(0),
            Instruction::SinkCheck(Box::new(metalogos::bytecode::SinkCheckData {
                fn_name: "print".to_string(),
                arg: "w".to_string(),
                line: 3,
                arg_index: 0,
                deny: None,
                slot: Some(0),
            })),
            Instruction::Const(Box::new(Value::Float(0.0))),
        ],
        is_pure: false,
    }]);
    program.main_code = vec![Instruction::CallPattern(0, 0)];
    let err = Vm::new()
        .run(program)
        .expect_err("the WorldState materialization refuses");
    assert!(
        err.contains("WORLD_STATE_PRIVATE"),
        "the typed canon survives the earlier twin: {err}"
    );
    assert!(
        !err.contains("SINK_CLEARANCE_RUNTIME"),
        "the general verdict must not shadow the typed one: {err}"
    );
}

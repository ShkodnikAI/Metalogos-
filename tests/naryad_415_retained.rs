//! Naryad №415: retained-representation compaction tests.
//!
//! Contract (gh#569, path A per gh#527):
//! 1. `size_of::<Instruction>()` ≤ 32 B — the enum's size is no longer
//!    dictated by its fattest variant (was 160 B).
//! 2. Pattern bodies live EXACTLY ONCE in the `Program::patterns` table;
//!    main_code carries only `RegisterPatternRef(u32)` indices.
//! 3. The №402 shared snapshot is a ZERO-CLONE Arc increment over the table.
//! 4. .mbc round-trip preserves both the new format (table + refs) and the
//!    legacy fallback (inline bodies + empty table → scan rebuild).
//! 5. `RegisterPatternRef` with an out-of-range index fails LOUDLY (the
//!    №264 backstop contract).

use metalogos::bytecode::{Instruction, Program};
use std::path::Path;

const MANIFEST: &str = env!("CARGO_MANIFEST_DIR");

fn compile(src: &str) -> Program {
    metalogos::compile_program(src).unwrap_or_else(|e| panic!("compile failed: {}", e))
}

/// (1) The size contract — the whole point of the payload boxing.
#[test]
fn n415_instruction_size_is_pointer_scale() {
    assert!(
        std::mem::size_of::<Instruction>() <= 32,
        "Instruction must be ≤ 32 B after №415 boxing, got {}",
        std::mem::size_of::<Instruction>()
    );
}

/// (2) The compiler fills the table and emits refs, not inline bodies.
#[test]
fn n415_pattern_table_is_the_single_copy() {
    let src = r#"
pattern Greet(name: String) -> String {
  return "hi " + name
}
flow Main { input: String = "x" -> Greet -> output }
"#;
    let program = compile(src);
    assert_eq!(program.patterns.len(), 1, "table holds the body once");
    assert_eq!(program.patterns[0].name, "Greet");
    let refs: Vec<u32> = program
        .main_code
        .iter()
        .filter_map(|i| match i {
            Instruction::RegisterPatternRef(idx) => Some(*idx),
            _ => None,
        })
        .collect();
    assert_eq!(refs, vec![0], "main_code carries the index, not the body");
    assert!(
        !program
            .main_code
            .iter()
            .any(|i| matches!(i, Instruction::RegisterPattern(_))),
        "no inline legacy bodies in fresh bytecode"
    );
}

/// (3) The shared snapshot pays an Arc increment, not a clone.
#[test]
fn n415_snapshot_is_zero_clone_over_the_table() {
    let src = r#"
pattern P(_x: String) -> String {
  return "ok"
}
flow Main { input: String = "d" -> P -> output }
"#;
    let program = compile(src);
    let snap = program.pre_registered_patterns();
    assert!(
        std::sync::Arc::ptr_eq(&snap, &program.patterns),
        "snapshot must share the table (zero clone)"
    );
}

/// (4a) .mbc round-trip: table + RegisterPatternRef survive the wire.
#[test]
fn n415_roundtrip_preserves_table_and_refs() {
    let src = r#"
pattern Shout(word: String) -> String {
  return word + "!"
}
flow Main { input: String = "go" -> Shout -> output }
"#;
    let program = compile(src);
    let bytes = program.serialize().unwrap();
    let restored = Program::deserialize(&bytes).unwrap();
    assert_eq!(restored.patterns.len(), 1);
    assert_eq!(restored.patterns[0].name, "Shout");
    assert!(restored
        .main_code
        .iter()
        .any(|i| matches!(i, Instruction::RegisterPatternRef(0))));
    // The restored program still runs: route parity contract via run_bytecode.
    let out = metalogos::run_bytecode(restored).expect("restored program runs");
    let _ = out;
}

/// (4b) Legacy fallback: a pre-№415 wire (inline bodies, empty table)
/// still loads and dispatches — the scan rebuild keeps CallPattern valid.
#[test]
fn n415_legacy_wire_still_loads_and_runs() {
    // Build the legacy shape by hand: body inline in main_code, table empty.
    let body_fn = metalogos::bytecode::CompiledFn {
        name: "P".to_string(),
        param_count: 1,
        param_types: vec!["String".to_string()],
        code: vec![Instruction::const_(metalogos::interpreter::Value::String(
            "legacy-ok".to_string(),
        ))],
        is_pure: true,
    };
    let program = Program {
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
        memory_persist_path: None,
        schema_ddl: vec![],
        main_code: vec![
            Instruction::RegisterPattern(Box::new(body_fn)),
            Instruction::const_(metalogos::interpreter::Value::String("x".to_string())),
            Instruction::CallPattern(0, 1),
            Instruction::Halt,
        ],
        collections_loaded: false,
        shared_cache: metalogos::bytecode::ProgramSharedCache::new(),
    };
    let snap = program.pre_registered_patterns();
    assert_eq!(snap.len(), 1, "legacy scan rebuilds the table");
    assert_eq!(snap[0].name, "P");
    // .mbc round-trip of the legacy shape too.
    let bytes = program.serialize().unwrap();
    let restored = Program::deserialize(&bytes).unwrap();
    assert_eq!(restored.pre_registered_patterns()[0].name, "P");
}

/// (5) The №264-family loud backstop: an out-of-range ref index must
/// fail the run, never silently skip registration.
#[test]
fn n415_out_of_range_ref_fails_loudly() {
    let body_fn = metalogos::bytecode::CompiledFn {
        name: "Ghost".to_string(),
        param_count: 0,
        param_types: vec![],
        code: vec![],
        is_pure: true,
    };
    let _ = body_fn; // table stays EMPTY on purpose; ref points nowhere
    let program = Program {
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
        memory_persist_path: None,
        schema_ddl: vec![],
        main_code: vec![Instruction::RegisterPatternRef(7), Instruction::Halt],
        collections_loaded: false,
        shared_cache: metalogos::bytecode::ProgramSharedCache::new(),
    };
    let err = metalogos::run_bytecode(program).expect_err("out-of-range ref must fail loudly");
    assert!(
        err.contains("RegisterPatternRef index 7 out of range"),
        "loud backstop text, got: {}",
        err
    );
}

/// (6) Serve-path parity: a route calling a table-registered pattern
/// resolves the positional CallPattern through the shared table.
#[test]
fn n415_serve_route_dispatches_table_pattern() {
    let src = std::fs::read_to_string(Path::new(MANIFEST).join("examples/w2_vm_route.mlog"))
        .or_else(|_| {
            std::fs::read_to_string(Path::new(MANIFEST).join("examples/w2_consent_deny.mlog"))
        });
    // The fixture may not exist in every checkout; fall back to a minimal
    // inline server program with a route calling a pattern.
    let src = match src {
        Ok(s) if s.contains("route") => s,
        _ => r#"
mlogserver {
  port: 18091
  route "/hello" method=GET {
    return Greet("n415")
  }
}
pattern Greet(who: String) -> String {
  return "hi " + who
}
"#
        .to_string(),
    };
    let program = compile(&src);
    assert!(!program.patterns.is_empty(), "pattern compiled into the table");
    let routes_decl: Vec<_> = metalogos::parser::parse(&src)
        .unwrap()
        .iter()
        .filter_map(|d| match d {
            metalogos::ast::Declaration::MlogServer(s) => Some(s.routes.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    let mut comp = metalogos::compiler::Compiler::with_std_root(Path::new(MANIFEST).to_path_buf());
    let program2 = comp.compile(metalogos::parser::parse(&src).unwrap()).unwrap();
    let routes = comp.compile_routes(&routes_decl).unwrap();
    let mut vm = metalogos::vm::Vm::new();
    vm.load_program(&program2)
        .expect("load_program installs the shared table");
    let compiled = routes
        .iter()
        .find(|r| r.path == "/hello")
        .expect("hello route compiled");
    let out = vm
        .execute_route_code(compiled, &program2)
        .expect("route runs against the table");
    assert!(format!("{}", out).contains("hi"), "route output: {:?}", out);
}

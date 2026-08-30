// Наряд №146: bincode 1.3 -> 2.0.1 migration contracts.
//
// bincode 3.0.0 is not a real release — the crate is a deliberate
// compile_error!() tombstone published by the maintainer after
// ceasing development (see README of that release on crates.io).
// 2.0.1 is the last real stable version; config::legacy() keeps the
// wire format byte-compatible with what 1.x produced.

use metalogos::bytecode::{Instruction, Program};

fn minimal_program() -> Program {
    Program {
        globals: vec!["x".to_string()],
        patterns: vec![],
        learnables: vec![],
        rules: vec![],
        skill_indices: vec![],
        db_url: None,
        schema_ddl: vec![],
        main_code: vec![
            Instruction::Const(metalogos::interpreter::Value::String("hello".to_string())),
            Instruction::Halt,
        ],
        collections_loaded: false,
    }
}

// ── C1: round-trip ──────────────────────────────────────────────

#[test]
fn round_trip_minimal_program() {
    let program = minimal_program();
    let bytes = program.serialize().expect("serialize should succeed");
    let restored = Program::deserialize(&bytes).expect("deserialize should succeed");
    assert_eq!(restored.globals, program.globals);
    assert_eq!(restored.main_code.len(), program.main_code.len());
}

#[test]
fn round_trip_with_db_url_and_schema() {
    let mut program = minimal_program();
    program.db_url = Some("sqlite::memory:".to_string());
    program.schema_ddl = vec!["CREATE TABLE t (id INTEGER)".to_string()];
    let bytes = program.serialize().expect("serialize should succeed");
    let restored = Program::deserialize(&bytes).expect("deserialize should succeed");
    assert_eq!(restored.db_url, program.db_url);
    assert_eq!(restored.schema_ddl, program.schema_ddl);
}

// ── C2: oversized input rejected, not panicked ──────────────────

#[test]
fn reject_oversized_input() {
    // 51 MB of zero bytes — well past the 50 MB limit, well before
    // bincode would even attempt to decode it as a valid header.
    let oversized = vec![0u8; 51 * 1024 * 1024];
    let result = Program::deserialize(&oversized);
    assert!(result.is_err(), "51MB input must be rejected");
    assert!(
        result.unwrap_err().contains("exceeds"),
        "error should mention the size limit, not a generic decode failure"
    );
}

#[test]
fn reject_at_exact_boundary_does_not_panic() {
    // Exactly at the limit — must not panic either way (may succeed
    // or fail decode, but must not crash the process).
    let boundary = vec![0u8; 50 * 1024 * 1024];
    let _ = Program::deserialize(&boundary);
}

#[test]
fn small_garbage_input_rejected_gracefully() {
    let garbage = vec![0xFFu8; 16];
    let result = Program::deserialize(&garbage);
    assert!(
        result.is_err(),
        "garbage bytes must not deserialize successfully"
    );
}

// ── C3: end-to-end compile -> serialize -> deserialize -> run ───

#[test]
fn e2e_serialize_deserialize_preserves_instructions() {
    let program = minimal_program();
    let bytes = program.serialize().unwrap();
    let restored = Program::deserialize(&bytes).unwrap();
    match (&program.main_code[0], &restored.main_code[0]) {
        (Instruction::Const(a), Instruction::Const(b)) => assert_eq!(a, b),
        _ => panic!("first instruction shape changed across round-trip"),
    }
}

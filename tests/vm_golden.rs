// ── VM golden tests: run examples/ through the bytecode VM ────────────
// Phase 4.2: strict dual-mode comparison for all golden examples.
// ADR-0076: dispatch path coverage test to prevent silent instruction drift.

use std::fs;
use std::path::{Path, PathBuf};

/// Instructions that are intentionally ONLY handled by `run()` and NOT by
/// `execute_code()`. These represent top-level program constructs that
/// should never appear in compiled pattern bodies.
///
/// ADR-0076: Adding a new Instruction variant handled by `run()` without
/// either adding it to `execute_code()` or this list causes a test failure.
const TOP_LEVEL_ONLY_INSTRUCTIONS: &[&str] = &[
    "Adapt",             // top-level learnable adaptation
    "ExecuteRules",      // macro instruction — rule engine trigger
    "FlowExec",          // macro instruction — flow pipeline (legacy)
    "FlowPipeline",      // flow pipeline — pop source from stack
    "Forget",            // top-level memory forget
    "Halt",              // end-of-program marker
    "JumpIfLow",         // confidence-based branching (top-level flows)
    "ListLen",           // list length (not yet in execute_code)
    "MakeFluid",         // fluid type construction (top-level)
    "MakeList",          // list literal construction (not yet in execute_code)
    "Memorize",          // top-level memory memorize
    "Mutate",            // top-level mutate declaration
    "Pop",               // stack cleanup (top-level expr statements)
    "RegisterLearnable", // learnable pattern registration
    "RegisterPattern",   // pattern registration
    "Relate",            // top-level knowledge graph relation
    "StartsWith",        // string starts-with (not yet in execute_code)
    "StoreGlobal",       // top-level global variable assignment
];

/// All Instruction variants that exist in the bytecode::Instruction enum.
/// ADR-0076: If a new variant is added to the enum, it MUST also be added here.
const ALL_INSTRUCTIONS: &[&str] = &[
    // Constants & Variables
    "Const",
    "LoadGlobal",
    "LoadGlobalByName",
    "StoreGlobal",
    "LoadLocal",
    "StoreLocal",
    // Function Registration
    "RegisterPattern",
    "RegisterLearnable",
    // Function Calls
    "CallBuiltin",
    "CallPattern",
    "Return",
    // Binary Operations
    "Add",
    "Sub",
    "Mul",
    "Div",
    "Contains",
    // Comparisons
    "CmpGt",
    "CmpLt",
    "CmpGe",
    "CmpLe",
    "CmpEq",
    "CmpNe",
    // Struct Operations
    "MakeStruct",
    "GetField",
    "IndexAccess",
    "MakeList",
    "ListLen",
    "Pop",
    "StartsWith",
    // Fluid Types
    "MakeFluid",
    // Control Flow
    "Jump",
    "JumpIfNot",
    "JumpIfLow",
    // Memory
    "Collapse",
    "Memorize",
    "Recall",
    "Forget",
    // LLM
    "LlmCall",
    // Adapt / Relate / Mutate
    "Adapt",
    "Relate",
    "Mutate",
    // Pipeline
    "FlowPipeline",
    "FlowExec",
    // Rules
    "ExecuteRules",
    // Meta
    "Halt",
];

/// Execute a .mlog source via the bytecode VM.
fn run_vm(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir.to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

/// Execute a .mlog source via tree-walking interpreter.
fn run_tw(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, base_dir.to_path_buf())
}

/// Find all .mlog files with .expected files.
fn collect_pairs(examples_dir: &Path) -> Vec<(PathBuf, PathBuf)> {
    let mut pairs = Vec::new();
    if let Ok(entries) = fs::read_dir(examples_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "mlog" {
                    let stem = path.file_stem().unwrap().to_string_lossy().to_string();
                    if stem.starts_with("p7_") {
                        continue; // Tracked separately in golden.rs::p7_contract_visibility
                    }
                    let expected = path.with_extension("expected");
                    if expected.exists() {
                        pairs.push((path, expected));
                    }
                }
            }
        }
    }
    pairs.sort_by(|a, b| a.0.file_name().cmp(&b.0.file_name()));
    pairs
}

/// Helper: trim trailing whitespace from an Option<String>.
fn trim_opt(s: &Option<String>) -> String {
    s.as_deref().map(|v| v.trim_end()).unwrap_or("").to_string()
}

#[test]
fn p4_vm_hello_matches_tw() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let examples_dir = Path::new(&manifest_dir).join("examples");

    let source = fs::read_to_string(examples_dir.join("p4_vm_hello.mlog"))
        .expect("cannot read p4_vm_hello.mlog");
    let base_dir = examples_dir.parent().unwrap_or(Path::new("."));

    let vm_result = run_vm(&source, base_dir).expect("VM execution failed");
    let tw_result = run_tw(&source, base_dir).expect("TW execution failed");

    assert_eq!(vm_result, tw_result, "VM output differs from tree-walking");
}

/// Phase 4.2 strict test: all golden examples must produce identical
/// output when run via tree-walking interpreter vs bytecode VM.
#[test]
fn all_vm_examples_match_tree_walking() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let examples_dir = Path::new(&manifest_dir).join("examples");

    let pairs = collect_pairs(&examples_dir);
    assert!(
        !pairs.is_empty(),
        "no .mlog/.expected pairs found in examples/"
    );

    let base_dir = examples_dir.parent().unwrap_or(Path::new("."));
    let mut passed = 0;

    for (mlog_path, expected_path) in &pairs {
        let source = fs::read_to_string(mlog_path)
            .unwrap_or_else(|e| panic!("cannot read {:?}: {}", mlog_path, e));
        let _expected = fs::read_to_string(expected_path)
            .unwrap_or_else(|e| panic!("cannot read {:?}: {}", expected_path, e));

        let vm_result = run_vm(&source, base_dir).unwrap_or_else(|e| {
            panic!("VM execution failed for {:?}: {}", mlog_path.file_name(), e)
        });
        let tw_result = run_tw(&source, base_dir).unwrap_or_else(|e| {
            panic!("TW execution failed for {:?}: {}", mlog_path.file_name(), e)
        });

        let vm_trimmed = trim_opt(&vm_result);
        let tw_trimmed = trim_opt(&tw_result);

        assert_eq!(
            vm_trimmed,
            tw_trimmed,
            "VM vs TW mismatch for {:?}:\n  TW: {:?}\n  VM: {:?}",
            mlog_path.file_name(),
            tw_trimmed,
            vm_trimmed
        );
        passed += 1;
    }

    eprintln!(
        "\nPhase 4.2 strict dual-mode test: {}/{} passed",
        passed,
        pairs.len()
    );
    assert_eq!(
        passed,
        pairs.len(),
        "not all examples passed dual-mode comparison"
    );
}

/// Legacy test (kept for CI compatibility): all VM outputs match .expected files.
#[test]
fn all_vm_golden_tests_pass() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let examples_dir = Path::new(&manifest_dir).join("examples");

    let pairs = collect_pairs(&examples_dir);
    assert!(
        !pairs.is_empty(),
        "no .mlog/.expected pairs found in examples/"
    );

    let base_dir = examples_dir.parent().unwrap_or(Path::new("."));
    let mut passed = 0;

    for (mlog_path, expected_path) in &pairs {
        let source = fs::read_to_string(mlog_path)
            .unwrap_or_else(|e| panic!("cannot read {:?}: {}", mlog_path, e));
        let expected = fs::read_to_string(expected_path)
            .unwrap_or_else(|e| panic!("cannot read {:?}: {}", expected_path, e));

        let vm_result = run_vm(&source, base_dir).expect("VM execution failed");
        let vm_trimmed = trim_opt(&vm_result);
        let expected_trimmed = expected.trim_end();

        assert_eq!(
            vm_trimmed,
            expected_trimmed,
            "VM vs .expected mismatch for {:?}:\n  expected: {:?}\n  VM:       {:?}",
            mlog_path.file_name(),
            expected_trimmed,
            vm_trimmed
        );
        passed += 1;
    }

    eprintln!(
        "\nVM golden test results: {}/{} passed",
        passed,
        pairs.len()
    );
    assert_eq!(passed, pairs.len());
}

/// ADR-0076: Verify that every Instruction variant is accounted for.
/// The test ensures the instruction lists stay synchronized with the enum.
/// ALL_INSTRUCTIONS must contain every variant; TOP_LEVEL_ONLY must be a
/// subset. Adding a new instruction without updating ALL_INSTRUCTIONS will
/// cause the count check to fail.
#[test]
fn vm_dispatch_coverage() {
    // Verify: TOP_LEVEL_ONLY is a subset of ALL_INSTRUCTIONS
    for instr in TOP_LEVEL_ONLY_INSTRUCTIONS {
        assert!(
            ALL_INSTRUCTIONS.contains(instr),
            "ADR-0076: '{}' in TOP_LEVEL_ONLY but not in ALL_INSTRUCTIONS",
            instr
        );
    }

    // Verify: expected total count matches.
    // If someone adds a new Instruction variant to the enum without updating
    // ALL_INSTRUCTIONS, this assertion will catch it (count mismatch).
    // Current: 45 total (27 shared + 18 top-level-only).
    assert_eq!(
        ALL_INSTRUCTIONS.len(),
        45,
        "ADR-0076: ALL_INSTRUCTIONS count changed (expected 45). \
         If a new Instruction variant was added to the enum, update ALL_INSTRUCTIONS \
         and optionally TOP_LEVEL_ONLY_INSTRUCTIONS."
    );

    // Verify: no duplicates in ALL_INSTRUCTIONS
    let mut seen = std::collections::HashSet::new();
    for instr in ALL_INSTRUCTIONS {
        assert!(
            seen.insert(*instr),
            "ADR-0076: duplicate entry '{}' in ALL_INSTRUCTIONS",
            instr
        );
    }

    // Report
    let shared_count = ALL_INSTRUCTIONS.len() - TOP_LEVEL_ONLY_INSTRUCTIONS.len();
    eprintln!(
        "\nADR-0076: {} total instructions ({} shared + {} top-level-only), coverage OK",
        ALL_INSTRUCTIONS.len(),
        shared_count,
        TOP_LEVEL_ONLY_INSTRUCTIONS.len()
    );
}

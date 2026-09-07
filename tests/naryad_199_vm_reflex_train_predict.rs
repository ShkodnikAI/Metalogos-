// ── tests/naryad_199_vm_reflex_train_predict.rs ───────────────────
// Наряд №199, Contract 2: direct VM-backend run of reflex_train_predict.
//
// Compiles `examples/reflex_train_predict.mlog` to bytecode and runs it
// through the VM (not the interpreter). Verifies:
//   1. The VM does NOT produce the "VM backend does not yet support Reflex"
//      stub error.
//   2. The VM successfully trains the model (reflex_train runs).
//   3. The VM successfully predicts (reflex_predict runs).
//   4. The output matches the expected golden output (same as interpreter).

use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run_vm(source: &str, base_dir: &std::path::Path) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir.to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

#[test]
fn naryad_199_vm_reflex_train_predict_matches_expected() {
    let project_dir = manifest_dir();
    let mlog_path = project_dir.join("examples/reflex_train_predict.mlog");
    let expected_path = project_dir.join("examples/reflex_train_predict.expected");

    let source = std::fs::read_to_string(&mlog_path)
        .unwrap_or_else(|e| panic!("cannot read {:?}: {}", mlog_path, e));
    let expected = std::fs::read_to_string(&expected_path)
        .unwrap_or_else(|e| panic!("cannot read {:?}: {}", expected_path, e));

    let vm_result = run_vm(&source, &project_dir);

    match &vm_result {
        Ok(output) => {
            let actual = output.as_deref().unwrap_or("").trim();
            let expected_trimmed = expected.trim();

            assert!(
                !actual.is_empty(),
                "VM produced empty output for reflex_train_predict.mlog"
            );

            // Must NOT contain the stub error message.
            assert!(
                !actual.contains("VM backend does not yet support Reflex"),
                "VM produced the stub error — reflex_train/predict was not intercepted:\n{}",
                actual
            );

            // Must match the expected golden output.
            assert_eq!(
                actual, expected_trimmed,
                "VM output does not match expected golden output.\n\
                 expected: {}\n  actual: {}",
                expected_trimmed, actual
            );
        }
        Err(e) => {
            panic!(
                "VM run failed for reflex_train_predict.mlog:\n  error: {}\n\n\
                 This means the VM-side Reflex intercept is not working.\n\
                 The error should NOT be 'VM backend does not yet support Reflex'.",
                e
            );
        }
    }
}

#[test]
fn naryad_199_vm_reflex_train_does_not_stub() {
    // Focused test: verify the VM does NOT produce the stub error for
    // reflex_train. This is a subset of the full test above — if the full
    // test passes, this one is redundant, but it gives a clearer failure
    // message if only the stub-intercept is broken.
    let project_dir = manifest_dir();
    let mlog_path = project_dir.join("examples/reflex_train_predict.mlog");
    let source = std::fs::read_to_string(&mlog_path).unwrap();

    let vm_result = run_vm(&source, &project_dir);

    if let Err(e) = &vm_result {
        assert!(
            !e.contains("VM backend does not yet support Reflex"),
            "VM produced the stub error for reflex_train/predict.\n\
             The call_builtin intercept in src/vm.rs is not working.\n\
             error: {}",
            e
        );
    }
}

#[test]
fn naryad_199_vm_reflex_predict_returns_fluid() {
    // Verify that reflex_predict on the VM returns a Fluid value (the
    // label with highest confidence), not a raw struct or error.
    // The expected output contains "predict([0.1,0.1])=near" — the "near"
    // part comes from the Fluid Display of the prediction result.
    let project_dir = manifest_dir();
    let mlog_path = project_dir.join("examples/reflex_train_predict.mlog");
    let source = std::fs::read_to_string(&mlog_path).unwrap();

    let vm_result = run_vm(&source, &project_dir);
    let output = vm_result.expect("VM run should succeed");
    let actual = output.unwrap_or_default();

    assert!(
        actual.contains("predict([0.1,0.1])=near"),
        "VM output must contain 'predict([0.1,0.1])=near' (Fluid Display of reflex_predict result).\n\
         actual: {}",
        actual
    );
}

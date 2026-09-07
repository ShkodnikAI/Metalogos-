// ── tests/naryad_199_vm_matches_tw_determinism.rs ─────────────────
// Наряд №199, Contract 3: same seed → identical accuracy when training
// through the VM vs through the interpreter (TW).
//
// ADR-0121 explicitly calls determinism mandatory: "the same seed must
// produce the same weights, the same holdout split, and the same final
// accuracy — not assumed, verified." This test verifies that by running
// the same .mlog source through both backends and comparing the output
// byte-for-byte.
//
// The test uses `examples/reflex_train_predict.mlog` (seed: 42, 200 epochs,
// 12 samples). The output contains:
//   accuracy=1 loss=0.07944506667943449 threshold_met=true predict([0.1,0.1])=near
//
// Both backends must produce this exact string. Any difference in the
// `loss` or `accuracy` fields indicates a determinism violation — either
// the weight initialization or the training loop differs between backends.

use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run_tw(source: &str, base_dir: &std::path::Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, base_dir.to_path_buf())
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
fn naryad_199_vm_matches_tw_determinism_reflex_train_predict() {
    let project_dir = manifest_dir();
    let mlog_path = project_dir.join("examples/reflex_train_predict.mlog");
    let source = std::fs::read_to_string(&mlog_path)
        .unwrap_or_else(|e| panic!("cannot read {:?}: {}", mlog_path, e));

    let tw_result = run_tw(&source, &project_dir);
    let vm_result = run_vm(&source, &project_dir);

    let tw_output = tw_result
        .expect("TW (interpreter) run should succeed")
        .unwrap_or_default();
    let vm_output = vm_result
        .expect("VM run should succeed")
        .unwrap_or_default();

    let tw_trimmed = tw_output.trim();
    let vm_trimmed = vm_output.trim();

    assert!(
        !tw_trimmed.is_empty(),
        "TW produced empty output for reflex_train_predict.mlog"
    );
    assert!(
        !vm_trimmed.is_empty(),
        "VM produced empty output for reflex_train_predict.mlog"
    );

    // ADR-0121 determinism: same seed → same output, byte-for-byte.
    assert_eq!(
        tw_trimmed, vm_trimmed,
        "ADR-0121 determinism violation: TW and VM produced different outputs.\n\
         This means the same seed produced different weights, different holdout\n\
         splits, or different final accuracy — one of the backends is not\n\
         deterministic relative to the other.\n\n\
         TW (interpreter): {}\n\
         VM:               {}\n",
        tw_trimmed, vm_trimmed
    );

    // Verify the output contains the expected fields (not just "some" output).
    // The determinism contract is specifically about the loss/accuracy values
    // matching — if they match, the underlying weight init + training + holdout
    // split are all deterministic.
    assert!(
        tw_trimmed.contains("accuracy="),
        "TW output must contain 'accuracy=': {}",
        tw_trimmed
    );
    assert!(
        tw_trimmed.contains("loss="),
        "TW output must contain 'loss=': {}",
        tw_trimmed
    );
    assert!(
        tw_trimmed.contains("threshold_met="),
        "TW output must contain 'threshold_met=': {}",
        tw_trimmed
    );
    assert!(
        tw_trimmed.contains("predict("),
        "TW output must contain 'predict(': {}",
        tw_trimmed
    );
}

#[test]
fn naryad_199_vm_reflex_train_accuracy_is_high() {
    // Verify that the VM-side training achieves high accuracy (the dataset
    // is linearly separable, so accuracy should be 1.0 after 200 epochs).
    // This guards against the VM silently failing to train (e.g., producing
    // accuracy=0 because the weights weren't initialized).
    let project_dir = manifest_dir();
    let mlog_path = project_dir.join("examples/reflex_train_predict.mlog");
    let source = std::fs::read_to_string(&mlog_path).unwrap();

    let vm_result = run_vm(&source, &project_dir);
    let vm_output = vm_result
        .expect("VM run should succeed")
        .unwrap_or_default();

    assert!(
        vm_output.contains("accuracy=1"),
        "VM should achieve accuracy=1 (linearly separable dataset, 200 epochs).\n\
         actual: {}",
        vm_output
    );
    assert!(
        vm_output.contains("threshold_met=true"),
        "VM should report threshold_met=true (accuracy 1.0 >= 0.85 threshold).\n\
         actual: {}",
        vm_output
    );
}

#[test]
fn naryad_199_vm_reflex_predict_label_matches_tw() {
    // Specifically verify that the predict() output label matches between
    // TW and VM. The predict([0.1, 0.1]) call should return "near" (class 0)
    // on both backends — this confirms the trained weights are identical
    // enough to produce the same classification on a new point.
    let project_dir = manifest_dir();
    let mlog_path = project_dir.join("examples/reflex_train_predict.mlog");
    let source = std::fs::read_to_string(&mlog_path).unwrap();

    let tw_output = run_tw(&source, &project_dir)
        .expect("TW run should succeed")
        .unwrap_or_default();
    let vm_output = run_vm(&source, &project_dir)
        .expect("VM run should succeed")
        .unwrap_or_default();

    // Extract the predict(...) portion from both outputs.
    let tw_predict = tw_output.split("predict(").nth(1).unwrap_or("").trim();
    let vm_predict = vm_output.split("predict(").nth(1).unwrap_or("").trim();

    assert_eq!(
        tw_predict, vm_predict,
        "reflex_predict output differs between TW and VM.\n\
         TW predict:  {}\n\
         VM predict:  {}\n\
         This means the trained weights differ enough to produce different\n\
         predictions on the same input point — a determinism violation.",
        tw_predict, vm_predict
    );
}

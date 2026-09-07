// ── tests/naryad_205_vm_distill.rs ─────────────────────────────────
// Наряд №205: ADR-0121 stage 6 — VM distillation parity.
//
// Verifies that the VM backend handles the full TEACHING→DISTILLED→FALLBACK
// state machine, matching the interpreter byte-for-byte. The crosscheck
// already verifies all 3 distill examples pass on both backends — this file
// adds focused tests that give clearer failure messages.

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

fn run_tw(source: &str, base_dir: &std::path::Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, base_dir.to_path_buf())
}

#[test]
fn naryad_205_vm_distill_teaching_matches_tw() {
    let project_dir = manifest_dir();
    let mlog_path = project_dir.join("examples/reflex_distill_teaching.mlog");
    let source = std::fs::read_to_string(&mlog_path)
        .unwrap_or_else(|e| panic!("cannot read {:?}: {}", mlog_path, e));

    let tw = run_tw(&source, &project_dir).expect("TW should succeed");
    let vm = run_vm(&source, &project_dir).expect("VM should succeed");

    let tw_out = tw.unwrap_or_default();
    let tw_trimmed = tw_out.trim();
    let vm_out = vm.unwrap_or_default();
    let vm_trimmed = vm_out.trim();

    assert_eq!(
        tw_trimmed, vm_trimmed,
        "TW and VM output differ for reflex_distill_teaching.mlog.\n\
         TW: {}\n\
         VM: {}",
        tw_trimmed, vm_trimmed
    );
}

#[test]
fn naryad_205_vm_distill_switch_matches_tw() {
    let project_dir = manifest_dir();
    let mlog_path = project_dir.join("examples/reflex_distill_switch.mlog");
    let source = std::fs::read_to_string(&mlog_path)
        .unwrap_or_else(|e| panic!("cannot read {:?}: {}", mlog_path, e));

    let tw = run_tw(&source, &project_dir).expect("TW should succeed");
    let vm = run_vm(&source, &project_dir).expect("VM should succeed");

    let tw_out = tw.unwrap_or_default();
    let tw_trimmed = tw_out.trim();
    let vm_out = vm.unwrap_or_default();
    let vm_trimmed = vm_out.trim();

    assert_eq!(
        tw_trimmed, vm_trimmed,
        "TW and VM output differ for reflex_distill_switch.mlog.\n\
         TW: {}\n\
         VM: {}",
        tw_trimmed, vm_trimmed
    );
}

#[test]
fn naryad_205_vm_distill_fallback_matches_tw() {
    let project_dir = manifest_dir();
    let mlog_path = project_dir.join("examples/reflex_distill_fallback.mlog");
    let source = std::fs::read_to_string(&mlog_path)
        .unwrap_or_else(|e| panic!("cannot read {:?}: {}", mlog_path, e));

    let tw = run_tw(&source, &project_dir).expect("TW should succeed");
    let vm = run_vm(&source, &project_dir).expect("VM should succeed");

    let tw_out = tw.unwrap_or_default();
    let tw_trimmed = tw_out.trim();
    let vm_out = vm.unwrap_or_default();
    let vm_trimmed = vm_out.trim();

    assert_eq!(
        tw_trimmed, vm_trimmed,
        "TW and VM output differ for reflex_distill_fallback.mlog.\n\
         TW: {}\n\
         VM: {}",
        tw_trimmed, vm_trimmed
    );
}

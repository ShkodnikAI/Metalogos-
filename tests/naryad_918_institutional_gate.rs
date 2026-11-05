// ── tests/naryad_918_institutional_gate.rs ─────────────────────────
// №918 (Wave 83 — W-L1; the dispatch gh#1834): the INSTITUTIONAL_READY
// fact machine (ADR-0219 §2) is pinned by THIS test — the gate is not a
// self-asserting script: the test spawns the machine against the live
// tree and refuses the merge when
//   1. `gen_metrics.py --check` is not green (the floors + the blocks),
//   2. the OK line does not carry the INSTITUTIONAL_READY 12/12 verdict
//      (the gate_044 floor — the wave-83 dispatch constant, only-up),
//   3. `gen_metrics.py --tamper-test` is not green (a dead trap is a
//      lying checker — the №841 discipline).
// The №535 blocking_checks.tsv row (check|all|institutional-roster)
// binds to this file by name (n918_institutional_gate) — the cell dies
// when the test is renamed or deleted (the sync gate's drift rule).
// The №475 fs_gate posture: this file reads the docs and spawns the
// generator gate BY DESIGN (the naryad_566 precedent).
#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    // The root crate's manifest dir IS the repo root (the naryad_566
    // precedent — no .parent(): the CI workspace nests the repo one
    // level deeper, and the parent of the manifest dir is NOT the repo).
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set by cargo"),
    )
}

fn run_gen_metrics(args: &[&str]) -> (i32, String) {
    let out = Command::new("python3")
        .arg(repo_root().join("scripts").join("gen_metrics.py"))
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("python3 must be available (the CI image carries it)");
    (
        out.status.code().unwrap_or(-1),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

#[test]
fn institutional_gate_check_is_green_and_counts_12() {
    let (code, out) = run_gen_metrics(&["--check"]);
    assert_eq!(
        code, 0,
        "gen_metrics --check must be green on the live tree:\n{out}"
    );
    assert!(
        out.contains("INSTITUTIONAL_READY 12/12"),
        "the OK line must carry the INSTITUTIONAL_READY 12/12 verdict \
         (the gate_044 floor, only-up):\n{out}"
    );
}

#[test]
fn institutional_gate_double_check_is_deterministic() {
    let first = run_gen_metrics(&["--check"]);
    let second = run_gen_metrics(&["--check"]);
    assert_eq!(first.0, 0, "the first --check must be green:\n{}", first.1);
    assert_eq!(
        second.0, 0,
        "the second --check must be green:\n{}",
        second.1
    );
    assert_eq!(
        first.1, second.1,
        "the double --check must agree byte-for-byte (the determinism \
         contract — a hand-edited GENERATED row diverges the second run)"
    );
}

#[test]
fn institutional_gate_tamper_traps_are_alive() {
    let (code, out) = run_gen_metrics(&["--tamper-test"]);
    assert_eq!(
        code, 0,
        "the tamper traps must all pass on the live tree:\n{out}"
    );
}

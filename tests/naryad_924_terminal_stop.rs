// ── tests/naryad_924_terminal_stop.rs ───────────────────────────────
// №924 (Wave 85 — W-T; the dispatch gh#1853): the TERMINAL_STOP
// aggregate machine (ADR-0221 §2) is pinned by THIS test — the gate is
// not a self-asserting script: the test spawns the machine against the
// live tree and refuses the merge when
//   1. `gen_metrics.py --check` is not green (the floors + the blocks),
//   2. the OK line does not carry the TERMINAL_STOP 13/13 verdict
//      (the gate_046 floor — the wave-85 dispatch constant, only-up),
//   3. `gen_metrics.py --tamper-test` is not green (a dead trap is a
//      lying checker — the №841 discipline),
//   4. the ADR-0221 definition literal (`TERMINAL_STOP :=`) does not
//      stand in the record file (the №923 mutation contract — the
//      deleted literal reds THIS test: the quote-literal is not found),
//   5. the gate_046 roster does not carry the owner floor
//      (goal_terminal_stop_min 13 — the dispatch constant).
// The №535 blocking_checks.tsv row (check|all|terminal-stop) binds to
// this file by name (n924_terminal_stop_gate) — the cell dies when
// the test is renamed or deleted (the sync gate's drift rule).
// The №475 fs_gate posture: this file reads the docs and spawns the
// generator gate BY DESIGN (the naryad_566/918/921 precedent).
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
fn n924_terminal_stop_gate_check_is_green_and_counts_13() {
    let (code, out) = run_gen_metrics(&["--check"]);
    assert_eq!(
        code, 0,
        "gen_metrics --check must be green on the live tree:\n{out}"
    );
    assert!(
        out.contains("TERMINAL_STOP 13/13"),
        "the OK line must carry the TERMINAL_STOP 13/13 verdict \
         (the gate_046 floor, only-up):\n{out}"
    );
}

#[test]
fn n924_terminal_stop_gate_double_check_is_deterministic() {
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
         contract — a synthesized green diverges the second run)"
    );
}

#[test]
fn n924_terminal_stop_gate_tamper_test_is_green() {
    let (code, out) = run_gen_metrics(&["--tamper-test"]);
    assert_eq!(
        code, 0,
        "gen_metrics --tamper-test must be green (the traps 55–58 all \
         spring on the live tree):\n{out}"
    );
    assert!(
        out.contains("TERMINAL_STOP 13/13"),
        "the tamper-test OK line must carry the TERMINAL_STOP 13/13 \
         verdict:\n{out}"
    );
}

#[test]
fn n924_terminal_stop_adr_definition_literal_stands() {
    // The №923 mutation contract: the ADR-0221 definition literal
    // (`TERMINAL_STOP :=`) is machine-pinned in the record file — a
    // deleted literal reds the aggregate (nothing is anchored) AND
    // THIS test (the quote-literal is not found).
    let adr = std::fs::read_to_string(
        repo_root()
            .join("docs")
            .join("adr")
            .join("0221-terminal-stop-1-0.md"),
    )
    .expect("the ADR-0221 record must exist (the №923 record)");
    assert!(
        adr.contains("TERMINAL_STOP :="),
        "the ADR-0221 definition literal (`TERMINAL_STOP :=`) must stand \
         in the record file (the №923 mutation contract)"
    );
}

#[test]
fn n924_terminal_stop_roster_carries_the_owner_floor() {
    let roster = std::fs::read_to_string(
        repo_root()
            .join("scripts")
            .join("ci")
            .join("gate_046_goals.txt"),
    )
    .expect("the gate_046 roster must exist (the №924 floor file)");
    assert!(
        roster.contains("goal_terminal_stop_min 13"),
        "the gate_046 roster must carry the owner floor \
         (goal_terminal_stop_min 13 — the wave-85 dispatch constant, \
         only-up)"
    );
}

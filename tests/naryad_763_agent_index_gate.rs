// ── №763 (gh#1416, Wave 58): the agent-index consistency gate — the pin
// binding for the blocking_checks.tsv cell
// agent-index|all|llms-counters-and-bridge-sync|blocks ─────────────────
//
// The finding (R-01): llms.txt is the FIRST SCREEN for IDE agents and RAG
// pipelines and was covered by NO generator gate — it drifted (409→526
// builtins, 214→253 examples, 132→187 ADR, №1-№289+→№1-№763), and the
// declared "bridge copy" CLAUDE.md was not byte-identical to AGENTS.md
// (8958 vs 10703 bytes). This test binds the blocking cell: the python
// gate must pass on the clean tree, its tamper traps must spring, and
// the byte-identity of the bridge copy must hold right here in Rust (the
// gate and the test disagreeing is itself a drift).
#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn gate() -> PathBuf {
    repo_root().join("scripts/ci/agent_index_gate.py")
}

#[test]
fn n763_agent_index_gate_blocks() {
    // 1. The clean tree passes the gate.
    let out = Command::new("python3")
        .arg(gate())
        .current_dir(repo_root())
        .output()
        .expect("python3 must be available (the CI gates are python)");
    assert!(
        out.status.success(),
        "agent-index-gate must pass on the clean tree: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // 2. The tamper traps spring (the gate's own negative circuit).
    let out = Command::new("python3")
        .arg(gate())
        .arg("--self-test")
        .current_dir(repo_root())
        .output()
        .expect("python3 must be available");
    assert!(
        out.status.success(),
        "the gate self-test traps must spring: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // 3. The bridge-copy byte identity — the Rust twin of the gate's
    //    check 5 (the gate and the pin must never disagree).
    let agents = repo_root().join("AGENTS.md");
    let claude = repo_root().join("CLAUDE.md");
    assert!(
        agents.exists() && claude.exists(),
        "AGENTS.md and CLAUDE.md both exist (the №676 A5 bridge contract)"
    );
    assert_eq!(
        std::fs::read(&agents).expect("AGENTS.md readable"),
        std::fs::read(&claude).expect("CLAUDE.md readable"),
        "CLAUDE.md must be byte-identical to AGENTS.md — copy AGENTS.md over CLAUDE.md"
    );

    // 4. The llms.txt numbers stay machine-verifiable in Rust too (a
    //    cheap grep twin: the builtins line names 526 — the registry SSOT
    //    count the gen_metrics machine prints).
    let llms = std::fs::read_to_string(repo_root().join("llms.txt")).expect("llms.txt readable");
    assert!(
        llms.contains("526 builtins"),
        "llms.txt must state the live builtins count (526 — the registry scan)"
    );
    assert!(
        !llms.contains("409 builtins"),
        "the stale 409 must never reappear (the R-01 drift)"
    );
}

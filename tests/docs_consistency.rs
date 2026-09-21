// ── Naryad #384: docs-consistency — README vs docs/limitations.md ─────
//
// The external engineering guide (2026-09-17) fixed a docs drift:
// `docs/limitations.md` honestly marks the VM Stage 1 gaps and the
// adapt-metric as CLOSED, while README still carried stale claims
// ("not supported yet" next to match/BlockIfElse/PRNG; "fixed mock
// value (0.95)" without the mock-mode caveat). The prose is rewritten
// by naryad #384; THIS test is the mechanical gate that keeps it true:
//
//   (а) no README line says "not supported yet" next to a VM feature
//       that limitations.md marks CLOSED;
//   (б) every README claim carrying the 0.95 accuracy figure names the
//       mock-mode caveat (`mock mode` / `METALOGOS_MOCK_LLM`);
//   (в) the "Dual Execution Backend" section mentions the parity gate
//       (`crosscheck_backends`), the staged plan owner (ADR-0141) and
//       the serve knob contract — the VM default with the interpreter
//       opt-out (ADR-0171 flip) — the newcomer contract.
//
// The check is purely textual (std-only, the readme_consistency.rs
// pattern): file reads + line asserts, no interpreter, no language run.
// The mutation checks below pin that the linter is RED on the exact
// stale lines this naryad removed — reverting the prose turns CI red.

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    Path::new(&manifest).to_path_buf()
}

fn readme_lines() -> Vec<String> {
    let p = repo_root().join("README.md");
    fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("cannot read {:?}: {}", p, e))
        .lines()
        .map(str::to_string)
        .collect()
}

fn limitations_text() -> String {
    let p = repo_root().join("docs").join("limitations.md");
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("cannot read {:?}: {}", p, e))
}

// ── (а) CLOSED VM rows in limitations.md must not be denied in README ─

/// The VM-feature keywords a stale "not supported yet" claim would deny
/// — kept in sync with the CLOSED rows of the limitations.md VM table.
const VM_CLOSED_FEATURES: [&str; 6] = ["match", "BlockIfElse", "binop", "PRNG", "Bool", "VM"];

#[test]
fn readme_never_denies_a_closed_vm_feature() {
    let readme = readme_lines();
    let mut offenders: Vec<(usize, String)> = Vec::new();
    for (i, line) in readme.iter().enumerate() {
        if line.contains("not supported yet") {
            offenders.push((i + 1, line.clone()));
        }
    }
    // The full-file rule: after naryad #384 the phrase "not supported
    // yet" must not appear in README AT ALL next to any VM feature word.
    // (The only legitimate use left in the repo is the sandbox-timeout
    // caveat in limitations/README §5 — checked separately below.)
    let vm_denials: Vec<&(usize, String)> = offenders
        .iter()
        .filter(|(_, line)| {
            VM_CLOSED_FEATURES
                .iter()
                .any(|f| line.to_ascii_lowercase().contains(&f.to_ascii_lowercase()))
        })
        .collect();
    assert!(
        vm_denials.is_empty(),
        "README denies a VM feature that docs/limitations.md marks CLOSED (naryad #384 drift): {:?}",
        vm_denials
    );
}

#[test]
fn limitations_md_still_carries_the_closed_vm_rows() {
    // The gate is only as honest as its source: if the CLOSED rows are
    // removed from limitations.md, this test screams — the two docs
    // cannot silently drift apart in either direction.
    let text = limitations_text();
    for marker in [
        "CLOSED (№369)",
        "CLOSED (№370)",
        "CLOSED (№371)",
        "CLOSED (№372",
        "CLOSED for real mode (№375)",
    ] {
        assert!(
            text.contains(marker),
            "limitations.md lost its {} row — the docs-consistency gate depends on it",
            marker
        );
    }
}

// ── (б) 0.95 claims must carry the mock-mode caveat ───────────────────

/// The linter for one README line carrying the 0.95 accuracy figure:
/// acceptable iff the line names the mock-mode caveat. Public to this
/// module so the mutation checks below can pin it red on the exact
/// stale line naryad #384 removed.
fn mock_claim_has_caveat(line: &str) -> Result<(), String> {
    if !line.contains("0.95") {
        return Ok(());
    }
    let has_caveat = line.contains("mock mode") || line.contains("METALOGOS_MOCK_LLM");
    if has_caveat {
        Ok(())
    } else {
        Err(format!(
            "a 0.95 claim without the mock-mode caveat: {}",
            line.trim()
        ))
    }
}

#[test]
fn readme_095_claims_carry_the_mock_mode_caveat() {
    let readme = readme_lines();
    let mut failures: Vec<String> = Vec::new();
    for (i, line) in readme.iter().enumerate() {
        if let Err(e) = mock_claim_has_caveat(line) {
            failures.push(format!("line {}: {}", i + 1, e));
        }
    }
    assert!(
        failures.is_empty(),
        "README carries 0.95 accuracy claims without the mock-mode caveat: {:?}",
        failures
    );
}

// ── (в) The Dual Backend section names the parity contract ────────────

#[test]
fn dual_backend_section_mentions_parity_gate_and_the_flipped_default() {
    let readme = readme_lines();
    let start = readme
        .iter()
        .position(|l| l.starts_with("### 3. Dual Execution Backend"))
        .expect("README must keep the 'Dual Execution Backend' section");
    let end = readme[start + 1..]
        .iter()
        .position(|l| l.starts_with("### "))
        .map(|off| start + 1 + off)
        .unwrap_or(readme.len());
    let section: String = readme[start..end].join("\n");

    for needle in [
        "crosscheck_backends",                 // the parity gate (№373)
        "ADR-0141",                            // the staged plan owner
        "METALOGOS_SERVE_BACKEND=interpreter", // the explicit opt-out of the flipped default (ADR-0171)
        "CLOSED",                              // Stage 1 gaps are closed, not "not supported yet"
    ] {
        assert!(
            section.contains(needle),
            "the Dual Execution Backend section must mention '{}' — the newcomer contract (2-minute read) requires it",
            needle
        );
    }
    // The flipped default (ADR-0171) and the preserved opt-out must both
    // be stated.
    assert!(
        section.contains("VM by default"),
        "the section must state that mlog serve defaults to the VM (ADR-0171 flip)"
    );
}

#[test]
fn self_modification_section_states_real_metric_and_mock_scope() {
    let readme = readme_lines();
    let start = readme
        .iter()
        .position(|l| l.starts_with("### 5. Self-Modification"))
        .expect("README must keep the 'Self-Modification' section");
    let end = readme[start + 1..]
        .iter()
        .position(|l| l.starts_with("### "))
        .map(|off| start + 1 + off)
        .unwrap_or(readme.len());
    let section: String = readme[start..end].join("\n");

    assert!(
        section.contains("REAL in real mode"),
        "the section must state the metric is real in real mode (№375)"
    );
    assert!(
        section.contains("mock mode") && section.contains("METALOGOS_MOCK_LLM"),
        "the 0.95 stub scope must be named: mock mode / METALOGOS_MOCK_LLM"
    );
    assert!(
        !section.contains("Revisit point"),
        "the 2026-09-10 revisit point is exhausted by №375 — it must not return"
    );
}

// ── Mutation checks: the linter is RED on the exact stale lines ───────

#[test]
fn mutation_restoring_the_old_vm_line_turns_the_linter_red() {
    // The verbatim pre-#384 line (git history, README.md:195).
    let old_line = "Tree-walking interpreter (full language) + bytecode VM (47 instructions; experimental for full-language use — `match` (statement and `let`-binding expression), `Expr::BlockIfElse` (if/else as value), heterogeneous binop coercion, PRNG state — not supported yet, see ADR-0105 + ADR-0141 for staged closure plan).";
    assert!(
        old_line.contains("not supported yet"),
        "the mutation fixture must be the stale claim"
    );
    let denied = VM_CLOSED_FEATURES.iter().any(|f| {
        old_line
            .to_ascii_lowercase()
            .contains(&f.to_ascii_lowercase())
    });
    assert!(
        denied,
        "the stale line names CLOSED VM features — the (а) check must flag it"
    );
}

#[test]
fn mutation_restoring_the_old_mock_line_turns_the_linter_red() {
    // The verbatim pre-#384 line (git history, README.md:203).
    let old_line = "Quality metric is currently a fixed mock value (0.95), not a real accuracy computation — rollback logic exists but does not yet respond to actual quality degradation. See ADR-0112.";
    let res = mock_claim_has_caveat(old_line);
    assert!(
        res.is_err(),
        "the stale 0.95 line has no mock-mode caveat — the (б) check must reject it: {:?}",
        res
    );
}

#[test]
fn the_new_prose_passes_its_own_linter() {
    // The rewritten lines (green direction of the mutation pair).
    let new_mock = "The 0.95 stub remains ONLY in mock mode (`METALOGOS_MOCK_LLM`, the default-on test mode) and is loudly documented at the call site — it exercises the rollback mechanism, it is not a quality signal.";
    assert!(mock_claim_has_caveat(new_mock).is_ok());
}

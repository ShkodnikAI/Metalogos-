//! Наряд №373 (ADR-0141 Stage 2 / §D3): parity gate — the crosscheck must run
//! WITHOUT VM-uncovered exclusions.
//!
//! The ONLY sanctioned runtime exclusions in `tests/crosscheck_backends.rs`
//! are (each is loud, classified, and re-asserted here):
//!   1. Negative-test contracts (`*unknown_fn*`, `*wrong_*`) — designed to
//!      FAIL, not a parity concern (sanctioned by ADR-0141 §D3 and наряд
//!      №373 explicitly).
//!   2. Candle-feature-gated examples (reflex_seq_*/reflex_gen_*, 11 files) —
//!      fail on BOTH backends identically without the `candle` feature ("the
//!      'candle' feature is not enabled"); verified under the candle-tests CI
//!      job (№200). A feature gate, not a VM-uncovered construct.
//!
//! If parity ever regresses (someone adds a new `continue;` exclusion), this
//! test FAILS LOUDLY and наряд-style re-justification is forced.

use std::fs;
use std::path::Path;

/// The frozen list of candle-feature-gated examples excluded from the
/// no-candle crosscheck (№204/№205, ADR-0121; re-justified №373).
const CANDLE_GATED: &[&str] = &[
    "reflex_seq_declare.mlog",
    "reflex_seq_mixed_error.mlog",
    "reflex_seq_missing_labels_error.mlog",
    "reflex_seq_train_predict.mlog",
    "reflex_seq_transformer_block.mlog",
    "reflex_seq_gqa.mlog",
    "reflex_seq_stacked.mlog",
    "reflex_seq_gqa_stack.mlog",
    "reflex_gen_declare.mlog",
    "reflex_gen_from_text.mlog",
    "reflex_batch_train.mlog",
];

/// Negative-test contract name patterns (designed-to-fail programs).
fn is_negative_contract(name: &str) -> bool {
    name.contains("unknown_fn") || name.contains("wrong_")
}

/// The crosscheck source must contain EXACTLY two `continue;` exclusion
/// sites: the negative-contract filter and the candle-gated block.
#[test]
fn naryad_373_crosscheck_has_only_sanctioned_exclusion_sites() {
    let src = fs::read_to_string("tests/crosscheck_backends.rs").expect("crosscheck source exists");
    let count = src.matches("continue;").count();
    assert_eq!(
        count, 2,
        "crosscheck_backends.rs now has {} `continue;` sites — a NEW exclusion was added. \
         VM-uncovered exclusions are forbidden at Stage 2 (ADR-0141 §D3): parity must be 100% \
         modulo designed-to-fail contracts and candle feature gates. \
         Re-justify loudly or close the gap in the VM instead.",
        count
    );
}

/// Every .mlog example with a .expected file is either crosschecked, a
/// negative contract, or in the frozen candle list — nothing else may be
/// silently skipped, and the candle list must not rot (files must exist).
#[test]
fn naryad_373_every_golden_example_is_classified() {
    let examples = Path::new("examples");
    let mut crosschecked = 0usize;
    let mut negatives = 0usize;
    let mut candle_gated = 0usize;
    let mut unclassified: Vec<String> = Vec::new();

    let mut entries: Vec<_> = fs::read_dir(examples)
        .expect("examples dir exists")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "mlog").unwrap_or(false))
        .collect();
    entries.sort();

    for path in entries {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if is_negative_contract(&name) {
            negatives += 1;
        } else if CANDLE_GATED.contains(&name.as_str()) {
            candle_gated += 1;
        } else if path.with_extension("expected").exists() {
            crosschecked += 1;
        } else {
            // No .expected → not part of the golden crosscheck set at all
            // (examples without goldens are exercised by integration tests).
            // Track them so the volume is visible in the assertion message.
            unclassified.push(name);
        }
    }

    assert!(
        candle_gated == CANDLE_GATED.len(),
        "expected {} candle-gated examples, matched {} — the frozen list in \
         naryad_373_parity_gate.rs rotted (file renamed/removed?)",
        CANDLE_GATED.len(),
        candle_gated
    );
    assert!(
        negatives > 0,
        "negative-contract examples disappeared — the designed-to-fail class is empty"
    );
    assert!(
        crosschecked > 100,
        "crosschecked golden set implausibly small: {}",
        crosschecked
    );
    eprintln!(
        "№373 parity classification: crosschecked={} negatives={} candle_gated={} non_golden={}",
        crosschecked,
        negatives,
        candle_gated,
        unclassified.len()
    );
}

/// docs/limitations.md: every Stage-1 VM row must be CLOSED (№369–№372);
/// the only open VM row allowed is the Stage-3+ default-flip gate.
#[test]
fn naryad_373_limitations_stage1_rows_closed() {
    let doc = fs::read_to_string("docs/limitations.md").expect("limitations.md exists");
    for (fragment, naryad) in [
        ("`Match` statement not compiled to VM", "369"),
        (
            "`Expr::BlockIfElse` (if/else as value) not compiled to VM",
            "370",
        ),
        ("`match_expr` (`let x = match y {...}`) — TW-only", "369"),
        ("Binop coercion (heterogeneous List+String)", "371"),
        ("PRNG state (`random_seed`/`random`)", "372"),
        ("Bool→String formatting", "372"),
    ] {
        // Find the table row containing the limitation text…
        let row = doc
            .lines()
            .find(|l| l.contains(fragment))
            .unwrap_or_else(|| panic!("limitations.md lost the row for {}", fragment));
        // …and require the CLOSED marker with the right наряд number.
        assert!(
            row.contains("CLOSED") && row.contains(naryad),
            "Stage-1 VM row '{}' must be marked CLOSED (№{}) — got: {}",
            fragment,
            naryad,
            row
        );
    }
    // The default-flip row is CLOSED: Stage 5 was EXECUTED 2026-09-21 — the
    // owner's flip decision («Флипай») on the re-gate №3 3/3 GREEN evidence
    // (both thresholds), recorded in ADR-0171 + ADR-0141 Addendum 7.
    let flip_row = doc
        .lines()
        .find(|l| l.contains("VM is not the default backend"))
        .expect("default-flip row must remain present (as the CLOSED history row)");
    assert!(
        flip_row.contains("CLOSED") && flip_row.contains("№404"),
        "the serve default-flip row must be marked CLOSED (№404 Stage 5, ADR-0171) — got: {}",
        flip_row
    );
}

/// The soak workflow exists and is nightly-scheduled + dispatchable.
#[test]
fn naryad_373_soak_workflow_is_nightly() {
    let wf = fs::read_to_string(".github/workflows/soak.yml")
        .expect("soak.yml must exist (parity soak, ADR-0141 §D4/№373)");
    assert!(wf.contains("schedule:"), "soak must be scheduled");
    assert!(wf.contains("cron:"), "soak needs a cron expression");
    assert!(
        wf.contains("workflow_dispatch:"),
        "soak must be manually dispatchable for on-demand soak evidence"
    );
    assert!(
        wf.contains("crosscheck_backends"),
        "soak must run the parity crosscheck"
    );
    assert!(wf.contains("cargo test"), "soak must run the test suite");
}

/// No stubs in the changed/test files (№16.0-D). The markers are assembled
/// from parts so this source does not self-match.
#[test]
fn naryad_373_no_stubs() {
    let markers = [
        concat!("todo", "!"),
        concat!("unimplemented", "!"),
        concat!("SKELE", "TON"),
    ];
    for f in [
        "tests/crosscheck_backends.rs",
        "tests/naryad_373_parity_gate.rs",
        ".github/workflows/soak.yml",
    ] {
        let s = fs::read_to_string(f).unwrap_or_default();
        for m in markers {
            assert!(!s.contains(m), "{} contains stub marker {}", f, m);
        }
    }
}

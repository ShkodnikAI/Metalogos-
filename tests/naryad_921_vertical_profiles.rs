// ── tests/naryad_921_vertical_profiles.rs ───────────────────────────
// №921 (Wave 84 — W-L2; the dispatch gh#1842): the VERTICAL_PROFILES
// fact machine (ADR-0220 §2) is pinned by THIS test — the gate is not a
// self-asserting script: the test spawns the machine against the live
// tree and refuses the merge when
//   1. `gen_metrics.py --check` is not green (the floors + the blocks),
//   2. the OK line does not carry the VERTICAL_PROFILES 12/12 verdict
//      (the gate_045 floor — the wave-84 dispatch constant, only-up),
//   3. `gen_metrics.py --tamper-test` is not green (a dead trap is a
//      lying checker — the №841 discipline).
// The №535 blocking_checks.tsv row (check|all|vertical-roster) binds to
// this file by name (n921_vertical_profiles_gate) — the cell dies when
// the test is renamed or deleted (the sync gate's drift rule).
// The №475 fs_gate posture: this file reads the docs and spawns the
// generator gate BY DESIGN (the naryad_566/918 precedent).
//
// The kernel side (the fail-closed verdict, the no-default-attach
// negative control, the digest pins) is pinned HERE over the live
// library; the package-path e2e (mlogpkg pack/sign/verify with the
// `[vertical]` manifest) lives in mlogpkg/tests/naryad_921_vertical_e2e.rs
// (the binary e2e needs CARGO_BIN_EXE_mlogpkg — the mlogpkg package's
// own integration tests).
#![allow(clippy::disallowed_methods)]

use metalogos::vertical::{
    refusals, vertical_registry, vertical_verdict, VerticalContour, VerticalDeclaration,
    VerticalId, VerticalProfile,
};
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
fn vertical_gate_check_is_green_and_counts_12() {
    let (code, out) = run_gen_metrics(&["--check"]);
    assert_eq!(
        code, 0,
        "gen_metrics --check must be green on the live tree:\n{out}"
    );
    assert!(
        out.contains("VERTICAL_PROFILES 12/12"),
        "the OK line must carry the VERTICAL_PROFILES 12/12 verdict \
         (the gate_045 floor, only-up):\n{out}"
    );
}

#[test]
fn vertical_gate_double_check_is_deterministic() {
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
fn vertical_gate_tamper_traps_are_alive() {
    // Traps 51–54: the removed seeding (а), the emptied
    // mandatory_contours (б), the dead digest-pin consumption (в) and
    // the hand-edited GENERATED row (г) — every trap must stay red
    // under the tamper (a silent trap is a lying checker).
    let (code, out) = run_gen_metrics(&["--tamper-test"]);
    assert_eq!(
        code, 0,
        "gen_metrics --tamper-test must be green (all traps alive):\n{out}"
    );
    assert!(
        out.contains("VERTICAL_PROFILES 12/12"),
        "the OK line must carry the VERTICAL_PROFILES 12/12 verdict:\n{out}"
    );
}

#[test]
fn the_registry_carries_the_two_r38_roster_profiles() {
    let reg = vertical_registry();
    assert_eq!(reg.len(), 2, "the R38(5) roster: payments + public-sector");
    assert_eq!(reg[0].id, VerticalId::Payments);
    assert_eq!(reg[1].id, VerticalId::PublicSector);
    for p in reg {
        assert_eq!(
            p.mandatory_contours.len(),
            6,
            "the full six mandatory contours (ADR-0220 §2)"
        );
        // The self-describing digest pin re-derives byte-for-byte (the
        // G5/№828 form, the №868 re-attestation discipline).
        assert_eq!(p.carrier_digest, p.derive_digest());
        assert_eq!(p.carrier_digest.len(), 64, "hex(SHA-256)");
    }
    assert_ne!(
        reg[0].carrier_digest,
        reg[1].carrier_digest,
        "the id discriminates the pins"
    );
}

#[test]
fn the_constructor_refuses_a_partial_contour_set_and_a_drifted_needle() {
    // A partial set (a duplicated contour) refuses at construction —
    // the №879 form.
    let contours = [
        VerticalContour::JurisdictionCoverage,
        VerticalContour::HumanOversightApproval,
        VerticalContour::DecisionTraceVerify,
        VerticalContour::TaintFloor,
        VerticalContour::LicenseClass,
        VerticalContour::JurisdictionCoverage, // duplicate — the set is partial
    ];
    assert!(VerticalProfile::new(
        VerticalId::Payments,
        contours,
        [
            "payments contour (1) jurisdiction-coverage — the JurisdictionMatrix package, strictest-wins (ADR-0209)",
            "payments contour (2) human-oversight-approval — the typed approval mandate (ADR-0205)",
            "payments contour (3) decision-trace-verify — the replay/forensic ledger (ADR-0210)",
            "payments contour (4) taint-floor — the taint saturation lane (ADR-0217)",
            "payments contour (5) license-class — the license-typed model layer (ADR-0202)",
            "payments contour (6) refusal-vocabulary-pin — the frozen №385 dictionary, the version-pin",
        ],
        "385-frozen-2026-11",
    )
    .is_err());
    // A drifted anchor literal refuses (the fail-closed literal
    // contract — the roster needle can never lie about the contour).
    let live = [
        VerticalContour::JurisdictionCoverage,
        VerticalContour::HumanOversightApproval,
        VerticalContour::DecisionTraceVerify,
        VerticalContour::TaintFloor,
        VerticalContour::LicenseClass,
        VerticalContour::RefusalVocabularyPin,
    ];
    let mut anchors = [
        "payments contour (1) jurisdiction-coverage — the JurisdictionMatrix package, strictest-wins (ADR-0209)",
        "payments contour (2) human-oversight-approval — the typed approval mandate (ADR-0205)",
        "payments contour (3) decision-trace-verify — the replay/forensic ledger (ADR-0210)",
        "payments contour (4) taint-floor — the taint saturation lane (ADR-0217)",
        "payments contour (5) license-class — the license-typed model layer (ADR-0202)",
        "payments contour (6) refusal-vocabulary-pin — the frozen №385 dictionary, the version-pin",
    ];
    anchors[0] = "payments contour (1) A COMPLETELY DIFFERENT CLAIM";
    assert!(
        VerticalProfile::new(VerticalId::Payments, live, anchors, "385-frozen-2026-11")
            .is_err(),
        "a drifted needle refuses at construction"
    );
}

#[test]
fn the_verdict_is_fail_closed_on_every_named_divergence() {
    // (1) An unknown id → the named refusal (no fuzzy match, no
    // default; medicine stays backlog — R38(5)).
    let bad_id = VerticalDeclaration {
        id: "medicine".to_string(),
        digest: "00".repeat(32),
    };
    let err = vertical_verdict(Some(&bad_id)).unwrap_err();
    assert!(
        err.starts_with(refusals::UNKNOWN_ID),
        "the unknown-id refusal must name itself: {err}"
    );
    // (2) A digest mismatch → the named refusal (the package was
    // attested against a different profile generation).
    let bad_digest = VerticalDeclaration {
        id: "payments".to_string(),
        digest: "ab".repeat(32),
    };
    let err = vertical_verdict(Some(&bad_digest)).unwrap_err();
    assert!(
        err.starts_with(refusals::DIGEST_MISMATCH),
        "the digest-mismatch refusal must name itself: {err}"
    );
    // (3) The live pin → Ok(Some(profile)).
    let live = VerticalDeclaration {
        id: "payments".to_string(),
        digest: vertical_registry()[0].carrier_digest.clone(),
    };
    let v = vertical_verdict(Some(&live)).expect("the live pin passes");
    assert_eq!(v.expect("a profile attaches").id.as_str(), "payments");
}

#[test]
fn no_declaration_attaches_no_profile() {
    // The negative control (ADR-0220 §2): no `[vertical]` declaration →
    // Ok(None) — the undeclared manifest proceeds with ZERO behavioral
    // mutations (no default attach, no closest-match attach).
    assert!(
        matches!(vertical_verdict(None), Ok(None)),
        "the absence of a declaration must attach nothing"
    );
}

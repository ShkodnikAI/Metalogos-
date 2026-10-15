// ── tests/naryad_686_sanitizer_guard.rs ──────────────────────────────
// №686 (gh#1253, wave 44; the audit f92c61c S-1/S-2): the risk-review
// sanitizer guard is pinned by a fixture self-test and content checks.
//
// The audit's S-1 lesson: a sanitizer-semantics change (N9 in render())
// shipped without reaching the risk-review checklist, the domain quota,
// or the honest-boundaries protocol — work outside the machine. The
// repair has two halves that MUST move together (a checklist edit
// without the guard = a failure here, and vice versa):
//
//   1. `docs/risk-review-checklist.md` item 10 — the sanitizer/source/
//      sink section a PR touching the taint/label sites must fill;
//   2. `scripts/ci/risk_review.py` — the mechanical reminder: a diff
//      touching a site file (audit.rs / html_label.rs /
//      builtins_classification.rs / semantic.rs) carries the item-10
//      section in the report AND triggers the report on its own.
//
// The self-test fixtures live INSIDE the script (--self-test mode:
// synthetic unified-0 diffs, no git, no network — the merge_ci_audit
// form); this file is the CI-side driver: the self-test MUST stay
// green, and the site set + the checklist header MUST stay in sync with
// this file's pins.
#![allow(clippy::disallowed_methods)]

use std::process::Command;

const SCRIPT: &str = include_str!("../scripts/ci/risk_review.py");
const CHECKLIST: &str = include_str!("../docs/risk-review-checklist.md");

/// The №686 site set — MUST stay identical to the script's
/// `SANITIZER_SITE_FILES` and to the file list of checklist item 10.
/// All three move together in one PR, never alone.
const SANITIZER_SITES: [&str; 4] = [
    "audit.rs",
    "html_label.rs",
    "builtins_classification.rs",
    "semantic.rs",
];

#[test]
fn n686_risk_review_self_test_is_green() {
    let out = Command::new("python3")
        .arg("scripts/ci/risk_review.py")
        .arg("--self-test")
        .output()
        .expect("python3 must exist (the dev machines and the CI image run the gate scripts)");
    assert!(
        out.status.success(),
        "risk_review --self-test must stay green: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("4/4 fixture groups green"),
        "the self-test must report all fixture groups: {stdout}"
    );
}

#[test]
fn n686_checklist_carries_item_10() {
    // The checklist grew to ten items and item 10 is the sanitizer
    // section (the header the reviewer and the report both reference).
    assert!(
        CHECKLIST.contains("## The ten items"),
        "the checklist intro must name ten items"
    );
    assert!(
        CHECKLIST.contains("10. **Sanitizers, sources, and sinks"),
        "checklist item 10 (sanitizers/sources/sinks) is missing — the guard reminds about a dead item"
    );
    // The three content obligations of the item (the report's reminder
    // mirrors them; the pair drifts = a failure here).
    for phrase in [
        "source → sink",
        "regression pair",
        "render`/`template_render`/`set_header`-class contracts",
    ] {
        assert!(
            CHECKLIST.contains(phrase),
            "checklist item 10 must carry the obligation phrase: {phrase}"
        );
    }
}

#[test]
fn n686_script_site_set_is_pinned() {
    for site in SANITIZER_SITES {
        assert!(
            SCRIPT.contains(&format!("\"{site}\",")),
            "the script's SANITIZER_SITE_FILES must name {site}"
        );
    }
    // The reminder section exists and carries the REQUIRED marker (the
    // grep-able fact the PR reviewer looks for in the job summary).
    assert!(
        SCRIPT.contains("Item 10 — sanitizer / source / sink changes"),
        "the script must carry the item-10 section header"
    );
    assert!(
        SCRIPT.contains("REQUIRED"),
        "the reminder must be marked REQUIRED"
    );
    // The trigger: a site hit alone must trigger the report (--changed
    // superset — a PR touching only html_label.rs must not slip past).
    assert!(
        SCRIPT.contains("or sanitizer_hits"),
        "the --changed trigger must include the sanitizer-site hits"
    );
}

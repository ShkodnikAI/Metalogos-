// ── tests/naryad_687_off_naryad_quota.rs ─────────────────────────────
// №687 (gh#1254, wave 44; the audit f92c61c S-2): the off-naryad surface
// scan of the quota counter is pinned by a fixture self-test and pins.
//
// The S-2 lesson: PE-1/PE-2 — 14 commits, more than half of the window,
// new product surfaces (HTML/XML parsing, Telegram, outbound HTTP,
// headers, raw insert) — shipped OUTSIDE the naryads and outside the
// domain quota; the wave's quota line read «0/15 PASS» while the
// registry grew 516 → 526. The counter must not be ABLE to "not notice"
// a surface growth: a commit without a recognized naryad number that
// touches `src/builtins/registry.rs` or ADDS a Cargo.toml feature/
// dependency key is a red-line finding, and the window verdict FAILS.
//
// The self-test fixtures live INSIDE the script (--self-test mode: the
// pure classification core over synthetic commits — no git, no network,
// the merge_ci_audit form); this file is the CI-side driver: the
// self-test MUST stay green, and the surface markers + the three-line
// report vocabulary MUST stay in sync with this file's pins (a marker
// rename without the test = a failure here, and vice versa).
#![allow(clippy::disallowed_methods)]

use std::process::Command;

const SCRIPT: &str = include_str!("../scripts/ci/naryad_classes.py");

#[test]
fn n687_naryad_classes_self_test_is_green() {
    let out = Command::new("python3")
        .arg("scripts/ci/naryad_classes.py")
        .arg("--self-test")
        .output()
        .expect("python3 must exist (the dev machines and the CI image run the gate scripts)");
    assert!(
        out.status.success(),
        "naryad_classes --self-test must stay green: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("6/6 fixture groups green"),
        "the self-test must report all fixture groups: {stdout}"
    );
}

#[test]
fn n687_surface_markers_are_pinned() {
    // The two surface markers of the №687 red line — the registry growth
    // and the Cargo feature/dependency graph.
    assert!(
        SCRIPT.contains("\"src/builtins/registry.rs\""),
        "the registry marker must stay pinned in the script"
    );
    assert!(
        SCRIPT.contains("CARGO_PATH = \"Cargo.toml\""),
        "the Cargo marker must stay pinned in the script"
    );
    // The naryad-commit recognition covers BOTH spellings (the Latin
    // "Naryad №…" of the squash convention AND the Cyrillic "Наряд №…"
    // of the older windows) — the fixture test caught the Latin gap.
    assert!(
        SCRIPT.contains("(?:[Nn]aryad|[Нн]аряд)"),
        "the naryad-subject pattern must cover the Latin and the Cyrillic spellings"
    );
}

#[test]
fn n687_three_line_report_vocabulary_is_pinned() {
    // The report is THREE lines (the naryad quota PASS/FAIL, the
    // off-naryad findings, the combined verdict); the dispatch --line
    // format is unchanged (backward compatibility of reading).
    for phrase in [
        "naryad-quota (№470/№608):",
        "off-naryad surface commits (№687):",
        "quota verdict:",
        "FAIL (the surface grew outside the naryads",
    ] {
        assert!(
            SCRIPT.contains(phrase),
            "the three-line report must carry: {phrase}"
        );
    }
    // The dispatch line vocabulary survives untouched.
    assert!(
        SCRIPT.contains("= {share:.1%} — {'PASS' if verdict else 'FAIL'} ({kind})"),
        "the dispatch --line format must stay byte-compatible"
    );
}

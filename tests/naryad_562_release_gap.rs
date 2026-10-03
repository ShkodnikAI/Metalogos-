// ── tests/naryad_562_release_gap.rs ──────────────────────────────────
// №562 (Wave 25 P2; the audit 02.10 §6.1; dispatch gh#925): the tag↔main
// gap gate — the M-3 class ("a High fix lives outside a release")
// becomes a MEASURED state. A pending Security entry in [Unreleased]
// older than the floor (N=14 days, the OWNER's parameter — №562
// proposes, it does not choose) fails the blocking `release-gap
// (blocking)` CI job; an UNDATED pending Security entry fails too
// (fail-closed: an undated pending fix cannot be measured).
//
// The fixtures drive the REAL script (scripts/ci/release_gap_gate.py)
// through N562_CHANGELOG fixtures; the live run proves the committed
// CHANGELOG passes today; the structural pin keeps the job and the
// required set in sync (the №551 posture).
#![allow(clippy::disallowed_methods)]

use std::process::Command;

fn script() -> &'static str {
    "scripts/ci/release_gap_gate.py"
}

fn write_fixture(name: &str, body: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("n562_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, body).unwrap();
    path
}

#[test]
fn n562_self_test_is_green() {
    let out = Command::new("python3")
        .args([script(), "--self-test"])
        .output()
        .expect("python3 must exist (the CI image runs the gate scripts)");
    assert!(
        out.status.success(),
        "the release-gap self-test must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("SELF-TEST OK"),
        "the self-test must print SELF-TEST OK, got: {stdout}"
    );
    for case in [
        "no-security-part",
        "fresh-security-pass",
        "over-age-security-fails",
        "unstamped-security-fails",
        "no-unreleased-at-all",
    ] {
        assert!(
            stdout.contains(case),
            "the self-test must cover the '{case}' case, got: {stdout}"
        );
    }
}

#[test]
fn n562_committed_changelog_passes() {
    // The live state: [Unreleased] carries NO Security part — the gate is
    // green. A pending Security entry landing later moves this to the
    // clock (the measured limbo), and an over-age one goes red.
    let out = Command::new("python3")
        .arg(script())
        .output()
        .expect("python3 must exist");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "the committed CHANGELOG must pass the release-gap gate, output: {stdout}"
    );
    assert!(stdout.contains("release-gap: OK"), "got: {stdout}");
}

#[test]
fn n562_fresh_security_entry_passes() {
    let p = write_fixture(
        "fresh.mlog",
        "# Changelog\n\n## [Unreleased]\n\n### Security\n\n\
         - **the fix (gh#1):** landed 2026-10-01\n\n## [0.28.0] - 2026-09-01\n",
    );
    let out = Command::new("python3")
        .arg(script())
        .env("N562_CHANGELOG", &p)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "a date-stamped fresh Security entry passes: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn n562_over_age_security_entry_fails() {
    let p = write_fixture(
        "overage.mlog",
        "# Changelog\n\n## [Unreleased]\n\n### Security\n\n\
         - **the fix (gh#1):** landed 2026-01-01\n\n## [0.27.1] - 2026-08-01\n",
    );
    let out = Command::new("python3")
        .arg(script())
        .env("N562_CHANGELOG", &p)
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "an over-age pending Security entry must FAIL the gate"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("2026-01-01") && stdout.contains("release-gap: FAIL"),
        "the FAIL names the oldest stamp: {stdout}"
    );
}

#[test]
fn n562_unstamped_security_entry_fails() {
    let p = write_fixture(
        "unstamped.mlog",
        "# Changelog\n\n## [Unreleased]\n\n### Security\n\n\
         - **the fix (gh#1):** no date anywhere\n\n## [0.28.0] - 2026-10-01\n",
    );
    let out = Command::new("python3")
        .arg(script())
        .env("N562_CHANGELOG", &p)
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "an UNDATED pending Security entry must fail (fail-closed)"
    );
}

#[test]
fn n562_the_job_is_blocking_and_in_the_required_set() {
    // The structural pin: the release-gap job exists, is blocking, and
    // joined the №551 required set (the three synced places).
    let ci = include_str!("../.github/workflows/ci.yml");
    assert!(
        ci.contains("release-gap:\n    name: release-gap (blocking)"),
        "the release-gap (blocking) job must exist in ci.yml"
    );
    let py = include_str!("../scripts/ci/merge_ci_audit.py");
    assert!(
        py.contains("\"release-gap (blocking)\""),
        "the audit script's REQUIRED_CHECKS must pin the job"
    );
    let checklist = include_str!("../docs/maintainers.md");
    assert!(
        checklist.contains("`release-gap (blocking)`"),
        "the branch-protection checklist must pin the job"
    );
}

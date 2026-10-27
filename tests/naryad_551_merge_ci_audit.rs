// ── tests/naryad_551_merge_ci_audit.rs ───────────────────────────────
// №551 (Wave 25 P0; the audit 02.10 M-1 + §6.2): the weekly merge↔CI
// audit script — scripts/ci/merge_ci_audit.py — is pinned by a fixture
// self-test and a content check.
//
// The script's contract:
//   - the required set is EXACTLY the live branch protection (№824:
//     the 2026-10-28 admin read, 23 contexts — the four pillar
//     contexts dropped by the owner's №1593 toggle) — the job DISPLAY
//     names as the check-runs API reports them;
//   - a `skipped` required check is NOT green (a check that can
//     silently skip protects nothing);
//   - an absent check is a named problem ("NO RUN"), never silence
//     (fail-closed: a silent auditor is a liar auditor);
//   - a PR whose head data cannot be fetched is a DIVERGENCE, not a
//     pass ("UNAVAILABLE" — unknown is never green);
//   - a commit without the squash-merge "(#N)" tail is a divergence
//     too (direct pushes to main are outside the process);
//   - №824: the reverse direction — a LIVE protection context without
//     a required-set row is a divergence (the M-1 blind class); the
//     protection read needs BP_TOKEN and an unreadable rule is exit 2
//     (fail-closed, №525 — never a pass);
//   - №824: the tripwire — a required context with NO
//     pull_request-producing workflow job is a divergence (the №809
//     class: a PR could never turn it green; the offline
//     --tripwire-check mode runs here on every test lane).
//
// The self-test fixtures live INSIDE the script (--self-test mode:
// green, no-runs, skipped, failed, no-PR, unavailable — six cases, the
// last two cover the fail-closed posture; plus the №824 drift and
// tripwire fixture cases) — this file is the CI-side driver: the
// self-test MUST stay green for the workflow lane to be trusted, the
// script's required set MUST stay in sync with this file's
// enumeration (a checklist edit without the script = a failure here,
// and vice versa), and the live tripwire MUST stay green (a required
// context losing its PR producer = a failure here).
#![allow(clippy::disallowed_methods)]

use std::process::Command;

const SCRIPT: &str = include_str!("../scripts/ci/merge_ci_audit.py");

/// The required set — MUST stay identical to the script's
/// `REQUIRED_CHECKS` and to the branch-protection checklist in
/// docs/maintainers.md ("When CI is down"). All three move together in
/// one PR, never alone. (`msrv (blocking)` joined by №555, issue #916;
/// `release-gap (blocking)` by №562; the other ten joined by №824,
/// issue #1592 — the live-protection read of 2026-10-28, 23 contexts,
/// the pillar four dropped by the owner's №1593 toggle.)
const REQUIRED_SET: [&str; 23] = [
    "test-lib (blocking)",
    "test-integration (blocking)",
    "crosscheck (blocking)",
    "clippy (blocking)",
    "fmt (blocking)",
    "cargo-audit (blocking)",
    "cargo-deny (blocking)",
    "gitleaks (blocking)",
    "gate-facts-sync (blocking)",
    "blocking-checks-sync (blocking)",
    "registry-arity-check (blocking)",
    "msrv (blocking)",
    "release-gap (blocking)",
    "ADR numbering (blocking)",
    "branch-freshness (blocking)",
    "doc-tests (blocking)",
    "html-tests (blocking)",
    "ledger-golden (blocking)",
    "minimal-build (blocking)",
    "module-size-guard (blocking)",
    "test-llm-cache-contract (blocking)",
    "vscode-extension (blocking)",
    "xml-tests (blocking)",
];

#[test]
fn merge_ci_audit_self_test_is_green() {
    let out = Command::new("python3")
        .arg("scripts/ci/merge_ci_audit.py")
        .arg("--self-test")
        .output()
        .expect("python3 must exist (the dev machines and the CI image run the gate scripts)");
    assert!(
        out.status.success(),
        "the merge-ci-audit self-test must exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("SELF-TEST OK"),
        "the self-test must print SELF-TEST OK, got: {stdout}"
    );
    for case in [
        "green",
        "no-runs",
        "skipped",
        "failed",
        "no-PR",
        "unavailable",
    ] {
        assert!(
            stdout.contains(case),
            "the self-test must cover the '{case}' case, got: {stdout}"
        );
    }
}

#[test]
fn merge_ci_audit_required_set_is_pinned_in_the_script() {
    for name in REQUIRED_SET {
        assert!(
            SCRIPT.contains(&format!("\"{name}\"")),
            "the audit script's REQUIRED_CHECKS must pin '{name}' \
             (the №551 checklist, the ci.yml display names) — the script, \
             this file and docs/maintainers.md move together"
        );
    }
    // The fail-closed posture, pinned by string: unknown is a divergence,
    // skipped is not green, silence is never a pass.
    for marker in ["UNAVAILABLE", "skipped", "NO RUN", "fail-closed"] {
        assert!(
            SCRIPT.contains(marker),
            "the audit script must keep the fail-closed marker '{marker}'"
        );
    }
}

#[test]
fn merge_ci_audit_reports_instead_of_mutating() {
    // The наряд's граница: the script only REPORTS — it never merges,
    // reverts, or edits history. Pin it at the HTTP level: the only
    // mutating call in the script is the divergence issue (POST /issues);
    // no merge/revert/patch/delete API calls, no git history commands.
    assert!(
        SCRIPT.contains("repos/{repo}/issues"),
        "the script's only write is the divergence issue (POST /issues)"
    );
    for forbidden in [
        "method=\"PUT\"",
        "method=\"PATCH\"",
        "method=\"DELETE\"",
        "/merges",
        "git revert",
        "git reset",
        "git push",
    ] {
        assert!(
            !SCRIPT.contains(forbidden),
            "the audit script must not contain the mutating call '{forbidden}' \
             — it reports, it does not act on history"
        );
    }
}

#[test]
fn merge_ci_audit_reverse_direction_is_fail_closed() {
    // №824: the reverse direction reads the LIVE protection with the
    // Administration:read token (BP_TOKEN — the BRANCH_PROTECTION_TOKEN
    // repo secret, the №586 precedent). An unreadable protection is
    // exit 2 (№525: an unreadable state is not a pass), never a silent
    // skip; the drift line names the enforced-but-unaudited context.
    for marker in [
        "branches/{branch}/protection",
        "BP_TOKEN",
        "ProtectionUnreadable",
        "the M-1 blind class",
    ] {
        assert!(
            SCRIPT.contains(marker),
            "the script must carry the reverse-direction marker '{marker}'"
        );
    }
}

#[test]
fn merge_ci_audit_tripwire_is_green_on_live_workflows() {
    // №824: the №809-class tripwire, offline mode, against the LIVE
    // .github/workflows of this checkout: every required context must
    // have a pull_request-producing job. A context losing its producer
    // (the №809 episode reintroduced) fails this test on every CI lane.
    let out = Command::new("python3")
        .arg("scripts/ci/merge_ci_audit.py")
        .arg("--tripwire-check")
        .output()
        .expect("python3 must exist (the dev machines and the CI image run the gate scripts)");
    assert!(
        out.status.success(),
        "the tripwire must exit 0 (every required context produced on PR), stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("TRIPWIRE OK: all 23"),
        "the tripwire must cover all 23 required contexts, got: {stdout}"
    );
}

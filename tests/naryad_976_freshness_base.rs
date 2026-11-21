// ── №976 follow-up (gh#1985, the owner decision 21.11): the
//    branch-freshness base exception — the two-sided pin ──────────────
//
// The freshness bound keys on the PR BASE, not the branch name. The pin
// runs the REAL run-script extracted from .github/workflows/ci.yml in a
// throwaway git repo whose origin/main is 25 commits ahead (threshold
// is 20) and asserts BOTH sides:
//
//   1. base = main        → the bound applies → exit 1 (red, loud).
//   2. base = release/…   → exit 0 with the printed reason (success,
//      never skipped — the release-commit gate is fail-closed on
//      missing/red/pending/skipped).
//
// The mutation: invert the base comparison (`!= "main"` → `== "main"`)
// and the second side turns red — the exception is load-bearing, and
// removing it is caught here, not on a release day.
//
// Naryad #475's fs_gate ratchet targets PRODUCTION I/O paths; this test
// builds a real git repo on the real filesystem by design — the scoped
// allow mirrors the naryad_481/465 test posture (naryad_507/517, the
// naryad_976 doctest pin).
#![allow(clippy::disallowed_methods)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Extract the branch-freshness run-script from the live ci.yml and
/// substitute the GitHub Actions `${{ github.base_ref }}` placeholder.
fn script_for(base_ref: &str) -> String {
    let ci = fs::read_to_string(".github/workflows/ci.yml").expect("ci.yml readable");
    let start = ci
        .find("name: Check PR base is not stale relative to main")
        .expect("the branch-freshness step exists");
    let run = ci[start..]
        .find("run: |\n")
        .expect("the step has a run block")
        + start
        + 7;
    let end = ci[run..]
        .find("\n  test-lib:")
        .expect("the job ends before test-lib");
    let block = &ci[run..run + end];
    let mut out = String::new();
    for line in block.lines() {
        out.push_str(line.strip_prefix("          ").unwrap_or(line));
        out.push('\n');
    }
    out.replace("${{ github.base_ref }}", base_ref)
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
}

/// origin/main 25 commits ahead of the checked-out branch (threshold 20).
fn fresh_fixture(root: &Path, name: &str) -> PathBuf {
    let work = root.join(name);
    fs::create_dir_all(&work).expect("work dir");
    let origin = root.join(format!("{}.origin.git", name));
    git(root, &["init", "--bare", origin.to_str().unwrap()]);
    git(&work, &["init", "-b", "main"]);
    git(&work, &["config", "user.email", "n976@test"]);
    git(&work, &["config", "user.name", "n976"]);
    git(
        &work,
        &["remote", "add", "origin", origin.to_str().unwrap()],
    );
    fs::write(work.join("f.txt"), "base\n").expect("write");
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-q", "-m", "c0"]);
    git(&work, &["push", "-q", "origin", "HEAD:main"]);
    for i in 0..25 {
        fs::write(work.join("f.txt"), format!("main-{i}\n")).expect("write");
        git(&work, &["commit", "-q", "-am", &format!("c{}", i + 1)]);
    }
    git(&work, &["push", "-q", "origin", "HEAD:main"]);
    git(&work, &["checkout", "-q", "-b", "pr-branch", "main~25"]);
    work
}

fn run_script(script: &str, work: &Path) -> (i32, String) {
    let out = Command::new("bash")
        .arg("-c")
        .arg(script)
        .current_dir(work)
        .output()
        .expect("bash runs");
    (
        out.status.code().unwrap_or(-1),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

fn with_root(name: &str, f: &dyn Fn(&Path)) {
    // a UNIQUE root per test — the tests run in parallel and one fixture
    // must never see another's directory disappear underneath it.
    let root = std::env::temp_dir().join(format!("n976-fresh-base-{}-{name}", std::process::id()));
    fs::create_dir_all(&root).expect("root dir");
    f(&root);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn n976_freshness_pr_to_main_stays_red_on_staleness() {
    with_root("to-main", &|root| {
        let work = fresh_fixture(root, "to-main");
        let (code, out) = run_script(&script_for("main"), &work);
        assert_eq!(
            code, 1,
            "base=main must stay subject to the bound, got: {out}"
        );
        assert!(
            out.contains("behind main (threshold: 20)"),
            "the loud refusal: {out}"
        );
    });
}

#[test]
fn n976_freshness_pr_to_non_main_base_succeeds_with_reason() {
    with_root("to-release", &|root| {
        let work = fresh_fixture(root, "to-release");
        let (code, out) = run_script(&script_for("release/0.40.1-base"), &work);
        assert_eq!(
            code, 0,
            "base≠main must exit 0 (success, not skip), got: {out}"
        );
        assert!(
            out.contains(
                "branch-freshness: base is release/0.40.1-base, not main — \
                 merging this PR does not touch main, so the freshness bound \
                 does not apply"
            ),
            "the printed reason is the contract: {out}"
        );
        assert!(
            !out.contains("threshold: 20"),
            "the bound must not run past the base gate: {out}"
        );
    });
}

#[test]
fn n976_freshness_mutation_inverting_the_base_check_is_caught() {
    with_root("mutant", &|root| {
        let work = fresh_fixture(root, "mutant");
        let script = script_for("release/0.40.1-base").replace(
            "[ \"${BASE_REF}\" != \"main\" ]",
            "[ \"${BASE_REF}\" == \"main\" ]",
        );
        let (code, out) = run_script(&script, &work);
        assert_eq!(
            code, 1,
            "the mutant (the inverted base check) must turn the second side \
             red — the exception is load-bearing: {out}"
        );
    });
}

#[test]
fn n976_freshness_exception_is_in_the_live_workflow() {
    // the same file-level pin the release path depends on: the exception
    // lives in ci.yml itself (a copy in a test proves nothing).
    let ci = fs::read_to_string(".github/workflows/ci.yml").expect("ci.yml readable");
    assert!(
        ci.contains("branch-freshness: base is ${BASE_REF}, not main"),
        "the exception line must stay in the live ci.yml job"
    );
}

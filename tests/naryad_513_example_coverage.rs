//! №513 (Wave 21, dispatch gh#804) — the audit 28.09 C-09: 45 examples had
//! neither a sidecar nor any mention in tests/CI, 35 of them were
//! unverifiable by construction. This naryad closes the hole TWO ways:
//!
//!   1. COVERAGE: every previously-unchecked example is now either
//!      checked (a golden sidecar, an mlog-test run, a serve golden, an
//!      explicit run here) or honestly COMPAT-513-tagged (stale grammar
//!      prototypes are NOT checked with invented behavior — the rewrite
//!      is a separate naryad). The inventory invariant below is the
//!      permanent fence: a NEW example without any check fails CI.
//!   2. DEBT COUNTER: scripts/ci/debt_counters.py grows the
//!      `example_uncovered` counter (sidecar-less, mention-less, COMPAT-less
//!      .mlog files) with the checked-in baseline — the counter moves ONLY
//!      DOWN (the №468 hygiene rule).
//!
//! Red-before/green-after: on the pre-№513 tree the inventory invariant
//! fails with the 35 uncovered names (the audit's exact count).

#![allow(clippy::disallowed_methods)] // the harness reads examples/ and shells the mlog binary

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

fn manifest_dir() -> PathBuf {
    std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| ".".to_string())
        .into()
}

fn read(name: &str) -> String {
    fs::read_to_string(Path::new("examples").join(name))
        .unwrap_or_else(|e| panic!("cannot read examples/{}: {}", name, e))
}

// ── 1. The inventory invariant ─────────────────────────────────────────
//
// Every examples/*.mlog must be covered by ONE of:
//   • a golden sidecar (.expected / .error — the golden.rs suites run it);
//   • a mention in tests/, benches/, scripts/, .github/ (a named CI check);
//   • a COMPAT-N tag (an honest removal, the reason recorded in the file).
// Otherwise the example is unverifiable — this test fails (audit C-09).

#[test]
fn all_examples_have_a_check_or_an_honest_compat_tag() {
    let examples_dir = manifest_dir().join("examples");
    let repo = manifest_dir();

    // Collect the searchable repo text ONCE (tests/benches/scripts/.github).
    let mut haystack = String::new();
    for tree in ["tests", "benches", "scripts", ".github"] {
        let root = repo.join(tree);
        let mut stack = vec![root];
        while let Some(dir) = stack.pop() {
            let entries = match fs::read_dir(&dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    // do not descend into build artifacts
                    if p.file_name().map(|n| n != "target").unwrap_or(true) {
                        stack.push(p);
                    }
                    continue;
                }
                if p.extension()
                    .map(|e| {
                        e == "rs"
                            || e == "yml"
                            || e == "yaml"
                            || e == "py"
                            || e == "txt"
                            || e == "toml"
                    })
                    .unwrap_or(false)
                {
                    if let Ok(s) = fs::read_to_string(&p) {
                        haystack.push_str(&s);
                        haystack.push('\n');
                    }
                }
            }
        }
    }

    let mut mlogs: Vec<PathBuf> = fs::read_dir(&examples_dir)
        .expect("examples/ must exist")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "mlog").unwrap_or(false))
        .collect();
    mlogs.sort();

    assert!(
        mlogs.len() >= 244,
        "the live example corpus must not silently shrink (244 after the №533 compat move: 259 − 14 stale-syntax examples archived to examples/compat/, got {})",
        mlogs.len()
    );
    // The archive exists and holds the moved corpus.
    let compat_dir = examples_dir.join("compat");
    let compat_count = fs::read_dir(&compat_dir)
        .expect("examples/compat/ must exist (№533)")
        .flatten()
        .filter(|e| e.path().extension().map(|e| e == "mlog").unwrap_or(false))
        .count();
    assert_eq!(
        compat_count, 14,
        "examples/compat/ holds exactly the 14 №513-tagged stale-syntax examples"
    );

    let mut uncovered: Vec<String> = Vec::new();
    for path in &mlogs {
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();
        let source = fs::read_to_string(path).unwrap_or_default();
        // №533: the path rule — a live (top-level) example must NOT carry a
        // COMPAT tag at all; the stale-syntax examples live in
        // examples/compat/ (the archive), which the walker never enters.
        assert!(
            !source.lines().take(5).any(|l| l.contains("COMPAT-513")),
            "the top-level example '{}' is COMPAT-tagged — move it to examples/compat/ (№533)",
            stem
        );
        let has_sidecar =
            path.with_extension("expected").exists() || path.with_extension("error").exists();
        let mentioned = haystack.contains(&stem);
        if !has_sidecar && !mentioned {
            uncovered.push(stem);
        }
    }

    assert!(
        uncovered.is_empty(),
        "{} example(s) are unverifiable (no sidecar, no mention, no COMPAT-513 tag) — the audit C-09 hole reopened:\n  - {}",
        uncovered.len(),
        uncovered.join("\n  - ")
    );
}

// ── 2. p7_cyrillic — a golden contract the golden.rs suite skips ───────
//
// golden.rs collect_pairs excludes p7_* (Наряд №49: p7 examples needed
// env vars / a live server back then). p7_cyrillic is pure string
// processing (len/to_string on Cyrillic literals) — fully deterministic,
// no env. The exclusion must not leave it unchecked (the audit C-09
// found exactly such gaps).

#[test]
fn p7_cyrillic_golden_contract() {
    let expected = read("p7_cyrillic.expected");
    let source = read("p7_cyrillic.mlog");
    let (tx, rx) = std::sync::mpsc::channel();
    let src = source.clone();
    std::thread::spawn(move || {
        let _ = tx.send(metalogos::run_program(&src));
    });
    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(Ok(decls)) => {
            let actual = decls.unwrap_or_default();
            assert_eq!(
                actual.trim_end(),
                expected.trim_end(),
                "p7_cyrillic golden mismatch"
            );
        }
        Ok(Err(e)) => panic!("p7_cyrillic must run cleanly, got: {}", e),
        Err(_) => panic!("p7_cyrillic timed out"),
    }
}

// ── 3. p120_* — the `mlog test` contract family ────────────────────────
//
// These examples demonstrate the test-runner CLI itself (Наряд №120):
// passing must exit 0; failing must exit non-zero; multiple must report
// EVERY failure (none stops on first failure); filter must accept a
// --filter run. Verified through the REAL binary (CARGO_BIN_EXE).

fn run_mlog(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_mlog"))
        .args(args)
        .output()
        .expect("the mlog binary must be runnable");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn p120_test_passing_exits_zero() {
    let (code, stdout, _) = run_mlog(&["test", "examples/p120_test_passing.mlog"]);
    assert_eq!(code, 0, "p120_test_passing must pass; stdout: {}", stdout);
    assert!(
        stdout.contains("Shout uppercases and adds exclamation"),
        "the passing test name must be reported; stdout: {}",
        stdout
    );
}

#[test]
fn p120_test_failing_exits_nonzero() {
    let (code, _, _) = run_mlog(&["test", "examples/p120_test_failing.mlog"]);
    assert_ne!(
        code, 0,
        "p120_test_failing demonstrates a FAILING test — a non-zero exit IS its contract"
    );
}

#[test]
fn p120_test_multiple_reports_every_failure() {
    let (code, stdout, _) = run_mlog(&["test", "examples/p120_test_multiple.mlog"]);
    assert_ne!(code, 0, "2 of the 4 tests are wrong on purpose");
    // Contract 3 (№120): none stops on first failure — both wrong tests
    // are reported in one run.
    assert!(
        stdout.contains("addition is not multiplication") && stdout.contains("zero identity"),
        "the runner must report BOTH failures; stdout: {}",
        stdout
    );
}

#[test]
fn p120_test_filter_runs_the_selected_test_only() {
    let (code, stdout, _) = run_mlog(&[
        "test",
        "examples/p120_test_filter.mlog",
        "--filter",
        "double of two",
    ]);
    assert_eq!(code, 0, "the filtered test is green; stdout: {}", stdout);
    assert!(
        stdout.contains("double of two is four"),
        "the selected test must run; stdout: {}",
        stdout
    );
    assert!(
        !stdout.contains("unrelated: add works"),
        "--filter must NOT run unrelated tests; stdout: {}",
        stdout
    );
}

// ── 4. Serve goldens (MockLlm, random ports — the №496 harness) ────────

async fn http_get(port: u16, path: &str) -> (u16, String) {
    let r = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{}{}", port, path))
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .expect("GET should connect");
    let s = r.status().as_u16();
    (s, r.text().await.unwrap_or_default())
}

async fn http_post_json(port: u16, path: &str, body: &str) -> (u16, String) {
    let r = reqwest::Client::new()
        .post(format!("http://127.0.0.1:{}{}", port, path))
        .header("content-type", "application/json")
        .body(body.to_string())
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .expect("POST should connect");
    let s = r.status().as_u16();
    (s, r.text().await.unwrap_or_default())
}

/// p31_concurrency: the /fast route must answer without any external
/// dependency. (The /slow route reaches httpbin.org by design — the demo
/// of a slow upstream — and is deliberately NOT exercised in CI; the
/// external call is the example's point, not a checkable contract.)
#[tokio::test]
#[cfg(feature = "server")]
async fn p31_concurrency_fast_route_golden() {
    std::env::set_var("METALOGOS_MOCK_LLM", "1");
    let source = read("p31_concurrency.mlog");
    let (port, handle) = metalogos::server::run_test_server(&source)
        .await
        .expect("p31_concurrency must boot");
    let (status, body) = http_get(port, "/fast").await;
    handle.abort();
    assert_eq!(status, 200);
    assert_eq!(body, "fast-done");
}

/// p51_concurrency: same /fast contract (the /slow route targets a
/// companion server on a fixed port 10098 — not reproducible on the
/// random-port harness; not exercised).
#[tokio::test]
#[cfg(feature = "server")]
async fn p51_concurrency_fast_route_golden() {
    std::env::set_var("METALOGOS_MOCK_LLM", "1");
    let source = read("p51_concurrency.mlog");
    let (port, handle) = metalogos::server::run_test_server(&source)
        .await
        .expect("p51_concurrency must boot");
    let (status, body) = http_get(port, "/fast").await;
    handle.abort();
    assert_eq!(status, 200);
    assert_eq!(body, "fast-done");
}

/// vm_serve_realistic_dept_{a,b,c} are ONE-LINE fragments (a single
/// pattern each) — they are consumed by EMBEDDING into a server (the
/// №502 VM-serve posture), so the honest check composes each fragment
/// into a mlogserver wrapper and runs it on the VM backend (their name
/// is the contract: these fragments exist for the VM serve path).
#[tokio::test]
#[cfg(feature = "server")]
async fn vm_serve_realistic_dept_fragments_compose_and_answer() {
    std::env::set_var("METALOGOS_MOCK_LLM", "1");
    // Named explicitly: the coverage inventory (debt_counters №513) is
    // text-based — every checked example must appear by name.
    let fragments = [
        ("a", "vm_serve_realistic_dept_a.mlog"),
        ("b", "vm_serve_realistic_dept_b.mlog"),
        ("c", "vm_serve_realistic_dept_c.mlog"),
    ];
    for (dept, file) in fragments {
        let fragment = read(file);
        let composed = format!(
            "mlogserver {{\n  port: 0\n  route \"/ask\" method=POST {{\n    let q = json_body()\n    let r = HandleDept{}(q.text)\n    respond(\"200\", r)\n  }}\n}}\n{}",
            dept.to_uppercase(),
            fragment
        );
        let (port, handle) = metalogos::server::run_test_server_with_backend(
            &composed,
            metalogos::server::ServeBackend::Vm,
        )
        .await
        .unwrap_or_else(|e| panic!("{} must boot on the VM backend: {}", file, e));
        let (status, body) = http_post_json(port, "/ask", r#"{"text":"ping"}"#).await;
        handle.abort();
        assert_eq!(status, 200, "{}", file);
        assert_eq!(body, format!("dept-{}: ping", dept), "{}", file);
    }
}

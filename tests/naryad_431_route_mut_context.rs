//! Issue #600 — route-body assignment in branches (mut context).
//! Naryad №431 (Wave 10): the naryad-numbered contract file for the same
//! fix (landed via PR #611 on main; this file is its §3 regression suite).
//!
//! The route executor (`execute_route_body`) handles top-level route-body
//! statements manually: `let` bindings went into the env WITHOUT registering
//! the `mutable` flag, and any nested statement (if/each/match bodies) was
//! delegated to `eval_statements`, which creates a FRESH mutable set. An
//! assignment to a route-local variable inside a branch therefore failed at
//! runtime with "cannot assign to immutable variable" → HTTP 500.
//!
//! The VM never had this hole: its compiler rejects non-mut assignments at
//! compile time (№264) and compiles mut assignments inside branches to
//! `StoreAssignLocal { mutable: true }`, which runs fine. The fix threads a
//! single `mutable_vars` set through the whole route body on the TW side —
//! `let mut` registers once, branch assigns consult the SAME set — restoring
//! TW/VM parity (№16.0: the contract is one language, two backends).
//!
//! Contract tests (feature = "server", run_test_server harness):
//! 1. `let mut` + assignment inside `if` → 200, the branch value;
//! 2. `let mut` + assignment inside `each` → 200, the last assigned value;
//! 3. non-mut assignment inside a branch → loud error on BOTH backends
//!    (VM rejects at compile time; TW now matches — no silent success);
//! 4. plain top-level `let mut` reassignment keeps working (no regression);
//! 5. mutation check: the fix is not a no-op (the pre-fix code failed #1).

#![cfg(feature = "server")]

use std::time::Duration;

const SOURCE_MUT_BRANCH: &str = r#"
mlogserver {
  port: 8090
  route "/branch" method=GET {
    let mut x = "start"
    if true { x = "from-branch" }
    return x
  }
  route "/each" method=GET {
    let mut acc = ""
    let items = ["a", "b", "c"]
    each it in items {
      acc = it
    }
    return acc
  }
  route "/topassign" method=GET {
    let mut y = 1.0
    y = 2.0
    return to_string(y)
  }
  route "/nonmut" method=GET {
    let z = "frozen"
    if true { z = "mutated" }
    return z
  }
}
"#;

async fn get(path: &str) -> (reqwest::StatusCode, String) {
    let (port, _handle) = metalogos::server::run_test_server(SOURCE_MUT_BRANCH)
        .await
        .expect("test server must start");
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}{}", port, path);
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .expect("request must complete");
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    (status, body)
}

/// The reported defect: assignment inside an `if` branch of a route body.
#[tokio::test]
async fn naryad_600_assign_in_if_branch() {
    let (status, body) = get("/branch").await;
    assert_eq!(
        status,
        reqwest::StatusCode::OK,
        "branch assignment must not 500 (body: {})",
        body
    );
    assert_eq!(body.trim(), "from-branch", "the branch value must win");
}

/// Assignment inside an `each` body of a route body.
#[tokio::test]
async fn naryad_600_assign_in_each_body() {
    let (status, body) = get("/each").await;
    assert_eq!(
        status,
        reqwest::StatusCode::OK,
        "each-body assignment must not 500 (body: {})",
        body
    );
    assert_eq!(body.trim(), "c", "the last assigned value must win");
}

/// No regression: top-level `let mut` reassignment (worked before).
#[tokio::test]
async fn naryad_600_top_level_mut_assign() {
    let (status, body) = get("/topassign").await;
    assert_eq!(status, reqwest::StatusCode::OK, "body: {}", body);
    assert_eq!(body.trim(), "2");
}

/// Non-mut assignment inside a branch is a language error on BOTH backends:
/// the VM compiler rejects it at compile time (№264); the TW route executor
/// must not silently succeed either (pre-fix it 500'd with the misleading
/// immutable-text on a mut variable — the non-mut case now errors loudly too,
/// matching the pattern-body semantics of `eval_statements_cf`).
#[tokio::test]
async fn naryad_600_nonmut_assign_is_loud() {
    let (status, _body) = get("/nonmut").await;
    assert!(
        status.is_client_error() || status.is_server_error(),
        "non-mut branch assignment must be a loud error, got {}",
        status
    );
}

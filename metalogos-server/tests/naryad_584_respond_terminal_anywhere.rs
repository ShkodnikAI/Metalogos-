// ── НАРЯД №584 (gh#998, the audit d63cc1d X-1 step 2) ──
// The VM compiles a bare respond* statement as TERMINAL on the TW
// early-answer surface: the route-body compiler (`compile_routes`) lowers
// it to `return respond*(...)` — the route body's direct statements, the
// IfThen/Match/cycle bodies (any depth; cycles answer on the FIRST
// iteration), and the DIRECT statements of top-level if/else branches.
// (The TW serve lane is the etalon and does not change: under a top-level
// block-form if, a response carried by a NESTED statement is discarded and
// the branch continues — the VM mirrors that form-specific surface.)
// The №581 RESPOND_NOT_TERMINAL semantic gate is retired to a style
// advisory: the bare form WORKS on both backends, `return respond(...)`
// stays the RECOMMENDED explicit shape.
//
// Blocks:
//   A — the audit guard WITHOUT return now works on the VM: non-admin gets
//       403, the protected code does NOT run (the per-test kv marker stays
//       empty); the admin path still runs it. BOTH backends, real HTTP.
//   B — the surfaces: match-statement arms, while/each FIRST iteration,
//       guard chains, respond_html mid-route, and the depth-2 swallow
//       (a nested respond does NOT stop the route — BOTH backends agree) —
//       (status, body) EQUAL on both backends.
//   C — the epilogue invariant (№582) survives the lowering: a let-tail
//       route after a non-taken guard answers the shared 200 OK default
//       (the route exit NEVER reads a local slot).
//   D — the semantic advisory: the bare form yields a WARNING (the stable
//       RespondNotTerminal kind + the return-form hint) and ZERO errors;
//       the run path no longer refuses on the class.
#![cfg(feature = "server")]
#![allow(clippy::disallowed_methods)]

use metalogos_server::server::ServeBackend;

async fn start(source: &str, backend: ServeBackend) -> u16 {
    let (port, _handle) = metalogos_server::server::run_test_server_with_backend(source, backend)
        .await
        .expect("test server should start");
    port
}

async fn http_get(port: u16, path: &str) -> (u16, String) {
    let url = format!("http://127.0.0.1:{}{}", port, path);
    let resp = reqwest::get(&url)
        .await
        .expect("GET request should succeed");
    let status = resp.status().as_u16();
    let body = resp.text().await.expect("response body should be readable");
    (status, body)
}

async fn http_post(port: u16, path: &str, query: &str) -> (u16, String) {
    let url = format!("http://127.0.0.1:{}{}?{}", port, path, query);
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .send()
        .await
        .expect("POST request should succeed");
    let status = resp.status().as_u16();
    let body = resp.text().await.expect("response body should be readable");
    (status, body)
}

// ── A: the audit guard WITHOUT return — the exact X-1 shape, now safe ──

// The kv marker key is UNIQUE PER TEST: kv_store is process-global and the
// tokio tests of one binary share the process — a shared key would let the
// admin-path request of one test set the marker another test reads.
fn guard_bare_source(marker: &str) -> String {
    format!(
        r#"
db {{
  url: "sqlite::memory:"
}}

mlogserver {{
  port: 8094
  route "/purge" method=POST {{
    let user = query_param("user")
    // №327: untrusted data must not DECIDE — the one-way hash restores
    // trust (the same shape the №581 migrated guard used, MINUS the
    // `return` — the bare form is the X-1 shape under test).
    let key = redact(user, "hash_only")
    let expected = redact("admin", "hash_only")
    if key != expected {{
      respond("403", "forbidden")
    }}
    kv_set("{marker}", "1")
    respond("200", "purged")
  }}
  route "/purge_ran" method=GET {{
    respond("200", kv_get("{marker}"))
  }}
}}
"#
    )
}

#[tokio::test]
async fn n584_bare_guard_vm_non_admin_403_and_protected_code_skipped() {
    let src = guard_bare_source("purge_vm_guest");
    let port = start(&src, ServeBackend::Vm).await;
    let (status, body) = http_post(port, "/purge", "user=guest").await;
    assert_eq!(
        status, 403,
        "VM: the bare guard must answer 403 for a non-admin (the terminal lowering)"
    );
    assert_eq!(body, "forbidden", "VM: the guard answer body");
    // The protected code did NOT run: the kv marker was never set.
    let (marker_status, marker) = http_get(port, "/purge_ran").await;
    assert_eq!(marker_status, 200);
    assert_eq!(
        marker, "",
        "VM: the protected code must NOT have run (the kv marker stays empty)"
    );
}

#[tokio::test]
async fn n584_bare_guard_interpreter_non_admin_403_and_protected_code_skipped() {
    let src = guard_bare_source("purge_tw_guest");
    let port = start(&src, ServeBackend::Interpreter).await;
    let (status, body) = http_post(port, "/purge", "user=guest").await;
    assert_eq!(status, 403, "TW: the bare guard must answer 403");
    assert_eq!(body, "forbidden", "TW: the guard answer body");
    let (marker_status, marker) = http_get(port, "/purge_ran").await;
    assert_eq!(marker_status, 200);
    assert_eq!(marker, "", "TW: the protected code must NOT have run");
}

#[tokio::test]
async fn n584_bare_guard_vm_admin_path_runs() {
    let src = guard_bare_source("purge_vm_admin");
    let port = start(&src, ServeBackend::Vm).await;
    let (status, body) = http_post(port, "/purge", "user=admin").await;
    assert_eq!(status, 200, "VM: the admin path must serve");
    assert_eq!(
        body, "purged",
        "VM: the admin path must run the protected code"
    );
    let (_, marker) = http_get(port, "/purge_ran").await;
    assert_eq!(marker, "1", "VM: the protected code ran for the admin");
}

#[tokio::test]
async fn n584_bare_guard_interpreter_admin_path_runs() {
    let src = guard_bare_source("purge_tw_admin");
    let port = start(&src, ServeBackend::Interpreter).await;
    let (status, body) = http_post(port, "/purge", "user=admin").await;
    assert_eq!(status, 200, "TW: the admin path must serve");
    assert_eq!(
        body, "purged",
        "TW: the admin path must run the protected code"
    );
    let (_, marker) = http_get(port, "/purge_ran").await;
    assert_eq!(marker, "1", "TW: the protected code ran for the admin");
}

// ── B: the surfaces — (status, body) equality on BOTH backends ───────

const NESTED_SHAPES: &str = r#"
mlogserver {
  port: 8094
  // DEPTH-2 swallow (the honest TW surface, mirrored by the VM): a bare
  // respond carried by a statement NESTED under a top-level block-form if
  // does NOT stop the route on EITHER backend — the branch continues.
  // Only the branch's DIRECT responds answer (who=boss skips the inner if
  // and answers "inner ok boss" from the direct respond).
  route "/nested_if" method=GET {
    let who = query_param("who")
    if who != "" {
      if who != "boss" {
        respond("403", "inner no")
      }
      respond("200", "inner ok " + who)
    }
    respond("200", "outer fall")
  }
  // match statement mid-route: the matched arm's bare respond answers
  // (the Match surface propagates from any depth).
  route "/match_mid" method=GET {
    let who = query_param("who")
    match who {
      "boss" then { respond("200", "hello boss") }
      "guest" then { respond("403", "no guest") }
      else { respond("200", "default") }
    }
    respond("200", "after match")
  }
  // while: the FIRST iteration answers (the TW first-iteration parity).
  route "/while_first" method=GET {
    let n = 0
    while n < 5 {
      respond("200", "left at " + to_string(n))
    }
    respond("200", "unreachable")
  }
  // each: the FIRST item answers.
  route "/each_first" method=GET {
    each item in ["a", "b", "c"] {
      respond("200", "item " + item)
    }
    respond("200", "unreachable")
  }
  // guard chain: the SECOND guard fires.
  route "/chain" method=GET {
    let who = query_param("who")
    if who == "boss" {
      respond("200", "boss here")
    }
    if who == "guest" {
      respond("403", "chain no")
    }
    respond("200", "chain end")
  }
  // respond_html family mid-route (a DIRECT branch respond answers).
  route "/html_mid" method=GET {
    let who = query_param("who")
    if who == "guest" {
      respond_html("<p>no</p>")
    }
    respond("200", "html end")
  }
}
"#;

#[tokio::test]
async fn n584_nested_shapes_vm() {
    let port = start(NESTED_SHAPES, ServeBackend::Vm).await;
    // The depth-2 swallow: BOTH backends continue the branch and answer
    // from the DIRECT respond (the form-specific TW surface, mirrored).
    let (status, body) = http_get(port, "/nested_if?who=stranger").await;
    assert_eq!(
        status, 200,
        "VM: the depth-2 respond does NOT stop the route (the branch continues)"
    );
    assert_eq!(body, "inner ok stranger");
    let (status, body) = http_get(port, "/nested_if?who=boss").await;
    assert_eq!(status, 200, "VM: the direct branch respond answers");
    assert_eq!(body, "inner ok boss");
    let (status, body) = http_get(port, "/nested_if?who=").await;
    assert_eq!(status, 200, "VM: the outer fall-through answers");
    assert_eq!(body, "outer fall");

    let (status, body) = http_get(port, "/match_mid?who=guest").await;
    assert_eq!(status, 403, "VM: the matched arm's bare respond answers");
    assert_eq!(body, "no guest");
    let (status, body) = http_get(port, "/match_mid?who=other").await;
    assert_eq!(status, 200);
    assert_eq!(body, "default", "VM: the match else-arm answers");

    let (status, body) = http_get(port, "/while_first").await;
    assert_eq!(status, 200, "VM: the while answers on the FIRST iteration");
    assert_eq!(body, "left at 0");

    let (status, body) = http_get(port, "/each_first").await;
    assert_eq!(status, 200, "VM: the each answers on the FIRST item");
    assert_eq!(body, "item a");

    let (status, body) = http_get(port, "/chain?who=guest").await;
    assert_eq!(status, 403, "VM: the second guard fires");
    assert_eq!(body, "chain no");
    let (status, body) = http_get(port, "/chain?who=boss").await;
    assert_eq!(status, 200);
    assert_eq!(body, "boss here");
    let (status, body) = http_get(port, "/chain?who=nobody").await;
    assert_eq!(status, 200);
    assert_eq!(body, "chain end");

    let (status, body) = http_get(port, "/html_mid?who=guest").await;
    assert_eq!(status, 200, "VM: respond_html mid-route answers");
    assert_eq!(body, "<p>no</p>");
    let (status, body) = http_get(port, "/html_mid?who=boss").await;
    assert_eq!(status, 200);
    assert_eq!(body, "html end");
}

#[tokio::test]
async fn n584_nested_shapes_cross_backend_equality() {
    // The TW/VM cross-check extended with mid-route forms (the №585 corpus
    // rides the same shapes): every (status, body) pair must be IDENTICAL.
    let vm = start(NESTED_SHAPES, ServeBackend::Vm).await;
    let tw = start(NESTED_SHAPES, ServeBackend::Interpreter).await;
    let cases = [
        "/nested_if?who=stranger",
        "/nested_if?who=boss",
        "/nested_if?who=",
        "/match_mid?who=guest",
        "/match_mid?who=boss",
        "/match_mid?who=other",
        "/while_first",
        "/each_first",
        "/chain?who=guest",
        "/chain?who=boss",
        "/chain?who=nobody",
        "/html_mid?who=guest",
        "/html_mid?who=boss",
    ];
    for case in cases {
        let (vm_status, vm_body) = http_get(vm, case).await;
        let (tw_status, tw_body) = http_get(tw, case).await;
        assert_eq!(
            (vm_status, vm_body.as_str()),
            (tw_status, tw_body.as_str()),
            "TW/VM divergence on {case}: VM ({vm_status}, {vm_body:?}) != TW ({tw_status}, {tw_body:?})"
        );
    }
}

// ── C: the №582 epilogue invariant survives the lowering ─────────────

const EPILOGUE_SHAPES: &str = r#"
mlogserver {
  port: 8094
  // A non-taken guard followed by a let TAIL: the fall-through must be the
  // shared Unit default (200 OK) — NEVER the local slot's content.
  route "/let_tail" method=GET {
    let who = query_param("who")
    if who == "guest" {
      respond("403", "no")
    }
    let row = "row-data-" + who
  }
  // A taken guard followed by a let tail: the guard still answers first.
  route "/let_tail_blocked" method=GET {
    let who = query_param("who")
    if who != "" {
      respond("403", "stopped")
    }
    let secret = "must-not-leak"
  }
}
"#;

#[tokio::test]
async fn n584_epilogue_let_tail_fall_through_vm() {
    let port = start(EPILOGUE_SHAPES, ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/let_tail?who=x").await;
    assert_eq!(status, 200, "VM: the let-tail fall-through answers 200");
    assert_eq!(
        body, "OK",
        "VM: the №582 default — NEVER the local slot (row-data-x)"
    );
    let (status, body) = http_get(port, "/let_tail_blocked?who=x").await;
    assert_eq!(status, 403, "VM: the guard answers before the let tail");
    assert_eq!(body, "stopped");
}

#[tokio::test]
async fn n584_epilogue_let_tail_fall_through_interpreter() {
    let port = start(EPILOGUE_SHAPES, ServeBackend::Interpreter).await;
    let (status, body) = http_get(port, "/let_tail?who=x").await;
    assert_eq!(status, 200, "TW: the let-tail fall-through answers 200");
    assert_eq!(body, "OK", "TW: the same shared default");
    let (status, body) = http_get(port, "/let_tail_blocked?who=x").await;
    assert_eq!(status, 403, "TW: the guard answers before the let tail");
    assert_eq!(body, "stopped");
}

// ── D: the semantic advisory — warnings, not errors ──────────────────

#[test]
fn n584_bare_form_yields_warning_with_stable_kind_and_hint() {
    use metalogos::parser;
    use metalogos::semantic::{check_program, SemanticErrorKind};
    let src = r#"
mlogserver {
  port: 8094
  route "/x" method=GET {
    let a = 1
    respond("403", "no")
    respond("200", "yes")
  }
}
"#;
    let decls: Vec<metalogos::ast::Declaration> =
        parser::parse(src).expect("the fixture must parse");
    let result = check_program(&decls);
    assert!(
        result.errors.is_empty(),
        "the bare form must NOT block anymore (№584), got: {:?}",
        result.errors
    );
    let hit = result
        .warnings
        .iter()
        .find(|w| w.kind == SemanticErrorKind::RespondNotTerminal)
        .expect("the bare mid-route respond must carry the advisory");
    assert_eq!(
        hit.kind.stable_code(),
        Some("RESPOND_NOT_TERMINAL"),
        "the stable №479 code stays machine-readable on the advisory"
    );
    assert!(
        hit.message.contains("return respond("),
        "the advisory keeps the migration hint, got: {}",
        hit.message
    );
}

#[test]
fn n584_every_respond_name_advised() {
    use metalogos::parser;
    use metalogos::semantic::{check_program, SemanticErrorKind};
    for name in [
        "respond",
        "respond_html",
        "respond_html_status",
        "respond_html_doc",
    ] {
        let src = format!(
            r#"
mlogserver {{
  port: 8094
  route "/x" method=GET {{
    let a = 1
    {name}("403", "no")
    respond("200", "yes")
  }}
}}
"#
        );
        let decls: Vec<metalogos::ast::Declaration> =
            parser::parse(&src).expect("the fixture must parse");
        let result = check_program(&decls);
        assert!(
            result.errors.is_empty(),
            "{name}: no blocking error after №584"
        );
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.kind == SemanticErrorKind::RespondNotTerminal
                    && w.message.contains("return respond(")),
            "{name}: the bare mid-route call must carry the advisory"
        );
    }
}

#[test]
fn n584_return_form_and_tail_stay_unflagged() {
    use metalogos::parser;
    use metalogos::semantic::{check_program, SemanticErrorKind};
    let src = r#"
mlogserver {
  port: 8094
  route "/ok" method=GET {
    let a = 1
    if a == 1 {
      return respond("403", "no")
    }
    respond("200", "yes")
  }
  route "/tail" method=GET {
    respond("200", "tail")
  }
}
"#;
    let decls: Vec<metalogos::ast::Declaration> =
        parser::parse(src).expect("the fixture must parse");
    let result = check_program(&decls);
    assert!(
        !result
            .warnings
            .iter()
            .any(|w| w.kind == SemanticErrorKind::RespondNotTerminal),
        "the return-form and the route-tail respond are idiomatic — no advisory"
    );
}

#[tokio::test]
async fn n584_run_path_no_longer_refuses_on_the_class() {
    // The run path shares the semantic pass (the №523 shape): before №584
    // the guard program refused with [RESPOND_NOT_TERMINAL] stamped
    // first; after №584 the class produces no blocking finding at all.
    // Whatever the run output is (routes are a serve-surface construct),
    // the RESPOND_NOT_TERMINAL code must never appear in an error.
    match metalogos::run_program(&guard_bare_source("purge_run_probe")) {
        Ok(_) => {}
        Err(err) => {
            assert!(
                !err.contains("RESPOND_NOT_TERMINAL"),
                "the run path must not refuse on the retired class, got: {err}"
            );
        }
    }
}

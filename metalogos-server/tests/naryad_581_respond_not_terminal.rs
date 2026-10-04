// ── НАРЯД №581 (gh#995, the audit d63cc1d X-1 step 1, release-block) ──
// RESPOND_NOT_TERMINAL: a bare respond* call in a NON-terminal position of
// a route body must fail the semantic pass on BOTH backends (fail-closed) —
// the TW answers immediately from any block, the VM compiles the call with
// Instruction::Pop and silently falls through to the protected code (the
// guard-pattern bypass). The sanctioned early answer is `return respond(...)`.
//
// Blocks:
//   A — the migrated audit guard: non-admin gets 403, the protected
//       db_execute DELETE never runs (BOTH backends, real HTTP).
//   B — the bare mid-route form refuses to START on BOTH backends with the
//       [RESPOND_NOT_TERMINAL] code stamped first (the №523 refusal shape).
//   C — tail legality survives: route-tail / if-tail / match-tail bare
//       responds stay LEGAL (the VM's tail chain and this gate agree), and
//       `return respond(...)` inside cycles stays legal.
//   D — every registry respond* name is covered; the semantic kind carries
//       the stable code (machine consumers read the CODE, not the prose).
#![cfg(feature = "server")]
#![allow(clippy::disallowed_methods)]

use metalogos_server::server::ServeBackend;

// ── A: the migrated audit guard, both backends ───────────────────────

const GUARD_MIGRATED: &str = r#"
db {
  url: "sqlite::memory:"
}

mlogserver {
  port: 8094
  route "/purge" method=POST {
    let user = query_param("user")
    // №327: untrusted data must not DECIDE — the one-way hash restores
    // trust (redact(x, "hash_only") is public AND trusted), so the guard
    // compares the presented key with the expected one. The shape mirrors
    // the audit's guard pattern `if is_admin(session_user()) == false`.
    let key = redact(user, "hash_only")
    let expected = redact("admin", "hash_only")
    if key != expected {
      return respond("403", "forbidden")
    }
    db_execute("CREATE TABLE IF NOT EXISTS purge_log (id INTEGER)")
    db_execute("INSERT INTO purge_log VALUES (1)")
    let g = grant_issue("db:delete:purge_log", 60, "n", 1)
    db_execute_with_grant(g, "DELETE FROM purge_log")
    respond("200", "purged")
  }
}
"#;

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

#[tokio::test]
async fn n581_guard_migrated_vm_non_admin_gets_403_no_delete() {
    let port = start(GUARD_MIGRATED, ServeBackend::Vm).await;
    let (status, body) = http_post(port, "/purge", "user=guest").await;
    assert_eq!(
        status, 403,
        "VM: the guard must answer 403 for a non-admin (the X-1 bypass is closed)"
    );
    assert!(
        !body.contains("purged"),
        "VM: the protected code must NOT have run (the DELETE never executed)"
    );
}

#[tokio::test]
async fn n581_guard_migrated_interpreter_non_admin_gets_403_no_delete() {
    let port = start(GUARD_MIGRATED, ServeBackend::Interpreter).await;
    let (status, body) = http_post(port, "/purge", "user=guest").await;
    assert_eq!(status, 403, "TW: the guard must answer 403 for a non-admin");
    assert!(
        !body.contains("purged"),
        "TW: the protected code must NOT have run"
    );
}

#[tokio::test]
async fn n581_guard_migrated_vm_admin_path_still_serves() {
    // The return-form must not break the LEGAL path: admin reaches the
    // protected code (the db_execute DELETE runs) and gets "purged".
    let port = start(GUARD_MIGRATED, ServeBackend::Vm).await;
    let (status, body) = http_post(port, "/purge", "user=admin").await;
    assert_eq!(status, 200, "VM: the admin path must serve");
    assert_eq!(body, "purged", "VM: the admin path must run the DELETE");
}

#[tokio::test]
async fn n581_guard_migrated_interpreter_admin_path_still_serves() {
    let port = start(GUARD_MIGRATED, ServeBackend::Interpreter).await;
    let (status, body) = http_post(port, "/purge", "user=admin").await;
    assert_eq!(status, 200, "TW: the admin path must serve");
    assert_eq!(body, "purged", "TW: the admin path must run the DELETE");
}

// ── B: the bare mid-route form refuses to start, both backends ───────
//
// The full startup entry (`run_server`) is the refuser — the same posture
// the №523 gate tests pin: a refused program returns Err BEFORE binding.
// (The `run_test_server_with_backend` helper is the fast harness that
// intentionally skips the semantic pass — it must NOT be used to assert
// startup refusals.)

const GUARD_BARE: &str = r#"
db {
  url: "sqlite::memory:"
}

mlogserver {
  port: 0
  route "/purge" method=POST {
    let user = query_param("user")
    let key = redact(user, "hash_only")
    let expected = redact("admin", "hash_only")
    if key != expected {
      respond("403", "forbidden")
    }
    db_execute("CREATE TABLE IF NOT EXISTS purge_log (id INTEGER)")
    db_execute("INSERT INTO purge_log VALUES (1)")
    let g = grant_issue("db:delete:purge_log", 60, "n", 1)
    db_execute_with_grant(g, "DELETE FROM purge_log")
    respond("200", "purged")
  }
}
"#;

#[tokio::test]
async fn n581_bare_mid_route_refuses_vm_startup() {
    let err = metalogos_server::server::run_server(GUARD_BARE)
        .await
        .expect_err("the bare mid-route respond must REFUSE serve startup");
    let msg = err.to_string();
    assert!(
        msg.contains("[RESPOND_NOT_TERMINAL]"),
        "the refusal must stamp the stable code first, got: {msg}"
    );
    assert!(
        msg.contains("return respond("),
        "the refusal must carry the migration hint, got: {msg}"
    );
}

#[tokio::test]
async fn n581_bare_mid_route_refuses_run_startup() {
    // The run path shares the same semantic gate (lib.rs, the №523 shape):
    // `mlog run` refuses too — ONE gate, both execution surfaces.
    let err = metalogos::run_program(GUARD_BARE)
        .expect_err("the bare mid-route respond must REFUSE the run path");
    assert!(
        err.contains("[RESPOND_NOT_TERMINAL]"),
        "the run refusal must stamp the stable code first, got: {err}"
    );
}

#[tokio::test]
async fn n581_bare_mid_route_refuses_interpreter_startup() {
    // The interpreter backend shares the SAME semantic gate — the refusal
    // happens before the backend is even chosen (fail-closed on both).
    let err = metalogos_server::server::run_server(GUARD_BARE)
        .await
        .expect_err("the bare mid-route respond must REFUSE serve startup (both backends)");
    assert!(
        err.to_string().contains("[RESPOND_NOT_TERMINAL]"),
        "the refusal must stamp the stable code first"
    );
}

// ── C: tail legality survives (the gate and the VM tail chain agree) ──

const TAIL_LEGAL: &str = r#"
mlogserver {
  port: 8094
  route "/tail" method=GET {
    let who = query_param("who")
    respond("200", "tail " + who)
  }
  route "/if_tail" method=GET {
    let who = query_param("who")
    if who != "boss" {
      respond("403", "no")
    }
  }
  route "/match_tail" method=GET {
    let who = query_param("who")
    match who {
      "boss" then { respond("200", "hello boss") }
      else { respond("403", "no") }
    }
  }
  route "/cycle_return" method=GET {
    let n = 0
    while n < 3 {
      return respond("200", "left the cycle")
    }
    respond("200", "unreachable")
  }
}
"#;

#[tokio::test]
async fn n581_tail_forms_start_and_serve_vm() {
    let port = start(TAIL_LEGAL, ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/tail?who=x").await;
    assert_eq!(status, 200);
    assert_eq!(body, "tail x");
    // The if-tail bare respond: a taken branch answers 403 (the VM's
    // branch-tail keep carries the HttpResponse).
    let resp = reqwest::get(format!("http://127.0.0.1:{}/if_tail?who=guest", port))
        .await
        .expect("if_tail route must serve");
    assert_eq!(
        resp.status().as_u16(),
        403,
        "VM: the if-tail respond answers"
    );
    // The match-tail bare respond: the matched arm answers.
    let resp = reqwest::get(format!("http://127.0.0.1:{}/match_tail?who=boss", port))
        .await
        .expect("match_tail route must serve");
    assert_eq!(
        resp.status().as_u16(),
        200,
        "VM: the match-tail respond answers"
    );
    // `return respond(...)` inside a while body: the sanctioned early answer.
    let resp = reqwest::get(format!("http://127.0.0.1:{}/cycle_return", port))
        .await
        .expect("cycle_return route must serve");
    assert_eq!(resp.status().as_u16(), 200, "VM: return-in-cycle answers");
}

// The reqwest::Response::text call stays inline in the async tests — no
// extra runtime helper is needed.

#[tokio::test]
async fn n581_tail_forms_start_and_serve_interpreter() {
    let port = start(TAIL_LEGAL, ServeBackend::Interpreter).await;
    let resp = reqwest::get(format!("http://127.0.0.1:{}/if_tail?who=guest", port))
        .await
        .expect("if_tail route must serve (TW)");
    assert_eq!(
        resp.status().as_u16(),
        403,
        "TW: the if-tail respond answers"
    );
    let resp = reqwest::get(format!("http://127.0.0.1:{}/match_tail?who=boss", port))
        .await
        .expect("match_tail route must serve (TW)");
    assert_eq!(
        resp.status().as_u16(),
        200,
        "TW: the match-tail respond answers"
    );
}

// ── D: the whole respond* family + the stable code on the kind ───────

#[test]
fn n581_every_respond_name_flagged_by_the_semantic_pass() {
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
            result
                .errors
                .iter()
                .any(|e| e.kind == SemanticErrorKind::RespondNotTerminal
                    && e.message.contains("return respond(")),
            "{name}: the bare mid-route call must carry the RespondNotTerminal kind"
        );
    }
}

#[test]
fn n581_stable_code_is_stamped() {
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
    let hit = result
        .errors
        .iter()
        .find(|e| e.kind == SemanticErrorKind::RespondNotTerminal)
        .expect("the bare mid-route respond must be flagged");
    assert_eq!(
        hit.kind.stable_code(),
        Some("RESPOND_NOT_TERMINAL"),
        "the stable №479 code must be machine-readable"
    );
}

#[test]
fn n581_return_form_and_tail_are_never_flagged() {
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
}
"#;
    let decls: Vec<metalogos::ast::Declaration> =
        parser::parse(src).expect("the fixture must parse");
    let result = check_program(&decls);
    assert!(
        !result
            .errors
            .iter()
            .any(|e| e.kind == SemanticErrorKind::RespondNotTerminal),
        "the return-form and the route-tail respond are the sanctioned shapes"
    );
}

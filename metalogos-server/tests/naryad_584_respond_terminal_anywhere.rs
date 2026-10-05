// ── НАРЯД №584 (gh#998, the audit d63cc1d X-1 step 2) + №600/№601 ──────
// (the audit 25b375e Y-1, High, release-block — Волна 31)
// The VM compiles a bare respond* statement as TERMINAL on the TW
// early-answer surface: the route-body compiler (`compile_routes`) lowers
// it to `return respond*(...)` — the route body's direct statements, the
// IfThen/Match/cycle bodies (any depth; cycles answer on the FIRST
// iteration), and the DIRECT statements of top-level if/else branches.
// (The TW serve lane is the etalon and does not change: under a top-level
// block-form if, a response carried by a NESTED statement is discarded and
// the branch continues — the VM mirrors that form-specific surface.)
// The №581 RESPOND_NOT_TERMINAL semantic gate stays retired to a style
// advisory for the WORKING surface: the bare form WORKS there on both
// backends, `return respond(...)` stays the RECOMMENDED explicit shape.
//
// №600 RESTORED the fail-closed refusal for the ONE form the lowering does
// NOT reach: a bare respond* carried by a statement NESTED under a
// top-level block-form if/else branch (depth ≥ 2). The serve lane
// discards nested-statement responses on BOTH backends — №584 had retired
// the №581 error for ALL forms at once, including this non-working one
// (the audit's guard-bypass regression: the protected DELETE executed for
// a non-admin under a style-only warning). The finding is the BLOCKING
// RESPOND_SWALLOWED error again (run/serve refuse at startup), the SSOT
// position predicate `semantic::RespondPosition` classifies the sites for
// BOTH the semantic walk and the lowering.
//
// Blocks:
//   A — the audit guard WITHOUT return at the TOP-LEVEL if now works as
//       before (a DIRECT branch statement answers): non-admin gets 403,
//       the protected code does NOT run (the per-test kv marker stays
//       empty); the admin path still runs it. BOTH backends, real HTTP.
//   B — the surfaces: match-statement arms, while/each FIRST iteration,
//       guard chains, respond_html mid-route — (status, body) EQUAL on
//       both backends. (The depth-2 swallow moved to block E — it REFUSES
//       to start since №600, it is no longer a serving shape.)
//   C — the epilogue invariant (№582) survives the lowering: a let-tail
//       route after a non-taken guard answers the shared 200 OK default
//       (the route exit NEVER reads a local slot).
//   D — the semantic advisory: the bare form on the surface yields a
//       WARNING (the stable RespondNotTerminal kind + the return-form
//       hint) and ZERO errors; the run path does not refuse on the class.
//   E — №600/№601: the swallowed shapes REFUSE AT STARTUP on BOTH
//       backends with the stable [RESPOND_SWALLOWED] code stamped first
//       (the №523 refusal shape) — the audit §3 guard scenario (the 403
//       guard at depth 2 inside a session guard) included; the one-word
//       migration (`return respond(...)`) of the SAME route compiles and
//       answers correctly (403 to a non-admin BEFORE the protected
//       DELETE, 200 + the marker to the admin); the no-false-positive
//       probes (a branch with nested statements but no respond; a respond
//       consumed in a let value) start and serve untouched.
#![cfg(feature = "server")]
#![allow(clippy::disallowed_methods)]

use metalogos_server::server::ServeBackend;

async fn start(source: &str, backend: ServeBackend) -> u16 {
    let (port, _handle) = metalogos_server::server::run_test_server_with_backend(source, backend)
        .await
        .expect("test server should start");
    port
}

/// №600: the swallowed shapes must REFUSE the startup — the Err message
/// carries the stable code (the №523 refusal shape stamps the FIRST
/// blocking finding's stable code at position 0). `label` names the
/// fixture so a failure points at the exact shape. The refusal goes
/// through the PRODUCTION serve entry (`run_server` — the №523 semantic
/// gate lives there; the run_test_server harness skips the gate by
/// design, the №523 suite pins the same layering).
async fn assert_startup_refused(source: &str, _backend: ServeBackend, label: &str) {
    let outcome = metalogos_server::server::run_server(source).await;
    match outcome {
        Ok(_) => panic!(
            "{label}: the swallowed respond* must REFUSE the startup \
             (RESPOND_SWALLOWED), but the server started"
        ),
        Err(err) => {
            let msg = err.to_string();
            assert!(
                msg.contains("RESPOND_SWALLOWED"),
                "{label}: the refusal must stamp the stable RESPOND_SWALLOWED \
                 code, got: {msg}"
            );
        }
    }
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

// ── A: the audit guard WITHOUT return at the TOP-LEVEL if — still safe ──

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
    // `return` — the bare form is the X-1 shape under test; the guard if
    // is TOP-LEVEL, so the bare respond is a DIRECT branch statement —
    // the early-answer surface).
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
// №601: the depth-2 swallow LEFT this fixture — since №600 it refuses the
// startup (block E), it is no longer a serving shape.

const SURFACE_SHAPES: &str = r#"
mlogserver {
  port: 8094
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
  // №601 no-false-positive probe: a top-level block-if branch carrying
  // NESTED statements (an inner if, a cycle) but NO respond at all —
  // the №600 predicate must not manufacture a refusal here.
  route "/branch_no_respond" method=GET {
    let who = query_param("who")
    if who != "" {
      if who == "boss" {
        let salute = "hello " + who
      }
      each item in ["1", "2"] {
        let acc = item
      }
    }
    respond("200", "clean " + who)
  }
}
"#;

#[tokio::test]
async fn n584_surface_shapes_vm() {
    let port = start(SURFACE_SHAPES, ServeBackend::Vm).await;

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

    let (status, body) = http_get(port, "/branch_no_respond?who=boss").await;
    assert_eq!(
        status, 200,
        "VM: the nested-statement branch without respond starts and serves (no false positive)"
    );
    assert_eq!(body, "clean boss");
}

#[tokio::test]
async fn n584_surface_shapes_cross_backend_equality() {
    // The TW/VM cross-check extended with mid-route forms (the №585 corpus
    // rides the same shapes): every (status, body) pair must be IDENTICAL.
    let vm = start(SURFACE_SHAPES, ServeBackend::Vm).await;
    let tw = start(SURFACE_SHAPES, ServeBackend::Interpreter).await;
    let cases = [
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
        "/branch_no_respond?who=boss",
        "/branch_no_respond?who=",
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

// ── D: the semantic advisory — warnings, not errors (the surface) ────

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
        "the bare form must NOT block on the surface (№584), got: {:?}",
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
    // The run path shares the semantic pass (the №523 shape): the surface
    // class produces no blocking finding at all. Whatever the run output
    // is (routes are a serve-surface construct), the RESPOND_NOT_TERMINAL
    // code must never appear in an error.
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

// ── E: №600/№601 — the swallowed shapes REFUSE, the migration works ──

/// The audit §3 guard scenario, verbatim shape: a session-style outer
/// guard (the top-level block-if) with the 403 guard INSIDE it (depth 2)
/// and the protected DELETE after. With the BARE inner respond this must
/// NOT start (the pre-№600 behavior executed the DELETE for a non-admin).
fn audit_guard_swallowed(marker: &str) -> String {
    format!(
        r#"
db {{
  url: "sqlite::memory:"
}}

mlogserver {{
  port: 8094
  route "/admin/purge" method=POST {{
    let user = query_param("user")
    // №327: the one-way hash restores trust (the audit's
    // `if is_admin(...) == false` shape, expressed with the same
    // redact-compare guard the №581/№584 corpus uses).
    let key = redact(user, "hash_only")
    let expected = redact("admin", "hash_only")
    if key != "" {{
      if key != expected {{
        respond("403", "forbidden")
      }}
      db_execute("CREATE TABLE IF NOT EXISTS purge_log (id INTEGER)")
      db_execute("INSERT INTO purge_log VALUES (1)")
      kv_set("{marker}", "1")
      respond("200", "purged")
    }}
  }}
  route "/purge_ran" method=GET {{
    respond("200", kv_get("{marker}"))
  }}
}}
"#
    )
}

/// THE SAME route with the one-word migration (`return respond(...)`) —
/// the shape the current advisory already recommends. Must start and
/// answer correctly: 403 BEFORE the protected DELETE for a non-admin,
/// 200 + the marker for the admin.
fn audit_guard_migrated(marker: &str) -> String {
    audit_guard_swallowed(marker).replace(
        "        respond(\"403\", \"forbidden\")",
        "        return respond(\"403\", \"forbidden\")",
    )
}

#[tokio::test]
async fn n600_audit_guard_swallowed_refuses_vm() {
    assert_startup_refused(
        &audit_guard_swallowed("purge_n600_vm"),
        ServeBackend::Vm,
        "audit guard (bare, depth 2)",
    )
    .await;
}

#[tokio::test]
async fn n600_audit_guard_swallowed_refuses_interpreter() {
    assert_startup_refused(
        &audit_guard_swallowed("purge_n600_tw"),
        ServeBackend::Interpreter,
        "audit guard (bare, depth 2)",
    )
    .await;
}

#[tokio::test]
async fn n600_audit_guard_migrated_vm_non_admin_403_no_delete() {
    let port = start(&audit_guard_migrated("purge_n600m_vm"), ServeBackend::Vm).await;
    let (status, body) = http_post(port, "/admin/purge", "user=guest").await;
    assert_eq!(
        status, 403,
        "VM: the migrated depth-2 guard answers 403 BEFORE the protected code"
    );
    assert_eq!(body, "forbidden");
    let (marker_status, marker) = http_get(port, "/purge_ran").await;
    assert_eq!(marker_status, 200);
    assert_eq!(
        marker, "",
        "VM: the protected DELETE must NOT have run for the non-admin"
    );
}

#[tokio::test]
async fn n600_audit_guard_migrated_interpreter_non_admin_403_no_delete() {
    let port = start(
        &audit_guard_migrated("purge_n600m_tw"),
        ServeBackend::Interpreter,
    )
    .await;
    let (status, body) = http_post(port, "/admin/purge", "user=guest").await;
    assert_eq!(status, 403, "TW: the migrated depth-2 guard answers 403");
    assert_eq!(body, "forbidden");
    let (marker_status, marker) = http_get(port, "/purge_ran").await;
    assert_eq!(marker_status, 200);
    assert_eq!(marker, "", "TW: the protected DELETE must NOT have run");
}

#[tokio::test]
async fn n600_audit_guard_migrated_vm_admin_path_runs() {
    let port = start(&audit_guard_migrated("purge_n600ma_vm"), ServeBackend::Vm).await;
    let (status, body) = http_post(port, "/admin/purge", "user=admin").await;
    assert_eq!(status, 200, "VM: the admin path must serve");
    assert_eq!(body, "purged", "VM: the admin path runs the protected code");
    let (_, marker) = http_get(port, "/purge_ran").await;
    assert_eq!(marker, "1", "VM: the marker proves the protected code ran");
}

#[tokio::test]
async fn n600_audit_guard_migrated_interpreter_admin_path_runs() {
    let port = start(
        &audit_guard_migrated("purge_n600ma_tw"),
        ServeBackend::Interpreter,
    )
    .await;
    let (status, body) = http_post(port, "/admin/purge", "user=admin").await;
    assert_eq!(status, 200, "TW: the admin path must serve");
    assert_eq!(body, "purged");
    let (_, marker) = http_get(port, "/purge_ran").await;
    assert_eq!(marker, "1", "TW: the marker proves the protected code ran");
}

/// №601: every nested-statement KIND under a top-level block-if branch
/// must refuse — the swallow is not specific to the inner-if shape.
/// The conditions decide on LITERAL-derived values (trusted) so the
/// №327 UNTRUSTED_DECISION gate does not stamp its code first — the
/// assert names RESPOND_SWALLOWED as the FIRST blocking code.
#[tokio::test]
async fn n600_every_nested_kind_refuses_on_both_backends() {
    let shapes = [
        // nested block-if (IfElseBlock inside a branch)
        r#"
mlogserver {
  port: 8094
  route "/x" method=GET {
    let who = "guest"
    if who != "" {
      if who == "guest" {
        respond("403", "no")
      } else {
        respond("200", "yes")
      }
    }
  }
}
"#,
        // nested match under a branch
        r#"
mlogserver {
  port: 8094
  route "/x" method=GET {
    let who = "boss"
    if who != "" {
      match who {
        "boss" then { respond("200", "hello boss") }
        else { respond("403", "no") }
      }
    }
  }
}
"#,
        // nested cycle under a branch
        r#"
mlogserver {
  port: 8094
  route "/x" method=GET {
    let who = "loop"
    if who != "" {
      while who == "loop" {
        respond("200", "left")
      }
    }
  }
}
"#,
        // nested each under a branch
        r#"
mlogserver {
  port: 8094
  route "/x" method=GET {
    let who = "run"
    if who != "" {
      each item in ["a", "b"] {
        respond("200", "item " + item)
      }
    }
  }
}
"#,
        // nested single-line if (IfThen) under a branch, and the respond
        // sits deeper than depth 2 in the nested cycle — the swallow
        // propagates through ANY depth below the branch.
        r#"
mlogserver {
  port: 8094
  route "/x" method=GET {
    let who = "deep"
    if who != "" {
      if who == "deep" {
        while who == "deep" {
          respond("200", "deep")
        }
      }
    }
  }
}
"#,
    ];
    for (i, shape) in shapes.iter().enumerate() {
        let label = format!("nested kind #{i}");
        // The №523 semantic gate lives on the PRODUCTION serve entry and is
        // backend-independent (it refuses before any backend spins up) —
        // one refusal check per shape covers both backends.
        assert_startup_refused(shape, ServeBackend::Vm, &label).await;
    }
}

// ── F: №600 semantic unit pins — the SSOT predicate, kinds and codes ──

#[test]
fn n600_swallowed_shape_yields_blocking_error_with_stable_code() {
    use metalogos::parser;
    use metalogos::semantic::{check_program, SemanticErrorKind};
    let src = r#"
mlogserver {
  port: 8094
  route "/x" method=GET {
    let who = query_param("who")
    if who != "" {
      if who == "guest" {
        respond("403", "no")
      }
      respond("200", "yes")
    }
  }
}
"#;
    let decls: Vec<metalogos::ast::Declaration> =
        parser::parse(src).expect("the fixture must parse");
    let result = check_program(&decls);
    let hit = result
        .errors
        .iter()
        .find(|e| e.kind == SemanticErrorKind::RespondSwallowed)
        .expect("the depth-2 bare respond must be a BLOCKING error");
    assert_eq!(
        hit.kind.stable_code(),
        Some("RESPOND_SWALLOWED"),
        "the stable №479 code is machine-readable on the refusal"
    );
    assert!(
        hit.message.contains("return respond("),
        "the refusal carries the one-word migration hint, got: {}",
        hit.message
    );
    assert!(
        hit.message.contains("route GET /x"),
        "the refusal names the route (parity with the advisory), got: {}",
        hit.message
    );
    // The Span is carried verbatim from the statement (the №486 posture —
    // statement-level ExprStmt spans are `Span::unknown()` today, exactly
    // like the advisory's; the finding never invents a line number).
    let _ = hit.span;
    // The DIRECT branch respond in the same route stays advisory-only.
    assert!(
        !result
            .warnings
            .iter()
            .any(|w| w.kind == SemanticErrorKind::RespondSwallowed),
        "the swallowed class never lands in warnings"
    );
}

#[test]
fn n600_direct_branch_respond_stays_advisory_not_error() {
    use metalogos::parser;
    use metalogos::semantic::{check_program, SemanticErrorKind};
    // THE SAME guard shape WITHOUT the inner if: the bare respond is a
    // DIRECT statement of the top-level branch — the early-answer surface
    // (№584 posture must not regress into a blanket refusal).
    let src = r#"
mlogserver {
  port: 8094
  route "/x" method=GET {
    let who = query_param("who")
    if who == "guest" {
      respond("403", "no")
    }
    respond("200", "yes")
  }
}
"#;
    let decls: Vec<metalogos::ast::Declaration> =
        parser::parse(src).expect("the fixture must parse");
    let result = check_program(&decls);
    assert!(
        result
            .errors
            .iter()
            .all(|e| e.kind != SemanticErrorKind::RespondSwallowed),
        "a DIRECT branch respond is the surface — no RESPOND_SWALLOWED, got: {:?}",
        result.errors
    );
    assert!(
        result
            .warnings
            .iter()
            .any(|w| w.kind == SemanticErrorKind::RespondNotTerminal),
        "the direct branch respond keeps the style advisory (non-tail position)"
    );
}

#[test]
fn n600_respond_in_let_value_under_branch_is_not_the_class() {
    use metalogos::parser;
    use metalogos::semantic::{check_program, SemanticErrorKind};
    // A respond* consumed INSIDE a larger expression is not the bare
    // statement class on either backend (bare_respond_call) — the №600
    // predicate must not widen the net (the fail-closed rule forbids
    // manufactured false positives).
    let src = r#"
mlogserver {
  port: 8094
  route "/x" method=GET {
    let who = query_param("who")
    if who != "" {
      let caught = respond("403", "no")
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
            .any(|e| e.kind == SemanticErrorKind::RespondSwallowed),
        "a respond consumed in a let value is NOT the swallowed class"
    );
}

#[test]
fn n600_every_respond_name_refused_when_swallowed() {
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
    let who = query_param("who")
    if who != "" {{
      if who == "guest" {{
        {name}("403", "no")
      }}
    }}
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
                .any(|e| e.kind == SemanticErrorKind::RespondSwallowed),
            "{name}: the swallowed form must refuse for EVERY registry respond* name"
        );
    }
}

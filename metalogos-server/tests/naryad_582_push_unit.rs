// ── НАРЯД №582 (gh#996, the audit d63cc1d X-2) ──────────────────────
// PushUnit route-epilogue invariant: the body exit's stack.pop() NEVER
// reads a local slot. A route/branch body whose final statement leaves no
// value (a let/assignment tail, a loop tail, a no-match path) answers the
// same `200 OK` Unit the tree-walking lane yields — the queried row is no
// longer serialized into the response (the data-leak class).
//
// Blocks:
//   A — the audit's leak shape: the route ends with a let; the local value
//       must NOT reach the response (BOTH backends, real HTTP, and the
//       VM/TW bodies must be IDENTICAL — the strict cross-check).
//   B — the assignment tail and the loop tail (the same class by shape).
//   C — the if/match tails: a let-final branch, a taken-if guard with the
//       let branch, the no-else fall-through, the match no-match path —
//       every path answers Unit; the value tails still survive (№574).
//   D — the №574.1 regression: the value tails (respond in route/if/match
//       tails) keep serving their values on the VM.
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

// ── A: the audit's leak shape — the let-tail route ───────────────────

const LET_TAIL: &str = r#"
mlogserver {
  port: 0
  route "/audit_shape" method=GET {
    let user = query_param("user")
    let row = "email:phone:balance of " + user
  }
}
"#;

#[tokio::test]
async fn n582_let_tail_vm_never_serializes_the_local() {
    let port = start(LET_TAIL, ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/audit_shape?user=alice").await;
    assert_eq!(status, 200, "VM: the let-tail route answers 200 OK");
    assert!(
        !body.contains("email:phone:balance"),
        "VM: the queried row must NOT be serialized into the response, got: {body:?}"
    );
    assert!(
        !body.contains("alice"),
        "VM: the local's data must not leak through the epilogue, got: {body:?}"
    );
}

#[tokio::test]
async fn n582_let_tail_vm_tw_crosscheck_identical() {
    // The strict cross-check: the VM answer is byte-identical to the TW
    // answer (the №582 parity — Unit falls through to `200 OK` on both).
    let vm_port = start(LET_TAIL, ServeBackend::Vm).await;
    let tw_port = start(LET_TAIL, ServeBackend::Interpreter).await;
    let vm = http_get(vm_port, "/audit_shape?user=alice").await;
    let tw = http_get(tw_port, "/audit_shape?user=alice").await;
    assert_eq!(vm, tw, "VM and TW must answer identically on the let tail");
}

// ── B: the assignment tail and the loop tail ─────────────────────────

const ASSIGN_AND_LOOP_TAIL: &str = r#"
mlogserver {
  port: 0
  route "/assign_tail" method=GET {
    let mut acc = "start"
    let row = "SECRET-ROW"
    acc = row
  }
  route "/loop_tail" method=GET {
    let mut n = 0
    let secret = "LOOP-SECRET"
    while n < 2 {
      n = n + 1
    }
  }
}
"#;

#[tokio::test]
async fn n582_assign_tail_vm_answers_unit() {
    let port = start(ASSIGN_AND_LOOP_TAIL, ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/assign_tail").await;
    assert_eq!(status, 200, "VM: the assign-tail route answers 200 OK");
    assert!(
        !body.contains("SECRET-ROW") && !body.contains("start"),
        "VM: neither the assigned value nor the slot may leak, got: {body:?}"
    );
}

#[tokio::test]
async fn n582_loop_tail_vm_answers_unit() {
    let port = start(ASSIGN_AND_LOOP_TAIL, ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/loop_tail").await;
    assert_eq!(status, 200, "VM: the loop-tail route answers 200 OK");
    assert!(
        !body.contains("LOOP-SECRET"),
        "VM: the loop-adjacent local must not leak, got: {body:?}"
    );
}

// ── C: the if/match tails — every path carries exactly one value ─────

const IF_MATCH_TAILS: &str = r#"
mlogserver {
  port: 0
  route "/if_let_branch" method=GET {
    let flag = 1 == 1
    if flag {
      let inner = "INNER-LET"
    }
  }
  route "/if_let_else" method=GET {
    let flag = 1 == 2
    if flag {
      let inner = "TAKEN-LET"
    } else {
      respond("200", "else-ran")
    }
  }
  route "/guard_no_else" method=GET {
    let flag = 1 == 2
    if flag {
      respond("200", "taken")
    }
  }
  route "/match_let" method=GET {
    let k = "beta"
    match k {
      "alpha" then { let v = "AAA" }
      else { let v = "BBB" }
    }
  }
  route "/match_no_match" method=GET {
    let k = "gamma"
    match k {
      "alpha" then { respond("200", "AAA") }
      "beta" then { respond("200", "BBB") }
    }
  }
}
"#;

#[tokio::test]
async fn n582_if_let_branch_vm_answers_unit() {
    let port = start(IF_MATCH_TAILS, ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/if_let_branch").await;
    assert_eq!(
        status, 200,
        "VM: the if-tail with a let branch answers 200 OK"
    );
    assert!(
        !body.contains("INNER-LET"),
        "VM: the branch-local value must not leak, got: {body:?}"
    );
}

#[tokio::test]
async fn n582_if_else_branches_vm_path_balance() {
    // The else branch TAKEN (a respond value); the then branch is a let.
    // Both paths must behave like the TW: the taken branch answers.
    let port = start(IF_MATCH_TAILS, ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/if_let_else").await;
    assert_eq!(status, 200);
    assert_eq!(body, "else-ran", "VM: the taken else branch must serve");
}

#[tokio::test]
async fn n582_guard_no_else_vm_falls_through_to_unit() {
    // cond=false, no else: the fall-through used to read the LAST LOCAL
    // SLOT (the leftover-local class) — now it is the Unit `200 OK`.
    let vm_port = start(IF_MATCH_TAILS, ServeBackend::Vm).await;
    let (status, body) = http_get(vm_port, "/guard_no_else").await;
    assert_eq!(status, 200, "VM: the no-else fall-through answers 200 OK");
    assert!(
        !body.contains("taken"),
        "VM: the not-taken branch's value must not surface, got: {body:?}"
    );
    // TW parity.
    let tw_port = start(IF_MATCH_TAILS, ServeBackend::Interpreter).await;
    let tw = http_get(tw_port, "/guard_no_else").await;
    assert_eq!(
        (status, body),
        tw,
        "VM and TW must agree on the no-else guard fall-through"
    );
}

#[tokio::test]
async fn n582_match_let_tail_vm_answers_unit() {
    let port = start(IF_MATCH_TAILS, ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/match_let").await;
    assert_eq!(
        status, 200,
        "VM: the match tail with let arms answers 200 OK"
    );
    assert!(
        !body.contains("AAA") && !body.contains("BBB"),
        "VM: the matched arm's local must not leak, got: {body:?}"
    );
}

#[tokio::test]
async fn n582_match_no_match_vm_falls_through_to_unit() {
    // The scrutinee matches NO arm and there is NO else: the old epilogue
    // popped the last local slot — now it is the Unit `200 OK`.
    let vm_port = start(IF_MATCH_TAILS, ServeBackend::Vm).await;
    let (status, body) = http_get(vm_port, "/match_no_match").await;
    assert_eq!(status, 200, "VM: the no-match fall-through answers 200 OK");
    assert!(
        !body.contains("AAA") && !body.contains("BBB"),
        "VM: no arm value may surface on the no-match path, got: {body:?}"
    );
}

// ── D: the №574.1 regression — the value tails still survive ─────────

const VALUE_TAILS: &str = r#"
mlogserver {
  port: 0
  route "/route_tail" method=GET {
    let marker = "NOT-VISIBLE"
    respond("200", "route-tail-value")
  }
  route "/if_tail" method=GET {
    let flag = 1 == 1
    if flag {
      respond("200", "if-tail-value")
    }
  }
  route "/match_tail" method=GET {
    let k = "alpha"
    match k {
      "alpha" then { respond("200", "match-tail-value") }
      else { respond("200", "no") }
    }
  }
}
"#;

#[tokio::test]
async fn n582_value_tails_still_serve_vm() {
    let port = start(VALUE_TAILS, ServeBackend::Vm).await;
    let (_, body) = http_get(port, "/route_tail").await;
    assert_eq!(
        body, "route-tail-value",
        "VM: the route-tail value survives (№250)"
    );
    let (status, body) = http_get(port, "/if_tail").await;
    assert_eq!(
        (status, body.as_str()),
        (200, "if-tail-value"),
        "VM: the if-tail value survives (№574)"
    );
    let (status, body) = http_get(port, "/match_tail").await;
    assert_eq!(
        (status, body.as_str()),
        (200, "match-tail-value"),
        "VM: the match-tail value survives (№369)"
    );
}

#[test]
fn n582_pushunit_is_in_the_vm_dispatch_gate() {
    // ADR-0076: the new instruction is registered in the vm_golden
    // ALL_INSTRUCTIONS table (the dispatch-coverage gate) — the compile
    // gate below is the cheap twin: the route with a let tail COMPILES
    // and its bytecode carries the epilogue PushUnit.
    use metalogos::compiler::Compiler;
    use metalogos::parser;
    let src = r#"
mlogserver {
  port: 0
  route "/x" method=GET {
    let row = "data"
  }
}
"#;
    let declarations = parser::parse(src).expect("the fixture must parse");
    let routes: Vec<metalogos::ast::RouteDecl> = declarations
        .iter()
        .filter_map(|d| match d {
            metalogos::ast::Declaration::MlogServer(s) => Some(s.routes.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    let compiler = Compiler::new();
    let program = compiler
        .compile_routes(&routes)
        .expect("the route must compile");
    let code = &program[0].code;
    let is_push_unit = matches!(
        code.last(),
        Some(metalogos::bytecode::Instruction::PushUnit)
    );
    assert!(
        is_push_unit,
        "the route epilogue must end with the PushUnit invariant, got: {code:?}"
    );
}

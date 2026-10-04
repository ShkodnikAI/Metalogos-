#![cfg(feature = "server")]
// ── Наряд №574 (gh#975; волна 27): the VM-side pin of the json_body
// serve contract ─────────────────────────────────────────────────────
//
// The five scenarios lifted from ignore in tests/server_json_body.rs run
// through `run_test_server` — the tree-walking (TW) backend harness. The
// gh#967 §2 finding was the VM serve path (the DEFAULT backend per
// ADR-0171): the same scenarios must serve the SAME bodies on the VM, or
// the lifted ignores would pin only the backend they never exercised.
// This file pins the VM side: every scenario boots the server with the
// VM backend EXPLICITLY (run_test_server_with_backend_in_dir — the №207
// harness; not modified), posts the same JSON, asserts the same body.
//
// The compile-side root cause (fixed in №574): a respond() inside a
// taken if-branch compiled its value away (branch-final Pop), so the VM
// route fall-through returned the last local slot value instead of the
// branch's HttpResponse (the bool-field test served `true` — the
// condition value — instead of `yes`). The fix extends the №250
// route-tail rule to the branch tails (compile_if_stmt_with_keep).

use std::path::PathBuf;

async fn serve_vm(source: &str) -> u16 {
    let (port, _handle) = metalogos_server::server::run_test_server_with_backend_in_dir(
        source,
        metalogos_server::server::ServeBackend::Vm,
        PathBuf::from("."),
    )
    .await
    .expect("server should start");
    port
}

async fn post_json(port: u16, path: &str, body: serde_json::Value) -> (u16, String) {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://127.0.0.1:{}{}", port, path))
        .json(&body)
        .send()
        .await
        .unwrap();
    (resp.status().as_u16(), resp.text().await.unwrap())
}

const SOURCE_WEBHOOK: &str = r#"
mlogserver {
  port: 8091
  route "/webhook" method=POST {
    let data = json_body()
    let text = data.message.text
    let chat_id = data.message.chat.id
    respond("Got: " + text + " from " + to_string(chat_id))
  }
}
"#;

/// §2 scenario 1 (was server_json_body.rs :153): the leading concat
/// operand survives the VM serve path.
#[tokio::test]
async fn vm_webhook_telegram_contract() {
    let port = serve_vm(SOURCE_WEBHOOK).await;
    let (status, body) = post_json(
        port,
        "/webhook",
        serde_json::json!({
            "message": { "text": "hello", "chat": { "id": 12345 } }
        }),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body, "Got: hello from 12345");
}

/// §2 scenario 2 (was :179): respond(data.name) on a flat JSON body.
#[tokio::test]
async fn vm_webhook_flat_json() {
    let source = r#"
mlogserver {
  port: 8091
  route "/echo" method=POST {
    let data = json_body()
    respond(data.name)
  }
}
"#;
    let port = serve_vm(source).await;
    let (status, body) = post_json(port, "/echo", serde_json::json!({"name": "Fosved"})).await;
    assert_eq!(status, 200);
    assert_eq!(body, "Fosved");
}

/// §2 scenario 3 (was :200): get(data.items, 0) — the array-field access.
#[tokio::test]
async fn vm_webhook_array_field() {
    let source = r#"
mlogserver {
  port: 8091
  route "/items" method=POST {
    let data = json_body()
    let first = get(data.items, 0)
    respond(first)
  }
}
"#;
    let port = serve_vm(source).await;
    let (status, body) = post_json(
        port,
        "/items",
        serde_json::json!({"items": ["alpha", "beta"]}),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body, "alpha");
}

/// §2 scenario 4 (was :247): the bool field — the respond() inside the
/// taken if-branch IS the response (the №574 compile-side finding).
#[tokio::test]
async fn vm_webhook_bool_field() {
    let source = r#"
mlogserver {
  port: 8091
  route "/check" method=POST {
    let data = json_body()
    let active = data.active
    if active { respond("yes") } else { respond("no") }
  }
}
"#;
    let port = serve_vm(source).await;
    let (status, body) = post_json(port, "/check", serde_json::json!({"active": true})).await;
    assert_eq!(status, 200);
    assert_eq!(body, "yes");
}

/// §2 scenario 5 (was :278): the null field surfaces as the unit ().
#[tokio::test]
async fn vm_webhook_null_field_becomes_unit() {
    let source = r#"
mlogserver {
  port: 8091
  route "/null" method=POST {
    let data = json_body()
    let value = data.maybe
    respond(to_string(value))
  }
}
"#;
    let port = serve_vm(source).await;
    let (status, body) = post_json(port, "/null", serde_json::json!({"maybe": null})).await;
    assert_eq!(status, 200);
    assert_eq!(body, "()");
}

/// №574: the if/else-respond shape with a LITERAL condition — the same
/// route-tail contract without json_body (the minimal repro of the
/// compile-side finding; a literal `true` condition served `true`).
#[tokio::test]
async fn vm_route_tail_if_else_respond_literal_condition() {
    let source = r#"
mlogserver {
  port: 8091
  route "/check2" method=POST {
    let active = true
    if active { respond("yes") } else { respond("no") }
  }
}
"#;
    let port = serve_vm(source).await;
    let (status, body) = post_json(port, "/check2", serde_json::json!({})).await;
    assert_eq!(status, 200);
    assert_eq!(body, "yes");
}

/// №574: the if-as-EXPRESSION shape (never broken — the value flows
/// through the let binding; pinned so the fix cannot regress it).
#[tokio::test]
async fn vm_route_tail_if_expression_still_works() {
    let source = r#"
mlogserver {
  port: 8091
  route "/check3" method=POST {
    let data = json_body()
    let active = data.active
    let answer = if active { "yes" } else { "no" }
    respond(answer)
  }
}
"#;
    let port = serve_vm(source).await;
    let (status, body) = post_json(port, "/check3", serde_json::json!({"active": true})).await;
    assert_eq!(status, 200);
    assert_eq!(body, "yes");
}

/// №574: the else branch of a route-tail if/else serves ITS value too
/// (both branch tails keep; the condition is false here).
#[tokio::test]
async fn vm_route_tail_if_else_else_branch_serves() {
    let source = r#"
mlogserver {
  port: 8091
  route "/check4" method=POST {
    let active = false
    if active { respond("yes") } else { respond("no") }
  }
}
"#;
    let port = serve_vm(source).await;
    let (status, body) = post_json(port, "/check4", serde_json::json!({})).await;
    assert_eq!(status, 200);
    assert_eq!(body, "no");
}

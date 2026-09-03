// ── НАРЯД #160: VM-for-serve full verification ────────────────────
// Closes the remaining unverified items from ADR-0081/ADR-0088:
//   Block 1: Re-check `match` in FOSVED routes (current .mlog corpus)
//   Block 2: Real HTTP-stack testing (TcpListener + reqwest, not direct calls)
//   Block 3: True parallel request isolation (tokio::spawn)
//   Block 4: TW vs VM side-by-side via real HTTP (status + body + audit log)

#![cfg(feature = "server")]

use metalogos::server::ServeBackend;

const SOURCE_REALISTIC_ROUTES: &str = r#"
mlogserver {
  port: 8090
  route "/hello" method=GET {
    let name = query_param("name")
    respond("200", "hello " + name)
  }
  route "/echo" method=POST {
    let body = json_body()
    let msg = body.message
    respond("200", "echo: " + msg)
  }
  route "/concat" method=GET {
    let a = query_param("a")
    let b = query_param("b")
    respond("200", a + "-" + b)
  }
  route "/status" method=GET {
    respond("201", "created")
  }
  route "/greet" method=POST {
    let body = json_body()
    let name = body.name
    let up = upper(name)
    respond("200", "HELLO " + up)
  }
}
"#;

const SOURCE_KV_ROUTES: &str = r#"
mlogserver {
  port: 8090
  route "/set" method=GET {
    let key = query_param("key")
    let val = query_param("val")
    kv_set(key, val)
    respond("200", "set " + key + "=" + val)
  }
  route "/get" method=GET {
    let key = query_param("key")
    let val = kv_get(key)
    respond("200", val)
  }
}
"#;

const SOURCE_ISOLATION: &str = r#"
mlogserver {
  port: 8090
  route "/whoami" method=GET {
    let id = query_param("id")
    respond("200", "you are " + id)
  }
  route "/echo_n" method=GET {
    let n = query_param("n")
    respond("200", "n=" + n)
  }
}
"#;

// ── Helpers ──────────────────────────────────────────────────────────

async fn start_server(
    source: &str,
    backend: ServeBackend,
) -> (
    u16,
    tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
) {
    metalogos::server::run_test_server_with_backend(source, backend)
        .await
        .expect("test server should start")
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

async fn http_post_json(
    port: u16,
    path: &str,
    json: &serde_json::Value,
) -> (u16, String) {
    let url = format!("http://127.0.0.1:{}{}", port, path);
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(json)
        .send()
        .await
        .expect("POST request should succeed");
    let status = resp.status().as_u16();
    let body = resp.text().await.expect("response body should be readable");
    (status, body)
}

// ═══════════════════════════════════════════════════════════════════
// БЛОК 1 — Перепроверить `match` в маршрутах
// ═══════════════════════════════════════════════════════════════════

#[test]
fn block1_no_match_in_test_route_bodies() {
    for (name, source) in [
        ("REALISTIC_ROUTES", SOURCE_REALISTIC_ROUTES),
        ("KV_ROUTES", SOURCE_KV_ROUTES),
        ("ISOLATION", SOURCE_ISOLATION),
    ] {
        assert!(
            !source.contains("match "),
            "Block 1: {} source contains 'match' — VM will reject it",
            name
        );
    }
}

#[tokio::test]
async fn block1_match_still_rejected_by_vm_compiler() {
    let source = r#"
mlogserver {
  port: 8090
  route "/test" method=GET {
    let x = "hello"
    match x {
      "hello" then { respond("200", "yes") }
      else { respond("200", "no") }
    }
  }
}
"#;
    let result =
        metalogos::server::run_test_server_with_backend(source, ServeBackend::Vm)
            .await;
    assert!(
        result.is_err(),
        "Block 1: VM server with match in route should fail to start"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Match statement not yet supported"),
        "Block 1: error should mention Match, got: {}",
        err
    );
}

#[test]
fn block1_scan_all_examples_for_match_in_routes() {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let examples_dir = std::path::Path::new(&manifest_dir).join("examples");
    if let Ok(entries) = std::fs::read_dir(&examples_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("mlog") {
                continue;
            }
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            if !content.contains("mlogserver") {
                continue;
            }
            let name = path.file_name().unwrap_or_default().to_str().unwrap_or("?");
            let in_route = extract_route_bodies(&content);
            for (_route_sig, body) in &in_route {
                if body.contains("match ") {
                    let has_match_in_code = body.lines().any(|line| {
                        let trimmed = line.trim();
                        trimmed.starts_with("match ") || trimmed.contains(" match ")
                    });
                    if has_match_in_code {
                        panic!("Block 1 VIOLATION: {} has 'match' in route body", name);
                    }
                }
            }
        }
    }
}

fn extract_route_bodies(source: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        if line.starts_with("route ") {
            let sig = line.to_string();
            let mut body_lines = Vec::new();
            let mut brace_depth = 0;
            let mut found_open = false;
            if line.contains('{') {
                found_open = true;
                brace_depth += line.matches('{').count() as i32;
                brace_depth -= line.matches('}').count() as i32;
            }
            i += 1;
            while i < lines.len() && brace_depth > 0 {
                body_lines.push(lines[i]);
                brace_depth += lines[i].matches('{').count() as i32;
                brace_depth -= lines[i].matches('}').count() as i32;
                i += 1;
            }
            if found_open {
                results.push((sig, body_lines.join("\n")));
            }
        } else {
            i += 1;
        }
    }
    results
}

// ═══════════════════════════════════════════════════════════════════
// БЛОК 2 — Настоящий HTTP-стек
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn block2_vm_get_with_query_param() {
    let (port, _handle) = start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/hello?name=Alice").await;
    assert_eq!(status, 200, "VM GET /hello should return 200");
    assert_eq!(body, "hello Alice", "VM GET /hello?name=Alice body");
}

#[tokio::test]
async fn block2_vm_get_empty_query_param() {
    let (port, _handle) = start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/hello").await;
    assert_eq!(status, 200);
    assert_eq!(body, "hello ");
}

#[tokio::test]
async fn block2_vm_post_with_json_body() {
    let (port, _handle) = start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let json = serde_json::json!({"message": "test payload"});
    let (status, body) = http_post_json(port, "/echo", &json).await;
    assert_eq!(status, 200);
    assert_eq!(body, "echo: test payload");
}

#[tokio::test]
async fn block2_vm_get_concat() {
    let (port, _handle) = start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/concat?a=foo&b=bar").await;
    assert_eq!(status, 200);
    assert_eq!(body, "foo-bar");
}

#[tokio::test]
async fn block2_vm_get_custom_status() {
    let (port, _handle) = start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/status").await;
    assert_eq!(status, 201, "VM /status should return 201");
    assert_eq!(body, "created");
}

#[tokio::test]
async fn block2_vm_post_upper() {
    let (port, _handle) = start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let json = serde_json::json!({"name": "alice"});
    let (status, body) = http_post_json(port, "/greet", &json).await;
    assert_eq!(status, 200);
    assert_eq!(body, "HELLO ALICE");
}

#[tokio::test]
async fn block2_vm_404_unknown_route() {
    let (port, _handle) = start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/nonexistent").await;
    assert_eq!(status, 404);
    assert_eq!(body, "404 Not Found");
}

#[tokio::test]
async fn block2_vm_405_wrong_method() {
    let (port, _handle) = start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let url = format!("http://127.0.0.1:{}/hello", port);
    let client = reqwest::Client::new();
    let resp = client.post(&url).send().await.expect("POST should succeed");
    assert_eq!(resp.status().as_u16(), 405);
}

#[tokio::test]
async fn block2_tw_get_with_query_param() {
    let (port, _handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Interpreter).await;
    let (status, body) = http_get(port, "/hello?name=Bob").await;
    assert_eq!(status, 200);
    assert_eq!(body, "hello Bob");
}

// ═══════════════════════════════════════════════════════════════════
// БЛОК 3 — Параллельная нагрузка
// ═══════════════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn block3_vm_parallel_query_param_isolation() {
    let (port, _handle) = start_server(SOURCE_ISOLATION, ServeBackend::Vm).await;
    let requests: Vec<(&str, &str)> = vec![
        ("id=alice", "you are alice"),
        ("id=bob", "you are bob"),
        ("id=charlie", "you are charlie"),
        ("id=diana", "you are diana"),
        ("id=eve", "you are eve"),
        ("id=frank", "you are frank"),
        ("id=grace", "you are grace"),
        ("id=heidi", "you are heidi"),
    ];
    let mut handles = Vec::new();
    for (query, expected_body) in requests {
        let url = format!("http://127.0.0.1:{}/whoami?{}", port, query);
        let expected = expected_body.to_string();
        handles.push(tokio::spawn(async move {
            let resp = reqwest::get(&url)
                .await
                .expect("request should succeed");
            let status = resp.status().as_u16();
            let body = resp.text().await.expect("body should be readable");
            assert_eq!(
                status,
                200,
                "parallel /whoami?{}: expected 200, got {}",
                query,
                status
            );
            assert_eq!(
                body,
                expected,
                "parallel /whoami?{}: expected {:?}, got {:?}",
                query,
                expected,
                body
            );
        }));
    }
    for handle in handles {
        handle.await.expect("parallel task should not panic");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn block3_vm_parallel_mixed_routes() {
    let (port, _handle) = start_server(SOURCE_ISOLATION, ServeBackend::Vm).await;
    let mut handles = Vec::new();
    for i in 0..4u32 {
        let url = format!("http://127.0.0.1:{}/whoami?id=worker-{}", port, i);
        let expected = format!("you are worker-{}", i);
        handles.push(tokio::spawn(async move {
            let resp = reqwest::get(&url)
                .await
                .expect("request should succeed");
            assert_eq!(resp.status().as_u16(), 200);
            let body = resp.text().await.unwrap();
            assert_eq!(body, expected);
        }));
    }
    for i in 0..4u32 {
        let url = format!("http://127.0.0.1:{}/echo_n?n={}", port, i);
        let expected = format!("n={}", i);
        handles.push(tokio::spawn(async move {
            let resp = reqwest::get(&url)
                .await
                .expect("request should succeed");
            assert_eq!(resp.status().as_u16(), 200);
            let body = resp.text().await.unwrap();
            assert_eq!(body, expected);
        }));
    }
    for handle in handles {
        handle.await.expect("parallel task should not panic");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn block3_vm_parallel_kv_isolation() {
    let (port, _handle) = start_server(SOURCE_KV_ROUTES, ServeBackend::Vm).await;
    let mut handles = Vec::new();
    for i in 0..8u32 {
        let url = format!("http://127.0.0.1:{}/set?key=n160-{}&val=v{}", port, i, i);
        let expected = format!("set n160-{}=v{}", i, i);
        handles.push(tokio::spawn(async move {
            let resp = reqwest::get(&url)
                .await
                .expect("set request should succeed");
            assert_eq!(resp.status().as_u16(), 200);
            let body = resp.text().await.unwrap();
            assert_eq!(body, expected);
        }));
    }
    for handle in handles {
        handle.await.expect("parallel kv_set should not panic");
    }
    for i in 0..8u32 {
        let (status, body) = http_get(port, &format!("/get?key=n160-{}", i)).await;
        assert_eq!(status, 200);
        assert_eq!(
            body,
            format!("v{}", i),
            "kv_get for n160-{}: expected v{}, got {}",
            i,
            i,
            body
        );
    }
}

// ═══════════════════════════════════════════════════════════════════
// БЛОК 4 — TW vs VM бок о бок
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn block4_tw_vs_vm_get_query_param() {
    let (tw_port, tw_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Interpreter).await;
    let (tw_status, tw_body) = http_get(tw_port, "/hello?name=TestUser").await;
    tw_handle.abort();
    let (vm_port, vm_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (vm_status, vm_body) = http_get(vm_port, "/hello?name=TestUser").await;
    vm_handle.abort();
    assert_eq!(
        tw_status, vm_status,
        "Block 4: GET /hello?name=TestUser status mismatch: TW={} VM={}",
        tw_status, vm_status
    );
    assert_eq!(
        tw_body, vm_body,
        "Block 4: GET /hello?name=TestUser body mismatch: TW={:?} VM={:?}",
        tw_body, vm_body
    );
}

#[tokio::test]
async fn block4_tw_vs_vm_post_json_body() {
    let json = serde_json::json!({"message": "hello world"});
    let (tw_port, tw_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Interpreter).await;
    let (tw_status, tw_body) = http_post_json(tw_port, "/echo", &json).await;
    tw_handle.abort();
    let (vm_port, vm_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (vm_status, vm_body) = http_post_json(vm_port, "/echo", &json).await;
    vm_handle.abort();
    assert_eq!(
        tw_status, vm_status,
        "Block 4: POST /echo status mismatch: TW={} VM={}",
        tw_status, vm_status
    );
    assert_eq!(
        tw_body, vm_body,
        "Block 4: POST /echo body mismatch: TW={:?} VM={:?}",
        tw_body, vm_body
    );
}

#[tokio::test]
async fn block4_tw_vs_vm_get_concat() {
    let (tw_port, tw_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Interpreter).await;
    let (tw_status, tw_body) = http_get(tw_port, "/concat?a=hello&b=world").await;
    tw_handle.abort();
    let (vm_port, vm_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (vm_status, vm_body) = http_get(vm_port, "/concat?a=hello&b=world").await;
    vm_handle.abort();
    assert_eq!(tw_status, vm_status, "Block 4: GET /concat status mismatch");
    assert_eq!(
        tw_body, vm_body,
        "Block 4: GET /concat body mismatch: TW={:?} VM={:?}",
        tw_body, vm_body
    );
}

#[tokio::test]
async fn block4_tw_vs_vm_custom_status() {
    let (tw_port, tw_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Interpreter).await;
    let (tw_status, tw_body) = http_get(tw_port, "/status").await;
    tw_handle.abort();
    let (vm_port, vm_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (vm_status, vm_body) = http_get(vm_port, "/status").await;
    vm_handle.abort();
    assert_eq!(
        tw_status, vm_status,
        "Block 4: GET /status status mismatch: TW={} VM={}",
        tw_status, vm_status
    );
    assert_eq!(
        tw_body, vm_body,
        "Block 4: GET /status body mismatch: TW={:?} VM={:?}",
        tw_body, vm_body
    );
}

#[tokio::test]
async fn block4_tw_vs_vm_post_upper() {
    let json = serde_json::json!({"name": "metalogos"});
    let (tw_port, tw_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Interpreter).await;
    let (tw_status, tw_body) = http_post_json(tw_port, "/greet", &json).await;
    tw_handle.abort();
    let (vm_port, vm_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (vm_status, vm_body) = http_post_json(vm_port, "/greet", &json).await;
    vm_handle.abort();
    assert_eq!(
        tw_status, vm_status,
        "Block 4: POST /greet status mismatch: TW={} VM={}",
        tw_status, vm_status
    );
    assert_eq!(
        tw_body, vm_body,
        "Block 4: POST /greet body mismatch: TW={:?} VM={:?}",
        tw_body, vm_body
    );
}

#[tokio::test]
async fn block4_tw_vs_vm_empty_query_param() {
    let (tw_port, tw_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Interpreter).await;
    let (tw_status, tw_body) = http_get(tw_port, "/hello").await;
    tw_handle.abort();
    let (vm_port, vm_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (vm_status, vm_body) = http_get(vm_port, "/hello").await;
    vm_handle.abort();
    assert_eq!(tw_status, vm_status, "Block 4: /hello no-param status mismatch");
    assert_eq!(
        tw_body, vm_body,
        "Block 4: /hello no-param body mismatch: TW={:?} VM={:?}",
        tw_body, vm_body
    );
}

#[tokio::test]
async fn block4_tw_vs_vm_404() {
    let (tw_port, tw_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Interpreter).await;
    let (tw_status, tw_body) = http_get(tw_port, "/nonexistent").await;
    tw_handle.abort();
    let (vm_port, vm_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (vm_status, vm_body) = http_get(vm_port, "/nonexistent").await;
    vm_handle.abort();
    assert_eq!(tw_status, vm_status, "Block 4: 404 status mismatch");
    assert_eq!(tw_body, vm_body, "Block 4: 404 body mismatch");
}

#[tokio::test]
async fn block4_tw_vs_vm_405() {
    let client = reqwest::Client::new();
    let (tw_port, tw_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Interpreter).await;
    let url_tw = format!("http://127.0.0.1:{}/hello", tw_port);
    let tw_resp = client.post(&url_tw).send().await.unwrap();
    let tw_status = tw_resp.status().as_u16();
    tw_handle.abort();
    let (vm_port, vm_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let url_vm = format!("http://127.0.0.1:{}/hello", vm_port);
    let vm_resp = client.post(&url_vm).send().await.unwrap();
    let vm_status = vm_resp.status().as_u16();
    vm_handle.abort();
    assert_eq!(
        tw_status, vm_status,
        "Block 4: 405 status mismatch: TW={} VM={}",
        tw_status, vm_status
    );
}

#[tokio::test]
async fn block4_tw_vm_kv_shared_store() {
    let (tw_port, tw_handle) =
        start_server(SOURCE_KV_ROUTES, ServeBackend::Interpreter).await;
    let (tw_status, tw_body) =
        http_get(tw_port, "/set?key=n160-shared&val=tw-wrote-this").await;
    assert_eq!(tw_status, 200);
    assert_eq!(tw_body, "set n160-shared=tw-wrote-this");
    tw_handle.abort();
    let (vm_port, vm_handle) =
        start_server(SOURCE_KV_ROUTES, ServeBackend::Vm).await;
    let (vm_status, vm_body) = http_get(vm_port, "/get?key=n160-shared").await;
    assert_eq!(vm_status, 200);
    assert_eq!(
        vm_body,
        "",
        "Block 4: kv store should NOT be shared across server instances"
    );
    vm_handle.abort();
}

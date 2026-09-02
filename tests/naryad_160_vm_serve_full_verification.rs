// ── НАРЯД #160: VM-for-serve full verification ────────────────────
// Closes the remaining unverified items from ADR-0081/ADR-0088:
//   Block 1: Re-check `match` in FOSVED routes (current .mlog corpus)
//   Block 2: Real HTTP-stack testing (TcpListener + reqwest, not direct calls)
//   Block 3: True parallel request isolation (tokio::spawn)
//   Block 4: TW vs VM side-by-side via real HTTP (status + body + audit log)
//
// All tests use `run_test_server_with_backend` which spins up a real
// Axum TCP server on OS-assigned port, then sends actual HTTP requests.
// This exercises the full middleware chain (CORS, rate-limit, CSRF,
// security headers, session) — not just `execute_route_body` directly.

#![cfg(feature = "server")]

use metalogos::server::ServeBackend;

// ── Shared test .mlog sources ──────────────────────────────────────

/// Routes exercising query_param, string concat, respond, let bindings,
/// kv_set/kv_get, upper builtin — no match, no block-if-else (VM-safe per ADR-0105).
/// Note: ADR-0105 documents that IfElseBlock silently evaluates to Unit in VM.
/// All routes below use only IfThen (no else branch) or no branching at all.
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

/// Routes with kv_set/kv_get for isolation testing.
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

/// Multi-route source for parallel isolation: each request uses
/// query_param for its own data (no kv_set needed).
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

/// Spawn a real HTTP server on OS-assigned port, return (port, handle).
async fn start_server(
    source: &str,
    backend: ServeBackend,
) -> (u16, tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>) {
    metalogos::server::run_test_server_with_backend(source, backend)
        .await
        .expect("test server should start")
}

/// Send GET request, return (status, body).
async fn http_get(
    port: u16,
    path: &str,
) -> (u16, String) {
    let url = format!("http://127.0.0.1:{}{}", port, path);
    let resp = reqwest::get(&url).await.expect("GET request should succeed");
    let status = resp.status().as_u16();
    let body = resp.text().await.expect("response body should be readable");
    (status, body)
}

/// Send POST request with JSON body, return (status, body).
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
// БЛОК 1 — Перепроверить `match` в маршрутах на актуальном корпусе
// ═══════════════════════════════════════════════════════════════════

/// Блок 1: Verify that NONE of the route bodies used in this test
/// file contain `match`. This is a compile-time sanity check — if
/// someone adds a match to one of the test routes, this test fails
/// loudly instead of silently producing a VM compile error.
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

/// Блок 1: Verify that the VM compiler still rejects match statements
/// in route bodies (regression guard for ADR-0088 Block 1).
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
    let result = metalogos::server::run_test_server_with_backend(
        source,
        ServeBackend::Vm,
    )
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

/// Блок 1: Scan ALL .mlog files in examples/ for `match` inside
/// mlogserver route bodies. This re-validates the ADR-0081 assessment
/// against the current codebase.
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
            let content =
                std::fs::read_to_string(&path).unwrap_or_default();

            // Only check files that have mlogserver blocks with routes
            if !content.contains("mlogserver") {
                continue;
            }

            // Extract route bodies (between { } after route ... method=...)
            // Simple heuristic: look for `match ` keyword between
            // `route` and the closing `}` of the mlogserver block.
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or("?");

            // Find all route bodies and check for match
            let in_route = extract_route_bodies(&content);
            for (route_sig, body) in &in_route {
                if body.contains("match ") {
                    // Check if it's inside a comment
                    let has_match_in_code = body
                        .lines()
                        .any(|line| {
                            let trimmed = line.trim();
                            trimmed.starts_with("match ")
                                || trimmed.contains(" match ")
                                || trimmed.ends_with(" match")
                        });
                    if has_match_in_code {
                        panic!(
                            "Block 1 VIOLATION: {} has 'match' in route body \n  route: {} \n  body: {}",
                            name, route_sig, body
                        );
                    }
                }
            }
        }
    }
}

/// Extract (route_signature, body_text) pairs from mlogserver source.
fn extract_route_bodies(source: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        if line.starts_with("route ") {
            // Collect the route signature (everything up to the first {)
            let mut sig = line.to_string();
            let mut body_lines = Vec::new();
            let mut brace_depth = 0;
            let mut found_open = false;

            // Check if the opening brace is on this line
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
// БЛОК 2 — Настоящий HTTP-стек (не прямые вызовы)
// ═══════════════════════════════════════════════════════════════════

/// Блок 2: VM backend — GET route with query_param via real HTTP.
#[tokio::test]
async fn block2_vm_get_with_query_param() {
    let (port, _handle) = start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/hello?name=Alice").await;
    assert_eq!(status, 200, "VM GET /hello should return 200");
    assert_eq!(body, "hello Alice", "VM GET /hello?name=Alice body");
}

/// Блок 2: VM backend — GET route with empty query_param (default branch).
#[tokio::test]
async fn block2_vm_get_empty_query_param() {
    let (port, _handle) = start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/hello").await;
    assert_eq!(status, 200);
    assert_eq!(body, "hello ");
}

/// Блок 2: VM backend — POST route with JSON body via real HTTP.
#[tokio::test]
async fn block2_vm_post_with_json_body() {
    let (port, _handle) = start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (status, body) = http_post_json(
        port,
        "/echo",
        &serde_json::json!({"message": "test payload"}),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body, "echo: test payload");
}

/// Блок 2: VM backend — GET with string concatenation.
#[tokio::test]
async fn block2_vm_get_concat() {
    let (port, _handle) = start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/concat?a=foo&b=bar").await;
    assert_eq!(status, 200);
    assert_eq!(body, "foo-bar");
}

/// Блок 2: VM backend — GET with non-200 status code.
#[tokio::test]
async fn block2_vm_get_custom_status() {
    let (port, _handle) = start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/status").await;
    assert_eq!(status, 201, "VM /status should return 201");
    assert_eq!(body, "created");
}

/// Блок 2: VM backend — POST with upper builtin.
#[tokio::test]
async fn block2_vm_post_upper() {
    let (port, _handle) = start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (status, body) = http_post_json(
        port,
        "/greet",
        &serde_json::json!({"name": "alice"}),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body, "HELLO ALICE");
}

/// Блок 2: VM backend — 404 for unknown route via real HTTP.
#[tokio::test]
async fn block2_vm_404_unknown_route() {
    let (port, _handle) = start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/nonexistent").await;
    assert_eq!(status, 404);
    assert_eq!(body, "404 Not Found");
}

/// Блок 2: VM backend — 405 for wrong method via real HTTP.
#[tokio::test]
async fn block2_vm_405_wrong_method() {
    let (port, _handle) = start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    // /hello is GET-only; POST should return 405
    let url = format!("http://127.0.0.1:{}/hello", port);
    let client = reqwest::Client::new();
    let resp = client.post(&url).send().await.expect("POST should succeed");
    assert_eq!(resp.status().as_u16(), 405);
}

/// Блок 2: TW backend — same GET route (baseline for Block 4 comparison).
#[tokio::test]
async fn block2_tw_get_with_query_param() {
    let (port, _handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Interpreter).await;
    let (status, body) = http_get(port, "/hello?name=Bob").await;
    assert_eq!(status, 200);
    assert_eq!(body, "hello Bob");
}

// ═══════════════════════════════════════════════════════════════════
// БЛОК 3 — Настоящая параллельная нагрузка
// ═══════════════════════════════════════════════════════════════════

/// Блок 3: Parallel GET requests — each gets its own query_param,
/// responses must not be mixed.
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
            let resp = reqwest::get(&url).await.expect("request should succeed");
            let status = resp.status().as_u16();
            let body = resp.text().await.expect("body should be readable");
            assert_eq!(
                status, 200,
                "parallel /whoami?{}: expected 200, got {}",
                query, status
            );
            assert_eq!(
                body, expected,
                "parallel /whoami?{}: expected {:?}, got {:?}",
                query, expected, body
            );
        }));
    }

    for handle in handles {
        handle.await.expect("parallel task should not panic");
    }
}

/// Блок 3: Parallel mixed routes — /whoami and /echo_n interleaved.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn block3_vm_parallel_mixed_routes() {
    let (port, _handle) = start_server(SOURCE_ISOLATION, ServeBackend::Vm).await;

    let mut handles = Vec::new();

    // 4 /whoami requests
    for i in 0..4u32 {
        let url = format!(
            "http://127.0.0.1:{}/whoami?id=worker-{}",
            port, i
        );
        let expected = format!("you are worker-{}", i);
        handles.push(tokio::spawn(async move {
            let resp = reqwest::get(&url).await.expect("request should succeed");
            assert_eq!(resp.status().as_u16(), 200);
            let body = resp.text().await.unwrap();
            assert_eq!(body, expected);
        }));
    }

    // 4 /echo_n requests
    for i in 0..4u32 {
        let url = format!("http://127.0.0.1:{}/echo_n?n={}", port, i);
        let expected = format!("n={}", i);
        handles.push(tokio::spawn(async move {
            let resp = reqwest::get(&url).await.expect("request should succeed");
            assert_eq!(resp.status().as_u16(), 200);
            let body = resp.text().await.unwrap();
            assert_eq!(body, expected);
        }));
    }

    for handle in handles {
        handle.await.expect("parallel task should not panic");
    }
}

/// Блок 3: Parallel kv_set in one request must NOT leak into
/// concurrent requests reading via kv_get. Each request sets
/// a unique key; no request should see another's key.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn block3_vm_parallel_kv_isolation() {
    let (port, _handle) = start_server(SOURCE_KV_ROUTES, ServeBackend::Vm).await;

    let mut handles = Vec::new();
    for i in 0..8u32 {
        let url = format!(
            "http://127.0.0.1:{}/set?key=n160-{}&val=v{}",
            port, i, i
        );
        let expected = format!("set n160-{}=v{}", i, i);
        handles.push(tokio::spawn(async move {
            let resp = reqwest::get(&url).await.expect("set request should succeed");
            assert_eq!(resp.status().as_u16(), 200);
            let body = resp.text().await.unwrap();
            assert_eq!(body, expected);
        }));
    }

    for handle in handles {
        handle.await.expect("parallel kv_set should not panic");
    }

    // Verify each key independently (sequential, after all sets complete)
    for i in 0..8u32 {
        let url = format!(
            "http://127.0.0.1:{}/get?key=n160-{}",
            port, i
        );
        let (status, body) = http_get(port, &format!("/get?key=n160-{}", i)).await;
        assert_eq!(status, 200);
        assert_eq!(
            body, format!("v{}", i),
            "kv_get for n160-{}: expected v{}, got {}",
            i, i, body
        );
    }
}

// ═══════════════════════════════════════════════════════════════════
// БЛОК 4 — Сравнение TW vs VM бок о бок (реальный HTTP)
// ═══════════════════════════════════════════════════════════════════

/// Блок 4: TW vs VM — GET with query_param, both via real HTTP.
#[tokio::test]
async fn block4_tw_vs_vm_get_query_param() {
    // TW
    let (tw_port, tw_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Interpreter).await;
    let (tw_status, tw_body) = http_get(tw_port, "/hello?name=TestUser").await;
    tw_handle.abort();

    // VM
    let (vm_port, vm_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (vm_status, vm_body) = http_get(vm_port, "/hello?name=TestUser").await;
    vm_handle.abort();

    assert_eq!(tw_status, vm_status,
        "Block 4: GET /hello?name=TestUser status mismatch: TW={} VM={}",
        tw_status, vm_status);
    assert_eq!(tw_body, vm_body,
        "Block 4: GET /hello?name=TestUser body mismatch: TW={:?} VM={:?}",
        tw_body, vm_body);
}

/// Блок 4: TW vs VM — POST with JSON body, both via real HTTP.
#[tokio::test]
async fn block4_tw_vs_vm_post_json_body() {
    let json = serde_json::json!({"message": "hello world"});

    // TW
    let (tw_port, tw_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Interpreter).await;
    let (tw_status, tw_body) = http_post_json(tw_port, "/echo", &json).await;
    tw_handle.abort();

    // VM
    let (vm_port, vm_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (vm_status, vm_body) = http_post_json(vm_port, "/echo", &json).await;
    vm_handle.abort();

    assert_eq!(tw_status, vm_status,
        "Block 4: POST /echo status mismatch: TW={} VM={}",
        tw_status, vm_status);
    assert_eq!(tw_body, vm_body,
        "Block 4: POST /echo body mismatch: TW={:?} VM={:?}",
        tw_body, vm_body);
}

/// Блок 4: TW vs VM — GET with string concatenation.
#[tokio::test]
async fn block4_tw_vs_vm_get_concat() {
    // TW
    let (tw_port, tw_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Interpreter).await;
    let (tw_status, tw_body) = http_get(tw_port, "/concat?a=hello&b=world").await;
    tw_handle.abort();

    // VM
    let (vm_port, vm_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (vm_status, vm_body) = http_get(vm_port, "/concat?a=hello&b=world").await;
    vm_handle.abort();

    assert_eq!(tw_status, vm_status,
        "Block 4: GET /concat status mismatch");
    assert_eq!(tw_body, vm_body,
        "Block 4: GET /concat body mismatch: TW={:?} VM={:?}",
        tw_body, vm_body);
}

/// Блок 4: TW vs VM — GET with custom status code.
#[tokio::test]
async fn block4_tw_vs_vm_custom_status() {
    // TW
    let (tw_port, tw_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Interpreter).await;
    let (tw_status, tw_body) = http_get(tw_port, "/status").await;
    tw_handle.abort();

    // VM
    let (vm_port, vm_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (vm_status, vm_body) = http_get(vm_port, "/status").await;
    vm_handle.abort();

    assert_eq!(tw_status, vm_status,
        "Block 4: GET /status status mismatch: TW={} VM={}",
        tw_status, vm_status);
    assert_eq!(tw_body, vm_body,
        "Block 4: GET /status body mismatch: TW={:?} VM={:?}",
        tw_body, vm_body);
}

/// Блок 4: TW vs VM — POST with upper builtin.
#[tokio::test]
async fn block4_tw_vs_vm_post_upper() {
    let json = serde_json::json!({"name": "metalogos"});

    // TW
    let (tw_port, tw_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Interpreter).await;
    let (tw_status, tw_body) = http_post_json(tw_port, "/greet", &json).await;
    tw_handle.abort();

    // VM
    let (vm_port, vm_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (vm_status, vm_body) = http_post_json(vm_port, "/greet", &json).await;
    vm_handle.abort();

    assert_eq!(tw_status, vm_status,
        "Block 4: POST /greet status mismatch: TW={} VM={}",
        tw_status, vm_status);
    assert_eq!(tw_body, vm_body,
        "Block 4: POST /greet body mismatch: TW={:?} VM={:?}",
        tw_body, vm_body);
}

/// Блок 4: TW vs VM — empty query param (default branch).
#[tokio::test]
async fn block4_tw_vs_vm_empty_query_param() {
    // TW
    let (tw_port, tw_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Interpreter).await;
    let (tw_status, tw_body) = http_get(tw_port, "/hello").await;
    tw_handle.abort();

    // VM
    let (vm_port, vm_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (vm_status, vm_body) = http_get(vm_port, "/hello").await;
    vm_handle.abort();

    assert_eq!(tw_status, vm_status, "Block 4: /hello no-param status mismatch");
    assert_eq!(tw_body, vm_body,
        "Block 4: /hello no-param body mismatch: TW={:?} VM={:?}",
        tw_body, vm_body);
}

/// Блок 4: TW vs VM — 404 for unknown route.
#[tokio::test]
async fn block4_tw_vs_vm_404() {
    // TW
    let (tw_port, tw_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Interpreter).await;
    let (tw_status, tw_body) = http_get(tw_port, "/nonexistent").await;
    tw_handle.abort();

    // VM
    let (vm_port, vm_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let (vm_status, vm_body) = http_get(vm_port, "/nonexistent").await;
    vm_handle.abort();

    assert_eq!(tw_status, vm_status, "Block 4: 404 status mismatch");
    assert_eq!(tw_body, vm_body, "Block 4: 404 body mismatch");
}

/// Блок 4: TW vs VM — 405 for wrong method.
#[tokio::test]
async fn block4_tw_vs_vm_405() {
    // TW
    let (tw_port, tw_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Interpreter).await;
    let url_tw = format!("http://127.0.0.1:{}/hello", tw_port);
    let client = reqwest::Client::new();
    let tw_resp = client.post(&url_tw).send().await.unwrap();
    let tw_status = tw_resp.status().as_u16();
    tw_handle.abort();

    // VM
    let (vm_port, vm_handle) =
        start_server(SOURCE_REALISTIC_ROUTES, ServeBackend::Vm).await;
    let url_vm = format!("http://127.0.0.1:{}/hello", vm_port);
    let vm_resp = client.post(&url_vm).send().await.unwrap();
    let vm_status = vm_resp.status().as_u16();
    vm_handle.abort();

    assert_eq!(tw_status, vm_status,
        "Block 4: 405 status mismatch: TW={} VM={}",
        tw_status, vm_status);
}

/// Блок 4: TW vs VM — kv_set/kv_get cross-backend parity.
/// Set a key via TW, read via VM (shared kv store).
#[tokio::test]
async fn block4_tw_vm_kv_shared_store() {
    // Start TW server, set a value
    let (tw_port, tw_handle) =
        start_server(SOURCE_KV_ROUTES, ServeBackend::Interpreter).await;
    let (tw_status, tw_body) =
        http_get(tw_port, "/set?key=n160-shared&val=tw-wrote-this").await;
    assert_eq!(tw_status, 200);
    assert_eq!(tw_body, "set n160-shared=tw-wrote-this");
    tw_handle.abort();

    // Start VM server, read the value (should see it if kv is truly shared)
    // NOTE: kv store is per-interpreter instance, NOT cross-server.
    // This test documents the actual behavior: each server instance
    // has its own interpreter and thus its own kv store.
    let (vm_port, vm_handle) =
        start_server(SOURCE_KV_ROUTES, ServeBackend::Vm).await;
    let (vm_status, vm_body) =
        http_get(vm_port, "/get?key=n160-shared").await;
    // kv_get returns empty string for missing key
    assert_eq!(vm_status, 200);
    // The VM server has its own interpreter, so kv is NOT shared
    // across server instances. This is expected — kv is in-memory.
    assert_eq!(vm_body, "",
        "Block 4: kv store should NOT be shared across server instances");
    vm_handle.abort();
}

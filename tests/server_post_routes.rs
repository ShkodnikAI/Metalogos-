#![cfg(feature = "server")]
// ── Наряд №2: POST/PUT/DELETE routes integration tests ──────────
// Verifies that non-GET HTTP methods are correctly routed through axum.
// Contract (Telegram webhook pattern):
//   GET  "/"      → "GET works"
//   POST "/hook"  → "POST works"
//   PUT  "/update"  → "PUT works"
//   DELETE "/remove" → "DELETE works"

/// Source with GET + POST (the Telegram webhook pattern).
const SOURCE_GET_POST: &str = r#"
mlogserver {
  port: 8090
  route "/" method=GET { return "GET works" }
  route "/hook" method=POST { return "POST works" }
}
"#;

/// Source with all 4 HTTP methods.
const SOURCE_ALL_METHODS: &str = r#"
mlogserver {
  port: 8090
  route "/" method=GET { return "GET works" }
  route "/hook" method=POST { return "POST works" }
  route "/update" method=PUT { return "PUT works" }
  route "/remove" method=DELETE { return "DELETE works" }
}
"#;

// ── Parsing contracts ──────────────────────────────────────────────

#[test]
fn test_parse_post_route_method() {
    let decls = metalogos::parser::parse(SOURCE_ALL_METHODS).unwrap();
    if let metalogos::ast::Declaration::MlogServer(srv) = &decls[0] {
        assert_eq!(srv.routes.len(), 4);
        assert_eq!(srv.routes[0].method, "GET");
        assert_eq!(srv.routes[0].path, "/");
        assert_eq!(srv.routes[1].method, "POST");
        assert_eq!(srv.routes[1].path, "/hook");
        assert_eq!(srv.routes[2].method, "PUT");
        assert_eq!(srv.routes[2].path, "/update");
        assert_eq!(srv.routes[3].method, "DELETE");
        assert_eq!(srv.routes[3].path, "/remove");
    } else {
        panic!("expected MlogServer declaration");
    }
}

#[test]
fn test_parse_post_route_body_has_return() {
    let source = r#"
mlogserver {
  port: 8090
  route "/hook" method=POST { return "POST works" }
}
"#;
    let decls = metalogos::parser::parse(source).unwrap();
    if let metalogos::ast::Declaration::MlogServer(srv) = &decls[0] {
        assert_eq!(srv.routes[0].body.len(), 1);
        match &srv.routes[0].body[0] {
            metalogos::ast::Statement::Return(expr) => match expr {
                metalogos::ast::Expr::StringLit(s) => assert_eq!(s, "POST works"),
                other => panic!("expected StringLit, got: {:?}", other),
            },
            other => panic!("expected Return statement, got: {:?}", other),
        }
    }
}

// ── Routing contracts (via live server) ───────────────────────────

#[tokio::test]
async fn test_get_route_works() {
    let (port, _handle) = metalogos::server::run_test_server(SOURCE_GET_POST)
        .await
        .expect("server should start");
    let url = format!("http://127.0.0.1:{}/", port);
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert_eq!(body, "GET works");
}

#[tokio::test]
async fn test_post_route_responds() {
    let (port, _handle) = metalogos::server::run_test_server(SOURCE_GET_POST)
        .await
        .expect("server should start");
    let url = format!("http://127.0.0.1:{}/hook", port);
    let client = reqwest::Client::new();
    let resp = client.post(&url).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert_eq!(body, "POST works");
}

#[tokio::test]
async fn test_post_on_get_only_route_returns_405() {
    let (port, _handle) = metalogos::server::run_test_server(SOURCE_GET_POST)
        .await
        .expect("server should start");
    let url = format!("http://127.0.0.1:{}/", port);
    let client = reqwest::Client::new();
    let resp = client.post(&url).send().await.unwrap();
    // axum returns 405 Method Not Allowed when method doesn't match
    assert_eq!(resp.status(), 405);
}

#[tokio::test]
async fn test_get_on_post_only_route_returns_405() {
    let (port, _handle) = metalogos::server::run_test_server(SOURCE_GET_POST)
        .await
        .expect("server should start");
    let url = format!("http://127.0.0.1:{}/hook", port);
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 405);
}

#[tokio::test]
async fn test_unknown_path_returns_404() {
    let (port, _handle) = metalogos::server::run_test_server(SOURCE_GET_POST)
        .await
        .expect("server should start");
    let url = format!("http://127.0.0.1:{}/nonexistent", port);
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 404);
}

// ── PUT route ─────────────────────────────────────────────────────

#[tokio::test]
async fn test_put_route_responds() {
    let (port, _handle) = metalogos::server::run_test_server(SOURCE_ALL_METHODS)
        .await
        .expect("server should start");
    let url = format!("http://127.0.0.1:{}/update", port);
    let client = reqwest::Client::new();
    let resp = client.put(&url).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert_eq!(body, "PUT works");
}

// ── DELETE route ──────────────────────────────────────────────────

#[tokio::test]
async fn test_delete_route_responds() {
    let (port, _handle) = metalogos::server::run_test_server(SOURCE_ALL_METHODS)
        .await
        .expect("server should start");
    let url = format!("http://127.0.0.1:{}/remove", port);
    let client = reqwest::Client::new();
    let resp = client.delete(&url).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert_eq!(body, "DELETE works");
}

#![cfg(feature = "server")]
// ── НАРЯД #161: VM cross-module route compilation ─────────────
// Bug: `mlog serve` with VM backend fails at startup when routes
// call patterns defined in imported modules.
//
// Helper files at project root (resolvable by Compiler::new() via cwd):
//   p161_route_helper.mlog  — defines HandleHelper
//   p161_deep_b.mlog        — imports p161_deep_c, defines DeepB
//   p161_deep_c.mlog        — defines DeepC (A→B→C chain)

use metalogos::ast::*;
use metalogos::compiler::Compiler;
use metalogos::server::ServeBackend;

// ═══════════════════════════════════════════════════════════════════
// БЛОК 1 — Компилятор: маршруты используют импортированные паттерны
// ═══════════════════════════════════════════════════════════════════

#[test]
fn block1_imported_pattern_in_route_compiles() {
    let source = r#"
import p161_route_helper

mlogserver {
  port: 0
  route "/" method=GET {
    let result = HandleHelper("test")
    respond("200", result)
  }
}
"#;
    let declarations = metalogos::parser::parse(source).expect("parse should succeed");
    let server_config = declarations
        .iter()
        .find_map(|d| match d {
            Declaration::MlogServer(s) => Some(s.clone()),
            _ => None,
        })
        .expect("should have mlogserver block");
    let mut compiler = Compiler::new();
    let _program = compiler
        .compile(declarations)
        .expect("compile should resolve p161_route_helper");
    let routes = compiler
        .compile_routes(&server_config.routes)
        .expect("compile_routes should find HandleHelper");
    assert_eq!(routes.len(), 1);
    assert!(!routes[0].code.is_empty(), "route should have bytecode");
}

#[test]
fn block1_qualified_call_in_route_compiles() {
    let source = r#"
import p161_route_helper as helper

mlogserver {
  port: 0
  route "/" method=GET {
    let result = helper.HandleHelper("test")
    respond("200", result)
  }
}
"#;
    let declarations = metalogos::parser::parse(source).expect("parse should succeed");
    let server_config = declarations
        .iter()
        .find_map(|d| match d {
            Declaration::MlogServer(s) => Some(s.clone()),
            _ => None,
        })
        .expect("should have mlogserver block");
    let mut compiler = Compiler::new();
    let _program = compiler
        .compile(declarations)
        .expect("compile should resolve p161_route_helper as helper");
    let _routes = compiler
        .compile_routes(&server_config.routes)
        .expect("compile_routes should find HandleHelper via qualified call");
}

#[test]
fn block1_no_import_produces_undefined_error() {
    let source = r#"
mlogserver {
  port: 0
  route "/" method=GET {
    let result = HandleHelper("test")
    respond("200", result)
  }
}
"#;
    let declarations = metalogos::parser::parse(source).expect("parse should succeed");
    let server_config = declarations
        .iter()
        .find_map(|d| match d {
            Declaration::MlogServer(s) => Some(s.clone()),
            _ => None,
        })
        .expect("should have mlogserver block");
    let mut compiler = Compiler::new();
    let _program = compiler
        .compile(declarations)
        .expect("compile without import should succeed");
    let result = compiler.compile_routes(&server_config.routes);
    assert!(result.is_err(), "should fail without import");
    let err = result.unwrap_err();
    assert!(
        err.contains("undefined function"),
        "expected 'undefined function', got: {}",
        err
    );
    assert!(
        err.contains("HandleHelper"),
        "expected 'HandleHelper', got: {}",
        err
    );
}

// ═══════════════════════════════════════════════════════════════════
// БЛОК 1 — A→B→C цепочка импортов
// ═══════════════════════════════════════════════════════════════════

#[test]
fn block1_transitive_import_chain_a_b_c() {
    let source = r#"
import p161_deep_b

mlogserver {
  port: 0
  route "/" method=GET {
    let result = DeepB("hello")
    respond("200", result)
  }
}
"#;
    let declarations = metalogos::parser::parse(source).expect("parse should succeed");
    let server_config = declarations
        .iter()
        .find_map(|d| match d {
            Declaration::MlogServer(s) => Some(s.clone()),
            _ => None,
        })
        .expect("should have mlogserver block");
    let mut compiler = Compiler::new();
    let _program = compiler
        .compile(declarations)
        .expect("compile should resolve A→B→C chain");
    let routes = compiler
        .compile_routes(&server_config.routes)
        .expect("compile_routes should find DeepB (which calls DeepC)");
    assert_eq!(routes.len(), 1);
}

// ═══════════════════════════════════════════════════════════════════
// БЛОК 3 — Контракт через реальный HTTP: TW и VM дают одинаковый ответ
// ═══════════════════════════════════════════════════════════════════

const SOURCE_WITH_IMPORT: &str = r#"
import p161_route_helper

mlogserver {
  port: 0
  route "/" method=GET {
    let result = HandleHelper("world")
    respond("200", result)
  }
}
"#;

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
    let resp = reqwest::get(&url).await.expect("GET should succeed");
    let status = resp.status().as_u16();
    let body = resp.text().await.expect("body should be readable");
    (status, body)
}

#[tokio::test]
async fn block3_tw_serves_imported_pattern() {
    let (port, _handle) = start_server(SOURCE_WITH_IMPORT, ServeBackend::Interpreter).await;
    let (status, body) = http_get(port, "/").await;
    assert_eq!(status, 200, "TW should return 200");
    assert_eq!(
        body, "handled: world",
        "TW should call imported HandleHelper"
    );
}

#[tokio::test]
async fn block3_vm_serves_imported_pattern() {
    let (port, _handle) = start_server(SOURCE_WITH_IMPORT, ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/").await;
    assert_eq!(status, 200, "VM should return 200");
    assert_eq!(
        body, "handled: world",
        "VM should call imported HandleHelper"
    );
}

#[tokio::test]
async fn block3_tw_vm_parity_imported_pattern() {
    let (tw_port, tw_handle) = start_server(SOURCE_WITH_IMPORT, ServeBackend::Interpreter).await;
    let (tw_status, tw_body) = http_get(tw_port, "/").await;
    tw_handle.abort();
    let (vm_port, vm_handle) = start_server(SOURCE_WITH_IMPORT, ServeBackend::Vm).await;
    let (vm_status, vm_body) = http_get(vm_port, "/").await;
    vm_handle.abort();
    assert_eq!(tw_status, vm_status, "TW/VM status mismatch");
    assert_eq!(
        tw_body, vm_body,
        "TW/VM body mismatch: TW={:?} VM={:?}",
        tw_body, vm_body
    );
}

#[tokio::test]
async fn block3_vm_transitive_import_chain() {
    let source = r#"
import p161_deep_b

mlogserver {
  port: 0
  route "/" method=GET {
    let result = DeepB("chain")
    respond("200", result)
  }
}
"#;
    let (port, _handle) = start_server(source, ServeBackend::Vm).await;
    let (status, body) = http_get(port, "/").await;
    assert_eq!(status, 200, "VM A→B→C chain should return 200");
    assert_eq!(body, "deep-c: chain", "VM should resolve A→B→C");
}

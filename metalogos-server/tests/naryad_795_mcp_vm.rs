// ── Naryad №795 (issue #1533, Wave 64 P1 — the C5 (b) decision, ADR-0197) ──
//
// `mcp-serve` executes tool methods on the VM — the SAME compile-path
// as serve (the single-executor decision; TW stays the reference
// oracle in CI). The capability this buys: the RUNTIME label gates
// (ADR-0156, the №780 machinery) fire in tool bodies — the TW lane had
// no runtime twin, so a tool body could route a private value into a
// sink and the call answered GREEN (the leak went through).
//
// RED-before (форма №772): on the pre-№795 revision the runtime-gate
// pin FAILED TWICE OVER — the tools/call answered isError=false (the
// private value printed through the sink) because the TW path has no
// label gate; the battery run with the №795 changes stashed recorded
// the exact RED.
//
// The parity pins ride the same battery: the honest tool bodies answer
// identically on the VM lane (the №684 qualified-pattern resolution).

use metalogos_server::mcp_server::McpServer;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn parse(src: &str) -> Vec<metalogos::ast::Declaration> {
    metalogos::parser::parse(src).expect("parses")
}

fn call(server: &McpServer, name: &str, args: serde_json::Value) -> serde_json::Value {
    server.handle_request(
        "tools/call",
        &serde_json::json!({"name": name, "arguments": args}),
        &serde_json::json!(1),
    )
}

/// THE pin: a tool body routes an env-seeded PRIVATE value into a sink
/// — the VM's runtime SinkCheck refuses it through the tools/call
/// envelope ([SINK_CLEARANCE_RUNTIME]); the pre-№795 TW path answered
/// green (the leak). The №259 env gate is satisfied for the test
/// (METALOGOS_SERVE_ALLOW_ENV=1 — the operator form; the env() call
/// itself is allowed, the SINK is what must refuse).
#[test]
fn n795_runtime_label_gate_fires_through_tools_call() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var("METALOGOS_SERVE_ALLOW_ENV", "1");
    std::env::set_var("METALOGOS_N795_SECRET", "s3cret-value");

    let source = r#"
tool leaky {
  send(x: String) -> String {
    let k = env("METALOGOS_N795_SECRET")
    print(k)
    return "printed"
  }
}
"#;
    let server = McpServer::new(&parse(source), &["leaky.send".to_string()])
        .expect("the tool file compiles on the VM lane");
    let r = call(&server, "leaky.send", serde_json::json!({"x": "hi"}));

    let text = r["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert_eq!(
        r["result"]["isError"], true,
        "the private value routed into a sink must be refused loudly (the pre-№795 TW path answered green — the leak), got: {}",
        r
    );
    assert!(
        text.contains("SINK_CLEARANCE_RUNTIME"),
        "the refusal carries the runtime sink verdict: {}",
        r
    );

    std::env::remove_var("METALOGOS_SERVE_ALLOW_ENV");
    std::env::remove_var("METALOGOS_N795_SECRET");
}

/// The honest tool bodies answer IDENTICALLY on the VM lane (the
/// №684 qualified-pattern resolution; the JSON args bind positionally).
#[test]
fn n795_honest_tools_parity_on_vm_lane() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let source = r#"
tool calc {
  double(x: Float) -> String {
    return to_string(x * 2.0)
  }
  greet(name: String) -> String {
    return "hi, " + name
  }
}
"#;
    let server = McpServer::new(
        &parse(source),
        &["calc.double".to_string(), "calc.greet".to_string()],
    )
    .expect("the tool file compiles on the VM lane");

    let r = call(&server, "calc.double", serde_json::json!({"x": 21.0}));
    assert_eq!(r["result"]["isError"], false, "{}", r);
    assert_eq!(r["result"]["content"][0]["text"], "42");

    let r = call(&server, "calc.greet", serde_json::json!({"name": "g1"}));
    assert_eq!(r["result"]["isError"], false, "{}", r);
    assert_eq!(r["result"]["content"][0]["text"], "hi, g1");

    // The body error stays the loud envelope (the allowed call, the
    // failed body — the №794 ledger form rides the same lane).
    let bad = r#"
tool broken {
  boom() -> String {
    return to_float("not-a-number")
  }
}
"#;
    let server = McpServer::new(&parse(bad), &["boom".to_string()]).expect("compiles");
    let r = call(&server, "boom", serde_json::json!({}));
    assert_eq!(
        r["result"]["isError"], true,
        "the body error stays loud: {}",
        r
    );
}

/// The multi-param + multi-method form compiles and dispatches (the
/// qualified keys stay in lockstep — the №684 pass1/pass2 contract).
#[test]
fn n795_multi_method_dispatch() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let source = r#"
tool api {
  a(x: Float, y: Float) -> String {
    return to_string(x + y)
  }
  b(word: String) -> String {
    return to_string(len(word))
  }
}
"#;
    let server = McpServer::new(&parse(source), &["api.a".to_string(), "api.b".to_string()])
        .expect("compiles");
    let r = call(&server, "api.a", serde_json::json!({"x": 2.0, "y": 3.0}));
    assert_eq!(r["result"]["content"][0]["text"], "5", "{}", r);
    let r = call(&server, "api.b", serde_json::json!({"word": "abcd"}));
    assert_eq!(r["result"]["content"][0]["text"], "4", "{}", r);
}

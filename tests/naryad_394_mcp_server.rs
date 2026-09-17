// ── Naryad #394 (P1, feature/mcp, ADR-0168): MCP server transports ────
//
// Contract tests for the server half of the MCP contour:
//   - the tool-policy is COMPILED from the profile (№316 SSOT over the
//     method body's builtin calls) — no manual YAML;
//   - stdio request core: initialize / tools/list (with the policy
//     _meta) / tools/call / unknown-tool refusal;
//   - HTTP transport: an external JSON-RPC client walks tools/list and
//     tools/call over HTTP POST (the "external client" acceptance);
//   - SSE transport: the endpoint event + a response delivered on the
//     session stream (the MCP HTTP+SSE shape);
//   - the security matrix is GREEN ON BOTH TRANSPORTS: fail-closed
//     allowlist, exec gate inside a tool method, bearer auth 401/200 —
//     the request core is single-sourced, so stdio and http/sse cannot
//     drift (pinned here anyway).

use metalogos::mcp_server::{McpAuth, McpServer};

const TOOL_SOURCE: &str = r#"
tool notify {
  send(message: String, channel: String) -> String {
    write_file("target/n394_mcp_test_out.txt", "to " + channel + ": " + message)
    return "sent:" + message
  }
  stats(rows: Float) -> String {
    return "rows:" + to_string(rows)
  }
}
"#;

const EXEC_TOOL_SOURCE: &str = r#"
tool runner {
  run(cmd: String) -> String {
    return exec(cmd)
  }
}
"#;

fn parse(src: &str) -> Vec<metalogos::ast::Declaration> {
    metalogos::parser::parse(src).expect("parses")
}

fn jsonrpc_request(method: &str, params: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    })
}

// ── 1. The compiled tool policy (no manual YAML) ──────────────────────

#[test]
fn tool_policy_is_compiled_from_the_classification() {
    let decls = parse(TOOL_SOURCE);
    let server = McpServer::new(&decls, &["notify.send".to_string()]).expect("server builds");

    let resp = server.handle_request(
        "tools/list",
        &serde_json::json!({}),
        &serde_json::Value::Null,
    );
    let tools = resp["result"]["tools"].as_array().expect("tools array");
    assert_eq!(tools.len(), 1);
    let tool = &tools[0];
    assert_eq!(tool["name"], "notify.send");

    let policy = &tool["_meta"]["metalogos.dev/policy"];
    assert_eq!(policy["version"], 1);
    // notify.send calls write_file — a Sink of the "file" class per the
    // audit::sink_kind SSOT.
    let sinks = policy["sink_calls"].as_array().expect("sink_calls");
    assert_eq!(sinks.len(), 1, "exactly one sink call: {}", policy);
    assert_eq!(sinks[0]["builtin"], "write_file");
    assert_eq!(sinks[0]["class"], "file");
    // Both tool params flow into the sink's arguments — clearance args.
    let clearance = policy["clearance_args"].as_array().expect("clearance");
    assert!(
        clearance.contains(&serde_json::json!("message")),
        "{}",
        policy
    );
    assert!(
        clearance.contains(&serde_json::json!("channel")),
        "{}",
        policy
    );
    // write_file is Reversible in the №316 map.
    assert_eq!(policy["irreversible"], false);
    assert!(policy["compiled_from"].is_string());
}

#[test]
fn tool_policy_marks_irreversible_and_exec_class() {
    let src = r#"
tool admin {
  wipe(table: String) -> String {
    return db_execute("DROP TABLE " + table)
  }
}
"#;
    let decls = parse(src);
    let server = McpServer::new(&decls, &["admin.wipe".to_string()]).expect("server builds");
    let resp = server.handle_request(
        "tools/list",
        &serde_json::json!({}),
        &serde_json::Value::Null,
    );
    let policy = &resp["result"]["tools"][0]["_meta"]["metalogos.dev/policy"];
    assert_eq!(policy["sink_calls"][0]["builtin"], "db_execute");
    assert_eq!(policy["sink_calls"][0]["class"], "db");
    assert_eq!(policy["irreversible"], true, "DROP is irreversible");
    let clearance = policy["clearance_args"].as_array().expect("clearance");
    assert!(
        clearance.contains(&serde_json::json!("table")),
        "{}",
        policy
    );
}

// ── 2. The stdio request core ─────────────────────────────────────────

#[test]
fn stdio_core_handles_initialize_list_call_and_refusals() {
    let decls = parse(TOOL_SOURCE);
    let server = McpServer::new(&decls, &["notify.send".to_string(), "stats".to_string()])
        .expect("server builds");

    // initialize
    let init = server.handle_request("initialize", &serde_json::json!({}), &serde_json::json!(1));
    assert_eq!(init["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(init["result"]["serverInfo"]["name"], "metalogos-mcp-server");

    // tools/call by FULL name
    let call = server.handle_request(
        "tools/call",
        &serde_json::json!({"name": "notify.send", "arguments": {"message": "hi", "channel": "ops"}}),
        &serde_json::json!(2),
    );
    assert_eq!(call["result"]["isError"], false);
    assert_eq!(call["result"]["content"][0]["text"], "sent:hi");

    // tools/call by METHOD name (the allowlist also accepts bare names)
    let call2 = server.handle_request(
        "tools/call",
        &serde_json::json!({"name": "stats", "arguments": {"rows": 7}}),
        &serde_json::json!(3),
    );
    assert_eq!(call2["result"]["content"][0]["text"], "rows:7");

    // Unknown tool → typed JSON-RPC refusal (fail-closed surface).
    let unknown = server.handle_request(
        "tools/call",
        &serde_json::json!({"name": "notify.panic", "arguments": {}}),
        &serde_json::json!(4),
    );
    assert_eq!(unknown["error"]["code"], -32602);
}

#[test]
fn empty_allowlist_is_fail_closed_on_the_core() {
    let decls = parse(TOOL_SOURCE);
    let err = McpServer::new(&decls, &[])
        .err()
        .expect("empty allowlist refuses");
    assert!(
        err.contains("fail-closed"),
        "the refusal must name the fail-closed rule: {}",
        err
    );
}

// ── 3. The exec gate inside a tool method (transport-independent) ─────

#[test]
fn exec_gate_refuses_inside_a_tool_method_on_the_core() {
    // The tool method runs through the interpreter — the №253-А exec
    // gate applies verbatim, on every transport (the core is shared;
    // the HTTP matrix below pins the same refusal over the wire).
    let decls = parse(EXEC_TOOL_SOURCE);
    let server = McpServer::new(&decls, &["runner.run".to_string()]).expect("server builds");
    let call = server.handle_request(
        "tools/call",
        &serde_json::json!({"name": "runner.run", "arguments": {"cmd": "echo hi"}}),
        &serde_json::json!(9),
    );
    // Default: METALOGOS_ALLOW_EXEC unset → EXEC_NOT_PERMITTED refusal
    // surfaces as the tool's isError result (loud, not a silent pass).
    let text = call["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert_eq!(call["result"]["isError"], true, "{}", call);
    assert!(
        text.contains("EXEC_NOT_PERMITTED") || text.contains("Error"),
        "the exec refusal must be loud: {}",
        text
    );
}

// ── 4. HTTP + SSE transports (feature `server`, default-on) ───────────

#[cfg(feature = "server")]
mod network {
    use super::*;
    use metalogos::mcp_server::run_test_mcp_server;

    async fn post_json(
        port: u16,
        path: &str,
        token: Option<&str>,
        body: serde_json::Value,
    ) -> (u16, serde_json::Value) {
        let addr = format!("http://127.0.0.1:{}{}", port, path);
        let client = reqwest::Client::new();
        let mut req = client
            .post(&addr)
            .header("Content-Type", "application/json");
        if let Some(t) = token {
            req = req.header("Authorization", format!("Bearer {}", t));
        }
        let resp = req.body(body.to_string()).send().await.expect("POST works");
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        let json = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    async fn get_status(port: u16, path: &str, token: Option<&str>) -> u16 {
        let addr = format!("http://127.0.0.1:{}{}", port, path);
        let client = reqwest::Client::new();
        let mut req = client.get(&addr);
        if let Some(t) = token {
            req = req.header("Authorization", format!("Bearer {}", t));
        }
        // A short timeout: the SSE stream stays open — we only assert
        // the handshake status, then drop the request.
        req.timeout(std::time::Duration::from_millis(500))
            .send()
            .await
            .map(|r| r.status().as_u16())
            .unwrap_or(0)
    }

    #[tokio::test]
    async fn external_client_walks_tools_over_http() {
        let decls = parse(TOOL_SOURCE);
        let (port, handle) = run_test_mcp_server(
            &decls,
            &["notify.send".to_string(), "stats".to_string()],
            &McpAuth::OpenLocal,
        )
        .await
        .expect("test server starts");

        // (а) The external JSON-RPC client gets tools/list over HTTP.
        let (status, list) = post_json(
            port,
            "/mcp",
            None,
            jsonrpc_request("tools/list", serde_json::json!({})),
        )
        .await;
        assert_eq!(status, 200);
        let tools = list["result"]["tools"].as_array().expect("tools over http");
        assert_eq!(tools.len(), 2);
        let policy = &tools[0]["_meta"]["metalogos.dev/policy"];
        assert_eq!(policy["sink_calls"][0]["builtin"], "write_file");

        // ...and calls a tool over HTTP — the result executes.
        let (status, call) = post_json(
            port,
            "/mcp",
            None,
            jsonrpc_request(
                "tools/call",
                serde_json::json!({"name": "notify.send", "arguments": {"message": "hi", "channel": "ops"}}),
            ),
        )
        .await;
        assert_eq!(status, 200);
        assert_eq!(call["result"]["content"][0]["text"], "sent:hi");

        // Unknown tool over HTTP → the same typed refusal.
        let (status, unknown) = post_json(
            port,
            "/mcp",
            None,
            jsonrpc_request("tools/call", serde_json::json!({"name": "nope"})),
        )
        .await;
        assert_eq!(status, 200);
        assert_eq!(unknown["error"]["code"], -32602);

        handle.abort();
    }

    #[tokio::test]
    async fn bearer_auth_matrix_on_http_and_sse() {
        let decls = parse(TOOL_SOURCE);
        let (port, handle) = run_test_mcp_server(
            &decls,
            &["notify.send".to_string()],
            &McpAuth::Bearer("s3cret".to_string()),
        )
        .await
        .expect("test server starts");

        // No token → 401 (loud, fail-closed).
        let (status, _) = post_json(
            port,
            "/mcp",
            None,
            jsonrpc_request("tools/list", serde_json::json!({})),
        )
        .await;
        assert_eq!(status, 401);

        // Wrong token → 401.
        let (status, _) = post_json(
            port,
            "/mcp",
            Some("wrong"),
            jsonrpc_request("tools/list", serde_json::json!({})),
        )
        .await;
        assert_eq!(status, 401);

        // Correct token → the full contract works.
        let (status, list) = post_json(
            port,
            "/mcp",
            Some("s3cret"),
            jsonrpc_request("tools/list", serde_json::json!({})),
        )
        .await;
        assert_eq!(status, 200);
        assert_eq!(list["result"]["tools"].as_array().expect("tools").len(), 1);

        // The SSE handshake obeys the same gate: 401 without the token.
        let s = get_status(port, "/sse", None).await;
        assert_eq!(s, 401, "SSE without the token must be refused");

        handle.abort();
    }

    #[tokio::test]
    async fn sse_transport_delivers_the_response_on_the_session_stream() {
        // The MCP HTTP+SSE shape: GET /sse → `endpoint` event; the client
        // POSTs JSON-RPC to that endpoint and reads the response as a
        // `message` event on the same stream.
        let decls = parse(TOOL_SOURCE);
        let (port, handle) =
            run_test_mcp_server(&decls, &["notify.send".to_string()], &McpAuth::OpenLocal)
                .await
                .expect("test server starts");

        let client = reqwest::Client::new();
        let mut stream = client
            .get(format!("http://127.0.0.1:{}/sse", port))
            .send()
            .await
            .expect("SSE connects");
        assert_eq!(stream.status().as_u16(), 200);
        let ct = stream
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        assert!(
            ct.starts_with("text/event-stream"),
            "SSE content-type: {}",
            ct
        );

        // Read SSE frames until the mandatory `endpoint` event arrives.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut buf = String::new();
        let mut endpoint_path = String::new();
        while std::time::Instant::now() < deadline {
            let done = buf.contains("event: endpoint") && buf.contains("data: /mcp?session=");
            if done {
                break;
            }
            if let Ok(Some(chunk)) = stream.chunk().await {
                buf.push_str(&String::from_utf8_lossy(&chunk));
            } else {
                break;
            }
        }
        for line in buf.lines() {
            if let Some(rest) = line.strip_prefix("data: ") {
                if rest.starts_with("/mcp?session=") {
                    endpoint_path = rest.to_string();
                }
            }
        }
        assert!(
            !endpoint_path.is_empty(),
            "the endpoint event must carry the POST path: {:?}",
            buf
        );

        // POST the JSON-RPC call to the session endpoint (202 accepted;
        // the response arrives on the stream as a `message` event).
        let post_addr = format!("http://127.0.0.1:{}{}", port, endpoint_path);
        let call_body = jsonrpc_request(
            "tools/call",
            serde_json::json!({"name": "notify.send", "arguments": {"message": "hi", "channel": "ops"}}),
        );
        let pr = client
            .post(&post_addr)
            .header("Content-Type", "application/json")
            .body(call_body.to_string())
            .send()
            .await
            .expect("POST to the session endpoint works");
        assert_eq!(pr.status().as_u16(), 202);

        // Read the stream until the response event arrives.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if std::time::Instant::now() > deadline {
                panic!("no message event within the deadline; buffer: {:?}", buf);
            }
            if buf.contains("\"sent:hi\"") {
                break;
            }
            match stream.chunk().await {
                Ok(Some(chunk)) => buf.push_str(&String::from_utf8_lossy(&chunk)),
                Ok(None) => break,
                Err(e) => panic!("SSE read error: {}", e),
            }
        }
        assert!(
            buf.contains("event: message"),
            "the response must ride a message event: {:?}",
            buf
        );
        assert!(
            buf.contains("\"sent:hi\""),
            "the tool result must reach the stream: {:?}",
            buf
        );

        handle.abort();
    }
}

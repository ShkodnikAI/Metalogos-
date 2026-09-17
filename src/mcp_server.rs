// ── MCP Server: Metalogos as MCP tool provider (Наряд №297, issue #361) ──
//
// JSON-RPC 2.0 server that exposes user `tool` constructs from a .mlog
// file as MCP tools. This is the reverse of the MCP client (Наряд №268,
// ADR-0132) — Metalogos IS the tool server.
//
// Transports (Naryad #394, ADR-0168 — the server half of ADR-0132):
//   stdio (default) — newline-framed JSON-RPC 2.0, behavior identical to
//     №297 (std-only, always available);
//   http            — JSON-RPC over HTTP POST /mcp (feature `server`);
//   sse             — the MCP HTTP+SSE transport: GET /sse event stream
//     + POST /mcp (feature `server`).
//
// Tool policy (Naryad #394, ADR-0168 §4): the per-tool policy block in
// `tools/list` `_meta["metalogos.dev/policy"]` is COMPILED from the
// program contour via the №316 SSOT classification (src/mcp_policy.rs)
// — no manual YAML.
//
// Fail-closed: without `--allowlist`, the server refuses to start — on
// EVERY transport. The same allowlist/exec/env gates, label clearance
// and taint rules apply to tool execution regardless of transport: the
// request core (`McpServer::handle_request`) is transport-agnostic and
// single-sourced.

use crate::ast::{Declaration, Param};
use crate::interpreter::Interpreter;
use crate::interpreter::Value;
use std::io::{self, BufRead, Write};

/// Maximum response size (bytes) before truncation.
const MAX_RESPONSE_SIZE: usize = 1_000_000;

/// Auth modes for the network transports (Naryad #394 §4).
#[derive(Debug, Clone)]
pub enum McpAuth {
    /// stdio: no auth surface — the process boundary is the boundary.
    None,
    /// Every request must carry `Authorization: Bearer <token>`.
    Bearer(String),
    /// No token: localhost-only bind is the accepted alternative; a
    /// non-loopback bind without a token is a loud WARN (the №263
    /// bind-WARN posture), never a silent open port.
    OpenLocal,
}

/// Options for the network transports.
#[derive(Debug, Clone, Default)]
pub struct McpServeOpts {
    /// Bearer token (from --auth-token or METALOGOS_MCP_AUTH_TOKEN).
    pub auth_token: Option<String>,
    /// Suppress nothing — reserved for future knobs.
    pub _reserved: (),
}

/// The transport-agnostic MCP server core: built once per process, then
/// asked per request. Both the stdio loop and the network transports
/// dispatch through `handle_request` — the security surface cannot drift
/// between transports by construction.
pub struct McpServer {
    tools: Vec<ExposedTool>,
    /// The declarations the tools come from (methods execute against
    /// them). Not exposed over the wire.
    declarations: Vec<Declaration>,
}

impl McpServer {
    /// Build the server: fail-closed on an empty allowlist (№297
    /// posture, unchanged — on every transport).
    pub fn new(declarations: &[Declaration], allowlist: &[String]) -> Result<McpServer, String> {
        if allowlist.is_empty() {
            return Err("mcp-serve: --allowlist is required (fail-closed). \
                 Specify tool names to expose, e.g., --allowlist math_api.send"
                .to_string());
        }

        // Collect exposed tools (filtered by allowlist).
        let tools = collect_exposed_tools(declarations, allowlist);
        if tools.is_empty() {
            return Err(format!(
                "mcp-serve: no tools from allowlist {:?} found in the .mlog file. \
                 Check tool/method names.",
                allowlist
            ));
        }

        eprintln!(
            "[mcp-serve] exposing {} tool(s): {}",
            tools.len(),
            tools
                .iter()
                .map(|t| t.full_name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        );

        Ok(McpServer {
            tools,
            declarations: declarations.to_vec(),
        })
    }

    /// The JSON-RPC request core (transport-agnostic).
    pub fn handle_request(
        &self,
        method: &str,
        params: &serde_json::Value,
        id: &serde_json::Value,
    ) -> serde_json::Value {
        match method {
            "initialize" => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {
                        "name": "metalogos-mcp-server",
                        "version": env!("CARGO_PKG_VERSION")
                    },
                    "capabilities": {
                        "tools": {}
                    }
                }
            }),

            "notifications/initialized" => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {}
            }),

            "tools/list" => {
                let tool_list: Vec<serde_json::Value> = self
                    .tools
                    .iter()
                    .map(|t| {
                        let properties = t.params.iter().map(|p| {
                            (p.name.clone(), serde_json::json!({
                                "type": json_type_for(&p.type_name),
                                "description": format!("Parameter {} of type {}", p.name, p.type_name)
                            }))
                        }).collect::<serde_json::Map<_, _>>();
                        // Naryad #394: the compiled tool policy rides in
                        // _meta — mechanical, from the №316 SSOT.
                        let meta = serde_json::json!({
                            crate::mcp_policy::ToolPolicy::META_KEY: {
                                "version": crate::mcp_policy::ToolPolicy::VERSION,
                                "sink_calls": t.policy.sink_calls,
                                "source_calls": t.policy.source_calls,
                                "clearance_args": t.policy.clearance_args,
                                "irreversible": t.policy.irreversible,
                                "unclassified": t.policy.unclassified,
                                "compiled_from": "spec!-registry + No316 classification (auto; no manual YAML)",
                            }
                        });
                        serde_json::json!({
                            "name": t.full_name,
                            "description": t.description,
                            "inputSchema": {
                                "type": "object",
                                "properties": properties,
                                "required": t.params.iter().map(|p| p.name.clone()).collect::<Vec<_>>()
                            },
                            "_meta": meta,
                        })
                    })
                    .collect();
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {"tools": tool_list}
                })
            }

            "tools/call" => {
                let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let arguments = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);

                // Find the tool method by name.
                let tool = self
                    .tools
                    .iter()
                    .find(|t| t.full_name == tool_name || t.method_name == tool_name);
                if tool.is_none() {
                    return serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {"code": -32602, "message": format!("Unknown tool: {} (not in allowlist)", tool_name)}
                    });
                }
                let Some(tool) = tool else {
                    return serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {"code": -32602, "message": format!("Tool not in allowlist: {}", tool_name)}
                    });
                };

                // Execute the tool method body.
                match execute_tool_method(
                    &self.declarations,
                    &tool.tool_name,
                    &tool.method_name,
                    &arguments,
                ) {
                    Ok(result) => {
                        let result_str = if result.len() > MAX_RESPONSE_SIZE {
                            format!(
                                "{}... (truncated, {} bytes)",
                                &result[..MAX_RESPONSE_SIZE],
                                result.len()
                            )
                        } else {
                            result
                        };
                        serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "content": [{"type": "text", "text": result_str}],
                                "isError": false
                            }
                        })
                    }
                    Err(e) => serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{"type": "text", "text": format!("Error: {}", e)}],
                            "isError": true
                        }
                    }),
                }
            }

            _ => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32601, "message": format!("Method not found: {}", method)}
            }),
        }
    }
}

/// Run the MCP server loop over stdio: read JSON-RPC from stdin, write
/// to stdout. Behavior identical to №297 (the default transport).
pub fn run_mcp_server(declarations: &[Declaration], allowlist: &[String]) -> Result<(), String> {
    let server = McpServer::new(declarations, allowlist)?;

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line.map_err(|e| format!("stdin read error: {}", e))?;
        if line.is_empty() {
            continue;
        }

        let req: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let resp = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {"code": -32700, "message": format!("Parse error: {}", e)}
                });
                writeln!(stdout, "{}", resp).ok();
                stdout.flush().ok();
                continue;
            }
        };

        let id = req.get("id").cloned().unwrap_or(serde_json::Value::Null);
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = req
            .get("params")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        let response = server.handle_request(method, &params, &id);
        writeln!(stdout, "{}", response).ok();
        stdout.flush().ok();
    }

    Ok(())
}

struct ExposedTool {
    full_name: String,
    tool_name: String,
    method_name: String,
    params: Vec<Param>,
    description: String,
    /// Naryad #394: the compiled policy (№316 SSOT, no manual YAML).
    policy: crate::mcp_policy::ToolPolicy,
}

fn collect_exposed_tools(declarations: &[Declaration], allowlist: &[String]) -> Vec<ExposedTool> {
    let mut exposed = Vec::new();
    for decl in declarations {
        if let Declaration::Tool(t) = decl {
            for method in &t.methods {
                let full_name = format!("{}.{}", t.name, method.name);
                if allowlist
                    .iter()
                    .any(|a| a == &full_name || a == &method.name)
                {
                    exposed.push(ExposedTool {
                        full_name: full_name.clone(),
                        tool_name: t.name.clone(),
                        method_name: method.name.clone(),
                        params: method.params.clone(),
                        description: format!(
                            "Metalogos tool {}.{} — returns {}",
                            t.name, method.name, method.return_type
                        ),
                        policy: crate::mcp_policy::compile_policy(method),
                    });
                }
            }
        }
    }
    exposed
}

fn execute_tool_method(
    declarations: &[Declaration],
    tool_name: &str,
    method_name: &str,
    arguments: &serde_json::Value,
) -> Result<String, String> {
    // Find the tool declaration.
    let tool_decl = declarations
        .iter()
        .find_map(|d| {
            if let Declaration::Tool(t) = d {
                if t.name == tool_name {
                    return Some(t);
                }
            }
            None
        })
        .ok_or_else(|| format!("Tool not found: {}", tool_name))?;

    // Find the method.
    let method = tool_decl
        .methods
        .iter()
        .find(|m| m.name == method_name)
        .ok_or_else(|| format!("Method not found: {}.{}", tool_name, method_name))?;

    // Build interpreter with declarations (load patterns/templates/etc).
    let mut interp = Interpreter::new();
    interp.set_base_dir(std::path::PathBuf::from("."));
    let _ = interp.run(
        declarations
            .iter()
            .filter(|d| !matches!(d, Declaration::Tool(_)))
            .cloned()
            .collect::<Vec<_>>(),
    );

    // Convert JSON arguments to a Value env (param_name → value).
    let mut env = std::collections::HashMap::new();
    if arguments.is_object() {
        for p in &method.params {
            let json_val = arguments.get(&p.name).unwrap_or(&serde_json::Value::Null);
            env.insert(p.name.clone(), json_to_value(json_val));
        }
    }

    // Execute the method body via eval_statements (pub(crate) on Interpreter).
    let result = interp.eval_statements(&method.body, &mut env)?;
    Ok(format!("{}", result))
}

fn json_to_value(json: &serde_json::Value) -> Value {
    match json {
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::String(n.to_string())
            }
        }
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Array(arr) => Value::List(arr.iter().map(json_to_value).collect()),
        serde_json::Value::Object(obj) => {
            let mut fields = std::collections::HashMap::new();
            for (k, v) in obj {
                fields.insert(k.clone(), json_to_value(v));
            }
            Value::Struct {
                type_name: "Object".to_string(),
                fields,
            }
        }
        serde_json::Value::Null => Value::Unit,
    }
}

fn json_type_for(type_name: &str) -> &'static str {
    match type_name {
        "String" => "string",
        "Float" | "Int" => "number",
        "Bool" => "boolean",
        "List" => "array",
        _ => "string", // conservative default
    }
}

// ── Network transports (Naryad #394, ADR-0168; feature `server`) ──────
//
// axum + tokio are ALREADY workspace dependencies (the mlogserver stack,
// feature `server`, default-on) — the FEATURE_INTAKE §5 delta of this
// naryad is ZERO new crates. Minimal builds without `server` keep the
// stdio transport and refuse the network transports loudly (main.rs).

/// Shared per-session outbox for the SSE transport: server→client
/// messages (responses to POSTs) are pushed here and drained by the
/// GET /sse stream.
#[cfg(feature = "server")]
type SseOutbox = tokio::sync::mpsc::UnboundedSender<serde_json::Value>;
/// Run the MCP server over HTTP and/or SSE (Naryad #394). Blocks until
/// the listener dies. `bind` is `addr:port` (loopback default); auth per
/// `McpAuth` — Bearer token required on EVERY request when configured;
/// an unauthenticated non-loopback bind is a loud WARN, never silent.
#[cfg(feature = "server")]
pub fn run_mcp_server_network(
    declarations: &[Declaration],
    allowlist: &[String],
    transport: &str,
    bind: &str,
    auth: &McpAuth,
) -> Result<(), String> {
    let server = std::sync::Arc::new(McpServer::new(declarations, allowlist)?);

    // ── The loud auth posture (№263 WARN precedent, Naryad #394 §4) ──
    match auth {
        McpAuth::Bearer(_) => {
            eprintln!(
                "[mcp-serve] transport {} on {}: auth=bearer (every request requires \
                 Authorization: Bearer <token>)",
                transport, bind
            );
        }
        McpAuth::OpenLocal => {
            let host = bind.rsplit_once(':').map(|(h, _)| h).unwrap_or(bind);
            if host == "0.0.0.0" || host == "::" || host.is_empty() {
                eprintln!(
                    "  WARNING: mcp-serve {} transport binds to {} with NO auth — \
                     reachable from all network interfaces. Set --auth-token or bind \
                     127.0.0.1 for local-only access.",
                    transport, bind
                );
            } else {
                eprintln!(
                    "[mcp-serve] transport {} on {}: auth=none (localhost-only bind is \
                     the accepted alternative; set --auth-token to require bearer)",
                    transport, bind
                );
            }
        }
        McpAuth::None => {}
    }

    let app = mcp_router(server, auth);

    // Bind on the blocking side (fail loudly BEFORE the runtime starts),
    // then hand the fd to tokio.
    let std_listener = std::net::TcpListener::bind(bind)
        .map_err(|e| format!("mcp-serve: cannot bind {}: {}", bind, e))?;
    std_listener
        .set_nonblocking(true)
        .map_err(|e| format!("mcp-serve: cannot set nonblocking: {}", e))?;
    eprintln!(
        "[mcp-serve] {} transport listening on http://{} (POST /mcp, GET /sse)",
        transport, bind
    );
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("mcp-serve: tokio runtime: {}", e))?;
    rt.block_on(async move {
        let listener = tokio::net::TcpListener::from_std(std_listener)
            .map_err(|e| format!("mcp-serve: tokio listener: {}", e))?;
        axum::serve(listener, app)
            .await
            .map_err(|e| format!("mcp-serve: server error: {}", e))
    })?;
    Ok(())
}

/// The MCP network router (Naryad #394): POST /mcp — JSON-RPC request/
/// response (the HTTP transport and the SSE client→server direction);
/// GET /sse — the per-session server→client event stream. The SAME
/// `McpServer` core handles requests on every route — the security
/// surface cannot drift between transports by construction.
#[cfg(feature = "server")]
fn mcp_router(server: std::sync::Arc<McpServer>, auth: &McpAuth) -> axum::Router {
    use axum::response::IntoResponse;
    use axum::routing::{get, post};
    use axum::Router;

    // Session outboxes for the SSE transport.
    let sessions: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, SseOutbox>>> =
        std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));

    let auth_for_post = auth.clone();
    let auth_for_sse = auth.clone();

    // POST /mcp[?session=<id>] — the JSON-RPC endpoint (HTTP transport
    // and the SSE transport's client→server direction).
    let state = server.clone();
    let sessions_post = sessions.clone();
    let post_handler = move |headers: axum::http::HeaderMap, uri: axum::http::Uri, body: String| {
        let state = state.clone();
        let sessions = sessions_post.clone();
        let auth = auth_for_post.clone();
        async move {
            if let Err(resp) = check_auth(&headers, &auth) {
                return resp;
            }
            let req: serde_json::Value = match serde_json::from_str(&body) {
                Ok(v) => v,
                Err(e) => {
                    return axum::Json(serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": {"code": -32700, "message": format!("Parse error: {}", e)}
                    }))
                    .into_response()
                }
            };
            let id = req.get("id").cloned().unwrap_or(serde_json::Value::Null);
            let method = req
                .get("method")
                .and_then(|m| m.as_str())
                .unwrap_or("")
                .to_string();
            let params = req
                .get("params")
                .cloned()
                .unwrap_or(serde_json::Value::Null);

            // Heavy tool execution runs OFF the async runtime.
            let state2 = state.clone();
            let response =
                tokio::task::spawn_blocking(move || state2.handle_request(&method, &params, &id))
                    .await
                    .unwrap_or_else(|e| {
                        serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": null,
                            "error": {"code": -32603, "message": format!("internal error: {}", e)}
                        })
                    });

            // SSE mode: the response is delivered on the session stream
            // (POST acknowledges with 202 — the MCP HTTP+SSE shape).
            let session = uri
                .query()
                .and_then(|q| {
                    q.split('&').find_map(|kv| {
                        let (k, v) = kv.split_once('=')?;
                        (k == "session").then(|| v.to_string())
                    })
                })
                .unwrap_or_default();
            if !session.is_empty() {
                let outbox = sessions
                    .lock()
                    .map(|m| m.get(&session).cloned())
                    .unwrap_or(None);
                if let Some(tx) = outbox {
                    let _ = tx.send(response.clone());
                    return axum::http::StatusCode::ACCEPTED.into_response();
                }
                return axum::http::StatusCode::NOT_FOUND.into_response();
            }
            axum::Json(response).into_response()
        }
    };

    // GET /sse — the server→client event stream (MCP HTTP+SSE).
    let sessions_sse = sessions.clone();
    let sse_handler = move |headers: axum::http::HeaderMap| {
        let sessions = sessions_sse.clone();
        let auth = auth_for_sse.clone();
        async move {
            if let Err(resp) = check_auth(&headers, &auth) {
                return resp;
            }
            let session = uuid::Uuid::new_v4().to_string();
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<serde_json::Value>();
            if let Ok(mut map) = sessions.lock() {
                map.insert(session.clone(), tx);
            }
            let endpoint = format!("/mcp?session={}", session);
            let stream = SseOutboxStream {
                first: Some(
                    axum::response::sse::Event::default()
                        .event("endpoint")
                        .data(endpoint),
                ),
                rx,
            };
            axum::response::sse::Sse::new(stream)
                .keep_alive(axum::response::sse::KeepAlive::default())
                .into_response()
        }
    };

    Router::new()
        .route("/mcp", post(post_handler))
        .route("/sse", get(sse_handler))
}

/// Test-server variant (the `run_test_server` pattern, Naryad #394):
/// binds 127.0.0.1:0 (OS-assigned port), spawns the router and returns
/// (port, JoinHandle) — the integration tests walk the REAL HTTP and
/// SSE surfaces through this.
#[cfg(feature = "server")]
pub async fn run_test_mcp_server(
    declarations: &[Declaration],
    allowlist: &[String],
    auth: &McpAuth,
) -> Result<
    (
        u16,
        tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    ),
    String,
> {
    let server = std::sync::Arc::new(McpServer::new(declarations, allowlist)?);
    let app = mcp_router(server, auth);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("test mcp-serve bind: {}", e))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("test mcp-serve addr: {}", e))?
        .port();

    let handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        Ok(())
    });

    Ok((port, handle))
}

/// Bearer check for the network transports. With a token configured,
/// EVERY request (POST and SSE GET) must carry
/// `Authorization: Bearer <token>` — a mismatch is 401, loud.
#[cfg(feature = "server")]
#[allow(clippy::result_large_err)] // Response-as-Err carries the loud 401 body (server.rs precedent)
fn check_auth(
    headers: &axum::http::HeaderMap,
    auth: &McpAuth,
) -> Result<(), axum::response::Response> {
    use axum::response::IntoResponse;
    // `result_large_err` is loud here: the Response carries the reason
    // string the caller (browser/agent) needs; boxed errors would hide it.
    if let McpAuth::Bearer(expected) = auth {
        let got = headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if got != format!("Bearer {}", expected) {
            let resp = (
                axum::http::StatusCode::UNAUTHORIZED,
                "mcp-serve: missing or invalid bearer token (Authorization: Bearer <token>)",
            )
                .into_response();
            return Err(resp);
        }
    }
    Ok(())
}

/// The SSE event stream: the mandatory `endpoint` event first (the
/// client POSTs JSON-RPC there), then `message` events carrying the
/// JSON-RPC responses pushed into the session outbox. A hand-rolled
/// `Stream` over the tokio channel — no extra dependency beyond the
/// `futures-core` trait (already in the tree via axum, declared direct
/// under the `server` feature for the ADR-0168 honest accounting).
#[cfg(feature = "server")]
struct SseOutboxStream {
    first: Option<axum::response::sse::Event>,
    rx: tokio::sync::mpsc::UnboundedReceiver<serde_json::Value>,
}

#[cfg(feature = "server")]
impl futures_util::Stream for SseOutboxStream {
    type Item = Result<axum::response::sse::Event, std::convert::Infallible>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll;
        if let Some(ev) = self.first.take() {
            return Poll::Ready(Some(Ok(ev)));
        }
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(msg)) => Poll::Ready(Some(Ok(axum::response::sse::Event::default()
                .event("message")
                .data(serde_json::to_string(&msg).unwrap_or_default())))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

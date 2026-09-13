// ── MCP Server: Metalogos as MCP tool provider (Наряд №297, issue #361) ──
//
// Stdio-based JSON-RPC 2.0 server (newline-framed) that exposes user
// `tool` constructs from a .mlog file as MCP tools. This is the reverse
// of the MCP client (Наряд №268, ADR-0132) — Metalogos IS the tool server.
//
// Protocol: MCP over stdio (JSON-RPC 2.0, one message per line).
//   initialize → capabilities (tools)
//   tools/list → array of tool schemas
//   tools/call → execute tool method body, return result
//
// Fail-closed: without `--allowlist`, server refuses to start.

use crate::ast::{Declaration, Param};
use crate::interpreter::Interpreter;
use crate::interpreter::Value;
use std::io::{self, BufRead, Write};

/// Maximum response size (bytes) before truncation.
const MAX_RESPONSE_SIZE: usize = 1_000_000;

/// Run the MCP server loop: read JSON-RPC from stdin, write to stdout.
/// The `declarations` come from parsing the .mlog file; the `allowlist`
/// filters which tool methods are exposed.
pub fn run_mcp_server(declarations: &[Declaration], allowlist: &[String]) -> Result<(), String> {
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

        let response = handle_request(method, &params, &id, &tools, declarations);
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
                    });
                }
            }
        }
    }
    exposed
}

fn handle_request(
    method: &str,
    params: &serde_json::Value,
    id: &serde_json::Value,
    tools: &[ExposedTool],
    declarations: &[Declaration],
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
            let tool_list: Vec<serde_json::Value> = tools
                .iter()
                .map(|t| {
                    let properties = t.params.iter().map(|p| {
                        (p.name.clone(), serde_json::json!({
                            "type": json_type_for(&p.type_name),
                            "description": format!("Parameter {} of type {}", p.name, p.type_name)
                        }))
                    }).collect::<serde_json::Map<_, _>>();
                    serde_json::json!({
                        "name": t.full_name,
                        "description": t.description,
                        "inputSchema": {
                            "type": "object",
                            "properties": properties,
                            "required": t.params.iter().map(|p| p.name.clone()).collect::<Vec<_>>()
                        }
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
            let tool = tools
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
            match execute_tool_method(declarations, &tool.tool_name, &tool.method_name, &arguments)
            {
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

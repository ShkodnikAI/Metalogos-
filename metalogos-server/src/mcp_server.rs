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
// EVERY transport. The same allowlist/exec/env gates apply to tool
// execution regardless of transport: the request core
// (`McpServer::handle_request`) is transport-agnostic and single-sourced.
//
// Enforcement (the №401 live-verification findings, gh#536):
//   - Finding 1 (env): tool method bodies execute under the SAME
//     `ServeRouteExecGuard` thread-local as serve route bodies (№253-А
//     mechanic) — `env()` is №259-gated (ENV_NOT_PERMITTED unless
//     METALOGOS_SERVE_ALLOW_ENV=1 or the name is in
//     METALOGOS_ENV_ALLOWLIST) and `exec()` is №253-gated
//     (METALOGOS_SERVE_ALLOW_EXEC=1). Previously tool methods ran in the
//     Process context, so `env(key)` leaked process secrets to the
//     MCP caller.
//   - Finding 2 (unchecked files): mcp-serve used to only PARSE the
//     file, so a tool file that was never `mlog check`-ed ran tool
//     bodies unchecked on the interpreter path. Every serve entrypoint
//     (stdio, http, sse — and the test harness) now enforces the
//     Category A startup gate (the №98 `run_server` precedent):
//     IRREVERSIBLE_NO_GRANT, SQL_DYNAMIC, SECRET_LEAK, … refuse the
//     process loudly.
//   - №795 (the C5 (b) owner decision, ADR-0197): the tool methods
//     execute on the VM — the SAME compile-path as serve, the strict
//     compile at startup (a non-compilable file refuses the gateway;
//     the pre-№795 TW path silently ignored the non-tool decls' run
//     errors). The runtime taint twin (VM SinkCheck, the №780/ADR-0156
//     machinery) now fires IN TOOL BODIES through the same VM — the
//     "the startup gate is the only backstop" boundary is closed: a
//     private value routed into a sink at RUNTIME is refused loudly
//     ([SINK_CLEARANCE_RUNTIME]), not just the statically-provable
//     forms. TW stays the reference oracle in CI (ADR-0197).

use metalogos::ast::{Declaration, Param};
use metalogos::interpreter::Value;
// №795 (the C5 (b) owner decision, ADR-0197): the tool methods execute on
// the VM — the SAME compile-path as serve. The TW interpreter is gone from
// the mcp-serve lane (it has no runtime label-gate twin — the header
// contract); TW stays the reference oracle in CI.
// №793 (the gateway G1): the upstream session pool — the session client
// lives in the metalogos crate, the gateway holds ONE process per declared
// upstream for its lifetime.
use metalogos::mcp_session::{mcp_allowlist_gate, parse_upstreams_spec, McpSession};
use std::io::{self, BufRead, Write};
use std::sync::Mutex;

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
    /// №795 (the C5 (b) decision, ADR-0197): the VM tool state — the
    /// declarations (the param lists), the COMPILED program (the same
    /// compile-path as serve) and the qualified pattern index. The tool
    /// methods execute on the VM (the runtime label gates fire); the TW
    /// interpreter is gone from this lane.
    vm_state: std::sync::Arc<VmToolState>,
    /// №793 (the gateway G1): the declared upstream sessions — ONE live
    /// process per entry for the gateway's lifetime; a dead upstream is
    /// reconnected LOUDLY on the next use. Mutex: handle_request takes
    /// &self (the transports share the core through Arc).
    upstreams: Mutex<Vec<McpSession>>,
    /// №801 (the gateway G2): the upstream tool classes from the
    /// `mcp_policy` declaration — the full tool name (`upstream.tool`) →
    /// Source(label) | Sink(required label). The map is the ONLY thing
    /// the floating session label reads; unpoliced upstream tools pass
    /// through unclassified (the №793 posture — the pilot is opt-in).
    policy: std::collections::HashMap<String, McpToolClass>,
    /// №801: the floating (current) session label — ONE label per agent
    /// session (the stdio transport IS one session; the network
    /// transports share the core through Arc and join into the same
    /// label — the per-connection split rides with the http session
    /// work; the pilot documents this honestly). Source successes JOIN
    /// their label in; Sink calls check flows_to against it.
    session_label: Mutex<metalogos::labels::Label>,
    /// №801: the one-time loud notes for unpoliced upstream tools (the
    /// stderr noise fires ONCE per tool, not per call).
    unpoliced_warned: Mutex<std::collections::HashSet<String>>,
    /// №828 (the gateway G5): the session schema-digest pins — upstream
    /// name → (upstream-local tool name → SHA-256 hex over the pinned
    /// representation). Recorded at EVERY client-facing `tools/list`
    /// (the explicit re-pin the naryad names) and, for a first use
    /// without a listing, at the call-time re-list itself — LOUDLY (the
    /// `mcp.pin` ledger record + stderr), never silently. Every upstream
    /// `tools/call` verifies the CURRENT representation against the pin:
    /// a divergence is a fail-closed refusal + a `mcp.call.denied` ledger
    /// record (the rug-pull / tool-poisoning class — the mcp-hangar
    /// digest-pinning reference). The pin only NARROWS: the allowlist
    /// stays the only publishing control (the №297 posture).
    digest_pins:
        Mutex<std::collections::HashMap<String, std::collections::HashMap<String, String>>>,
}

/// №801: the compiled class of one upstream tool.
#[derive(Debug, Clone)]
pub enum McpToolClass {
    /// Ingress: a successful call JOINs this label into the session.
    Source(metalogos::labels::Label),
    /// Egress: the session label must FLOW TO this clearance.
    Sink(metalogos::labels::Label),
}

/// The compiled tool surface (№795): built ONCE at `McpServer::new` —
/// the strict form: a non-compilable file REFUSES the gateway loudly
/// (the pre-№795 TW path silently ignored the non-tool decls' run
/// errors — the quiet start is gone).
pub struct VmToolState {
    pub(crate) declarations: Vec<Declaration>,
    pub(crate) program: metalogos::bytecode::Program,
    pub(crate) pattern_index: std::collections::HashMap<String, usize>,
}

impl McpServer {
    /// Build the server: fail-closed on an empty allowlist (№297
    /// posture, unchanged — on every transport).
    pub fn new(declarations: &[Declaration], allowlist: &[String]) -> Result<McpServer, String> {
        // №477 (gh#725): the tool-registration phase is the explicit top-level
        // zone (the №457 serve precedent): under the inverted process default
        // (set_process_mode(Serve) at the mcp-serve entrypoint) the
        // registration thread keeps the Process context, so a future
        // registration step that touches env/exec fails LOUD only when it is
        // genuinely unmarked outside this zone — a forgotten mark costs a
        // spurious denial, never silent process rights.
        let _toplevel_registration = metalogos::builtins::io::TopLevelRegistrationGuard::new();

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

        // ── №793 (the gateway G1): the upstream session pool ──
        // METALOGOS_MCP_UPSTREAMS is read ONCE here (the №263 discipline);
        // every declaration's argv[0] passes the SAME allowlist gate as the
        // stateless builtins (no weakening) — a declaration outside the
        // allowlist REFUSES the gateway fail-closed (a config error, not a
        // transient state). A spawn failure at startup is LOUD (stderr) and
        // leaves the session degraded until the first use retries.
        let upstreams = match std::env::var("METALOGOS_MCP_UPSTREAMS") {
            Err(_) => Vec::new(),
            Ok(raw) => {
                let mut sessions = parse_upstreams_spec(&raw)?;
                for s in &sessions {
                    // The allowlist gate rides the declaration (fail-closed).
                    mcp_allowlist_gate(s.command())?;
                }
                // The collision rule: the gateway's OWN tools win a name
                // collision with an upstream prefix — detected loudly here.
                for s in &sessions {
                    if tools
                        .iter()
                        .any(|t| t.full_name == s.name() || t.tool_name == s.name())
                    {
                        eprintln!(
                            "[mcp-serve] WARNING: upstream name `{}` collides with a local tool namespace — the LOCAL tool wins every ambiguous call (№793)",
                            s.name()
                        );
                    }
                }
                // Eager spawn: a dead upstream at startup is a loud note,
                // not a constructor failure (the reconnect rides ensure_alive).
                for s in &mut sessions {
                    if let Err(e) = s.ensure_alive() {
                        eprintln!(
                            "[mcp-serve] upstream `{}` failed to start (degraded until the first retry): {}",
                            s.name(),
                            e
                        );
                    }
                }
                eprintln!(
                    "[mcp-serve] upstream sessions declared: {}",
                    sessions
                        .iter()
                        .map(|s| s.name().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                sessions
            }
        };
        // №401 findings (gh#536): the tool-method gate state is loud at
        // startup, mirroring the №253/№259 serve posture — the operator
        // must see WHEN tool methods may read env/exec.
        let env_all = std::env::var("METALOGOS_SERVE_ALLOW_ENV").unwrap_or_default() == "1";
        let env_list = std::env::var("METALOGOS_ENV_ALLOWLIST").unwrap_or_default();
        let env_state = if env_all {
            "allow-all (METALOGOS_SERVE_ALLOW_ENV=1)".to_string()
        } else if env_list.trim().is_empty() {
            "deny (set METALOGOS_SERVE_ALLOW_ENV=1 or METALOGOS_ENV_ALLOWLIST to allow)".to_string()
        } else {
            format!("allowlist: {}", env_list)
        };
        eprintln!(
            "[mcp-serve] tool env: {}; tool exec: {}",
            env_state,
            if std::env::var("METALOGOS_SERVE_ALLOW_EXEC").unwrap_or_default() == "1" {
                "ENABLED (METALOGOS_SERVE_ALLOW_EXEC=1)"
            } else {
                "denied"
            }
        );

        // ── №795: the VM compile at startup — the SAME compile-path as serve ──
        // Tool methods compile as qualified patterns (№684); the program
        // executes tool bodies on the VM per call (the runtime label gates
        // fire — the capability the TW lane never had). The db bootstrap
        // mirrors serve (№1169/№1170): resolved ONCE, eagerly.
        let mut compiler = metalogos::compiler::Compiler::new();
        let mut program = compiler
            .compile(declarations.to_vec())
            .map_err(|e| format!("mcp-serve: VM compile error: {}", e))?;
        metalogos::vm::boot_resolve_db_url(&mut program)
            .map_err(|e| format!("mcp-serve: VM db bootstrap error: {}", e))?;
        let mut pattern_index = std::collections::HashMap::new();
        for t in &tools {
            if let Some(idx) = compiler.pattern_index(&t.full_name) {
                pattern_index.insert(t.full_name.clone(), idx);
            }
        }
        let vm_state = std::sync::Arc::new(VmToolState {
            declarations: declarations.to_vec(),
            program,
            pattern_index,
        });

        // ── №801 (the gateway G2): the policy map from the parsed
        // declarations — the floating session label's ONLY input. Each
        // entry must parse as a Label annotation (the semantic gate
        // validates too; the gateway re-parses so a hot load path never
        // trusts a stale validation) and name an upstream-qualified tool.
        let mut policy: std::collections::HashMap<String, McpToolClass> =
            std::collections::HashMap::new();
        for decl in declarations {
            if let Declaration::McpPolicy(p) = decl {
                for entry in &p.entries {
                    let label = match metalogos::labels::Label::parse(&entry.label) {
                        Ok(l) => l,
                        Err(e) => {
                            return Err(format!(
                                "mcp-serve: mcp_policy entry '{}': invalid label annotation '{}' ({}). The label is a Label annotation: the conf word required (public|consented|private|poisoned), the integrity optional (trusted|untrusted), e.g. \"public, untrusted\"",
                                entry.tool, entry.label, e
                            ));
                        }
                    };
                    if !entry.tool.contains('.') {
                        return Err(format!(
                            "mcp-serve: mcp_policy entry '{}' must be the UPSTREAM form `upstream.tool` (a bare name is the local-tool namespace; its policy compiles from the body, №394)",
                            entry.tool
                        ));
                    }
                    let class = match entry.kind {
                        metalogos::ast::McpPolicyKind::Source => McpToolClass::Source(label),
                        metalogos::ast::McpPolicyKind::Sink => McpToolClass::Sink(label),
                    };
                    policy.insert(entry.tool.clone(), class);
                }
            }
        }
        if !policy.is_empty() {
            eprintln!(
                "cp-serve] mcp_policy: {} upstream tool class(es) loaded (the floating session label, №801)",
                policy.len()
            );
        }

        Ok(McpServer {
            tools,
            vm_state,
            upstreams: Mutex::new(upstreams),
            policy,
            session_label: Mutex::new(metalogos::labels::Label::bottom()),
            unpoliced_warned: Mutex::new(std::collections::HashSet::new()),
            digest_pins: Mutex::new(std::collections::HashMap::new()),
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
            "initialize" => {
                // №793: the version negotiation — the server answers the
                // CLIENT's requested revision when it supports it (the spec
                // matrix), its latest known otherwise; an unsupported pair
                // surfaces as the client's loud refusal naming both versions.
                const SERVER_KNOWN_VERSIONS: [&str; 3] = ["2024-11-05", "2025-03-26", "2025-11-25"];
                let requested = params
                    .get("protocolVersion")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let answer = if SERVER_KNOWN_VERSIONS.contains(&requested) {
                    requested
                } else {
                    "2025-11-25"
                };
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": answer,
                        "serverInfo": {
                            "name": "metalogos-mcp-server",
                            "version": env!("CARGO_PKG_VERSION")
                        },
                        "capabilities": {
                            "tools": {}
                        }
                    }
                })
            }

            "notifications/initialized" => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {}
            }),

            "tools/list" => {
                let mut tool_list: Vec<serde_json::Value> = self
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
                // ── №793 (the gateway G1): the upstream tools aggregation ──
                // Each declared upstream's tools/list rides the LONG-LIVED
                // session; the tool names carry the `name.` prefix. A down
                // upstream drops its tools from THIS list with a loud stderr
                // note (the call attempt to it refuses loudly with the reason
                // — never a silent omission without a trace).
                if let Ok(mut pool) = self.upstreams.lock() {
                    for s in pool.iter_mut() {
                        match s.tools_list_json() {
                            Ok(tools_json) => {
                                if let Some(arr) = tools_json.as_array() {
                                    // ── №828 (the gateway G5): the re-pin ──
                                    // The client-facing listing re-pins the
                                    // session: the digest of every tool the
                                    // client JUST SAW is recorded, one
                                    // `mcp.pin` ledger record per tool (the
                                    // pin is visible in the tape — the naryad
                                    // observability clause). The map REPLACES
                                    // the upstream's previous pins: a tool
                                    // that vanished from the listing loses its
                                    // pin (a call to it refuses fail-closed on
                                    // the call-time re-list).
                                    let mut pins = std::collections::HashMap::new();
                                    for tool in arr.iter() {
                                        if let Some(name) =
                                            tool.get("name").and_then(|n| n.as_str())
                                        {
                                            let digest = tool_schema_digest(tool);
                                            pins.insert(name.to_string(), digest.clone());
                                            metalogos::ledger::record(
                                                "mcp.pin",
                                                "mcp-gateway",
                                                &format!("{}.{}", s.name(), name),
                                                &format!(
                                                    "pinned schema digest={} (client listing re-pin, G5)",
                                                    &digest[..16]
                                                ),
                                            );
                                        }
                                    }
                                    if let Ok(mut store) = self.digest_pins.lock() {
                                        store.insert(s.name().to_string(), pins);
                                    }
                                    for tool in arr {
                                        let mut entry = tool.clone();
                                        if let Some(name) = entry
                                            .get("name")
                                            .and_then(|n| n.as_str())
                                            .map(str::to_string)
                                        {
                                            entry["name"] = serde_json::Value::String(format!(
                                                "{}.{}",
                                                s.name(),
                                                name
                                            ));
                                        }
                                        tool_list.push(entry);
                                    }
                                } else {
                                    eprintln!(
                                        "[mcp-upstream] `{}` tools/list: result.tools is not an array — skipped loudly",
                                        s.name()
                                    );
                                }
                            }
                            Err(e) => {
                                eprintln!(
                                    "[mcp-upstream] `{}` tools/list failed (its tools are omitted from this listing): {}",
                                    s.name(),
                                    e
                                );
                            }
                        }
                    }
                }
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

                // ── №793 (the gateway G1): the upstream proxy ──
                // `name.tool` routes to the declared upstream's LONG-LIVED
                // session (one process per upstream, the loud reconnect on
                // death). The LOCAL tools win a collision (full_name or the
                // bare method_name match is checked FIRST — the rule pinned
                // loudly at startup). The response shape, the size cap and
                // the error envelope are byte-identical to the local path.
                let local_match = self
                    .tools
                    .iter()
                    .any(|t| t.full_name == tool_name || t.method_name == tool_name);
                if !local_match {
                    if let Some((upstream_name, upstream_tool)) = tool_name.split_once('.') {
                        // ── №801 (the gateway G2): the floating session
                        // label. The policy class decides BEFORE the call:
                        // a Sink checks `flows_to` (with the owner-grant
                        // fallback); a Source joins its label on success.
                        // An UNPOLICED upstream tool passes through
                        // unclassified (the №793 posture — the pilot is
                        // opt-in) with a ONE-TIME loud note.
                        let class = self.policy.get(tool_name);
                        if class.is_none() {
                            let mut warned = self
                                .unpoliced_warned
                                .lock()
                                .unwrap_or_else(|p| p.into_inner());
                            if warned.insert(tool_name.to_string()) {
                                eprintln!(
                                    "cp-serve] G2: upstream tool '{}' has NO mcp_policy class — it passes UNCLASSIFIED (the №793 posture); declare a Source/Sink entry for the session-label gate",
                                    tool_name
                                );
                            }
                        }
                        if let Some(McpToolClass::Sink(required)) = class {
                            let current = self
                                .session_label
                                .lock()
                                .unwrap_or_else(|p| p.into_inner())
                                .clone();
                            if !current.flows_to(required) {
                                // The downgrade path: an OWNER GRANT only —
                                // the scope convention is `mcp:<tool>`, the
                                // class "once" consumes on the first pass
                                // (ADR-0155). The grant's use is recorded in
                                // the grant ledger by grant_use itself; the
                                // gateway ledger records the DECISION.
                                let action_scope = format!("mcp:{}", tool_name);
                                let granted =
                                    metalogos::grants::lookup_active_covering(&action_scope)
                                        .and_then(|h| {
                                            metalogos::grants::grant_use(
                                                &h,
                                                &format!(
                                                    "mcp-gateway sink clearance for {}",
                                                    tool_name
                                                ),
                                            )
                                            .ok()
                                            .map(|_| h)
                                        });
                                if granted.is_none() {
                                    ledger_mcp_record(
                                        false,
                                        tool_name,
                                        "denied: SINK_CLEARANCE (the floating session label does not flow to the sink clearance; no covering owner grant)",
                                    );
                                    return serde_json::json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                            "content": [{"type": "text", "text": format!(
                                                "[SINK_CLEARANCE] upstream tool '{}' requires clearance '{}'; the session label is '{}' — the call is refused. The downgrade path: an owner grant (grant_issue scope \"mcp:{}\", class \"once\") or the redact policy.",
                                                tool_name, required, current, tool_name
                                            )}],
                                            "isError": true
                                        }
                                    });
                                }
                                ledger_mcp_record(
                                    true,
                                    tool_name,
                                    "granted: SINK_CLEARANCE bypassed by the owner once-grant (consumed)",
                                );
                            }
                        }
                        let mut proxy = None;
                        if let Ok(mut pool) = self.upstreams.lock() {
                            if let Some(s) = pool.iter_mut().find(|s| s.name() == upstream_name) {
                                // ── №828 (the gateway G5): the schema digest
                                // pin verification — BEFORE the forward. The
                                // CURRENT representation is re-listed from the
                                // live upstream (ensure_alive reconnects
                                // loudly on a dead one — the G1 contract rides
                                // through) and compared against the pin. A
                                // divergence (or an unverified schema) is a
                                // fail-closed refusal + a G3 ledger record —
                                // «молча пропустить» запрещён.
                                if let Err(reason) =
                                    self.verify_upstream_schema_pin(s, upstream_tool)
                                {
                                    ledger_mcp_record(false, tool_name, &reason);
                                    return serde_json::json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                            "content": [{"type": "text", "text": format!(
                                                "[SCHEMA_DIGEST_MISMATCH] upstream tool '{}' is refused: {}. The pinned schema is the machine link between what the listing showed and what executes; re-list the tools to re-pin the session (the legitimate path back).",
                                                tool_name, reason
                                            )}],
                                            "isError": true
                                        }
                                    });
                                }
                                proxy = Some(
                                    match s.tools_call_text(upstream_tool, arguments.clone()) {
                                        Ok(text) => Ok(text),
                                        Err(e) => Err(e),
                                    },
                                );
                            }
                        }
                        if let Some(result) = proxy {
                            return match result {
                                Ok(text) => {
                                    // №801: a Source SUCCESS joins its label
                                    // into the session (the LIO current-label
                                    // discipline — the conservative join: the
                                    // session only rises, never falls).
                                    if let Some(McpToolClass::Source(label)) = class {
                                        let mut sl = self
                                            .session_label
                                            .lock()
                                            .unwrap_or_else(|p| p.into_inner());
                                        *sl = sl.join(label);
                                    }
                                    // №794 (the gateway G3): the ALLOWED call — the ledger
                                    // carries the decision, the tool and the outcome
                                    // (the arguments ride only as the args_hash — the ledger
                                    // must not become a leak channel).
                                    ledger_mcp_record(true, tool_name, "ok");
                                    let result_str = if text.len() > MAX_RESPONSE_SIZE {
                                        format!(
                                            "{}... (truncated, {} bytes)",
                                            &text[..MAX_RESPONSE_SIZE],
                                            text.len()
                                        )
                                    } else {
                                        text
                                    };
                                    // №801: the session label rides every
                                    // result _meta (the observability the
                                    // naryad names).
                                    let session_label = self
                                        .session_label
                                        .lock()
                                        .unwrap_or_else(|p| p.into_inner())
                                        .to_string();
                                    serde_json::json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                            "content": [{"type": "text", "text": result_str}],
                                            "isError": false,
                                            "_meta": {"metalogos.dev/session-label": session_label}
                                        }
                                    })
                                }
                                Err(e) => {
                                    // Allowed (the proxy routed the call) but the
                                    // upstream/tool failed — the OUTCOME, not a denial.
                                    ledger_mcp_record(true, tool_name, &format!("error: {}", e));
                                    serde_json::json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                            "content": [{"type": "text", "text": format!("Error: {}", e)}],
                                            "isError": true
                                        }
                                    })
                                }
                            };
                        }
                    }
                }

                // Find the tool method by name.
                let tool = self
                    .tools
                    .iter()
                    .find(|t| t.full_name == tool_name || t.method_name == tool_name);
                if tool.is_none() {
                    // №794 (the gateway G3): the DENIED call — the ledger records
                    // every refusal with the reason (the args hash-only).
                    ledger_mcp_record(false, tool_name, "denied: unknown tool (not in allowlist)");
                    return serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {"code": -32602, "message": format!("Unknown tool: {} (not in allowlist)", tool_name)}
                    });
                }
                let Some(tool) = tool else {
                    ledger_mcp_record(false, tool_name, "denied: not in allowlist");
                    return serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {"code": -32602, "message": format!("Tool not in allowlist: {}", tool_name)}
                    });
                };

                // Execute the tool method body (№795: on the VM).
                match execute_tool_method(
                    &self.vm_state,
                    &tool.tool_name,
                    &tool.method_name,
                    &arguments,
                ) {
                    Ok(result) => {
                        // №794: the allowed call — ok.
                        ledger_mcp_record(true, tool_name, "ok");
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
                    Err(e) => {
                        // №794: the allowed call whose BODY failed — the outcome,
                        // not a denial (the tool executed; the body errored).
                        ledger_mcp_record(true, tool_name, &format!("error: {}", e));
                        serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "content": [{"type": "text", "text": format!("Error: {}", e)}],
                                "isError": true
                            }
                        })
                    }
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

/// The Category A startup gate for every mcp-serve entrypoint (the
/// №401 live-verification findings, gh#536; the №98 `run_server`
/// precedent). mcp-serve used to only PARSE the file, so a tool file
/// that was never `mlog check`-ed ran tool bodies unchecked — a literal
/// `DROP TABLE` reached the SQL layer (`IRREVERSIBLE_NO_GRANT` was a
/// front-door-only defense) and dynamic SQL reached it too. Now the
/// same `audit_category_a` pass the HTTP server enforces refuses the
/// process loudly, with the exact `[CODE] message` lines `mlog check`
/// prints — the codes, not the prose, are the contract (ADR-0131).
pub fn enforce_category_a_startup(declarations: &[Declaration]) -> Result<(), String> {
    let cat_a = metalogos::audit::audit_category_a(declarations, "");
    let errors: Vec<String> = cat_a
        .iter()
        .filter_map(|f| match f.severity {
            metalogos::audit::Severity::Error | metalogos::audit::Severity::Warning => {
                Some(format!("[{}] {}", f.check_id, f.message))
            }
            metalogos::audit::Severity::Info => None,
        })
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "mcp-serve: Category A security invariant violated:\n{}",
            errors.join("\n")
        ))
    }
}

/// №557 (issue #918; the audit 02.10 M-7): the SEMANTIC startup gate for
/// every mcp-serve entrypoint — the same level of protection `mlog serve`
/// got in №523. mcp-serve ran NO semantic pass at all: a tool file never
/// `mlog check`-ed could carry blocking semantic findings and serve them
/// (an undefined function inside a tool body evaluates to the truthy
/// liar string at call time on the interpreter path — the exact exploit
/// shape №523 closed for serve). Every `semantic::check_program` error
/// blocks the STARTUP; exemptions classify ONLY by the structured kind in
/// `semantic::is_exempt_from_blocking` (the ONE explicit place), never by
/// substring. Imports resolve statically with the same "." base_dir rule
/// the №523 serve gate uses (identical to the runtime loader's lookup),
/// and the refusal carries the FIRST blocking finding's stable code at
/// position 0 (the №479 posture), in the serve refusal's format.
pub fn enforce_semantic_startup(declarations: &[Declaration]) -> Result<(), String> {
    let base_dir = std::path::PathBuf::from(".");
    let module_decls = metalogos::semantic::resolve_imports_statically(declarations, &base_dir)
        .map_err(|e| format!("Compilation error (Naryad #557): {}", e))?;
    let mut merged_decls = module_decls;
    merged_decls.extend(declarations.to_vec());
    let sem_result = metalogos::semantic::check_program(&merged_decls);
    // №802 note: the mcp-serve startup keeps the UNFILTERED refusal
    // surface (no source threading through the MCP entrypoints in this
    // naryad) — a suppressed-at-check program may refuse here; the
    // asymmetry is fail-closed (stricter, never looser) and is recorded
    // in the naryad report.
    let blocking: Vec<&metalogos::semantic::SpannedError> = sem_result
        .errors
        .iter()
        .filter(|err| !metalogos::semantic::is_exempt_from_blocking(err.kind))
        .collect();
    if blocking.is_empty() {
        return Ok(());
    }
    // №479: the refusal carries the FIRST blocking finding's stable code
    // at position 0 (mirrors the run gate in lib.rs and the №523 serve
    // gate in server.rs).
    let code = blocking.iter().find_map(|err| err.kind.stable_code());
    let stamp = code.map(|c| format!("[{}] ", c)).unwrap_or_default();
    let lines: Vec<String> = blocking
        .iter()
        .map(|err| metalogos::semantic::format_blocking_line(err))
        .collect();
    Err(format!(
        "{}Compilation error (Naryad #557): semantic findings block mcp-serve startup:\n{}",
        stamp,
        lines.join("\n")
    ))
}

/// Run the MCP server loop over stdio: read JSON-RPC from stdin, write
/// to stdout. Behavior identical to №297 (the default transport).
pub fn run_mcp_server(declarations: &[Declaration], allowlist: &[String]) -> Result<(), String> {
    // gh#536 Finding 2: the gate runs BEFORE the allowlist check — a
    // dangerous file is refused for what it IS, not for what it exposes.
    enforce_category_a_startup(declarations)?;
    // №557: the semantic gate — a lying tool never serves (the №523 serve
    // posture on the MCP surface).
    enforce_semantic_startup(declarations)?;
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

/// №794 (the gateway G3): the ledger record on EVERY `tools/call` —
/// allowed (`mcp.call`) or refused (`mcp.call.denied`), the tool in the
/// scope, the reason/outcome in the detail. The detail is hashed into
/// the record's `args_hash` BY THE LEDGER SCHEMA — the arguments and
/// the reasons never ride the journal in the open form (the ledger must
/// not become a leak channel). The actor is the gateway process; the
/// floating session label extends this record with G2 (#1481) — the
/// `label=pre-G2` marker is the explicit extension point, not silence.
fn ledger_mcp_record(allowed: bool, tool: &str, outcome: &str) {
    let action = if allowed {
        "mcp.call"
    } else {
        "mcp.call.denied"
    };
    let detail = format!("tool={} outcome={} label=pre-G2", tool, outcome);
    metalogos::ledger::record(action, "mcp-gateway", tool, &detail);
}

/// №828 (the gateway G5): the digest over the DETERMINISTIC representation
/// of one upstream tool entry — the four fields the naryad pins (name +
/// description + inputSchema + `_meta`), serialized through serde_json's
/// BTreeMap (sorted keys — an upstream reordering its JSON keys cannot
/// rotate the digest). The hash is SHA-256 from the EXISTING dep graph
/// (`metalogos::ledger::sha256_hex` — the same primitive the ledger chain
/// itself uses; no new crate, the ADR-0168 D3 form).
fn tool_schema_digest(entry: &serde_json::Value) -> String {
    let repr = serde_json::json!({
        "name": entry.get("name"),
        "description": entry.get("description"),
        "inputSchema": entry.get("inputSchema"),
        "_meta": entry.get("_meta"),
    });
    let bytes = serde_json::to_vec(&repr).unwrap_or_default();
    metalogos::ledger::sha256_hex(&bytes)
}

impl McpServer {
    /// №828 (the gateway G5): verify the session's schema pin for one
    /// upstream tool against its CURRENT representation (re-listed from the
    /// live upstream — the mcp-hangar discipline: the call-time view, not a
    /// stale cache).
    ///
    /// - pin present + digest equal → forward;
    /// - pin present + digest different → the rug-pull refusal (fail-closed);
    /// - the tool vanished from the current listing → the same mismatch
    ///   class (the schema the client saw is gone);
    /// - the re-list itself failed → fail-closed (an unverified schema is
    ///   not forwarded);
    /// - NO pin (a first use without a client-facing listing) → the
    ///   baseline is established FROM the current representation — LOUDLY:
    ///   the `mcp.pin` ledger record + the stderr note. The pin only
    ///   NARROWS (the call was already allowed by the allowlist/policy);
    ///   every LATER call verifies against it.
    fn verify_upstream_schema_pin(
        &self,
        s: &mut metalogos::mcp_session::McpSession,
        upstream_tool: &str,
    ) -> Result<(), String> {
        let pinned = {
            let store = self.digest_pins.lock().unwrap_or_else(|p| p.into_inner());
            store
                .get(s.name())
                .and_then(|m| m.get(upstream_tool))
                .cloned()
        };

        let listing = match s.tools_list_json() {
            Ok(l) => l,
            Err(e) => {
                return Err(format!(
                    "schema unverified — the upstream re-list failed: {}",
                    e
                ));
            }
        };
        let current_entry = listing
            .as_array()
            .and_then(|arr| {
                arr.iter()
                    .find(|t| t.get("name").and_then(|n| n.as_str()) == Some(upstream_tool))
            })
            .cloned();
        let Some(current_entry) = current_entry else {
            return Err(format!(
                "schema digest mismatch — tool '{}' is absent from the current upstream listing",
                upstream_tool
            ));
        };
        let current_digest = tool_schema_digest(&current_entry);

        match pinned {
            None => {
                eprintln!(
                    "[mcp-upstream] G5: pinned `{}.{}` schema digest {} (first use without a listing — the baseline rides the ledger)",
                    s.name(),
                    upstream_tool,
                    &current_digest[..16]
                );
                metalogos::ledger::record(
                    "mcp.pin",
                    "mcp-gateway",
                    &format!("{}.{}", s.name(), upstream_tool),
                    &format!(
                        "pinned schema digest={} (first-use baseline, G5)",
                        &current_digest[..16]
                    ),
                );
                if let Ok(mut store) = self.digest_pins.lock() {
                    store
                        .entry(s.name().to_string())
                        .or_default()
                        .insert(upstream_tool.to_string(), current_digest);
                }
                Ok(())
            }
            Some(p) if p == current_digest => Ok(()),
            Some(p) => Err(format!(
                "schema digest mismatch (pinned {}, current {})",
                &p[..16],
                &current_digest[..16]
            )),
        }
    }
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
    vm_state: &VmToolState,
    tool_name: &str,
    method_name: &str,
    arguments: &serde_json::Value,
) -> Result<String, String> {
    // gh#536 Finding 1: a tool method body runs under the SAME
    // exec-context guard as serve route bodies (№253-А thread-local
    // mechanic) — the header contract ("the same allowlist/exec/env
    // gates apply to tool execution") is now enforced, not just
    // declared. The guard is alive for the whole call: env() inside the
    // body is №259-gated (ENV_NOT_PERMITTED unless
    // METALOGOS_SERVE_ALLOW_ENV=1 or the name is allowlisted) and
    // exec() is №253-gated (METALOGOS_SERVE_ALLOW_EXEC=1; the
    // process-level METALOGOS_ALLOW_EXEC flag does NOT apply — the
    // MCP caller is untrusted, the exact route-body threat shape).
    let _serve_guard = metalogos::builtins::ServeRouteExecGuard::new();

    // Find the tool declaration (the param list rides the DECLARATION —
    // the execution rides the compiled program).
    let tool_decl = vm_state
        .declarations
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

    let method = tool_decl
        .methods
        .iter()
        .find(|m| m.name == method_name)
        .ok_or_else(|| format!("Method not found: {}.{}", tool_name, method_name))?;

    // №795: the VM lane — the qualified key compiles as a REGULAR PATTERN
    // (№684); the invocation is the CallPattern convention (the args bind
    // as the frame locals). The runtime label gates (ADR-0156) fire in the
    // body — the capability the TW lane never had (the RED pin n795).
    let qualified = format!("{}.{}", tool_name, method_name);
    let idx = *vm_state
        .pattern_index
        .get(&qualified)
        .ok_or_else(|| format!("Method not compiled for VM: {}", qualified))?;

    // Convert JSON arguments to positional Values (the declaration order).
    let mut args = Vec::with_capacity(method.params.len());
    for p in &method.params {
        let json_val = arguments.get(&p.name).unwrap_or(&serde_json::Value::Null);
        args.push(json_to_value(json_val));
    }

    let mut vm = metalogos::vm::Vm::new();
    let result = vm.invoke_pattern(idx, args, &vm_state.program)?;
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
    // gh#536 Finding 2: same startup gate as the stdio transport — the
    // transports cannot drift (the request core is shared; so is the
    // front door).
    enforce_category_a_startup(declarations)?;
    // №557: the semantic gate — the transports cannot drift on it either
    // (the same posture the stdio front door carries).
    enforce_semantic_startup(declarations)?;
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
    // gh#536 Finding 2: the test harness walks the REAL entrypoint
    // surface — the startup gate is part of it (the №394 suite's clean
    // sources pass it; a dirty source is refused before any bind).
    enforce_category_a_startup(declarations)?;
    // №557: the semantic gate — the same posture on the test harness (it
    // walks the REAL entrypoint surface, so it carries BOTH gates).
    enforce_semantic_startup(declarations)?;
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

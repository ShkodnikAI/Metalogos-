# ADR-0168: MCP server transports — stdio + HTTP/SSE, bearer auth, compiled tool-policy

**Status:** Accepted
**Date:** 2026-09-17
**Naryad:** #394 (issue #488; dispatch #491, wave 3)
**Pillar:** cross-cutting (feature/mcp); the SERVER half of the MCP contour — extends ADR-0132 (client, stdio-only) with an explicit **"client → +server transport"** scope marker per its Future Directions
**Consumers:** naryad #395 (dogfood office — MCP surface), IDE/agent/remote integrations

## 1. Context

The MCP server (naryad #297, `src/mcp_server.rs`) is stdio-only: `mlog mcp-serve <file> --allowlist tool1,tool2` speaks newline-framed JSON-RPC 2.0 over stdin/stdout. External ecosystems (IDE extensions, remote agents, dashboards) speak MCP over HTTP — today they cannot reach a Metalogos tool surface at all. The tool policy a consumer sees is implicit (whatever the allowlist lets through); the naryad requires it to be **compiled from the program contour** — what the tool does is a fact about the code, not a YAML file maintained by hand next to it.

Protocol base: ADR-0132 D1 fixes JSON-RPC 2.0 over the legacy handshake (`initialize` → `notifications/initialized` → `tools/list`/`tools/call`) against spec revision 2026-07-28 (modern/legacy split; the legacy client speaks to servers of revisions 2024-11-05…2025-11-25). The server keeps exactly this protocol base — the same request core the stdio server has always had.

## 2. Decision Drivers

1. **One request core, many transports.** The security surface (fail-closed allowlist, exec/env gates, label clearance on tool I/O) must be IDENTICAL on stdio, http and sse — the only way that cannot drift is a single `McpServer::handle_request` every transport dispatches through (`src/mcp_server.rs`, the §3.1 core). Transport code routes bytes; it never re-implements policy.
2. **Fail-closed is not negotiable.** An empty allowlist refuses to start on every transport (the №297 posture). The allowlist remains the ONLY publishing control; the compiled policy annotates, it never widens.
3. **Zero new compiled crates.** axum/tokio/tower are already workspace dependencies behind the `server` feature (default-on). The only new *declared* dependency is `futures-util` — already in the tree via axum, compiled anyway; FEATURE_INTAKE §5 delta is zero new crates (honest accounting: it moves from transitive to direct, scoped to `server`).
4. **Auth first stage: bearer OR localhost-only, never a silent open port.** A bearer token (`--auth-token` or `METALOGOS_MCP_AUTH_TOKEN`) gates every request (401 on mismatch — POST and SSE alike). Without a token, a loopback bind is the accepted alternative and a non-loopback bind is a loud WARN (the №263 bind-WARN posture, `src/server.rs`).
5. **Policy is compiled, not authored.** The per-tool policy block is derived from the method body AST + the №316 SSOT classification (`src/mcp_policy.rs`) — the same map every static gate reads. A hand-maintained YAML would drift from the code on the first edit; a compiled policy cannot.
6. **Honest SSE scope.** The SSE transport implements the MCP HTTP+SSE shape (GET /sse → `endpoint` event → POST /mcp?session=… → 202 → responses as `message` events on the session stream) — the wire shape of the 2025-03-26 revision that the legacy protocol base (ADR-0132 D1) pairs with. Modern-era revisions' full transport matrix (streamable-HTTP-only variants, MRTR) is Future — same boundary statement as the client side.

## 3. Decision

### 3.1 Transports

| Transport | Flag | Wire | Feature | Auth |
|---|---|---|---|---|
| stdio (default) | `--transport stdio` | newline-framed JSON-RPC 2.0 over stdin/stdout — byte-identical behavior to №297 | always | none (the process boundary IS the boundary) |
| HTTP | `--transport http` | JSON-RPC over HTTP `POST /mcp`, request→response 200 | `server` | bearer / localhost-only |
| SSE | `--transport sse` | MCP HTTP+SSE: `GET /sse` (`endpoint` event → POST path) + `POST /mcp?session=…` (202; responses as `message` events on the session stream) | `server` | bearer / localhost-only |

Options: `--bind <addr:port>` (default `127.0.0.1:8770`), `--auth-token <token>` (else `METALOGOS_MCP_AUTH_TOKEN`). Unknown transport → usage error (exit 2). http/sse on a binary built without `server` → loud refusal (exit 2), stdio still available.

### 3.2 Security parity (the matrix the tests pin)

| Gate | stdio | http | sse | Mechanism |
|---|---|---|---|---|
| allowlist fail-closed (empty → refuse) | ✔ | ✔ | ✔ | `McpServer::new` |
| unknown tool → JSON-RPC -32602 | ✔ | ✔ | ✔ | `handle_request` |
| exec gate (`EXEC_NOT_PERMITTED`) | ✔ | ✔ | ✔ | the interpreter executing the method body — transport-blind |
| env gate, label clearance, taint on tool I/O | ✔ | ✔ | ✔ | same execution path |
| bearer 401 | n/a | ✔ | ✔ | `check_auth` on POST **and** SSE GET |
| non-loopback bind without auth | n/a | WARN | WARN | the №263 posture |
| audit events (sink calls inside methods) | ✔ | ✔ | ✔ | the interpreter's audit surface |

### 3.3 Compiled tool-policy

`compile_policy(method)` (`src/mcp_policy.rs`) walks the method body AST, resolves every call through `builtins_classification::classify` (№316) and emits:

- `sink_calls` — every Role::Sink builtin with its class from `audit::sink_kind` (the SSOT the deny gates use): these are the audited effects;
- `clearance_args` — tool params that lexically flow into a sink call's arguments (conservative: any identifier inside a sink argument subtree);
- `irreversible` — whether any Irreversible-classified builtin can fire;
- `source_calls` — ingress points (Role::Source);
- `unclassified` — calls outside the classification map, listed honestly instead of silently dropped.

The block rides in each `tools/list` entry as `_meta["metalogos.dev/policy"]` (version 1, `compiled_from: "spec!-registry + No316 classification (auto; no manual YAML)"`). The allowlist alone decides what is published — the policy makes the published tool's effect profile VISIBLE, it cannot widen it.

### 3.4 Auth model

- `McpAuth::Bearer(token)` — every request must carry `Authorization: Bearer <token>`; mismatch → 401 with a loud body. Applies to POST /mcp and GET /sse identically.
- `McpAuth::OpenLocal` (no token) — acceptable ONLY as a localhost-only posture; a bind to `0.0.0.0`/`::` without a token prints the loud multi-line WARNING (anti-pattern of the open port, §2 driver 4) and stays up — the WARN is the owner's explicit trade-off surface, mirroring `mlogserver`'s host WARN, not a hidden default.
- stdio needs no auth: the parent process already owns the transport.

## 4. Alternatives considered

1. **Hand-rolled std-only HTTP server.** Avoids axum but re-implements HTTP parsing/keep-alive/chunking and SSE framing next to a battle-tested stack already in the tree — more code, more risk, no dependency delta (axum is already there). Rejected.
2. **A separate `mcp-serve-http` binary.** Two processes to keep in security parity is exactly the drift §2 driver 1 forbids. One CLI, one core.
3. **Policy as sidecar YAML with a validator.** A validator can check a stale file only against the CURRENT code — the policy still drifts on every method edit. Compiled-from-AST has no staleness failure mode.
4. **rmcp SDK (the official Rust SDK).** A new dependency tree for a protocol our core is already a correct JSON-RPC 2.0 implementation of; the client side already established the hand-rolled posture (ADR-0132 D2). Rejected for v1.

## 5. Consequences

- External MCP clients reach Metalogos tools over HTTP/SSE with the same security posture as stdio — the acceptance target of the wave-3 dispatch (#491, item 3).
- The tool policy in `_meta` gives consumers a mechanical effect profile per tool — the input for #395's dogfood and any remote deployment review.
- `futures-util` becomes a direct (optional, server-scoped) dependency — zero new compiled crates, declared for honest accounting.
- The SSE transport implements the HTTP+SSE shape of the legacy protocol base; modern-only transport variants stay Future (documented boundary, same as the client).

## 6. Threat analysis (server side)

- **Open-port abuse:** the bearer gate (or the loud localhost-only posture) is the first line; the allowlist is the second (only named tools exist); the exec/env gates and label clearance apply inside every method body regardless of who called.
- **Policy as a false assurance:** the compiled policy is an annotation (lexical, conservative) — it does NOT clear anything; security verdicts stay with the gates. Honest limitation: `clearance_args` uses lexical flow, not full data-flow — documented in `mcp_policy.rs`.
- **Session hijacking (SSE):** the session id is a UUID served only on the (optionally bearer-gated) /sse stream; POSTs to a session path without auth (when configured) are 401 before session lookup — a known session id alone grants nothing.
- **Cross-transport drift:** structurally excluded — one `handle_request`, one `execute_tool_method`; the tests pin the matrix (§3.2) anyway.

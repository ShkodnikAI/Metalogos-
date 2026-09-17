# ADR-0132: MCP client — hand-rolled JSON-RPC over stdio, stateless, output with taint `UserInput`

**Status:** Accepted (approved by the owner 2026-09-12; taint kind of MCP output — reuse `UserInput`, D3 confirmed. The approval removes stop-gate 1 of dispatch #267–279 and unblocks #268)
**Date:** 2026-09-11
**Naryad:** #267 (research, issue #303) — implementation in #268 (issue #304)
**Research base:** `docs/research/naryad-267-mcp-recon.md` (all facts captured 2026-09-11: crates.io API, GitHub API, the `modelcontextprotocol/modelcontextprotocol@2026-07-28` specification, grep over `src/` v0.19.0)

## Context

Metalogos is cut off from the main integration standard of AI agents: MCP is entirely absent from the code base, while the `tool` construct (ADR-0054) is implemented, the LLM loop (`call_llm` → SmartRouter) and outbound HTTP (SSRF batch #261) exist, and the exec-gates #253-A give language-level control over process spawning. ADR-0054 §Future Directions records only the reverse bridge (expose tools AS an MCP server); the forward bridge (a Metalogos program **calls** tools of external MCP servers) is not designed. Grant context: the Proposal_Restack section "MCP with language-level security control" — a unique edge: no mainstream MCP client has taint statics and language-level exec-gates.

The current specification revision — **2026-07-28** — split the protocol into modern (per-request `_meta`, no `initialize`) and legacy (handshake, ≤ 2025-11-25). The official Rust SDK `rmcp` exists, is active and mature (3.3.0, Apache-2.0).

## Decision Drivers

1. Dependency discipline FEATURE_INTAKE §5: warning at 2 new crates per version, **hard limit 5**.
2. Execution model: TW/VM are synchronous, builtins are blocking; in serve, handlers run in `spawn_blocking` (ADR-0096) — outside the async context.
3. Security model: exec-gates #253-A, the taint system (`TaintKind`), audit log — reuse, do not duplicate.
4. Interop: maximize existing MCP servers at a minimal protocol surface.

## Decision

### D1. Transport — stdio, spec revision 2026-07-28 as reference

`std::process::Child`, newline-delimited JSON-RPC 2.0 over stdin/stdout (spec: messages MUST NOT contain embedded `\n`; stderr — logs only; shutdown by closing the stream). Rejected: streamable HTTP — it pulls in the spec's OAuth loop (`oauth2`, `jsonwebtoken`), SSE, session management; Future, pending a real remote-server use case.

Interop boundary: the v1 client speaks **legacy** (`initialize` → `notifications/initialized` → `tools/list`/`tools/call`) — it covers servers of revisions 2024-11-05…2025-11-25 and dual-era servers. Modern-only servers are not supported (per the spec compatibility matrix a legacy client is incompatible with a modern-only server — a deliberate break). Modern-era, MRTR, subscriptions, progress — Future.

### D2. Implementation — hand-rolled JSON-RPC client, 0 new dependencies

`std::process` + `serde_json` (both already in the tree), ~300–400 lines.

**Rejected alternative — `rmcp` 3.3.0 (official SDK):**
- dependencies: 9 new crates even for a single stdio transport (`rmcp`, `futures`, `indexmap`, `tokio-util`, `tracing`, `pin-project-lite`, `process-wrap`, `which`, `pastey`) — **exceeds the hard limit of 5**; the HTTP feature would add 15+, including `reqwest 0.13` next to our 0.12 (double TLS stack);
- async model: tokio-async (confirmed: `tokio ^1` is a non-optional core dependency) — every builtin would require an async bridge out of the blocking context (`Handle::block_on`/a one-shot `Runtime`), permanent integration complexity, and a nested-runtime risk of the class dissected in ADR-0096;
- what the SDK honestly provides: upstream maintenance of protocol evolution. The price of the choice — evolution is now our work; accepted, because the v1 surface (D4) is narrow and covered by contract tests. Reopen the Decision when extending to HTTP/OAuth: the dependency math is different there.

### D3. Security design — reuse, not a new policy

- **exec-gate:** `mcp_call`/`mcp_list_tools` call the SSOT `exec_gate(context)` (#253-A) before spawn: `Process` → `METALOGOS_ALLOW_EXEC=1`, `ServeRoute` → `METALOGOS_SERVE_ALLOW_EXEC=1` (replacement, not AND). Denial code — the existing `EXEC_NOT_PERMITTED`. Every spawn entry goes to `METALOGOS_AUDIT_LOG_PATH` with the `mcp` field.
- **Taint kind of output — `UserInput` (reuse).** The output of `mcp_call` gets `TaintKind::UserInput` — the existing Category-A/B checks work without a single change: `UNTRUSTED_TRAINING_DATA` blocks `reflex_train` on MCP data, the pipelines in `respond()`/`write_file()`/`http_post()` are covered. **Rejected alternative — a new `ToolOutput`:** more honest by name, but it requires a new "kind × sink" branch in every existing check — the risk of a missed branch without a single policy that distinguishes kinds. Introduce `ToolOutput` only together with the first such policy (Future). *Explicitly escalated to the owner: the choice of kind is part of this ADR's approval.* Owner decision (2026-09-12): **reuse `UserInput`** — confirmed, the `ToolOutput` alternative remains a Future item.
- **`METALOGOS_MCP_ALLOWLIST`:** comma-separated (trim/empty entries ignored — convention #259), exact match on argv[0]. Unset — does not narrow (only the exec-gate applies); empty string — deny all MCP; non-empty — only the listed commands, denial `MCP_NOT_ALLOWLISTED` (code per ADR-0131 convention). The third Metalogos allowlist after `METALOGOS_ENV_ALLOWLIST` (#259) and `MLOG_VISION_WEIGHTS_ALLOWLIST` (ADR-0125).
- **Field trust:** server info and tool metadata (including descriptions) — no taint; descriptions are third-party text entering LLM context, the prompt-injection surface is honestly recorded as outside the taint system's scope (the program includes them in context explicitly). Call arguments — trusted (statically checked .mlog code). Output — untrusted (see above).

### D4. Scope v1 — tools only, stateless, 2 builtins

- `mcp_call(command, args_json, tool, arguments_json) -> string` and `mcp_list_tools(command, args_json) -> string`. Resources/prompts/sampling, list notifications, subscriptions — not included.
- **Stateless**: spawn → handshake → call → shutdown on every call. The main argument — one-to-one security attribution: one call = one exec-gate = one audit entry (grant demonstration). Cost: +spawn/handshake (~10–50 ms) per call. **Rejected alternative — stateful** (`mcp_start`/`mcp_stop` + a handle registry): cheaper on long chains, but it requires a process registry, orphan cleanup, and the gate "blurs" over time; Future — revisit with real benchmarks from #268.

## Consequences

- Positive: 0 new dependencies; uniformity with the http_* builtins in execution model; exec-gates and taint work from day one; the allowlist gives deployments a minimal surface; the audit log demonstrates security control for the grant.
- Negative / accepted risks: protocol evolution is our responsibility; modern-only servers are outside v1; spawn overhead per call; tool descriptions as a prompt-injection vector are closed only procedurally (documented), not technically.
- Implementation (#268) must deliver: contract tests on a fixture stdio server (handshake, framing, isError, stream truncation, garbage in stdout), integration of `exec_gate`/audit/allowlist, a taint pin by statics (`reflex_train` on MCP output — an error), threat-model and REFERENCE updates.

## Go/No-Go

**GO** for #268 — the ADR was approved by the owner 2026-09-12 (including the D3 taint kind: reuse `UserInput`). The 3–5 day estimate is confirmed by the reconnaissance; no blockers.

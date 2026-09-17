# Naryad #267 — Reconnaissance: MCP client for Metalogos (transport, implementation, security design, v1 scope)

> **Status:** Research report + ADR-0132 draft — not production code. Input material for the owner's decision (stop-gate 1 of dispatch #267–279).
> **Date:** 2026-09-11
> **Priority:** P0 (blocks #268 — the MCP stdio client).
> **Method:** all external facts captured 2026-09-11 from primary sources (crates.io API, GitHub API, raw files of the `modelcontextprotocol/modelcontextprotocol@2026-07-28` specification), all internal ones — via grep over `src/` of v0.19.0 at commit `e5eb2c8`. Nothing "from memory".

---

## Block 0 — Fact-check of the starting task statement

The naryad's task statement is confirmed against the v0.19.0 code:

- MCP is entirely absent from the codebase: `grep -ri "model context protocol\|mcp_" src/` — 0 meaningful matches (`mcp_` matches are absent; the only occurrences of the substring "mcp" are accidental, inside other identifiers, and unrelated to the protocol).
- The `tool` construct is implemented and alive: `Declaration::Tool` in `src/ast.rs` (ADR-0054), contract pin — 9 tests in `tests/tool_abstraction_contract.rs`.
- The LLM circuit exists: `call_llm`/`call_claude` → SmartRouter (naryad #4), `llm_usage`; taint kind `LlmOutput` for their result (`src/audit.rs`, `TaintKind`).
- Outbound HTTP exists: `http_get`/`http_post`/`http_post_multipart`/`http_download` on `reqwest 0.12 (blocking)`, closed off by SSRF package #261 (redirects are not followed, address classes extended).
- Exec gates #253-A work in `src/builtins/io.rs`: the SSOT function `exec_gate(context)` with `ExecContext::Process` / `ExecContext::ServeRoute`, code `EXEC_NOT_PERMITTED`, subprocess audit in `METALOGOS_AUDIT_LOG_PATH`.
- ADR-0054 §Future Directions records only the **reverse** bridge ("Expose Metalogos tools as MCP tools" — us as a server). The forward bridge (us as a client to third-party MCP servers) is not designed anywhere. Dispatch #267–279 assigns the reverse bridge to work **after** #268.

Conclusion: the integration surface is new code from scratch; the reusable surroundings are exec gates, the taint system, the audit log, `serde_json` (already in the tree).

---

## Block 1 — Transport: `stdio` vs `streamable HTTP`

### Facts from the specification (revision 2026-07-28 — the current stable)

Specification tags: `2024-11-05` → `2025-03-26` → `2025-11-25` → **`2026-07-28`** (the latest stable, published ~6 weeks ago; a `draft` follows it in the repo).

`stdio` (`docs/specification/2026-07-28/basic/transports/stdio.mdx`), verified verbatim:

- the client launches the MCP server as a **subprocess**; the exchange is JSON-RPC 2.0 over `stdin`/`stdout`;
- messages are separated by **newlines**; embedded `\n` inside a message is **forbidden** (MUST NOT) — framing is trivial: line-by-line reading + `serde_json::from_str`;
- `stderr` — logs only; the client MAY capture/ignore it and SHOULD NOT treat it as a sign of error;
- the server MUST NOT write anything but valid MCP messages to `stdout`; the client MUST NOT write anything but valid messages to `stdin`; the client does not write responses at all (only requests/notifications);
- lifecycle: shutdown = close the stream; restarting the process is the client's responsibility;
- the spec explicitly permits reusing the same framing over Unix sockets/TCP, but the subprocess semantics (launch, stderr, shutdown) remain ours.

The key innovation of the 2026-07-28 revision (`basic/versioning.mdx`): the protocol split into **modern** (version/identity/capabilities are passed per-request in `_meta`; no sessions, no `initialize` handshake) and **legacy** (the handshake `initialize` → `notifications/initialized`, revisions ≤ 2025-11-25). The spec's compatibility matrix: a legacy client against a modern-only server **does not work**; dual-era servers serve legacy clients via `initialize`.

`streamable HTTP` (`transports/streamable-http.mdx`): an HTTP transport with server-side authorization (a separate section `basic/authorization/` — OAuth 2.0, authorization-server discovery, dynamic client registration), the `MCP-Protocol-Version` header, SSE streams.

### Choice for v1: **stdio** — exactly one transport

1. **Ecosystem coverage.** The overwhelming majority of 2026 MCP servers are local processes (filesystem, git, sqlite, playwright, dozens of reference ones from `modelcontextprotocol/servers`) launched by a command. stdio covers all of them.
2. **Zero authorization.** streamable HTTP drags in the spec's OAuth circuit — a separate project by volume, incompatible with the #268 budget (3–5 days) and with dependency discipline (see Block 2: the official OAuth crates are `oauth2`, `jsonwebtoken`).
3. **Natural fit with the security model.** stdio = subprocess = exec gates #253-A apply "for free" and by meaning (launching a third-party process is precisely exec). For streamable HTTP we would have to design a new outbound-connection gate on top of SSRF package #261 with different semantics (a long-lived session vs request/response).
4. **Framing — one function.** Newline-delimited JSON-RPC over `std::process::Child` stdin/stdout: `BufReader::lines()` + `serde_json`. All protocol specifics fit into a handshake + 2 methods (Block 4).

**Rejected for v1 — streamable HTTP:** OAuth surface, SSE streams, session management; moved to Future in ADR-0132. Revisit the question on a real use case (the first remote MCP server the owner actually needs).

**v1 compatibility boundary (honest):** our client speaks **legacy** (the `initialize` handshake) — this covers all servers of revisions 2024-11-05…2025-11-25 and all dual-era servers. Modern-only servers (revision 2026-07-28 without a legacy mode) will not be supported by v1 — per the spec's compatibility matrix this is a deliberate break, recorded in ADR-0132 as Future. Today the share of modern-only servers is negligible (the revision is ~6 weeks old; all major SDKs retain legacy support).

---

## Block 2 — Implementation: the official Rust SDK (`rmcp`) vs a hand-rolled JSON-RPC client

### 2.1 Factual profile of `rmcp` (the official SDK, crates.io + GitHub API, captured 2026-09-11)

| Parameter | Fact |
|---|---|
| Crate | `rmcp` — "Rust SDK for Model Context Protocol", repo `modelcontextprotocol/rust-sdk` (the specification's official org) |
| Version | **3.3.0** (released 2026-09-10, i.e. yesterday; summer 2026 release cadence: 3.1.2 2026-08-07, 3.1.3 2026-08-17, 3.1.4 2026-08-20, 3.2.0 2026-08-31, 3.3.0 2026-09-10 — a release every 1–2 weeks) |
| License | Apache-2.0 (in the crates.io metadata of the version; the GitHub repo shows NOASSERTION due to an exception file — what matters for the tree is the crates.io label) |
| Activity | 3915 stars, 51 open issues, last push 2026-09-11 (the current day) — actively maintained |
| Adoption | 25 731 586 total downloads, 13 230 132 over the last ~90 days |

### 2.2 Dependencies of `rmcp 3.3.0` — actual count (crates.io `/v1/crates/rmcp/3.3.0/dependencies`)

Non-optional `normal` dependencies of the core (what enters the tree under any use):

`chrono`, `futures`, `indexmap`, `pin-project-lite`, `serde`, `serde_json`, `thiserror`, `tokio`, `tokio-util`, `tracing` — **10 crates** (plus `rmcp-macros`, marked optional — needed for server-side derive macros, not required for the client role).

Transport features are optional: the stdio client pulls in `pastey`, `process-wrap ^10`, `which ^8`; streamable-HTTP pulls in `reqwest ^0.13.2`, `hyper`, `hyper-util`, `http*`, `sse-stream`, `oauth2 ^5`, `jsonwebtoken ^11`, `base64`, `hmac`, `sha2`, `rand`, `zeroize`, `url`, `uuid`, `tokio-stream`, `tower-service`, `async-trait`.

### 2.3 Impact on the Metalogos tree

Current state (Cargo.toml v0.19.0, **47 direct dependencies**): `tokio` is already present (direct, features = ["full"] — for serve), `serde`/`serde_json`/`thiserror`/`chrono` are already present, `reqwest 0.12` is already present.

| Scenario | New crates in the tree | Against FEATURE_INTAKE §5 |
|---|---|---|
| Hand-rolled JSON-RPC client (`std::process` + `serde_json`) | **0** | Would fit even under the warning threshold (2) |
| `rmcp` + stdio feature | `rmcp`, `futures`, `indexmap`, `tokio-util`, `tracing`, `pin-project-lite` + transport: `process-wrap`, `which`, `pastey` = **9** | **Hard limit of 5 exceeded** |
| `rmcp` + streamable-HTTP feature | the 9 from the row above + `reqwest 0.13` (a duplicate version next to our 0.12!), `hyper*`, `sse-stream`, `oauth2`, `jsonwebtoken`, … = **15+** | Exceeded threefold |

A separate note on the duplicate `reqwest` version: rmcp 3.3.0 requires `^0.13.2`, the Metalogos tree is pinned to 0.12 — cargo will carry both major versions in parallel (a duplicated TLS stack in the binary; our binary-size budget is a warning at 8 MB).

### 2.4 Async model — the task statement's expectation confirmed by fact

The task statement expected "the SDK is almost certainly tokio-async". **Confirmed**: `tokio ^1` is a non-optional non-dev core dependency of rmcp (alongside `futures`, `tokio-util`); the SDK's public API is `async` traits (`ServiceExt`, async transport connectors).

Compatibility with the Metalogos runtime (facts):

- TW and VM are synchronous interpreters; builtins are blocking functions returning `Result<Value, String>`.
- `mlog serve` builds a tokio multi-thread runtime (`src/main.rs:480`), but route handlers execute the DSL via `spawn_blocking` (5 sites in `src/server.rs`, ADR-0096) — i.e. even in serve, a builtin is invoked **outside** the async context, on a blocking thread.
- `mlog run` (CLI) keeps no permanent runtime; `reqwest::blocking` internally spins up its own one-shot runtime — this pattern works precisely because the builtin call is synchronous.

Integrating a tokio-async SDK into a blocking builtin means one of: (a) a dedicated runtime thread + `Handle::block_on` on every call with data shuttled between threads; (b) its own `Runtime::block_on` per `mcp_call` — the loss of the stateless model's time advantage (see Block 4) and one more nested-runtime risk of exactly the class ADR-0096 already analyzed (the panic "runtime dropped within async context"). This is solvable, but it is a permanent per-call integration cost, not a one-off.

Compatibility with TW/VM: the hand-rolled client is an ordinary blocking call, identical to `http_post` in execution model (including VM parity: the `execute_code` path for external calls is no different from http_*). The SDK variant would require an async bridge **both** in TW **and** in VM.

### 2.5 Comparison table

| Criterion | `rmcp` 3.3.0 (official SDK) | Hand-rolled JSON-RPC client (~300–400 lines) |
|---|---|---|
| New dependencies | 9 (stdio) / 15+ (HTTP) — **exceeds the hard limit of 5** | **0** (`std::process` + `serde_json` already in the tree) |
| Async model | tokio-async (confirmed by the facts of §2.4) | Synchronous — matches the model of all builtins and of ADR-0096 |
| Protocol correctness | Maintained upstream; the SDK keeps pace with spec revisions | Only what we have implemented; the v1 surface is narrow: `initialize` + `tools/list` + `tools/call` + 2 notifications — verifiable by contract tests |
| Evolution speed | A release every 1–2 weeks — upgrades are not free (see reqwest 0.13) | No external upgrades |
| License / activity | Apache-2.0; release 2026-09-10, push 2026-09-11 | — |
| TW/VM fit | An async bridge in both backends | Identical to `http_*` |
| Own code volume | ~0 lines of protocol, but an integration async bridge + type conversions | ~300–400 lines: spawn + framing + handshake + 2 methods + error mapping |

### 2.6 Conclusion

**Hand-rolled JSON-RPC client.** Decisive arguments: (1) the dependency budget — the SDK exceeds the FEATURE_INTAKE §5 hard limit by 1.8–3x, manual = 0; (2) the async mismatch — the SDK forces a tokio bridge on top of a blocking runtime, breaking the economics of ADR-0096; (3) the v1 scope (Block 4) narrows the protocol to 1 handshake + 2 methods + 2 notifications — a surface where a hand-rolled implementation is verifiable and cheap to maintain.

The honest price of the choice (recorded in ADR-0132 as consequences): protocol evolution is now our work. Modern-era spec features (per-request `_meta` instead of a handshake), MRTR (`InputRequiredResult`), subscriptions, progress notifications — all of this is Future, not v1. If the scope ever expands to HTTP transport/OAuth, the SDK decision should be reopened: there the dependency math is different (an OAuth stack would have to be taken anyway, and then it is 9 SDK crates vs 12+ hand-rolled).

---

## Block 3 — Security design (the main part — the unique Restack edge)

### 3.1 (a) Spawning an MCP server = exec → inherits gates #253-A

Launching an MCP server is by nature launching a third-party process, i.e. semantically equivalent to `exec()`. Design: **reuse, not a new policy**. `mcp_call`/`mcp_list_tools` call the SSOT `exec_gate(context)` (`src/builtins/io.rs`) before the spawn:

| Call context | Gate | Variable |
|---|---|---|
| `mlog run` / `mlog serve` top level | `ExecContext::Process` | `METALOGOS_ALLOW_EXEC=1` |
| serve route body | `ExecContext::ServeRoute` | `METALOGOS_SERVE_ALLOW_EXEC=1` |

The entire established semantics is inherited: the error code `EXEC_NOT_PERMITTED` (a stable diagnostic code per ADR-0131), the "replacement, not AND" rule for the serve context (the lesson of #253-A: the process flag does not apply in routes, the route flag does not require the process flag), the subprocess audit log in `METALOGOS_AUDIT_LOG_PATH` (the same channel `exec()` already writes to; entries gain an `mcp` field with the server command name and the tool). No new flags, codes, or contexts are introduced for v1 — the gate is already written, tested (the #253/#259 packages), and documented.

### 3.2 (b) Taint kind of the `mcp_call` result: reuse `UserInput` vs a new `ToolOutput`

The output of an external tool is untrusted data: the server is controlled by a third party, its output may contain prompt injection, PII, poisoning data. The question is which taint kind to assign.

**Option R — reuse `UserInput`** (like `form_data`/`json_body`/`query_param`):

- **0 changes** in `src/audit.rs` and in the threat model: all existing Category-A/B checks already know `UserInput`.
- `UNTRUSTED_TRAINING_DATA` (Category-A) is covered automatically: `reflex_train(data, labels)` with MCP output among its arguments will be rejected statically — poisoning the model through an MCP tool is impossible from day one.
- The pipelines `mcp_call → respond()` / `→ write_file()` / `→ http_post()` fall under the existing untrusted-data rules.
- Risk: `UserInput` semantically means "user input" — MCP output is technically "input of a remote tool". For the current checks there is no difference in consequences (both entities are equally untrusted), but the kind's name in diagnostics can be confusing.

**Option T — a new `ToolOutput`:**

- Honest kind semantics ("data of an external tool") — precise diagnostic messages.
- Cost: every existing Category-A/B check (`SECRET_LEAK`, `UNTRUSTED_TRAINING_DATA`, SVG/HTML lint, vision checks — see `docs/threat-model.md`) gains a new branch in the matrix "which kind with which sink". Missing even one branch = a hole that reuse would not have created. On v0.19.0 the checks that know `TaintKind` number more than a dozen tracking sites + checks matching `UserInput`/`LlmOutput`/`Secret` by name.
- The bonus that would have justified the option: policy differentiation (e.g. "ToolOutput may go into LLM context, UserInput may not") — but today such a policy exists **for no kind at all**, and the #268 task statement does not require it.

**Recommendation: option R (`UserInput`) for v1.** Conservatism for free: reuse gives exactly the same guarantees with zero risk of a missed branch. `ToolOutput` is a deliberate Future in ADR-0132: introduce it together with the first policy that actually distinguishes kinds, not earlier. **This is stop-gate 1: the final decision belongs to the owner** — ADR-0132 §Decision carries the question as a separate item.

### 3.3 (c) Allowlist of server commands `METALOGOS_MCP_ALLOWLIST`

The exec gate (3.1) decides "whether exec is allowed at all in this context", but not "which exact server is allowed". The semantics of the proposal:

- Format: comma-separated; whitespace at the edges of elements is trimmed, empty elements are ignored — the **`METALOGOS_ENV_ALLOWLIST` convention of naryad #259**, uniformity of Metalogos flags.
- Matching: an **exact match of the first token of the command** (argv[0] as written in the invocation: `uvx`); the optional full pattern `argv[0] + argument substring` we are not building — v1 stays simple. Example: `METALOGOS_MCP_ALLOWLIST="uvx,npx,node"`.
- States:
  - **unset** — the allowlist narrows nothing: only the exec gate applies (the Metalogos default: opt-in tightening, not opt-out permissions; the default position is "exec is already permitted by a flag, MCP is no worse");
  - **empty string** (`METALOGOS_MCP_ALLOWLIST=""`) — an explicit deny of all MCP (code `MCP_NOT_ALLOWLISTED` — a new diagnostic code per the ADR-0131 convention, this one only, no new flags);
  - **non-empty** — only the listed commands are allowed; a refusal carries `MCP_NOT_ALLOWLISTED` + the command name.
- Precedents in the codebase: `METALOGOS_ENV_ALLOWLIST` (#259), `MLOG_VISION_WEIGHTS_ALLOWLIST` (provenance gates #125/ADR-0125) — the third Metalogos allowlist, not a new mechanism.
- Audit: allowed calls are also written to `METALOGOS_AUDIT_LOG_PATH` (who, when, which server, which tool — the grant's "language-level security control" section demonstrates this with a live log).

An open question for the owner (not blocking v1): should the **serve context** unconditionally require an allowlist (deny when unset)? The reconnaissance position: no — a double default-deny (the exec flag AND the allowlist) breaks the "replacement, not AND" principle and complicates the model without a real use case; serve routes are already denied by default by gate #253-A.

### 3.4 (d) Trust in protocol fields

| MCP field | Status | Handling |
|---|---|---|
| Server info (name/version from `initialize`) | metadata | No taint; does not reach sinks; for error messages/logs |
| Tool list: names, `inputSchema`, **descriptions** | metadata | No taint, but we record honestly: descriptions are third-party text that reaches LLM context if the program puts it there; this is a prompt-injection surface that the v1 taint system does not close (no SDK closes it — it is the nature of LLM agents). Grant wording: "tool metadata receives no taint but also does not pass through the language's security sinks; the decision to include descriptions in LLM context is made explicitly by the program" |
| Call arguments `arguments_json` | trusted | Literals/variables from .mlog code — statically checked by the compiler; no taint assigned |
| **`mcp_call` output** | **untrusted** | Taint `UserInput` (3.2) — all Category-A/B checks are active |

### 3.5 Alignment with the threat model

The design adds to `docs/threat-model.md` (in #268) exactly one new "untrusted data" entity row — MCP output — and reuses both existing defense lines: exec gates (primary) and static taint analysis (secondary). No new sinks, no new Severity levels, no new Categories. The OWASP mapping correspondence holds: `UNTRUSTED_TRAINING_DATA` is already assigned to A09/A02, exec gates to A03/A08.

---

## Block 4 — v1 scope and API shape

### 4.1 Scope: tools only, 2 builtins

- **Tools only**: `tools/list` + `tools/call`. Resources/prompts/sampling are not implemented (not needed for the scenario "the language invokes a server tool"; each of these extensions = separate methods, list caching, reverse channels).
- **v1 builtins (2):**
  - `mcp_call(command, args_json, tool, arguments_json) -> string` — tool invocation;
  - `mcp_list_tools(command, args_json) -> string` — a JSON list {name, description, inputSchema} for tool selection by the program/LLM.
- Not in v1: progress notifications, `notifications/tools/list_changed`, subscriptions, MRTR/`InputRequiredResult` (the 2026-07-28 revision supports it — we do not request it in legacy mode), logging notifications (written by the server to stderr — we could route them into the audit log if desired, but do not parse them).

### 4.2 API shape: stateless (recommendation) vs stateful

| | **Stateless** `mcp_call(...)` | Stateful `mcp_start` → `mcp_list_tools`/`mcp_call` → `mcp_stop` |
|---|---|---|
| Lifecycle | spawn → handshake → call → shutdown **on every call** | Manual; the process lives between calls |
| Infrastructure | No registry, no handles in VM/TW, no leak surface | A process registry (`dashmap` exists), a handle as a `Value` variant or a numeric handle, cleanup on completion of a VM frame/route, handling of hung servers, cancel semantics |
| Call cost | +spawn/handshake (~10–50 ms for typical uvx/npx servers — acceptable for v1; the handshake = 1 RTT over a local pipe) | RTT only |
| Sandbox hygiene | The process does not outlive the call — no state between calls to clean up | Orphan processes on an error between start/stop — the problem class of `exec` naryads |
| Gate alignment | Each call is a separate exec gate + audit entry: trivial attribution | The gate on start; calls by handle bypass the exec gate — the gate "smears out" over time |

**Recommendation: stateless for v1.** The main argument is security attribution: with stateless, every exec gate and every audit entry corresponds to one call one-to-one — which is precisely the grant's demonstration of "language-level security control". The stateful optimization is a deliberate Future in ADR-0132 (with a registry, a TTL, and a gate on the first use of a handle). If the #268 benchmark shows unacceptable overhead for a real scenario (long chains of calls to one server) — revisit based on data, not on speculation.

---

## Block 5 — ADR-0132 draft

Ready: `docs/adr/0132-mcp-client.md` (in this PR). Status **Proposed** — approval by the owner = stop-gate 1 of the dispatch. Contains: context, decisions (stdio / hand-rolled client / stateless / `UserInput` / allowlist semantics), explicitly rejected alternatives (the SDK, streamable HTTP, stateful, `ToolOutput`, no allowlist), consequences and risks, explicit Future items, Go/No-Go.

---

## Block 6 — Go/No-Go for #268

### **GO** — provided all five conditions hold (each is recorded above and in ADR-0132):

1. A hand-rolled JSON-RPC client (`std::process` + `serde_json`), **0 new dependencies**;
2. Transport **stdio** (newline-delimited JSON-RPC 2.0), the legacy `initialize` handshake; modern-only servers are outside v1 (a recorded boundary);
3. **Stateless** API: `mcp_call` + `mcp_list_tools`, a spawn per call;
4. Security: reuse `exec_gate` (#253-A) + taint `UserInput` on the output + the optional `METALOGOS_MCP_ALLOWLIST` (convention #259) + the audit log;
5. The 3–5 day estimate holds up: the protocol surface (1 handshake + 2 methods + 2 notifications) ≈ 300–400 lines of client + 2 builtins + contract tests with a local fixture server (a small `python3`/`node` script or a fixture on the Rust test harness emulating a stdio server).

No blockers found. The only owner decision before the start of #268 is approval of ADR-0132 (stop-gate 1, including the choice of the taint kind in 3.2).

---

## Cross-references

- **ADR-0054** (`tool`, Future Directions — the reverse bridge after #268), **ADR-0096** (blocking/spawn_blocking — the argument against an async SDK), **ADR-0131** (the `MCP_NOT_ALLOWLISTED` code convention), **ADR-0125** (the allowlist precedent).
- **Naryad #253-A** (exec gates — reuse), **#259** (ENV_ALLOWLIST — the format convention), **#261** (SSRF — the boundary with the Future HTTP transport), **#252** (the audit log).
- Dispatch #267–279: grant context C3 (the Proposal_Restack section "MCP with language-level security control"); idea A1. Blocks **#268** (issue #304).

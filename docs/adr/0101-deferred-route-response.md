# ADR-0101: Deferred Route Response (post-respond continuation)

**Status:** Accepted (contract phase — implementation deferred to separate narad)
**Date:** 2026-08-10
**Narad:** #69 (Block 2), implementation → future narad

## Context

In webhook handling (e.g., Telegram bots), the server must ACK the request
quickly (within Telegram's retry window) but continue processing afterward
(e.g., preparing a report via OSP). Currently, `execute_route_body` in
`src/server.rs` returns immediately upon encountering an `HttpResponse` value
— any code after `respond()` in the route body is **never executed**.

Both backends exhibit this behavior:
- Tree-walking interpreter: `Statement::ExprStmt` / `Statement::Return`
  checks `if let Value::HttpResponse { .. } = val { return Ok(...) }`.
- VM: same early-return pattern in `execute_route_body_vm`.

The deferred-response feature requires continuing execution after the HTTP
response has been sent to the client, which means code after `respond()` must
run in a background task.

## Decision

### 1. Syntax: No grammar changes

Code after `respond()` in a route body becomes "background work" implicitly.
No new syntax (`spawn { ... }`) is introduced. This keeps the parser,
crosscheck infrastructure, and existing `.mlog` programs untouched.

The first `respond()` (or any expression producing `HttpResponse`) in a route
body sends the response and marks the "response point." All subsequent
statements in the same route body continue executing in a spawned background
task.

**Rationale:** Introducing `spawn { ... }` would require parser changes
(new keyword, new statement type), crosscheck updates for both backends,
and a new concept in the language. The implicit approach is simpler and
sufficient for the webhook use case.

### 2. Audit logging for background work

Currently, `flush_audit_to_db()` is called once at the return point, capturing
all audit entries from the interpreter. When the response is sent before route
body completion, audit entries from the background phase must also be captured.

**Decision:** After spawning the background task, call `flush_audit_to_db()`
again when the background task completes. The background task holds its own
clone of the per-request interpreter, so its audit entries are isolated.
The implementation will:
1. Flush audit at the response point (before sending to client) — covers
   everything up to `respond()`.
2. After the background task completes, flush audit again — covers everything
   after `respond()`.
3. Both flushes use the same DB connection pattern as the existing
   `flush_audit_to_db()` in server.rs.

### 3. Errors in background work

If code after `respond()` panics or returns an error, the client has already
received the HTTP response and cannot be notified.

**Decision:** Background errors are caught by the spawned task and logged to
the server's stderr/stdout via `tracing::error!` (or `eprintln!` in the
current minimal logging setup). The error must NOT propagate to the tokio
runtime — the spawn closure wraps execution in a `match` that logs errors
without panicking.

### 4. TW/VM parity

Both `execute_route_body` (tree-walking) and `execute_route_body_vm` must
implement the same deferred-response behavior. The crosscheck suite must
include a golden test for deferred response to catch divergence early.

**Decision:** The refactoring follows the same pattern as narad #67 (recipe_search):
- Identify the early-return point in both functions.
- Replace with: (a) send response, (b) spawn remaining statements as background
  task, (c) return response from the outer function.
- Both backends use the same spawn mechanism (`tokio::spawn`).
- The implementation narad adds a golden test `p69_deferred_response.golden`
  (or equivalent crosscheck case).

## Prior Art

- **Node.js:** `res.end()` sends the response; code continues executing
  unless `return` or `next()` is called. Background work is done via
  `Promise` or `setImmediate()`.
- **Go (net/http):** `w.Write()` sends data; the handler function continues.
  Background goroutines are spawned explicitly.
- **AWS Lambda:** `context.callbackWaitsForEmptyEventLoop = false` allows
  the Lambda to return a response while async callbacks continue.

## Consequences

- Route bodies gain implicit "background phase" after first `respond()`.
- No syntax changes — all existing `.mlog` programs continue to work
  identically (they don't have code after `respond()`).
- Implementation requires careful interleaving of response-sending and
  background-task spawning in both `execute_route_body` and
  `execute_route_body_vm`.
- Sandbox enforcement continues to apply to the background phase
  (same interpreter instance, same restrictions).

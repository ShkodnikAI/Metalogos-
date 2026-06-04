# ADR-0028: HTTP Server with Axum

**Status:** Accepted
**Date:** 2026-06-03
**Phase:** 6.1 — HTTP Server

## Context

Phase 6 introduces web-facing capabilities to Metalogos. The interpreter needs an HTTP server to serve `.mlog` web applications, handle routes, and render templates. The server must be embedded directly into the Metalogos runtime — not launched as a separate process.

Key requirements:
- Async I/O for handling concurrent HTTP requests.
- Composable middleware stack for logging, auth, and CSRF.
- Tight integration with the Metalogos interpreter so route handlers can call patterns directly.

## Decision

Use **Axum** as the HTTP framework, built on **Tower** middleware and **Tokio** async runtime.

```mlog
mlogserver {
    listen ":8080"

    route GET "/hello" => {
        respond status:200 body:"Hello!"
    }
}
```

### Implementation Details

1. **Pest DSL:** A new `mlogserver` block is parsed as a top-level declaration in the Metalogos grammar, defined in a dedicated Pest rule file.
2. **Route Mapping:** Each `route METHOD "/path" => { handler }` compiles to an Axum router entry.
3. **Handler Execution:** The handler block runs in the Metalogos interpreter with access to `request`, `params`, `session`, and `response` bindings.
4. **Tower Middleware:** Standard layers — tracing/logging, CORS, compression — applied via Tower's `ServiceBuilder`.

## Prior Art

- **Rust/Axum:** Tower-based, ergonomic routing, async-first.
- **Elixir/Phoenix:** Embedded web server as part of the language ecosystem (Cowboy/Plug).
- **Go/Net::HTTP:** Standard library includes HTTP server primitives.

## Consequences

- **Positive:** Axum provides a mature, performant async HTTP stack with composable middleware.
- **Positive:** Tower middleware integrates naturally with future auth, rate-limiting, and logging layers.
- **Neutral:** Tokio becomes a required dependency of the Metalogos runtime, increasing binary size.
- **Neutral:** All route handlers run asynchronously; synchronous Metalogos pattern calls are wrapped in `tokio::spawn_blocking` if they block.

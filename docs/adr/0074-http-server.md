# ADR-0074: HTTP Server — Axum

**Status:** Accepted
**Date:** 2026-06-02
**Phase:** 6.1

## Context

Metalogos needs to serve HTTP requests for web applications, bots, and APIs.
Phase 6 introduces web capabilities, starting with a minimal HTTP server.

## Decision

Use **Axum** (tower-based async web framework) as the HTTP server backend.

### Why Axum

1. **Tower ecosystem** — Axum is built on `tower::Service`, the de facto standard
   for async service composition in Rust. Middleware, routing, and state management
   all work through tower's `Layer` and `Service` traits. This gives us a mature
   middleware stack for future phases (CORS, sessions, rate limiting, auth).

2. **Async runtime compatibility** — Axum requires tokio, which we already
   need for the LSP server (`mlog-lsp`). Sharing the same async runtime avoids
   bloat and compatibility issues.

3. **Production-grade** — Axum is maintained by the tokio team. It powers
   real-world production services. Performance is competitive with the best
   Rust HTTP frameworks (comparable to Actix-web in benchmarks).

4. **Ergonomic handler model** — Handlers are plain async functions. State
   is extracted via typed extractors. This maps cleanly to Metalogos' declarative
   route model: `route "/" method=GET { respond("Hello") }` → `async fn() → String`.

5. **Matchit routing** — Axum uses matchit for path matching, which supports
   path parameters (`:id`), wildcards (`*path`), and exact matches. This covers
   all routing needs for Phases 6.1–6.6.

### Why not alternatives

| Alternative | Rejected because |
|---|---|
| **Actix-web** | Uses its own actor framework (Actix), not tower. Different async runtime semantics. Would require a separate runtime from tokio (already used by LSP). |
| **Rocket** | Requires `#![feature]` nightly Rust for some features. Tight coupling to its own macros. Less flexible for composable middleware. |
| **Warp** | Filter-based API is powerful but verbose for simple routes. Steeper learning curve for Metalogos contributors. Less intuitive mapping from declarative syntax. |
| **Hyper directly** | Too low-level. No built-in routing, state extraction, or middleware. We'd rebuild what Axum gives us for free. |
| **Poem** | Good framework but smaller ecosystem. Less battle-tested than Axum/tower. |

### Prior art

- **Phoenix** (Elixir): Endpoints framework on top of Plug (similar to tower layers).
- **Express** (Node.js): Middleware chain pattern, similar to tower Layer composition.
- **Gin** (Go): Minimal HTTP framework with fast routing — similar philosophy.

## Consequences

- Root `Cargo.toml` gains `tokio` (rt-multi-thread, macros, net) and `axum = "0.7"` as dependencies.
- `main.rs` switches to `#[tokio::main]` for async runtime support. Sync commands (`run`, `repl`, `check`) continue to work as blocking operations on the tokio runtime.
- New `src/server.rs` module: parses server blocks, builds axum Router, starts TCP listener.
- New CLI command: `mlog serve <file.mlog>`.
- `server` is a new declaration type in grammar, AST, parser, and interpreter.
- Phase 6.1 scope: GET routes with `respond(string_literal)` only. POST, templates, DB, auth are subsequent phases.
- The `serve` command runs non-server declarations (entities, patterns, flows) through the interpreter before starting the server — this populates the namespace for future route handlers that call patterns.

## Syntax

```mlog
server {
  port: 8080
  route "/" method=GET {
    respond("Hello from Metalogos!")
  }
  route "/health" method=GET {
    respond("OK")
  }
}
```

## Future phases

- 6.2: `template` for type-safe HTML, `respond` with pattern evaluation
- 6.3: `db` + `query()` for parameterized SQL
- 6.4: `Secret`, `Encrypted` types
- 6.5: Middleware (session, CSRF, security headers, `requires=[role]`)
- 6.6: Webhook integrations (Telegram, Discord)

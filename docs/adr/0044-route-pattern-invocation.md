# ADR-0044: Route Pattern Invocation Fix

**Status:** Implemented
**Date:** 2025-06-08
**Priority:** Blocker — 13 Fosved Office dispatch patterns couldn't run from route handlers.

## Context

Route handlers in `server.rs` created a fresh `Interpreter::new()` for each HTTP request. While `clone_definitions_into()` copied `patterns`, `learnable_patterns`, `templates`, `struct_types`, and `variables`, several critical runtime components were NOT available in the per-request interpreter:

1. **`db_conn`** — SQLite database connection. Patterns using `query()` would fail with "no database connection".
2. **`db_url`** — Resolved database URL. The per-request interpreter had no way to open a new connection.
3. **`memory`** — InMemoryStore was created fresh per request (not shared). With persistence configured, SQLite-backed store was initialized separately.
4. **`embedding_manager`** — Embeddings for semantic `recall()` were default-initialized per request.

## Root Cause

The per-request interpreter setup (`execute_route_body` in `server.rs`) called `clone_definitions_into()` but:
- Did NOT copy `db_url`, so per-request interpreters couldn't open database connections
- Did NOT call `reconnect_db()`, so even if db_config was copied, no actual connection existed
- Memory was only configured if `state.memory_persist` was set; InMemoryStore was not shared

## Symptoms

| Scenario | Before Fix | After Fix |
|---------|------------|-----------|
| Simple pattern from route | Pattern found, works (no external deps) | Same |
| Pattern using `query()` | Error: "no database connection" | Works (new SQLite conn per request) |
| Pattern using `recall()` | Default embedding manager, empty memory | Works with persist; isolated without |
| Pattern using `memorize()` | Per-request InMemoryStore (not shared) | Shared via SQLite if persist configured |
| Pattern using `call_llm()` | Works (LLM backend is global) | Same |

## Decision

### Changes to Interpreter (`src/interpreter.rs`)

1. **Added `db_url: Option<String>` field** — stores the resolved database URL string after `init_db_connection()` succeeds. This allows per-request interpreters to open their own connections.

2. **Added `reconnect_db()` method** — opens a new SQLite connection using the stored `db_url`. Called by per-request interpreters to get isolated but schema-compatible connections.

3. **Updated `clone_definitions_into()`** — now also copies `db_url` so per-request interpreters can reconnect.

### Changes to Server (`src/server.rs`)

1. **Per-request `reconnect_db()` call** — after `clone_definitions_into()`, the route body executor calls `interp.reconnect_db()` to open a fresh SQLite connection. Each request gets its own connection (safe for concurrent access).

2. **Explicit memory configuration** — clarified that per-request memory uses the shared SQLite DB (when persist is configured).

## Architecture

```
Request → route_handler() → execute_route_body()
  ├── new Interpreter::new()
  ├── clone_definitions_into() from shared interpreter
  │     └── copies: patterns, learnable_patterns, templates, struct_types,
  │         rules, sandboxes, module_namespaces, variables, db_config, db_url
  ├── configure_memory() if persist configured
  │     └── opens new SQLite conn to shared memory.db
  ├── reconnect_db() if db_url present
  │     └── opens new SQLite conn to shared app.db
  ├── set_server_json_body() for POST JSON
  └── execute route body statements
        └── patterns now have access to:
              - patterns HashMap (copied)
              - builtins (from Builtins::new())
              - memory (SQLite-backed if persist)
              - db_conn (fresh per-request SQLite conn)
              - embedding_manager (default per-request)
```

## Concurrency Model

- Each request gets its own `Interpreter` instance (isolation)
- Each request gets its own SQLite connections (memory + db)
- SQLite WAL mode enables concurrent reads
- Pattern definitions are immutable after startup (safe to clone)
- Variables are scoped per-request (route body local env)
- Memory is shared via SQLite (when persist is configured)
- KV store is shared via global static + SQLite write-through

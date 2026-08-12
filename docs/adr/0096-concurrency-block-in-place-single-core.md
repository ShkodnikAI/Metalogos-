# ADR-0096: Replace `block_in_place` with `spawn_blocking` in route handlers

**Status:** Accepted (implemented)
**Date:** 2026-08-07
**Наряд:** #50 (diagnosis), #52 (implementation, cherry-picked from #51)

## Context

Two concurrent 5-second requests to `mlog serve` took ~10s instead of ~5s.
Investigation revealed two distinct problems:

### Problem 1: Single-core worker thread starvation

`tokio::runtime::Runtime::new()` creates a runtime with `worker_threads =
available_parallelism`. On single-core instances (Render free tier), this
produces 1 worker thread. `tokio::task::block_in_place()` at 5 call sites
in `server.rs` blocks the sole worker, serializing all requests.

### Problem 2: Nested tokio runtime panic

Route handlers in `execute_route_body` and `execute_route_body_vm` used
`tokio::task::block_in_place()` to run synchronous DSL evaluation. This
worked until routes invoked builtins that use `reqwest::blocking::Client`
(e.g. `http_post`, `http_get`).

`reqwest::blocking::Client` internally creates a **tokio `Runtime`** for DNS
resolution and TLS. When this inner runtime is dropped, tokio detects that a
runtime is being dropped from within an asynchronous context and panics:

```
Cannot drop a runtime in a context where blocking is not allowed.
This happens when a runtime is dropped from within an asynchronous context.
```

Even though `block_in_place` marks the thread as "blocking-allowed", tokio's
`Runtime::drop` guard checks at a higher level and still considers us inside
an async context. This makes `block_in_place` fundamentally unsafe when the
blocking code may create and destroy its own tokio runtime.

This explains why `/slow` (which calls `http_post`) panicked in production
regardless of worker count.

## Decision

### Worker count (Наряд #50)

Use `tokio::runtime::Builder::new_multi_thread()` with
`worker_threads(max(4, available_parallelism()))`. `METALOGOS_WORKERS` env var
overrides. Invalid value logs warning instead of panic.

### spawn_blocking replacement (Наряд #51, merged via #52)

Replace **all** `block_in_place` calls in route execution with
`tokio::task::spawn_blocking`:

1. **`execute_route_body`** — wrap the entire statement evaluation loop in a
   single `spawn_blocking` call. This moves the interpreter to a genuine
   OS thread (the blocking thread pool), where `reqwest::blocking::Client` can
   safely create and drop its own runtime.

2. **`execute_route_body_vm`** — replace `block_in_place` with
   `spawn_blocking`. Clone `program` (`Arc<Program>`), `CompiledRoute`,
   `Bytes`, and `HashMap` into the closure so it satisfies `Send + 'static`.

3. **Test helper `call_route_vm_direct`** — same treatment as #2.

4. **Audit flushing** — extracted `flush_audit_entries_to_db(state, entries,
   sandbox)` as a shared implementation. The interpreter path now collects
   audit entries inside the blocking closure and flushes them after `.await`.
   Removed the now-unused `flush_audit_to_db` wrapper.

### Send bound verification

All types moved into `spawn_blocking` closures satisfy `Send`:
- `Interpreter` — fields are `HashMap`, `Mutex<Box<dyn MemoryStore>>`
  (`MemoryStore: Send + Sync`), `Mutex<Box<dyn KgStore>>`, etc. No `Rc` or
  `RefCell`.
- `Vm` — fields are `Vec<Value>`, `Vec<CompiledFn>`, `Builtins` (holds
  `fn` pointers, which are `Send`). No `Rc` or `RefCell`.
- `Statement` — `#[derive(Clone)]`, all fields are owned types.
- `Program`, `CompiledRoute` — `#[derive(Clone)]`, all fields are owned.

The old comment "Vm is !Send (holds rusqlite::Connection)" was inaccurate;
the Vm never held a `rusqlite::Connection` directly (builtins open their own
connections via `Arc<Mutex<...>>`).

## Consequences

- **Nested runtime panic is eliminated** — `reqwest::blocking::Client` runs on
  the blocking thread pool, where runtime creation/dropping is safe.
- **Single-core serialization is fixed** — minimum 4 workers + blocking pool
  offload means even single-core hosts handle concurrent requests.
- **Interpreter evaluation is fully offloaded** to the blocking pool, which
  is the correct pattern for sync-heavy work inside an async server.
- **Minor overhead** from cloning `body_stmts` / `CompiledRoute` / `Program`
  per request. Acceptable given the simplicity benefit.
- **Audit flushing is deferred** to after the blocking task completes,
  instead of being interleaved mid-evaluation. Functionally equivalent because
  all entries are flushed before the response is sent.

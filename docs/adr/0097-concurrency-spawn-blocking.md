# ADR-0097: Replace block_in_place with spawn_blocking

## Status
Accepted

## Context
ADR-0096 identified that `block_in_place()` on single-core tokio
serialized all requests. Block 1 of Наряд №51 increased worker count
to minimum 4, but testing revealed a deeper issue: `block_in_place`
panics when `reqwest::blocking` (used by `http_post`/`http_get`)
internally creates and drops a tokio `Runtime` for DNS resolution.

Panic: "Cannot drop a runtime in a context where blocking is not allowed.
This happens when a runtime is dropped from within an asynchronous context."

## Decision
Replace all 6 `tokio::task::block_in_place()` calls in server.rs with
`tokio::task::spawn_blocking().await`. Verified that `Interpreter` and
`Vm` are `Send + 'static` — the old comment about Vm holding
`rusqlite::Connection` was inaccurate (it uses `Mutex<Box<dyn MemoryStore>>`).

## Consequences
- `http_post`/`http_get` work correctly inside route handlers
- Blocking tasks run on tokio's dedicated blocking thread pool
- Two concurrent blocking routes execute in parallel
- Single `/slow` = 5013ms (was: panic)
- `/fast` = 9ms (was: N/A due to crash)

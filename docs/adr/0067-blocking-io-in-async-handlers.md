# ADR-0067: Blocking I/O in Async Handlers

## Status

Accepted

## Context

Metalogos serves HTTP requests via Axum (async) but evaluates route bodies
through a synchronous interpreter that calls `reqwest::blocking` from 21
builtin functions (http_get, http_post, etc.).

On production (FOSVED 0.9.5), a single request took ~5s, but two concurrent
requests took ~10s — serialized. A fast request during a slow one waited
~4s. The blocking calls held tokio worker threads, preventing the
scheduler from driving other tasks.

Additionally, the background scheduler (5s tick) held a write lock on the
shared interpreter for the entire iteration, blocking all request handlers
that needed even a read lock.

## Decision

### Fix 1: `block_in_place` on interpreter eval calls

Wrap the three `eval_statements` calls in `execute_route_body` with
`tokio::task::block_in_place`. This moves blocking work off the async
task's worker thread, allowing the tokio scheduler to park the current
future and drive other tasks on the same thread.

Boundary: the wrapping is done at the async→sync boundary (server.rs),
not inside the sync interpreter or builtins. The builtins are plain
`fn(&[Value]) -> Result<Value, String>` — no `.await` is possible there.

Requires multi-thread runtime (confirmed: `Runtime::new()` defaults to
multi-thread).

### Fix 2: Shorten scheduler write-lock scope

Split the scheduler loop into three phases:
1. Short write lock: collect `check_reminders` + `cron_list` data (owned
   `Value`s). Drop lock.
2. No lock: process reminders (eprintln), evaluate `cron_expr_matches`
   (pure function).
3. Per-job short write lock: fire + mark_fired. Drop lock.

## Alternatives considered

1. **Translate interpreter to async** — massive refactor of the entire
evaluation pipeline. Not proportional to the problem.

2. **`spawn_blocking` for each eval** — requires `'static + Send` for
all captured data. The current code borrows `&mut interp` and `&mut env`
across the loop with interleaved `.await` (flush_audit_to_db). Would need
to restructure ownership for each call.

3. **Replace reqwest::blocking with async reqwest** — cascading change
through all 21 builtins, requiring the interpreter itself to become async.
Separate future discussion.

4. **`block_in_place` (chosen)** — minimal change, preserves existing
ownership patterns, correct semantics for multi-thread runtime.

## Prior art

- Tokio docs, `task::block_in_place`: "Allows a non-send future to be
  run on the current thread, moving other futures off the thread."
- Alice Ryhl, "Async: What is blocking?" — canonical description of this
  class of bug.

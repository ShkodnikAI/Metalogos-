# ADR-0096: Request Serialization via block_in_place on Single-Core Tokio

## Status
Accepted (diagnosis); fix deferred

## Context
Two concurrent 5-second requests to `mlog serve` take ~10s instead of ~5s.

## Root Cause
`tokio::runtime::Runtime::new()` creates runtime with `worker_threads = num_cpus`.
On single-core instances (Render free tier), this produces 1 worker thread.
`tokio::task::block_in_place()` at 5 call sites in server.rs blocks the sole
worker, serializing all requests.

## Decision
Defer fix. Options: `METALOGOS_WORKERS` env var, `spawn_blocking`, dedicated
thread pool.

## Consequences
- Single-core: full request serialization
- Multi-core: unaffected
- Fix requires Send-safe Interpreter or explicit thread pool sizing

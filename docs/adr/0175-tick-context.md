# ADR-0175: The tick context — the cron dispatch executes in the program context

**Status:** Accepted
**Date:** 2026-09-22
**Naryad:** #426 (issue #597; dispatch #598, wave 9 — registry Phase 4; the №423 bring-up defects)
**Pillar:** serve/cron (cross-cutting: server contexts + schema-as-code + the №418 dispatch contract)
**Consumers:** №418 (the cron SSOT — the dispatch CONTRACT is untouched: arity, D1–D6, the fire decision), №423 (the seed — the workarounds close), ADR-0060 (schema-as-code — the replay discipline), №413 (the CRON_JOB_FAILED stamp on every dispatch failure), the office dogfood (№395-class: cron+db apps become expressible)

## 1. Context

The №423 serve-soak bring-up on 0.21.0 surfaced four defects of the cron-dispatch context, each worked around in the seed:

1. **The tick does not bind the program's `db{}`** — the route sees the store, the tick reports "no database connection".
2. **An HTTP self-call from a tick fails at the transport level** (a blocking outbound call from the scheduler tick).
3. **`sqlite::memory:` is isolated per context** — the seed had to fall back to a file URL for cross-context state.
4. **The schema DDL does not reach the cron context** — the seed needed a DDL handler inside the tick.

The mechanics, verified on the merged main:

- **(1)** the scheduler dispatched the tick by calling `call_pattern` DIRECTLY on the shared serve interpreter — the same instance that holds the startup merge. The startup chain runs each declaration on a THROWAWAY interpreter and merges; the `db{}` connection lands on the shared instance through the №381 Arc guard, but a fresh FILE connection is what makes route contexts self-heal (`reconnect_db` opens a new connection when the merged one is absent). Any ordering gap between the connection and the merge left the tick conn-less while routes self-healed — the tick had no reconnect path at all.
- **(2)** the tick executed INSIDE the async scheduler loop while holding the interpreter WRITE lock. Blocking builtins (`http_post`'s `reqwest::blocking`, the ADR-0096 posture) panic in that context, and even a successful call would have blocked every route for the tick's whole duration. The route bodies already run on `spawn_blocking` for exactly this reason.
- **(3)/(4)** each `schema{}` declaration ALSO ran on a conn-less throwaway (fresh `Interpreter::new()`), so its immediate DDL apply failed and the error was swallowed by the merge loop — the DDL never executed at startup; file DBs appeared to work only because earlier connections had created the tables, and memory DBs could not work cross-context at all.

## 2. Decision Drivers

1. **One context construction, not two.** "Тик исполняется в контексте программы" — the tick must be built by the SAME code path as a route body: definitions cloned from the shared interpreter, memory persistence re-attached, db reconnected. Anything else re-creates the drift the №423 defects came from.
2. **The scheduler must not hold locks across user code.** A tick is arbitrary program execution; holding the interpreter write lock across it blocks every route (and, with the №352 duplex, session surfaces too). Lock-free dispatch is also what makes the self-call loop back into a live server.
3. **Blocking builtins are a fact of the builtin surface** (ADR-0096): the tick runs on `spawn_blocking` like route bodies — the route-path posture, not a new one.
4. **Schema-as-code must be order-independent.** The DDL is a DECLARATION (ADR-0060): it travels with the program definitions and replays additively (`CREATE TABLE IF NOT EXISTS`) on every connection that appears after it — startup, route contexts, tick contexts. The declaration order (db before schema or after) must not matter.
5. **The №418 contract is untouchable**: arity 2..5, D1–D6, the pure fire decision, the persisted job store — all unchanged. The dispatch EXECUTION is what this ADR fixes.
6. **Failures stay loud and stamped**: any tick failure surfaces through the existing `CRON_JOB_FAILED`-stamped dispatch error line (№413) — never a silent drop, never a panic escaping the scheduler.

## 3. Decision

### 3.1 The unified tick executor

`execute_tick_call(state, target, args)` (server.rs): builds the route-style program context (`fresh_program_context` — `clone_definitions_into` from the shared interpreter under a READ lock, memory persistence, `reconnect_db`) and executes the target on `spawn_blocking` — builtin-name first, then pattern (the №418 resolution order, unchanged). The scheduler loop holds NO interpreter lock during the call: `mark_fired_with_window` and the fire decision keep their own (lock-free store) paths. Both the cron tick (Phase 3) resolution and the target vocabulary stay byte-identical to №418.

Consequences: the tick gets the same stor-set as a route — `db{}` bound (defect 1 closed), the schema DDL replayed (defect 4 closed), patterns/templates/variables identical; blocking builtins are safe (defect 2 closed — an HTTP self-call loops back into the live server while the scheduler awaits lock-free); routes are never blocked by a tick.

### 3.2 The http self-call boundary

The transport failure was the async-context blocking call + the write lock, not the network: with the unified executor the self-call is an ordinary outbound request from a blocking thread while the server serves concurrently. The deterministic reproduction is the test surface `test_tick_self_call` (a real listener + the self-URL handed to the tick as its argument). A genuine failure (server down, connection refused) surfaces as the `CRON_JOB_FAILED`-stamped dispatch error — the №413 typed contour, unchanged.

### 3.3 `sqlite::memory:` semantics — unified with routes

The in-memory connection is the STARTUP connection, `Arc`-shared into every fresh context by the existing merge discipline (the №381 guard) — routes and ticks see THE SAME memory database. Per-context isolation of `sqlite::memory:` is therefore NOT the semantics: it was the observed symptom of the tick's missing context construction. A context-local memory DB remains reachable the honest way — a file URL is the documented cross-restart store; `:memory:` is the documented same-process shared store. The seed №423 reverts to `sqlite::memory:` (the workaround is closed by construction, and the nightly soak proves it).

### 3.4 Schema DDL — the replay discipline

The interpreter STORES the program's `schema{}` declarations (`schemas: Vec<SchemaDecl>`; they travel through `clone_definitions_into` with the other definition classes, UNION by name, first-wins). Whenever a connection becomes available — `init_db_connection` (startup), `reconnect_db` (a fresh file connection in any context), and once after the startup merge in `build_state` — the stored DDL replays additively (`CREATE TABLE IF NOT EXISTS`, the ADR-0060 discipline; replay failures are loud stderr, never aborting). Consequences: the declaration order stops mattering (schema before db works), a brand-new file DB is schema-ready on first connect, and routes/ticks/the top level see the same schema by construction. The immediate `schema{}` apply on a direct run keeps its historical contract (schema-without-db is a loud error there).

### 3.5 The test surfaces

`test_tick_call(source, target, args)` — the serve state built exactly like `run_test_server` (no listener), one tick through the scheduler executor; `test_tick_self_call(source, target, self_path)` — a real listener + the self-URL as the tick's single String argument. Both are public integration surfaces (the №401 harness posture): the tests assert the db binding, the schema replay, the memory unification and the self-call against the SAME executor the scheduler uses — no 5-second waits, no wall-clock flakes.

### 3.6 Mutation verification (the №382 protocol)

M-CRON-BIND — neuter the tick context's db binding (the `reconnect_db` call in `fresh_program_context`) → the tick-db tests MUST fail; M-SCHEMA-REPL — neuter the replay (`replay_schemas` body) → the schema-visibility tests MUST fail. Harness: `scripts/mutation_verify_426.sh` (the №412/№413 worktree contour).

## 4. Consequences

- Cron+db dogfood applications become expressible without workarounds (the №395-class consumer): the tick is a full program context — db, schema, patterns, ledger.
- The seed №423 loses all four workarounds; the nightly serve-soak (P0–P8) is the standing proof.
- The reminder delivery path (Phase 2) keeps its synchronous dispatch closure under a READ lock — the same defect CLASS exists there (a blocking builtin in a reminder target would panic); it is a documented residual, deliberately out of this naryad's scope (the reminder surface predates the cron SSOT and its failure mode is bounded to due reminders only).
- Untouched by decision: the №418 fire-decision machinery (D1–D6), the dispatcher's scheduling policy (5s loop, no prioritization/worker pool), the №398 thresholds, REFERENCE narrative (№425), the office migration (MLOG_BIN).
- Honest boundaries: the tick context is the TW interpreter construction (the VM serve routes are a separate path — unchanged); `sqlite::memory:` state lives and dies with the process (the documented store semantics); the DDL replay is additive-only — drops/alters remain outside schema-as-code by ADR-0060.

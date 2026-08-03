# ADR-0088: VM Backend for `mlog serve`

## Status

Implemented (Наряд №40, extended by №41). Default remains `interpreter`.
Switch is reversible via `METALOGOS_SERVE_BACKEND=vm`.

## Context

After five нарядs (№36–№39), the VM reached full equivalence with the
interpreter on 58/58 golden examples, with all three crosscheck asserts green.
Compilation: `app.mlog` (2,496 lines) → 216 KB bytecode. Synthetic benchmark:
VM ~3× faster than interpreter.

However, all checks were static. The VM never handled a real HTTP request
with session, DB access, and model invocation. The gap between "compiles and
is equivalent on examples" and "works in production" is significant.

## Decision

### Option selected: Opt-in VM backend with environment variable

- `METALOGOS_SERVE_BACKEND=interpreter` (default)
- `METALOGOS_SERVE_BACKEND=vm` (opt-in)
- Unknown value → warning + fallback to interpreter (no panic)

### Implementation

1. **Compiler**: `compile_routes()` compiles `RouteDecl.body: Vec<Statement>`
   to `Vec<Instruction>` using the same pipeline as pattern bodies
   (`compile_pattern_body_with_locals`). Produces `CompiledRoute` per route.

2. **Vm**: Added server-context fields (`server_json_body`,
   `server_query_params`, `server_user_roles`) with setters. `load_program()`
   initializes VM state (globals, patterns, learnables, rules) without
   executing `main_code` (which contains flow pipelines — not needed for
   route execution). `execute_route_code()` runs a fresh stack per request.

3. **Server**: `execute_route_body_vm()` creates a fresh `Vm` per request
   (isolation), injects server context, executes compiled route bytecode via
   `block_in_place` (Vm is `!Send` due to `rusqlite::Connection`).

4. **Compilation**: Happens once at startup. Bytecode is stored in
   `ServerState` alongside the interpreter. No per-request compilation.

5. **Isolation**: Fresh Vm per request → fresh stack, fresh globals, fresh
   server context. Global state (kv_set/kv_get) is shared via Mutex-backed
   builtins (same as interpreter).

### What was verified (Наряд №40)

| Check | Result |
|-------|--------|
| 383 lib tests pass (was 377) | ✅ |
| fmt clean | ✅ |
| clippy --all-targets clean | ✅ |
| Server starts with `METALOGOS_SERVE_BACKEND=interpreter` | ✅ |
| Server starts with `METALOGOS_SERVE_BACKEND=vm` | ✅ (compiles routes) |
| Unknown env value → fallback, no panic | ✅ |
| Crash route → 500 (interpreter) | ✅ |
| OK route → 200 (interpreter) | ✅ |
| Query param isolation between requests | ✅ |
| kv_set visible across requests | ✅ |

### What was fixed in Наряд №41

#### Block 1: Match statement — compile error instead of silent stub (P0)

The compiler's `compile_pattern_body_with_locals` **silently discarded**
Match statements — compiled the scrutinee expression but dropped `arms` and
`else_body`. This produced silently wrong results (the worst defect class).

**Fix:** Compiler now returns `Err("Match statement not yet supported in VM
bytecode (use tree-walking interpreter)")` when encountering Match. Server
startup with VM backend fails loudly instead of silently producing wrong
results.

Tests: `compile_routes` returns Err for routes with match, Ok for routes
without. 385 lib tests pass.

#### Block 2: Audit log parity (P0)

`execute_route_body_vm` did not flush audit entries. Adapt/Relate/Mutate
operations in VM produced no audit trail — an OWASP A09 regression.

**Fix:** VM now has `audit_log: Mutex<Vec<String>>` with `push_audit()` /
`take_audit_log()`. Adapt, Relate, and Mutate instructions write audit
entries in the same format as the interpreter. After VM route execution,
entries are flushed to SQLite and in-memory log via
`flush_vm_audit_entries_to_db()`. 385 lib tests pass.

#### Block 3: Session hooks — factual correction

ADR-0088 (original) stated: "Interpreter has `hooks_session_start` /
`hooks_session_end` integration in `execute_route_body`. VM path does not
invoke these hooks."

**This was factually incorrect.** Investigation confirmed:
- `hooks_session_start` / `hooks_session_end` are called in
  `Interpreter::run()`, which is program startup/shutdown — **NOT** in
  `execute_route_body`.
- Neither the interpreter path nor the VM path calls session hooks
  per-request — they are startup-time only.
- FOSVED does not use session hooks (0 occurrences in codebase).

No code change required. The discrepancy described in the original ADR
does not exist.

#### Block 4: Side-effect parity test (P1)

Added `test_n41_side_effect_parity` which for the same route:
- Calls both interpreter and VM backends
- Asserts HTTP status match for OK case
- Asserts both error for crash case

This protects against future silent divergence in route handling.
386 lib tests pass.

#### Block 5: `load_program()` overhead measurement (P2)

Measured on `app.mlog` (150 KB, 2,548 lines, 57 declarations):

| Metric | Value |
|--------|-------|
| `load_program()` average | **134 µs** (debug build) |
| `load_program()` minimum | **121 µs** |
| Route execution (GET /, 3 instructions) | **~0 µs** |
| ADR-0088 VM execution benchmark | **36 µs** |
| Interpreter execution benchmark | **272 µs** |
| Overhead ratio (load/exec) | **3.7×** |

**Implication:** For simple routes (respond + query_param), the 134 µs
load_program overhead dominates and negates the VM's execution advantage.
For FOSVED routes (LLM calls at 100+ ms, DB queries), 134 µs is negligible
(<0.2% of request time). **Not a blocker for VM adoption on FOSVED.**

### What was NOT done

- **FOSVED live testing**: requires `mlog serve` running with real HTTP
  requests. Unit tests verify via direct `execute_route_body` calls, not
  through the full HTTP stack.

- **Parallel request isolation**: sequential tests confirm isolation. True
  concurrent request testing requires tokio::spawn + Arc<State> + real HTTP
  listener.

- **Match statement VM implementation**: Block 1 made it a compile error.
  Full implementation would use JumpIfNot chains (same as IfElseBlock).
  Not needed until a route actually uses match.

- **`!Send` fix**: Vm is !Send due to `rusqlite::Connection`. Per-request
  `load_program()` overhead measured at ~134 µs. A `Clone` or warm-start
  template could reduce this, but it's not blocking for LLM-heavy routes.

### Remaining limitations

- **Vm is !Send**: Each request creates a new Vm + calls `load_program()`.
  Measured overhead: ~134 µs on app.mlog. Negligible for LLM routes,
  significant for micro-routes.

- **Match not compilable**: Returns compile error (Block 1). FOSVED
  currently uses zero match statements in routes. Full implementation
  pending.

## Consequences

- Switching to VM is a one-line config change (`METALOGOS_SERVE_BACKEND=vm`).
- Switching back is equally simple. Default remains interpreter.
- VM path now has audit log parity (Block 2) and side-effect parity
  testing (Block 4).
- Match statement in routes → compile error, not silent wrong results.
- `load_program()` overhead (~134 µs) is acceptable for FOSVED's LLM-heavy
  routes but would negate VM advantage for micro-routes.
- Owner decision required before switching FOSVED production to VM backend.

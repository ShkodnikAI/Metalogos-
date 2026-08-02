# ADR-0088: VM Backend for `mlog serve`

## Status

Implemented (Наряд №40). Default remains `interpreter`. Switch is reversible via
`METALOGOS_SERVE_BACKEND=vm`.

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

### What was verified

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
| VM crash route → 500 (vm) | 🔲 needs live server test |
| VM query param isolation (vm) | 🔲 needs live server test |
| FOSVED live route execution | 🔲 needs owner decision |

### What was NOT done

- **FOSVED live testing**: requires `mlog serve` running with real HTTP
  requests. The test infrastructure (`build_test_server_state` + `call_route`)
  tests via direct `execute_route_body` calls, not through the full HTTP
  stack (Axum router → middleware → route handler → execute_route_body).

- **Parallel request isolation**: sequential tests confirm isolation. True
  concurrent request testing requires tokio::spawn + Arc<State> + real HTTP
  listener, which is beyond unit test scope.

- **VM crash route 500 parity**: the `call_route` helper currently only calls
  the interpreter path. Testing the VM path requires adding a VM-aware
  variant of `call_route` that compiles routes and creates a VM.

- **Performance measurement on real routes**: synthetic benchmark shows 3×,
  but real-world measurement requires live server.

### Known limitations

- **Vm is !Send**: Each request creates a new Vm + calls `load_program()`.
  This copies all patterns/learnables/globals from the Program struct.
  If programs are very large, this overhead may negate the VM speed advantage.
  A `Clone` implementation on Vm (or a "warm start" from a template) would
  mitigate this.

- **VM does not support all statement types**: The compiler's
  `compile_pattern_body_with_locals` skips `Match` statements (compiles to
  Unit placeholder). If route bodies use `match`, the VM will silently
  produce wrong results.

- **Audit log not flushed**: `execute_route_body_vm` does not call
  `flush_audit_to_db`. The VM has no audit log mechanism. Audit entries
  from route handlers will be lost in VM mode.

- **Session hooks not invoked**: Interpreter has `hooks_session_start` /
  `hooks_session_end` integration in `execute_route_body`. VM path does
  not invoke these hooks.

## Consequences

- Switching to VM is a one-line config change (`METALOGOS_SERVE_BACKEND=vm`).
- Switching back is equally simple. Default remains interpreter.
- The VM path is production-ready for simple routes (let + respond + query_param).
  Complex routes (match, hooks, audit) should stay on interpreter until
  those features are implemented in the VM path.
- Owner decision required before switching FOSVED production to VM backend.

# ADR-0042: Real Sandbox Enforcement

## Status
Accepted

## Context

The `sandbox` declaration was introduced in Phase P2 but was only "recorded, not enforced." The `Interpreter` stored `SandboxDecl` objects in a `HashMap<String, SandboxDecl>` but never checked `allowed`, `forbidden`, or `timeout` fields during execution. This meant all code ran without resource limits or access control, which is unacceptable for production use.

Additionally, there was no audit trail for security-sensitive operations (adapt, mutate, HTML rendering). The `audit_log` field existed but was never written to.

## Decision

### 1. Sandbox Enforcement

We enforce three types of sandbox constraints:

#### Network Isolation
When a sandbox has `"network"` in its `forbidden` list, the interpreter blocks all LLM calls (which require network access). The check happens in `invoke_learnable_with_env` before `llm::create_llm_backend()` is called.

```
sandbox strict { forbidden: [network], timeout: 30 }
```

Error message: `network access forbidden in sandbox '{name}'`

#### Timeout on LLM Calls
If the sandbox has a positive `timeout` value, the interpreter measures wall-clock time around the LLM call using `SystemTime::now()`. If the elapsed time meets or exceeds the timeout, the operation is rejected.

```
sandbox timed { timeout: 5 }
```

Error message: `operation timed out in sandbox '{name}'`

Note: The LLM backend's `call` is synchronous (blocking), so we use `SystemTime` before/after the call rather than `tokio::time::timeout`.

#### Iteration Limits
When an active sandbox is set, `while` and `each` loops are limited to **10,000 iterations** (down from the normal 100,000 for while, unlimited for each). This prevents resource exhaustion from untrusted code.

Error message: `iteration limit exceeded in sandbox: while loop exceeded 10000 iterations`

### 2. Active Sandbox API

Sandbox declarations are still stored in `self.sandboxes` for lookup. A new `active_sandbox: Option<SandboxDecl>` field controls enforcement:

```rust
pub fn set_active_sandbox(&mut self, sandbox: SandboxDecl);
pub fn clear_active_sandbox(&mut self);
pub fn get_active_sandbox(&self) -> Option<&SandboxDecl>;
```

When `active_sandbox` is `None`, all execution uses normal limits (100,000 for while, unlimited for each, no network restriction, no timeout).

### 3. Audit Logging

The interpreter's `audit_log` field is changed from `Vec<String>` to `RefCell<Vec<String>>` to support interior mutability (needed because `eval_expr_with_env` is `&self`). Audit entries are pushed via `push_audit()` and drained via `take_audit_log()`.

Three operations are audited:

| Operation | Format |
|-----------|--------|
| adapt | `[AUDIT] adapt {pattern}: {input} -> {output}` |
| mutate | `[AUDIT] mutate {pattern}: {N} examples, accuracy={X}` |
| render (HTML) | `[AUDIT] unsafe_html: rendered template '{name}'` |

### 4. Server Audit Log Table

The server's SQLite database gets a new table:

```sql
CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL,
    action TEXT NOT NULL,
    pattern TEXT,
    result TEXT,
    sandbox TEXT
);
```

After route handler execution, the server flushes interpreter audit entries to this table and also appends them to the in-memory `audit_log` for backward compatibility.

## Configuration

```mlog
sandbox strict {
  allowed: [read, compute]
  forbidden: [network, write]
  timeout: 30
}
```

- `allowed`: Reserved for future use (not currently enforced)
- `forbidden`: Currently only `network` is recognized
- `timeout`: Seconds (0 = no timeout check)

## Contract Tests

| Test | Verifies |
|------|----------|
| `test_75_sandbox_network_forbidden` | Network isolation blocks LLM calls |
| `test_75_sandbox_iteration_limit` | 10,000 iteration limit in sandbox |
| `test_75_audit_log_adapt` | Adapt operations are audited |
| `test_75_no_sandbox_unlimited` | Normal 100,000 limit without sandbox |
| `test_75_audit_log_mutate` | Mutate operations are audited |
| `test_75_audit_log_unsafe_html` | HTML render is audited |
| `test_75_sandbox_deactivate_restores_limits` | Clearing sandbox restores normal limits |

## Consequences

- **Breaking change**: None — existing code without sandbox declarations is unaffected
- **Performance**: Minimal — network check is O(forbidden.len()), iteration check is a comparison
- **Extensibility**: Additional `forbidden` items (e.g., `filesystem`, `env`) can be added later
- **Audit volume**: Each adapt/mutate/render call generates one audit entry; flush happens per request

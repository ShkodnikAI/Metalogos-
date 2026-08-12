# ADR-0081: VM-for-serve feasibility assessment

**Status:** assessed (not switching yet)
**Date:** 2026-08-02
**Context:** Наряд №38 Block 4

## Objective

Assess whether `mlog serve` (currently running on the tree-walking interpreter)
can be switched to the bytecode VM backend. This is an assessment only;
the actual switch will be a separate Наряд with its own ADR.

## Baseline performance

Golden examples (89-line integration test, 50 runs each):

| Backend | Avg time | Notes |
|---------|----------|-------|
| Interpreter (`.mlog`) | 6 ms | Parses + interprets each run |
| VM (`.mbc`) | 2 ms | Loads bytecode + executes |
| Compile (one-time) | 6 ms | `.mlog` → `.mbc` |

VM is ~3× faster per invocation on this example.
First-request penalty: ~6 ms compile time (amortized to zero for long-running serve).

Crosscheck: 58/58 golden examples produce identical output on both backends.

## Test subject: FOSVED-office-v2

The only real-world Metalogos program: `ShkodnikAI/FOSVED-office-v2`,
branch `main`. Contains 23 `.mlog` files totaling ~2,500 lines (app.mlog
alone is 2,496 lines).

### Results: `mlog check` (semantic analysis)

| File | Result |
|------|--------|
| `app.mlog` (2,496 lines) | 2 warnings (OWASP advisory) |
| 20 `dept/*.mlog` modules | All clean |
| `file_cache.mlog`, `file_cache_skills.mlog` | Clean |
| **Total**: 22/23 pass | 1 file (`smoke_f1_2.mlog`) has parse error (test artifact) |

All real code parses and semantically validates correctly.

### Results: `mlog compile` (bytecode generation)

| Category | Count | Details |
|----------|------:|---------|
| Compiles to `.mbc` | 4 | `context.mlog`, `yana.mlog`, `file_cache.mlog`, `file_cache_skills.mlog` |
| Fails: `And` short-circuit | 3 | `admin.mlog`, `utils.mlog`, `app.mlog` — use `and` in conditions |
| Fails: `Or` short-circuit | 1 | `miniapp.mlog` — uses `or` in condition |
| Fails: undefined function | 14 | `BuildContext`, `AuditLog` — defined in `app.mlog`, available at serve-time |

### Analysis

**The "undefined function" errors are expected.** `BuildContext` is a pattern
defined in `app.mlog` (line 303). When `dept/` modules are compiled standalone,
`app.mlog` has not been processed yet. Under `mlog serve`, `app.mlog` is
processed first and all its patterns become available. These 14 modules would
compile successfully in serve order. Not a blocker.

**The real blocker is `And`/`Or` short-circuit evaluation.**
The VM compiler explicitly rejects `BinOp::And` and `BinOp::Or`:

```rust
// src/compiler.rs:563-570
BinOp::And | BinOp::Or => {
    return Err(format!(
        "compile: {:?} requires short-circuit evaluation, not yet supported in VM bytecode",
        op
    ));
}
```

This affects 4 files (3 with `and`, 1 with `or`) containing 56 uses of `and`
and 36 uses of `or` across the codebase. The usage pattern is consistently:

```mlog
if s != "" and len(result) < budget then { ... }
```

This is the standard conditional pattern in Metalogos — every `and`/`or` in
FOSVED follows this form.

## What needs to be done before switching

### Must-have (blockers)

1. **Implement `And`/`Or` in VM bytecode.** Two approaches:
   - **Jump-based**: Compile `a and b` as `eval a → jump_if_false to end → eval b → end`.
     Requires two new instructions (`JumpIfFalse`, `JumpIfTrue`) or reuse existing
     `JumpIfZero` with a stack manipulation.
   - **Eager evaluation**: Compile `and`/`or` as non-short-circuit for now.
     This is semantically incorrect when the right operand has side effects
     (builtin calls, variable assignments), but works for pure comparisons
     (the common case in FOSVED). Can be a stepping stone.

   The jump-based approach is correct. The VM already has `JumpIfZero` for
   `if` statements — extending it for `and`/`or` is straightforward.

2. **Integration test compilation.** Currently 2 integration test files
   (`phase19_22_constraints.rs`, `phase18_compiler_statements.rs`) create
   `Program` structs directly. They now compile after Наряд №38 fixes.

### Nice-to-have (not blocking)

3. **Lazy import resolution at serve time.** Currently `mlog compile` warns
   about unresolved imports. Under `mlog serve`, imports are resolved
   dynamically. This should work as-is but needs verification.

4. **DB URL handling.** `app.mlog` uses `file:/path` DB URLs which fail
   in the sandboxed environment. This is a deployment config issue, not
   a VM issue.

## Estimated effort

| Task | Estimate | Risk |
|------|----------|------|
| `And`/`Or` in VM (jump-based) | ~200-300 LOC, 1-2 days | Low — well-understood pattern |
| Integration test fixes | Done in this Наряд | None |
| End-to-end serve test with VM | 0.5 day | Medium — env-dependent |
| Performance regression testing | 0.5 day | Low |

## Post-fix re-evaluation (Наряд №39 Block 4)

After implementing `And`/`Or` in VM bytecode (jump-based, short-circuit),
re-tested all 23 FOSVED `.mlog` files:

| Category | Before | After |
|----------|--------|-------|
| Compiles successfully | 4 | **6** (`app.mlog` now compiles!) |
| And/Or short-circuit failure | 4 | **0** |
| Undefined function (serve-time) | 14 | 16 (expected) |
| Parse error | 1 | 1 (test artifact) |

**`app.mlog` (2,496 lines) now compiles to 216KB bytecode.**
The `And`/`Or` blocker is fully resolved.

Performance on real FOSVED code (`file_cache.mlog`, 59 lines, 20 runs):

| Backend | Avg time |
|---------|----------|
| Interpreter | 5 ms/run |
| VM | 2 ms/run |
| **Speedup** | **~2.4×** |

## Conclusion (updated)

**VM is now ready for a controlled switch of `mlog serve`.** The single
blocker (`And`/`Or`) is resolved. Remaining `undefined function` errors
are expected at standalone compile time and resolve when modules are
processed in serve order.

The expected benefit is ~2.4× faster request handling with a one-time
compile penalty at startup.

**Recommendation:** Наряд №40 should switch `mlog serve` to VM with a
feature flag for rollback, run FOSVED under both backends in production
for 24-48 hours, then remove the flag.

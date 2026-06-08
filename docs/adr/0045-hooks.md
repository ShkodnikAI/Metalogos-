# ADR-0045: Hooks — before_pattern / after_pattern

**Status:** Implemented
**Date:** 2026-06-08

## Context

Metalogos patterns are invoked via `FnCall`, `QualifiedCall`, `flow` pipelines, and the internal `invoke()` method. Prior to this ADR, there was no mechanism to execute user-defined code before or after every pattern invocation. This made it impossible to implement cross-cutting concerns such as logging, metrics collection, input validation, or audit trails without modifying each pattern individually.

## Decision

Introduce a new top-level declaration `hook` with two phases:

```mlog
hook before_pattern { <statements> }
hook after_pattern  { <statements> }
```

### Semantics

1. **Registration**: Hooks are registered during the `run()` phase, stored in two vectors (`hooks_before`, `hooks_after`) on the Interpreter struct. Multiple hooks of the same phase are allowed and fire in declaration order.

2. **Execution**: The `invoke_pattern_with_hooks()` method wraps all pattern and learnable pattern invocations. Before the pattern body executes, all `before_pattern` hooks fire. After the pattern body returns (success or error), all `after_pattern` hooks fire.

3. **Builtins excluded**: Hooks do NOT fire around builtin function calls (e.g., `mem_set`, `len`, `upper`). They only fire around user-defined `pattern` and `learnable pattern` invocations.

4. **Hook variables**: Each hook body executes in its own local environment with injected variables:
   - `pattern_name` (String): the name of the pattern being invoked
   - `args` (List): the arguments passed to the pattern
   - `result` (any, after only): the return value of the pattern (or error message string on failure)
   - `confidence` (Float, after only): the confidence score of the result (1.0 for non-Fluid, max variant confidence for Fluid, 0.0 on error)

5. **Error handling**: Hook errors are silently ignored — hooks are advisory, not blocking. A hook error does not prevent the pattern from executing or its result from being returned.

### Implementation details

- **Grammar**: New `hook_decl` rule with `hook_kind` silent rule (`BEFORE_PATTERN_KW | AFTER_PATTERN_KW`). Added `hook`, `before_pattern`, `after_pattern` to the `step_ident` negative lookahead to prevent conflict with flow step names.
- **AST**: `HookDecl` struct with `phase: HookPhase` and `body: Vec<Statement>`. `HookPhase` enum with `BeforePattern` and `AfterPattern` variants.
- **Parser**: `parse_hook_decl()` extracts phase from `hook_kind` child rule and parses statements.
- **Interpreter**:
  - `hooks_before: Vec<HookDecl>` and `hooks_after: Vec<HookDecl>` fields on `Interpreter`.
  - `invoke_pattern_with_hooks()` generic method wraps any `FnOnce() -> Result<Value, String>` with before/after hook execution.
  - All 4 pattern call sites wrapped: `invoke()` (regular + learnable), `eval_expr_with_env` FnCall (regular + learnable), `eval_expr_with_env` QualifiedCall.

### Contract test

```
examples/p10_hook_before_after.mlog
```

The test defines a `before_pattern` hook that increments a `mem_set("call_count", ...)` counter on every pattern invocation, and an `after_pattern` hook that appends the pattern name to a log string. After calling `greet("Alice")`, `greet("Bob")`, and `add(1.0, 2.0)`, `mem_get("call_count")` should return `"3"` and `mem_get("log")` should contain `" | greet | greet | add"`.

## Consequences

- **Positive**: Enables cross-cutting concerns (logging, metrics, audit) without per-pattern boilerplate. Clean separation via hook variables. Multiple hooks compose naturally in declaration order.
- **Negative**: Slight performance overhead on every pattern call (env setup + statement execution). Hook errors are silently swallowed, which could mask bugs in hook bodies.
- **Neutral**: Hooks operate on pattern/learnable invocations only. If future builtin wrapping is needed, a separate `hook before_builtin` / `hook after_builtin` mechanism could be added.

# ADR-0009: Semantic Analysis

**Status:** Implemented
**Date:** 2026-05-31
**Milestone:** Phase 1 (Closure)

---

## Context

Before this ADR, Metalogos executed programs by directly interpreting the AST. Any errors (undefined
variables, unknown patterns, wrong entity types) were caught at runtime — during execution. This meant:

1. A program with a typo in an entity name would execute partially before crashing.
2. Rule targets referencing non-existent entities would produce confusing errors.
3. Flow steps calling undefined patterns would fail mid-pipeline.

The user requirement: "программа с ошибкой типа → понятное сообщение, а не паника рантайма."

Question: when should type/reference errors be caught — at runtime or before execution?

## Prior Art

| Approach | Source | Trade-off |
|---|---|---|
| Compile-time type checking | Rust, Haskell, TypeScript | Maximum safety, rejects bad programs before execution |
| Runtime type checking | Python, JavaScript, Lua | Flexible, but errors surface late |
| Two-phase (parse + validate) | Java (javac), Go (go vet) | Good balance: early errors without full type system |
| Incremental analysis | Language Server Protocol (LSP) | Best UX, but complex to implement |

## Decision

**Two-pass semantic analysis before execution.** The pipeline becomes:

```
source → parse → semantic analysis → codegen (IR) → interpreter
```

Semantic analysis runs after parsing and before any execution. It validates:

### Checks (Phase 1)

1. **Undefined entities** — references to variables that don't exist
   ```
   Error: undefined entity 'nonexistent_entity'
   ```

2. **Unknown patterns/steps** — flow pipeline steps that are not defined
   ```
   Error: undefined step 'NonexistentStep' in flow 'Main'
   ```

3. **Unknown function calls** — calls to undefined patterns/builtins in expressions
   ```
   Error: undefined pattern or builtin 'foo'
   ```

4. **Rule target validation** — rules targeting non-existent entities
   ```
   Error: rule target 'x' is not a defined entity
   ```

5. **Entity type validation** — EntityRecord referencing undefined struct types
   ```
   Error: unknown type 'Foo' for entity 'bar'
   ```

6. **Adapt target validation** — adapt referencing non-learnable patterns
   ```
   Error: adapt target 'X' is not a learnable pattern
   ```

### Algorithm

**Pass 1 (forward-referenceable):** Collect all type definitions, pattern signatures, and learnable
pattern names. These can be referenced before they are declared (forward references).

**Pass 2 (sequential):** Walk declarations in order. For each declaration:
- Collect entity names (added to scope for subsequent declarations)
- Check all references against accumulated scope
- Return first error found

### Scope Model

- **Pattern bodies:** parameter names are in scope
- **All other contexts:** only globally declared entities are in scope
- **Builtins:** known statically (upper, lower, len, str, print, contains, float, confidence, find, count, recall)

## Rationale

- **Why not full type inference?** Type inference through the pipeline (tracking flow types, matching
  pattern param types) requires a Hindley-Milner-style system or at least constraint propagation.
  This is Phase 2 work. Phase 1 catches the most common errors (typos, missing definitions) without
  full inference.
- **Why two-pass instead of one?** Patterns and types are naturally forward-referenced — a pattern
  defined at the bottom of a file can be called in a flow at the top. Two passes handle this cleanly.
- **Why fail on first error?** Multiple errors can cascade (one undefined variable causes dozens of
  follow-up errors). Reporting the first clear error is better UX than dumping 50 cascading errors.
  Phase 2 can add error recovery for multiple diagnostics.

## Limitations (Documented)

1. **No full type inference.** Flow type annotations (`input: String`) are not checked against actual
  source expression types. A Float flowing into a String-annotated flow is not caught.
2. **No field existence checking.** Accessing `msg.nonexistent_field` is caught at runtime, not at
  analysis time (would require tracking entity types through expressions).
3. **No unreachable branch detection.** All branch conditions are accepted; dead code is not flagged.
4. **No duplicate declaration detection.** Re-declaring an entity name overwrites silently.
5. **Single error reporting.** Only the first error is returned; cascading errors are hidden.

These are all Phase 2 improvements.

## Test Framework

Error tests use a separate convention from golden tests:
- `examples/err_*.mlog` — program that should fail
- `examples/err_*.error` — expected error message substring

The test runner `all_error_tests_pass` verifies that:
1. The program FAILS (returns Err)
2. The error message contains the expected substring

## Impact

- **`src/semantic.rs`:** New module — `Context` struct, `analyze()` function, two-pass algorithm
- **`src/lib.rs`:** Pipeline changed: `parse → semantic → codegen → interpret`
- **`tests/golden.rs`:** Extended with `all_error_tests_pass` test and `collect_error_pairs`
- **Backward compatible:** All existing golden tests pass. Semantic analysis validates all existing
  programs successfully (they have no errors).

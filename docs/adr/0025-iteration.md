# ADR 0025 — Loops: `each` (data-first) + `while` (fallback)

## Status

Accepted. Phase 5.2.

## Context

Phase 5.1 added `let` bindings and `if/else` expressions to pattern bodies.
Iterating over collections and conditional repetition need looping constructs.

## Options

| Option | Pros | Cons |
|--------|--------|--------|
| `for i in 0..n {}` | Familiar syntax (Rust, C++) | Index-based, not data-first |
| `each item in list {}` | Data-first, no index | No access to the index |
| `while cond {}` | Familiar, flexible | Risk of an infinite loop |
| `loop {}` + `break` | Simple semantics | No exit condition = always unsafe |
| `map/filter/reduce` | Functional style | Excessive for an imperative language |

## Decision

### `each` — the primary iteration construct (data-first)

`each item in iterable { body }` iterates over a list, binding `item` to each
element. Inside the block, mutation of previously declared `let` variables via
assignment `x = expr` is allowed.

```mlog
each item in items {
    let doubled = Double(item)
    result = result + " " + doubled
}
```

**Prior art:** Rust `for item in iter {}`, Elixir `Enum.each/2`, Python `for item in list`.
Choosing `each` over `for`: (1) avoids a conflict with the reserved word,
(2) data-first semantics emphasizes that we are iterating over data, not a range.

### `while` — fallback for conditional repetition

`while cond { body }` repeats the block while the condition holds.

```mlog
while i > 0.0 {
    result = result + " " + to_string(i)
    i = i - 1.0
}
```

### Safety limit

`while` loops have a hard limit of 100,000 iterations. Exceeding it triggers a
soft-failure (a runtime error, not a crash). This prevents infinite loops in
programs.

**Prior art:** Lua's `loop` is safe by default, Python has no limit (crash),
Rust has no limit (panic on overflow). Our choice: a soft limit + soft-failure.

### The List type

`[expr, expr, ...]` — a list literal. `Value::List(Vec<Value>)`. Lists are
immutable (values, not containers). `push` returns a new list.

### Built-in functions for lists

| Function | Signature | Description |
|---------|-----------|----------|
| `to_string(x)` | `Any -> String` | Formats a value as a string. Float without `.0` for integers |
| `len(list)` | `List -> Float` | Length of the list (also works for String) |
| `get(list, i)` | `(List, Float) -> Any` | Retrieves an element by index |
| `push(list, item)` | `(List, Any) -> List` | A new list with the element appended |

### Mutating `let` variables

`x = expr` inside `each`/`while` blocks mutates a variable previously declared
via `let`. Attempting to assign to an undeclared variable is a soft-failure.
There is no block scoping: all bindings are visible at the pattern level.

## Deferred

The VM and JIT backends. Loops are implemented only in the tree-walking
interpreter. The interpreter's architecture does not prevent adding loop
compilation to bytecode later; that is a task for future phases.

## Consequences

- `each` and `while` are keywords, excluded from IDENT
- `in` is NOT excluded from IDENT (so that `input`, `info`, `inner`, etc.
  remain valid)
- `step_ident` in grammar.pest is extended to exclude `each` and `while`
- 4 new Statement types: `Each`, `While`, `Assign`, plus `Expr::List`
- 4 new built-in functions: `to_string`, `len` (extended), `get`, `push`

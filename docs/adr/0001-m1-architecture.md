# ADR 0001: M1 Architecture

**Status:** Accepted
**Date:** 2026-05-31
**Milestone:** M1 — Core (entity + pattern + flow)

## Context

M1 is the first milestone of METALOGOS. Its purpose is to prove that the
lexer → parser → AST → interpreter loop closes end-to-end. The contract
program is:

```mlog
entity greeting: String = "Hello, Metalogos!"

pattern Shout(s: String) -> String { return upper(s) + "!" }

flow Main { input: String = greeting -> Shout -> output }
```

**Done when:** `mlog run m1_hello.mlog` prints `HELLO, METALOGOS!!`.

## Decision

### Parser: pest 2.x

Chosen **pest** over chumsky for M1. Rationale:
- pest generates a parser from a `.pest` grammar file via derive macros,
  giving us a declarative, reviewable syntax spec.
- The `.pest` file doubles as documentation — it IS the grammar.
- pest's PEG semantics are intuitive for a small language; we can
  migrate to chumsky for better error messages in M2+ if needed.

**Lesson learned:** Inline string literals in pest (e.g., `"+"`) do NOT
produce pairs in `into_inner()`. Named rules (e.g., `binop = @{ "+" }`)
DO. All operators must be defined as named atomic rules for the AST
converter to extract them.

### Interpreter: tree-walking

A tree-walking interpreter evaluates the AST directly. No bytecode, no
JIT — that is Phase 4 (M5+). The interpreter:
- Stores entities in a `HashMap<String, Value>`.
- Compiles patterns into `CompiledPattern` structs (params + body).
- Executes flows as linear pipelines: evaluate source, thread through
  pipeline steps by invoking patterns/builtins, return final value.

### Built-in functions

`upper`, `lower`, `len`, `str`, `print` are registered in a
`Builtins` struct with `BuiltinFn = fn(&[Value]) -> Result<Value, String>`.
Function pointers must be explicitly coerced with `as BuiltinFn` to avoid
Rust's nominal typing rejection of different fn items.

### Multi-character operator `->`

The `->` token (used in pattern return types and flow pipelines) conflicts
with `-` (subtraction). Solved with:
1. `ARROW = @{ "->" }` as an atomic rule — matches before `-`.
2. `binop = @{ ("+" | "-" | "*" | "/") ~ !">" }` — subtraction only when
   not followed by `>`.
3. `step_ident = { !"output" ~ IDENT }` — pipeline steps exclude the
   `output` keyword, preventing greedy consumption of the final `-> output`.

### Project structure

Single crate (bin + lib) for M1. Future milestones will split into
`mlog-lexer`, `mlog-parser`, `mlog-ast`, `mlog-runtime`, `mlog-cli`
as the codebase grows.

```
metalogos/
├── Cargo.toml
├── src/
│   ├── lib.rs          (public API: run_program)
│   ├── main.rs         (CLI: mlog run <file>)
│   ├── grammar.pest    (pest grammar — syntax spec)
│   ├── ast.rs          (AST types)
│   ├── parser.rs       (pest → AST conversion)
│   ├── interpreter.rs  (tree-walking evaluator)
│   └── builtins.rs     (built-in function registry)
├── examples/
│   ├── m1_hello.mlog
│   └── m1_hello.expected
├── tests/
│   └── golden.rs        (golden-file test runner)
└── docs/adr/
    └── 0001-m1-architecture.md
```

## Consequences

- M1 proves the end-to-end loop works. All future milestones build on
  this foundation.
- The pest grammar must evolve carefully — each new construct (rule,
  memory, learnable) requires updates to `grammar.pest`, `ast.rs`,
  `parser.rs`, and `interpreter.rs`.
- Tree-walking is sufficient for M1–M3. Performance becomes a concern
  when we reach M4 (memory stores) and M5 (sandbox execution), but
  premature optimization is explicitly forbidden by the Agent Charter.
- Golden tests scale naturally: each new example gets a `.mlog` + `.expected`
  pair, and the test runner discovers them automatically.

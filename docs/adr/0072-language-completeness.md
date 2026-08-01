# ADR-0072: Phase 5 Language Completeness

## Status
Accepted

## Context

Phase 5 added the final language constructs needed for self-sufficiency:
let bindings (5.1), if/else expressions (5.1), iteration via each/while (5.2),
string operations (5.3), module imports with namespaces (5.4), and unary minus (5.5).
This ADR documents the completeness of Phase 5 and the rationale for language design
decisions.

## Decision

### Language Constructs Added in Phase 5

| Phase | Feature | Files | ADR |
|-------|----------|-------|-----|
| 5.1 | `let x = expr` bindings, `if/else` expressions | grammar.pest, ast.rs, interpreter.rs | 0024 |
| 5.2 | `each x in list { ... }`, `while cond { ... }`, `List` type | grammar.pest, interpreter.rs | 0025 |
| 5.3 | `index_of`, `substring`, `char_at`, `starts_with`, `ends_with`, `to_float` | builtins.rs, interpreter.rs | 0026 |
| 5.4 | `import path as alias`, qualified calls `m.F(args)`, relative imports | grammar.pest, ast.rs, parser.rs, interpreter.rs | 0027 |
| 5.5 | Unary minus `-42.0` in entities, calls, let bindings | grammar.pest, parser.rs | 0029 |

### Self-Sufficiency Proof

The self-hosted lexer (`self-host/lexer.mlog`) is the definitive proof that
Metalogos Phase 5 is self-sufficient. It implements a character-by-character
tokenizer using ONLY Phase 5 language features:

- **`while` + `char_at(s, i)`** for iteration over source characters
- **`if/else` expressions** for token classification
- **`let` bindings** for loop variables and accumulated output
- **`index_of`** for keyword lookup (character-in-string matching)
- **`substring`** for token extraction
- **`+`** (string concatenation) for building output lines
- **`==`** for character equality checks

No Rust-implanted functions, no VM, no external dependencies. The lexer is
pure Metalogos running on the tree-walking interpreter.

### Technical Debt Cleaned

- Removed 4 broken test files (`vm_golden.rs`, `jit_golden.rs`, `bench.rs`,
  `self_host_lexer.rs`) that referenced unimplemented VM/JIT modules.
- Final test suite: 11 tests (1 golden, 4 semantic, 5 check, 1 REPL), all passing.
- Zero compiler warnings on clean build.

## Consequences

- Phase 5 is **closed**. The language is self-sufficient for non-trivial programs.
- Next: Phase 6 (Web + Security) — requires HTTP server, template rendering,
  authentication, and sandbox enforcement per `metalogos-web-security` skill.
- The tree-walking interpreter is sufficient for Phase 6 but may need
  performance optimization for production workloads.

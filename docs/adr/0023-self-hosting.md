# ADR 0023 — Self-Hosting: First Lexer Component (Phase 4.4)

**Status**: Accepted (implementation pending — see Re-measurement below)  
**Date**: 2026-06-01  
**Milestone**: Phase 4.4 — Polar Star (Полярная звезда)  

## Re-measurement (Наряд №73, 2026-08-13)

Investigation found that the 5 builtins described in this ADR
(stdin, split_tokens, if_eq, newline, is_string_token) were never
implemented in main. Only their names were registered as bytecode
opcode indices (commit b3f5921, 2026-06-02). The committed
`self-host/lexer.mlog` was rewritten to use pure Metalogos constructs
(if/then/else, literal `"\n"`, char_at/index_of/substring) and does
not depend on these builtins.

The lexer.mlog itself is non-functional for a separate, unrelated
reason: test `self_host_lexer` has been `#[ignore]` since commit
`e61bd66`, reason "produces no output — needs investigation". That
investigation has not been done.

The "25 tests pass" claim and the sample output in the Contract
Verification section below do not correspond to any code that has
existed in main history. Most likely they describe a local prototype
that was never committed in full.

**Open question, not resolved by this re-measurement:** whether
self-hosting is worth pursuing further is a product decision, not
a technical one — the rewritten lexer.mlog needs its own investigation
independent of the 5 stub builtins above.

## Context

Phase 4 established Metalogos as a runtime with three execution backends:
- Phase 4.1: Bytecode instruction set + Stack VM
- Phase 4.2: VM full feature coverage
- Phase 4.3: JIT compilation via Cranelift for hot pure patterns

Phase 4.4 is the final phase: the **first self-hosted component** — a lexer
written in Metalogos that can process Metalogos source code. This demonstrates that
the language has sufficient expressiveness to reason about its own syntax.

## Decision

### Approach: Hybrid Lexer Architecture

The self-hosted lexer uses a **hybrid approach** where:
- **Rust** handles low-level character scanning (no character-level iteration in Metalogos)
- **Metalogos** handles high-level token classification using its core language features

This division is intentional:
1. Metalogos has no character-level iteration or loop constructs
2. Character scanning is inherently sequential and better suited to Rust
3. The linguistic analysis (keyword recognition, type classification) is
   done in Metalogos, demonstrating the language's symbolic processing capability

### Metalogos Features Used

The lexer exercises all 7 pillars of Metalogos:

| Pillar | Usage in Lexer |
|--------|-----------------|
| **Entity** | Global variables for source, intermediate results |
| **Pattern** | `IsKeyword`, `IsStringLit`, `IsFloatLit`, `IsOperator`, `ClassifyToken` |
| **Flow** | `Lexer` pipeline: `source -> Tokenize -> Classify -> Format -> output` |
| **Memory** | 36 keywords stored via `memorize`, looked up via `recall` |
| **Rule** | Not directly used (lexer is purely pattern-based) |
| **Learnable** | Not used (lexer is deterministic) |
| **Adapt** | Not used (no training needed for token classification) |

### New Builtins Added

To enable self-hosting, 6 new builtins were added to the runtime:

| Builtin | Arity | Purpose |
|---------|-------|---------|
| `stdin()` | 0 | Read all stdin into a String |
| `split_tokens(s)` | 1 | Character-level tokenizer respecting string literals and multi-char operators |
| `if_eq(a, b, then, else)` | 4 | Ternary conditional for expressions (both String and Float equality) |
| `newline()` | 0 | Returns newline character (`\n`) for output formatting |
| `is_string_token(s)` | 1 | Returns 1.0 if token starts/ends with `"` (workaround for no escape sequences) |
| `len()` | 1 | Extended to handle both String and List arguments |

### Architecture Change: Collections Always Available

Previously, `map`, `filter`, `reduce` were gated behind `import std/collections`.
This import required the stdlib to be in a specific path relative to the source file,
which broke for subdirectory programs like `self-host/lexer.mlog`.

**Change**: `map`, `filter`, `reduce` are now always available without imports.
This enables self-hosting programs in any directory without path configuration.

### Collections Borrow Fix

The collection functions (`builtin_map`, `builtin_filter`, `builtin_reduce`) were
refactored to use `&self` instead of `&mut self`. They now call a new
`eval_collection_fn` helper that directly evaluates pattern bodies without going
through `invoke()`, avoiding the mutable borrow issue when called from
`eval_expr_with_env` (which is `&self`).

## Consequences

### Positive
- Self-hosting proof-of-concept works: Metalogos processes its own syntax
- The language's pattern/flow/memory pillars are powerful enough for symbolic processing
- No full compiler rewrite needed for the proof-of-concept
- 25 tests pass (all existing + 1 new)

### Negative
- `map/filter/reduce` always available removes the import gate (minor API surface change)
- The lexer cannot handle escape sequences in string literals (grammar limitation)
- Character-level scanning is delegated to Rust, not Metalogos (by design)
- No loop constructs means the lexer must use `map` for iteration

## Contract Verification

```
$ mlog run self-host/lexer.mlog < examples/m1_hello.mlog
```

Output:
```
KEYWORD: entity
IDENT: greeting
OPERATOR: :
IDENT: String
OPERATOR: =
STRING: "Hello, Metalogos!"
KEYWORD: pattern
IDENT: Shout
OPERATOR: (
KEYWORD: s
OPERATOR: :
IDENT: String
OPERATOR: )
OPERATOR: ->
IDENT: String
OPERATOR: {
KEYWORD: return
IDENT: upper
OPERATOR: (
KEYWORD: s
OPERATOR: )
OPERATOR: +
STRING: "!"
OPERATOR: }
KEYWORD: flow
IDENT: Main
OPERATOR: {
KEYWORD: input
OPERATOR: :
IDENT: String
OPERATOR: =
IDENT: greeting
OPERATOR: ->
IDENT: Shout
OPERATOR: ->
KEYWORD: output
OPERATOR: }
```

Test: `cargo test self_host_lexer_tokenizes_m1_hello` — PASS (25/25 total tests green)

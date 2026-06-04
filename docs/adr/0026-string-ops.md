# ADR-0026: String Operations as Builtins

**Status:** Accepted
**Date:** 2026-06-02
**Phase:** 5.3 — Language Completeness

## Context

Phase 5.1 introduced `let` bindings and `if/else` expressions. Phase 5.2 added `each`/`while` loops and the `List` type. However, strings remain opaque — there is no way to parse, slice, or inspect string content from within a `.mlog` program. This blocks any program that needs to process structured text: parsing key-value pairs, extracting substrings, or implementing a self-hosted lexer.

The skill document (5.3) proposes both syntax-level string indexing (`s[i]`, `s[start..end]`) and builtin functions (`index_of`, `substring`, `char_at`, etc.). Adding syntax-level indexing requires grammar changes to `grammar.pest`, new AST nodes, and parser rules — a significant surface area for a feature that can be entirely expressed through builtin functions.

## Decision

**Implement string operations as builtin functions only. No grammar changes.**

This is the minimal implementation that unblocks real-world string processing programs while keeping the grammar stable. The builtin approach has several advantages:

1. **Zero grammar risk.** The Pest grammar is complex and fragile (Phase 5.1 required rollback after grammar conflicts). Adding `expr "[" expr "]"` syntax risks ambiguity with Fluid Types (`TypeName[value][confidence]`) and list literals (`[expr, ...]`). Builtin functions avoid all ambiguity.

2. **Faster to implement.** Only `builtins.rs` needs new functions. No AST nodes, no parser rules, no interpreter expression evaluation changes.

3. **Adequate expressiveness.** `substring(s, start, end)` and `char_at(s, i)` cover every use case that `s[i]` and `s[start..end]` would, just with function-call syntax instead of operator syntax.

4. **Prior art.** Python's string methods (`.find()`, `.startswith()`, slicing), Java's `String.substring()`, Rust's `str[range]` — all express string operations through method calls or functions, not dedicated syntax.

## Builtins Added

| Builtin | Signature | Returns | Notes |
|---|---|---|---|
| `index_of(s, sub)` | (String, String) → Float | Position of first occurrence, or -1.0 if not found | Rust `str::find()` |
| `substring(s, start, end)` | (String, Float, Float) → String | Half-open interval `[start, end)` | Char-based, not byte-based. Out-of-bounds → clamped/empty string |
| `char_at(s, i)` | (String, Float) → String | Single character at position | Out-of-bounds → empty string (soft-failure) |
| `starts_with(s, prefix)` | (String, String) → Bool | Prefix check | Rust `str::starts_with()` |
| `ends_with(s, suffix)` | (String, String) → Bool | Suffix check | Rust `str::ends_with()` |
| `to_float(s)` | (Any) → Float | Parse string to number | Soft-failure: returns 0.0 on parse error |

Additionally, `confidence(v)` builtin was added to support Fluid type introspection:
- `Fluid` → highest confidence score
- Concrete value → 1.0

And `find(type_name, field, op, threshold)` was added as an interpreter-level special form (not a pure builtin) to support entity store queries from the 3ae5ce2 test suite.

## Soft-Failure Semantics

All string builtins follow the Metalogos principle of soft-failure over hard errors:

- **`substring` with out-of-bounds indices:** clamps to valid range, returns empty string if `start >= len(s)`.
- **`char_at` with out-of-bounds index:** returns empty string.
- **`to_float` with unparseable string:** returns 0.0.
- **`index_of` with no match:** returns -1.0 (not an error — this is a valid query result).

This matches the `recall()` pattern (empty string on no match) and the general Metalogos philosophy that runtime errors should be recoverable, not fatal.

## Syntax-Level Indexing: Deferred

The skill document proposes `s[i]` and `s[start..end]` as syntax. This is **deferred** to a future phase because:

1. Grammar ambiguity risk with Fluid Types and list literals.
2. Builtin functions provide equivalent functionality.
3. The priority is unblocking programs, not maximizing syntactic sugar.

If added later, the syntax `s[i]` would parse as `Expr::Index { base, index }` and `s[start..end]` as `Expr::Slice { base, start, end }`, distinct from Fluid Type syntax which is parsed at the declaration level.

## Consequences

- **Positive:** String processing is now possible in `.mlog` programs without Rust-builtins. The self-hosted lexer (Phase 5.5) can use `char_at`, `substring`, `index_of` instead of custom Rust functions.
- **Positive:** `len(s)` already worked for strings (Phase 5.2). Now complemented by the full string operation suite.
- **Positive:** No grammar changes = no risk of parser regressions.
- **Negative:** Slightly more verbose than syntax-level indexing (`substring(s, 0.0, 5.0)` vs `s[0..5]`). Acceptable for Phase 5.3 scope.
- **Neutral:** `to_float` duplicates `float()` semantics but with soft-failure (0.0 on error vs hard error). Both are available; `to_float` is preferred for safe parsing.

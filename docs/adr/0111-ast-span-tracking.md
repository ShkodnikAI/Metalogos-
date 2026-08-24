# ADR-0111: Inline span tracking in AST nodes

**Status:** Accepted
**Date:** 2026-08-25
**Naryad:** #121

## Context

Before this change the `Span` struct existed in `ast.rs` (fields
`start_line`, `start_col`, `end_line`, `end_col`) but was completely
unused: no AST node stored its position, the parser never populated
these fields, and error messages never showed a line number.

Semantic errors like `duplicate entity type: User` gave no indication of
*where* in the source the problem occurred, making debugging non-trivial
programs difficult.

## Decision

### 1. Inline `span` field, not `Spanned<T>` wrapper

We considered wrapping every node in `Spanned<T> { node: T, span: Span }`.
Rejected because `Expr` had 15 tuple variants (e.g. `StringLit(String)`)
that would all need converting to struct variants anyway — the wrapper
adds indirection without saving work. Inline fields are the standard Rust
approach (used by `rustc` and `syn`).

### 2. `Span::from_pest()` method

A single conversion method on `Span` rather than a free function in
`parser/helpers.rs`. Accepts `pest::Span`, converts 1-indexed lines /
0-indexed columns directly (matching pest convention).

### 3. Improved `Display` for `Span`

Single-line spans render as `"line:col"` instead of the full
`"line:col-end_line:end_col"` range. Multi-line spans keep the full
range.

### 4. `Span::unknown()` for programmatic constructions

Test fixtures and programmatically built AST nodes (audit, server)
use `Span::unknown()` (all zeros). Semantic errors with unknown span
(`start_line == 0`) omit the line prefix.

### 5. Error message format

Semantic errors now include `"строка N: "` prefix:
```
строка 5: duplicate entity type: User
```
If the span is unknown, the prefix is not added (backward compatible).

## Consequences

- All 15 `Expr` tuple variants became struct variants with a `span` field.
  Three `Statement` variants (`IfThen`, `Return`, `ExprStmt`) similarly
  converted. `Break` and `Stay` remain unit variants (keywords with no
  meaningful source extent).
- All 41 `Declaration` inner structs received `pub span: Span`.
- ~270 match sites across the codebase updated to use `..` to ignore
  the new field — zero change in execution logic.
- 5 new parser tests verify real positions in error messages.
- AST grew from 731 to 1 289 lines; total src from ~58 000 to ~59 000 LOC.
- Golden tests and crosscheck backends (1 passed, 0 failed) unchanged.
- Future work: LSP diagnostics, IDE goto-definition, and error underlining
  can consume `Span` directly.

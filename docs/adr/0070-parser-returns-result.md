# ADR-0070: Parser returns Result instead of abort()

**Status:** Accepted
**Date:** 2026-07-31
**Narad:** #31, Block 1

## Context

`src/parser.rs` contained 27 calls to `std::process::abort()` in two categories:

1. **True grammar invariants (25)** — `unwrap_or_else` guards asserting that Pest's
   AST always contains expected child rules. These never trigger on valid input
   but kill the process without stack unwinding, `Drop`, or diagnostic output when
   they do.

2. **Fallback handler (1)** — the "unrecognized statement" branch in
   `parse_single_statement` (L2047) which aborted on genuinely malformed input.
   This is the only case that represents a real error path, not a grammar invariant.

All 27 were introduced during three iterations in Narad #29 as `.expect()` →
`unreachable!()` → `abort()`. None changed the behavior — each iteration merely
renamed the abort.

## Problem

- **DoS vector:** A malformed `.mlog` file triggers `abort()` in `mlog serve`,
  killing the entire process (no per-request isolation since tokio catches `panic!`
  but not `SIGABRT`).
- **Untestable:** `#[should_panic]` tests cannot catch `abort()` — the test
  runner dies with the process.
- **No diagnostics:** Only `eprintln!` to stderr without source position.

## Decision

All 27 `abort()` calls replaced with `Result<_, ParseError>` propagation.

### Approach

1. Added `pair_error(pair, msg)` helper that creates a `pest::error::Error<Rule>`
   with line:col position from `pair.as_span().start_pos().line_col()`.

2. Changed ~35 function return types from bare values to `Result<T, ParseError>`:
   - `parse_expression` → `Result<Expr, ParseError>`
   - `parse_single_statement` → `Result<Statement, ParseError>`
   - All declaration parsers that call them (directly or transitively)
   - Leaf types: `parse_compare_op`, `parse_binop`, `parse_branch_condition`

3. Replaced each `abort()` with `?`-based error propagation:
   - `unwrap_or_else(|| { abort() })` → `.ok_or_else(|| pair_error(...))?`
   - `_ => abort()` → `_ => Err(pair_error(...))?`
   - Fallback: `return Err(pair_error(&pair, "unrecognized statement ..."))`

4. Added `?` to ~50+ call sites, `.collect::<Result<_, _>>()?` for iterator
   chains, and `.transpose().unwrap_or(Ok(default))?` for optional expressions.

5. For optional fields (DB URL, LLM provider key, default model) where parse
   errors should silently yield `None`, used `.and_then(|e| parse_expression(e).ok())`
   to convert `Result` to `Option`.

### Prior art

- **rustc**: Every parse function returns `Result` or `PResult`. Errors include
  `Span` for position. Invalid tokens produce structured diagnostics.
- **Elm compiler**: Parse errors carry source position and are collected
  (not fatal). The compiler continues to find multiple errors in one pass.
  Both give position and continue parsing rather than aborting.

## Consequences

- `parse()` already returned `Result<_, ParseError>` — this change propagates
  Result through internal functions without changing the public API.
- All 373 existing unit tests continue to pass (they call `parse().unwrap()`).
- `mlog run malformed.mlog` now exits code 1 with `error: parse error: ...`
  including line:col position.
- No behavioral change for valid input — all paths that previously returned
  values now return `Ok(value)`.

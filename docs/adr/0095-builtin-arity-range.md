# ADR-0095: Builtin Arity Range

## Status
Accepted

## Context
BUILTIN_REGISTRY had single `arity: usize`. Functions with optional args
could not be described. ~59 entries had incorrect arity values.

## Decision
1. Add `max_arity: Option<usize>` to BuiltinSpec
2. `spec!` macro for concise entries
3. `check_builtin_arity()` for compile-time validation
4. Expression walker in semantic.rs

## Consequences
- `mlog check` catches wrong-arity calls
- Two arity tests enabled
- ~59 entries corrected

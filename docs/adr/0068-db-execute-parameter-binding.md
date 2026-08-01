# ADR-0068: Parameterised db_execute

## Status

Accepted

## Context

`db_execute` executed SQL with `conn.execute(&sql, [])` — the second
argument (parameters) was silently ignored. All 37 calls in FOSVEN
production code assembled SQL via string concatenation, using `escape_json`
for quoting. Since `escape_json` does not escape single quotes, any
user message containing an apostrophe (e.g., "I'm", "O'Brien")
corrupted or broke INSERT/UPDATE statements.

`query` already supported a second List argument with proper binding
via `rusqlite::params_from_iter`. The two builtins were out of parity.

## Decision

Add an optional second argument (List) to `db_execute`, using the same
binding mechanism as `query`. Single-argument calls (SQL string only)
continue to work unchanged.

Value conversion for parameters:
- `String` → `rusqlite::types::Value::Text`
- `Float` → `rusqlite::types::Value::Real`
- `Unit` → `rusqlite::types::Value::Null`
- Other types → empty string (defensive)

## Alternatives considered

1. **String concatenation with proper escaping** — fragile, requires
   callers to remember to escape. Easy to forget.

2. **Replace reqwest::blocking with async reqwest** — unrelated scope.

3. **Parameterised queries (chosen)** — standard practice, eliminates
   injection surface entirely. Prior art: OWASP A03:2021 (Injection).

## Prior art

- `query()` implementation in `src/interpreter.rs` — existing pattern.
- `rusqlite::params_from_iter` — idiomatic Rust SQLite parameter binding.
- OWASP Top 10, A03:2021 — Injection.

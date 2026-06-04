# ADR-0030: Parameterized Queries via Opaque Query Type

**Status:** Accepted
**Date:** 2026-06-03
**Phase:** 6.3 — Database Access

## Context

SQL injection remains one of the most critical web vulnerabilities (OWASP A03:2021). It occurs when user input is concatenated into SQL strings rather than passed as parameters.

Metalogos web applications need database access, but must make SQL injection structurally impossible — not merely discouraged through best practices.

## Decision

Introduce an **opaque `Query` value variant** that can only be produced by the `query()` builtin with a string literal SQL template and parameterized arguments.

```mlog
let q = query("SELECT * FROM users WHERE email = ? AND active = ?", [email, true])
// q is Query — opaque, cannot be concatenated from String
let results = db_exec(q)
// String + Query → runtime error
// Query + String → runtime error
```

### Design Rules

1. `Query` is a distinct `Value::Query { sql: String, params: Vec<Value> }` variant.
2. `query()` requires its first argument to be a **string literal** (enforced at parse time or early validation). Runtime strings cannot be passed as the SQL template.
3. `?` placeholders are bound positionally from the params list.
4. `db_exec(q)` sends the query to the database using prepared statements.
5. No `Value::String` to `Value::Query` conversion exists.

## Prior Art

- **sqlx (Rust):** Compile-time SQL verification, mandatory parameterized queries.
- **Haskell Persistent/Hdbc:** Typed query combinators that prevent raw SQL concatenation.
- **Java PreparedStatement:** The industry-standard parameterized query mechanism.

## Consequences

- **Positive:** SQL injection is structurally impossible — user-supplied data cannot alter the SQL structure.
- **Positive:** Prepared statements provide additional performance benefits (query plan caching).
- **Neutral:** Dynamic query construction (e.g., optional WHERE clauses) requires composable query builders, not string concatenation.
- **Negative:** The string-literal restriction means some advanced dynamic SQL patterns need a dedicated builder API rather than ad-hoc string assembly.

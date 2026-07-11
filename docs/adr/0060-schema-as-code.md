# ADR-0060: Schema-as-Code — Additive-Only Table DDL in .mlog

**Date:** 2026-07-12
**Status:** Accepted
**Context:** Наряд METALOGOS_4_PRIMITIVES v2, Problem C

## Problem

FOSVED-office-v2 has two parallel sources of database schema:
1. `llm_proxy.py init_db()` — creates 10 tables
2. `app.mlog InitDB()` pattern — creates 22 tables via raw SQL strings

Tables like `research_source`, `analysis`, `yana_events` are referenced in mlog code but defined in raw SQL strings scattered across the codebase. When creating a new department, there's no way to declare archive tables together with the department profile — table creation is desynchronized from the code that uses them.

## Decision

Add a `schema` declaration to the Metalogos language:

```mlog
schema osp_analysis {
  table analysis {
    id: Int primary_key auto_increment
    topic: String
    status: String default("drafted")
    created_at: DateTime default(now())
  }
  table research_source {
    id: Int primary_key auto_increment
    analysis_id: Int references(analysis.id)
    url: String nullable
    title: String
  }
}
```

### Design

- **Additive-only migration**: `CREATE TABLE IF NOT EXISTS` — never drops or alters existing columns
- **Type mapping**: Int→INTEGER, Float→REAL, String/Text→TEXT, Bool→INTEGER, DateTime→TEXT
- **Modifiers**: `primary_key`, `auto_increment`, `nullable`, `references(table.field)`
- **Defaults**: `default("value")` for literals, `default(now())` → `DEFAULT (datetime('now'))`
- **Requires `db` declaration first** — schema processing needs an active database connection

### Backend Support

| Backend | Schema DDL | db_insert |
|---|---|---|
| Tree-walking | Full support | Full support |
| Bytecode/VM | No-op (compile-time only) | Deferred — VM has no db_conn |
| JIT | No-op (compile-time only) | Deferred |

Schema is inherently runtime-only (requires SQLite connection). The VM/JIT path is deferred until a DB connection strategy is designed for those backends.

### STOP Trigger #2 Resolution

The existing `db { url: ... }` handles connection config only. A new `schema` keyword was required — extending `db` was rejected to avoid conflating connection config with table definitions.
---
name: database-design
description: Database schema design principles for Postgres + Prisma stack. Normalization with deliberate denormalization, naming conventions, indexing strategy, migration discipline, soft deletes vs hard deletes, audit trails. Decisions made at schema design time compound for the lifetime of the project — bad schemas haunt for years.
---

# Database Design — Schemas That Age Well

A database schema is the most expensive thing in a project to change after launch. Every other layer (UI, API, business logic) can be rewritten relatively cheaply. Schema migrations on production tables with real data are slow, risky, and sometimes irreversible. Design carefully.

## Prerequisites

- Project type requires persistence
- Postgres chosen (default per tech radar)
- Prisma chosen as ORM (default)
- Domain modeled at conceptual level before schema

## Core principle

> Schema decisions are 100x more expensive to undo than make. The hour spent designing carefully pays back over years. Naive schemas built for "we'll figure it out later" become the technical debt that defines the rest of the project.

## Naming conventions

**Tables:** snake_case plural — `users`, `dev_tasks`, `architecture_decisions`
**Columns:** snake_case — `user_id`, `created_at`, `is_active`
**Booleans:** `is_` or `has_` prefix — `is_active`, `has_subscription`
**Timestamps:** `created_at`, `updated_at`, `deleted_at`, `verified_at`, `archived_at`
**Foreign keys:** `<table_singular>_id` — `user_id`, `task_id`
**Indexes:** Prisma generates names; for manual `idx_<table>_<columns>`

Prisma models in PascalCase singular — `User`, `DevTask`, `ArchitectureDecision`. Map to snake_case via `@@map`.

## Required columns on every table

- `id` — primary key, autoincrement int OR UUID for distributed/external-facing
- `created_at` — timestamp default now()
- `updated_at` — timestamp with `@updatedAt` directive

Almost-always required:
- `user_id` for multi-tenant data (default 2044005421 for now per multi-user readiness)

For soft-delete-eligible tables:
- `deleted_at` — nullable timestamp

For audited entities:
- `created_by`, `updated_by` (user IDs)

## Foreign keys discipline

Use foreign keys with explicit `onDelete` behavior:

```prisma
model Post {
  id     Int  @id @default(autoincrement())
  userId Int  @map("user_id")
  user   User @relation(fields: [userId], references: [id], onDelete: Cascade)
  // ^^ when user deleted, their posts go too

  // OR
  user User @relation(..., onDelete: SetNull)  // post stays, user_id nulled

  // OR
  user User @relation(..., onDelete: Restrict)  // deletion of user blocked if posts exist
}
```

Pick the right behavior. Default Cascade is dangerous — accidentally delete user deletes all their data.

## Soft delete vs hard delete

**Hard delete:** row removed from table. Use when:
- Data has no historical value
- Regulatory deletion required (GDPR, etc.)
- Temporary/cache data
- Junction table entries

**Soft delete:** `deleted_at` timestamp set, row stays. Use when:
- Audit trail required
- Recoverability needed
- Referential integrity matters (other tables reference it)
- Default for business-critical entities (Users, Orders, Documents)

Add to soft-delete tables:
```prisma
model Document {
  id        Int       @id @default(autoincrement())
  ...
  deletedAt DateTime? @map("deleted_at")

  @@index([deletedAt])  // for filtering active records
}
```

Queries always filter `deletedAt: null` unless explicitly recovering.

## Indexing strategy

**Always indexed:**
- Foreign keys (Prisma auto-indexes via `@@index`)
- Columns in WHERE clauses for common queries
- Columns in ORDER BY for common queries
- `deleted_at` if soft delete

**Composite indexes** for multi-column filters:
```prisma
@@index([userId, createdAt(sort: Desc)])  // for "my posts, newest first"
@@index([status, priority])               // for "active high-priority tasks"
```

**Unique constraints** for business rules:
```prisma
@@unique([userId, slug])  // user can't have two posts with same slug
```

**Don't over-index:** every index slows writes. Index when query plan shows need. Use `EXPLAIN ANALYZE` to verify.

**Partial indexes** for selective queries:
```sql
CREATE INDEX idx_active_users ON users(email) WHERE deleted_at IS NULL;
```

## Normalization level

**3NF (Third Normal Form) as default.** Each table represents one entity. Foreign keys link related entities.

**Deliberate denormalization** when:
- Read-heavy query needs join'd data constantly (denormalize for read speed)
- Aggregations needed in real-time (store computed values)
- Historical accuracy needed (don't let parent update affect historical record)

Example: Order stores `customer_name` snapshot at order time, even though Customer table exists. Why: customer changes name later, the historical order still shows the name at time of order.

Don't denormalize prematurely. Only when proven performance need or business need.

## Data types

**Postgres types map to Prisma:**

| Domain | Prisma | Postgres |
|--------|--------|----------|
| Identifier | `Int @id @default(autoincrement())` | INTEGER PRIMARY KEY |
| External ID / UUID | `String @id @default(uuid())` | UUID |
| Short string | `String` | VARCHAR |
| Long text | `String @db.Text` | TEXT |
| Email | `String` | VARCHAR (validate in app) |
| Money | `Decimal @db.Decimal(12, 2)` | NUMERIC(12,2) (never float for money!) |
| Boolean | `Boolean` | BOOLEAN |
| Timestamp | `DateTime` | TIMESTAMP |
| JSON | `String @db.Text` storing JSON | TEXT (Prisma JSON type also exists) |
| Enum | `String` with app-level enum | VARCHAR + check constraint |

**Never use float for money.** Use Decimal.

**JSON columns:** stored as TEXT with JSON serialization. Sometimes Prisma JSON type, but TEXT gives more control. Validate JSON structure at app level.

## Enum-like values

**Avoid Postgres ENUM type** — changing values requires migration. Use VARCHAR with app-level constants:

```typescript
// types/orderStatus.ts
export const ORDER_STATUSES = ['pending', 'paid', 'shipped', 'delivered', 'cancelled'] as const;
export type OrderStatus = typeof ORDER_STATUSES[number];

// validate at boundaries
if (!ORDER_STATUSES.includes(status)) throw new Error('Invalid status');
```

Schema:
```prisma
model Order {
  status String @default("pending")
  ...
}
```

This lets you add statuses without migration.

## Migration discipline

**Every schema change is a migration.** Never edit production schema directly.

```bash
npx prisma migrate dev --name add_user_avatar --create-only
# Review generated SQL
cat prisma/migrations/*/migration.sql
# If correct:
npx prisma migrate dev  # apply locally
# After testing:
npx prisma migrate deploy  # in production
```

**Migration safety:**
- Adding columns: safe (with default or nullable)
- Adding indexes: safe but locks table briefly (use CONCURRENTLY for large tables)
- Renaming columns: NOT safe, requires deploy-time coordination
- Dropping columns: NOT safe, breaks running services
- Type changes: usually NOT safe

**Two-phase rollouts for breaking changes:**
1. Add new column, write to both old and new in code
2. Backfill data
3. Switch reads to new
4. Drop old column

**Test migrations on copy of production data** before running in production.

## Audit trails

For business-critical entities, log changes:

**Option A: separate audit table:**
```prisma
model OrderAudit {
  id        Int      @id @default(autoincrement())
  orderId   Int
  fieldName String
  oldValue  String?
  newValue  String?
  changedBy Int
  changedAt DateTime @default(now())
}
```

**Option B: temporal table (Postgres feature):**
Use `pg_temporal` extension for automatic history.

**Option C: append-only design:**
Don't update rows; insert new version with version number. Old versions kept.

For most cases: Option A is pragmatic. Triggers can populate it automatically.

## Patterns by entity type

**User-owned content:** `user_id` foreign key, soft delete, index on user_id.

**Junction tables:** composite primary key on both FKs. No soft delete usually (just delete row).
```prisma
model UserRole {
  userId Int
  roleId Int
  @@id([userId, roleId])
}
```

**Time-series data:** `recorded_at` timestamp, index on it for range queries. Consider partitioning if growing rapidly.

**Hierarchical data (tree):** either parent_id (adjacency list) or materialized path. Adjacency list simpler for shallow trees; materialized path faster for deep queries.

**Tags/metadata:** JSONB column with GIN index for flexible search. Or separate tags table with junction.

## Connection pooling

Postgres connections are expensive. Use a pooler:
- **PgBouncer** (most common, transaction mode default)
- **Supabase Transaction Pooler** (built-in)

Prisma connection limit: default 10. Configure based on load: `connection_limit=20` in DATABASE_URL.

For serverless: use pooler URL (port 6543 in Supabase). Direct URL (port 5432) only for migrations.

## Anti-patterns

- **Singular table names.** `user` instead of `users`. Convention violation, also conflicts with reserved words.
- **No `created_at`.** Critical for debugging and analytics.
- **Boolean status fields.** `is_active`, `is_paid`, `is_shipped` instead of single `status` enum. Hard to reason about state.
- **Storing arrays in single column as comma-separated strings.** Use proper junction table or JSONB.
- **Mixing concerns in one table.** `users` table with `last_login_ip` makes it bloated. Separate sessions table.
- **No foreign keys.** "We'll enforce in code" — never works, eventually orphan records appear.
- **NULL boolean.** Three-state boolean is confusing. Use timestamp (`verified_at` nullable) instead.
- **Floating point money.** $9.99 stored as 9.99000000003. Always Decimal.
- **String enum without validation.** Free-form text in status column. Eventually `paid `, `Paid`, `PAID` all exist.
- **No indexes on FK.** Default in many DBs but explicit is better.
- **Wide tables.** 50+ columns means model is doing too much. Decompose.

## Schema review checklist

Before merging schema migration:
- [ ] Naming follows conventions
- [ ] FKs have explicit onDelete
- [ ] Soft-delete decision documented
- [ ] Indexes for known query patterns
- [ ] No floating point money
- [ ] Enums via VARCHAR + constants
- [ ] Required columns present (id, created_at, updated_at)
- [ ] user_id present for multi-tenancy
- [ ] Migration tested on dev DB with sample data
- [ ] ADR written if non-obvious decision

## Integration

- `nextjs-architecture` uses Prisma client in lib/db.ts singleton
- `api-design` works against schema entities
- `code-organization-standards` requires prisma/ in repo root
- Schema design choices recorded as ADRs
- `lib/dev.js` `createWebAppRepo` scaffolds Prisma setup per this skill

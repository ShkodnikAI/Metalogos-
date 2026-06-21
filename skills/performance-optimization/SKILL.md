---
name: performance-optimization
description: Methodology for measuring, identifying, and fixing performance bottlenecks. Optimization without measurement is gambling. The discipline: profile first, find actual bottleneck, fix it, measure again. Premature optimization is the root of complexity. This skill is invoked only when performance becomes an actual problem (defined by SLO breach or user complaint), not preemptively.
---

# Performance Optimization — Measure, Don't Guess

The most common mistake in performance work: optimizing code that's not the bottleneck. The second most common: optimizing without measuring before/after.

The discipline: never optimize without data. The data tells you what's slow. The data tells you whether your fix worked. Without data, you're hill-climbing in fog.

## Prerequisites

- Active performance problem (SLO breach, user complaint, observable degradation)
- Profiling tools available (browser DevTools, Node.js profiler, database EXPLAIN, etc.)
- NOT for: speculative "this might be slow later"

## Core principle

> Profile first. Measure twice. Optimize once. Verify. Then move on. The optimization you imagine is rarely the optimization that helps. Every optimization adds complexity — pay the cost only when measured benefit justifies.

## The 5-step performance protocol

### Step 1 — Define the problem with numbers

Not "it's slow". Specifically:
- What action? (page load, API call, query, render)
- What measurement? (p50, p95, p99 latency, throughput, memory)
- What baseline? (current value)
- What target? (acceptable value)

Example:
- "API endpoint GET /api/users/:id"
- "p95 latency"
- "current: 2400ms"
- "target: <500ms"

Without these numbers, you can't know when you're done.

### Step 2 — Profile the actual workload

Get data on where time is actually spent.

For Node.js backend:
```javascript
// Built-in profiler
node --prof app.js
// Then: node --prof-process isolate-*.log > profile.txt

// Or clinic.js for visual
npx clinic doctor -- node app.js

// Or just add timestamps
console.time('db_query');
const result = await db.query(...);
console.timeEnd('db_query');
```

For database:
```sql
EXPLAIN ANALYZE SELECT * FROM users WHERE id = 1;
-- Shows actual execution plan, time, rows
```

For frontend:
```javascript
// React Profiler (built-in)
import { Profiler } from 'react';
<Profiler id="UserList" onRender={callback}>
  <UserList />
</Profiler>

// Chrome DevTools Performance tab — record, analyze
```

For full-stack tracing:
- Sentry Performance
- Datadog APM
- OpenTelemetry + Jaeger

### Step 3 — Identify the bottleneck

The profile reveals where time goes. Almost always concentrated:
- **Pareto principle:** 80% of latency in 20% of operations
- Most code is NOT the bottleneck
- Don't optimize what doesn't show in profile

Common bottlenecks by category:

**Database (most frequent):**
- N+1 queries (loop with query inside)
- Missing index
- Full table scan
- Locking / contention

**Network:**
- Sequential requests that could be parallel
- Large payloads
- No caching
- Chatty APIs (10 calls for one user action)

**CPU:**
- Synchronous heavy computation in main thread
- Inefficient algorithms (O(n²) where O(n log n) possible)
- Excessive serialization/deserialization

**Memory:**
- Memory leaks (refs not freed)
- Large objects loaded entirely (read 1GB file into memory)
- Many small allocations triggering GC pressure

**Frontend specific:**
- Large bundles (slow first paint)
- Unnecessary re-renders
- Blocking main thread
- Layout thrashing

### Step 4 — Fix the bottleneck

Match fix to bottleneck:

**N+1 query → batch with JOIN or eager loading**
```javascript
// Before (N+1)
const users = await db.user.findMany();
for (const u of users) {
  u.posts = await db.post.findMany({ where: { userId: u.id } });
}
// N+1 queries

// After (single query with relation)
const users = await db.user.findMany({
  include: { posts: true }
});
// 1 query
```

**Missing index → add index**
```sql
CREATE INDEX idx_users_email ON users(email);
```

**Sequential network calls → parallel:**
```javascript
// Before
const a = await fetchA();
const b = await fetchB();
const c = await fetchC();

// After
const [a, b, c] = await Promise.all([fetchA(), fetchB(), fetchC()]);
```

**Heavy CPU work → worker thread or queue:**
```javascript
// Move expensive computation off main thread
const { Worker } = require('worker_threads');
```

**Memory leak → fix the reference cycle, use WeakRef, profile heap snapshots**

**Large bundles → code splitting, dynamic imports**
```javascript
// Lazy load heavy library
const heavyLib = await import('./heavy-lib');
```

**Re-renders → memoization (React.memo, useMemo, useCallback)**

### Step 5 — Measure again

Same profiler, same workload, same measurement. Compare:
- Did latency drop?
- By how much?
- Did anything else regress?

If improvement < 20% — optimization probably not worth complexity cost. Consider reverting.

If improvement ≥ 50% — keep it. Document in ADR.

If unchanged — fix the wrong thing. Re-profile.

## Common patterns of "fast enough"

Not every operation needs to be optimal. "Fast enough" criteria:
- Page load: <2s on average connection
- API: <500ms p95 for typical endpoint
- Database query: <100ms p95
- Background job: variable, but should not block user-facing work

If you're well below threshold — don't optimize. Allocate effort elsewhere.

## Premature optimization warning signs

If asking "should I optimize this?", check:
- Is there measured evidence of slowness? If no — don't optimize.
- Is the slow operation in the critical path? If no — don't optimize.
- Will the optimization add significant complexity? If yes and benefit unclear — don't optimize.
- Could a profile reveal a bigger fish elsewhere? Profile first.

## Caching as performance solution

Caching is the most common optimization. Three layers:

**Application-level cache:**
```javascript
const cache = new Map();
async function getUser(id) {
  if (cache.has(id)) return cache.get(id);
  const user = await db.user.findUnique({ where: { id } });
  cache.set(id, user);
  setTimeout(() => cache.delete(id), 60000); // TTL
  return user;
}
```

**Redis/Memcached** for distributed caches across instances.

**HTTP caching** via `Cache-Control` headers for static or semi-static responses.

**CDN** for assets and edge-cacheable responses.

Cache invalidation is the hard part. Document invalidation strategy in ADR.

## Anti-patterns

- **Optimizing without profiling.** Most common mistake. You don't know what's slow.
- **Optimizing for theoretical scale.** "What if we have 1M users?" — when you have 100. Premature.
- **Micro-optimization at expense of clarity.** Converting `arr.length` to `len` doesn't matter; making code unreadable does.
- **Caching everything.** Caches add complexity. Use where measured benefit exists.
- **One-off optimizations without measuring.** "I tightened this loop" — did it help? Without measurement, you don't know.
- **Ignoring the obvious bottleneck.** Database is usually slowest. Network second. CPU rarely the issue in web apps.
- **Optimizing during initial build.** Build for correctness first. Optimize only after measurement reveals problem.

## When to call this skill

NOT preventively. Triggers:
- SLO/SLI breach in monitoring
- User reports of slowness
- Specific scaling event (load testing reveals issue)
- Significant cost issue (slow == expensive in serverless)

## Integration with other skills

- `observability-setup` provides the data this skill operates on
- `database-design` covers schema-level performance (indexes, denormalization)
- `architecture-decision-records` records optimization decisions
- `qa/performance-testing` validates improvements
- `refactoring-discipline` ensures optimization doesn't sacrifice maintainability

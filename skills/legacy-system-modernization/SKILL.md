---
name: legacy-system-modernization
description: Strategies for modernizing existing codebases without rewriting from scratch. Big-bang rewrites usually fail. Strangler fig pattern, incremental migration, parallel running, deprecation paths — these work. Applied when inheriting a project, when adopting Fosved methodology for existing code, or when major architectural shift needed without rebuild risk.
---

# Legacy System Modernization — Strangle, Don't Rewrite

The most appealing approach to legacy code is rewrite. The most failure-prone approach is rewrite. Joel Spolsky's classic essay "Things You Should Never Do" calls full rewrites "the single worst strategic mistake".

This skill is about modernizing without rewriting: incrementally replacing parts of the system while keeping it running.

## Prerequisites

- Existing system (legacy code, third-party app being adopted, old project being modernized)
- Clear understanding of WHY modernization is needed (problem statement, not just "it's old")
- Active production usage that can't tolerate downtime
- `architecture-decision-records` discipline ready

## Core principle

> The system you have, however imperfect, currently works in production. The system you imagine to build is hypothetical. Replace the working system gradually — each step keeps production working. Big-bang rewrites enter a long valley of "nothing works yet" that often kills them before they emerge.

## When NOT to rewrite

Common pressures to rewrite (resist):
- "The code is messy" — refactor in place, don't rewrite
- "It uses old framework" — most frameworks remain functional decades
- "Original author left" — fix knowledge gap, not code
- "I could write it better" — probably overestimating
- "It's slow" — profile and optimize specific bottlenecks
- "It has bugs" — fix bugs

Legitimate reasons to consider modernization (not necessarily rewrite):
- Architectural ceiling reached (can't scale, can't add features)
- Critical security vulnerabilities can't be patched in current architecture
- Underlying platform is being EOL'd (e.g., language version no longer supported)
- Business model fundamentally changed
- Maintenance cost exceeds rebuild cost over reasonable horizon

## The strangler fig pattern (foundational)

Named after the strangler fig tree, which grows around its host tree, eventually replacing it. The host tree dies, the strangler stands where it was.

Apply to software:
1. Build new functionality alongside the old
2. Route specific requests/features to the new system gradually
3. Old system continues serving everything else
4. Over time, more and more flows to new system
5. Eventually nothing routes to old system
6. Decommission old

This avoids the "big bang rewrite" failure mode. At every point, system works.

### Strangler fig in practice

**Web API example:**
1. Add proxy/router in front of old API
2. New endpoints route to new service
3. Old endpoints route to old service
4. Migrate endpoint by endpoint
5. After all migrated, remove proxy

```
Client ───→ Router (NEW)
              │
              ├─→ /api/v1/users/* → Old Backend (legacy)
              ├─→ /api/v1/posts/* → Old Backend (legacy)
              └─→ /api/v2/* → New Backend (modern)

# Over time:
# Migrate /api/v1/users → New Backend
# Migrate /api/v1/posts → New Backend
# Eventually all routes go to New Backend
# Decommission Old Backend
```

**Database example:**
- Write to both old and new DB schemas (dual-write)
- Read from old DB primarily
- Gradually shift reads to new DB
- Once confidence built, write only to new
- Decommission old

## The 6 R's of legacy modernization

Borrowed from cloud migration, adapted:

### 1. Retire
The system isn't needed anymore. Just shut it off. Easiest "modernization".
Audit before this: what depends on it? What breaks?

### 2. Retain
Leave it alone. Sometimes the right answer.
- If it works and isn't blocking other improvements — let it be.
- The cost of touching old systems is often hidden.

### 3. Rehost (lift-and-shift)
Move to new infrastructure, no code changes.
- Database migration (Postgres 10 → Postgres 16)
- Server migration (own server → Render)
- Container migration

Cheapest modernization. Lowest risk. Limited benefit.

### 4. Replatform
Lift-and-shift with small adjustments.
- Switch caching layer (Memcached → Redis)
- Upgrade language version (Node 14 → 20)
- Replace small library

Some risk, some benefit, manageable scope.

### 5. Refactor
Restructure code internally, keep behavior same.
- Extract modules
- Improve naming
- Reduce coupling
- See `refactoring-discipline` skill

This is the most common, ongoing modernization.

### 6. Replace (rewrite)
Full rewrite. Only when other R's exhausted.
Strangle approach (above) recommended even when replacing — don't go big-bang.

## Modernization planning protocol

1. **State the problem precisely.** Not "it's old". Specifically: "Cannot scale beyond X req/s due to monolithic architecture" or "Cannot adopt feature Y without breaking change."

2. **Inventory current state.**
   - Architecture diagram (actual, not aspirational)
   - Dependency map
   - Performance characteristics
   - Known issues and tech debt
   - Business value (which parts are critical)

3. **Define target state.**
   - What modern architecture would solve the stated problem?
   - What's NOT changing?
   - What's the smallest change that delivers value?

4. **Plan in increments.**
   - Each increment ships independently
   - Each increment keeps system working
   - Each increment delivers some value
   - Roll-back plan for each increment

5. **Identify seams.**
   - Where can old and new coexist?
   - Where can routing decisions split traffic?
   - Where are integration points?

6. **Sequence by risk.**
   - Start with lowest-risk, highest-confidence increments
   - Build patterns and tooling on simple migrations first
   - Tackle complex/risky parts later with experience

## The shadow run pattern

Before fully cutting over:
1. Old system continues serving production
2. New system runs alongside, receives same inputs
3. Compare outputs (don't return new system's responses to user yet)
4. Investigate any differences
5. Only cut over when outputs match consistently for production load

This catches subtle differences in behavior that tests miss. Especially valuable for:
- Database migrations
- Algorithm changes
- API replacements

```javascript
async function getUser(id) {
  const oldResult = await oldSystem.getUser(id);
  
  // Shadow call new system, don't use result
  if (process.env.SHADOW_NEW_SYSTEM === 'true') {
    try {
      const newResult = await newSystem.getUser(id);
      const diff = compareResults(oldResult, newResult);
      if (diff) logger.warn('Shadow run difference', { id, diff });
    } catch (e) {
      logger.error('Shadow run failed', { id, error: e });
    }
  }
  
  return oldResult; // Production uses old system
}
```

After confidence: flip env var, new system serves, old fallback.

## Specific scenarios

### Scenario: monolith → modular monolith

NOT "monolith → microservices". That's usually a mistake. First, modularize within the monolith.

1. Identify domain boundaries within code
2. Refactor to enforce module boundaries (don't share data tables across modules)
3. Modules communicate via well-defined APIs
4. Could later become services, but often modular monolith is the right end state

### Scenario: old framework → new framework

E.g., AngularJS → React, Express → Next.js

Strangler approach:
1. Add new framework alongside
2. New pages/routes use new framework
3. Old pages remain in old framework
4. Iframe or direct routing to coexist
5. Migrate page by page

Don't try to migrate components — migrate by route/page.

### Scenario: synchronous → event-driven

1. Identify event boundaries
2. Continue calling synchronously, also emit events
3. New consumers subscribe to events
4. Once all consumers migrated, remove sync calls

### Scenario: own infrastructure → managed

E.g., self-hosted Postgres → Supabase, self-hosted Redis → cloud

Rehost approach:
1. Set up new managed instance
2. Replicate data
3. Switch app to new instance
4. Monitor
5. Decommission old

Test thoroughly in staging first.

## Anti-patterns

- **Big-bang rewrite.** Starts ambitious, ends in valley of "nothing works yet", often abandoned. Usually replaces a working system with hypothetical better system that never ships.
- **Modernization for modernization's sake.** "We should use the new framework" — without business need.
- **No rollback plan.** Each increment must be reversible. Without rollback, one bad increment kills modernization.
- **Touching too much at once.** Larger increments = more failure modes. Smaller = safer.
- **Ignoring the working system's wisdom.** Legacy code contains accumulated bug fixes and edge case handling. Discarding is discarding knowledge.
- **Two-system maintenance for too long.** Strangler pattern is meant to converge. If you've maintained old + new for 3 years, something is wrong.
- **Modernizing without measuring.** Did modernization actually solve the stated problem? Without before/after metrics, you don't know.

## Integration with other skills

- `architecture-decision-records` — every modernization step is an ADR
- `iterative-implementation` — small commits, working state
- `refactoring-discipline` — for in-place improvements
- `qa/regression-test-discipline` — critical for catching modernization breakage
- `devops-deployment` — feature flags for gradual rollouts
- `observability-setup` — measure before/after

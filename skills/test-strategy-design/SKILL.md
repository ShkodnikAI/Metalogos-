---
name: test-strategy-design
description: Designing a test strategy for a project — what to test at which level, coverage targets, test pyramid balance, what to NOT test. Strategy precedes individual tests because random test writing produces incomplete coverage and slow brittle suites. Strategy is opinionated about quality investment.
---

# Test Strategy Design — Architect Quality Before Writing Tests

Writing tests randomly produces noisy suites: too many low-level tests of trivial things, missing tests of critical paths, slow E2E for things unit could verify. Strategy comes first.

## Prerequisites

- Project has clear scope
- Critical user paths identified
- Failure modes considered (`failure-modes-mapping`)
- Risk tolerance understood

## Core principle

> Tests are not free. They take time to write, time to run, time to maintain. Strategic test investment maximizes confidence per minute of test time. The strategy decides where to invest, where to economize, where to skip.

## The test pyramid (classic shape)

```
        /\
       /E2E\        ← few, slow, full system
      /------\
     /Integ.  \     ← some, medium, components together
    /----------\
   /Unit Tests  \   ← many, fast, isolated functions
  /--------------\
```

Numbers (for typical project):
- Unit tests: 70% of test count
- Integration: 20%
- E2E: 10%

Why pyramid shape:
- Unit tests fast, easy to write, catch most issues at source
- Integration catches inter-component issues
- E2E catches issues only visible in full system, but slow + brittle

**Inverted pyramid (anti-pattern):** lots of E2E, few unit. Tests slow, brittle, expensive to maintain. Avoid.

## What to test at each level

### Unit tests

**Test:**
- Pure functions (deterministic, easy)
- Business logic in isolation
- Edge cases (boundary values, null, empty, max)
- Error handling paths
- State transitions

**Don't test:**
- Framework code (React, Next.js — already tested upstream)
- Trivial getters/setters
- Third-party libraries (tested by their maintainers)

### Integration tests

**Test:**
- API endpoint contracts (request → expected response)
- Database integration (queries return expected data)
- Multiple components working together
- Service boundaries
- Authentication/authorization flow

**Don't test:**
- UI rendering (E2E or component tests for that)
- External services directly (mock at boundary)

### E2E tests

**Test:**
- Critical user journeys (signup, checkout, etc.)
- Cross-system flows (Telegram bot → DB → response)
- Anything where multiple layers must align

**Don't test:**
- Every UI variant (combinatorial explosion)
- Edge cases (use unit/integration for those)

## Coverage targets

**Code coverage** is one metric, not the only.

- 80% line coverage on critical paths — reasonable target
- 95%+ on core business logic (auth, payments, data integrity)
- 50-70% on UI / glue code
- Lower acceptable for prototypes, scripts, throwaway code

**Don't chase 100%.** Diminishing returns. The last 10% covers code that's already simple or that tests poorly.

**Better metric: critical path coverage.** Identify top user journeys. Verify each has explicit test.

## Strategy per project type

### Web app

- Unit: business logic, utilities (Vitest)
- Integration: API routes, DB layer
- E2E: Playwright on critical journeys (login, primary task)
- Accessibility: automated (axe-core) + manual sampling
- Performance: budget assertions in CI

### Backend API

- Unit: services, validators
- Integration: full endpoint tests with test DB
- Contract tests: OpenAPI spec verification
- Performance: load test critical endpoints

### Telegram bot / AI office

- Unit: business logic, parsers, formatters
- Integration: handler tests with mocked Telegram
- AI evals: prompt regression, output quality (see `ai-evals-framework`)
- E2E: a few golden conversations end-to-end

### Static site

- Build tests: site builds successfully
- Accessibility: automated audit
- Link check: no broken internal links
- Visual regression (optional): screenshot diff for key pages

### Local AI deployment

- Unit: integration code
- AI evals: output quality vs cloud baseline
- Performance: latency, throughput targets
- Hardware verification: model fits in memory

## What NOT to test

**Don't test:**

- **Trivial accessors:** `function getName() { return this.name; }` — pointless test.
- **Framework internals:** React renders correctly, Next.js routes work. Trust them.
- **Library code:** Lodash, Prisma, etc. — tested by maintainers.
- **Constants:** `const MAX = 100` — what's there to test?
- **Implementation details:** "Test that this function calls helper A then helper B." Brittle, breaks on refactor. Test behavior, not implementation.
- **Layout pixel-perfectness:** "Button is 32px wide." Pin layout in design tokens, not tests.

**Test focus on:**

- **Behavior at boundaries:** what your code promises to do
- **Edge cases:** empty, null, max, weird inputs
- **Error paths:** what happens when things fail
- **Critical business logic:** money calculation, auth checks, data integrity
- **Cross-boundary interactions:** where bugs hide

## Test database strategy

For backend tests:

**Option 1: In-memory SQLite.** Fast, isolated per test. Limitations: not exact prod parity.

**Option 2: Real Postgres in container.** Spin up test container, run tests against it. Slower but accurate.

**Option 3: Shared test DB with cleanup.** Each test cleans up after. Race conditions if parallel.

For Fosved: **Option 2** (Postgres container) for integration tests. Vitest + Testcontainers.

```typescript
beforeAll(async () => {
  await prisma.$executeRaw`TRUNCATE TABLE users, posts CASCADE`;
});

beforeEach(async () => {
  // seed minimum data
});
```

## Mocking strategy

**Mock at boundaries** (external services, databases for unit tests).

**Don't over-mock.** If you mock everything, you're testing your mocks, not your code.

**For LLM in tests:**
- Unit tests: mock LLM (return canned responses)
- Integration tests: mock LLM (consistent inputs)
- AI evals: real LLM (the point IS testing LLM behavior)

```typescript
// Mock provider for non-AI tests
const mockProvider: LLMProvider = {
  name: 'mock',
  call: async () => ({
    text: 'mocked response',
    usage: { inputTokens: 10, outputTokens: 5 },
    model: 'mock',
    finishReason: 'stop'
  })
};
```

## Speed budget

Test suite should be:
- Unit suite: < 30 seconds (run on every save)
- Integration: < 5 minutes (run pre-push)
- E2E: < 15 minutes (run in CI)
- Full suite: < 30 minutes (CI gate)

If exceeding: investigate slow tests. Maybe parallelize. Maybe redesign suite.

Slow tests skipped → defeats purpose.

## CI integration

```yaml
# .github/workflows/ci.yml
- run: npm run test:unit        # always
- run: npm run test:integration  # always, against test DB
- run: npm run test:e2e          # on PR + main
- run: npm run test:ai-evals     # on main (cost — don't run on every PR)
```

**Failure means PR blocked.** Tests aren't suggestions.

## When tests fail in CI

1. Re-run once (transient issue?)
2. If failing again: investigate, don't merge
3. If failing on main: triage immediately — production protection broken

**Don't ignore CI red.** Every red CI = trust in tests degrades. Trust gone, tests stop being useful.

## Coverage reporting

Run with coverage:
```bash
npm run test:unit -- --coverage
```

Report shows:
- Per-file coverage
- Uncovered lines highlighted
- Branch coverage (each if/else path)

Use to find gaps in critical code. Don't optimize for the metric per se.

## Test data management

**Fixtures:** static test data in files.

**Factories:** functions that build test data with sensible defaults.
```typescript
function userFactory(overrides = {}) {
  return {
    id: 'test-id',
    email: 'test@example.com',
    name: 'Test User',
    ...overrides
  };
}
```

**Snapshots:** captured outputs that future test runs compare against. Useful for stable output but fragile when output changes intentionally.

## Anti-patterns

- **Implementation tests.** Test that function calls other function. Brittle. Test behavior.
- **Tests that test nothing.** "expect(true).toBe(true)". Passing test isn't passing test.
- **Inverted pyramid.** Lots of slow E2E, few unit. Suite slow, fragile.
- **No critical path coverage.** 90% line coverage but checkout flow not tested. Lots of tests, low confidence.
- **Coverage chasing.** Adding meaningless tests to hit 100%.
- **Flaky tests left flaky.** Tests sometimes fail "for no reason". Trust degrades. Fix or remove.
- **No CI gate.** Tests run locally but don't block merge.
- **Tests testing mocks.** So much mocked, real code not exercised.
- **Brittle E2E.** Selectors break on any UI change. Use stable selectors (data-testid).
- **Test pollution.** Tests affect each other through shared state.
- **No test database isolation.** Tests share DB, race conditions emerge.
- **Untested error paths.** Happy path covered, errors never tried. Production breaks on first real error.

## Strategy document

For each project, document strategy:

```markdown
# Test Strategy: <project>

## Risk profile
[critical / standard / experimental]

## Coverage targets
- Critical business logic: 95%
- API endpoints: 80%
- UI: 60%
- Utilities: 70%

## Test pyramid breakdown
- Unit: ~70%
- Integration: ~25%
- E2E: ~5%

## What's tested
- [List of areas]

## What's deliberately not tested
- [List + rationale]

## Test data approach
[Fixtures / factories / strategy]

## Mock boundaries
[Where mocks are placed]

## Test environments
- Local: <description>
- CI: <description>
- Staging: <description>

## Tooling
- Unit: Vitest
- Integration: Vitest + Testcontainers (Postgres)
- E2E: Playwright
- AI evals: Promptfoo
- Coverage: c8

## CI integration
[What blocks merge]

## Performance budget
- Unit < 30s
- Integration < 5min
- E2E < 15min
```

Stored as `/docs/test-strategy.md` in project.

## Integration

- Foundation for all QA work
- `failure-modes-mapping` informs strategy
- `unit-testing-craft` / `integration-testing-patterns` / `e2e-testing-with-playwright` execute strategy
- `regression-test-discipline` ensures defects don't reoccur
- `ai-evals-framework` covers AI-specific testing
- ADR records strategy decisions for the project

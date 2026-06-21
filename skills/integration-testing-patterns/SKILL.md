---
name: integration-testing-patterns
description: Testing how components work together — API endpoints with real database, service integration, contract verification. Catches bugs that unit tests miss because units that each work can still fail when combined. The middle layer of the test pyramid.
---

# Integration Testing Patterns — Where Units Meet

A function works. The database works. But the function calling the database — does that work? Integration tests verify the seams. Most production bugs live in the seams.

## Prerequisites

- `test-strategy-design` and `unit-testing-craft` understood
- Components exist that integrate (API + DB, service + service)
- Test environment for integration (test database, mock external services)

## Core principle

> Unit tests verify pieces in isolation. Integration tests verify the connections. A bug-free set of units can still produce a broken system if the connections are wrong — mismatched contracts, wrong assumptions about each other, transaction boundary errors. Integration tests find those.

## What integration tests cover

- **API endpoint + database:** request comes in, data persists correctly, response correct
- **Service + service:** one service calling another, data flows correctly
- **Code + external API:** integration with third-party (mocked at the boundary)
- **Multiple components in a flow:** auth → authorization → business logic → persistence
- **Transaction boundaries:** multi-step operations commit/rollback correctly
- **Contract adherence:** API matches its OpenAPI spec

## API endpoint integration tests

The most common integration test: hit an endpoint, verify the full round trip.

```typescript
describe('POST /api/users', () => {
  beforeEach(async () => {
    await resetTestDatabase();  // clean slate
  });

  test('creates user and returns 201', async () => {
    const response = await request(app)
      .post('/api/users')
      .send({ email: 'alice@example.com', name: 'Alice' });

    expect(response.status).toBe(201);
    expect(response.body.data.email).toBe('alice@example.com');

    // Verify it actually persisted
    const dbUser = await prisma.user.findUnique({
      where: { email: 'alice@example.com' }
    });
    expect(dbUser).not.toBeNull();
    expect(dbUser.name).toBe('Alice');
  });

  test('rejects duplicate email with 409', async () => {
    await prisma.user.create({
      data: { email: 'alice@example.com', name: 'Existing' }
    });

    const response = await request(app)
      .post('/api/users')
      .send({ email: 'alice@example.com', name: 'Alice' });

    expect(response.status).toBe(409);
  });

  test('rejects invalid email with 400', async () => {
    const response = await request(app)
      .post('/api/users')
      .send({ email: 'not-an-email', name: 'Alice' });

    expect(response.status).toBe(400);
  });
});
```

Note: tests verify BOTH the response AND the database state. The endpoint claims to create a user — verify it actually did.

## Test database strategy

Integration tests need a real database (the integration with DB is the point).

**Recommended for Fosved: Testcontainers + Postgres.**

```typescript
import { PostgreSqlContainer } from '@testcontainers/postgresql';

let container;
let prisma;

beforeAll(async () => {
  container = await new PostgreSqlContainer().start();
  process.env.DATABASE_URL = container.getConnectionUri();
  // Run migrations
  execSync('npx prisma migrate deploy');
  prisma = new PrismaClient();
}, 60000);  // container startup takes time

afterAll(async () => {
  await prisma.$disconnect();
  await container.stop();
});
```

Real Postgres, isolated, disposable. Matches production behavior exactly (unlike SQLite substitute).

**Cleanup between tests:**
```typescript
beforeEach(async () => {
  // Truncate all tables, preserving schema
  await prisma.$executeRaw`TRUNCATE TABLE users, posts, comments CASCADE`;
});
```

Each test starts clean. No order dependence.

## Mocking external services

Your code integrates with external APIs (Anthropic, payment providers, etc.). In integration tests, mock at that boundary.

```typescript
// Mock the LLM provider — deterministic, fast, free
const mockLLM = {
  call: vi.fn().mockResolvedValue({
    text: 'Analysis: the situation is stable.',
    usage: { inputTokens: 100, outputTokens: 50 },
    model: 'mock',
    finishReason: 'stop'
  })
};

test('analyze endpoint creates analysis record', async () => {
  const response = await request(app)
    .post('/api/analyze')
    .send({ topic: 'test topic' })
    .set('x-llm-provider', 'mock');  // inject mock

  expect(response.status).toBe(201);

  const analysis = await prisma.analysis.findFirst({
    where: { topic: 'test topic' }
  });
  expect(analysis).not.toBeNull();
  expect(analysis.fullAnalysis).toContain('stable');
});
```

Why mock external services in integration tests:
- Deterministic (real LLM gives different output each call)
- Fast (no network round trip)
- Free (no API charges)
- Reliable (no dependency on external uptime)

The real external integration is tested separately, sparingly (or via AI evals for LLMs).

## Contract testing

If your API has an OpenAPI spec, verify the implementation matches:

```typescript
import { validateAgainstSchema } from './openapi-validator';

test('GET /api/users matches OpenAPI schema', async () => {
  const response = await request(app).get('/api/users');
  const validation = validateAgainstSchema(
    response.body,
    openApiSpec.paths['/api/users'].get.responses['200']
  );
  expect(validation.valid).toBe(true);
});
```

Catches drift between spec and implementation. Consumers rely on the spec.

## Transaction boundary testing

Multi-step operations must be atomic — all succeed or all roll back.

```typescript
test('transferFunds rolls back fully on failure', async () => {
  const accountA = await createAccount({ balance: 100 });
  const accountB = await createAccount({ balance: 0 });

  // Force failure mid-transaction (e.g., accountB write fails)
  mockFailOnSecondWrite();

  await expect(
    transferFunds(accountA.id, accountB.id, 50)
  ).rejects.toThrow();

  // Verify NEITHER account changed — full rollback
  const a = await getAccount(accountA.id);
  const b = await getAccount(accountB.id);
  expect(a.balance).toBe(100);  // unchanged
  expect(b.balance).toBe(0);    // unchanged
});
```

A partial transaction (money left A but didn't reach B) is a catastrophic bug. Test the rollback.

## Authentication and authorization flow

```typescript
describe('protected endpoint', () => {
  test('rejects request without auth', async () => {
    const response = await request(app).get('/api/profile');
    expect(response.status).toBe(401);
  });

  test('rejects request with invalid token', async () => {
    const response = await request(app)
      .get('/api/profile')
      .set('Authorization', 'Bearer invalid');
    expect(response.status).toBe(401);
  });

  test('allows request with valid token', async () => {
    const token = await getValidTestToken(testUser);
    const response = await request(app)
      .get('/api/profile')
      .set('Authorization', `Bearer ${token}`);
    expect(response.status).toBe(200);
  });

  test('user cannot access another users data', async () => {
    const tokenA = await getValidTestToken(userA);
    const response = await request(app)
      .get(`/api/users/${userB.id}/private`)
      .set('Authorization', `Bearer ${tokenA}`);
    expect(response.status).toBe(403);  // forbidden, not 200
  });
});
```

The last test (cross-user access) is critical — a common security bug is missing authorization checks. Verify users can't reach each other's data.

## Telegram bot integration tests

For Fosved bot, integration tests with mocked Telegram:

```typescript
test('/analyze command creates analysis and replies', async () => {
  const mockBot = createMockBot();
  
  await handleMessage(mockBot, {
    text: '/analyze экономика Беларуси',
    from: { id: OWNER_ID },
    chat: { id: OWNER_ID }
  });

  // Verify analysis created
  const analysis = await prisma.analysis.findFirst({
    orderBy: { createdAt: 'desc' }
  });
  expect(analysis.topic).toContain('Беларуси');

  // Verify bot replied
  expect(mockBot.sendMessage).toHaveBeenCalled();
  const reply = mockBot.sendMessage.mock.calls[0][1];
  expect(reply).toContain('анализ');  // some confirmation
});
```

Tests the handler + DB + reply flow, with Telegram and LLM mocked.

## Multi-component flow tests

For flows spanning multiple components:

```typescript
test('full order flow: create → pay → fulfill', async () => {
  // Create
  const order = await createOrder(testUser.id, testItems);
  expect(order.status).toBe('pending');

  // Pay
  await processPayment(order.id, mockPaymentMethod);
  const afterPay = await getOrder(order.id);
  expect(afterPay.status).toBe('paid');

  // Fulfill
  await fulfillOrder(order.id);
  const afterFulfill = await getOrder(order.id);
  expect(afterFulfill.status).toBe('fulfilled');

  // Verify side effects: inventory decremented, notification sent
  const inventory = await getInventory(testItems[0].id);
  expect(inventory.count).toBe(initialCount - 1);
});
```

This catches bugs where each step works alone but the sequence has issues (wrong status transitions, missing side effects).

## Speed management

Integration tests are slower than unit (real DB, more setup). Keep manageable:

- Target: full integration suite < 5 minutes
- Parallelize where possible (separate DB per worker, or careful isolation)
- Don't put unit-testable logic in integration tests (wrong layer, slow)
- Reuse container across tests (start once in `beforeAll`, not per test)

If suite exceeds budget: profile, find slow tests, optimize.

## What NOT to integration test

- **Pure logic** — that's unit test territory
- **UI rendering** — that's component test or E2E
- **Every input combination** — use unit tests for combinatorial cases
- **Third-party internals** — trust the library, test your integration with it

## Anti-patterns

- **Integration tests as unit tests.** Testing pure logic through the full stack. Slow, wrong layer.
- **No database isolation.** Tests share data, pollute each other, order-dependent.
- **Real external services.** Tests hit real Anthropic API — slow, flaky, costs money, non-deterministic.
- **Testing only happy path.** No 400, 401, 403, 409, 500 cases. Production breaks on errors.
- **Not verifying side effects.** Endpoint returns 201 but didn't actually persist. Test checks response only.
- **Not testing rollback.** Multi-step operations assumed atomic, never verified.
- **Missing authz tests.** Auth tested, but not "user A can't see user B's data".
- **Slow suite.** Integration suite takes 20 minutes. Gets skipped. Protects nothing.
- **Container per test.** Starting fresh container each test. Massively slow. Start once.
- **No cleanup.** Tests leave data, next test sees it, mysterious failures.

## Integration

- Implements `test-strategy-design`'s integration layer
- `unit-testing-craft` covers the layer below
- `e2e-testing-with-playwright` covers the layer above
- `api-design` — integration tests verify the API contract
- `database-design` — integration tests exercise real schema
- `failure-modes-mapping` — failure modes at integration boundaries
- `regression-test-discipline` — integration regressions go here

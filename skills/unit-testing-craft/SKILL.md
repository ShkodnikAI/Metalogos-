---
name: unit-testing-craft
description: Writing good unit tests — fast, isolated, deterministic, readable. Test structure (arrange/act/assert), what makes a test valuable vs noise, mocking discipline, testing edge cases. The craft of the foundational test layer that catches most bugs at the source.
---

# Unit Testing Craft — The Foundational Layer

Unit tests are 70% of the test pyramid. They're fast, cheap, catch bugs at the source. But badly-written unit tests are noise: brittle, slow, testing nothing. This skill is the craft of writing good ones.

## Prerequisites

- `test-strategy-design` understood
- Test framework set up (Vitest for Fosved JS/TS projects)
- Code to test that has unit-testable logic

## Core principle

> A unit test verifies one behavior of one unit in isolation. Fast (milliseconds), deterministic (same result every run), independent (order doesn't matter), readable (failure message tells you what broke). A test missing any of these properties is a liability, not an asset.

## Anatomy of a unit test

**Arrange / Act / Assert** structure:

```typescript
test('calculateTax applies rate to subtotal', () => {
  // Arrange — set up inputs
  const subtotal = 100;
  const taxRate = 0.2;

  // Act — execute the thing under test
  const tax = calculateTax(subtotal, taxRate);

  // Assert — verify the outcome
  expect(tax).toBe(20);
});
```

Three clear sections. Don't interleave. Readability matters — a failing test should be instantly understandable.

## What makes a test valuable

**Tests behavior, not implementation.**

```typescript
// ✗ Bad — tests implementation detail
test('uses reduce internally', () => {
  const spy = jest.spyOn(Array.prototype, 'reduce');
  sumArray([1, 2, 3]);
  expect(spy).toHaveBeenCalled();
});
// Breaks if you refactor to a for-loop. But behavior is identical!

// ✓ Good — tests behavior
test('sums array elements', () => {
  expect(sumArray([1, 2, 3])).toBe(6);
});
// Survives any refactor that preserves behavior.
```

**Tests one thing.**

```typescript
// ✗ Bad — tests many things, unclear what failed
test('user operations work', () => {
  const user = createUser('Alice');
  expect(user.name).toBe('Alice');
  updateUser(user, { name: 'Bob' });
  expect(user.name).toBe('Bob');
  deleteUser(user);
  expect(getUser(user.id)).toBeNull();
});

// ✓ Good — one behavior each, failure is specific
test('createUser sets the name', () => {
  expect(createUser('Alice').name).toBe('Alice');
});
test('updateUser changes the name', () => {
  const user = createUser('Alice');
  updateUser(user, { name: 'Bob' });
  expect(user.name).toBe('Bob');
});
```

**Descriptive name.** The test name describes the behavior. When it fails, the name alone tells you what broke.

- ✗ `test('test1')`, `test('user')`
- ✓ `test('createUser throws when name is empty')`

## Test isolation

Each test independent — no shared state, no order dependence.

```typescript
// ✗ Bad — tests share state
let counter = 0;
test('increments', () => {
  counter++;
  expect(counter).toBe(1);  // passes only if run first
});

// ✓ Good — each test sets up its own state
test('increments from given value', () => {
  const result = increment(0);
  expect(result).toBe(1);
});
```

Use `beforeEach` for fresh setup:
```typescript
let service;
beforeEach(() => {
  service = new UserService(mockDb());  // fresh instance each test
});
```

## Edge cases — the real value

Happy path tests are easy and low-value (the code usually works for the happy path). Edge cases are where bugs hide.

For any function, test:
- **Empty:** empty string, empty array, empty object
- **Null/undefined:** missing values
- **Boundaries:** 0, -1, 1, MAX, MAX+1
- **Single item:** array with one element
- **Many items:** large input
- **Special values:** NaN, Infinity, negative zero
- **Wrong types:** if not TypeScript-protected

```typescript
describe('parseAge', () => {
  test('parses valid age', () => {
    expect(parseAge('25')).toBe(25);
  });
  test('returns null for empty string', () => {
    expect(parseAge('')).toBeNull();
  });
  test('returns null for non-numeric', () => {
    expect(parseAge('abc')).toBeNull();
  });
  test('returns null for negative', () => {
    expect(parseAge('-5')).toBeNull();
  });
  test('handles zero', () => {
    expect(parseAge('0')).toBe(0);
  });
  test('returns null for unreasonably large', () => {
    expect(parseAge('99999')).toBeNull();
  });
});
```

One happy-path test, five edge-case tests. The edge cases earn their keep.

## Error path testing

Test that errors happen when they should:

```typescript
test('createUser throws on empty name', () => {
  expect(() => createUser('')).toThrow('Name required');
});

test('divide throws on zero divisor', () => {
  expect(() => divide(10, 0)).toThrow('Division by zero');
});

// Async errors
test('fetchUser rejects on invalid id', async () => {
  await expect(fetchUser('invalid')).rejects.toThrow('User not found');
});
```

Untested error paths break in production on first real error.

## Mocking discipline

**Mock at boundaries** — external dependencies that make tests slow or non-deterministic.

```typescript
// Mock the database for unit testing business logic
test('UserService.register hashes password', async () => {
  const mockDb = {
    user: { create: vi.fn().mockResolvedValue({ id: '1' }) }
  };
  const service = new UserService(mockDb);

  await service.register('alice@example.com', 'password123');

  // Verify password was hashed before storing
  const createCall = mockDb.user.create.mock.calls[0][0];
  expect(createCall.data.password).not.toBe('password123');  // hashed
});
```

**Don't over-mock.** If everything is mocked, you test mocks, not code.

```typescript
// ✗ Over-mocked — tests nothing real
test('processOrder works', () => {
  const mockCalculate = vi.fn().mockReturnValue(100);
  const mockValidate = vi.fn().mockReturnValue(true);
  const mockSave = vi.fn();
  // ... the actual processOrder logic is entirely mocked away
  // What is this even testing?
});
```

Mock external/slow/non-deterministic things. Test real logic.

## Deterministic tests

Tests must produce the same result every run.

**Non-determinism sources to control:**

```typescript
// Time — inject or mock
test('isExpired with fixed time', () => {
  const now = new Date('2026-01-01');
  expect(isExpired(token, now)).toBe(false);
});

// Randomness — seed or mock
test('with seeded random', () => {
  const rng = seededRandom(42);
  expect(pickItem(items, rng)).toBe(items[2]);  // deterministic
});

// Order — don't rely on object key order, array order from sets, etc.
```

A test that passes sometimes and fails sometimes (flaky) is worse than no test — it erodes trust in the whole suite.

## Test data builders

For complex objects, use factory functions:

```typescript
function buildUser(overrides = {}) {
  return {
    id: 'test-id',
    email: 'test@example.com',
    name: 'Test User',
    role: 'member',
    createdAt: new Date('2026-01-01'),
    ...overrides
  };
}

// Tests specify only what matters
test('admin can delete', () => {
  const admin = buildUser({ role: 'admin' });
  expect(canDelete(admin)).toBe(true);
});

test('member cannot delete', () => {
  const member = buildUser({ role: 'member' });
  expect(canDelete(member)).toBe(false);
});
```

Tests stay focused on the relevant field. Builders absorb the boilerplate.

## Parameterized tests

For testing many inputs of same behavior:

```typescript
test.each([
  [0, 'zero'],
  [1, 'one'],
  [-1, 'negative'],
  [100, 'large'],
])('classifyNumber(%i) returns %s', (input, expected) => {
  expect(classifyNumber(input)).toBe(expected);
});
```

Concise, each case independently reported.

## Testing pure vs impure functions

**Pure functions** (same input → same output, no side effects) are trivially testable:
```typescript
expect(add(2, 3)).toBe(5);  // that's it
```

**Impure functions** (side effects, depend on external state) are harder. Strategy:
- Extract pure logic, test that thoroughly
- Test the thin impure shell with mocks
- Push impurity to the edges

This is why "functional core, imperative shell" architecture is testable.

## Coverage interpretation

```bash
npm run test -- --coverage
```

Coverage shows which lines/branches ran during tests.

**Use coverage to find gaps:** "this error-handling branch has 0 coverage — add a test."

**Don't game coverage:** writing tests that execute lines without asserting anything just inflates the number.

**Branch coverage > line coverage:** a line can run without all its branches tested.

```typescript
function classify(n) {
  return n > 0 ? 'positive' : 'non-positive';  // one line, two branches
}
// Test with n=5 → line covered 100%, but branch n<=0 untested
```

## What good unit test suite feels like

- Runs in seconds (whole unit suite < 30s)
- Failure message points directly at the problem
- Adding a feature → obvious where to add tests
- Refactoring → tests still pass (behavior preserved)
- Reading tests → understand what the code is supposed to do
- High confidence: green suite means it probably works

## Anti-patterns

- **Testing implementation.** Spies on internal calls. Breaks on refactor.
- **Giant tests.** One test, 50 assertions. Failure unclear.
- **Shared mutable state.** Tests affect each other. Order-dependent.
- **Flaky tests.** Pass sometimes. Erodes trust.
- **Over-mocking.** Everything mocked, real code not exercised.
- **Happy-path only.** No edge cases, no error paths. Misses real bugs.
- **Vague names.** `test('works')`. Failure tells you nothing.
- **Assertion-free tests.** Runs code, asserts nothing. Coverage theater.
- **Slow unit tests.** Real DB, real network in "unit" tests. Suite becomes too slow to run.
- **Testing the framework.** Testing that React renders, that Prisma queries. Trust the framework.
- **Snapshot overuse.** Snapshot everything. Snapshots rot, get blindly updated.
- **No edge cases.** Only testing inputs the developer expects.

## Integration

- Implements `test-strategy-design`'s unit layer
- `failure-modes-mapping` provides the edge cases to test
- `defect-discipline` — defects generate unit-level regression tests where applicable
- `regression-test-discipline` — unit tests form bulk of regression suite
- `code-organization-standards` — tests in `tests/unit/` or co-located
- Runs constantly during `iterative-implementation`

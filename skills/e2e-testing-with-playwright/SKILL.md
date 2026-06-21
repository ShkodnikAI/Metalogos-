---
name: e2e-testing-with-playwright
description: End-to-end testing of critical user journeys with Playwright. Testing the full system as a user experiences it — browser, UI, backend, database. Few but high-value tests. Stable selector strategy, handling async, avoiding flakiness. The top of the test pyramid.
---

# E2E Testing with Playwright — Verify the Whole Journey

E2E tests do what a user does: open a browser, click, type, navigate, verify. They catch bugs invisible to unit and integration tests — bugs that only appear when the whole system runs together. They're slow and can be flaky, so: few, high-value, well-crafted.

## Prerequisites

- `test-strategy-design` understood (E2E is ~10% of pyramid)
- Critical user journeys identified
- Playwright installed (Fosved default E2E tool)

## Core principle

> E2E tests are expensive (slow, flaky-prone, maintenance-heavy) so each one must earn its place by covering a critical journey that lower layers can't verify. Don't E2E everything — E2E the handful of journeys where "the whole thing must work" matters most.

## What to E2E test

**Critical user journeys** — the few flows that, if broken, the product is broken:

- Sign up / log in
- The primary task (whatever the product is FOR)
- Payment / checkout (if applicable)
- Critical multi-step flows

For Fosved bot: a few golden conversations end-to-end (e.g., `/analyze` → analysis appears → archived).

For fosved-miniapp: load app → navigate → view data → core interaction.

**Don't E2E test:**
- Every form field validation (unit/integration)
- Every UI variant (combinatorial explosion)
- Edge cases (lower layers)
- Error messages wording (lower layers)

Rule of thumb: 5-15 E2E tests for a typical project. If you have 100, you've over-invested.

## Playwright basics

```typescript
import { test, expect } from '@playwright/test';

test('user can log in and see dashboard', async ({ page }) => {
  await page.goto('/login');

  await page.getByLabel('Email').fill('test@example.com');
  await page.getByLabel('Password').fill('password123');
  await page.getByRole('button', { name: 'Log in' }).click();

  // Verify navigation happened and dashboard loaded
  await expect(page).toHaveURL('/dashboard');
  await expect(page.getByRole('heading', { name: 'Dashboard' })).toBeVisible();
});
```

Playwright auto-waits for elements — no manual sleeps needed (usually).

## Selector strategy — the key to non-flaky tests

The #1 cause of flaky E2E tests: brittle selectors.

**Selector priority (best to worst):**

1. **Role-based** — `getByRole('button', { name: 'Submit' })` — accessible, stable, semantic
2. **Label-based** — `getByLabel('Email')` — stable, accessibility-aligned
3. **Text-based** — `getByText('Welcome')` — stable if text stable
4. **Test ID** — `getByTestId('submit-button')` — stable but requires adding `data-testid` attributes
5. **CSS/XPath** — `page.locator('.btn-primary')` — BRITTLE, breaks on style changes

Prefer 1-2. Use 4 when 1-3 don't work. Avoid 5.

```typescript
// ✗ Brittle — breaks when CSS class changes
await page.locator('.css-1a2b3c > div:nth-child(2) > button').click();

// ✓ Stable — survives style refactors
await page.getByRole('button', { name: 'Save' }).click();
```

**Adding test IDs when needed:**
```tsx
<button data-testid="submit-order">Place Order</button>
```
```typescript
await page.getByTestId('submit-order').click();
```

Bonus: role-based selectors double as accessibility verification. If `getByRole('button')` can't find your button, neither can a screen reader.

## Handling async / waiting

Playwright auto-waits for elements to be actionable. But sometimes you need explicit waits:

```typescript
// Auto-wait (preferred) — Playwright waits for element
await page.getByRole('button', { name: 'Load' }).click();
await expect(page.getByText('Results')).toBeVisible();  // waits up to timeout

// Wait for specific condition
await page.waitForURL('/success');
await page.waitForLoadState('networkidle');

// Wait for API response
await page.waitForResponse(resp => 
  resp.url().includes('/api/data') && resp.status() === 200
);
```

**Never use fixed sleeps:**
```typescript
// ✗ Bad — flaky (too short fails, too long slows suite)
await page.waitForTimeout(3000);

// ✓ Good — wait for the actual condition
await expect(page.getByText('Loaded')).toBeVisible();
```

Fixed sleeps are the #2 cause of flaky tests.

## Test structure

```typescript
import { test, expect } from '@playwright/test';

test.describe('Order flow', () => {
  test.beforeEach(async ({ page }) => {
    // Set up: log in, seed state
    await loginAs(page, 'test@example.com');
  });

  test('completes full order journey', async ({ page }) => {
    // 1. Browse to product
    await page.goto('/products');
    await page.getByText('Test Product').click();

    // 2. Add to cart
    await page.getByRole('button', { name: 'Add to Cart' }).click();
    await expect(page.getByTestId('cart-count')).toHaveText('1');

    // 3. Checkout
    await page.getByRole('link', { name: 'Checkout' }).click();
    await page.getByLabel('Card number').fill('4242424242424242');
    await page.getByRole('button', { name: 'Place Order' }).click();

    // 4. Verify success
    await expect(page.getByText('Order confirmed')).toBeVisible();
    await expect(page).toHaveURL(/\/orders\/\d+/);
  });
});
```

## Test isolation

Each E2E test independent:
- Fresh browser context (Playwright does this per test by default)
- Independent test data (don't depend on previous test's state)
- Can run in any order
- Can run in parallel

```typescript
// Each test gets its own data
test('user A sees their orders', async ({ page }) => {
  const userA = await createTestUser();  // unique user
  await seedOrders(userA, 3);
  // ... test
});
```

## Test data for E2E

Options:
- **Seed via API before test:** fast, reliable
- **Seed via DB directly:** fastest, requires DB access
- **Create through UI:** slow, but tests creation flow too

For setup (not the thing under test): seed via API/DB. For the journey under test: use the UI.

```typescript
test.beforeEach(async ({ request }) => {
  // Seed via API — fast setup
  await request.post('/api/test/seed', { data: { users: 1, products: 5 } });
});
```

**Reset between tests:** clean database or use unique data per test.

## Cross-browser testing

Playwright runs tests across browsers:

```typescript
// playwright.config.ts
export default {
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'firefox', use: { ...devices['Desktop Firefox'] } },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } },
    { name: 'mobile', use: { ...devices['iPhone 13'] } }
  ]
};
```

Critical journeys tested on all target browsers. Catches browser-specific bugs (Safari especially).

## Visual regression (optional)

Playwright can screenshot and compare:

```typescript
test('dashboard looks correct', async ({ page }) => {
  await page.goto('/dashboard');
  await expect(page).toHaveScreenshot('dashboard.png');
});
```

First run captures baseline. Future runs compare. Catches unintended visual changes.

Caveat: visual tests are flaky-prone (fonts, rendering differences). Use sparingly, for key pages.

## Debugging failing E2E tests

Playwright tools:
```bash
npx playwright test --debug        # step through interactively
npx playwright test --headed       # see the browser
npx playwright show-trace trace.zip  # post-mortem trace viewer
```

On CI failure, Playwright captures:
- Screenshot at failure point
- Video of the run
- Trace (full timeline, network, DOM snapshots)

Configure:
```typescript
use: {
  screenshot: 'only-on-failure',
  video: 'retain-on-failure',
  trace: 'retain-on-failure'
}
```

These artifacts make CI failures debuggable without reproducing locally.

## Flakiness — the E2E enemy

Flaky E2E tests (pass sometimes, fail sometimes) destroy trust. Sources and fixes:

| Flaky source | Fix |
|--------------|-----|
| Brittle CSS selectors | Use role/label/testid selectors |
| Fixed sleeps | Wait for conditions |
| Race conditions | Wait for specific state, not time |
| Shared test data | Unique data per test |
| Animations | Disable animations in test config |
| Network timing | Wait for response, mock if needed |
| Test order dependence | Full isolation |

```typescript
// Disable animations for stability
use: {
  // in config
}
// Or per-test:
await page.addStyleTag({ content: '* { animation: none !important; transition: none !important; }' });
```

**Zero tolerance for flaky tests in main suite.** Quarantine and fix, or remove.

## CI integration

```yaml
- name: Install Playwright
  run: npx playwright install --with-deps
- name: Run E2E tests
  run: npx playwright test
- name: Upload artifacts on failure
  if: failure()
  uses: actions/upload-artifact@v4
  with:
    name: playwright-report
    path: playwright-report/
```

E2E runs on PR + main. Slower than unit/integration — that's expected.

## Speed management

E2E is slow. Keep suite manageable:
- Target: full E2E suite < 15 minutes
- Run in parallel (Playwright parallelizes by default)
- Shard across CI machines for large suites
- Only critical journeys (don't E2E everything)

## Anti-patterns

- **E2E everything.** 200 E2E tests. Suite takes an hour. Flaky. Maintenance nightmare.
- **Brittle selectors.** CSS-class selectors break on every style change.
- **Fixed sleeps.** `waitForTimeout(3000)`. Flaky and slow.
- **Test interdependence.** Test B depends on Test A running first.
- **Shared test data.** Tests pollute each other's data.
- **No failure artifacts.** CI fails, no screenshot/video/trace. Undebuggable.
- **Flaky tests tolerated.** "Just re-run it." Trust erodes, real failures ignored.
- **Testing edge cases in E2E.** Slow way to test what unit tests cover fast.
- **No cross-browser.** Only Chrome tested. Safari users hit bugs.
- **E2E against production.** Tests modify real production data. Use staging/test env.
- **Giant E2E tests.** One test does 30 steps. When it fails, unclear which step.

## Integration

- Implements `test-strategy-design`'s E2E layer (the top, ~10%)
- `unit-testing-craft` and `integration-testing-patterns` cover lower layers
- `accessibility-first` (Design) — role-based selectors verify accessibility
- `regression-test-discipline` — E2E regressions for critical journey bugs
- `failure-modes-mapping` — journey-level failure modes
- Runs in CI, artifacts captured on failure

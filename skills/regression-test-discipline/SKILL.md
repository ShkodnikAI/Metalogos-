---
name: regression-test-discipline
description: Ensuring that fixed bugs stay fixed and working features keep working. Every defect generates a regression test. Test suites grow as a ratchet — coverage only increases. The discipline that prevents the exhausting cycle of bugs returning after they were "fixed".
---

# Regression Test Discipline — Fixed Stays Fixed

The most demoralizing bug is the one you already fixed, returned. Someone refactored, a dependency updated, a merge went wrong — and the old bug is back. Regression discipline prevents this: every fix is protected by a test that fails if the bug returns.

## Prerequisites

- `defect-discipline` understood
- `test-strategy-design` in place
- Test suite exists and runs in CI

## Core principle

> A test suite is a ratchet — it should only tighten. Every bug fixed adds a test. Every feature shipped adds tests. Coverage never decreases. The suite becomes the accumulated, executable memory of "everything that must keep working." Refactor freely; the ratchet catches regressions.

## What is a regression

**Regression:** something that worked before now doesn't.

Causes:
- Refactoring changed behavior unintentionally
- New feature broke existing feature
- Dependency update changed behavior
- Merge combined incompatible changes
- Configuration change had side effects
- A fix for bug A caused bug B

Regressions are insidious because the broken thing wasn't being worked on — nobody's looking at it.

## The regression test rule

**Every confirmed defect produces a regression test.** From `defect-discipline`, restated because it's central:

```
Bug confirmed → fix written → regression test written → committed together
```

The test:
1. Reproduces the bug (verify: fails without the fix)
2. Passes with the fix
3. Lives in the suite permanently
4. Named/tagged with the defect ID

```typescript
test('order total includes tax (regression DEFECT-103)', () => {
  const order = createOrder({ subtotal: 100, taxRate: 0.2 });
  expect(order.total).toBe(120);  // bug was: returned 100, tax forgotten
});
```

If someone later refactors order calculation and drops tax — this test fails. Bug caught before production.

## Verify the test actually catches the bug

Critical step often skipped: **confirm the regression test fails without the fix.**

```
1. Write the regression test
2. Revert the fix (temporarily)
3. Run test → it MUST fail
4. Re-apply fix
5. Run test → it MUST pass
```

If the test passes even without the fix — it doesn't actually test the bug. Worthless. Rewrite.

This verification proves the test has teeth.

## Feature regression tests

Beyond bug fixes, features get regression protection too.

When a feature ships:
- Core behavior covered by tests
- These tests now guard the feature against future regressions
- Refactoring elsewhere can't silently break this feature

The full test suite IS the regression suite — every test guards against regression of whatever it tests.

## Suite as ratchet

**The ratchet principle:** coverage only goes up.

- New code → new tests (don't merge untested code)
- Bug fixed → regression test added
- Feature added → feature tests added
- Refactor → tests still pass (or updated if behavior intentionally changed)

**Coverage going down is a red flag.** If a PR reduces coverage, scrutinize: did tests get deleted? Was code added without tests?

CI can enforce: fail build if coverage drops below threshold or below previous.

## When tests must change

Sometimes behavior changes intentionally — the old test is now wrong.

**Legitimate test changes:**
- Requirement changed → test updated to new requirement
- Feature removed → its tests removed
- Behavior intentionally different → test reflects new behavior

**Discipline:** changing a test requires same scrutiny as changing code. In PR review, a changed test should have a clear reason. "Why did this test change?" must have a good answer.

**Red flag:** test changed to make it pass, because the code broke it. That's "fixing the test" instead of "fixing the code" — backwards. The test was protecting something; understand what before changing.

## Regression test organization

Tag/name regression tests with their origin:

```typescript
// Group regression tests or tag them
describe('regressions', () => {
  test('DEFECT-042: special chars in password', () => { ... });
  test('DEFECT-103: order total includes tax', () => { ... });
  test('DEFECT-118: empty search returns all not error', () => { ... });
});
```

Or inline with feature tests but clearly marked. Either works — consistency matters.

Benefit of marking: when investigating, you can see "this area has had 5 regressions" — a hotspot signal.

## Running regression tests

Regression tests run with the rest of the suite:

- **Unit-level regressions:** in unit suite, run constantly (every save)
- **Integration regressions:** in integration suite, run pre-push
- **E2E regressions:** in E2E suite, run in CI

**The full suite runs in CI on every PR.** This is what catches regressions before merge.

If suite is too slow and gets skipped → regressions slip through. Keep suite fast (see `test-strategy-design` speed budget).

## Regression test for AI systems

AI systems regress differently — a prompt change can break previously-working behavior.

**Prompt regression suite:** golden dataset of inputs with expected output properties.

```
Before changing a prompt:
1. Run golden dataset through current prompt → baseline outputs
2. Change prompt
3. Run golden dataset through new prompt → new outputs
4. Compare: did anything that worked now fail?
```

See `ai-evals-framework` for the full mechanism. The principle is the same: changes shouldn't break what worked.

For Fosved: the bot itself has prompt regression tests. Changing OSP's prompt → run OSP golden dataset → verify no regression.

## Bisecting regressions

When a regression is found but not which change caused it:

`git bisect` — binary search through commits:
```bash
git bisect start
git bisect bad           # current commit has the bug
git bisect good v1.2.0   # this old version was fine
# git checks out midpoint commits, you test each
git bisect good/bad      # mark each
# git narrows to the exact commit that introduced regression
```

Fast way to find the culprit commit. Then understand why that change caused it.

This works well BECAUSE you have tests — you can mechanically verify each commit.

## The cost of no regression discipline

Without it:
- Bugs return, get re-reported, re-investigated, re-fixed — wasted effort multiplied
- Refactoring is scary (might break something, no safety net) → code rots
- Confidence low → releases slow, manual testing marathons
- Same bug fixed 3 times across a year

With it:
- Bugs fixed once, stay fixed
- Refactoring is safe (suite catches breaks)
- Confidence high → ship faster
- Effort compounds instead of repeating

## Flaky regression tests

A regression test that sometimes passes, sometimes fails — worse than no test (erodes trust in suite).

Causes:
- Timing/race conditions in test
- Shared state between tests
- External dependency in test (network, time, randomness)
- Order dependence

**Fix flaky tests immediately.** Options:
- Make deterministic (mock time, seed randomness, isolate state)
- Fix the actual race condition (if test reveals real bug)
- If truly can't stabilize: quarantine (separate suite, doesn't block) and fix soon

Don't tolerate flaky tests in main suite. One flaky test makes people ignore failures → real regression slips through.

## Anti-patterns

- **Fix without regression test.** Bug WILL return eventually. No protection.
- **Test that doesn't catch the bug.** Not verified to fail without the fix. False protection.
- **Deleting tests to make CI green.** Removing the alarm instead of fixing the fire.
- **Changing tests to pass.** Code broke test → test "fixed" to match broken code. Backwards.
- **Coverage decreasing.** Ratchet slipping. Code added without tests, or tests removed.
- **Flaky tests tolerated.** Erodes trust. Real failures ignored as "probably flaky".
- **Slow suite skipped.** Regression tests that don't run protect nothing.
- **No CI gate.** Suite exists but doesn't block merge. Regressions merge freely.
- **Regression tests not marked.** Can't see hotspots, can't tell regression tests from feature tests.
- **AI prompt changes without regression check.** Prompt "improved", silently broke 5 other cases.

## Integration

- `defect-discipline` — every defect generates a regression test (the rule lives in both)
- `test-strategy-design` — regression tests are part of the suite strategy
- `unit-testing-craft` / `integration-testing-patterns` / `e2e-testing-with-playwright` — where regression tests are implemented
- `ai-evals-framework` — regression for AI systems (prompt regression)
- CI enforces suite passing + coverage ratchet
- `git bisect` for finding regression-causing commits

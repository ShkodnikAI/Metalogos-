---
name: defect-discipline
description: How defects are recorded, classified, prioritized, root-caused, and closed. Each defect is data — about where quality processes failed and how to improve them. Without discipline, defects are forgotten, recur, or fixed without understanding. With discipline, the defect log becomes a learning instrument.
---

# Defect Discipline — Every Bug is Data

A bug found and quietly fixed is a missed lesson. A bug found, recorded, root-caused, and prevented from recurring is process improvement. Defect discipline turns the bug stream into a learning instrument.

## Prerequisites

- A defect discovered (via testing, monitoring, or user report)
- `DefectRecord` model available
- Mindset: defects are information, not shame

## Core principle

> A defect is not just a thing to fix — it's evidence about where your quality process has a gap. The fix closes the symptom. The discipline asks "how did this get here? what process would have caught it? how do we prevent the class?" That's where compounding improvement comes from.

## Defect lifecycle

```
discovered → recorded → classified → prioritized → 
  assigned → fixed → verified → regression-tested → closed
```

Every defect walks this. No shortcuts (a "quick fix" without recording is a lost lesson).

## Recording a defect

When defect found, immediately record (don't rely on memory):

```typescript
{
  title: 'Login fails on Safari when password contains special chars',
  description: 'Detailed explanation...',
  severity: 'high',
  category: 'functional',
  stepsToReproduce: '1. Open app in Safari\n2. Enter password with & character\n3. Click login\n4. Observe 500 error',
  expectedBehavior: 'Login succeeds, special chars handled',
  actualBehavior: '500 Internal Server Error, login fails',
  environment: 'Safari 17.2, macOS 14.3',
  status: 'open'
}
```

**Good defect record:**
- Title is specific (not "login broken")
- Reproduction steps exact (someone else can follow)
- Expected vs actual explicit
- Environment captured

**Bad defect record:**
- "Doesn't work" — what doesn't? when?
- No reproduction — can't verify fix
- No environment — maybe it's browser-specific

## Severity classification

**Critical:** system unusable, data loss, security breach, blocks all users.
- Drop everything. Fix now.
- Example: login broken for everyone, payment double-charges, data deleted.

**High:** major feature broken, blocks significant use case, affects many users.
- Fix this cycle, prioritize.
- Example: search returns nothing, can't save work, broken on a major browser.

**Medium:** feature impaired but workaround exists, affects some users.
- Fix soon, schedule.
- Example: sorting wrong, minor visual glitch affecting comprehension.

**Low:** minor issue, cosmetic, edge case.
- Fix opportunistically.
- Example: typo, slight misalignment, rare edge case.

**Trivial:** barely noticeable, debatable if bug.
- Fix if convenient, may defer indefinitely.

**Severity ≠ priority.** Severity is impact. Priority factors in frequency, visibility, effort. A low-severity bug on the homepage seen by everyone might get priority over high-severity bug in rarely-used admin panel.

## Category classification

- **functional** — feature doesn't work as intended
- **security** — vulnerability, exposure
- **performance** — too slow, resource heavy
- **accessibility** — fails for assistive tech users
- **ai_quality** — LLM output wrong, hallucination, format violation
- **ui** — visual, layout, rendering
- **data** — corruption, loss, inconsistency

Category helps spot patterns ("we have lots of ai_quality defects — the prompt engineering needs work").

## Reproduction is mandatory

A defect you can't reproduce is a defect you can't verify fixed.

**If can't reproduce:**
- Gather more info (logs, exact environment, user's exact steps)
- Add logging to capture next occurrence
- Mark as "needs-info", don't close
- Don't claim "fixed" — you don't know

**Heisenbug (disappears when observed):**
- Often concurrency or timing
- Add extensive logging
- Stress test to trigger
- Don't dismiss as "ghost"

## Root cause analysis

The fix addresses the symptom. RCA addresses the cause.

**5 Whys technique:**
```
Defect: Login fails on Safari with special char passwords.
Why? → Password not URL-decoded properly server-side.
Why? → Used wrong decoding function.
Why? → Copied pattern from another endpoint that had different encoding.
Why? → No shared utility for password handling, each endpoint rolls own.
Why? → No code review caught the inconsistency.

Root cause: no shared password-handling utility + review gap.
```

The fix: decode properly here.
The RCA-driven improvement: create shared utility, add review checklist item.

Symptom fix prevents THIS bug. RCA prevents the CLASS of bug.

**Root cause categories** (record in `rootCauseCategory`):
- **bug** — straightforward coding error
- **spec_mismatch** — code correct but spec was wrong/unclear
- **test_gap** — should have been caught by test that didn't exist
- **dependency** — third-party issue
- **config** — environment/configuration issue
- **infrastructure** — platform/deployment issue

Categories aggregate into patterns. Lots of `test_gap` → testing strategy needs work. Lots of `spec_mismatch` → design handoff needs work.

## The regression test rule

**Every confirmed defect gets a regression test.** Non-negotiable.

```
Defect found → fix written → regression test written → both committed together
```

The regression test:
- Reproduces the original bug (fails before fix)
- Passes after fix
- Stays in suite forever
- Prevents this exact bug from returning

```typescript
// regression test for DEFECT-042
test('login handles special characters in password (regression DEFECT-042)', async () => {
  const res = await login({ 
    email: 'user@example.com', 
    password: 'p@ss&w0rd!' 
  });
  expect(res.status).toBe(200);
});
```

Without regression test: bug fixed, then 6 months later someone refactors, bug returns, nobody notices. With regression test: refactor breaks test, caught immediately.

Record: `regressionTestAdded: true`, `regressionTestRef: 'tests/auth.test.ts:DEFECT-042'`.

## Verification

After fix:
1. Verify the fix works (the reported scenario now succeeds)
2. Verify regression test passes
3. Verify fix didn't break adjacent functionality (run related tests)
4. Verify in environment where bug was found (if browser-specific, test that browser)

Only then: `status: verified`, then `closed`.

**Don't close on "should be fixed".** Verify.

## Production escape tracking

Critical metric: did this defect reach production?

```typescript
{
  reachedProduction: true,        // it shipped
  detectedInProduction: true,     // found by monitoring/users, not pre-release
  productionImpact: 'Affected ~30 Safari users over 2 days before fix'
}
```

**Defect escape rate** = defects detected in production / total defects.

Target ≤ 10%. If higher, pre-production testing has gaps.

Each production escape is a serious lesson: which test level should have caught this? Strengthen there.

## Defect triage

When multiple defects open, triage regularly:

- **Critical:** immediate, interrupt current work
- **High:** this cycle
- **Medium:** scheduled, next cycle or two
- **Low/Trivial:** backlog, opportunistic

Re-triage periodically — a "low" defect affecting growing user base may rise.

## Defect patterns (monthly review)

Monthly, analyze the defect log:

- **By category:** which categories dominate? (lots of `ai_quality` → prompt work needed)
- **By root cause:** which root causes recur? (lots of `test_gap` → strategy weak)
- **By area:** which part of codebase generates most defects? (hotspot — maybe needs refactor)
- **By severity trend:** are critical defects increasing or decreasing?
- **Escape rate trend:** improving or worsening?

This analysis drives process improvement. The defect log is the instrument.

## When NOT to fix

Not every defect should be fixed:

- **Trivial + expensive to fix:** cost exceeds benefit. Mark `wontfix` with rationale.
- **Edge case that can't actually occur:** theoretical, no real path. Document and close.
- **Working as designed:** reporter misunderstood. Clarify, close as `not-a-bug`.
- **Duplicate:** link to original, close.

`wontfix` is a legitimate decision IF documented with rationale. Silent ignoring is not.

## AI-specific defects

LLM systems generate unique defects:

- **Hallucination:** model stated false fact. Record exact prompt + wrong output. Root cause: prompt? model? missing grounding?
- **Format violation:** model broke expected JSON structure. Record. Root cause: prompt unclear? need stricter parsing?
- **Prompt injection success:** user input manipulated behavior. Critical severity. Record attack vector.
- **Refusal of valid request:** model wrongly refused. Record. Tune prompt.
- **Cost anomaly:** unexpected token explosion. Record. Find the loop.

These go in `DefectRecord` with `category: 'ai_quality'`. Often the "fix" is a prompt change — which then needs `ai-evals-framework` regression testing.

## Anti-patterns

- **Fix without record.** Quick fix, no DefectRecord. Lesson lost. Pattern invisible.
- **No reproduction steps.** Can't verify fix. Can't tell if it recurs.
- **No root cause.** Symptom patched, class of bug remains. Recurs in new form.
- **No regression test.** Bug fixed, then returns months later unnoticed.
- **Close on "should work".** Not verified. Maybe still broken.
- **Severity inflation.** Everything "critical". Real critical lost in noise.
- **Severity deflation.** Real critical called "medium" to avoid interrupting work. Production suffers.
- **Defect log as graveyard.** Defects recorded, never analyzed. No learning.
- **Blame culture.** Defects treated as personal failure. People hide bugs. Worse outcomes.
- **Ignoring escape rate.** Not tracking what reached production. Can't improve pre-release.

## Defect record completeness checklist

Before closing any defect:
- [ ] Title specific
- [ ] Reproduction steps exact
- [ ] Expected vs actual clear
- [ ] Severity + category assigned
- [ ] Root cause identified + categorized
- [ ] Fix verified in original scenario
- [ ] Regression test added and passing
- [ ] Production escape status recorded
- [ ] If wontfix: rationale documented

## Integration

- `DefectRecord` model stores all defects
- `failure-modes-mapping` — was this defect's mode in the map? If not, update mapping.
- `regression-test-discipline` — the regression test process
- `test-strategy-design` — escape patterns inform strategy
- Monthly QA review analyzes defect patterns
- Critical defects auto-create Dev hotfix tasks (cross-department hook)
- `ai-evals-framework` — AI defects feed eval golden dataset

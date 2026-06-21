---
name: estimation-discipline
description: How to produce honest estimates as ranges (low/expected/high), never as point estimates. Point estimates are systematically wrong because they collapse uncertainty into false precision. Range estimates with confidence levels enable proper planning, expose unknowns, and calibrate over time through estimate-vs-actual tracking. Foundation for compound learning about engineering velocity.
---

# Estimation Discipline — Honest Ranges, Not False Precision

"How long will this take?" is the most common engineering question, asked daily. Most answers are wrong because the question forces a single number when reality is a distribution.

The discipline: always answer as range (low / expected / high) with confidence level. Track estimate vs actual. Calibrate monthly. Become reliably honest about uncertainty over time.

## Prerequisites

- DevTask defined with clear scope
- Similar past tasks in archive (for anchoring) — even 5-10 past tasks help

## Core principle

> A point estimate of "3 hours" is a lie because it pretends certainty that doesn't exist. "1-3-8 hours, medium confidence" tells the truth: most likely 3, could be as fast as 1 if everything goes smooth, could blow up to 8 if unknowns hit. The truth supports planning. The lie destroys trust when reality hits.

## The range estimation method

For every non-trivial task, produce **three numbers + confidence**:

- **Low** — optimistic case. Everything works first try. No surprises. Familiar territory. ~10th percentile.
- **Expected** — realistic case. Some friction. Typical surprises. The "most likely" outcome. ~50th percentile.
- **High** — pessimistic case. Multiple surprises. Unknown unknowns hit. ~90th percentile.
- **Confidence** — how sure are you the range is correct? low / medium / high

Example for "add user search to dashboard":
- Low: 2 hours
- Expected: 6 hours
- High: 18 hours
- Confidence: medium

The High-to-Low ratio (18/2 = 9x) is the **uncertainty signal**. Big ratio = lots of unknowns. Small ratio (2x or less) = well-understood task.

## What the confidence level means

- **High confidence:** done this exact thing 3+ times, anchored on actuals. Range narrow (2-3x ratio).
- **Medium confidence:** similar tasks done before, but this has new wrinkles. Range moderate (3-6x ratio).
- **Low confidence:** novel territory, significant unknowns. Range wide (6-10x+ ratio).

If you can't honestly state confidence, you can't estimate. Break the task down further until you can.

## Why point estimates fail

Cognitive science is clear: humans systematically underestimate task duration by 30-50% on average ("planning fallacy", Kahneman). Point estimates anchor everyone on the optimistic number and then reality breaks the plan.

Range estimates work because:
- They make uncertainty **visible** to the requester
- They protect against overcommitment
- They enable better resource allocation (don't promise on optimistic numbers)
- They produce calibration data (was the expected accurate? was the high actually hit?)

## Anchoring on past tasks

Best estimate input: actual durations of similar past tasks.

Query the archive:
```javascript
const similar = await prisma.devTask.findMany({
  where: {
    taskType: currentTaskType,
    status: 'archived',
    actualMin: { not: null }
  },
  orderBy: { completedAt: 'desc' },
  take: 10
});

// Now you have 10 actuals. Compute:
// - median (Expected anchor)
// - 10th percentile (Low anchor)
// - 90th percentile (High anchor)
// Adjust for task-specific factors.
```

If <5 similar tasks exist: confidence is at best **medium**. If <2: confidence is **low** regardless.

## Decomposition before estimation

If a task feels like ">1 day", decompose it. Estimating large tasks accurately is impossible — the unknowns multiply.

Rule: any task with expected >8 hours should be broken into 2-5 subtasks. Estimate each. Sum the ranges (not the points — see below).

## Summing range estimates correctly

When you have subtasks with ranges, naive sum overestimates because not all worst-cases hit at once.

Correct method:
- **Sum the Expected values** — this is the most likely total
- **Sum the Lows** for Low total (still optimistic, but bounded)
- **For High total: NOT sum of Highs. Use sqrt(sum of squared deviations)** — variance adds, not standard deviation

Simple approximation:
```
total_low = sum(lows)
total_expected = sum(expecteds)
total_high = total_expected + sqrt(sum((high_i - expected_i)^2))
```

Or even simpler heuristic: `total_high = total_expected * 1.5 + max(high_i - expected_i)`.

Don't `sum(highs)` — that's worst-case for every subtask which never happens.

## Reasons High beats Expected (track these)

When actual exceeds Expected (lands in Low-to-High range or beyond), categorize why. Common reasons:

- **Unknown technical surface** — encountered API/lib quirk not anticipated
- **Tool friction** — env setup, dependency conflict, CI flake
- **Spec ambiguity** — had to clarify requirements mid-task
- **Side effects** — fixing one thing broke another
- **Testing exposed issues** — implementation needed redo
- **Estimation oversight** — task complexity wasn't grasped initially
- **External delay** — waiting for review, deployment, third party

Tagging by reason builds calibration. Next time you spot "spec might be ambiguous", add to estimate.

## Actuals tracking

When task completes:
```javascript
await prisma.devTask.update({
  where: { id },
  data: {
    actualMin: actualMinutes,
    completedAt: new Date()
  }
});
```

Three derived facts:
1. **In-range hit?** Was actual within Low-to-High?
2. **Near-Expected?** Was actual within ±20% of Expected?
3. **Direction of miss?** Underestimate vs overestimate

These three drive monthly calibration.

## Monthly calibration review

First of each month, scheduler runs `monthlyEstimateCalibration()`:

For tasks completed in last 30 days:
- % within Low-High range (target: ≥80%)
- % within Expected ±20% (target: ≥60% Y1, ≥75% Y2)
- Average over-estimate or under-estimate ratio
- Distribution of "reasons High beat Expected"
- Calibration curve plot (predicted vs actual)

If hitting in-range rate <70%: ranges are too narrow. Widen them.
If hitting Expected ±20% <50%: pattern of bias (usually underestimation). Apply correction factor.

This is the **compound learning loop** for estimation.

## The fudge factor

Engineers typically underestimate. After 6 months of calibration data, you may discover you systematically multiply by 1.4x to hit Expected accuracy.

If consistent: just apply the factor. Initial estimate × 1.4 = better Expected.

Don't be defensive about this. It's data. The factor is information about your own bias.

## Anti-patterns

- **"Quick" — gut feeling.** "Should be quick, maybe 2 hours" — point estimate, no range, no confidence. Forbidden.
- **Padding instead of estimating.** Doubling the gut number isn't honest range. The High should reflect actual worst case, not just "extra to be safe".
- **Estimating to please.** When pressure exists for low number, the temptation is to shrink Expected. Resist. Honest > pleasant.
- **Not tracking actuals.** Without actuals, no calibration possible.
- **Estimating without context.** "How long will the next feature take?" — invalid without scope. Don't answer until scope clear.
- **Same range for everything.** Every task gets "1-3-8 hours" — clearly not real estimation.
- **Confidence inflation.** Claiming high confidence on novel task to seem expert. Backfires when wrong.
- **Ignoring decomposition signal.** Range >1 day = decompose first.

## Storage

In `DevTask`:
- `estimateLowMin`, `estimateExpectedMin`, `estimateHighMin` (minutes)
- `estimateConfidence` (low/medium/high)
- `estimateRationale` (text: "anchored on 3 similar tasks, median 4hrs, slight unfamiliarity with new framework adds spread")
- `actualMin` filled at completion

## Integration

- Every `/dev` command auto-generates initial estimate (analyzed via `lib/dev.js`)
- `/dev-estimate <task_id>` refines after more info
- Anchoring queries hit DevTask archive for similar tasks
- Monthly cron calls `monthlyEstimateCalibration()` from `lib/dev.js`
- Calibration metrics feed quarterly review of `architecture-decision-records` (were our pattern decisions optimizing for the right thing?)

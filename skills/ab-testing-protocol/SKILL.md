---
name: ab-testing-protocol
description: Running honest A/B tests of marketing messages and channels — one variable at a time, adequate sample, predefined success metric, resisting the urge to call a winner too early. Turns the experimenter's instinct to try many variants into rigorous learning instead of noise.
---

# A/B Testing Protocol — Honest Experiments, Not Wishful Reading

The department's experimenter psychotype generates many variants — many headlines, many angles. A/B testing is how that generative instinct becomes *learning* rather than noise: by comparing variants rigorously instead of guessing which "feels" better.

## Prerequisites

- `positioning-and-messaging` — produced the variants to test
- A campaign with enough reach to give meaningful sample sizes
- A predefined success metric

## Core principle

> An A/B test only teaches you something if it's run honestly: one variable changed, a real success metric set in advance, and enough data before a winner is called. A test read too early, with too small a sample, or judged on a metric chosen afterward, doesn't reduce uncertainty — it manufactures false confidence.

## When A/B testing is worth it

A/B testing has overhead. It's worth it when:
- The campaign has enough reach to split and still get meaningful samples per variant
- The decision matters enough to justify the rigor
- You genuinely don't know which variant is better

It's NOT worth it when:
- Reach is tiny — splitting a small audience gives samples too small to conclude anything
- The variants barely differ — the test can't detect a difference that small
- One option is obviously correct — just use it

This is a Tier 3 skill: used deliberately for campaigns big enough to support it, not reflexively for everything.

## One variable at a time

The cardinal rule. A valid A/B test changes exactly one thing between variants:

- ✓ Same everything, different headline → the test isolates the headline's effect
- ✗ Different headline AND different image AND different channel → if B wins, *what* won? Unknowable.

When multiple things differ, the test can't attribute the result. It produces a winner but no knowledge. If you must test several elements, test them in sequence (one test each) or use a proper multivariate design — not a muddled "everything's different" comparison.

## The procedure

### Step 1 — State the hypothesis
Before the test: what do you believe and why? "Hypothesis: the pain-led headline (Angle A) will out-convert the time-led headline (Angle B) for this segment, because their pain is felt more as 'guessing' than as 'wasted time'."

A stated hypothesis makes the test a real experiment and the result interpretable.

### Step 2 — Define the success metric in advance
ONE primary metric, chosen before the test runs — conversion rate, normally. Writing it down beforehand prevents the after-the-fact temptation to pick whichever metric made your favorite variant look good.

### Step 3 — Isolate one variable
Confirm the variants differ in exactly one thing. Everything else identical: same channel, same audience, same timing, same offer.

### Step 4 — Split honestly
Randomly assign the audience across variants. Equal split unless there's a reason otherwise. Same time period — don't run A this week and B next week, because the weeks differ.

### Step 5 — Pre-commit to a sample size and duration
Decide before launch: how much data, how long, before reading the result. Then *don't look early and stop*. Stopping the moment a variant pulls ahead is the most common way to fool yourself — early leads reverse constantly with small samples.

### Step 6 — Run to completion
Let it reach the planned sample and duration. Resist calling it early.

### Step 7 — Read the result honestly
- Did one variant clearly win on the primary metric?
- Is the difference large enough to be real, not noise? (With small samples, small differences mean nothing.)
- If the result is inconclusive — say so. "No clear winner" is a valid, honest outcome.

### Step 8 — Record and apply
The winning variant (or the "inconclusive") goes into the campaign archive. The lesson — *why* the winner won — feeds future `positioning-and-messaging`.

## Sample size — the uncomfortable truth

Small samples lie. With 50 people per variant, a "20% vs 24% conversion" difference is almost certainly noise — random chance would produce swings that big easily.

You don't need exact statistics, but you need honesty about scale:
- Tiny samples (tens) — a test here concludes nothing. Don't pretend otherwise.
- Hundreds per variant — small differences still suspect; large differences may be real
- Thousands per variant — smaller differences become trustworthy

If the campaign's reach can't give meaningful samples per variant, **don't A/B test** — pick the variant by judgment and note that it wasn't tested. A test too small to conclude is worse than no test, because it produces false confidence.

## The early-stopping trap

The single most common way A/B tests deceive: watching the results live, seeing variant B leap ahead on day one, declaring B the winner, stopping.

Early leads are mostly noise. With small accumulated samples, variants trade the lead constantly. The variant that's "winning" on day one frequently loses by day seven.

Counter-measure: decide the sample size and duration up front, and hold to it. Don't read the result until the test is done. If you must monitor, monitor for *catastrophe* (a variant doing something badly wrong), not for *calling the winner*.

## Worked example

Campaign: Fosved miniapp, Segment A. Three message variants from `positioning-and-messaging`:
- Angle A: "Stop guessing where your business stands." (pain-led)
- Angle B: "Quit re-checking five places to know one thing." (time-led)
- Angle C: "Run your whole business from one view." (control-led)

**Is it worth testing?** Campaign reaches ~12,000 — splitting three ways gives ~4,000 each. Enough for meaningful samples. Decision matters (this headline carries the whole campaign). → Yes, test.

**Hypothesis:** Angle A wins — the segment research said the pain registers as "guessing", not as "time".

**Primary metric (pre-defined):** signup conversion rate. Not clicks, not engagement — signups.

**One variable:** only the headline differs. Same channel (newsletters), same body, same CTA, same audience, same period.

**Pre-commitment:** run until each variant has ~4,000 impressions or two weeks, whichever first. No reading the result before then.

**Result (after the full run):**
- Angle A: 2.1% signup
- Angle B: 1.4% signup
- Angle C: 1.6% signup

A clearly leads, by a margin large enough at this sample to trust. Hypothesis supported.

**Lesson recorded:** "For Segment A, the pain framed as 'guessing/uncertainty' out-converts both the time-saving and the control framings — by a meaningful margin. Future messaging to this segment leads with uncertainty, not time." This goes into the archive and updates how `positioning-and-messaging` approaches this segment.

Note: had the result been A 2.1%, C 2.0% — that's *inconclusive* at this sample, and the honest output is "no clear winner between A and C; B is weakest." Not "A wins by 0.1%."

## Anti-patterns

- **Multiple variables at once.** Headline + image + channel all differ. A winner emerges; no knowledge of what won.
- **Early stopping.** Calling the winner on day one because a variant jumped ahead. Early leads are noise.
- **Sample too small.** Testing on tens of people and treating the result as real.
- **Metric chosen after.** Picking whichever metric made the preferred variant look good. The metric must be pre-committed.
- **Vanity metric as success metric.** Testing on clicks/engagement instead of conversion.
- **Different time periods.** Running A this week, B next week. The weeks differ; the test is contaminated.
- **No hypothesis.** Testing variants with no belief about why one should win. The result then teaches nothing.
- **Forcing a winner.** Declaring a winner when the result is genuinely inconclusive. "No clear winner" is valid.
- **A/B testing everything reflexively.** Overhead without payoff on small or low-stakes campaigns.
- **Not recording the lesson.** The test ran, a winner emerged, nobody wrote down *why* — so it doesn't improve future messaging.

## Output template

```
A/B TEST — <campaign>

Worth testing? <yes — reach supports it / no — use judgment instead>

HYPOTHESIS
<what you believe will win, and why>

VARIANTS (one variable: <the single thing that differs>)
- Variant A: <description>
- Variant B: <description>
[etc.]

PRIMARY SUCCESS METRIC (pre-defined): <metric>
PRE-COMMITTED SAMPLE / DURATION: <target per variant> / <time>

RESULT (read only after completion)
- Variant A: <metric value>
- Variant B: <metric value>
Verdict: <clear winner: X / inconclusive>
Is the difference trustworthy at this sample? <yes/no>

LESSON
<why the winner won — concrete — for future positioning-and-messaging>
```

## Integration

- Tier 3 — used deliberately for campaigns large enough to support it
- Tests the variants produced by `positioning-and-messaging`; lessons feed back into it
- `channel-strategy` — channels can also be A/B tested
- `campaign-verification` — test results inform the campaign verification
- `forecast-before-launch` — variants may be forecast separately

---
name: campaign-verification
description: Reconciling campaign forecasts with actual results — conversion, CAC, reach — and diagnosing the source of every miss. The verification step is where marketing actually learns. Without it, forecasts are just guesses nobody checks. Closes the learning loop opened by forecast-before-launch.
---

# Campaign Verification — Where Marketing Learns

A forecast that's never checked against reality teaches nothing. Verification is the discipline of sitting down after a campaign, comparing prediction to outcome, and diagnosing *why* they differed. This is the single step that turns marketing from activity into a learning system.

## Prerequisites

- A campaign with a locked forecast (`forecast-before-launch` was done)
- The campaign has completed and actual data is available

## Core principle

> The purpose of verification is not to grade the campaign — it's to improve the next forecast. A campaign that missed its forecast but produced a clear diagnosis is more valuable than one that hit its forecast by luck and taught nothing. Verify the *forecast quality*, not just the *campaign outcome*.

## When to verify

Each campaign gets a verification date — typically 2-4 weeks after completion, once actual conversion and CAC data has settled. The scheduler surfaces campaigns due for verification.

Don't verify too early (data still moving) or too late (memory of context faded).

## The procedure

### Step 1 — Collect actuals
Pull the real numbers:
- Actual reach (how many were truly touched)
- Actual conversion rate
- Actual CAC (real spend ÷ real conversions)
- Actual spend (often differs from budget)

Record them in `MarketingCampaign`. No estimating — real data only.

### Step 2 — Compute accuracy
For each forecast metric:
```
accuracy delta = |actual - forecast_expected| / forecast_expected
```
Lower is better. A delta of 0.10 means the actual was within 10% of the expected forecast.

### Step 3 — Apply target verdicts
- **Conversion within target?** Actual within ±25% of forecast → yes
- **CAC within target?** Actual within ±30% of forecast → yes
- **Was overoptimistic?** Forecast exceeded actual by 50%+ → flag it (the tracked failure)
- **In forecast range?** Did actual land within the low–high range at all?

### Step 4 — Diagnose the miss source
This is the most important step. For every significant miss, assign a cause:

- **overoptimism** — the forecast was inflated by creative excitement; nothing else wrong
- **wrong_segment** — the segment didn't behave as described; segmentation was off
- **wrong_channel** — the channel didn't reach the segment effectively
- **wrong_message** — reached the right people but the message didn't land
- **external_factor** — something outside the campaign (market event, seasonality, competitor move)
- **none** — forecast was accurate, no significant miss

Be specific. "wrong_message" should come with *what* about the message failed.

### Step 5 — Extract the lesson
One sentence: what does the next forecast/campaign do differently because of this?

Not "be more careful" — concrete. "Telegram channel placements to this segment convert at ~1%, not 3% — anchor future forecasts lower" or "the segment responds to the time-saving frame, not the decision-quality frame".

### Step 6 — Update segment status
If the campaign confirmed or contradicted the segment hypothesis, update the `AudienceSegment`:
- Segment behaved as described → mark `validated`
- Segment behaved nothing like described → mark `disproven`, the segmentation needs rework

## Decision quality vs outcome quality

Crucial distinction (shared with the engineering ADR discipline):

- **Good forecast:** the best prediction given the data available at the time
- **Good outcome:** the campaign happened to do well

These differ. A low-confidence forecast that missed badly may have been a *fine forecast* — the uncertainty was honestly stated. A high-confidence forecast that missed is a *real problem* — the confidence was unwarranted.

Verification evaluates forecast quality. Penalizing every miss leads to sandbagging (deliberately low forecasts to always "win"). Improving the forecasting process is the goal.

## Worked example

Campaign: Fosved miniapp to Segment A. Forecast was conversion 1.5–3–5%, CAC ~111, confidence low.

**Actuals:**
- Reach: 11,000 (forecast expected 12,000 — close)
- Conversion: 0.9% (forecast expected 3% — well below, below the low end of 1.5%)
- CAC: 40,000 / (11,000 × 0.009) ≈ 404 (forecast ~111 — almost 4x worse)

**Computation:**
- Conversion delta: |0.009 - 0.03| / 0.03 = 0.70 → way outside ±25%
- CAC delta: |404 - 111| / 111 = 2.64 → way outside ±30%
- Overoptimistic? Forecast (3%) exceeded actual (0.9%) by far more than 50% → YES, flagged
- In range? Actual 0.9% below the 1.5% low bound → NO

**Diagnosis:** miss source = `wrong_channel` + `overoptimism`. The Telegram channels reached people, but those audiences treated paid placements as noise — engagement was minimal. The funnel-decomposition forecast assumed a 15% click rate; actual was ~3%.

**Lesson:** "Paid placement in general small-business Telegram channels converts ~0.9% for this product — far below the funnel-decomposition assumption. Future forecasts for this channel anchor at ~1%. Consider whether the channel is wrong for this segment entirely."

**Segment status:** Segment A is not disproven — the people exist and have the pain. But the *channel* to reach them was wrong. Segment stays `hypothesized`, with a note that paid Telegram placement is not the way in.

This verification turned a failed campaign into two concrete improvements: a recalibrated channel benchmark and a channel-strategy reconsideration. That's the point.

## Aggregation into monthly review

Individual verifications feed the monthly review (`MarketingMonthlyReview`):
- Average conversion accuracy across the month
- Average CAC accuracy
- Overoptimism rate (share of campaigns flagged)
- Segment hit rate
- Clustering of miss sources — if `wrong_channel` appears 4 times, that's a pattern, not noise

The monthly review then picks 1-2 improvements. The quarterly review may update skills.

## Anti-patterns

- **Skipping verification.** The campaign ends, everyone moves on. The forecast is never checked. All learning lost.
- **Verifying without diagnosis.** "Conversion was 0.9%, below forecast." OK — but *why*? No diagnosis = no lesson.
- **Vague diagnosis.** "The campaign underperformed." Underperformed how, because of what?
- **Vague lesson.** "Do better next time." Not actionable. Be concrete.
- **Grading instead of learning.** Treating verification as a report card. It's a learning tool.
- **Penalizing honest misses.** A low-confidence forecast that missed isn't a failure — the uncertainty was stated. Penalizing it causes sandbagging.
- **Cherry-picking metrics.** Reporting the metric that looked good, ignoring the one that didn't.
- **Not updating segment status.** The campaign proved the segment wrong, but `AudienceSegment` still says hypothesized.
- **Verifying too early.** Data still moving, conclusions premature.
- **Ignoring repeated miss sources.** Same diagnosis 3 months running and nothing changes.

## Output template

```
CAMPAIGN VERIFICATION — <campaign title> (#<id>)

FORECAST vs ACTUAL
                  Forecast (expected)   Actual      Delta
Reach:            <n>                   <n>         <%>
Conversion:       <%>                   <%>         <%>
CAC:              <amount>              <amount>    <%>

VERDICTS
- Conversion within ±25%:  yes / no
- CAC within ±30%:         yes / no
- Overoptimistic (50%+ over): yes / no
- Actual within forecast range: yes / no

DIAGNOSIS
Miss source: overoptimism | wrong_segment | wrong_channel | wrong_message | external_factor | none
Explanation: <specific — what happened and why>

LESSON LEARNED
<one concrete sentence — what the next forecast/campaign does differently>

SEGMENT STATUS UPDATE
<segment> → validated | disproven | unchanged (with reason)
```

This populates the `CampaignVerification` model. Campaign status moves `completed` → `verified`.

## Integration

- Tier 1 — loaded for every verification task
- Closes the loop opened by `forecast-before-launch`
- `audience-segmentation` — verification updates segment validation status
- Feeds `MarketingMonthlyReview` aggregation
- Repeated miss-source clusters trigger quarterly skill updates
- `/campaign-verify` command runs this procedure

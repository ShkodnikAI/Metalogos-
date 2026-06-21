---
name: forecast-before-launch
description: Mandatory numeric forecast before any campaign launches — predicted conversion rate, CAC, reach. A campaign without a forecast cannot be verified, and an unverifiable campaign teaches nothing. The forecast turns marketing from "spend and hope" into a testable hypothesis. The core learning mechanism of the department.
---

# Forecast Before Launch — A Campaign Is a Hypothesis

Marketing without forecasts is gambling with a creative budget. You spend, something happens, and you can't tell if it was good, bad, or luck. The forecast fixes this: it makes every campaign a falsifiable prediction.

## Prerequisites

- `audience-segmentation` complete — the target segment is defined
- Campaign concept exists (what's being promoted, through what channels)

## Core principle

> A campaign is a prediction about how a specific group of people will behave. State the prediction in numbers BEFORE launch, or you forfeit the ability to ever learn whether the campaign worked. "It felt successful" is not knowledge. "Predicted 4% conversion, got 3.6%" is knowledge.

## What must be forecast (hard rule 1)

Before a campaign moves from `drafted` to `launched`, it must have:

1. **Conversion rate** — what fraction of reached people take the target action. Stated as a number (e.g. 3.5%) or a range (2-5%).
2. **CAC** — cost to acquire one customer/user. The total spend divided by predicted conversions.
3. **Reach** — how many people the campaign will actually put the message in front of.
4. **Forecast basis** — the reasoning. Where do these numbers come from?
5. **Confidence** — high / medium / low. How sure are you?

No forecast → campaign cannot launch. This is non-negotiable.

## Forecast as a range, not a point

Like engineering estimates, marketing forecasts are honest as ranges:

- **Conversion: 2% – 4% – 7%** (low / expected / high)
- The spread reveals uncertainty. A 2%-to-7% spread means lots of unknowns. A 3.5%-to-4.5% spread means well-understood.

A single point ("4%") pretends a certainty that doesn't exist. The range tells the truth.

## How to forecast conversion

Best to worst basis:

1. **Past campaign actuals** — "our last 3 campaigns to this segment converted 3-5%". Strongest anchor.
2. **Comparable benchmark** — "campaigns of this type in this channel typically see 1-3%". Decent if the comparison is honest.
3. **Funnel decomposition** — estimate each step (see message → click → sign up → activate) and multiply. Useful when no direct precedent.
4. **Industry rule-of-thumb** — weak, wide ranges only.
5. **Pure guess** — confidence = low, range very wide. Acknowledge it.

Anchor on the archive whenever possible. The `MarketingCampaign` table accumulates actuals; query it for similar past campaigns.

## How to forecast CAC

```
CAC = total campaign spend / number of conversions

Predicted CAC = forecast budget / (forecast reach × forecast conversion rate)
```

Example:
- Budget: 50,000
- Reach: 20,000 people
- Conversion: 3%
- → Conversions = 20,000 × 0.03 = 600
- → CAC = 50,000 / 600 ≈ 83 per customer

Then sanity-check: is 83 acceptable? That depends on what a customer is worth — see `unit-economics-and-cac`. If CAC > customer value, the campaign loses money even if it "works".

## How to forecast reach

Reach is usually the most knowable number — it depends on channel mechanics:
- Paid channels: budget ÷ cost-per-impression × audience
- Owned channels: known subscriber/follower counts
- Earned: hardest, widest range

Don't confuse reach with segment size. Reach = how many you'll actually touch. Segment size = how many exist.

## The overoptimism trap

Marketers systematically over-forecast. The creative excitement inflates the numbers. This is the #1 tracked failure (`overoptimism rate` metric).

Counter-measures:
- Forecast the **expected** case, then ask "what would a skeptic predict?" — that's closer to reality
- If past campaigns to this segment averaged 3%, don't forecast 6% for the new one without a specific, stated reason
- A forecast that's "better than every past campaign" needs explicit justification, not enthusiasm
- When in doubt, the honest forecast is lower than the exciting one

## Worked example

Campaign: promote the Fosved miniapp to Segment A ("Overloaded solo operator").

**Forecast (before launch):**
- Channel: 4 Telegram channels about small business, paid placement
- Reach: 8,000 – 12,000 – 15,000 people (low/expected/high)
- Conversion (message → miniapp signup): 1.5% – 3% – 5%
  - Basis: no direct past campaign; funnel estimate — ~15% click the link, ~20% of those sign up. Confidence: low.
- Predicted conversions: at expected, 12,000 × 0.03 = 360
- Budget: 40,000
- Predicted CAC: 40,000 / 360 ≈ 111
- Confidence: low (first campaign to this segment, no anchor)

This forecast is now locked in `MarketingCampaign`. After the campaign, `campaign-verification` compares it to reality. Even if the campaign "feels" good or bad, the numbers settle it — and the low-confidence flag means a miss here is expected and informative, not a failure.

## What the forecast enables

1. **Verification** — without a forecast, nothing to compare actuals against
2. **Go/no-go** — if predicted CAC exceeds customer value, don't launch
3. **Budget allocation** — forecast comparison across campaigns guides spend
4. **Calibration** — over months, forecast-vs-actual data reveals your bias and improves it
5. **Honest expectations** — owner knows what to expect, isn't surprised

## Anti-patterns

- **No forecast, just launch.** "We'll see how it goes." Forfeits all learning.
- **Point estimate.** "4% conversion" with no range. False precision.
- **Forecast after launch.** Writing the prediction once results are in. That's not a forecast, it's a memory. Worthless.
- **Overoptimistic forecast.** Numbers driven by creative excitement, not evidence.
- **Forecast with no basis.** "5% feels right." Where does 5% come from?
- **Vanity forecasts.** Forecasting impressions and likes instead of conversions and CAC.
- **Ignoring the archive.** Forecasting from scratch when 5 similar past campaigns exist.
- **High confidence on novel campaign.** First-ever campaign to a segment can't be high-confidence.
- **CAC forecast without checking customer value.** A "successful" campaign that loses money per customer.

## Output template

```
CAMPAIGN FORECAST — <campaign title>

Target segment: <segment name / id>
Channels: <list>

Reach:           <low> – <expected> – <high> people
Conversion rate: <low> – <expected> – <high>
Budget:          <amount>
Predicted conversions (expected): <number>
Predicted CAC:   <amount> per customer

Forecast basis: <reasoning — past campaigns / benchmark / funnel decomposition>
Confidence: high | medium | low

Go/no-go check: predicted CAC <amount> vs customer value <amount> → <viable / not viable>
```

This populates the forecast fields of `MarketingCampaign`. Status moves `drafted` → `forecasted`.

## Integration

- Tier 1 — loaded for every campaign task
- `audience-segmentation` defines the segment the forecast is about
- `unit-economics-and-cac` provides the customer value for the go/no-go check
- `campaign-verification` compares this forecast to actuals
- `ab-testing-protocol` may forecast per variant
- Forecast stored in `MarketingCampaign`; verification reads it back

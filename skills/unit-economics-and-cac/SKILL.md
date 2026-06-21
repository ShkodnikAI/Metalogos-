---
name: unit-economics-and-cac
description: The economics that decide whether a campaign makes or loses money — CAC, customer lifetime value, payback period, the LTV:CAC ratio. A campaign that "works" by vanity metrics but costs more to acquire a customer than the customer is worth is a failure. This skill prevents profitable-looking losses.
---

# Unit Economics and CAC — Does the Campaign Make Money?

A campaign can hit every vanity target — great reach, strong conversion — and still lose money on every customer. Unit economics is the discipline of knowing, before and after, whether acquiring a customer costs less than that customer is worth.

## Prerequisites

- `forecast-before-launch` — CAC is part of the mandatory forecast
- Basic facts about what a customer is worth (price, retention)

## Core principle

> A conversion is not a win. A conversion is a win only if the customer is worth more than it cost to acquire them. Marketing that ignores this can run "successful" campaigns straight into a loss — every new customer making the hole deeper. Know the unit economics, or you don't know if you're succeeding.

## The core numbers

**CAC — Customer Acquisition Cost**
```
CAC = total spend to acquire customers / number of customers acquired
```
Total spend includes the campaign budget and, properly, the cost of producing it. Per campaign, budget ÷ conversions is the working figure.

**LTV — Customer Lifetime Value**
```
LTV = average revenue per customer per period × average number of periods retained
      (× gross margin, for the honest version)
```
How much a customer is worth over their whole relationship — not just the first payment.

**Payback period**
```
Payback = CAC / revenue per customer per period
```
How long until a customer has paid back what it cost to acquire them. A CAC of 100 with revenue of 20/month → 5-month payback.

**LTV:CAC ratio**
```
ratio = LTV / CAC
```
The headline health number.

## What the LTV:CAC ratio means

- **Below 1:1** — every customer loses money. The campaign is destroying value. Stop.
- **Around 1:1 to 2:1** — barely viable; little room for other costs. Fragile.
- **Around 3:1** — generally healthy. Common target.
- **Far above 3:1 (e.g. 5:1+)** — looks great, but may mean you're *underspending* on marketing and leaving growth on the table.

The ratio is a guide, not a law — but a campaign forecast to come in below 1:1 should not launch (`forecast-before-launch` go/no-go check).

## Payback period matters as much as the ratio

A great LTV:CAC ratio with a very long payback can still sink you — the cash is gone now, the return arrives slowly. For a small operation, payback period is often the more urgent number:

- Short payback (weeks to a few months) — cash recovers fast, campaigns can be reinvested
- Long payback (many months) — even profitable campaigns strain cash flow

This is where the boundary with the Finance department lives: Marketing forecasts CAC and payback; Finance integrates it into the overall cash position.

## The honest LTV

LTV is the easiest number to inflate. Honest LTV:

- Uses **gross margin**, not revenue — a customer paying 100 of which 40 is cost is worth 60, not 100
- Uses **realistic retention** — don't assume customers stay forever; use actual or conservatively estimated churn
- Doesn't count **speculative future upsells** as if guaranteed
- For a young product with no retention history — LTV is an *estimate with low confidence*; say so, and use a conservative figure

Inflated LTV makes terrible campaigns look fine. The discipline: when uncertain, the LTV estimate is the conservative one.

## The procedure

### Step 1 — Establish customer value
What does one customer pay, how often, for how long, at what margin? If the product has history, use it. If not, estimate conservatively and flag low confidence.

### Step 2 — Forecast CAC (pre-launch)
From the campaign forecast: budget ÷ (reach × conversion). This is part of `forecast-before-launch`.

### Step 3 — Compute the ratios
LTV:CAC and payback period from the forecast numbers.

### Step 4 — Go/no-go
- Forecast ratio below 1:1 → do not launch, rework the campaign
- Ratio thin (1-2:1) → launch only with eyes open, or improve before launching
- Ratio healthy → proceed
- Payback too long for the cash position → flag, coordinate with Finance

### Step 5 — Verify with actuals (post-campaign)
After the campaign, real CAC replaces forecast CAC. Recompute the real ratio. This feeds `campaign-verification`.

## Worked example

Campaign: Fosved miniapp to solo operators.

**Customer value:**
- Price: 500/month (illustrative)
- Gross margin: ~80% → 400/month of real value
- Estimated retention: ~10 months average (low confidence — product is young; conservative figure used)
- LTV (honest): 400 × 10 = 4,000

**Campaign forecast (from `forecast-before-launch`):**
- Budget 40,000, reach 12,000, conversion 3% → 360 customers
- Forecast CAC: 40,000 / 360 ≈ 111

**Ratios:**
- LTV:CAC = 4,000 / 111 ≈ 36:1 — looks extremely healthy
- Payback = 111 / 400 ≈ 0.3 months — recovered almost immediately

**Reading it honestly:** 36:1 is *suspiciously* high. Two possibilities: (a) the conversion forecast (3%) is overoptimistic and real CAC will be far higher, or (b) marketing is genuinely underspending and could grow much faster. Given the forecast confidence was *low*, (a) is the prudent assumption.

**Stress test:** if real conversion is 0.9% (the pessimistic case), CAC = 40,000 / (12,000 × 0.009) ≈ 370. LTV:CAC then = 4,000 / 370 ≈ 11:1 — still viable. Even the pessimistic case clears the 1:1 floor comfortably. **Go.**

Note what the analysis did: it didn't celebrate the 36:1. It distrusted it, stress-tested the downside, and confirmed the campaign survives even the bad case. That's the discipline.

## Anti-patterns

- **Ignoring CAC entirely.** Reporting conversions as wins with no cost side. Profitable-looking losses.
- **Revenue as LTV.** Counting the full price, ignoring margin. Inflates LTV, hides bad economics.
- **Forever retention.** Assuming customers never churn. The most common LTV inflation.
- **Speculative upsells in LTV.** Counting future revenue that may never happen.
- **Ratio without payback.** A healthy ratio with a brutal payback period still strains cash.
- **Celebrating a suspiciously high ratio.** 30:1 usually means a wrong number somewhere, or severe underspending — investigate, don't cheer.
- **High-confidence economics on a young product.** No retention history = LTV is a low-confidence estimate. Pretending otherwise.
- **Vanity over economics.** Reporting reach and engagement; burying CAC.
- **Not re-checking with actuals.** Forecast economics looked fine; nobody recomputes with real CAC.

## Output template

```
UNIT ECONOMICS — <campaign>

CUSTOMER VALUE
Price per period: <amount>
Gross margin: <%>  →  margin value per period: <amount>
Estimated retention: <N periods>  (confidence: high/medium/low)
LTV (honest, margin-based): <amount>

CAMPAIGN COST
Budget: <amount>
Forecast conversions: <N>
Forecast CAC: <amount>

RATIOS
LTV:CAC = <ratio>
Payback period = <N periods>

STRESS TEST (pessimistic case)
If conversion = <low-case>, CAC = <amount>, LTV:CAC = <ratio>
Survives the floor (1:1)? yes / no

GO / NO-GO: <go | rework | no-go>  — reason: <...>
Cash-flow note for Finance: <payback implication>
```

## Integration

- Tier 2 — loaded for campaign tasks
- `forecast-before-launch` — CAC forecast feeds these ratios; this skill supplies the go/no-go
- `campaign-verification` — recomputes ratios with actual CAC
- Boundary with the Finance department — payback period handed over for cash-flow modeling
- `honest-claims-discipline` — no inflated economics, same as no inflated claims

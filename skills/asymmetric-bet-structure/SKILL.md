---
name: asymmetric-bet-structure
description: Identifies and structures opportunities where downside is bounded and upside is large or unbounded — the Taleb / Spitznagel approach. Most decisions are not asymmetric; the minority that are can produce returns disproportionate to required confidence. Skill works at two levels — finding asymmetric opportunities in current analyses and structuring positions to be antifragile (benefiting from volatility rather than being damaged by it). The mechanism that converts ОСП analyses into actionable investment edge.
---

# Asymmetric Bet Structure — Where Limited Downside Meets Large Upside

Most decisions in life and investing have symmetric payoffs — if you're right you win some, if you're wrong you lose comparable amounts. These are coin-flip propositions where edge requires being right more often than wrong, by a margin large enough to overcome fees and friction.

Asymmetric decisions are different. If you're wrong you lose 1x; if you're right you win 5x or 10x or 100x. Even if you're right less than half the time, expected value is positive. Even if you're right only 20% of the time, asymmetric structure produces wealth over many bets.

The investing philosophy of Taleb (Universa Investments via Mark Spitznagel, "Black Swan" framework) is built on this insight. The core discipline: **don't predict; structure for asymmetric payoff regardless of prediction**. ОСП analyses identify possible asymmetric opportunities; this skill structures them.

For the Rheinmetall-before-Iran-war / Bitcoin-in-2010 type opportunities — these are all asymmetric structures. Limited downside (worst case: stocks move with market, modest losses), large upside (best case: war drives 5-10x revaluation; new asset class emergence drives 100x). The structure is what produces the return; prediction merely identifies candidates.

## Prerequisites

- ОСП analyses producing fruit projections from `decision-tree-forecast`
- `macro-regime-id` output (regime determines available asymmetric structures)
- `cross-asset-divergence` output (divergences often signal asymmetric mispricing)
- Familiarity with options structures, futures, asymmetric instruments

## Core principle

> Predicting outcomes correctly is hard. Structuring positions where being wrong costs little and being right pays a lot is achievable. The discipline is to filter analyses for asymmetric structure, not to seek high-probability predictions.

The mistake is to look for high-conviction predictions. High-conviction predictions are often already priced. The asymmetric opportunities are usually moderate-conviction predictions where the structure of the bet does the work.

## The structure of asymmetric opportunities

Three components define an asymmetric bet:

**1. Bounded downside.** Maximum loss is finite, known in advance, and acceptable. This is the foundation. Without bounded downside, no opportunity is asymmetric — no matter how attractive the upside, unbounded downside can ruin.

Examples:
- Long-only equity position with stop-loss: bounded at stop-loss level
- Long call options: bounded at premium paid
- Long out-of-the-money calls (specifically Talebian): bounded at small premium
- Holding physical asset: bounded at acquisition cost

Examples of NON-bounded downside (NOT asymmetric):
- Naked short positions (theoretically unlimited loss)
- Margined positions without stops
- Concentrated positions without exit liquidity

**2. Large or unbounded upside.** Best case scenario produces returns multiple times the bounded downside.

Examples:
- Equity early in disruption cycle: 5-10x potential
- Out-of-the-money call options: 10-100x on premium
- Pre-mass-adoption asset (early Bitcoin): 1000x+ potential

**3. Sufficient probability of upside materialization.** Not certainty — sufficient probability for expected value calculation.

If P(upside) × upside > P(downside) × downside, the bet has positive expected value. For asymmetric structures, even low P(upside) can produce positive EV.

Example:
- Bet costs 1 (bounded downside)
- 80% probability of zero/loss (lose 1)
- 20% probability of 10x return (gain 10)
- EV = 0.8 × (-1) + 0.2 × 10 = -0.8 + 2.0 = +1.2

Positive EV with only 20% probability of being right. This is what asymmetry buys.

## The procedure

### Step 1 — Receive forecast from ОСП

From `decision-tree-forecast`, take a fruit or seed projection with:
- Specific predicted outcome
- Probability of full ripening / germination
- Beneficiary (who benefits if it materializes)
- Time horizon for ripening
- Conditional dependencies

### Step 2 — Test for asymmetry

Ask three questions:

**Q1: What instruments are available to express this view?**
- Long stock of beneficiary
- Long call options on beneficiary
- Long futures on commodity
- Long ETF tracking sector
- Long debt instruments
- Direct asset acquisition

**Q2: What is the bounded downside for each instrument?**
- Stock: position size (or with stop, smaller)
- Call options: premium paid
- Futures with stops: stop distance × position size
- Direct asset: acquisition cost minus residual value

**Q3: What is the upside if the projection materializes?**
- Stock: typical 1-5x in major scenarios
- Call options: typically 5-50x for OTM, 1-5x for ITM
- Futures: leveraged version of underlying move
- Direct asset: depends on asset

### Step 3 — Filter for sufficient asymmetry

Reject candidates where upside / downside ratio is below 3:1. Asymmetric bets need substantial multiplier; otherwise they're just leveraged conviction trades and require high probability accuracy.

Specifically:
- Stock: requires upside scenario producing at least 3x return
- OTM call options: requires upside scenario producing at least 5x option return
- Direct asset: requires upside scenario producing at least 3x value

### Step 4 — Compute expected value

For each remaining candidate:

EV = P(upside) × upside_multiplier + P(downside) × (-1)

Where probabilities sum to 1 and the structure is normalized by downside (downside = -1).

Filter for EV > +0.5. This threshold ensures meaningful expected return after friction (commissions, slippage, opportunity cost).

### Step 5 — Stress test

For each candidate that passes EV filter:

**Stress test 1 — Worst case:** if everything goes wrong (predicted scenario doesn't materialize, plus general market stress), what is total loss?

**Stress test 2 — Time decay:** for options-based positions, what if outcome materializes later than expected? Premium decay can cost more than expected.

**Stress test 3 — Liquidity:** can the position be exited if needed? Asymmetric instruments often have lower liquidity than mainstream alternatives.

**Stress test 4 — Correlation breakdown:** does the asymmetric bet rely on certain correlations holding? Under stress, correlations shift; correlation-dependent structures can fail.

### Step 6 — Position sizing

Once a candidate passes filters and stress tests, size the position. Two principles:

**Kelly criterion (modified):**
Theoretical optimal sizing based on edge and odds. For asymmetric bets:
f* = (P × (b+1) - 1) / b
where P = probability of winning, b = ratio of win to loss

But actual sizing should be **fractional Kelly** — typically 25-50% of Kelly recommendation, due to model uncertainty and parameter estimation error.

**Concentration constraints:**
- No single asymmetric bet should exceed 5-10% of portfolio (regardless of Kelly)
- Aggregate asymmetric bet exposure should be 10-25% of portfolio (rest in stable assets)
- This protects against multiple bets being wrong simultaneously

### Step 7 — Define exit criteria

Asymmetric bets need explicit exit criteria:

**Upside exit:**
- Target: when upside scenario materializes, at what level do you take profits?
- Avoid: holding through full ripening to maximum, missing exit opportunity, returning to baseline

**Downside exit:**
- Stop level for stocks (typically 20-30% below entry, depending on volatility)
- Time decay management for options
- Re-evaluation triggers if scenario stops looking valid

**Re-evaluation triggers:**
- New information that contradicts thesis
- Indicator activation suggesting regime change
- ОСП analysis update changing fruit probabilities

### Step 8 — Monitor and learn

Track all asymmetric bets:
- Entry price, size, thesis
- Exit price, P&L, lessons
- Whether bet was right (thesis materialized) or wrong
- Whether structure performed as expected

Aggregate analysis monthly:
- Hit rate on asymmetric bets (typically 20-40% for asymmetric structures, but this is OK if structure was right)
- Average return on winners vs losers
- Whether expected value calculations were accurate ex post

## Worked examples

### Example 1: Defense before Iran war (2024-2025)

**ОСП forecast (2024):** Probability of major Middle East conflict involving US in next 24 months: 30-40%, range mid case. Specific defense beneficiaries identified: RTX, LMT, NOC, Rheinmetall (Europe).

**Asymmetric structure analysis:**

Q1: Instruments available
- Long stock RTX/LMT/NOC: yes
- Long call options: yes (LEAPS available with 1-2 year expiry)
- ETF (ITA): yes

Q2: Bounded downside
- Stock: position size; with 30% stop, ~30% of position
- LEAPS: premium (typically 5-15% of underlying)
- ETF: position size

Q3: Upside if conflict materializes
- Stocks: historical pattern suggests 2-4x for pure-play defense over 12-24mo
- LEAPS: typically 5-15x on premium for similar moves
- ETF: 1.5-2.5x typically

**EV calculation (using OTM LEAPS):**
- P(upside materialization) = 35% (mid case from forecast)
- Upside multiplier: 8x (mid case for OTM LEAPS in this scenario)
- EV = 0.35 × 8 + 0.65 × (-1) = 2.8 - 0.65 = +2.15

Strongly positive EV.

**Stress tests:**
- Worst case: no war, market stress simultaneously, options expire worthless, stocks down 20%. Acceptable bounded loss.
- Time decay: 2-year LEAPS provide enough time for thesis to play out
- Liquidity: defense LEAPS sufficiently liquid for institutional sizing
- Correlation: defense stocks may be correlated with general market in some stress scenarios but not perfectly

**Sizing (illustrative):**
- Kelly recommendation: ~30% based on parameters
- Fractional Kelly (25% of Kelly): 7.5%
- Cap at concentration limit: 5% of portfolio in this single bet

**Exit criteria:**
- Upside target: 5x return (sell partial), 10x return (sell remainder)
- Stop: thesis disconfirmation (peace deal, US-Iran rapprochement signals)
- Re-evaluate: any I&W watchlist alert level changes on Iran scenario

**Outcome (hypothetical):** Iran war Feb 2026. ITA peaked ~$240 (from $185 entry, +30%). LEAPS struck $200 produced ~6x return at peak. Asymmetric structure paid 6x on entry premium.

But: post-war divergence (defense -12% during war) was the exit signal. Smart money exited at peak.

### Example 2: Bitcoin 2010 (illustrative, retrospective)

**Hypothetical ОСП-like forecast (2010):** Probability of new asset class emergence around digital currency in next 10 years: 5-10%. Beneficiaries: foundational protocol holders.

**Asymmetric structure:**
- Q1: Instrument: direct BTC purchase
- Q2: Bounded downside: total loss of investment (1x downside)
- Q3: Upside if it materializes: 1000x+ (BTC went from <$1 to $50,000+)

**EV calculation:**
- P(upside) = 5% (low probability)
- Upside multiplier: 1000x
- EV = 0.05 × 1000 + 0.95 × (-1) = 50 - 0.95 = +49

Massively positive EV despite low probability.

**Sizing:** Even 1% of portfolio at 1000x return doubles portfolio. So Kelly says size aggressively. Concentration cap says 5%. With 5% sized at 5% probability of 1000x: expected return on portfolio = 5% × 5% × 1000x = +250%. This is why early-stage asymmetric bets dominate portfolio returns when they hit.

### Example 3: Belarus FX during war (current period)

From recent BYN/USD analysis. Forecast: 25% probability of regime change scenarios (combined B+C+D scenarios).

**Asymmetric instruments?**
- Long BYN: not asymmetric (downside not bounded — currency can go to crisis levels)
- Long USD/BYN: bounded downside if structured with stops
- Long EU defense if Belarus becomes acute concern: tangentially asymmetric

**EV check on USD/BYN long:**
- Bounded downside: 10-15% (stops or option premium)
- Upside if regime stress: 50-100% currency move
- P(stress scenario) = 25%
- EV = 0.25 × 5 + 0.75 × (-1) = 1.25 - 0.75 = +0.5

Marginal positive EV. Worth considering but not strongly asymmetric. Better opportunities exist elsewhere.

## Anti-patterns

- **Treating asymmetric structure as same as conviction trade.** A high-conviction trade can be asymmetric or symmetric depending on instrument. The structure is the asymmetric part, not the conviction.
- **Ignoring bounded-downside requirement.** "It's a great opportunity" — if downside is unbounded, it's not asymmetric. A trade that can ruin you is never asymmetric regardless of upside.
- **Excessive Kelly sizing.** Theoretical Kelly is wildly overconfident. Parameter estimation error and model uncertainty mean fractional Kelly (25-50%) is appropriate. Full Kelly leads to ruin.
- **No exit discipline.** Asymmetric bets often get held through full ripening into eventual reversal. Define exit criteria upfront.
- **Concentration in single asymmetric bet.** Even highest-confidence asymmetric bet should not exceed concentration limit. Multiple asymmetric bets are how returns compound.
- **Ignoring correlation:** asymmetric bets that all rely on similar regimes will all fail in adverse regime.
- **Treating bounded as zero downside.** Bounded means known maximum loss, not acceptable loss. Even bounded downside compounds — multiple losing bets erode capital. Position sizing must account for series-of-bounded-losses scenario.
- **Neglecting time decay.** Options-based asymmetric structures lose value over time even if thesis remains valid. Time decay is real cost.
- **Hindsight asymmetry.** Looking at successful asymmetric bets after the fact and concluding "I'd buy that." The hard part is identifying asymmetry before the outcome.

## Output template

```
─── ASYMMETRIC BET STRUCTURE ───

Source forecast: <reference to ОСП analysis ID>
Predicted outcome: <description>
Beneficiary if materialized: <named actor / asset>
Probability: <range>
Time horizon: <when to ripen>

INSTRUMENT CANDIDATES:
1. <Instrument 1>
   Bounded downside: <amount or %>
   Upside multiplier in scenario: <amount>
   Liquidity: <good | moderate | poor>
   Time decay risk: <none | moderate | high>

2. <Instrument 2>
   <same fields>

[2-5 candidates]

ASYMMETRY FILTER:
Candidates with ≥3:1 upside:downside ratio: <list>

EV CALCULATION:
For each remaining candidate:
- P(upside) × upside_multiplier + P(downside) × (-1) = <EV>

Filter for EV > +0.5: <list>

STRESS TESTS:
Worst case scenario for top candidate: <description with size of loss>
Time decay impact: <description>
Liquidity check: <status>
Correlation dependencies: <list>

POSITION SIZING:
Kelly recommendation: <%>
Fractional Kelly (used): <%>
Concentration cap: <%>
Final size: <%>

EXIT CRITERIA:
Upside target: <when to take profit>
Downside stop: <where>
Re-evaluation triggers: <events>

POST-TRADE TRACKING:
Entry: <date, price, size>
Thesis: <brief>
Indicators to monitor: <list>
Review schedule: <frequency>
```

## Integration with ОСП

This skill is invoked **on-demand** rather than continuously. Triggers:

- ОСП analysis identifies fruit with significant probability and named beneficiary → check for asymmetric structure
- `cross-asset-divergence` flags mispricing → may indicate asymmetric opportunity
- Owner request via `/recommend <category>` → structure recommendations as asymmetric bets

**Output stored in:** `AsymmetricBet` Prisma model. Tracks: source analysis, instruments considered, sizing, entry/exit, P&L, post-trade lessons.

**Periodic review:**
- Monthly: open positions status, re-evaluation if needed
- Quarterly: track record analysis (hit rate, average returns, lessons)
- Annually: methodology review (are filter thresholds calibrated correctly?)

This skill, more than any other, **converts analytical work into financial returns**. ОСП can produce excellent analysis without ever generating wealth if outputs aren't structured into asymmetric positions. This skill is the bridge.

The principle is unchanging: predict moderately well, structure for asymmetry, position size with discipline, and over many bets the asymmetric structure does the work of producing returns disproportionate to required prediction accuracy.

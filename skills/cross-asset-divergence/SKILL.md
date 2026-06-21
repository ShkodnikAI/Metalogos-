---
name: cross-asset-divergence
description: Systematic detection of divergences between asset classes that should normally move together — the strongest signal in financial analysis. When bonds say recession but equities say boom, when oil says shortage but gold says deflation, when currency says strong but flows say capital flight — one of them is wrong, and identifying which produces the highest-confidence forecasts. Updated daily on monitored asset pairs.
---

# Cross-Asset Divergence — When Markets Tell Different Stories

Different asset classes are priced by different participants with different time horizons, different mandates, and different information sets. Bonds are priced by long-duration institutional money concerned with deflation and credit. Equities are priced by mix of fundamentals investors and momentum traders. Commodities are priced by physical-economy participants and macro hedge funds. Currencies are priced by central bank reactions and capital flows.

When all these participant pools converge on the same view, prices move together — there's a single story the market is telling. When they diverge, **one or more participant pools is missing something**. Identifying which one — and why — produces the highest-conviction forecasts available in financial analysis.

This is the alpha layer for financial forecasting. Most professional macro funds run divergence analysis continuously. Retail investors don't have the tools or the discipline.

## Prerequisites

- Daily access to data on major asset classes (equities, bonds, commodities, FX, credit)
- `macro-regime-id` skill output (regime determines which divergences are meaningful)
- Historical baseline of normal cross-asset correlations
- Discipline to investigate divergences rather than dismiss them

## Core principle

> When asset classes that normally move together diverge, one of them is mispricing. Identifying which one — and why — is the highest-leverage forecasting question available.

The mistake is to treat divergence as noise. Divergence is signal — specifically, it's signal that some participant pool is operating with information or model the others don't have. Working out who is right and why is the analytical task.

## Standard cross-asset relationships

Under normal conditions, certain asset classes have predictable relationships. Departures from these are diagnostic.

### Bonds vs Equities

**Normal relationship:** in disinflationary growth (Q4 regime), both rise together. In stagflation, both fall (rates up, equity multiples compress). In deflationary recession, bonds rise, equities fall.

**Diagnostic divergences:**
- Bonds rising hard while equities rising hard → flight to safety meets equity euphoria. Usually means equity euphoria is wrong (rare but occurred 1999-2000).
- Bonds falling hard while equities rising hard → "good" inflation fears (rates up because growth is strong). May be sustainable in early-middle expansion.
- Bonds rising hard while equities falling hard → recession trade in motion. Usually accurate.

### Equities vs Credit

**Normal relationship:** equity vol and credit spreads track. Stress in credit precedes stress in equity by weeks or months.

**Diagnostic divergence:**
- Credit spreads widening while equities at highs → late-cycle warning. Equities lagging credit's accurate read on stress. Pattern in 2007 pre-crisis, 2018 late-cycle.

### Equities vs Equity Volatility

**Normal relationship:** VIX and equity prices move opposite (negative correlation).

**Diagnostic divergence:**
- VIX rising while equities also rising → "ride the wall of worry" in some interpretations, but historically often precedes tops.
- VIX collapsing while equities flat → vol-selling exhaustion, often precedes vol expansion.

### Currencies vs Capital Flows

**Normal relationship:** strong currency = capital inflows; weak currency = outflows.

**Diagnostic divergence:**
- Currency strong but capital fleeing (visible in central bank reserve drawdowns, deposits at foreign branches) → administrative price-setting, not market price. Currency is overvalued vs flows. Will resolve via devaluation.
- Currency weak but capital arriving → undervalued, may strengthen. Often happens with EM currencies post-crisis.

### Oil vs Industrial Metals

**Normal relationship:** both proxy for industrial activity, move together.

**Diagnostic divergence:**
- Oil up, copper/iron ore down → oil is up due to supply disruption (Iran 2026), not demand. Demand picture is bearish. Equity implications: avoid industrial cyclicals.
- Oil down, copper up → demand strength but oversupply in oil. Picture mixed.

### Gold vs Real Yields

**Normal relationship:** gold inversely correlated with real yields.

**Diagnostic divergence:**
- Gold rising while real yields rising → currency debasement fears overriding real rate effect. Indicates monetary regime concerns. Pattern in 2022 partially, post-Iran-war 2026.
- Gold falling while real yields falling → liquidation pressure (gold being sold for cash to cover other losses). Late-stage credit crisis pattern.

### Defense Equities vs Defense Spending Forecasts

**Normal relationship:** defense equities track defense spending growth expectations.

**Diagnostic divergence:**
- Defense equities falling during active war → counter-intuitive; signals smart money expects ceasefire/de-escalation; backlog already priced. (Pattern in March-April 2026 Iran-war.)
- Defense equities rising before war narrative is mainstream → leading indicator of conflict. (Rheinmetall 2024-2025 case.)

## The procedure

### Step 1 — Establish monitored asset pairs

Define pairs to monitor continuously. Typical set:

**Macro pairs:**
- 10Y Treasury yield vs S&P 500
- Credit spreads (HY OAS) vs S&P 500 vol
- Dollar index vs gold
- Oil vs copper/iron ore
- Gold vs real 10Y yield

**Sector pairs:**
- Defense ETF (ITA) vs aggregate defense spending forecasts
- Banks (KBE/XLF) vs yield curve steepness
- Energy (XLE) vs oil price
- Tech (QQQ) vs long-duration treasuries
- Consumer staples vs cyclicals

**FX pairs (under stress regimes):**
- EUR vs European fund flows
- JPY vs Japanese asset prices
- BYN vs Russian fiscal indicators (case from prior analysis)

### Step 2 — Establish normal correlations

For each pair, compute historical correlation (typically 6-month or 1-year rolling). This is the baseline.

Record:
- Mean correlation
- Standard deviation of correlation
- Correlation regime (some pairs have stable correlation; others vary by macro regime)

### Step 3 — Daily divergence scan

Each trading day, for each monitored pair:
- Compute current short-term correlation (typically 5-day or 10-day)
- Compare to baseline
- Flag pairs where short-term diverges from baseline by more than 2 standard deviations

Flagged divergences become **investigation candidates**.

### Step 4 — Investigate flagged divergences

For each flagged divergence:

**Sub-step 4a — Verify it's real, not data artifact**
- Single-day spikes can be data noise
- Multi-day persistence (3+ days at flagged divergence) likely real

**Sub-step 4b — Identify which side is mispricing**
For the divergent pair, ask which asset class is moving with current macro reality and which is not.

Methodology:
- Check `macro-regime-id` output — which asset class would the active regime support?
- Check fundamental data — is there company/sector-specific news driving one side?
- Check positioning data (CFTC COT, ETF flows, options) — which side has unusual positioning?
- Check who participates in each market — is one being driven by retail FOMO while the other reflects institutional discipline?

**Sub-step 4c — Hypothesis on why divergence**

Common patterns:
- **Information lag:** one side responding to news the other hasn't fully integrated
- **Forced flows:** one side being moved by mechanical (non-fundamental) flows
- **Regime shift:** divergence is the leading indicator of regime change
- **Manipulation:** prices artificially set in one market (rare but exists in administrative-currency cases)
- **Different time horizons:** different participants seeing different time scales

### Step 5 — Cross-reference with current ОСП analyses

If divergence relates to a topic with active ОСП analysis or watchlist scenario, integrate findings. Often the divergence provides the strongest signal for or against a scenario.

If divergence is novel — not connected to current analyses — it may warrant new watchlist entry or new analysis.

### Step 6 — Implications for forecasts

For each significant verified divergence, articulate forecast implication:
- What outcome does this divergence suggest?
- What confidence (calibrated)?
- What time horizon for resolution?
- What indicators would confirm or refute?

### Step 7 — Communicate

Daily: log divergences in archive
Weekly: summary to owner of significant divergences and their implications
Monthly: review of divergence track record (which divergences resolved as predicted, which didn't)

## Worked example

### Scenario: April 2026, Iran-war active phase

**Observed divergences (during war):**

**Divergence 1: Defense equities (ITA) -12% while war ongoing**
- Normal: defense should rise during war
- Reality: ITA -12% during peak conflict, RTX -11% post-earnings, LMT -13%
- Investigation:
  - Macro regime: stagflation forming, equity multiples generally pressured
  - Positioning: institutional positioning had been long defense going into war (Rheinmetall, Lockheed accumulated 2024-2025)
  - Hypothesis: peak demand priced in BEFORE war started (anticipatory positioning); war's onset is exit signal for early longs; current price reflects expectation of ceasefire
- Implication: smart money positioned for de-escalation/freeze, not escalation
- Forecast: probability of ceasefire/freeze in Iran scenario higher than mainstream narrative suggests
- Confidence: medium-high

**Divergence 2: Gold -20% while war ongoing**
- Normal: gold should rise on geopolitical stress
- Reality: gold -20% from peak ($6,500 → $5,180)
- Investigation:
  - Real rates: rising (Fed reluctant to cut despite slowing growth)
  - Dollar: strengthening (haven flows)
  - Positioning: retail had massive long gold positioning; institutional unwinding
  - ETF flows: GLD outflows accelerating
- Implication: monetary stress hypothesis (gold as currency debasement hedge) is being unwound; positioning now favors lower gold short-term
- Forecast: gold weakness continues short-term, but structural fundamentals (debt cycle, fiat regime stress) remain supportive long-term
- Confidence: medium

**Divergence 3: $580M oil short 15 minutes before Trump pause announcement**
- Normal: such a position would not appear coincidentally
- Reality: position appeared, was profitable
- Investigation: insider positioning, almost certainly
- Implication: government-level information asymmetry exists in current environment
- Forecast: continued state-level insider trading should be expected; positioning data has higher information content than usual
- Confidence: high (specific event with specific data)

### Tree projection enhancement:

These divergences enrich the `decision-tree-forecast` for the Iran war analysis:
- Higher probability on de-escalation paths (Defense divergence)
- Lower confidence on geopolitical-stress positioning (Gold divergence)
- Heightened attention to positioning data as information channel (insider trading divergence)

## Common false positives

Some divergences are not signal:

- **Currency mechanics:** USD strength when global stress sometimes due to mechanical USD funding stress, not fundamental view
- **End-of-quarter rebalancing:** institutional flows can create temporary divergences without informational content
- **Earnings season:** sector-specific moves during earnings can decorrelate from broader market without macro implications
- **Index reconstitutions:** specific sector adjustments due to index changes
- **Single large trader:** sometimes one large fund unwinding creates divergence in specific markets

The quality of investigation distinguishes signal from these false positives.

## Anti-patterns

- **Ignoring divergences as noise.** Every divergence is investigated. Most are explained as noise after investigation. The minority that aren't are highest-value signals.
- **Confirmation by single direction.** Looking for divergences only that confirm prior view. Discipline: monitor pairs in both directions, investigate any divergence regardless of which side it favors.
- **Static correlation baseline.** Asset correlations change by regime. Bond-equity correlation flipped sign post-2000. Static historical baseline can be wrong; baseline should be updated as regime evolves.
- **Single-pair monitoring.** Looking at only S&P-bond divergence misses many signals. Comprehensive monitoring across asset pairs essential.
- **No follow-through.** Identifying divergence without forecasting resolution. Divergences are diagnostic — forecasts are what they're for.
- **Short-term resolution expectation.** Divergences sometimes persist for months before resolving. Discipline of patience.
- **Ignoring positioning data.** Divergences often reveal which side has unusual positioning. Without positioning analysis, hypothesis on which side is wrong is incomplete.

## Output template

```
─── CROSS-ASSET DIVERGENCE ANALYSIS ───

Date: <ISO>
Active divergences (today):

DIVERGENCE 1: <Asset A vs Asset B>
Normal relationship: <description>
Current observation: <data>
Persistence: <single-day | multi-day | weeks>
Significance: <standard deviations from baseline>

Investigation:
- Regime context: <relevant macro regime info>
- Fundamental check: <recent news that might explain>
- Positioning data: <unusual positioning>
- Hypothesis: <which side is mispricing and why>

Implication: <forecast based on divergence>
Confidence: <low | medium | high>
Resolution time horizon: <weeks | months>
Confirmatory indicators: <what would confirm forecast>
Refuting indicators: <what would refute>

Linked to ОСП analyses: <IDs if applicable>
Linked to watchlist scenarios: <IDs if applicable>

[Repeat for each active divergence]

Daily summary:
- New divergences flagged today: <count>
- Persistent divergences (>3 days): <count>
- Divergences resolved: <count, with outcome>

Highest-conviction signals:
- <Top 1-3 divergences with forecast implications>
```

## Integration with ОСП and infrastructure

**Continuous skill** — runs daily, not per-analysis.

**Used by:**
- `iw-watchlist` (divergences feed into indicator activations)
- `decision-tree-forecast` (divergence implications inform fruit probabilities)
- `asymmetric-bet-structure` (divergences identify mispriced asymmetric opportunities)
- `macro-regime-id` (divergences are leading indicators of regime shifts)

**Stored:** dedicated `CrossAssetDivergence` Prisma model. Daily scan results, with multi-day persistence flagging, investigation notes, forecast implications, resolution tracking.

**Scheduler:**
- Daily 18:00 (after market close): full scan and update
- Weekly Monday: summary with persistence analysis
- Monthly: track record review (which divergences resolved as predicted)

This skill is one of the **highest-value-per-effort** in the entire ОСП toolkit. Most divergences resolve in weeks to months with clear implications; the analyst who systematically investigates them gets a steady stream of high-confidence signals that pure-fundamental analysts miss.

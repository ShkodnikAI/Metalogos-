---
name: order-of-battle
description: Tier 3 special technique. Pentagon-style systematic tracking of physical assets, capabilities, deployment patterns of major actors in conflict, competition, or strategic posture. Applied to military OOB (forces, weapons, logistics), industrial OOB (production capacity, supply chains, talent concentrations), financial OOB (capital reserves, leveraged positions, funding sources). Activated for high-stakes analyses where the gap between claimed capacity and physical capacity matters decisively.
---

# Order of Battle — Counting What's Physically There

Public statements describe what actors claim. Order of battle (OOB) describes what they physically have. The difference is often decisive — and almost always missed by analysts who don't do the counting.

Pentagon's J2 has maintained OOB tracking for foreign militaries since WWII. Modern application extends beyond military to industrial, financial, technological, and human capital. Wherever the gap between claimed capacity and physical reality matters, OOB analysis surfaces it.

This is a Tier 3 skill — applied selectively for high-stakes analyses, not routinely. The cost (research time) is significant; the value (knowing actual versus claimed capability) is decisive when it matters.

## Prerequisites

- High-stakes analysis where physical capacity matters
- Access to relevant tracking data (open sources mostly sufficient for major actors)
- Time investment justified by analysis stakes

## Core principle

> Capacity is physical. Statements are political. When statements diverge from physical reality, statements are managed for effect; physical reality eventually asserts itself.

## OOB types

**Military OOB:**
- Force composition (units, equipment, readiness)
- Logistics (supply chains, fuel, ammunition stocks)
- Personnel (training levels, attrition, replacement rates)
- Geographic deployment
- Production capacity for replacement weapons

**Industrial OOB:**
- Production capacity by sector
- Supply chain dependencies
- Bottleneck identification
- Talent concentrations
- Tooling and equipment availability

**Financial OOB:**
- Reserves (composition and liquidity)
- Leveraged positions
- Funding sources and stability
- Counterparty exposures
- Off-balance-sheet commitments

**Technological OOB:**
- R&D capacity
- Patent portfolios
- Engineering talent
- Tooling for next-generation production

**Human capital OOB:**
- Specific expertise concentrations
- Generational pipeline (training new specialists)
- Retention vs emigration patterns
- Critical-personnel single points of failure

## The procedure

### Step 1 — Define what to count

For the analysis topic, identify what physical capacity matters. Not everything; the specific assets relevant to the strategic question.

For a war analysis: weapons, ammunition, personnel, logistics
For a corporate analysis: production capacity, supply chains, talent, tooling
For a financial analysis: reserves, leverage, funding stability

### Step 2 — Establish counting methodology

How will assets be counted? Sources:
- Open sources: SIPRI databases, IISS Military Balance, satellite imagery, financial filings, customs data
- Specialized: industry analysts, trade publications, academic estimates
- Comparative: similar systems' baselines

Methodology must be reproducible — another analyst should arrive at similar count.

### Step 3 — Conduct count

Build inventory:
- What's there now
- What's been added recently
- What's been lost recently
- What's expected to arrive

For depth analyses, count by location/unit/category rather than just totals.

### Step 4 — Compare to claimed capacity

Statements claim X capability. Count shows Y. Gap analysis:
- Are claims accurate?
- Are claims understatement (hidden capacity)?
- Are claims overstatement (Potemkin capacity)?
- What does the gap pattern reveal about strategic intent?

### Step 5 — Identify dynamics

Static count is one snapshot. Dynamics matter more:
- Production rates vs consumption rates
- Replacement capability vs attrition
- Trajectory of key indicators
- Sustainability under stress

### Step 6 — Assess sustainability

Given current stocks + production rates + consumption rates: how long can current operations continue? What stresses would break sustainability?

### Step 7 — Translate to forecast

OOB analysis translates to forecast through:
- What can actor actually do (vs what they claim)?
- For how long?
- Under what stresses do operations break down?
- What capacity expansion would change the picture?

## Worked example — Russian munitions OOB during Ukraine war

**What to count:** key weapons systems consumption vs production rates.

**Findings (illustrative, mid-2025):**
- Artillery shells: pre-war stocks ~2M, consumption 1M+/year, production ~1.5M/year (post-mobilization), net trajectory toward depletion within 24-36 months
- Tanks: pre-war stocks plus mothballed vehicles refurbished; refurbishment rate declining as older models exhausted
- Precision munitions: consumption exceeded production from war start; relying on imports from North Korea, Iran
- Personnel: combat losses substantial; replacement through mobilization but training quality declining

**Implications:**
- Sustainability of current operations: 18-30 months without escalation in support
- Stress points: precision munitions first (reliance on imports = vulnerability), then artillery
- Forecast: pressure to negotiate or escalate (acquire new sources) increases over 2026

**Compare to claimed capacity:** Russian official statements claim sustainable indefinite operation. OOB analysis shows specific time-bounded constraints.

This gap is decisive intelligence — informs forecasting both Russian behavior (more aggressive negotiation push as constraints bind) and resolution timelines.

## Anti-patterns

- **Total counts without distribution.** Knowing X total tanks is less useful than knowing locations, units, readiness states.
- **Static analysis.** Single snapshot misses dynamics. Production-consumption-attrition rates matter more than current totals.
- **Believing claims.** OOB exists precisely because claims are unreliable. Discipline against accepting claimed capacity at face value.
- **Single-source counts.** Use multiple sources — SIPRI, IISS, commercial intelligence, satellite imagery — and reconcile differences.
- **Ignoring quality differences.** 1000 modern tanks ≠ 1000 obsolete tanks. Capability adjustments matter.
- **Industrial OOB focused only on big firms.** Critical capacity often concentrates in small specialty suppliers — counting only majors misses bottlenecks.

## Output template

```
─── ORDER OF BATTLE ANALYSIS ───

Subject: <topic>
Analysis purpose: <what strategic question this supports>
Sources used: <list>

INVENTORY:
Category 1: <name>
  Current stocks: <number with confidence range>
  Recent additions: <number, source>
  Recent losses/consumption: <number, source>
  Production rate: <units per period>
  Consumption rate: <units per period>
  Net trajectory: <growing/stable/depleting>

Category 2: <same structure>
[continue for each relevant category]

CLAIMED CAPACITY VS ACTUAL:
- <Specific claim by subject>: actual capacity <verified state>
- Gap analysis: <understatement/overstatement and pattern>

SUSTAINABILITY:
Current operations sustainable for: <time range>
Key stress points: <list ordered by severity>
Mitigation possibilities: <list>

FORECAST IMPLICATIONS:
- <What subject can actually do>
- <Time constraints>
- <Behavioral predictions from constraints>
```

## Integration with ОСП

Tier 3 — invoked for depth-3+ analyses where physical capacity matters. Auto-trigger conditions:
- War or military conflict analyses
- Major industrial competition (chip wars, EV competition, etc.)
- Financial stress scenarios involving specific institutions
- Technology race analyses (AI compute, biotech, etc.)

Output stored in Analysis record under `orderOfBattle` field. For ongoing situations (active war, sustained competition), OOB is updated periodically as data refreshes.

This skill, when applied, produces the most concrete grounding available for analyses. Other skills handle structure, narrative, dynamics. OOB handles the physical reality these operate within.

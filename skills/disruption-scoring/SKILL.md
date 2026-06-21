---
name: disruption-scoring
description: Quantitative assessment of disruption probability for a tracked technology — likelihood that it will displace incumbent industry/practice within specific time horizons (1/3/5/10 years). Based on five components — performance ratio vs incumbent, cost trajectory, patent activity concentration, talent flow, investment patterns. Filters genuine disruption candidates from noise; foundation for identifying durable shifts vs cyclical hype.
---

# Disruption Scoring — Distinguishing Real Threats From Noise

Most "disruption" claims are noise. The technology is interesting but won't actually displace incumbent industry. Some claims are real but too early — disruption coming but on longer horizon than claims suggest. A few claims are real and timely — these are where opportunity lives.

Distinguishing the three categories requires systematic scoring. This skill provides the discipline — five component evaluation that produces probability estimates for disruption at multiple time horizons.

## Prerequisites

- Technology identified with `hype-cycle-mapping` stage
- Defined incumbent industry/practice the technology might disrupt
- Available data on five scoring components

## Core principle

> Disruption requires multiple conditions converging: performance, cost, ecosystem, talent, capital. Single-condition strength is insufficient. Multi-condition convergence with right timing produces genuine disruption. Quantifying the components prevents narrative-driven over-prediction.

## The five scoring components

**1. Performance ratio vs incumbent**

How much better does the new technology perform on key metrics than the incumbent? The classic Christensen threshold is **>10x improvement** for genuine disruption potential.

- 1-2x: similar to incumbent — incremental, not disruptive
- 2-5x: meaningfully better — competitive but not disruptive
- 5-10x: significantly better — disruption possible
- 10-100x: dramatically better — disruption likely
- >100x: revolutionary — disruption nearly certain if other components align

Performance must be measured on metrics customers/users actually value, not just technical metrics. A technology 100x better at a metric nobody cares about is not disruptive.

**2. Cost trajectory**

How fast is the technology's cost declining? Cost decline drives adoption.

- Flat or rising costs: not disruptive (incumbent can match)
- 5-10% annual decline: slow — incumbent has time
- 10-30% annual decline (Wright's Law typical): meaningful
- 30-50% annual decline: rapid — incumbent struggling to keep up
- >50% annual decline: dramatic — disruption accelerating

Cost decline typically follows learning curves — Wright's Law (~15-25% per doubling) is base case; faster declines indicate factors beyond simple scaling.

**3. Patent activity concentration**

How concentrated is intellectual property activity?

- Diffuse activity (no clear leaders): early Innovation Trigger or weak technology
- Multiple specialized players: competition driving improvement
- Few dominant players accumulating IP: maturing technology with specific winners
- Single dominant player: monopoly forming or technology platform

Concentration trend matters as much as current state. Concentration increasing = winners emerging.

**4. Talent flow**

Where is top talent migrating?

- Talent leaving the field: technology in decline or fundamentally limited
- Talent stable: established but not growing
- Talent inflow from adjacent fields: growth phase
- Talent inflow from incumbent industry: incumbent recognizing threat
- Top talent specifically (not just any): high signal

Specific patterns matter:
- Researchers from major labs joining startups: pre-Peak signal
- Engineers from established companies joining: Slope signal
- Reverse flow (return to incumbent): Trough signal

**5. Investment patterns**

How is capital flowing?

- Government-only / specialized VC: Innovation Trigger
- Hype-driven VC, high valuations: Peak — overcapitalized
- VC declining, contraction: Trough
- Disciplined VC, real revenue traction: Slope
- M&A activity, strategic capital: Plateau

Investment quality matters more than quantity. $100M from sophisticated investor different from $100M from hype-following retail-adjacent capital.

## The scoring methodology

Score each component on 1-10 scale:

**Performance ratio score:**
- 1-2: 1: incremental
- 2-5x: 4: meaningful
- 5-10x: 6: significant
- 10-100x: 8: dramatic
- >100x: 10: revolutionary

**Cost trajectory score:**
- Flat: 1
- 5-10% decline: 3
- 10-30% decline: 5
- 30-50% decline: 7
- >50% decline: 9

**Patent concentration score:**
- Diffuse: 2
- Multiple competitors: 4
- Few dominant: 7
- Single dominant: 8 (winner identifiable; maybe overstating threat)
- Monopolistic with broad applicability: 9

**Talent flow score:**
- Outflow: 2
- Stable: 4
- Inflow: 6
- Inflow from incumbents: 8
- Top-talent specifically: 9

**Investment score:**
- Specialized only: 3
- Hype peak: 5 (high but unstable — could be 8 if separated from hype)
- Contraction: 4
- Disciplined revenue-based: 8
- M&A maturity: 7

**Disruption score = average of five components**

Threshold interpretation:
- 6+: genuine disruption candidate
- 7+: high-probability disruption
- 8+: imminent or in-progress disruption

## Time horizon adjustment

Score gives probability of eventual disruption, not timing. Timing comes from:

- **Cost trajectory:** when does cost cross adoption threshold? Linear extrapolation provides estimate
- **Performance gap closure:** when does performance reach customer threshold for switching?
- **Ecosystem maturity:** when is supply chain, talent pool, supporting infrastructure ready?
- **Regulatory environment:** when do regulators allow scaled deployment?

Time horizons:
- 1-year disruption: requires score >8 + already-deploying products
- 3-year disruption: score 7+ with cost trajectory crossing threshold within 3y
- 5-year disruption: score 6+ with multiple favorable factors aligned
- 10-year disruption: score 5+ with at least 2 strong components

## The procedure

### Step 1 — Define scope

What technology? What incumbent practice/industry? Specific scope is essential.

### Step 2 — Score each of five components

Gather data, score 1-10 with brief justification per component.

### Step 3 — Compute aggregate disruption score

Average of five components. Note any outlier components (extremely high or low) and investigate why.

### Step 4 — Estimate time horizon

Based on cost trajectory, performance gap, ecosystem readiness — what time horizon for disruption?

### Step 5 — Identify watchlist indicators

For tracked technologies, what specific events would update the score? List indicators that would push score up or down.

### Step 6 — Cross-check with hype cycle stage

Is the score consistent with hype cycle stage? Common patterns:
- Innovation Trigger: scores 4-7 typical
- Peak: often inflated scores (excitement-driven assessment)
- Trough: often deflated scores (disappointment-driven)
- Slope: scores stabilize, more reliable
- Plateau: scores trend down (already disrupted, now mature)

If score and stage seem inconsistent: investigate.

## Worked examples

**LLM-based code generation (May 2026):**
- Performance ratio: 6 (significantly faster code production but quality varies; 5-10x for routine tasks)
- Cost trajectory: 7 (compute costs declining rapidly, ~50% annually)
- Patent activity: 7 (concentrating around few major model providers)
- Talent flow: 8 (top engineering talent flowing in from incumbents)
- Investment: 7 (huge but maturing past peak hype)
- Aggregate: 7.0 — high disruption probability
- Time horizon: 3-5 year disruption of substantial portion of routine coding work

**Solid-state batteries:**
- Performance ratio: 8 (energy density ~2-3x lithium-ion, safety dramatically better)
- Cost trajectory: 4 (still very expensive; cost decline early)
- Patent activity: 5 (multiple competitors, no clear winner yet)
- Talent flow: 7 (significant inflow from battery industry)
- Investment: 6 (substantial but disciplined)
- Aggregate: 6.0 — disruption candidate
- Time horizon: 5-10 year disruption of EV battery market (cost trajectory the key constraint)

**Quantum computing for enterprise applications:**
- Performance ratio: 9 (exponential advantage on specific problems)
- Cost trajectory: 3 (extremely expensive, infrastructure-dependent)
- Patent activity: 7 (concentrating around IBM, Google, others)
- Talent flow: 6 (specialized, growing)
- Investment: 6 (specialized VC active)
- Aggregate: 6.2 — disruption candidate
- Time horizon: 10+ year for enterprise (cost the binding constraint); narrower applications sooner

## Anti-patterns

- **Hype-driven scoring.** Inflating scores during Peak hype. Use objective indicators only.
- **Single-component focus.** "Performance is incredible" → high score regardless of cost, talent, etc. All components matter.
- **Static scoring.** Scores change over time; periodic re-scoring required.
- **Confusing potential with current state.** Technology might score 8 in 5 years but currently scores 5. Score current state, then estimate trajectory.
- **Ignoring incumbent response.** Incumbent improvements during disruption attempt can change score. Sustained 5x performance ratio more meaningful than peak 10x quickly closed.
- **Single-application focus.** A technology might disrupt one industry but not another. Score per disruption target, not per technology.

## Output template

```
─── DISRUPTION SCORING ───

Technology: <specific>
Incumbent target: <specific industry/practice>
Date: <ISO>

COMPONENT SCORES:
1. Performance ratio: <1-10>
   Evidence: <brief>
2. Cost trajectory: <1-10>
   Evidence: <brief>
3. Patent activity: <1-10>
   Evidence: <brief>
4. Talent flow: <1-10>
   Evidence: <brief>
5. Investment patterns: <1-10>
   Evidence: <brief>

AGGREGATE DISRUPTION SCORE: <average>

CLASSIFICATION:
- 5-6: Possible disruption candidate
- 6-7: Probable disruption
- 7-8: High-probability disruption
- 8+: Imminent/in-progress disruption

TIME HORIZON ESTIMATES:
- 1-year disruption probability: <%>
- 3-year: <%>
- 5-year: <%>
- 10-year: <%>

KEY DEPENDENCIES:
- <Most important factor for full disruption>
- <Second most important>

WATCHLIST INDICATORS:
- <Event that would raise score>
- <Event that would lower score>
- <Event that would accelerate timeline>

CROSS-CHECK WITH HYPE CYCLE: <consistent | inconsistent>
```

## Integration with Лаборатория знаний

Used after `hype-cycle-mapping` for any technology being seriously evaluated.

**Updated:**
- Quarterly for tracked technologies
- After major events (product launches, funding rounds, regulatory shifts)
- After significant component changes (cost breakthrough, talent shift)

**Triggers other skills:**
- High score + favorable timeline triggers `inflection-detection`
- High score triggers cross-references with ОСП for investment analysis
- Sustained high score across quarters triggers Лаборатория alert to ОСП

Stored in KnowledgeArtifact Prisma model with current score, score history, time-horizon estimates.

---
name: technical-economics-validation
description: Validates the economic claims of a technical project — bill of materials (BOM) analysis, capex/opex realism, unit economics, path to break-even, capital efficiency. Even when meeting is "purely technical," money is always the subtext. Forces specificity on cost claims, identifies hidden costs typical for the technology category, anchors against industry benchmarks. Output: economic validation cards integrating into combat questions.
---

# Technical Economics Validation — Where The Money Story Breaks

The technical pitch usually emphasizes capability and timeline. The economics is mentioned in passing — "we'll be cost-competitive at scale." That phrase, said confidently, often hides where the project actually fails commercially. Strong technology with broken economics deploys nowhere.

This skill is the discipline of forcing specificity on the economic claims and testing them against domain anchors. Result: questions that move "we'll be cheap at scale" from confident assertion to either credible plan or evident gap.

## Prerequisites

- `rapid-domain-immersion` providing numerical anchors (Layer 4)
- `project-deconstruction` providing project shape
- Available materials including any economic claims (decks, projections, pitches)

## Core principle

> Technical projects have economic shape that must work for deployment. The discipline is not to evaluate whether "the numbers look good" but to force specificity in claims (BOM, capex, opex, unit economics, capital required) and test against industry anchors. Most projects collapse not on technical impossibility but on economic reality their pitch obscures.

## What to validate

Six economic dimensions for any technical project:

### 1. Bill of Materials (BOM)

The detailed cost of all components/inputs to produce one unit of output (or one unit of capacity).

**For a hardware product:** all physical components and their costs
**For a service:** infrastructure costs per delivery
**For a research output:** material and equipment costs per study

**What to test:**
- Is BOM disclosed at all? If not, that's a finding.
- Does claimed BOM include all components or are some omitted?
- Are component costs current or assumed for future?
- What's the assumed manufacturing scale for those costs (often "at scale" without specifying scale)?

### 2. Capex (capital expenditure)

Upfront investment to deploy capacity.

**What to test:**
- For first unit/facility vs nth unit (always vastly different)
- Includes all infrastructure (utilities, regulatory, site prep)?
- Compared to incumbent technology baseline
- Funding visibility — do they have funded plan to reach claimed capex?

### 3. Opex (operating expenditure)

Ongoing costs to operate at deployment.

**What to test:**
- Maintenance, replacement, repairs
- Labor costs (often understated for early deployment when labor specialized)
- Utilities and inputs
- Regulatory compliance ongoing
- Insurance
- Compared to revenue streams

### 4. Unit economics

Per-unit revenue and cost relationship.

**What to test:**
- Revenue per unit (with assumptions)
- Cost per unit at various scales
- Gross margin at maturity vs current
- Path of unit economics from launch to maturity
- Sensitivity to volume assumptions

### 5. Capital efficiency

How much capital required to reach milestones.

**What to test:**
- Capital per unit of demonstrated capability
- Capital to reach revenue
- Capital to reach profitability
- Capital required vs raised vs revenue projections

### 6. Path to break-even

When does cumulative revenue exceed cumulative cost?

**What to test:**
- Specific year for break-even
- Assumptions (revenue ramp, cost reduction curve)
- What happens if assumptions miss by 50%
- Cumulative capital required to break-even

## The procedure

### Step 1 — Extract all economic claims from materials

Read materials for any number related to cost, capital, revenue, or economics. List each with context.

Common findings: surprisingly few specific economic numbers in pitches. That's already information.

### Step 2 — Calibrate against domain anchors

Using numerical anchors from immersion (Layer 4), test each claimed number:
- Is it within typical range?
- Is it at low end of range (claiming aggressive cost reduction)?
- Is it below feasible range (suggesting fantasy or undisclosed assumption)?
- Is it above range (suggesting different scale or different category)?

### Step 3 — Identify missing economic claims

For complete economic picture, what claims should exist but don't? Common gaps:
- BOM never broken down
- Capex for first vs nth unit not distinguished
- Opex categories incomplete (often missing maintenance, replacement)
- Unit economics asserted without showing the model
- Path to break-even hand-waved

### Step 4 — Identify hidden costs

For the technology category, what costs are typically hidden in early-stage pitches?

**Examples by category:**
- Hardware: tooling, certification, returns, support, end-of-life
- SaaS: customer acquisition cost, churn, support, infrastructure scaling
- Biotech: clinical trials, regulatory, manufacturing scale-up, supply chain
- Construction: permitting, environmental, contingency, financing
- Energy: decommissioning, insurance, grid integration, fuel/feedstock
- AI: training compute, inference scaling, data acquisition, talent retention

Hidden costs are not malice typically — they're not yet visible to early-stage teams. But they materialize at deployment.

### Step 5 — Compare to incumbent baseline

For deployment to make economic sense, project usually must beat incumbent on cost (sometimes on cost per unit of value, not absolute cost). What's the incumbent baseline? How does claimed deployment economics compare?

Often projects compare to **today's** incumbent — but incumbent improves too. Five-year comparison should include reasonable incumbent improvement.

### Step 6 — Stress test assumptions

For top economic claims, what if assumption is 50% off?
- BOM 50% higher
- Volume 50% lower
- Time to break-even 50% longer
- Capital required 50% more

Does project still make economic sense, or does it break entirely?

### Step 7 — Generate economic combat questions

For the most decisive economic claims, formulate combat questions feeding into Phase 4. These tend to be powerful because:
- Specific numbers force specific answers
- Industry anchors prevent waving away
- Stress tests reveal sensitivity

### Step 8 — Output economic validation

Structured analysis of economic claims with credibility assessment per dimension.

## Worked example — for FusionCorp

**Economic claims extracted:**
- "$3 billion per gigawatt for commercial deployment"
- "Cost-competitive with existing baseload"
- "10-year ROI for utility customers"
- "Profitable at scale"

**Anchor comparison:**
- $3B/GW — current ITER trajectory and similar projects suggest $5-7B for first-of-kind. Claim is 50%+ below baseline.
- "Cost-competitive baseload" — coal $1B/GW, natural gas $1B/GW, nuclear $5-7B/GW, solar+storage $1.5-2B/GW. Their $3B is positioned between nuclear and solar — feasible if claim holds.
- "10-year ROI" — typical utility infrastructure is 20-30 year ROI. Implausibly fast.

**Missing claims:**
- No BOM breakdown for fusion plant
- No distinction between FOAK ($X) and NOAK ($Y) — must be different
- Operating costs not detailed (maintenance, tritium replacement, materials replacement)
- Decommissioning costs not mentioned
- Insurance for first-of-kind nuclear-equivalent facility not addressed

**Hidden costs typical for category:**
- First-of-kind regulatory approval costs (specifically — NRC has no precedent)
- First wall replacement every X years (cost per replacement substantial)
- Tritium handling specialized facilities and personnel
- Decommissioning provisioning required
- Insurance for nuclear-adjacent facility

**Stress tests:**
- BOM/capex 50% higher: $4.5B vs $3B claim — still feasible vs nuclear, but margin to alternatives erodes
- Volume 50% lower (10 plants instead of 20 by 2035): unit economics worse, capital efficiency drops
- Time to break-even 50% longer (15-year ROI instead of 10): becomes uncompetitive vs solar+storage
- Capital required 50% more: probably exceeds investor capacity, requires government

**Top economic combat questions:**

1. "Walk me through your $3B per gigawatt. For first-of-kind plant — is that the FOAK number or the NOAK number? What's the ratio?"
2. "What's your operating cost target — including maintenance, tritium, materials replacement, decommissioning provisioning?"
3. "Coal is $1B/GW capex, solar+storage trending to $1.5B/GW. By the time you deploy in 2035, what's incumbent baseline you're competing against?"
4. "What's your 10-year ROI assumption based on? Typical utility infrastructure is 20-30 year ROI."

## Anti-patterns

- **Accepting "cost-competitive at scale" without numbers.** This phrase is universal hedge. Force specificity.
- **Single-point estimates.** "BOM is $X" without range. Stress-test the range.
- **No incumbent baseline.** Claims must be relative to alternatives. Without baseline, claims are floating.
- **Static incumbent.** Comparing to today's incumbent ignores incumbent improvement over deployment timeframe.
- **FOAK vs NOAK confusion.** First-of-kind and nth-of-kind costs differ by 2-5x. Treating them as same is fundamental error.
- **Missing sensitivity analysis.** What if assumptions are 50% off — does project survive?
- **Confusing technical with economic feasibility.** Both must work; technical alone is insufficient.

## Output template

```
─── TECHNICAL ECONOMICS VALIDATION ───

Project: <identifier>

ECONOMIC CLAIMS EXTRACTED:
- <Claim 1>: source, context
- <Claim 2>: source, context

ANCHOR COMPARISON:
- <Claim>: anchor range, position [low/mid/high/below feasible]
- <Claim>: similar

MISSING CLAIMS (gaps):
- <What should exist but doesn't>: implication
- [list]

HIDDEN COSTS (category-typical):
- <Cost type 1>: how typically appears, magnitude
- [list]

STRESS TESTS:
- BOM +50%: project still works? [yes/marginally/no]
- Volume -50%: project still works? [same]
- Time-to-break-even +50%: project still works? [same]
- Capital +50%: project still works? [same]

INCUMBENT BASELINE:
- Today's incumbent: <description>
- Incumbent at deployment date: <projected>
- Project economics vs incumbent at deployment: <analysis>

DECISIVE ECONOMIC GAPS:
- <Gap 1>: question to ask
- [list of 2-4]

ECONOMIC COMBAT QUESTIONS:
1. <Question with expected response patterns>
2. [more]
```

## Integration with Expert protocol

Tier 2 — invoked when:
- Meeting is investor-style or economic-decision oriented
- Project makes specific economic claims
- Owner is funding/investor capacity

Output integrates into combat questions (Phase 4) and bullshit detection (parallel skill).

Stored in ExpertBriefing under `economicsValidation` field.

This skill prevents the most common technical due diligence error: technically correct, economically infeasible. Strong project must clear both bars; weak project clears one and fails on the other.

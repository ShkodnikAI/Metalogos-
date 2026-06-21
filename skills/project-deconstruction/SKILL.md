---
name: project-deconstruction
description: Decomposes a specific project/proposal/presentation against the domain understanding from rapid-domain-immersion. Identifies what they actually claim vs what they show, what's substantive vs cosmetic, where their approach fits in the schools-and-debates landscape, what's their differentiation (real or claimed). Output is a structured breakdown that makes the project's real shape visible — distinct from how they want to present it.
---

# Project Deconstruction — Seeing The Project Behind The Pitch

The pitch is what they've prepared. The project is what they've actually built. The two often differ substantially. Pitch maximizes positives, omits negatives, frames in best light. Project has actual technical decisions, actual capabilities, actual limitations.

This skill is the discipline of separating pitch from project. Using the domain immersion from Phase 1 as foundation, it deconstructs what's actually being presented to identify the real shape of what they have.

## Prerequisites

- `rapid-domain-immersion` completed for the domain
- Source materials available: their pitch deck, website, papers, demos, public statements
- Time budget appropriate to depth (15-30 minutes for quick, 45-90 for standard, 2+ hours for deep)

## Core principle

> The pitch is the project's marketing layer. Beneath it sits the actual technical shape — the choices made, capabilities built, limitations accepted. The discipline is to separate the two through structured questioning. What they present is data; what they don't present is also data; the gap reveals the real project.

## What to deconstruct

For any project pitch, six dimensions to extract:

**1. Core claim**
What is the central technical claim? Not the value proposition — the technical assertion.
"Our AI improves productivity 50%" is value proposition. The technical claim might be: "Our model achieves X benchmark score using Y architecture trained on Z data."

**2. Specific technical approach**
Within the domain, where does their approach sit? Which school or combination? What's their specific implementation choice?

**3. Demonstrated vs claimed**
What have they actually shown in demos, papers, deployments? What do they claim but not demonstrate?

**4. Differentiation (real vs claimed)**
What do they say makes them different/better? Is the differentiation real (quantified, demonstrated) or marketing (asserted, vague)?

**5. Scope of operation**
What conditions does it work under? Lab vs field, specific vs general, ideal vs adversarial inputs?

**6. Stage of maturity**
Where in the development cycle? Concept, prototype, alpha, beta, production, scaled deployment?

## The procedure

### Step 1 — Inventory their materials

Collect everything publicly available:
- Pitch decks (if shared)
- Company website
- Whitepapers
- Published papers (journals, arXiv)
- Patent applications
- Press releases over time
- Interviews with founders/researchers
- Conference talks (often more candid)
- Customer testimonials (if any)

### Step 2 — Extract central technical claim

Read materials looking specifically for the most concrete technical assertion. Distinguish from value claims. If only value claims appear, that's already a finding — they're not making testable technical claims.

### Step 3 — Map their approach onto domain landscape

Using Layer 3 from immersion (schools and debates), identify:
- Which school does their approach fit?
- Are they pure-school or hybrid?
- Their position relative to mainstream
- Specific technical choices that distinguish them

### Step 4 — Catalog demonstrated vs claimed

Build two columns:
- **Demonstrated:** specific results shown with verifiable evidence (paper, demo, deployment data)
- **Claimed:** assertions without specific evidence

The ratio matters. Strong projects have most claims demonstrated. Weak projects have many claims with few demonstrations.

For each "claimed but not demonstrated" item, note:
- What demonstration would verify the claim?
- Why might they not demonstrate it? (not yet capable, capability private, expensive to demonstrate, or outright untrue)

### Step 5 — Test differentiation

For each claimed differentiation, ask:
- **Real:** quantified comparison with alternatives, demonstrated advantage
- **Asserted:** "we're better because we have X" without comparing
- **Marketing:** "world-class," "revolutionary," "unique" — empty differentiation language

Real differentiation is rare. Most projects assert differentiation without demonstrating it.

### Step 6 — Assess operational scope

Where does it work, where doesn't it?
- Specific input conditions
- Environmental requirements
- Adversarial inputs (does it break under attack/edge cases?)
- Generalization beyond demo conditions

This is where many projects collapse — works on cherry-picked inputs, fails outside narrow scope.

### Step 7 — Determine maturity stage

Based on evidence, place project on maturity spectrum:
- **Concept:** white paper or talk, no working code/prototype
- **Prototype:** working in lab, not productized
- **Alpha:** early users testing, many bugs/limitations
- **Beta:** broader testing, refining for production
- **Production:** deployed, used by real customers
- **Scaled:** widely deployed, mature

Mismatch between claimed and actual maturity is common red flag.

### Step 8 — Output structured deconstruction

Produce the project profile.

## Worked example — hypothetical fusion startup pitch

**Project:** "FusionCorp" claims commercial fusion electricity by 2030.

### Step 2 — Central technical claim:
After parsing pitch deck and website: "Our compact tokamak using HTS magnets achieves Q=10 in 2027, commercial Q=30 by 2030."

### Step 3 — Approach in domain:
Compact tokamak school (Commonwealth Fusion Systems, Tokamak Energy similar). Specific choices: HTS magnets, deuterium-tritium fuel, conventional plasma physics with novel engineering.

Not novel physics — novel engineering enabled by HTS magnets. This is mainstream-engineering, not novel-physics approach.

### Step 4 — Demonstrated vs claimed:

Demonstrated:
- HTS magnet test at small scale (pictures, basic specs)
- Plasma sustained 30 seconds at moderate temperature (vs ITER target hundreds of seconds at higher)
- Team includes 3 researchers from MIT PSFC

Claimed but not demonstrated:
- Q=10 (not achieved by anyone yet at scale)
- Commercial economics ($3B per GW)
- 2027 timeline for first net energy
- 2030 commercial operation

The gap between demonstrated and claimed is large. This is consistent with industry — fusion claims always exceed demonstrations — but specifics matter for evaluating credibility.

### Step 5 — Differentiation testing:
Claim: "smaller and cheaper than ITER"
Real or asserted? Compact tokamak approach is real differentiation from ITER but **not unique** — Commonwealth Fusion is doing the same. Their specific differentiation vs Commonwealth would need to be technical specifics they don't share.

### Step 6 — Operational scope:
Demos shown only in laboratory. No field deployment. No engineering details about fuel handling, materials lifetime, plant integration.

### Step 7 — Maturity:
Prototype stage at best. Magnet testing at prototype, plasma sustaining at lab/research level. Many years from production. Self-positioning as "near-term commercial" is at maturity inflation.

### Step 8 — Deconstruction output:

Real shape of project: serious-but-early-stage engineering effort using mainstream physics + novel HTS engineering. Comparable to several other private fusion startups. Differentiation from competitors unclear from public materials. Timelines (2027-2030) are aggressive given current demonstration level. Capital required ($1-2B+ to prove technology) substantially exceeds current funding visible.

## Anti-patterns

- **Believing the pitch.** Pitch is designed to convince. Deconstruction must separate the underlying project from the marketing.
- **Reading only what they provide.** Their materials are curated. Outside sources (patents, papers, competitor analyses) provide independent view.
- **Conflating value and technical claims.** "Will revolutionize" is value; "achieves X benchmark" is technical. Different evaluation methods.
- **Skipping demonstrated/claimed split.** This is the core of deconstruction. Without explicit list, evaluation is impressionistic.
- **No differentiation testing.** Accepting claimed differentiation without testing whether it's real, asserted, or pure marketing.
- **Maturity inflation acceptance.** They claim "production ready"; deconstruction shows "alpha." Note the gap.
- **Over-deconstruction without immersion.** Cannot deconstruct without first understanding the domain. Phase 1 (immersion) is prerequisite.

## Output template

```
─── PROJECT DECONSTRUCTION ───

Project name: <identifier>
Source materials reviewed: <list>
Time invested: <duration>

CENTRAL TECHNICAL CLAIM:
<Specific assertion in technical terms, not value proposition>

APPROACH IN DOMAIN LANDSCAPE:
- School/camp: <which from immersion Layer 3>
- Specific choices: <technical decisions>
- Position vs mainstream: <description>

DEMONSTRATED vs CLAIMED:

Demonstrated:
- <Item 1>: <specific evidence, source>
- <Item 2>: <specific evidence>

Claimed but not demonstrated:
- <Item 1>: would be verified by <X>; possibly not demonstrated because <hypothesis>
- <Item 2>: same

DIFFERENTIATION ANALYSIS:
- Claim: <stated differentiation>
  Type: [real | asserted | marketing]
  Evidence (if real): <description>
  Comparison: <vs alternatives>

OPERATIONAL SCOPE:
- Where it works (demonstrated): <conditions>
- Where it claims to work but unverified: <conditions>
- Edge cases / adversarial behavior: <unknown / tested / untested>

MATURITY ASSESSMENT:
- Claimed stage: <what they say>
- Actual stage from evidence: <concept | prototype | alpha | beta | production | scaled>
- Gap analysis: <description>

REAL SHAPE OF PROJECT:
<2-3 sentences describing what this actually is, distinct from how it's pitched>

KEY UNKNOWNS:
- <What we cannot determine from public materials>
- <What we'd need to ask in meeting to determine>
```

## Integration with Expert protocol

This is **Phase 2** of the Expert protocol. Output feeds into:
- `failure-modes-mapping` (uses real shape to identify specific failure modes)
- `combat-questions-design` (questions target the gaps and unknowns identified here)
- `bullshit-detection` (compares pitch claims to deconstructed reality, identifies manipulation)

Stored in ExpertBriefing record under `projectDeconstruction` field. Updated post-meeting if meeting reveals significant new information about project shape.

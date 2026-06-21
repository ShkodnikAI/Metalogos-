---
name: failure-modes-mapping
description: Systematic identification of weak points and failure modes across 7 categories — technical bottlenecks, scaling challenges, supply chain, regulatory, economic, talent dependencies, market. Goes beyond "what could go wrong" to specific named risks tied to the project's specific architecture and choices. Output is targeted: each failure mode has triggering conditions, severity, and probe questions to test whether the project team is aware and addressing.
---

# Failure Modes Mapping — Where Will This Break

Every technology has failure modes. The amateur evaluator doesn't know what they are. The expert knows the catalog. The discipline is structured walkthrough of failure mode categories specific to the project architecture.

This skill is not pessimism — it's systematic risk identification. Strong projects have answers to known failure modes (sometimes the answer is "we accept this risk"; that's still an answer). Weak projects haven't thought about these failure modes, or worse, are unaware they exist.

## Prerequisites

- `project-deconstruction` completed (real shape of project known)
- `rapid-domain-immersion` completed (domain-specific failure modes understood)
- Time to walk through 7 categories systematically (15-30 minutes minimum)

## Core principle

> Failure modes cluster into 7 universal categories that apply across technical projects. Walking through all 7 systematically prevents the common error of focusing only on the 1-2 obvious risks while missing decisive ones in unexamined categories. The strongest team has identified more failure modes than the evaluator and has plans for each; the weakest team is unaware of failure modes the evaluator can identify in 30 minutes.

## The 7 categories

### Category 1 — Technical bottlenecks

**What:** specific technical capabilities that don't yet exist (or don't exist at required performance) but are essential to the project.

**Common patterns:**
- Material that doesn't exist with required properties
- Algorithm that needs accuracy not yet demonstrated
- Component that requires precision not yet achievable
- Interaction between subsystems unsolved
- Measurement/sensing capability missing

**Probe questions:**
- "What's the technical capability you most depend on that isn't yet at production grade?"
- "If [specific known challenge] doesn't get solved, what happens to your timeline?"
- "Walk me through your dependency on [Y]"

### Category 2 — Scaling challenges

**What:** working in lab/prototype is fundamentally different from working at scale. Specific physical, engineering, or operational issues emerge at scale that didn't exist at small.

**Common patterns:**
- Heat dissipation impossible at scale despite OK at lab scale
- Manufacturing yield collapses at production volumes
- Software architecture works for 100 users, not 1M
- Quality variance increases with batch size
- Supply chain can't sustain production rates

**Probe questions:**
- "Your demo runs at scale X; what specifically changes at scale 100X?"
- "What's the largest scale at which [critical subsystem] has been demonstrated by anyone?"
- "Where does your unit economics start working — at what production volume?"

### Category 3 — Supply chain

**What:** dependencies on specific inputs, components, or services from external suppliers.

**Common patterns:**
- Single-source critical components
- Geopolitically risky supply (Taiwan chips, Chinese rare earths, Russian palladium)
- Expensive precursors with limited production capacity globally
- Sustained quality required from suppliers who can't reliably provide
- Custom tooling requiring specific suppliers

**Probe questions:**
- "What's your most critical input that has only 1-2 suppliers globally?"
- "What's your tritium/lithium/rare earth/specific-component story?"
- "If [single supplier] decides to stop selling to you, what's plan B?"

### Category 4 — Regulatory

**What:** governmental, industry, or institutional requirements that must be met for deployment.

**Common patterns:**
- New technology in heavily regulated domain (medical, aerospace, financial)
- Cross-jurisdictional complexity
- Standards bodies haven't approved
- Safety case unproven
- Environmental compliance unclear

**Probe questions:**
- "Walk me through your regulatory pathway — specific approvals required, expected timeline"
- "What FDA/FAA/EMA/SEC/etc. precedent applies?"
- "Has any regulator publicly commented on this category of technology?"

### Category 5 — Economic

**What:** the unit economics, capital requirements, or business model that must work for deployment.

**Common patterns:**
- BOM (bill of materials) too expensive vs alternatives
- Capex per unit too high for market
- Opex including maintenance unsustainable
- Customer acquisition cost too high
- Path to break-even unclear or distant
- Capital required exceeds investor patience

**Probe questions:**
- "What's your detailed BOM cost target, and where are you today?"
- "What's your unit economics at maturity vs today?"
- "How many units of capital do you need to reach profitability?"

### Category 6 — Talent dependencies

**What:** critical capabilities concentrated in specific people whose loss would damage the project.

**Common patterns:**
- Single founder/CTO holds critical knowledge
- Specific PhD researchers irreplaceable for technical depth
- Domain expertise scarce globally
- Recruitment pipeline weak
- High burnout in specialized roles

**Probe questions:**
- "If your CTO/lead researcher left tomorrow, what specifically can't be replaced?"
- "How many people globally could replace [critical specialist]?"
- "What's your retention story for top talent?"

### Category 7 — Market

**What:** assumptions about customer demand, willingness to pay, or competitive dynamics that may not hold.

**Common patterns:**
- Customer doesn't actually want this
- Existing alternatives are "good enough"
- Competitor is faster/cheaper/better
- Switching costs make adoption hard
- Market timing wrong (too early or too late)
- Geographic/cultural fit overestimated

**Probe questions:**
- "Who's actually paying you today, for what specifically?"
- "What's your competitive analysis — top 3 alternatives and how you compare?"
- "What's the customer's current alternative, and why would they switch?"

## The procedure

### Step 1 — Walk through 7 categories systematically

For each category, dedicate at least 5 minutes (more for high-stakes evaluations). Don't skip categories that "seem irrelevant" — those are exactly where blind spots hide.

### Step 2 — Identify specific failure modes per category

For each category, generate concrete failure mode statements specific to this project. Not "supply chain risks" but "if Lockheed stops supplying X-band antennas, this product dies."

Aim for 1-3 specific failure modes per category. 7-21 total failure modes is normal range. Less suggests incomplete walkthrough; more suggests inadequate filtering for relevance.

### Step 3 — For each failure mode, characterize

Three properties:

**Severity:**
- **Catastrophic:** project dies if this materializes
- **Major:** significant delay/cost increase
- **Manageable:** problematic but recoverable

**Probability:**
- **High:** almost certain to occur
- **Medium:** likely if conditions hold
- **Low:** possible but specific triggers needed

**Awareness signal:** what would indicate the team is aware of and addressing this failure mode? (E.g., they have a plan, they've started mitigation, they have a backup)

### Step 4 — Identify the catastrophic-but-unaddressed

The most decisive failure modes are those scoring **high severity AND high probability AND no evidence of team awareness**. These are project killers.

Strong projects have considered these and either solved or accepted them with rationale. Weak projects haven't considered them.

### Step 5 — Generate probe questions for top failure modes

For each top-priority failure mode (catastrophic-and-unaddressed), generate a specific probe question that:
- Tests whether the team is aware
- Cannot be deflected with marketing language
- Has identifiable "good," "uncertain," and "evasive" response patterns

These probe questions feed into `combat-questions-design` skill.

### Step 6 — Output failure modes report

Structured catalog of all identified failure modes with characterization, plus highlighted top failure modes for combat questions.

## Worked example — hypothetical fusion startup

Continuing FusionCorp example.

### Technical bottlenecks:
1. **Plasma stability at long pulse durations.** Current achievement seconds; commercial requires hours/continuous. Severity: catastrophic if not solved. Probability: high (no one has solved this yet at relevant scale). Awareness signal: detailed roadmap with intermediate milestones.

2. **First wall material under 14 MeV neutron flux.** No material currently survives years. Severity: catastrophic. Probability: high. Awareness signal: specific material development partnership.

### Scaling challenges:
3. **HTS magnet manufacturing at multiple-meters scale.** Demonstrated at small scale; production magnets are much larger. Severity: major. Probability: medium-high. Awareness signal: manufacturing partner with track record.

### Supply chain:
4. **Tritium supply.** Global supply ~30 kg, plant needs 50-100 kg/year. Severity: catastrophic. Probability: certain (this isn't risk, it's known constraint). Awareness signal: plan for tritium breeding integration; specific supplier agreements.

### Regulatory:
5. **First-of-kind nuclear regulatory pathway.** No fusion plant has been licensed. Severity: major (delays). Probability: high. Awareness signal: NRC engagement, regulatory affairs hires.

### Economic:
6. **Capital cost per GW.** Current estimate >$5B; commercial parity requires <$3B. Severity: catastrophic for deployment. Probability: high (no demonstration of cost path). Awareness signal: detailed cost models with cost reduction roadmap.

7. **Tritium breeding economics.** Net tritium production must exceed consumption with margin; uncertain. Severity: major. Probability: medium. Awareness signal: detailed breeding blanket design.

### Talent dependencies:
8. **Plasma physics expertise concentrated in 3-4 senior researchers.** If any leave, major capability loss. Severity: major. Probability: medium (talent is mobile in this hot field). Awareness signal: deep bench, knowledge documentation.

### Market:
9. **Competitive alternatives (advanced fission, geothermal, solar+storage) by deployment date.** By 2035-2040 alternatives may dominate. Severity: catastrophic for commercial viability. Probability: medium. Awareness signal: defensible economic case vs alternatives at deployment time.

### Top failure modes (catastrophic AND unaddressed evidence):

**#1: Tritium supply.** This is the single most critical failure mode. If they don't have a credible tritium story, the entire commercial vision is fantasy. Probe question: "Walk me through your tritium supply for the first 5 years of operation — global production is ~30kg, you need 50-100kg/year, where does it come from?"

**#2: First wall materials.** Catastrophic if unsolved, no current solution exists. Probe question: "What materials work for a first wall under 14 MeV neutron flux for 5+ years? What's your specific material development partner, what's their timeline?"

**#3: Plasma stability at scale.** Probe question: "What's the longest pulse duration achieved at your target plasma parameters by anyone, and what's your specific plan to extend by 100-1000x?"

These three failure modes are the killer questions. If FusionCorp answers all three credibly, project is serious. If they hand-wave on any, that's the diagnostic.

## Anti-patterns

- **Skipping categories.** Discipline is walking through all 7. Even "obviously irrelevant" ones reveal blind spots.
- **Generic failure modes.** "Supply chain risks" is not a failure mode. "Tritium global supply 30kg vs need 50kg/year" is.
- **Severity inflation.** Marking everything "catastrophic" loses signal. Use the severity scale honestly.
- **Probability without evidence.** "Could happen" isn't probability. Use evidence — has this category of failure been observed in similar projects? What's the base rate?
- **Awareness assumption.** Assuming team has thought about something because it's obvious. Probe questions test whether they actually have.
- **Failure modes without probe questions.** Identifying problems without ability to test them on the meeting is incomplete. Each top failure mode needs probe questions for combat-questions-design.

## Output template

```
─── FAILURE MODES MAPPING ───

Project: <identifier>
Date: <ISO>

CATEGORY 1 — TECHNICAL BOTTLENECKS:
1. <Specific failure mode>
   Severity: <catastrophic | major | manageable>
   Probability: <high | medium | low>
   Awareness signal: <what would show team is addressing>

CATEGORY 2 — SCALING:
[similar structure]

CATEGORY 3 — SUPPLY CHAIN:
[similar]

CATEGORY 4 — REGULATORY:
[similar]

CATEGORY 5 — ECONOMIC:
[similar]

CATEGORY 6 — TALENT:
[similar]

CATEGORY 7 — MARKET:
[similar]

TOP FAILURE MODES (catastrophic + unaddressed evidence):
1. <Failure mode>
   Why critical: <reasoning>
   Probe question: <specific question for meeting>
   Good response: <what would indicate team has plan>
   Evasive response: <what would indicate team doesn't>

[Top 3-5 failure modes]
```

## Integration with Expert protocol

This is **Phase 3** of the Expert protocol. Output feeds:
- `combat-questions-design` (top failure modes become probe questions)
- `bullshit-detection` (failure modes the team avoids discussing are signals)
- `pre-meeting-intel` (Tier 3 — research who in the team handles each failure mode)

Stored in ExpertBriefing record under `failureModes` field. Post-meeting debrief updates: which failure modes were addressed credibly, which were hand-waved.

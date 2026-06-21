---
name: deconstruct
description: Master orchestration skill for ОСП — runs the full 4-phase deconstruction protocol (Reconnaissance → 12-Layer Preparation → Staff Block → Forecast). Coordinates all other ОСП skills in correct sequence with adaptive depth. Outputs both chat summary (compact, 15-25 lines for owner) and archive entry (full, in Postgres). Replaces flat 12-layer protocol with topology-aware analysis spanning leaves to fruits to seeds.
---

# Deconstruct — Master Protocol For Strategic Analysis

This is the orchestration skill — the main protocol that ОСП runs for any non-trivial analytical question. It is not itself a methodology; it sequences the methodologies that other skills provide. Calling `/analyze`, `/wargame`, `/deep`, or `/full` triggers this orchestration with different depth levels.

The protocol has four phases, each producing specific artifacts that feed forward to the next phase. Skipping phases or re-ordering them breaks the topology of analysis.

## Prerequisites

- All Tier 1 skills available: `awareness-frame`, `root-cause-mapping`, `forces-and-potentials`, `strategic-maneuvers`, `two-paths-synthesis`, `decision-tree-forecast`, `calibration`, `source-triangulation`
- Selected depth level (auto-determined or explicit override)
- Subject of analysis defined (precise, not vague)

## Core principle

> Sequential phases, each with a specific output that feeds the next. Reconnaissance positions the subject. Preparation collects the data. Staff Block synthesizes potentials and maneuvers. Forecast projects forward through topology. Skipping phases produces analysis that looks complete but is structurally hollow.

The temptation is to jump from observation to conclusion. The discipline is to walk through the topology — surface to depth in preparation, depth back to surface in forecast.

## The 4-phase protocol

### Phase 1 — Reconnaissance (5-7 lines, always first)

Skill: `awareness-frame`

The subject is positioned in its field of forces and time before any tool touches it. This is mandatory pre-step — without it, the 12 scalpels cut without orientation.

Output: 7-10 line awareness frame containing subject definition, macro phase, event phase, forces inward, forces outward, vulnerabilities, leverage points, substrate status, climate conditions, and internal health triggers.

If awareness frame flags `INTERNAL HEALTH` triggers, `system-health-diagnostics` skill is loaded automatically for the subsequent analysis.

### Phase 2 — Preparation (12 layers + topology mapping)

The 12 scalpels collect data. Then `root-cause-mapping` organizes that data into the five-level topology (leaves, branches, stem, roots, fruits). This is the analytical heart — finding the depth.

#### 12 layers (from V1 protocol, retained):

**Layer 1 — Source.** Who speaks. Position vs interest, deception capacity, motive.

**Layer 2 — Content.** What is said. Manipulation techniques used, loaded language, claimed facts.

**Layer 3 — Timing.** When said. Why now? What in the immediate context demanded this statement now?

**Layer 4 — Audience.** Apparent vs real audience. Who is actually being addressed by this communication?

**Layer 5 — Silence.** What is NOT said. The shape of the cavity often reveals more than the visible material.

**Layer 6 — Context.** Macro environment, parallel events, what frames this event makes inevitable or impossible.

**Layer 7 — Coordination.** Synchronization between actors. Independent or coordinated? Tight or loose?

**Layer 8 — Narrative vs Flows. 🔑** The alpha layer. What does the public narrative say vs what financial/material/personnel flows show? Divergence is the strongest signal in analysis.

**Layer 9 — Forces & Goals.** Replaces old "Beneficiaries 2x2." Brief inventory of major forces and their stated goals — but the deep treatment is in Phase 3 staff block (`forces-and-potentials`).

**Layer 10 — Historical analogies.** What past events does this resemble? Where do they differ?

**Layer 11 — ACH (Analysis of Competing Hypotheses).** Heuer's method. Generate 4-7 hypotheses including null/Hanlon's razor. Build matrix. Find surviving hypothesis (fewest inconsistencies).

**Layer 12 — Forecast.** Brief — full forecast happens in Phase 4 (`decision-tree-forecast`). Layer 12 in Phase 2 just establishes preliminary direction.

#### Topology mapping (`root-cause-mapping` skill)

After 12 layers produce data, map vertically:
- Leaves (events from Layer 1, 2, 3)
- Branches (clustered programs)
- Stems (strategic courses)
- Roots (deep processes)
- Fruits (durable outputs the system produces)
- Environmental context (soil/substrate, climate, external destructive forces, internal diseases & parasites)

If environmental analysis reveals significant disease/parasite signals, `system-health-diagnostics` skill is loaded for full diagnosis. This produces health score and time-to-collapse estimates that feed back into Phase 3.

This output becomes the input for Phase 3 and Phase 4.

### Phase 3 — Staff Block (the synthesis)

Skills used in sequence: `forces-and-potentials` → `strategic-maneuvers` → `two-paths-synthesis`

If `system-health-diagnostics` was loaded in Phase 2, its findings inform the E (environmental modifier) component in Phase 3a force scoring.

**Phase 3a — Forces & Potentials.** Measure. For each major force, compute P = A × R × S × L × E (ambition × resources × strategic-task-clarity × leverage × environmental modifier). Compute differential D between opposing forces. Account for third-force modulations. Project across time horizons.

**Phase 3b — Goals & Drivers.** Make explicit what each major force actually wants (not just what they say). This deepens Layer 9 with attention to actual goals vs claimed goals.

**Phase 3c — Strategic Maneuvers.** For the subject of analysis (especially if subject has lower potential than opposing force), generate 2-4 candidate maneuvers using the Sun Tzu catalog: Shape, Deception, Empty/Full, Shi, Third-Force Leverage, No-Fight Victory.

**Phase 3d — Two Paths Synthesis.** Run two independent reasoning paths:
- Path A: formal logic from potentials
- Path B: game-theoretic, modeling opponent intelligence and surprise

Convergence between paths = high confidence. Divergence = either weak modeling or genuine frontier (most actionable insight).

### Phase 4 — Forecast (project topology forward)

Skill: `decision-tree-forecast`

Project from current roots forward through evolving stems and branches to **future fruits and seeds**. Probability weights at each level reflect convergence/divergence from Phase 3d. Generate falsifiable indicators with specific dates.

Output is the projection tree with named beneficiaries, time horizons, and verifiable indicators.

## Adaptive depth implementation

Depth level determines which phases run with full rigor and which run in compact form.

**Level 0 (Quick take, ~1 minute):**
- Phase 1: short awareness frame (3-5 lines)
- Phase 2: skip 12 layers, just identify dominant root and stem from prior knowledge
- Phase 3: skip — compact reasoning instead
- Phase 4: brief fruit projection, 1-2 indicators
- Use case: routine, well-understood class of questions

**Level 1 (Standard, ~5-10 minutes):**
- Phase 1: full awareness frame (5-7 lines)
- Phase 2: 12 layers in compact form, full root-cause-mapping
- Phase 3: forces-and-potentials computed, maneuvers identified, two-paths brief
- Phase 4: full fruit projection with indicators
- Use case: default for most non-trivial questions

**Level 2 (Wargame, ~30-60 minutes):**
- All phases full
- Phase 3 receives extended attention — deep maneuver analysis
- Phase 4 includes detailed divergence branching
- Use case: complex topics, second consecutive miss on simpler depth

**Level 3 (Strategic deep dive, hours):**
- Level 2 plus integration with `iw-watchlist`, `macro-regime-id`, `cross-asset-divergence`
- Asymmetric structure search across all identified fruits and seeds
- Cross-checking with Лаборатория знаний for relevant emerging tech
- Use case: major capital decisions, persistent stake monitoring

**Level 4 (Full intelligence operation, days):**
- Level 3 plus deep profiling of decision-makers (`deep-profiling`)
- Open-source video analysis for psychological state
- Network analysis of personal connections
- Leaked data integration where available
- Dark web research for specific intelligence gaps
- Use case: highest-stakes decisions, requested via `/full`

## The procedure (step by step)

### Step 1 — Receive query and determine depth

When user invokes `/analyze`, `/wargame`, `/deep`, or `/full`:
- `/analyze`: auto-determine depth (default 1, escalated by triggers from awareness-frame outputs)
- `/quickanalyze`: force level 0
- `/wargame`: force level 2
- `/deep`: force level 3
- `/full`: force level 4 (requires owner approval)

Depth can be auto-escalated by:
- Topic in active I&W watchlist
- Topic in owner's top-3 personal interests
- Topic with 2+ historical misses
- Topic flagged "high stakes"

### Step 2 — Phase 1 (Reconnaissance)

Run `awareness-frame` skill. Produce 5-7 line frame.

If subject cannot be precisely defined → request clarification from owner before proceeding.

### Step 3 — Phase 2 (Preparation)

Run 12 layers in sequence (compact at low depth, full at high depth). Then run `root-cause-mapping` to organize into 5-level topology.

Quality gates:
- Each layer must have at least one finding (not empty)
- Layer 8 (narrative-vs-flows) must explicitly compare narrative claims to actual flows
- Layer 11 (ACH) must include null/Hanlon hypothesis
- Topology must include explicit fruits (not just leaves)
- Each fruit must have specific named beneficiary

### Step 4 — Phase 3 (Staff Block)

Run `forces-and-potentials` → `strategic-maneuvers` → `two-paths-synthesis` in sequence.

Quality gates:
- Forces table must include 3-8 forces with all four components scored
- Differential D must be computed
- At least 2 candidate maneuvers must be generated for subject (even if subject has higher potential)
- Both Path A and Path B must produce projections
- Convergence/divergence must be diagnosed for each major prediction

### Step 5 — Phase 4 (Forecast)

Run `decision-tree-forecast`. Produces full projection tree with fruits, seeds, indicators.

Quality gates:
- Roots must be projected at multiple horizons
- At least 3 fruits must be projected with named beneficiaries
- At least 5 falsifiable indicators must have specific dates
- Divergence branches from Phase 3d must appear in tree as explicit branching

### Step 6 — Generate dual output

**Chat summary (15-25 lines for owner in Telegram):**
```
🔬 #<id> · <topic — one line>

📊 Forecast (<horizon>):
A. <scenario> — <X%>
B. <scenario> — <Y%>
C. <scenario> — <Z%>

💰 Fruits ripening / beneficiaries:
near: <named actors with brief>
far: <named actors with brief>

💸 Costs / who pays:
near: <named victims>
far: <named victims>

📅 Indicators (with dates):
• <date> — <observable>
• <date> — <observable>
• <date> — <observable>

✅ Confidence: <low/medium/high> · Verify: <YYYY-MM-DD>
🗂 Full archive entry. /archive <id> — details
```

**Archive entry (full, in Postgres):**
- Full awareness frame
- All 12 layers with [ФАКТ]/[ИНТЕРПРЕТАЦИЯ]/[СПЕКУЛЯЦИЯ] labels
- Full root-cause-mapping with 5 levels
- Full forces-and-potentials with table
- Full strategic-maneuvers with all candidates
- Full two-paths-synthesis with convergence/divergence
- Full decision-tree-forecast with all fruits, seeds, indicators
- Confidence stamps per phase
- Verification date

### Step 7 — Persist to archive

Save to Analysis Prisma model with all fields. Create indicator records for verification cycle. Link to relevant KnowledgeArtifacts from Лаборатория знаний if applicable.

### Step 8 — Activate verification cycle

For each indicator with date:
- Schedule automatic check via scheduler at indicator date
- Pre-verify monitoring at -7 days from indicator date
- Verify result and update analysis status

## Quality gates summary

A deconstruction is rejected (returned for rework) if:

1. Awareness frame missing or vague subject definition
2. Any layer of 12 is empty
3. Layer 8 (narrative-vs-flows) lacks explicit comparison
4. Layer 11 (ACH) lacks null/Hanlon hypothesis
5. Topology lacks fruits or fruits lack named beneficiaries
6. Forces & potentials table missing component scores
7. Less than 2 candidate maneuvers
8. Two paths not run separately (or game-theoretic skipped)
9. Forecast missing falsifiable indicators with dates
10. Chat summary exceeds 25 lines (compactness violation)
11. Archive entry missing any phase output

## Anti-patterns

- **Skipping Phase 1.** Tempting because it seems short. But without awareness frame, all subsequent phases drift. Always do it.
- **Mechanical 12-layer slogging.** Treating layers as bureaucratic boxes to fill. Each layer should produce findings, not check marks.
- **Skipping Phase 3d (two paths).** Running only formal logic and skipping game-theoretic. This produces analysis that looks rigorous but misses opponent strategy.
- **Forecast without topology.** Producing scenarios at horizon dates without grounding them in projected fruits from current roots. This is the most common pre-V2 failure mode.
- **Chat summary too long.** Owner wants compact action-relevant output. If summary >25 lines, it has lost compactness — owner won't read.
- **Archive entry too short.** Archive must be full — this is the data that future verification and learning depend on. Shortened archive = lost learning.
- **No indicators.** Forecast without falsifiable indicators is forecast that can't be verified, which means it can't be learned from. Mandatory indicators with dates.
- **Confidence inflation.** Confidence high requires both convergence on Path A/B and abundant data quality. Default should be medium.

## Output template (orchestration metadata)

```
─── DECONSTRUCTION ORCHESTRATION ───

Query: <topic>
Depth level: <0-4>
Auto-escalation triggers active: <list if any>

Phase 1 (Reconnaissance): <completed timestamp>
Phase 2 (Preparation): <completed timestamp>
Phase 3 (Staff Block): <completed timestamp>
Phase 4 (Forecast): <completed timestamp>

Quality gates passed: <list>
Quality gates failed: <list, if any — analysis rejected>

Output produced:
- Chat summary: <lines>
- Archive entry: <character count, sections present>

Indicators created: <count, dates>
Verification scheduled: <next check date>

Linked KnowledgeArtifacts (from Лаборатория): <count, IDs>
```

## Integration with the rest of ОСП

This skill is the **orchestrator**. It calls all other Tier 1 skills in sequence, applies adaptive depth, manages quality gates, produces dual output, persists to archive, activates verification.

Tier 2 and Tier 3 skills are activated by depth level (or explicit triggers from Phase 1):
- Level 3+ activates `iw-watchlist`, `macro-regime-id`, `cross-asset-divergence`, `asymmetric-bet-structure`
- Level 4 activates `deep-profiling`, `cycle-position`, dark-web research
- Specialty topics activate Tier 2: `narrative-vs-flows` (financial topics), `cantillon` (monetary topics), `cycle-position` (macro topics)

This is the skill that makes ОСП feel coherent rather than a grab-bag of techniques. Without orchestration, individual skills produce fragments. With orchestration, fragments combine into actionable analysis.

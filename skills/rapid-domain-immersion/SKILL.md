---
name: rapid-domain-immersion
description: Methodology for compressing years of domain expertise into hours of preparation. Structured deep-dive into unfamiliar technical area to bring owner from "knows nothing" to "knows enough to ask hard questions" before a meeting. Uses 4-layer scaffold (fundamentals → state-of-the-art → schools and debates → numerical anchors) that prevents the common failure mode of broad-but-shallow OR deep-but-narrow knowledge.
---

# Rapid Domain Immersion — Compressing Years Into Hours

The owner has hours; the expert has years. Direct comparison is impossible. But targeted compression of expertise into specific question-asking ability is achievable. This skill is the compression engine.

The mistake is to read broadly hoping to absorb the area. Broad reading produces shallow understanding that experts immediately detect. The discipline is to compress through specific structure: physical principles first (you understand what's actually happening), state-of-the-art second (you know where the frontier is), schools and debates third (you know where consensus exists vs where smart people disagree), numerical anchors fourth (you have specific reference numbers to test claims against).

## Prerequisites

- Specific technical domain identified
- Time budget known (30 min for quick, 1-2 hours for standard, several hours for deep)
- Awareness that this is **focused study**, not general reading

## Core principle

> Expertise has two layers — knowledge of the field and knowledge of the discourse. Most people without expertise have neither; rapid immersion deliberately builds the second layer (discourse) faster than the first (knowledge), because in conversation the second is what's tested. Knowing what experts argue about reveals more than knowing what they agree on.

## The 4-layer scaffold

### Layer 1 — Physical/conceptual fundamentals (15-25% of time)

Goal: understand at the molecular/algorithmic/structural level what's actually happening in this technology.

For a technology to be discussed, the expert immerses in:
- What does this technology actually do at first principles?
- What physical/mathematical/biological mechanism underlies it?
- What are the conservation laws or fundamental constraints that bound it?
- What's the simplest correct mental model?

This is **not** the marketing description. This is the textbook chapter level. For fusion: not "clean energy of the future" but "confining a deuterium-tritium plasma at sufficient temperature and density that fusion reactions exceed losses." For mRNA: not "vaccine platform" but "lipid nanoparticle delivery of synthetic mRNA encoding antigen, translated by cellular ribosomes, displayed by MHC for immune recognition."

The mental model is the foundation for everything subsequent. Without it, all questions are surface.

### Layer 2 — State-of-the-art (25-35% of time)

Goal: know where the frontier currently is.

Questions to answer:
- What are the current best-demonstrated results in this field?
- Who demonstrated them and when?
- What hasn't been demonstrated yet that would matter?
- What are the key benchmarks and where do top performers stand?
- What's recent (last 2-3 years) vs what's older?

This calibrates expectations. When the speaker claims X, the expert immediately knows whether X matches state-of-the-art (impressive if independently verified), exceeds it (extraordinary claim requires extraordinary evidence), or is below it (why are they showing this?).

### Layer 3 — Schools, debates, controversies (25-35% of time)

Goal: know where smart people in the field disagree.

This is the **most underweighted layer in normal reading** but **the most valuable for conversation**.

Questions to answer:
- What are the major theoretical or methodological camps in this field?
- What do they argue about?
- What are the unresolved technical questions?
- Which results are disputed?
- Who are the prominent skeptics and what are their arguments?

When the expert can ask "your approach is closer to school A than school B — why did you choose A given criticism C from B?", the speaker immediately recognizes: this person knows the field. Conversely, when speaker dismisses school B's criticism with hand-waving, the expert recognizes: this person doesn't engage with the real debate.

Schools and debates are also where **real risk lives**. If two competing approaches exist with similar ambition, betting on one is contested. Knowing the contestation is part of due diligence.

### Layer 4 — Numerical anchors (15-25% of time)

Goal: have specific reference numbers to test claims.

Questions to answer:
- What are typical costs (per unit, per experiment, per deployment)?
- What are typical timelines (research → prototype → product)?
- What are typical efficiencies, accuracies, throughputs?
- What's the order-of-magnitude for capital required?
- What's the typical team size for serious work?

When speaker claims "we can do this for $X," the expert immediately mentally checks: is X in the right order of magnitude for this kind of system? If solar panel cost claims are $0.10/watt and the well-known floor is $0.20/watt with current technology, expert asks how. If claim is $1.00/watt, expert wonders why so much.

Numerical anchors prevent embarrassment in both directions: not believing reasonable claims (because they sound expensive) and not detecting impossible claims (because owner doesn't know what's expensive).

## The procedure

### Step 1 — Define domain precisely

Not "fusion" but "magnetic confinement fusion via tokamak." Not "AI" but "transformer-based language models for code generation." Precision focuses the immersion; vagueness diffuses effort.

### Step 2 — Time budget allocation

Distribute available time across 4 layers per percentages above. For 1 hour budget:
- Layer 1: ~12 minutes (fundamentals)
- Layer 2: ~18 minutes (SOA)
- Layer 3: ~18 minutes (schools/debates)
- Layer 4: ~12 minutes (numerical anchors)

### Step 3 — Layer 1: fundamentals

Source priority: textbook overview chapters, well-rated explanatory articles, Wikipedia for orientation followed by deeper sources. Goal: produce 2-3 paragraph "fundamental description" that's accurate and precise.

### Step 4 — Layer 2: SOA

Source priority: recent review papers (Nature Reviews, Annual Reviews series), conference proceedings (NeurIPS, ICML for AI; APS for physics; ASCO for oncology; etc.), top-lab websites with their latest results. Goal: list of current frontier achievements with names and dates.

### Step 5 — Layer 3: schools and debates

Source priority: review papers explicitly comparing approaches, debate-style articles in scientific press (Nature News & Views, Quanta Magazine, etc.), critical commentaries in journals, podcasts/interviews with leading researchers (often more candid about disagreements). Goal: 2-4 named schools/camps with their key claims and disagreements.

### Step 6 — Layer 4: numerical anchors

Source priority: industry reports, government R&D documents, IPO prospectuses (have detailed economic data), patent applications (often have specific technical parameters), specialized industry analysts. Goal: table of typical numbers with sources.

### Step 7 — Synthesis check

Before finalizing immersion, test: can the expert produce a 5-minute coherent explanation of the field for a smart non-expert that includes mechanism, current frontier, key disagreement, and specific numbers? If yes, immersion is sufficient. If hesitant, more time on weak layer.

### Step 8 — Output structured summary

Produce an immersion document with all 4 layers explicitly populated. This becomes the foundation for `project-deconstruction` and `combat-questions-design` skills downstream.

## Worked example — Fusion energy (for hypothetical Academy of Sciences meeting)

Time budget: 90 minutes for standard depth.

### Layer 1 — Fundamentals (~20 min):

Fusion = combining two light nuclei (typically deuterium + tritium) into one heavier nucleus, releasing energy because the combined nucleus has lower mass than the sum. Key challenge: nuclei must overcome electrostatic repulsion, requires extreme temperatures (>100 million K). At those temperatures matter is plasma — must be confined.

Two main confinement approaches:
- Magnetic confinement (tokamak, stellarator) — magnetic fields hold plasma in donut shape
- Inertial confinement (lasers compress fuel pellet) — used by NIF

Energy gain measured by Q = energy out / energy in. Q > 1 = scientific breakeven; Q > 10 = engineering breakeven; commercial requires Q > 30+ with continuous operation.

### Layer 2 — SOA (~28 min):

NIF (laser fusion) achieved Q > 1 in December 2022, repeated 2023-2024 with Q ~1.5-2.5 in single shots. Not continuous. Required conditions remain difficult.

Tokamaks: ITER under construction, scheduled 2025-2030 first plasma, target Q=10. Currently best Q from JET (UK) at ~0.67.

Private fusion: Commonwealth Fusion (SPARC, target 2025), TAE Technologies, Helion (different approach, no DT), Tokamak Energy. Several claim commercial fusion 2030-2035 but timeline disputed.

China EAST tokamak achieving longer pulse durations (recent 1000+ second confinement at lower temperatures).

Materials problem largely unsolved: first wall must withstand 14 MeV neutron flux for years; current candidate materials degrade.

### Layer 3 — Schools and debates (~28 min):

**Mainstream tokamak school (ITER):**
- Established physics, large public investment
- Critics: too slow, too large, too expensive
- Defenders: only path with proven scaling laws

**Compact tokamak school (Commonwealth Fusion, Tokamak Energy):**
- High-temperature superconductors enable smaller machines
- Critics: untested at scale, magnet challenges
- Defenders: physics same as ITER but engineering simpler/faster

**Inertial fusion school (NIF, others):**
- Pulsed approach
- Critics: unclear path to continuous power
- Defenders: NIF results validate physics

**Alternative approaches (Helion p-B11, Zap Energy z-pinch, etc.):**
- Different fuel cycles
- Critics: physics underdemonstrated
- Defenders: avoid neutron damage problem

**Critical voices:**
- Daniel Jassby (former PPPL): "Fusion never coming, here's why" — engineering problems
- Charles Seife: tritium supply unsolvable
- Various: economic case unclear even if technically achieved

### Layer 4 — Numerical anchors (~14 min):

- ITER cost: $25B+ international, behind schedule
- Private fusion company funding: typical Series A $50-200M, Series B+ $200M-$2B (Commonwealth raised $2B)
- Tritium global supply: ~30 kg, fusion plant needs ~50-100 kg/year, enormous gap
- Tokamak operating temperature: ~150 million K
- Capital cost target for commercial: ~$3-5B per GW (vs ~$5-7B for nuclear, ~$1-2B for solar+storage)
- Time from ITER first plasma (2025-2030) to commercial: estimated 20-30 years by mainstream, 10-15 by private
- Best Q achieved continuous: <1 (no continuous breakeven yet)
- Claimed Q for SPARC (Commonwealth): 11
- Claimed Q for Helion (different physics): "net electricity" by 2028 (controversial)

## Anti-patterns

- **Skipping Layer 3 (schools).** Most common error in rapid immersion. Without knowing debates, can't ask the questions that matter.
- **Trusting marketing-level Layer 1.** "Clean energy of the future" is not a fundamental description. Must reach actual mechanism.
- **Outdated Layer 2.** SOA from 5 years ago is not SOA. Especially in fast-moving fields, recent matters.
- **No numerical anchors.** Without specific numbers, can't test specific claims. Vague impressions don't catch fraud or exaggeration.
- **Reading without synthesis.** Reading 20 articles doesn't equal immersion. The synthesis check (5-minute coherent explanation) is the gate.
- **Time imbalance.** Spending 70% of time on Layer 1 (most familiar territory) and 30% on others. Discipline of percentages prevents this.
- **Believing one source.** Even good sources have biases. Cross-check across at least 2-3 sources per layer.

## Output template

```
─── RAPID DOMAIN IMMERSION ───

Domain: <precise definition>
Time invested: <minutes>
Confidence in immersion depth: <low | medium | high>

LAYER 1 — FUNDAMENTALS:
Mechanism: <2-3 paragraphs at first-principles level>
Key constraints: <fundamental limits>
Simplest correct mental model: <description>

LAYER 2 — STATE-OF-THE-ART:
Frontier achievements: <list with names and dates>
What hasn't been demonstrated yet: <list of unresolved>
Key benchmarks: <specific>

LAYER 3 — SCHOOLS AND DEBATES:
School A: <name, claims, key proponents>
School B: <name, claims, key proponents>
[2-4 schools]
Major debates: <list of unresolved technical questions>
Key skeptics: <named critics with their arguments>

LAYER 4 — NUMERICAL ANCHORS:
Cost typical: <ranges with sources>
Timeline typical: <stages with durations>
Performance typical: <metrics with values>
Capital required typical: <orders of magnitude>
Other relevant numbers: <list>

SYNTHESIS CHECK:
[Brief 5-min explanation in expert's own words covering mechanism, frontier, debate, numbers]

CONFIDENCE NOTES:
- Areas of high confidence in immersion: <list>
- Areas of weaker confidence: <list>
- Sources used: <list>
```

## Integration with Expert protocol

This is **Phase 1** of the Expert protocol — the foundation. Every other Expert skill assumes this immersion is complete. Without it:
- `project-deconstruction` works at surface level
- `failure-modes-mapping` misses domain-specific failure types
- `combat-questions-design` produces generic questions
- `bullshit-detection` can't detect domain-specific manipulation

The 4-layer output is stored in the ExpertBriefing record under `domainImmersion` field. Reused across multiple briefings on related topics.

---
name: iterative-research-loop
description: Three-iteration research methodology for Эксперт. Replaces single-shot research with broad-search → gap-identification → targeted-followup → cross-validation pattern. Inspired by Local Deep Research approach. Used inside Phase 1 (rapid-domain-immersion) and Phase 5 (peer-review-simulation, adjacent-fields). Each iteration accumulates sources to ResearchSource KB with proper tier classification and citations. The discipline is "you don't know the right query until you've made the first query."
---

# Iterative Research Loop — Multi-step Research with Compounding Knowledge

A single search query rarely captures a complex domain. The first query gives the lay of the land — but reveals what you don't know. The second query targets those gaps. The third validates by looking for contrarian/critical perspectives. This is the methodology.

This skill is invoked from inside other skills (`rapid-domain-immersion`, `peer-review-simulation`, etc.). It is the **research engine** — not a standalone protocol but a discipline applied throughout Эксперт's work. It uses `lib/research.js` for the operational mechanics.

## Prerequisites

- Topic defined precisely (not vague — "fusion" too broad; "compact tokamak with HTS magnets" specific)
- Time budget known (quick: 1 iteration; standard: 2-3; deep: 3+)
- Linkage target known (which ExpertBriefing or Analysis the sources will attach to)

## Core principle

> Research is iterative not because of inefficiency but because of structure. The first iteration produces unknowns; only after seeing them can you formulate the queries that resolve them. Adversarial validation (iteration 3) catches what confirmation bias hides in iterations 1-2. Compound knowledge: every source found feeds the knowledge base, so future research starts from a stronger position.

## The three iterations

### Iteration 1 — Broad search

**Goal:** map the territory. What's the field about? Who works in it? What recent results are notable?

**Queries are general:**
- "<topic> overview state-of-the-art 2026"
- "<topic> fundamentals current research recent breakthroughs"

**What to extract:**
- Key facts with sources
- State-of-the-art findings (recent 1-2 years)
- Major debates / disagreements
- Authoritative sources (peer-reviewed, government, industry analyst)
- Numerical anchors (typical costs, timelines, performance)

**Sources go into ResearchSource via `addResearchSource()`** — each gets auto-tier classification (`tierSource()` looks at URL).

**Output:** initial findings + identified themes. Not yet a complete understanding.

### Iteration 2 — Gap identification + targeted follow-up

**Goal:** resolve the questions that iteration 1 raised but didn't answer.

**Step 2a — Gap identification.** Use Claude to analyze iteration 1 output. For each gap:
- What's missing or unclear?
- What specific query would resolve it?
- What source type would best answer (peer_review/government/industry/news)?

**Step 2b — Targeted queries.** For each top 3 gap:
- Specific query
- Specific preferred source type
- Resolved findings + new sources

**Sources added with topic + gap tags** so KB can later be filtered.

**Output:** richer understanding with major gaps closed.

### Iteration 3 — Cross-validation (deep mode only)

**Goal:** find contrarian/critical perspectives. If iterations 1-2 told you the project is great, who says it's not?

**Queries:**
- "<topic> contrarian views skeptics critics failed predictions"
- "<topic> methodological criticisms"

**What to extract:**
- Notable skeptics and their arguments
- Failed predictions in this area
- Conflicting evidence
- Methodological criticisms

**Why this matters:** confirmation bias is invisible by default. Iteration 3 explicitly searches for the opposing view. Sources tagged "contrarian" so they're identifiable later.

**Output:** balanced understanding with adversarial perspective integrated.

## How it integrates with parent skills

### Inside `rapid-domain-immersion`

The 4-layer scaffold (fundamentals → SOA → schools/debates → numerical anchors) is **populated through iterative research**:
- Iteration 1 → covers fundamentals + initial SOA
- Iteration 2 → fills gaps in SOA + populates schools/debates
- Iteration 3 (deep) → strengthens schools/debates with contrarian voices, validates numerical anchors

### Inside `peer-review-simulation`

For each empirical claim being reviewed:
- Iteration 1 finds primary source for the claim
- Iteration 2 finds adjacent/competing studies
- Iteration 3 finds peer-review style criticisms

### Inside `adjacent-field-cross-check`

Each adjacent field gets its own mini iterative loop:
- Iteration 1: what does this field say about the topic?
- Iteration 2: what specific concerns from this field's perspective?
- (Iteration 3 typically not needed for adjacent fields — already inherently contrarian)

### Inside `competing-claims-comparison`

For each competitor:
- Iteration 1: their public claims
- Iteration 2: their actual results vs claims
- Iteration 3: critical reviews of their work

## Strategy selection by depth level

**Level 1 — Quick (`/quickexpert`):**
- Iterations: 1 only (broad)
- Knowledge base: query existing KB first, augment with one Grok call
- ~30 seconds-2 minutes per topic

**Level 2 — Standard (`/expert`, default):**
- Iterations: 2 (broad + targeted)
- Knowledge base: full integration
- ~5-15 minutes per topic

**Level 3 — Deep (`/deepexpert`):**
- Iterations: 3 (broad + targeted + cross-validation)
- Knowledge base: full integration + force lookup of related prior briefings
- ~20-60 minutes per topic

## Knowledge base usage

**Before iteration 1:** call `searchKnowledgeBase(topic)`. If high-tier sources already exist (tier 1-2 from prior research), prefer them — saves iterations and signals KB is paying off.

**During each iteration:** new sources auto-added via `addResearchSource()`. Tier auto-detected from URL.

**End of research:** sources auto-linked to current ExpertBriefing/Analysis via `linkSourceToBriefing` / `linkSourceToAnalysis`. Counters incremented (`citedInBriefings`).

This is how the KB compounds — every briefing makes the next one easier.

## Citation discipline (mandatory)

**Every numerical anchor** in output must have `[Source: <Title>, <type>, <date>]` citation.
**Every empirical state-of-the-art claim** in Layer 2 must have citation.
**Every "according to <X>" or "based on <Y>"** must reference a real source.

`validateCitations(text)` is run on output before delivery. If missing citations found:
- Flag the snippets
- Either fix (add citations from collected sources) or remove the unsupported claims

This is non-negotiable. Briefings with uncited claims are weak briefings.

## The procedure

### Step 1 — Detect strategy
Based on depth level (1/2/3), choose iterations count.

### Step 2 — Pre-search KB
```javascript
const existing = await searchKnowledgeBase(topic, {
  userId: OWNER_ID,
  minTier: 3,  // accept tier 1-3 only for pre-population
  limit: 10
});
```
If existing has high-tier sources (tier 1-2), use them as foundation.

### Step 3 — Run iterativeResearch
```javascript
const research = await iterativeResearch(topic, {
  maxIterations: depthLevel >= 3 ? 3 : depthLevel >= 2 ? 2 : 1,
  strategy: depthLevel === 1 ? 'quick' : depthLevel === 2 ? 'standard' : 'deep',
  linkTo: { type: 'briefing', id: briefingId },
  additionalContext: meetingContext  // helps Claude understand framing
});
```

### Step 4 — Validate citations on output
```javascript
const check = validateCitations(research.findings);
if (!check.valid) {
  // Either fix or remove uncited claims
  // Or fail-soft and flag for review
}
```

### Step 5 — Use findings + sources in parent skill output
Findings become the raw material for the parent skill's structured output (e.g., 4-layer scaffold for `rapid-domain-immersion`). Sources become the citations woven into that output.

## Worked example — for FusionCorp briefing

**Topic:** "compact tokamak fusion FusionCorp HTS magnets commercial 2030"

**Strategy:** depth 3 (deepexpert, $50M investment context).

### Iteration 1 — broad scan:

Query: "compact tokamak fusion FusionCorp HTS magnets state-of-the-art 2026"

Found:
- Commonwealth Fusion 2024 SPARC paper (peer_review, tier 1)
- IAEA tokamak comparison report (government, tier 2)
- "Fusion industry update Q1 2026" (industry, tier 3)
- Multiple recent Reuters articles (news, tier 4)

Findings synthesized: Compact tokamaks aim Q=10 at smaller scale via HTS magnets. Commonwealth leading. Several others (Tokamak Energy, FusionCorp claimed). $5-7B/GW typical FOAK, claims of $3B aggressive. Tritium remains universal challenge.

### Iteration 2 — gap identification + follow-up:

Claude identified 3 gaps:
1. **Gap:** what specifically is FusionCorp's claimed differentiation?
   - Query: "FusionCorp technology specifics magnet design"
   - Result: company website only — no peer-reviewed paper
   - Tagged as gap signal

2. **Gap:** tritium supply concrete plans across compact fusion startups?
   - Query: "tritium supply commercial fusion 2030 production capacity"
   - Result: ITER tritium breeding blanket reports + IAEA forecasts found
   - Numerical anchor: 30 kg global supply, 50-100 kg/year needed for first commercial

3. **Gap:** comparison between FusionCorp and CFS specifically?
   - Query: "FusionCorp Commonwealth Fusion comparison technical"
   - Result: industry analyst report (tier 3) — concludes CFS substantially ahead on funding + magnet R&D

### Iteration 3 — cross-validation:

Query: "compact tokamak fusion skeptics critics 2030 timeline failed predictions"

Found:
- Daniel Jassby critique paper (peer_review, tier 1)
- Charles Seife article (news/tier 4 but high-quality)
- Various physics blog posts (tier 5, low weight)

Findings: skeptical position substantial — fusion timelines historically optimistic by 2-3x. Tritium issue called "unsolved" by Jassby. First wall material problem unresolved.

### Final output:

22 unique sources across tiers 1-5 (3 tier 1, 4 tier 2, 5 tier 3, 8 tier 4, 2 tier 5).
Findings synthesized into 4-layer scaffold for `rapid-domain-immersion`.
Citations validated — all numerical anchors and SOA claims have inline citations.
Sources auto-linked to ExpertBriefing #N via junction table.
Identified gaps in iteration 2 surface as "key unknowns" in `project-deconstruction`.
Contrarian voices from iteration 3 inform `failure-modes-mapping`.

## Anti-patterns

- **Single-iteration deep research.** "I'll just do one big query and ask for everything." This works poorly — Grok dilutes attention across too many sub-questions. Multiple targeted queries beat one omnibus query.
- **Skipping iteration 3.** Confirmation bias is silent. Without explicit contrarian search, briefing is biased toward project's preferred narrative.
- **Ignoring KB.** Re-doing research that exists in KB wastes time and tokens. Always pre-search.
- **Accepting all sources equally.** Tier matters. Tier 5 blog post about fusion is not equal to tier 1 Nature paper. Don't cite tier 5 to support critical claims.
- **Citation theater.** Adding `[Source: X]` without verifying X actually says what's claimed. Always summarize source content briefly to confirm understanding.
- **Iteration 2 without gap analysis.** Skipping the Claude-driven gap identification step turns iteration 2 into another broad query — defeating the purpose.
- **Adding noise to KB.** Tier 5 sources without value pollute future searches. Be selective on what's added.

## Output template

```
─── ITERATIVE RESEARCH ───

Topic: <precise>
Strategy: <quick | standard | deep>
Iterations completed: <N>

ITERATION 1 — BROAD SCAN:
Query: "<broad query>"
Sources found: <count> (tier 1: X, tier 2: Y, ...)
Key findings: <synthesized>

ITERATION 2 — GAPS + TARGETED:
Gaps identified:
1. <gap>: query "<specific>" → resolved | partially | unresolved
2. <gap>: ...
3. <gap>: ...
Sources added: <count>
Findings: <gap-resolved content>

ITERATION 3 — CROSS-VALIDATION (if deep):
Contrarian query: "<query>"
Critical sources found: <count>
Contrarian findings: <integrated>

TOTAL SOURCES: <count>
TIER DISTRIBUTION: tier1=X tier2=Y tier3=Z tier4=W tier5=V
GAPS UNRESOLVED: <list> (if any)
CITATION VALIDATION: <pass | fail with snippets>
```

## Integration with Эксперт protocol

This skill is **not run as a standalone phase**. It's invoked from inside Phase 1 (`rapid-domain-immersion`) and Phase 5 (Tier 2 skills). It is the engine.

Output stored in ExpertBriefing under `iterativeResearchLog` field as JSON log.
Sources stored in ResearchSource model with junction-table linkage to ExpertBriefing.

The compounding effect: after 50 briefings, KB has 500-1000 high-quality sources. Future briefings start with relevant prior research. Domain coverage builds over time.

---
name: source-triangulation
description: Discipline of verifying claims through multiple independent sources of different types before treating them as fact. Critical claim must be confirmed by minimum two independent sources of different nature (not two articles citing same primary source — different types of evidence). Marks each claim in analysis with source confidence ([ФАКТ], [ИНТЕРПРЕТАЦИЯ], [СПЕКУЛЯЦИЯ]). Foundation for analytical credibility — without it, analyses chain together unverified claims and propagate errors.
---

# Source Triangulation — Verifying Before Believing

The single biggest error in amateur analysis: building chains of reasoning where each link rests on unverified claim. One bad source becomes "as widely reported," then "as established fact," then the foundation for further analysis. By the time the chain is complete, the original error is invisible but determinative.

The discipline of triangulation prevents this. Critical claims must be confirmed by multiple independent sources of different nature before treatment as fact. Each claim carries explicit confidence labeling — [ФАКТ] (verified), [ИНТЕРПРЕТАЦИЯ] (analyst reading), [СПЕКУЛЯЦИЯ] (hypothesis).

## Prerequisites

- Awareness that single-source claims are weak even when source is reputable
- Discipline to investigate before incorporating claims into analysis
- Understanding of source independence (two sources citing same origin are not independent)

## Core principle

> A claim is fact only when confirmed by minimum two sources of different nature. Otherwise it is interpretation or speculation. Mark each claim in analysis with its confidence label. Without this discipline, analyses chain unverified claims and amplify error.

## Source independence

Sources are **independent** when they have different origin chains:

**Independent:**
- Reuters article + Pentagon press release + satellite imagery showing same event
- Bloomberg article + court documents from related case + interview with named participant
- Government statement + corporate filing + market data showing consistent pattern

**NOT independent (same origin chain):**
- Reuters article + AP article both quoting same anonymous official
- Multiple media outlets citing same think tank report
- Multiple analyses based on same dataset

The test: would an error in the original source propagate to all listed sources? If yes, they're not independent — they're the same source duplicated.

## Source types

For triangulation, sources should ideally be of **different types**:

- **Official statement** (government, regulator, corporation)
- **Documentary evidence** (filings, court records, leaked documents)
- **Witness testimony** (interviews, named on-record statements)
- **Direct observation** (satellite imagery, market data, photographs)
- **Pattern data** (aggregated statistics, trade data, capital flows)
- **Adversarial source** (party with opposite interest confirming despite their interest)

Strongest triangulation: claim confirmed across multiple types. Same-type confirmation (three official statements) is weaker than cross-type (one official + one documentary + one observation).

## The procedure

### Step 1 — Identify critical claims

In the analysis, identify which claims are **critical** to conclusions. Not every claim needs full triangulation — that's impossible. Critical claims are those:
- Loadbearing for forecasts
- Disputed or unusual
- Originating from interested parties
- Counterintuitive

Non-critical (no triangulation required): well-established facts, obvious mathematical relationships, directly observable currents.

### Step 2 — For each critical claim, gather sources

Collect:
- Original source claiming this
- At least one additional independent source confirming
- Ideally: at least one source of different type

If sources are insufficient, claim is downgraded to interpretation or speculation.

### Step 3 — Check independence rigorously

For each pair of sources, verify they're truly independent:
- Different origin chains
- Different methodologies
- Different motivations
- Not citing each other

A common failure mode: news aggregators republish same wire story as if independent. Always check origin.

### Step 4 — Apply confidence labels

Mark each claim in analysis output with explicit label:

**[ФАКТ]** — Confirmed fact. Multiple independent sources of different types confirm. Treat as input to further reasoning.

**[ИНТЕРПРЕТАЦИЯ]** — Analyst's reading of evidence. Based on facts but adds interpretation. Reasonable but disputable.

**[СПЕКУЛЯЦИЯ]** — Hypothesis. Plausible but not directly supported by triangulated evidence. Used cautiously, often as scenarios rather than predictions.

These labels are mandatory in formal ОСП archive entries. Chat summaries can collapse but archive must show the structure.

### Step 5 — Document source chain

For [ФАКТ] claims, document briefly which sources confirm:
- "[ФАКТ] Russian fiscal deficit Q1 2026 reached X (Russian Ministry of Finance + IMF reports + satellite imagery of construction project halts)"

This documentation enables verification audit later. If a claim turns out wrong, the source chain shows where the error entered.

### Step 6 — Refuse to use single-source critical claims

If only one source supports a critical claim and triangulation isn't available:
- Either downgrade claim to [ИНТЕРПРЕТАЦИЯ] or [СПЕКУЛЯЦИЯ]
- Or invest effort to find additional sources before using
- Or note explicitly that the claim depends on a single source

Never proceed with critical-load reasoning on single-source claim while labeling it [ФАКТ].

## Source quality assessment

Beyond independence and type diversity, individual source quality matters:

**High-quality sources:**
- Direct, primary, no interpretation layer
- Track record of accuracy
- Methodology disclosed
- No known interest in misrepresenting

**Medium-quality:**
- Reasonable track record
- Some interpretation involved
- Some interest exposure but not dominating

**Low-quality:**
- Single anonymous source
- Heavy interpretation/spin
- Strong interest in particular framing
- No methodology

For [ФАКТ] designation, generally need at least two medium-or-better sources.

## Worked example

**Claim under analysis:** "$580M oil short was placed 15 minutes before Trump's pause announcement."

**Sources for triangulation:**

Source 1: Specific report by Wall Street Journal naming firms involved
- Type: media reporting
- Independence: based on their own reporting from market participants
- Quality: established outlet, specific details

Source 2: SEC investigation announcement
- Type: official government action
- Independence: independent investigation, not derived from media
- Quality: high — implies SEC has evidence

Source 3: Pattern data showing positioning timing
- Type: market data
- Independence: directly observable
- Quality: high — directly observable pattern

**Triangulation:** Three independent sources of three different types. Confirmed.

**Label:** [ФАКТ]

**Use in analysis:** loadbearing input — supports broader claim about insider information flow in current administration.

---

**Counter-example:** "Russia is preparing to withdraw from CSTO by 2027"

**Sources available:**

Source 1: Op-ed in Kyiv Post citing unnamed sources
- Type: media interpretation
- Independence: based on unnamed sources
- Quality: low (anonymous sources, partisan outlet)

Source 2: Estonian intelligence service report (paraphrased)
- Type: official statement
- Independence: limited (interested party)
- Quality: medium

**Triangulation:** Two sources, both with significant interest framing. Cross-type but not strong independence.

**Label:** [СПЕКУЛЯЦИЯ] at most

**Use in analysis:** can mention as a scenario, cannot support as a fact.

## Anti-patterns

- **Citation laundering.** "As widely reported" — usually means single-source claim repeated without verification.
- **Same-source triangulation.** Three media outlets citing same official statement is one source, not three.
- **Reputation bypass.** Trusting prestigious outlet without checking their source. Quality outlets sometimes report claims based on weak sources.
- **Confirmation triangulation.** Seeking confirming sources for what you already believe; ignoring contradicting sources.
- **Speculation labeled as fact.** [ФАКТ] used loosely. Discipline required — only true triangulation merits [ФАКТ].
- **No source documentation.** Claims labeled but sources not noted. Future verification audit impossible.
- **Ignoring adversarial sources.** When a party with strong interest in opposite framing nonetheless confirms a claim, that's exceptionally strong evidence. Often missed.

## Output template

For each critical claim in analysis output:

```
[ФАКТ | ИНТЕРПРЕТАЦИЯ | СПЕКУЛЯЦИЯ] <claim>
Sources (for ФАКТ):
- <Source 1> (type: <category>)
- <Source 2> (type: <category>)
- <Source 3 if applicable>
Confidence rationale: <brief>
```

In chat summary, labels collapsed to brief markers; in archive, full source documentation present.

## Integration with deconstruction protocol

This skill operates **across all 12 layers** — every claim made in any layer carries appropriate confidence label.

Layer 1 (Source) feeds source-quality assessment. Layer 2 (Content) tests specific claim triangulation. Layer 4 (Audience) analyzes who claims are aimed at — affecting their credibility.

Quality gates of deconstruction protocol require:
- Critical claims labeled with [ФАКТ]/[ИНТЕРПРЕТАЦИЯ]/[СПЕКУЛЯЦИЯ]
- [ФАКТ] claims have documented sources
- No claim of [ФАКТ] without triangulation

Output stored in Analysis record at the layer level — each layer's findings carry their confidence labels.

This skill is foundational. Without it, analysis quality degrades regardless of methodology sophistication. With it, analysis carries explicit epistemic structure that supports verification, learning, and accountability.

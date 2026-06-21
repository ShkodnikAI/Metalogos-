---
name: compression-principle
description: The foundational discipline for Visual Department — compression of understanding. A good infographic delivers full meaning in 5 seconds. If the viewer needs to read 30 seconds, it's a document not an infographic. This skill teaches what to include, what to remove, how to hierarchize, how to test for 5-second readability.
---

# Compression Principle — 5-Second Readability Discipline

The goal of an infographic is not "to display data". The goal is **compression of understanding** — converting hours of analysis into seconds of grasp. Every design decision serves this.

## Prerequisites

- `visual-grammar` loaded
- Source content available (analysis, briefing, artifact, etc.)

## Core principle

> An infographic that requires 30 seconds of reading is a document with pictures. An infographic that delivers meaning in 5 seconds is doing its job. The discipline is brutal subtraction — most data the source contains should NOT be in the visual. Only the load-bearing minimum survives.

## The 5-second test

**The fundamental test:** show the visual to someone for 5 seconds, then ask "what is this saying?"

If they can answer with the **central insight** of the source — infographic works.
If they answer with **structural description** ("it's about fusion") — infographic failed.
If they answer with **details but no insight** ("there's a table with 7 rows") — infographic failed.

Apply this test before publishing.

## Why compression is hard

The amateur impulse: "the source has 10 important points, let me show all 10".
Result: 10 equally-sized elements competing for attention, viewer sees none, infographic fails.

The discipline: rank the 10 points, pick the top 1-3, make those VISUALLY DOMINANT, demote the rest to supporting details or omit entirely.

This requires **judging what's actually important** — which is hard because everything seems important when you've done deep analysis.

## The 1-3-7 rule

A good infographic has:
- **1 dominant insight** (the answer to "what is this saying?")
- **3 supporting elements** (the answer to "how do we know?")
- **7 details** (the answer to "tell me more")

Not 1-3-7 elements total. 1-3-7 **hierarchical levels** of attention.

The 1 must be 4-5× more visually weighted than the 3. The 3 must be 2-3× more weighted than the 7.

## Compression techniques

### Technique 1: Hierarchical demotion

Start with source's structure (e.g., OSP V2 has leaves/branches/stem/roots/fruits + scenarios + forces + bifurcations + premortem...).

Ask: **if I could only show one thing, what would it be?** That's the "1".

Ask: **if I could show 3 more, what would they be?** Those are the supporting "3".

Everything else: either compressed into background/footnotes, or omitted.

For OSP Analysis, the typical 1-3-7:
- 1: most likely scenario (with probability)
- 3: top 3 fruits (anticipated outcomes), key actor with highest P, biggest bifurcation
- 7: details on each of the above

### Technique 2: Replacement by visual encoding

Long text → visual encoding:
- List of items by importance → bar chart sorted desc
- Geographic spread → map dots/heatmap
- Timeline → horizontal line with markers
- Comparison → side-by-side panels
- Hierarchical structure → tree/network
- Time series → sparkline
- Status of N items → small multiples grid

A bar chart of 5 items reads in 2 seconds. The same 5 items as bullet list reads in 8-10 seconds.

### Technique 3: Aggregation

Don't show 50 data points if 3 categories tell the story:
- Don't show "30 talent move events" — show "engineer flow from Google → OpenAI: 18 vs reverse: 4"
- Don't show "100 patents" — show "3 patent clusters: A=60, B=30, C=10"
- Don't show "20 sources" — show "tier 1 academic: 8, tier 2 government: 6, tier 3 industry: 4, others: 2"

### Technique 4: Anchoring through familiar reference

A number alone is meaningless. "Tritium supply: 30 kg globally" — does that mean a lot or a little?

Anchor: "Tritium global supply: 30 kg | Annual fusion plant need: 50-100 kg → 2-3× shortage".

The relationship is the insight. The number alone is not.

### Technique 5: Negative space as priority signal

If the 1 dominant insight occupies the upper third with large size and most space around it, the eye reads it first inevitably.

Crowding the dominant element next to others kills the hierarchy.

### Technique 6: Color as semantic compression

Color encodes meaning, not decoration:
- Gold (only on dominant insight + 1-2 key data points)
- Burgundy (only on identified risks/failures)
- Forest (only on validated/positive signals)
- Navy (structure)
- Greys (supporting)

When color is consistent and meaningful, the viewer "reads" colors faster than text.

### Technique 7: Annotation over labeling

Bad: every element labeled (everything competes).
Good: only the **critical** elements annotated, with line/bracket pointing to them. Others unlabeled — labeled in legend if needed.

Active annotation says "look here" without crowding. Passive labeling says "here's everything" and overwhelms.

## What to OMIT

The hardest part. Things that seem important but should be omitted from an infographic:

- **Methodology details.** The source explains methodology. Visual shows results.
- **Caveats and uncertainties.** Show confidence level once (in header). Don't repeat caveats on every element.
- **Provenance per data point.** Cite source in footer, not on every number.
- **Definitions.** If audience needs definitions, infographic isn't for them.
- **History of analysis.** "We considered X then rejected for Y" is documentation, not visualization.
- **Personal interpretation.** Show data. Let owner interpret.
- **Aspirational elements.** "If trends continue..." belongs in supporting text, not as headline.

## Worked example — OSP Analysis "Belarus BYN-USD"

**Source (compressed from full V2 analysis):**
- 5-level topology with 5×4 fields = 20 data points
- 4 scenarios with probabilities
- 3 bifurcation points
- 5 actors with P calculations (so 5×6 = 30 numbers)
- 2 premortems
- 12 watch indicators

**Total raw elements: ~75. An infographic showing all 75 is unreadable.**

Compression decision tree:

**1 (dominant):** Most likely scenario = "BYN devaluation 20-30% in 6 months" — 65% probability.

Visual: large headline at top in gold, with probability gauge.

**3 (supporting):**
- Biggest driver = Russian fiscal cascade (most important actor with high P)
- Biggest risk = forced peg break (the highest-impact alternative scenario)
- Verification date (when we'll know)

Visual: 3 panels of equal size below headline.

**7 (details):**
- Top 5 scenarios with probabilities (sorted bar chart, 1 line each)
- Top 3 watch indicators (3 inline with dates)

Visual: bottom third, 7 micro-elements at smallest size.

**Omitted:** all of stem/roots layers, individual actor breakdowns, methodology, all premortems, all but top 3 indicators, source citations (compressed to "12 sources, tier-mix" in footer).

**Result:** 5-second test passes. Viewer reads: "BYN likely devaluing 20-30% in 6 months. Russian fiscal pressure is driver. Verify in November."

Three sentences. That's the visual. That's compression.

## The compression workflow

When generating a visual:

1. **Identify the 1.** What is the single most important takeaway from this source? Write it as a sentence. If you can't pick one — go back to source and pick.
2. **Identify the 3.** What three elements support the 1? Write them.
3. **Identify the 7.** What seven details enrich the picture? List them.
4. **Mark visual weights.** 1 gets ~40% of visual real estate. 3 get ~40% total (so ~13% each). 7 get ~20% total (~3% each).
5. **Choose encoding.** For each element, what visual encoding is most efficient? Bar chart? Number? Icon?
6. **Apply color semantics.** Only the 1 and 1-2 of the 3 get accent colors. Rest in greys/navy.
7. **Test 5-second readability.** Imagine seeing the visual for 5 seconds. Can you state the 1? If no, redesign.
8. **Apply visual-grammar checklist.**

## Anti-patterns

- **"Show all the data" impulse.** The source HAS all the data — infographic is the compressed view, not the duplicate.
- **Equal weighting.** Everything same size = nothing important = visual fails.
- **More-is-more.** Crammed infographic = unreadable. Less elements better-treated > more elements crammed.
- **Aesthetic compression.** "It looks balanced" is not compression. Hierarchy of meaning is compression.
- **Annotation everywhere.** If every element labeled, none stand out. Annotate the dominant, label the supporting, leave details to legend.
- **Avoiding the hard call.** "I can't decide what's most important" = you haven't done the analysis. Pick. If wrong, you'll find out from recall tests.

## Integration

This skill applies to all 5 visual types but with different intensity:
- **Type 1 (Compression):** maximum application — its whole purpose IS compression
- **Type 2 (Memorable):** very high — memorability requires compression first
- **Type 3 (Analysis Card):** templated, so compression is already baked into template
- **Type 4 (Reference Sheet):** moderate — reference needs more detail than infographic, but still hierarchized
- **Type 5 (Premium):** very high — premium artifacts target maximum impact per element

Compression principle is enforced by quality gate before publishing any visual.

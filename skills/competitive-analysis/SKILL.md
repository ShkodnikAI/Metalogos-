---
name: competitive-analysis
description: Analyzing competitors honestly — what they offer, how they position, their pricing, their real weaknesses and strengths. Avoids both underestimating competitors (dangerous) and copying them blindly (also dangerous). Produces the differentiation basis a campaign needs.
---

# Competitive Analysis — Know the Field Honestly

You are never selling into a vacuum. The customer has alternatives — competitors, substitutes, or "do nothing". Competitive analysis is mapping that field honestly, so positioning is based on a real gap, not an imagined one.

## Prerequisites

- The product/service and its target segment are known
- `market-research-methods` data-quality discipline applies here

## Core principle

> The two failure modes are mirror images: dismissing competitors ("ours is obviously better") blinds you to why customers choose them; worshipping competitors ("we must match every feature") turns you into a worse copy of them. Honest analysis does neither — it finds what competitors genuinely do well, what they genuinely do badly, and where the real, defensible gap is.

## What counts as a competitor

Broader than you think:

- **Direct competitors** — products solving the same problem the same way
- **Indirect competitors** — products solving the same problem differently
- **Substitutes** — non-product solutions (a spreadsheet, a manual process, an assistant)
- **"Do nothing"** — the customer lives with the pain. Often the biggest competitor of all.

A campaign that only considers direct competitors misses that most prospects aren't choosing between you and a rival — they're choosing between you and their current spreadsheet, or you and inertia.

## What to analyze per competitor

For each significant competitor:

- **Offering** — what they actually provide (use it, don't guess)
- **Positioning** — how they frame themselves, who they say they're for
- **Pricing** — model and level
- **Target segment** — who they're really aimed at
- **Genuine strengths** — what they do well (be honest — this is where customers go)
- **Genuine weaknesses** — where they fall short for your target segment
- **Momentum** — growing, stable, declining?

The honesty requirement cuts both ways: list real strengths even when uncomfortable, list real weaknesses without exaggeration.

## The procedure

### Step 1 — Map the field
List competitors across all four categories above. Don't forget substitutes and "do nothing".

### Step 2 — Investigate, don't assume
Where possible, actually use the competitor's product, read their real customer reviews, see their actual pricing. Reviews — especially the critical ones — reveal genuine weaknesses better than any analysis. Tag findings by reliability (`market-research-methods` discipline).

### Step 3 — Honest strengths and weaknesses
For each competitor, force yourself to name real strengths (uncomfortable but necessary) and real weaknesses (without inflating them).

### Step 4 — Find the gap
Look for: a segment underserved by everyone, a pain all competitors handle badly, a positioning angle nobody owns. The gap is where differentiation lives.

### Step 5 — Test the gap is real and defensible
A gap is only useful if (a) the target segment actually cares about it, and (b) you can hold it — a gap a competitor closes in a week isn't differentiation.

### Step 6 — Output the differentiation basis
State clearly: what your product offers that the alternatives don't, for this segment, that matters and holds.

## Finding the gap — where to look

- **Underserved segment** — everyone targets the big mainstream group; a specific segment is ignored
- **Badly-handled pain** — all competitors technically address a need but all do it poorly
- **Unowned positioning** — competitors all say "powerful and complete"; nobody says "simple and focused" (or vice versa)
- **Price/complexity gap** — a cluster of expensive-complex tools and a cluster of cheap-toy tools, nothing in between
- **Experience gap** — competitors win on features but the experience is painful

The gap against "do nothing" is special: here the competitor is inertia. The differentiation isn't "better than product X" — it's "worth the effort of changing at all".

## Honest treatment of competitor strengths

When a competitor genuinely does something well, three honest responses — never denial:

1. **Match it** — if it's table-stakes, you may simply need it too
2. **Concede it** — "they're better for X; we're better for Y" — honest positioning, and it builds trust
3. **Reframe it** — their strength may be a weakness for *your* segment ("they're feature-rich" → "they're overwhelming for someone who just wants the basics")

What you must never do: pretend the strength doesn't exist. Customers see it. Denying it makes you untrustworthy.

## Worked example

Competitive analysis for the Fosved miniapp, target = solo business operators.

**Field mapped:**
- Direct: established all-in-one business dashboards
- Indirect: project management tools used as makeshift trackers
- Substitutes: spreadsheets, note apps, the operator's own memory
- Do nothing: living with scattered information

**Per competitor (abbreviated):**
- *All-in-one dashboards* — Strength: comprehensive, polished. Weakness for this segment: built for teams, overwhelming and overpriced for one person; long setup. Momentum: stable.
- *Project tools as trackers* — Strength: familiar, cheap. Weakness: not designed for this — operators bend the tool and it shows. 
- *Spreadsheets* — Strength: free, infinitely flexible, already in use. Weakness: manual, fragile, no consolidated view, breaks as complexity grows. **This is the real competitor — most prospects use this.**
- *Do nothing* — Strength: zero effort. Weakness: the pain (missed things, guesswork) that triggers the search.

**The gap:** every product-competitor is built for teams; nothing is designed specifically for the *solo* operator who wants consolidation without team-tool complexity. And against the real competitor — the spreadsheet — the gap is "automatic consolidated view vs manual fragile patchwork".

**Differentiation basis:** "For solo operators, against your current spreadsheet patchwork: one view that updates itself, without the weight of a team tool." Honest (concedes team tools are more comprehensive — for teams), specific, and defensible (focus on the solo segment is a deliberate position, not easily copied by tools committed to the team market).

## Anti-patterns

- **Dismissing competitors.** "Ours is obviously better." Then why do customers pick them? Blind spot.
- **Worshipping competitors.** Feature-matching everything. Becomes a worse copy.
- **Forgetting substitutes and "do nothing".** Analyzing only direct rivals; missing that the real competitor is a spreadsheet or inertia.
- **Guessing instead of investigating.** Analyzing competitors without using their product or reading real reviews.
- **Inflating weaknesses.** Exaggerating competitor flaws — comforting, but produces positioning that collapses on contact.
- **Denying strengths.** Pretending a real competitor advantage doesn't exist. Customers know better.
- **Imaginary gaps.** "Gaps" the target segment doesn't actually care about.
- **Undefendable gaps.** A differentiation a competitor can copy in a week.
- **Static analysis.** Treating competitors as fixed; missing momentum and direction.

## Output template

```
COMPETITIVE ANALYSIS — <product / market>
Target segment: <segment>

THE FIELD
Direct: <list>
Indirect: <list>
Substitutes: <list>
Do nothing: <description of living with the pain>

PER COMPETITOR
<name> [direct/indirect/substitute]
  Offering: <...>
  Positioning: <...>
  Pricing: <...>
  Genuine strengths: <...>
  Genuine weaknesses (for our segment): <...>
  Momentum: growing / stable / declining
[repeat]

THE REAL COMPETITOR: <which alternative most prospects actually use>

THE GAP
<underserved segment / badly-handled pain / unowned positioning>
Does the target segment care? <yes — why>
Is it defensible? <yes — why it can't be trivially copied>

DIFFERENTIATION BASIS
<what we offer that the alternatives don't, for this segment, that matters and holds>
Honest concessions: <where competitors are genuinely better, and for whom>
```

## Integration

- Tier 2 — loaded for research-mode and campaign tasks
- `positioning-and-messaging` builds the message on this differentiation basis
- `honest-claims-discipline` — comparative claims must survive verification
- `market-research-methods` data-quality tagging applies
- `/competitors` command runs this procedure

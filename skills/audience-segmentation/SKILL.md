---
name: audience-segmentation
description: Defining concrete target-audience segments instead of vague "everyone" marketing. A segment is a specific group with a named pain, a known location, and a current alternative. Without concrete segmentation, every campaign message is diluted to please everyone and converts no one. Foundation of all marketing work.
---

# Audience Segmentation — Concrete Groups, Not "Everyone"

The single most common marketing failure: trying to reach "everyone". A message for everyone speaks to no one. Segmentation is the discipline of naming specific groups so the message can be sharp.

## Prerequisites

- Product or service to be marketed is defined
- Some access to data about real users (even informal)

## Core principle

> "Our audience is everyone who needs X" is not a segment — it is an admission that segmentation hasn't been done. A real segment is so concrete you could picture one specific person in it: their situation, their frustration, where they spend time, and what imperfect thing they use today.

## What a segment must contain (hard rule 3)

A segment is not real until it has all four:

1. **Who they are** — concrete description. Not "businesses" but "solo founders running a one-person consulting practice, 30-50 years old, non-technical".
2. **Pain point** — the specific frustration. Not "wants to be efficient" but "loses 5+ hours a week manually copying data between tools".
3. **Where to reach them** — the actual channels they're on. Not "online" but "Telegram channels about solopreneurship, niche Reddit communities, specific newsletters".
4. **Current alternative** — what they use today. Not "nothing" but "a messy spreadsheet plus sticky notes" or "a competitor product they find too expensive".

If any of the four is missing or vague, the segment isn't done.

## Why "current alternative" is the most important field

Most marketers skip it. It's the most valuable.

People rarely have "no solution" — they have a bad solution. Your product competes with that bad solution, not with nothing. Knowing the alternative tells you:
- What to compare against in messaging
- What switching cost the person faces
- What "good enough" looks like to them
- Why they haven't already solved the problem

A segment whose current alternative is "nothing, they live with the pain" behaves completely differently from one whose alternative is "a competitor". Different message, different conversion.

## Segmentation dimensions

Don't segment on demographics alone (age, location, income). Those are weak predictors of behavior. Segment on:

- **Job-to-be-done** — what outcome the person is trying to achieve (Christensen's framing). The strongest dimension.
- **Pain intensity** — how much the problem hurts. High-pain segments convert faster.
- **Awareness level** — do they know the problem? know solutions exist? know your product? (Schwartz's 5 levels — see `positioning-and-messaging`)
- **Current alternative** — as above
- **Behavioral** — how they currently act, what they've tried

Demographics are useful only as a *reachability* filter (which channel, which language), not as the segment definition itself.

## The procedure

### Step 1 — Gather signal
Pull what you know about real users: who's actually using/buying, support conversations, who churns, who converts fast. Even informal observation beats invented personas.

### Step 2 — Cluster by job-to-be-done
Group people by what outcome they want, not by who they are. Two people of different ages with the same job-to-be-done are the same segment.

### Step 3 — Name 2-4 segments
Not 1 (too coarse), not 10 (unfocused). 2-4 concrete segments. Each gets the four required fields.

### Step 4 — Estimate size
For each segment, rough size + the basis for the estimate. "About 5000, based on the subscriber count of the 3 Telegram channels where they cluster." Size with basis, never size alone.

### Step 5 — Rank by attractiveness
Score each segment: pain intensity × size × reachability × willingness to pay. Pick the priority segment(s) to target first.

### Step 6 — Mark validation status
Each segment starts `hypothesized`. It becomes `validated` only when data or a test confirms it behaves as described. Until then, treat it as a guess.

## Worked example

Product: the Fosved miniapp (the analytics interface for fosved-bot).

**Weak segmentation (what to avoid):**
"Our audience is people who want to be more productive and organized."
→ Useless. No pain, no location, no alternative, no concreteness.

**Strong segmentation:**

*Segment A — "Overloaded solo operator"*
- Who: runs a small business alone, non-technical, 35-55, makes all decisions
- Pain: drowning in scattered information, no single place to see the state of things, decisions feel like guesswork
- Where: Telegram channels about small business, specific entrepreneurship newsletters
- Current alternative: a chaotic mix of notes, spreadsheets, and memory
- Size: ~roughly several thousand, based on subscriber counts of 4 relevant Telegram channels
- Validation: hypothesized

*Segment B — "Delegating owner"*
- Who: business owner with a small team, wants oversight without micromanaging
- Pain: doesn't have a trustworthy summarized view of what's happening; relies on asking people
- Where: business owner communities, management-focused content
- Current alternative: weekly meetings + asking staff directly
- Size: smaller than A, harder to estimate
- Validation: hypothesized

Now a campaign can pick Segment A, speak to "scattered information and guesswork decisions", reach them in those specific Telegram channels, and frame against "your current chaos of notes and spreadsheets".

## Anti-patterns

- **"Everyone" / "anyone who needs it".** Not a segment. The refusal to segment.
- **Demographic-only segments.** "Men 25-40" tells you nothing about why they'd buy.
- **Invented personas.** Detailed fictional people ("Marketing Mary, 34, drinks oat milk lattes") with no grounding in real data. Decorative, not useful.
- **Skipping current alternative.** The most important field, most often omitted.
- **Too many segments.** 10 segments = no focus. Pick 2-4.
- **One segment.** Often hides the fact that distinct groups were lumped together.
- **Size without basis.** "10,000 people" — from where? Unbasis'd numbers are fiction.
- **Treating hypothesized as validated.** Acting on guessed segments as if confirmed.
- **Segmenting after the campaign is built.** Segment first; the message depends on it.

## Output template

```
SEGMENTATION — <product>

Segment <N> — "<short evocative name>"
- Who they are: <concrete description>
- Pain point: <specific frustration>
- Where to reach: <actual channels>
- Current alternative: <what they use now>
- Estimated size: <number> (basis: <how estimated>)
- Job-to-be-done: <the outcome they want>
- Attractiveness: pain <H/M/L> | size <H/M/L> | reachability <H/M/L> | willingness-to-pay <H/M/L>
- Validation status: hypothesized | validated | disproven

[repeat for 2-4 segments]

PRIORITY SEGMENT(S): <which to target first, and why>
```

This template populates the `AudienceSegment` model in the archive.

## Integration

- Tier 1 — loaded for every marketing task
- `forecast-before-launch` forecasts per the priority segment
- `positioning-and-messaging` writes the message FOR the chosen segment
- `channel-strategy` uses the "where to reach" field
- `market-research-methods` provides the data that validates segments
- Segments are stored in `AudienceSegment` and reused across campaigns

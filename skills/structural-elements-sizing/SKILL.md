---
name: structural-elements-sizing
description: The inverse of a capacity check — selecting member sections and connections to meet a structural demand with the required margin. Choosing beams, columns, rafters from standard ranges so the structure passes all limit states. Structural-mechanics discipline block. Output is preliminary, requires a licensed engineer's review.
---

# Structural Elements Sizing — Choosing Members to Meet the Demand

`structural-load-bearing` answers "does this structure hold". This skill answers the inverse: "what members make it hold". Given the loads, select the sections and connections — from real, available, standard ranges — so the structure passes every check with the norm's margin, without gross over-sizing.

## Prerequisites

- `norm-base-fixation` and `load-case-enumeration` complete
- The load effects are known (or computed as part of this task)
- `engineering-disclaimer-discipline` governs the output

## Core principle

> Sizing is not "pick something big enough" — it is selecting from real, standard, available sections the one that meets every limit state with the required margin and no reckless excess. Too small fails. Wildly too large wastes material and may not even be available. The right size is the smallest standard section that passes every check the norm demands.

## Sizing vs checking — the relationship

- **Checking** (`structural-load-bearing`): section is given → does it pass?
- **Sizing** (this skill): demand is given → what section passes?

Sizing is iterative: pick a candidate section, check it (using `structural-load-bearing`'s limit-states method), adjust, repeat until the smallest passing standard section is found.

## Size from real, standard ranges

A sized member must be something that actually exists. Members are selected from **standard ranges** — standard rolled steel sections, standard timber dimensions, standard tube sizes. A calculated "ideal" section that no supplier produces is not a usable result.

So sizing produces a *standard* section that passes — not an arbitrary computed dimension. The output names a real, specifiable member.

## The procedure

### Step 1 — Establish the demand
The internal forces each member must carry — bending moments, axial forces, shears — for the governing load combinations (from `load-case-enumeration`). Different members, different governing combinations.

### Step 2 — Choose the governing criterion per member
For each member, what is likely to govern its size — bending, buckling, deflection, connection capacity. A compressed slender member is likely buckling-governed; a long beam under snow may be deflection-governed. This focuses the iteration.

### Step 3 — First candidate from a standard range
Pick an initial standard section — a reasoned starting guess, from a real product range.

### Step 4 — Check the candidate (full limit-states)
Run the candidate through every relevant check — ULS (strength, buckling, shear, connections) and SLS (deflection) — per `structural-load-bearing`. Every failure mode, both limit states.

### Step 5 — Iterate
- Candidate fails a check → step up to the next standard section, re-check.
- Candidate passes with excessive margin → step down, re-check — find the smallest passing section.
- Candidate passes with appropriate margin → that is the size.

The target is the smallest standard section that passes every check with the norm's required margin.

### Step 6 — Size the connections
Members sized, the connections that join them are sized to the forces they transfer — bolts, welds, plates. Connections are part of the structure; an unsized connection is an incomplete result.

### Step 7 — Margin and consistency
Confirm the final selection's safety margins meet the norm (hard rule 4). Check consistency — sized members and connections form a coherent structure, not a set of individually-sized parts that don't fit together.

### Step 8 — Verdict, verification, documentation
Verdict: a complete set of standard sections that makes the structure pass / or "no standard section in the considered range passes — the structural concept must change" (a real and important outcome — stated plainly, hard rule 10). Then `independent-verification`, `calculation-documentation`.

## Avoid both under- and over-sizing

- **Under-sizing** — a section that fails a check. The dangerous error. The limit-states checks catch it.
- **Gross over-sizing** — a section far larger than needed. Not dangerous, but it wastes material, adds weight (which adds load — a heavier roof loads its supports more), raises cost, and can signal the sizing was not actually done — just "pick something huge". The discipline finds the *right* size, not a *safe-feeling huge* size.

A modest margin above the minimum is fine and often sensible (room for the reviewing engineer's adjustments, for future minor load increases). Gross over-sizing is a different thing — it usually means the iteration to find the smallest passing section was skipped.

## When nothing in the range passes

Sometimes no standard section passes — the span is too long, the load too high for the chosen structural concept. This is not a failure of the sizing; it is a finding: **the structural concept itself must change** — add supports, change the structural system, reduce the span. The skill states this plainly rather than producing an ever-larger section that strains plausibility. This finding goes to the reviewing engineer as a concept-level issue.

## Worked example (structure, not a substitute for the calculation)

Task: size the rafters and the valley member for the greenhouse roof. Norms RF, load cases enumerated.

**Step 1 — demand:** bending moments in the standard rafters under uniform snow; the higher moment in the valley member under drift accumulation; uplift forces under wind.

**Step 2 — governing criteria:** standard rafters — likely a balance of bending strength and deflection; valley member — bending under the heavy drift case; all members — buckling if slender, and connections under uplift.

**Step 3-5 — iterate:** for the standard rafter — pick a candidate standard steel section, check ULS + SLS; if deflection fails, step up; if it passes with large excess, step down; converge on the smallest standard section passing both. Repeat for the valley member (higher demand → likely a larger section). Repeat for compressed members with buckling as the focus.

**Step 6 — connections:** size the rafter-to-support and valley connections to the transferred forces — including the uplift case (the connection must hold the light roof down).

**Step 7 — margin/consistency:** confirm margins per SP; confirm the sized members and connections form a buildable, coherent frame.

**Step 8 — verdict:** a complete set of named standard sections that makes the roof pass — or, if the valley span is too great for any standard section at acceptable margin, the finding that the concept needs an extra support. Then verification, documentation, disclaimer, reviewing engineer.

## Anti-patterns

- **"Pick something big."** Skipping the iteration; choosing a huge section by feel. Wasteful, and not actually a calculation.
- **Non-standard sections.** Producing an "ideal" computed dimension no supplier makes. Unusable.
- **Sizing for one criterion.** Sizing for bending only, when deflection or buckling governs that member.
- **Connections unsized.** Members sized, joints forgotten. Incomplete — connections are part of the structure.
- **Ignoring self-weight feedback.** A heavier section adds dead load, which raises the demand. For heavy oversizing this loop matters — re-check.
- **Inconsistent set.** Members sized individually that don't form a coherent, buildable structure.
- **Endless up-sizing.** When nothing passes, growing the section indefinitely instead of stating the concept must change.
- **Gross over-sizing as "safety".** A far-too-large section feels safe but usually means the sizing was not done. Find the right size.
- **Treating the output as final.** Preliminary. A licensed engineer reviews and signs.

## Output

Produces the sizing content for `calculation-documentation`. Populates `EngineeringCalculation`: `method`, `resultSummary` (the selected standard sections and connections), `resultDetails` (the iteration and checks), `safetyFactor`, `verdict`. Discipline = `structural`.

## Integration

- Tier 2 — structural-mechanics block
- The inverse of `structural-load-bearing`; uses its limit-states checks inside the iteration
- Built on `norm-base-fixation` + `load-case-enumeration`
- `independent-verification` + `calculation-documentation` complete it
- Output preliminary — `engineering-disclaimer-discipline` and engineer review apply
- A "concept must change" finding may route to a feasibility discussion (`feasibility-estimation`)

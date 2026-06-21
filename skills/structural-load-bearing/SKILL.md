---
name: structural-load-bearing
description: Preparing load-bearing capacity calculations for structures — roofs, greenhouses, frames — by the limit-states method. Determining whether a structure withstands its load cases with the required safety margin. Structural-mechanics discipline block. Output is preliminary and requires a licensed engineer's review.
---

# Structural Load-Bearing — Will the Structure Hold

This skill prepares the central structural question: does the structure withstand everything that loads it, with the safety margin the norms require? It produces a documented preliminary calculation for a licensed engineer to review and approve.

## Prerequisites

- `norm-base-fixation` complete — structural norms fixed (SP 20/16/64, Eurocode, etc.)
- `load-case-enumeration` complete — every load case and combination listed
- `engineering-disclaimer-discipline` governs the output

## Core principle

> A structure is adequate not when it survives the load you expect, but when it survives the worst load combination the norm requires, with the norm's safety margin still in reserve. "It looks strong enough" is not an answer. The limit-states method is the answer: check every relevant failure mode against every load combination, and confirm the margin.

## The limit-states method

Modern structural norms (SP, Eurocode) work by **limit states** — conditions the structure must not reach:

**Ultimate limit states (ULS)** — failure, collapse:
- Strength — material stressed beyond capacity
- Stability — buckling of compressed members, overall instability
- Loss of equilibrium — overturning, sliding

**Serviceability limit states (SLS)** — unfit for use though not collapsed:
- Excessive deflection — sagging beyond the norm's limit
- Excessive vibration
- Local damage affecting use

A structure must satisfy **both**. A member can pass strength (ULS) and fail deflection (SLS) — it would not collapse but would sag unacceptably. Both are checked; the governing one decides.

## Failure modes to check (the paranoid list)

For each structural element and the structure as a whole, ask what could fail:

- **Bending** — a beam, a rafter under transverse load
- **Axial** — tension members; compression members (which also buckle)
- **Buckling** — compressed members failing by instability before reaching material strength; this is critical for slender members and easy to underestimate
- **Shear** — at supports, at connections
- **Connections** — bolts, welds, joints; connections often govern, and are often the weak point
- **Local effects** — web crippling, local buckling of thin sections
- **Overall stability** — the whole frame, not just members; sway, lateral-torsional buckling
- **Foundations / supports** — within scope only as load transfer; soil/foundation design is out of scope (refer to a geotechnical specialist)

For a light structure (greenhouse, light roof), **uplift and buckling** often govern, not downward strength — wind suction lifting a light roof, slender members buckling. The paranoid check looks for these specifically.

## The procedure

### Step 1 — Structural model
Define how the structure is idealized: members, supports (pinned / fixed — a conservative assumption if uncertain), how loads transfer. This idealization is an assumption set — documented per `calculation-documentation`.

### Step 2 — Load effects per combination
For each load combination (from `load-case-enumeration`), determine the internal forces — bending moments, axial forces, shears — in each element. The worst combination for each element may differ.

### Step 3 — Resistance per element
For each element, the resistance the norm gives — based on material, section, length, end conditions. Material strengths and partial factors from the fixed norms.

### Step 4 — ULS checks
For every element, every relevant failure mode: is the load effect below the resistance? Strength, buckling, shear, connections. Every mode — a passed strength check does not excuse an unchecked buckling check.

### Step 5 — SLS checks
Deflection against the norm's limit. Other serviceability criteria if relevant.

### Step 6 — Safety margin
The safety factor / utilization ratio for each element. Confirm it meets the norm (hard rule 4). The governing element is the one with the least margin.

### Step 7 — Verdict
Passes (all elements, all modes, both limit states, with margin) / fails (any check not met) / conditional (passes given provisional inputs that must be confirmed). Stated plainly (hard rule 10).

### Step 8 — Independent verification and documentation
`independent-verification` then `calculation-documentation`. Status → `self_verified` → ready for the reviewing engineer.

## Worked example (structure of the calculation, not a substitute for one)

Task: load-bearing check of a greenhouse roof, light steel frame. Norms RF (SP 20/16/17). Load cases enumerated — governing cases flagged: valley snow drift, wind uplift.

**Step 1 — model:** the roof idealized as a series of rafters on supports; connections assumed pinned (conservative — a pinned connection attracts no moment, so the member is sized for the more demanding simply-supported case). Documented as an assumption.

**Step 2 — load effects:** for the valley-drift combination, the bending moments in the valley rafters; for the wind-uplift combination, the uplift forces in members and connections.

**Step 3 — resistance:** for the chosen steel sections — bending resistance, buckling resistance (the rafters are slender — buckling matters), connection resistance. Material partial factor from SP 16.

**Step 4 — ULS:**
- Bending — valley rafter under drift load: load effect vs bending resistance
- Buckling — slender members in compression: this is checked explicitly, not assumed away
- Connections under wind uplift — the light-roof danger: does the connection hold the roof down?

**Step 5 — SLS:** deflection of the rafters under the snow case vs the norm's deflection limit. (A member can pass bending strength and fail this — then deflection governs.)

**Step 6 — margin:** utilization ratio per element. The element with least margin governs. Confirm margins meet SP requirements.

**Step 7 — verdict:** if every element passes every mode in both limit states with adequate margin → passes. If the valley rafter fails the drift case, or a connection fails uplift → fails, stated plainly, with which element and which mode.

**Step 8 — verification + documentation**, then to the reviewing engineer with the disclaimer.

This is the *structure* of the calculation. The actual numbers depend on real inputs, real norms, real sections — and the result is preliminary regardless: a licensed structural engineer reviews and approves.

## Anti-patterns

- **Strength only, no stability.** Checking material strength but not buckling. Slender compressed members fail by buckling first — a strength-only check misses it.
- **ULS only, no SLS.** Checking collapse but not deflection. A member can be strong enough and still sag unacceptably.
- **Forgetting uplift.** For light roofs, wind suction (uplift) often governs over downward load. A "loads push down" mindset misses it.
- **Connections unchecked.** Sizing members but not the joints. Connections often govern and are often the weak point.
- **Whole-structure stability ignored.** Checking members individually but not overall frame stability / sway.
- **Optimistic idealization.** Assuming fixed supports (which reduce member forces) when pinned is the conservative, uncertain-case assumption.
- **Out-of-scope foundation work.** Designing the foundation or soil bearing — that is geotechnical, out of scope; refer it.
- **Single governing case assumed.** Checking only the case that seemed worst, when different elements are governed by different combinations.
- **Treating the output as final.** It is preliminary. A licensed engineer reviews and signs.

## Output

Produces the calculation content for `calculation-documentation` to assemble. Populates `EngineeringCalculation` fields: `method`, `resultSummary`, `resultDetails`, `safetyFactor`, `safetyFactorRequired`, `safetyFactorOk`, `verdict`. Discipline = `structural`.

## Integration

- Tier 2 — structural-mechanics block; loaded for structural tasks
- Built on `norm-base-fixation` + `load-case-enumeration`
- `structural-elements-sizing` handles the inverse task (choosing sections to meet a demand)
- `independent-verification` then `calculation-documentation` complete the calculation
- Output is preliminary — `engineering-disclaimer-discipline` and the engineer-review lifecycle apply

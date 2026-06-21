---
name: feasibility-estimation
description: Quick feasibility and order-of-magnitude estimates — answering "is this even possible, and roughly in what range" before committing to a full calculation. Explicitly an estimate, not a calculation. Used early, to scope a problem or screen an idea. Output is preliminary and clearly marked as an estimate.
---

# Feasibility Estimation — Is This Even Possible, Roughly

Before a full calculation is worth doing, a faster question often comes first: is this idea even feasible, and roughly in what range? A feasibility estimate answers that — quickly, approximately, and explicitly labelled as an estimate rather than a calculation.

## Prerequisites

- A discipline is identifiable (structural / hydraulic / power electronics)
- `engineering-disclaimer-discipline` governs the output — with an extra estimate caveat
- Tier 3 — used deliberately, for early scoping, not for every task

## Core principle

> A feasibility estimate and a calculation are different things, and the difference must never blur. An estimate answers "is this in the realm of possible, roughly where" with deliberate approximation. A calculation answers "does this specific design pass, precisely" with norms and verification. An estimate presented as a calculation is dangerous; a calculation skipped because an estimate "looked fine" is dangerous. The estimate's job is to decide whether the calculation is worth doing — nothing more.

## When a feasibility estimate is the right tool

Use it when:
- Screening an idea before investing in a full calculation — "is this worth pursuing?"
- Scoping — getting a rough sense of the size of a problem, what dominates
- Comparing options at a coarse level — "which of these three is roughly most promising"
- Sanity-checking a proposal — "does this claim even pass an order-of-magnitude check"

Do NOT use it when:
- A decision to build or buy depends on the result — that needs a calculation
- The answer is close to a limit — "roughly fine" near a boundary needs precision
- It would be mistaken for a calculation — if the recipient might act on it as final, do the calculation instead

## Estimate vs calculation — the explicit difference

| | Feasibility estimate | Calculation |
|---|---|---|
| Question | Is it possible, roughly where? | Does this design pass, precisely? |
| Norms | Awareness of them; not applied in full | Fixed and applied (`norm-base-fixation`) |
| Load cases | The dominant ones, roughly | Full enumeration (`load-case-enumeration`) |
| Method | Simplified, order-of-magnitude | Full method per norm |
| Verification | The estimate's own rough cross-check | Independent verification (`independent-verification`) |
| Precision | Order of magnitude, ranges | To the norm's required precision |
| Output label | "ESTIMATE — not a calculation" | "PRELIMINARY calculation — needs engineer review" |

Both carry the disclaimer. The estimate carries an *additional* caveat: it is not even a calculation.

## The procedure

### Step 1 — Frame the feasibility question
What is actually being asked? "Could a greenhouse of this span work at all?" "Is a hydraulic lift the right order of magnitude for this load?" "Could a solar array of this size cover this load?" A sharp question.

### Step 2 — Identify what dominates
In any estimate, a few factors dominate and the rest are noise. Find the dominant ones. For a structure — the main span and the main load. For a lift — the load and the height. For a solar system — the load energy and the available generation. Estimate around the dominant factors; ignore the small terms.

### Step 3 — Order-of-magnitude calculation
A deliberately rough calculation. Round numbers, simplified geometry, typical values, generous approximation. The aim is the right order of magnitude and a rough range — not a precise number.

### Step 4 — Compare to what is normal
Put the rough result against known reference points. Is the required member size in the range of what such structures normally use, or absurd? Is the lift pressure ordinary or extreme? Is the array area reasonable for the site or impossible? Reference points turn a rough number into a feasibility judgment.

### Step 5 — Verdict — in feasibility terms
- **Feasible** — comfortably in the range of the normal; a full calculation will likely confirm a workable design
- **Feasible but tight** — near the edge; possible, but the full calculation matters and the margins will be slim
- **Infeasible as posed** — outside the plausible range; the concept needs to change (more supports, different system, smaller span)
- **Cannot tell from an estimate** — too close to call roughly; only a calculation decides

### Step 6 — State what a real calculation would need
If the idea proceeds, list what the full calculation requires — the data, the norm fixing, the load enumeration. The estimate hands off to the calculation cleanly.

### Step 7 — Label and disclaim
The output is marked, prominently: an ESTIMATE, not a calculation, plus the standard disclaimer.

## Worked example

Question: a client asks, before any design work — "could we span a greenhouse roof this wide with a light steel frame, or is that unrealistic?"

**Step 1 — question:** is a light-steel greenhouse roof of this span feasible at all, before committing to a full design calculation?

**Step 2 — dominant factors:** the span, and the snow load for the region (snow usually dominates a greenhouse roof). Self-weight of a light frame is secondary; wind matters but the snow case is the screening case.

**Step 3 — order-of-magnitude:** a rough simply-supported estimate — the span, a rough snow load, a simple beam relation — gives a rough required member size for a single-span roof.

**Step 4 — compare:** is that rough member size in the range light-steel greenhouses normally use? If it lands in the ordinary range — feasible. If the rough estimate already demands a heavy section unlike anything used in such greenhouses — that single span is unrealistic for a light frame.

**Step 5 — verdict:** likely one of —
- "Feasible — the span is in the normal range for a light-steel greenhouse; a full calculation should confirm a workable design."
- "Feasible but tight — the span is at the upper end; a full calculation is essential and the design will need care, possibly an intermediate support."
- "Infeasible as a single span for a light frame — the concept needs intermediate supports or a different structural system."

**Step 6 — handoff:** if it proceeds — the full calculation will need the exact span, the regional snow and wind data, the norm fixing, the full load enumeration.

**Step 7 — label:** marked clearly as an ESTIMATE, not a calculation, with the disclaimer.

The estimate did its job: it decided, quickly, whether the full calculation is worth doing — and gave the client a rough answer — without pretending to be the calculation.

## Anti-patterns

- **Estimate presented as a calculation.** The cardinal sin. An order-of-magnitude estimate dressed as a precise result — and acted on as if final.
- **Building on an estimate.** "The estimate looked fine, so we built it." An estimate never substitutes for the calculation when a build decision depends on it.
- **Estimate near a limit.** Using a rough estimate when the answer sits close to a boundary. "Roughly OK" near the edge is not OK — that needs precision.
- **Over-precise estimate.** Producing many significant figures from a rough method — false precision that invites it to be mistaken for a calculation.
- **No reference comparison.** A rough number with nothing to compare it to. A number without a reference point is not a feasibility judgment.
- **Missing the dominant factor.** Estimating around minor terms and missing what actually dominates.
- **No handoff.** Concluding "feasible" without stating what the real calculation then needs.
- **Skipping the extra caveat.** An estimate carries the disclaimer *and* the explicit "this is an estimate, not a calculation" note. Both.

## Output template

```
[ disclaimer block — top ]
⚠ ОЦЕНКА ОСУЩЕСТВИМОСТИ — это НЕ расчёт.
   Порядок величины для предварительного скрининга. Не является
   расчётом и не заменяет его. Решение о реализации требует
   полноценного расчёта.

FEASIBILITY ESTIMATE — <question>
Discipline: <structural | hydraulic | power_electronics>

DOMINANT FACTORS: <what governs, roughly>
ORDER-OF-MAGNITUDE WORKING: <rough method, round numbers>
ROUGH RESULT: <result, as a range / order of magnitude>
REFERENCE COMPARISON: <how it compares to what is normal>

FEASIBILITY VERDICT: feasible / feasible but tight / infeasible as posed / cannot tell from estimate
<reasoning>

IF PURSUED, A FULL CALCULATION WILL NEED: <data, norms, load enumeration>

[ disclaimer block — bottom ]
```

This populates a `DesignReview` record with `reviewType: feasibility_check`, or an early-stage `EngineeringCalculation` clearly flagged as an estimate.

## Integration

- Tier 3 — used deliberately, for early scoping; not every task needs it
- Precedes the full process — if it says "pursue", `norm-base-fixation` → `load-case-enumeration` → the Tier 2 calculation follow
- An "infeasible as posed" verdict feeds back as a concept-level finding
- `engineering-disclaimer-discipline` applies, with the extra estimate caveat
- Distinct from `third-party-calc-review` — that reviews someone's finished calculation; this estimates a fresh idea

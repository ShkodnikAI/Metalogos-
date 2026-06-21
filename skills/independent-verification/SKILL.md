---
name: independent-verification
description: Verifying every calculation by an independent second method before it is considered complete — a different calculation path, an order-of-magnitude estimate, a dimensional check. A single calculation method, unchecked, is a guess. LLMs are confidently wrong with numbers; independent verification is the barrier that catches it.
---

# Independent Verification — One Method Is Not a Calculation

A number produced by a single calculation path, never cross-checked, is not a verified result — it is a hypothesis. Independent verification is the discipline of confirming every result by a second, different route. For a department where errors can mean collapse, and where the tool (an LLM) is known to be confidently wrong with arithmetic, this barrier is not optional.

## Prerequisites

- A calculation has been performed (a primary result exists)
- `norm-base-fixation` and `load-case-enumeration` complete

## Core principle

> A calculation checked only against itself is checked against nothing. Verification means a *second, independent* route to the same answer — a different method, a rough estimate, a dimensional analysis. If the two routes agree, confidence is real. If they disagree, one of them is wrong and the result is not done. A result that has passed only one method has not been verified; it has merely been computed.

## Why this is hard rule 7 and 8, and why it matters more here

Two reasons this barrier is mandatory:

1. **The stakes.** A structural or mechanical error that gets through can cause collapse. The cost of a missed error is not "redo the work" — it is catastrophic.

2. **The tool.** The department's calculations are assisted by an LLM. LLMs are *fluently, confidently wrong* with numbers — they produce plausible-looking arithmetic that is simply incorrect, and they do not signal doubt. This is a known, specific failure mode. Independent verification is the barrier engineered specifically to catch it.

A calculation does not reach `self_verified` status until it has passed independent verification. It is not sent to the reviewing engineer before that — sending unverified work wastes the engineer's time and inflates the correction rate.

## The three verification routes

### Route 1 — Independent method (the strongest)

Re-derive the result by a genuinely different calculation path. Not re-running the same arithmetic — a different *method*.

Examples:
- A member sized by a stress check → verify by a deflection check, or by a different formula from the norm
- A load found by one decomposition → verify by a different decomposition
- A hydraulic force from pressure × area → verify via an energy/work balance
- An inverter sizing from power → verify from current and voltage limits separately

If the two methods agree within an acceptable tolerance, the result is corroborated. If they diverge, stop — one is wrong.

### Route 2 — Order-of-magnitude estimate

A deliberately rough, simplified estimate — is the result even in the right ballpark?

A precise calculation that produces a beam needing a 2mm section for a 10-meter span is precisely wrong — an order-of-magnitude sense check catches it instantly. The rough estimate doesn't need to be accurate; it needs to confirm the precise result isn't off by 10× or 100×.

This catches the worst errors — the ones where a misplaced decimal or a unit confusion produces a result that is not slightly off but absurd.

### Route 3 — Dimensional analysis (always, mandatory)

Check that the units work out. Every formula, every step: do the dimensions on the left equal the dimensions on the right?

A result that comes out in the wrong units is definitely wrong — and dimensional analysis catches an entire class of errors (using a formula wrong, mixing unit systems, dropping a factor) cheaply and certainly. This route is done on *every* calculation, always — it is hard rule 8.

## The verification must be genuinely independent

The danger: a "verification" that just repeats the original method's assumptions and so cannot catch the original method's error. If the primary calculation used a wrong coefficient and the "check" uses the same wrong coefficient, the check agrees — and confirms nothing.

Genuine independence means:
- A different method, not the same method recomputed
- Where possible, deriving from different starting quantities
- The dimensional check is independent by nature — it checks units regardless of method

If a true independent method is not feasible for some step, say so — and lean harder on the order-of-magnitude and dimensional routes, and flag for the reviewing engineer that this step has weaker internal verification.

## The procedure

### Step 1 — Dimensional check (always)
Walk the calculation. Every equation: do the units balance? Any mismatch → error located, fix before proceeding.

### Step 2 — Order-of-magnitude estimate (always)
A rough, simplified estimate of the result. Is the precise result in the same ballpark? A divergence of 10×+ → something is badly wrong, find it.

### Step 3 — Independent method (where feasible)
Re-derive the key results by a different method. Compare to the primary result.

### Step 4 — Compare and judge
- All routes agree within tolerance → result corroborated, status → `self_verified`
- A route disagrees → the result is *not* verified. Find which is wrong. Do not proceed, do not send to the engineer, until resolved.

### Step 5 — Record the verification
What method(s) were used, what they gave, whether they agreed. This populates `verificationMethod` and `verificationAgrees` on `EngineeringCalculation`. An unrecorded verification did not happen.

## When the routes disagree

Disagreement is not a problem to be smoothed over — it is the verification *working*. It means an error exists and has been caught before reaching the engineer or the build.

Do not:
- Pick the answer you prefer
- Average the two
- Assume the primary is right because it was "more careful"

Do:
- Find the actual error. Re-examine both routes step by step. One has a mistake — locate it.
- Only when the error is found and corrected, and the routes then agree, is the result verified.

## Worked example

Calculation: a steel member in a greenhouse roof, sized for the governing load combination (valley snow drift, from `load-case-enumeration`).

**Primary method:** stress check — required section modulus from bending moment and allowable stress. Result: a member of a certain size.

**Route 3 — dimensional check:** moment [force × length] ÷ allowable stress [force / length²] → section modulus [length³]. Units balance. ✓

**Route 2 — order of magnitude:** rough estimate — span, rough load, simple beam formula. Gives a member size in the same range as the primary result. ✓ (If the primary had given something 10× smaller, this is where it would be caught.)

**Route 1 — independent method:** verify by the deflection criterion — the member must also not deflect more than the norm's limit. Compute the deflection for the primary-sized member. If deflection is within limits → the stress-based size is corroborated from a different criterion. If the member passes stress but *fails* deflection → the governing criterion is deflection, the primary result is incomplete, and the member must be resized.

**Outcome:** if all three agree, status → `self_verified`, the calculation can go to the reviewing engineer. If the deflection check governs and the stress-based size is insufficient — that disagreement just caught a real, classic error (sizing for strength but forgetting serviceability). The result is corrected before anyone's time is wasted.

## Anti-patterns

- **One method, no check.** A single calculation path, taken as the answer. Unverified.
- **Re-running the same method.** "Verifying" by recomputing the same arithmetic. Catches typos, not method errors.
- **Non-independent check.** A "verification" that reuses the original's wrong assumption, so it agrees and confirms nothing.
- **Skipping the dimensional check.** It is mandatory, every time. It is the cheapest, most certain error-catcher.
- **Skipping the sanity estimate.** Trusting a precise number without asking "is this even the right order of magnitude".
- **Trusting LLM arithmetic.** Taking a computed number at face value. The tool is confidently wrong with numbers — that is exactly what verification exists to catch.
- **Smoothing over disagreement.** Averaging, or picking the preferred answer, when routes disagree. Disagreement means find the error.
- **Sending unverified work to the engineer.** Skipping `self_verified` — wastes the engineer's time, inflates the correction rate.
- **Unrecorded verification.** Doing the check but not recording it. Then it cannot be confirmed it happened.

## Output template (verification block — part of every calculation)

```
INDEPENDENT VERIFICATION

Dimensional check (mandatory): performed — units balance: yes / no [if no: error located at <...>]

Order-of-magnitude estimate: <rough method> → <rough result>
  Primary result <value> is within the same order: yes / no

Independent method: <the different method used>
  → <result by independent method>
  Agreement with primary (<value>): within tolerance / DISAGREES
  [if independence not feasible for a step: stated, flagged for reviewing engineer]

VERDICT: self-verified (all routes agree) / NOT verified — discrepancy at <...>, resolved by <...>
```

This populates `verificationMethod`, `verificationAgrees`, `dimensionalCheckOk` on `EngineeringCalculation`. The calculation reaches `self_verified` status only when this block shows agreement.

## Integration

- Tier 1 — loaded for every calculation
- Runs after the Tier 2 calculation skill produces a primary result
- The `self_verified` status gate sits here — no calculation goes to the engineer without it
- `independent-method agreement` and `dimensional-error rate` are department metrics
- `dimensional_error` and `method_error` are tracked error classes in `EngineeringCalculationVerification`
- `engineering-disclaimer-discipline` — even a self-verified result still carries the disclaimer and still needs the engineer

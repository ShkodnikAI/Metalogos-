---
name: calculation-documentation
description: Documenting a calculation so a reviewing engineer can follow it step by step and arrive at the same result. Records norms, input data, load cases, assumptions, method, every step, the result, and the verification. An undocumented calculation cannot be reviewed, cannot be approved, and cannot be learned from.
---

# Calculation Documentation — A Calculation a Reviewer Can Follow

The department's output is not a number — it is a *documented calculation* that a licensed engineer can pick up, follow step by step, check, and approve. A bare result, however correct, is useless to a reviewer who cannot trace how it was reached. Documentation is what makes the department's work reviewable, approvable, and improvable.

## Prerequisites

- The calculation is complete and `self_verified` (`independent-verification` passed)
- `norm-base-fixation`, `load-case-enumeration` outputs available

## Core principle

> The reviewing engineer must be able to reconstruct the entire calculation from the documentation alone — same inputs, same norms, same assumptions, same steps, same result. If any link is missing — an unstated assumption, an unshown step, a coefficient with no source — the reviewer cannot verify that link, and an unverifiable link is where errors hide. Document so completely that the calculation could be re-derived by a stranger.

## Why documentation is a Tier 1 discipline

A calculation that is correct but undocumented:
- **Cannot be reviewed** — the engineer cannot check reasoning they cannot see
- **Cannot be approved** — a licensed engineer will not sign what they cannot trace
- **Cannot be learned from** — when a calculation is later found wrong, an undocumented one gives no clue where the error was
- **Cannot be reused** — a similar future task starts from zero

The documentation is not paperwork added at the end. It is the deliverable.

## What a documented calculation contains

In order:

### 1. Disclaimer block
The mandatory disclaimer (`engineering-disclaimer-discipline`), at the top.

### 2. Task statement
What is being calculated, the object, the discipline. Plain enough that a reviewer understands the scope before the detail.

### 3. Norm base
Which norms govern, editions, what each governs (`norm-base-fixation` output). The reviewer must know the rules the calculation followed.

### 4. Input data
Every input parameter, with value, unit, and **certainty tag** — known / assumed / provisional (hard rule 9). The reviewer must see which inputs are firm and which need confirmation. Missing data explicitly listed.

### 5. Load cases and combinations
The full enumeration (`load-case-enumeration` output) — every case and combination considered. The reviewer checks completeness here.

### 6. Assumptions
Every assumption, stated explicitly, each one conservative (hard rule 5). The reviewer must be able to see and judge each assumption — an unstated assumption is an unverifiable one.

### 7. Method
The calculation method, per the norm. Which formulas, which approach, why.

### 8. The calculation itself — every step
Not just the result — the working. Each step shown: the formula, the values substituted, the intermediate result, the units. A reviewer follows it line by line.

### 9. Result and verdict
The result, the safety factor and whether it meets the norm (hard rule 4), and the plain verdict — passes / fails / conditional / needs more data (hard rule 10).

### 10. Independent verification
The verification block (`independent-verification` output) — what methods checked the result, whether they agreed.

### 11. Disclaimer block (again)
The disclaimer repeated at the bottom.

## Show the working, not just the answer

The single most common documentation failure: presenting the result without the steps. "The member needs section X" — but how? From what moment, what stress, what formula?

A reviewer cannot verify a leap. Every step from input to result must be visible:
- The formula used (and its source — which norm clause)
- The values substituted in
- The arithmetic
- The intermediate result, with units
- The next step following from it

If a step is "obvious", show it anyway — obvious to the writer is not always obvious to the reviewer, and the reviewer's job is to check, not to assume.

## Every assumption explicit and conservative

Assumptions are where calculations quietly go wrong. Two rules:

1. **Explicit** — every assumption written down. "Assumed the connection is pinned, not fixed." "Assumed uniform snow distribution for this case." If it was assumed, it is on the page.

2. **Conservative** — every assumption errs toward safety, never toward making the structure pass. If unsure whether to assume pinned or fixed, assume the one that gives the more demanding result. This is hard rule 5 — the calculation never tilts assumptions to flatter the result.

The reviewer reads the assumptions and judges each. An assumption they cannot see, they cannot judge.

## Certainty tagging of inputs

Each input is tagged (hard rule 9):
- **Known** — firm, measured, given by a reliable source
- **Assumed** — the department chose a value; conservative
- **Provisional** — a placeholder; the real value must be confirmed

A result that depends on provisional inputs is itself provisional, and the documentation says so. The reviewer must not mistake a calculation built on placeholders for one built on firm data.

## The procedure

### Step 1 — Assemble in order
Lay out the eleven sections above, in sequence.

### Step 2 — Fill from the prior skills
Norms from `norm-base-fixation`, load cases from `load-case-enumeration`, verification from `independent-verification`. The documentation gathers what the process already produced.

### Step 3 — Write the working in full
The calculation steps, each shown completely — formula, source, substitution, intermediate result, units.

### Step 4 — Surface every assumption
Walk the calculation and extract each assumption. State it, confirm it is conservative.

### Step 5 — Tag every input
Known / assumed / provisional. List missing data.

### Step 6 — State the verdict plainly
Passes / fails / conditional / needs more data. No hedging (hard rule 10).

### Step 7 — Bracket with disclaimers
The disclaimer block top and bottom.

### Step 8 — Self-check for reviewability
Read it as if you were the reviewing engineer. Can every step be followed? Every assumption seen? Every coefficient traced to a norm? If any "no" — the gap is fixed before the calculation is sent.

## Anti-patterns

- **Result without working.** The answer with no steps. The reviewer cannot verify a leap.
- **Hidden assumptions.** Assumptions made but not written down. Unverifiable — and where errors hide.
- **Non-conservative assumptions.** Assumptions chosen to make the structure pass. Violates hard rule 5.
- **Coefficients with no source.** A number used with no norm clause cited. The reviewer cannot check it.
- **Untagged inputs.** Not distinguishing firm data from placeholders. The reviewer mistakes a provisional calculation for a firm one.
- **Missing data not listed.** Gaps in input data not flagged. The reviewer does not know what is unconfirmed.
- **Hedged verdict.** "Should be roughly OK" instead of a plain passes / fails. Violates hard rule 10.
- **Disclaimer omitted or buried.** Violates hard rule 1.
- **Documentation as afterthought.** Treating it as paperwork rather than the deliverable. The documentation *is* the product.
- **Not self-checking for reviewability.** Sending without reading it from the reviewer's seat.

## Output template (the documented calculation)

```
[ disclaimer block — top ]

ENGINEERING CALCULATION — <title>
Discipline: <structural | hydraulic | power_electronics>
Object: <object>

1. TASK
<what is being calculated, scope>

2. NORM BASE
<from norm-base-fixation>

3. INPUT DATA
<parameter> = <value> <unit>  [known | assumed | provisional]
[...]
Missing data: <list, or "none">

4. LOAD CASES & COMBINATIONS
<from load-case-enumeration>

5. ASSUMPTIONS  (each conservative)
- <assumption> — conservative because <...>
[...]

6. METHOD
<calculation method, per norm>

7. CALCULATION (full working)
Step 1: <formula> [source: <norm clause>]
        <substitution> = <intermediate result> <unit>
Step 2: ...
[...]

8. RESULT & VERDICT
Result: <...>
Safety factor: <value>  | required by norm: <value>  | meets norm: yes/no
VERDICT: passes | fails | conditional | needs more data
<if fails: stated plainly, no softening>

9. INDEPENDENT VERIFICATION
<from independent-verification>

[ disclaimer block — bottom ]
```

This populates the documentation fields of `EngineeringCalculation` and is what is sent to the reviewing engineer.

## Integration

- Tier 1 — loaded for every calculation; the final assembly step
- Gathers outputs of `norm-base-fixation`, `load-case-enumeration`, `independent-verification`
- Embeds the `engineering-disclaimer-discipline` block
- The documented calculation is what moves to `sent_for_review` — the reviewing engineer works from it
- A well-documented calculation that is later corrected shows exactly which step failed — feeding the learning loop

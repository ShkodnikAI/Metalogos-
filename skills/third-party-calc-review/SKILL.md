---
name: third-party-calc-review
description: Sanity-checking someone else's engineering calculation — finding gross errors, missing load cases, wrong norms, unsupported assumptions. A review, not an approval — it does not sign off the calculation, it surfaces problems for the responsible engineer. Used when evaluating a contractor's, supplier's, or partner's calculation.
---

# Third-Party Calculation Review — Finding the Gross Errors

Sometimes the task is not to produce a calculation but to *check* one — a calculation a contractor, supplier, or partner has submitted. This skill is the disciplined sanity-check: surfacing gross errors, missing cases, wrong norms, and weak assumptions. It is a review, not an approval.

## Prerequisites

- A third-party calculation to review (or an internal one being sanity-checked)
- The discipline is one of the three (structural / hydraulic / power electronics)
- `engineering-disclaimer-discipline` governs the output
- Tier 3 — used when the task is reviewing rather than producing

## Core principle

> Reviewing a calculation is not approving it. A review can say "I found these problems" or "I found no gross errors" — it can never say "this is correct and safe to build". Finding no errors in a review is not proof of correctness; it is the absence of *found* errors, which is a much weaker statement. The review surfaces problems for the engineer who carries responsibility; it does not lift that responsibility onto itself.

## Review is not approval — the critical boundary

This must be unambiguous:

- A **review** examines a calculation and reports what it finds — errors, gaps, weaknesses, or none of those.
- An **approval** is a licensed engineer taking responsibility for the calculation's correctness.

The department reviews. It never approves. Even a thorough review that finds nothing wrong produces only: "no gross errors found in this review" — never "this calculation is correct". The difference is not pedantry. A reviewer can miss what a calculation's author missed; "I found no errors" and "there are no errors" are different statements, and conflating them is how a flawed calculation gets a false blessing.

The output of a review carries the disclaimer like everything else — and the disclaimer here specifically notes that a review is not an approval.

## What a review looks for

A sanity-check, working through the same discipline the department's own calculations follow — but applied to someone else's work:

**Norm base:**
- Is a normative base stated at all?
- Are the norms the right ones for this object and jurisdiction?
- Are they the current editions?

**Load cases:**
- Is there a load-case enumeration?
- Are obvious cases missing? (snow drift, wind uplift, dynamic loads, the holding case for a lift, fault cases for an inverter)
- Are the load combinations per the norm?
- This is the highest-value part of a review — a missing load case is the most dangerous and the most common gross error.

**Assumptions:**
- Are assumptions stated, or hidden?
- Are they conservative, or do they tilt toward making the design pass?
- Is any assumption doing suspicious heavy lifting — one convenient assumption that, if wrong, breaks the result?

**Method and numbers:**
- Is the method appropriate for the problem and the norm?
- Order-of-magnitude check — are the results in a plausible range, or absurd?
- Dimensional check — do the units work?
- Spot-check key steps — not a full re-calculation (that would be doing the calculation, not reviewing it), but checking the critical steps.

**Verification and margin:**
- Did the author verify the result independently?
- Is the safety margin stated and does it meet the norm?
- Is the verdict (passes/fails) clearly stated?

**Completeness:**
- Are all elements covered, or only some?
- Connections checked, not just members?
- Both limit states (ULS and SLS)?

## Gross errors vs minor issues

A review classifies what it finds by severity:

- **Critical** — would lead to an unsafe result: a missing governing load case, a wrong norm, a non-conservative assumption that breaks the result, a fundamental method error.
- **Major** — a significant problem that should be corrected: an incomplete check, a questionable assumption, a missing verification.
- **Minor** — should be fixed but does not threaten the result: documentation gaps, unclear steps, small inconsistencies.
- **Note** — observations, not problems.

The review's job is especially to catch the **critical** ones — the gross errors that make a calculation unsafe. A review that lists ten minor formatting issues and misses one missing load case has failed.

## The procedure

### Step 1 — Understand what is being reviewed
What the calculation is for, the object, the discipline, what the third party claims it shows.

### Step 2 — Check the norm base
Is one stated? Right norms? Current editions? A calculation with no norm base is itself a critical finding.

### Step 3 — Check the load cases (the high-value step)
Is there an enumeration? Run the department's own `load-case-enumeration` mentally — what cases *should* be there? Is anything missing? Missing governing cases → critical.

### Step 4 — Examine the assumptions
Stated or hidden? Conservative or convenient? Any single assumption carrying too much weight?

### Step 5 — Sanity-check method and numbers
Method appropriate? Order-of-magnitude plausible? Dimensions consistent? Spot-check critical steps — not a full re-do.

### Step 6 — Check verification, margin, completeness
Verified by the author? Margin stated and meets norm? All elements, both limit states, connections covered?

### Step 7 — Classify findings
Each finding → critical / major / minor / note.

### Step 8 — Verdict and recommendation
- **Sound** — no gross errors found in this review (NOT "correct")
- **Has issues** — problems found, listed by severity, should be corrected
- **Unsound** — critical errors found; the calculation should not be relied upon as-is
- **Inconclusive** — the calculation cannot be properly reviewed (too little shown, key parts missing)

Recommendation: what the responsible engineer should do — accept with the noted minor fixes, return for correction, reject, or request more.

### Step 9 — Disclaim
The output carries the disclaimer, with the explicit note: this is a review, not an approval; the responsible licensed engineer decides.

## Worked example

Task: review a contractor's calculation for a canopy roof, submitted as "checked and adequate".

**Step 2 — norm base:** the contractor cites a structural norm — check it is the right one and the current edition. Suppose it is — note as adequate.

**Step 3 — load cases:** the calculation shows dead load and uniform snow. Run the mental enumeration: a canopy — what about **wind uplift**? A canopy is exactly the structure where wind suction governs, and it is absent. **Critical finding: wind uplift case missing.**

**Step 4 — assumptions:** the calculation assumes fixed connections. For a canopy, are the connections actually fixed? If they are pinned in reality, the members are under-designed. **Major finding: the fixed-connection assumption is not justified and is not conservative.**

**Step 5 — sanity check:** order-of-magnitude of the member sizes — plausible for the snow case shown. Dimensions consistent. The shown steps spot-check OK.

**Step 6 — verification/margin:** no independent verification shown by the author. **Major finding: no verification.** Margin stated, meets norm for the cases considered.

**Step 7-8 — classify and verdict:**
- Critical: wind uplift case missing
- Major: fixed-connection assumption unjustified; no independent verification
- Verdict: **has issues**, leaning unsound — the missing uplift case is critical for a canopy. Recommendation: return to the contractor for correction; the wind uplift case must be added and the connection assumption justified or revised; then the responsible engineer reviews the corrected version.

**Step 9 — disclaim:** the review carries the disclaimer and the note — this is a review surfacing problems, not an approval. The responsible licensed engineer decides what to accept.

The review did its job: it caught a critical gross error (a classic missing load case for that structure type) that the contractor's "checked and adequate" had missed — and it did so without ever claiming to approve or certify the calculation.

## Anti-patterns

- **Review as approval.** Saying "this calculation is correct" or "approved". A review never approves. The strongest it says is "no gross errors found".
- **"No errors found" = "correct".** Treating a clean review as proof of correctness. It is only the absence of *found* errors — a weaker statement.
- **Re-doing instead of reviewing.** Performing the whole calculation from scratch — that is producing a calculation, not reviewing one. Review spot-checks; it does not duplicate.
- **Minor-issue tunnel vision.** Listing formatting and documentation nitpicks while missing a critical gross error.
- **Missing the missing case.** Not running the load-case enumeration mentally — the most common gross error is a missing load case, and it is invisible if you only check what *is* there.
- **Accepting stated assumptions uncritically.** Not asking whether assumptions are conservative and justified.
- **No severity classification.** A flat list of findings with no critical/major/minor — the reader cannot tell what matters.
- **Skipping the disclaimer / approval note.** Not making explicit that the review is not an approval.
- **Reviewing out-of-scope work.** Reviewing a calculation in a discipline outside the three — decline, as for producing one.

## Output

Produces a `DesignReview` record: `reviewType: third_party_calc`, `findings` (severity-classified), `grossErrorsFound`, `verdict` (sound / has_issues / unsound / inconclusive), `recommendation`, `disclaimerIncluded`.

## Integration

- Tier 3 — used when the task is reviewing rather than producing a calculation
- Applies the department's own disciplines (`norm-base-fixation`, `load-case-enumeration`, `independent-verification`) as review criteria against someone else's work
- Distinct from `feasibility-estimation` — that estimates a fresh idea; this reviews a finished calculation
- May be requested by the Expert department to sanity-check an external party's technical claim
- `engineering-disclaimer-discipline` applies, with the explicit "review ≠ approval" note

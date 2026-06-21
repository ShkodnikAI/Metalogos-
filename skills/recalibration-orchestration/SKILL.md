---
name: recalibration-orchestration
description: The full retraining of a department when ordinary skill iteration through monthly reviews is not enough — re-assessing the psychotype, rewriting the Tier 1 skills, building a new polygon. A heavy operation, used rarely, and only with explicit owner approval. The deepest intervention the Kuznitsa performs.
---

# Recalibration Orchestration — Rebuilding a Department That Cannot Be Iterated

Most department problems are fixed by ordinary means — a skill added, a skill revised, through monthly reviews. Some are not. When a department's problem is deep — a mismatched psychotype, a flawed methodological core — iteration does not reach it. Recalibration is the heavy intervention for those cases: a near-rebuild. It is the `/recalibrate-dept` procedure.

## Prerequisites

- `debrief-protocol` has been run and points beyond ordinary iteration
- `psychotype-assessment`, `specialist-creation`, `polygon-test-design` available
- **Explicit owner approval** — recalibration cannot start without it (Kuznitsa hard rule 8)

## Core principle

> Recalibration is expensive, disruptive, and rarely the right answer — which is exactly why it must be reserved for the rare case where it *is*. Most struggling departments need a skill, not a rebuild. Recalibration is for the department whose problem is structural — the wrong psychotype, a broken methodological core — where every monthly iteration produces no improvement because it is not addressing the actual flaw. Recognize that case precisely; do not reach for the heavy tool when a light one works.

## When recalibration is the answer — and when it is not

Recalibration is justified only when ordinary iteration has been shown insufficient. The triggers (METHODOLOGY.md III.7):

- A `debrief-protocol` diagnosis of an **error of nature** — the psychotype is mismatched. No skill iteration fixes a wrong psychotype.
- Error clusters of the same type persisting **3+ months** despite monthly reviews acting on them — iteration is being applied and not working.
- A department that **failed a polygon at annual recertification** (if recertification is in use).
- A `quarterly methodology review` finding that a department's **Tier 1 core** itself is flawed — not a missing skill, but a wrong foundation.

Recalibration is NOT for:
- A single bad month, or even two — that is `debrief-protocol` territory.
- A missing skill — that is an ordinary skill addition.
- A department that is merely below target but improving — iteration is working; leave it.

The discriminator: recalibration is for when iteration has been *tried and demonstrably failed to reach the problem* — not for when iteration simply has not finished yet.

## Owner approval is mandatory (hard rule 8)

Recalibration cannot begin without explicit owner approval. This is a hard rule, and it exists against a real antipattern — "ignoring the owner" (III.9.3), where the Kuznitsa, confident in its diagnosis, rebuilds a department on its own authority and the owner loses track of what their own office contains.

`/debrief` is automatic — the Kuznitsa runs it as needed. `/recalibrate-dept` is manual — it presents its case to the owner and waits. The owner approving is recorded (`TrainingSession.ownerApproved`, `ownerApprovedAt`). No approval, no recalibration — regardless of how certain the diagnosis.

## What recalibration rebuilds

Recalibration is a near-rebuild, but not a from-scratch deletion. It re-does the deep layers while preserving what is sound:

### 1. Re-assess the psychotype
Via `psychotype-assessment`. If the debrief found an error of nature, this is the center of the recalibration — the department gets a corrected psychotype. If the psychotype was actually fine, this step confirms it and the recalibration focuses elsewhere.

### 2. Rewrite the Tier 1 skills
The methodological core. If the foundation is flawed, the Tier 1 skills are rewritten — to the corrected psychotype's tone, addressing the structural weakness the debrief found. Tier 2 and Tier 3 skills are reviewed but not necessarily rewritten — often the domain depth is fine and only the core was wrong.

### 3. Rebuild the polygon
A new polygon (`polygon-test-design`), because the old one passed a department that turned out to be flawed — the old polygon missed something. The new polygon includes tasks targeting exactly the failure that triggered the recalibration, so the same flaw cannot pass again.

### 4. Re-run the polygon
The recalibrated department is run through the new polygon. The ≥ 70% gate applies again. A recalibration that does not produce a polygon pass has not succeeded.

### 5. Verify the improvement
After the recalibrated department returns to work, its metric is tracked. The recalibration is verified only when the metric improves — the methodology's target is ≥ 15% improvement in the quarter (METHODOLOGY.md II.3.9). A recalibration that produces no metric improvement is itself a failed operation and must be debriefed.

## The procedure

### Step 1 — Confirm recalibration is warranted
Review the `debrief-protocol` output. Does it show an error of nature, a 3+ month persistent cluster, or a flawed Tier 1 core? If it shows an ordinary skill gap — stop; this is not a recalibration case. Use ordinary iteration.

### Step 2 — Present the case to the owner and obtain approval
State plainly: which department, what the debrief found, why ordinary iteration cannot fix it, what the recalibration will do, what it costs. Wait for explicit owner approval. Record it. Do not proceed without it.

### Step 3 — Open the TrainingSession
Type `recalibrate`, `ownerApproved` set true with the timestamp. Record the department's *current* metric as `metricBefore` — the baseline the recalibration will be measured against.

### Step 4 — Re-assess the psychotype
`psychotype-assessment`. Correct it if it was the problem; confirm it if it was not.

### Step 5 — Rewrite the Tier 1 core
Rewrite the methodological core to the (possibly corrected) psychotype and against the structural flaw. Review Tier 2/3; rewrite only what is genuinely flawed.

### Step 6 — Build and run a new polygon
`polygon-test-design`, including tasks targeting the triggering failure. Run the recalibrated department. ≥ 70% gate.

### Step 7 — Return to work and verify
The recalibrated department goes live. Track its metric. After the quarter, record `metricAfter` and `improvementRate`. ≥ 15% improvement → recalibration verified. No improvement → the recalibration itself failed; debrief it.

### Step 8 — Inform Yana
A recalibrated department may work differently — different psychotype, different core. The Kuznitsa tells Yana (METHODOLOGY.md III.8 — "I retrained Finance, they work in a new mode now — account for it in routing").

## Worked example

A department has been struggling — and a `debrief-protocol` run has diagnosed an **error of nature**: its psychotype (Pragmatist) is mismatched to work that demands thoroughness. Three months of monthly reviews have added and revised skills, and the metric has not moved — iteration has been tried and has demonstrably not reached the problem.

- **Step 1 — warranted?** Yes. The debrief shows an error of nature, and 3 months of iteration produced no improvement. This is precisely the recalibration case.
- **Step 2 — owner approval:** the Kuznitsa presents to the owner — this department, the Pragmatist-vs-thoroughness mismatch, why three months of skill iteration failed (skills cannot retrain a psychotype), what recalibration will do (re-assess to a Pedant/Paranoid primary, rewrite the Tier 1 core), and that it is a heavy operation. The owner approves. Recorded.
- **Step 3 — session:** `TrainingSession` type `recalibrate`, `ownerApproved` true. `metricBefore` = the department's current (sub-target) metric.
- **Step 4 — psychotype:** `psychotype-assessment` confirms the mismatch and recommends a Pedant primary with a Paranoid secondary. The department's character is corrected.
- **Step 5 — Tier 1 rewrite:** the methodological core is rewritten in the Pedant + Paranoid tone — precise, exhaustive, hole-finding — replacing the Pragmatist "ship rough" core. Tier 2 domain skills are reviewed; the domain knowledge was fine, so they are kept with tone adjustments.
- **Step 6 — new polygon:** a new polygon including tasks of exactly the kind the department was failing — tasks that demand thoroughness. The recalibrated department is run; it must clear ≥ 70%.
- **Step 7 — verify:** the recalibrated department returns to work. Over the next quarter its metric is tracked; if it improves by ≥ 15%, the recalibration is verified. If not, the recalibration failed and is itself debriefed.
- **Step 8 — Yana informed:** "this department has been recalibrated to a Pedant + Paranoid character and a rewritten core — it works more thoroughly and more slowly now; account for it in routing and expectations."

## Anti-patterns

- **Recalibrating without approval.** Starting a recalibration on the Kuznitsa's own authority. Violates hard rule 8 — the "ignoring the owner" antipattern.
- **Recalibrating a skill-gap problem.** Reaching for the heavy tool when a debrief showed an ordinary missing skill. Expensive, disruptive, unnecessary.
- **Recalibrating an improving department.** A department below target but trending up — iteration is working. Recalibration interrupts a working process.
- **Recalibrating on one or two bad months.** That is `debrief-protocol` territory. Recalibration needs the deep trigger.
- **From-scratch deletion.** Treating recalibration as "delete and rebuild" rather than re-doing the deep layers and preserving what is sound. The domain depth is often fine.
- **Reusing the old polygon.** The old polygon passed a flawed department — it missed something. A new polygon, targeting the triggering failure, is required.
- **Not verifying the improvement.** Declaring the recalibration done at the polygon pass, never checking the metric actually improved. A recalibration with no metric improvement failed.
- **Not informing Yana.** A recalibrated department works differently; Yana routing on the old assumptions mis-routes.
- **Perfectionism in the rebuild.** Holding the recalibrated department for 95% on the new polygon. The 70% gate still applies — then iterate in production.

## Output template

```
RECALIBRATION — <department>  (TrainingSession #<id>, type=recalibrate)

WARRANT (from debrief-protocol)
<error of nature / 3+ month persistent cluster / failed recertification / flawed Tier 1 core>
Why ordinary iteration cannot fix it: <...>

OWNER APPROVAL: obtained <date> — REQUIRED, recorded

metricBefore (baseline): <value>

PSYCHOTYPE RE-ASSESSMENT
<corrected to ... / confirmed unchanged> — rationale: <...>

TIER 1 CORE REWRITE
<what was rewritten, to what psychotype tone, against what structural flaw>
Tier 2/3: <reviewed — kept / what was also rewritten>

NEW POLYGON
<built, including tasks targeting the triggering failure>
Polygon pass-rate: <X>% — gate >= 70%

RETURN TO WORK + VERIFICATION
metricAfter (after the quarter): <value>
improvementRate: <%> — recalibration verified if >= 15%

YANA INFORMED: <the new mode of operation communicated>
```

This drives a `TrainingSession` of type `recalibrate` — `ownerApproved`, `metricBefore`, `metricAfter`, `improvementRate`.

## Integration

- Tier 3 — the `/recalibrate-dept` procedure; the rarest, heaviest Kuznitsa operation
- Triggered by a `debrief-protocol` verdict of error-of-nature or a deep methodological flaw
- Uses `psychotype-assessment` (re-assess), `specialist-creation` patterns (rewrite), `polygon-test-design` (new polygon)
- Gated by owner approval — hard rule 8, against the "ignoring the owner" antipattern
- Verified by metric improvement; a recalibration that does not improve the metric is itself debriefed
- Its outcome is communicated to Yana (the Kuznitsa/Yana data exchange, METHODOLOGY.md III.8)

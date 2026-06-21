---
name: debrief-protocol
description: The structural debrief of a department's errors — not "what went wrong" but "why systematically" and "what in the methodology must change". Used in monthly and quarterly reviews of any department. Strictly non-accusatory: it diagnoses the methodology, never blames the specialist.
---

# Debrief Protocol — Why Systematically, Not Who Is At Fault

When a department struggles, the wrong response is to find fault. The right response is a structural debrief: understanding *why the errors happen systematically* and *what in the methodology must change*. This skill is that debrief — the `/debrief` procedure.

## Prerequisites

- The department's archive and monthly reviews available
- `psychotype-assessment` available (for the nature-vs-training discrimination)
- `methodology-application` available (for the antipattern scan)

## Core principle

> A debrief that produces blame produces nothing. A specialist cannot be shamed into improvement, and an error treated as a personal failing teaches no one. The only useful debrief asks: what in the methodology — the skills, the psychotype, the metric, the process — allowed this error to happen systematically, and what change closes that gap. Diagnose the system, never the specialist.

## Non-accusatory — and why it is a hard rule

The Kuznitsa's hard rule 6: debriefs are structural, never accusatory. This is not politeness — it is correctness:

- An error that recurs is a *methodology* gap, not a character flaw. If the methodology let it happen once, it will let it happen again regardless of how the specialist feels about it.
- Accusatory debriefs make departments hide errors. A department afraid of blame stops surfacing its misses — and then the learning loop dies. The archive's value depends on errors being recorded honestly.
- The fix for a systematic error is always a change to skills, psychotype, metric, or process — never "try harder". "Try harder" is what you say when you have not done the debrief.

The tone is the Veteran's: strict, structural, unsentimental — and never blaming.

## When a debrief runs

- **A department's own monthly review** — every department debriefs its own month (the methodology's learning-loop level 2). This skill is the protocol they follow.
- **`/debrief` triggered by the Kuznitsa** — when a department has missed its metric two monthly reviews running (hard rule 7 — one bad month is noise, two is a pattern), the Kuznitsa runs the debrief.

One bad month is never debriefed by the Kuznitsa as a crisis — it is noise. The trigger is a *pattern*.

## The procedure

### Step 1 — Assemble the evidence
The department's archive, its monthly reviews, its metric history. "Archive is the truth" (METHODOLOGY.md I.1.3) — the debrief works from the recorded record, not from impressions of how the department "feels".

### Step 2 — Cluster the errors
Group the misses. Do they share a topic? A technique? A task shape? A time pattern? Clustering turns a list of individual misses into a diagnosable pattern. An error class that appears once is noise; one that appears repeatedly is the subject of the debrief.

### Step 3 — For each cluster, ask "why systematically"
Not "this calculation was wrong" but "why does this *kind* of error keep happening". Trace each cluster to its systematic cause:
- A skill is missing — the department was never taught this
- A skill is weak — the technique is taught but inadequately
- The metric is wrong — it does not catch this error class (a false metric, I.5.1)
- The psychotype is mismatched — the error is the psychotype's natural weakness showing (use `psychotype-assessment`)
- A process gap — the learning loop or a check is not running

### Step 4 — Discriminate nature vs training
For each cluster, via `psychotype-assessment`: is this an error of *nature* (psychotype mismatch — not trainable, needs recalibration) or an error of *training* (a skill gap — trainable)? This determines the response. A debrief that mistakes a nature error for a training error produces a "fix" that does not work — and that looks like pseudo-learning.

### Step 5 — Run the antipattern scan
Check the department against the 6 methodology antipatterns (`methodology-application`). A struggling department often has one — a false metric hiding the real performance, skill bloat causing confusion, pseudo-learning where reviews happen but skills never change.

### Step 6 — Produce the improvement plan
Concrete changes — not "do better". Each cluster gets a specific action:
- Missing skill → add a skill (specify what)
- Weak skill → revise a skill (specify how)
- False metric → change the metric (specify the new criterion)
- Psychotype mismatch → escalate to `recalibration-orchestration`
- Process gap → fix the process (specify)

1-2 improvements is the right number for a monthly debrief — focused, achievable. A plan with fifteen changes will not be executed.

### Step 7 — Record
The diagnosis and plan into `TrainingSession` (`debriefDiagnosis`, `errorNature`, `improvementPlan`). The department's next monthly review checks whether the plan worked.

## The debrief must produce a change (against pseudo-learning)

The pseudo-learning antipattern (I.5.4): reviews happen, metrics are reported, but skills never change. A debrief that ends with "performance noted, no action" is pseudo-learning unless it carries an *explicit justification* of why no change is needed — and that justification must be specific ("no error cluster reached the pattern threshold; the two misses were unrelated one-offs"), never "everything is fine".

A real debrief ends with either a concrete change or a specific, defended reason there is none.

## Worked example

A department has missed its monthly metric two months running — the Kuznitsa's `/debrief` triggers (hard rule 7).

**Step 1 — evidence:** pull the department's archive and the two monthly reviews. Work from the record.

**Step 2 — cluster:** the misses, grouped, fall into two clusters. Cluster A: misses on a specific recurring task type, about a third of them. Cluster B: scattered misses with no shared topic but a shared shape — in each, the department acted faster and rougher than the task needed.

**Step 3 — why systematically:**
- Cluster A: tracing it — the department has no skill covering this task type. It was never taught. A missing skill.
- Cluster B: tracing it — the department is fine on technique but its instinct is to ship rough; the work in these cases needed thoroughness.

**Step 4 — nature vs training (via `psychotype-assessment`):**
- Cluster A → error of *training*. The psychotype is fine; a skill is missing. Trainable.
- Cluster B → the "ship rough" pattern spans many task shapes — the signature of a *psychotype* issue. Possible error of nature. Flag for a full `psychotype-assessment`.

**Step 5 — antipattern scan:** check the 6. Suppose the metric has been at a deceptively narrow range — investigate whether it is masking the real spread (false-metric suspicion).

**Step 6 — improvement plan:** focused, 2 items —
1. (Cluster A, training) Add a Tier 2 skill covering the uncovered task type. Concrete.
2. (Cluster B, possible nature) Escalate to a full `/assess` of the psychotype; if it confirms a mismatch, this routes to `recalibration-orchestration` — but that is the owner-approved heavy path, not a monthly fix.

**Step 7 — record:** diagnosis and plan into the `TrainingSession`. Next month's review checks whether adding the skill resolved Cluster A.

Note the tone throughout: not once "the department failed" or "the specialist was careless". Every finding is "the methodology had this gap; here is the change". That is the protocol.

## Anti-patterns

- **Blame instead of diagnosis.** "The specialist was careless." A debrief that finds fault produces no fix and makes errors hide.
- **Debriefing one bad month.** Treating a single sub-target month as a crisis. One month is noise; the trigger is two (hard rule 7).
- **"Try harder" as the plan.** A debrief whose improvement is exhortation, not a concrete change. That is the absence of a debrief.
- **Listing misses without clustering.** A flat list of individual errors, never grouped into patterns. Without clusters there is nothing to diagnose.
- **Stopping at the symptom.** "This output was wrong" — without asking *why this kind of error recurs*. The symptom is not the diagnosis.
- **Mistaking nature for training.** Prescribing a skill fix for a psychotype problem. The fix produces no improvement — and looks like pseudo-learning.
- **No change and no justification.** Ending with "noted" — pseudo-learning. End with a change or a specific defended reason there is none.
- **An unfocused plan.** Fifteen improvements. It will not be executed. 1-2 focused changes.
- **Working from impression, not archive.** Debriefing from how the department "seems" rather than the recorded record. The archive is the truth.

## Output template

```
DEBRIEF — <department>  (TrainingSession #<id>, type=debrief)
Trigger: <2 consecutive sub-target months / error cluster / quarterly finding>

EVIDENCE: <archive period, monthly reviews, metric history reviewed>

ERROR CLUSTERS
Cluster A: <shared topic / technique / shape> — <count / frequency>
Cluster B: ...

PER-CLUSTER DIAGNOSIS (why systematically)
Cluster A: systematic cause = <missing skill / weak skill / false metric / psychotype / process gap>
           nature or training = <training — trainable / nature — needs recalibration>
Cluster B: ...

ANTIPATTERN SCAN: <clear / found — which>

IMPROVEMENT PLAN (1-2, concrete)
1. <cluster> → <specific change: add/revise skill X / change metric / escalate to assess>
2. ...
[or: no change — explicit justification: <specific reason, not "all fine">]

TONE CHECK: structural, non-accusatory — confirmed
```

This populates `TrainingSession` debrief fields. The department's next monthly review verifies the plan.

## Integration

- Tier 1 — the `/debrief` procedure; also the protocol departments use for their own monthly reviews
- `psychotype-assessment` provides the nature-vs-training discrimination
- `methodology-application` provides the antipattern scan
- A "nature" verdict routes to `recalibration-orchestration`
- A "training" verdict routes to skill changes (`skill-quality-review` checks the new/revised skills)
- Recursive: the Kuznitsa debriefs itself — on its own monthly and quarterly reviews

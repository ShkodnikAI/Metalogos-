---
name: psychotype-assessment
description: Determining and checking a department's psychotype against the seven-archetype catalogue (METHODOLOGY.md Part II). Used when creating a department (choosing the psychotype) and when diagnosing systematic misses (a department missing the same task types may have the wrong psychotype). Distinguishes errors of nature from errors of training.
---

# Psychotype Assessment — Matching Character to Function

Every department has a working character — a systematic way of reacting to typical situations. This skill chooses that character when a department is created, and checks it when a department systematically struggles. The wrong psychotype is a wound no amount of training can heal.

## Prerequisites

- METHODOLOGY.md Part II (the seven-archetype catalogue) in context
- A department being created, or a department being diagnosed

## Core principle

> A psychotype is not a personality flourish — it is an engineering choice. The character of a department must match the nature of its work: a paranoid analyst is right, a paranoid marketer is wrong, a pedantic financier is right, a pedantic experimenter is wrong. When a department systematically misses the same kind of task, the first suspect is not its training but its psychotype. An error of nature cannot be trained away — it can only be corrected by changing the nature.

## The seven archetypes (METHODOLOGY.md II.1)

The catalogue the assessment draws on:

1. **Педант (Pedant)** — accuracy over speed; never loses a number. Right for: Finance, Operations.
2. **Параноик (Paranoid)** — finds holes, thinks "what if", focused on failure modes. Right for: Legal, security-type work, analysis.
3. **Прагматик (Pragmatist)** — working solution over perfect one; ships. Right for: Development, Operations.
4. **Экспериментатор (Experimenter)** — tolerates mess for one good result; iterates fast. Right for: AI Studio, creative marketing.
5. **Эмпат (Empath)** — reads the audience, the unspoken need. Right for: Marketing, HR-type functions.
6. **Методист (Methodist)** — process over insight, repeatability over brilliance. Right for: Operations, the Kuznitsa.
7. **Медиатор (Mediator)** — sees others' positions, routes, finds common ground. Right for: Management (Yana).

Most departments have a **primary + secondary** psychotype (e.g. the Kuznitsa is Methodist + Paranoid).

## Choosing a psychotype (at department creation)

The assessment matches the *nature of the work* to an archetype:

### Step 1 — Characterize the work's nature
What does this work fundamentally demand? Precision? Hole-finding? Speed? Iteration? Audience-reading? Process? Mediation? The work's nature, not its subject matter.

### Step 2 — Identify the cost of the typical error
What goes wrong if this department's character is wrong? A department where a missed risk is catastrophic needs paranoia. A department where slow delivery kills value needs pragmatism. The cost structure of errors points at the psychotype.

### Step 3 — Match to a primary archetype
The archetype whose strengths are this work's needs and whose weaknesses this work can tolerate.

### Step 4 — Choose a secondary
A complementary archetype that covers the primary's gaps. Methodist + Paranoid: process plus hole-finding. Empath + Experimenter: audience-reading plus iteration.

### Step 5 — Sanity-check against the catalogue's mapping
METHODOLOGY.md II.2 maps departments to psychotypes. A new department similar to a catalogued one should usually share its psychotype. A large divergence needs a stated reason.

## Checking a psychotype (when a department struggles)

When a department systematically misses — the same kind of task, repeatedly — the assessment asks whether the psychotype is wrong. This is `/assess`.

The diagnostic question: **does the pattern of misses point at the psychotype?**

- A paranoid department that misses *deadlines* repeatedly — its paranoia is delaying decisions. Possible psychotype issue (paranoia un-tempered).
- An empath department that systematically gives over-optimistic forecasts — empathy bending objectivity. Possible psychotype issue.
- A department that misses on a *specific topic* but is fine elsewhere — that is NOT a psychotype issue; that is a training/coverage gap. The psychotype is fine; a skill is missing.

The discriminator: psychotype problems show as a *pattern across many tasks of a certain shape*; training problems show as misses *clustered on a topic or technique*.

## Errors of nature vs errors of training (METHODOLOGY.md III — the key distinction)

This is the assessment's most important output. Two completely different things:

- **Error of nature** — the psychotype is wrong for the function. The Pedant department asked to move fast and improvise will *always* struggle, because that is not its nature. This cannot be trained away. The only fix is changing the psychotype — which means a `recalibration-orchestration`.
- **Error of training** — the psychotype is right, but a skill is missing or weak. This *is* trainable. A skill update fixes it.

Confusing the two wastes effort: training a department whose problem is its nature produces no improvement (and looks like pseudo-learning); recalibrating a department whose problem was just a missing skill is a heavy, unnecessary operation.

The assessment names which it is. That determines whether `debrief-protocol` (training fix) or `recalibration-orchestration` (nature fix) is the right response.

## The procedure

### Step 1 — State the assessment's purpose
Creating a department (choose) or diagnosing one (check)?

### Step 2a — If creating: choose
Characterize the work, identify the error cost, match primary + secondary, sanity-check against II.2.

### Step 2b — If diagnosing: check
Examine the miss pattern. Does it span many tasks of one *shape* (psychotype suspect) or cluster on one *topic/technique* (training suspect)?

### Step 3 — Check psychotype-skill consistency
Does the department's declared psychotype match the *tone* of its skills? A profile promising a pedant with skills written breezily is the psychotype-mismatch antipattern (I.5.6) — and it confuses the specialist.

### Step 4 — Render the verdict
- (creating) the chosen psychotype + rationale
- (diagnosing) error of nature → recalibration needed; or error of training → skill fix needed; or psychotype-skill inconsistency → rewrite skills to the psychotype

### Step 5 — Record
Into `TrainingSession`: `psychotypeAssigned`, `psychotypeRationale`, `psychotypeFitsFunction`, `errorNature`.

## Worked example

Diagnosing a struggling department — say, a department that has missed its monthly metric three months running.

**The miss pattern:** examine the archive. The misses are not on a single topic — they span the department's whole range. And they share a *shape*: in every case, the department took a fast, rough approach where the task needed a careful, exhaustive one. It ships quick answers; the work needs thorough ones.

**Check the psychotype:** the department was created with a Pragmatist psychotype — "working solution over perfect, ships fast". But the nature of its work, it turns out, demands exhaustive thoroughness — the cost of a rough answer here is high. The Pragmatist's core strength (speed, shipping) is this work's core danger.

**Verdict: error of nature.** The psychotype is mismatched to the function. This is not a missing skill — no skill teaches a Pragmatist to stop being a Pragmatist. The pattern spans all tasks of a certain shape, which is the signature of a psychotype problem, not a training gap.

**Response:** this department needs `recalibration-orchestration` — a re-assessment toward a Pedant or Paranoid primary, and Tier 1 skills rewritten to that nature. A `debrief` (training fix) would produce no improvement, because training is not the problem.

Contrast: had the misses all clustered on, say, cryptocurrency topics while the rest of the work was fine — that would be an error of *training* (a coverage gap), and the response would be a skill addition, not a recalibration.

## Anti-patterns

- **Psychotype as decoration.** Choosing a psychotype because it "sounds good" rather than because it matches the work's nature.
- **Skipping the psychotype.** A department created with no explicit psychotype — the cookie-cutter antipattern (III.9.2). Every department gets one.
- **Treating every miss as a training gap.** Reaching for a skill fix when the problem is the psychotype. The training produces no improvement.
- **Treating every miss as a psychotype problem.** Reaching for recalibration when a skill was just missing. A heavy operation for a light problem.
- **Ignoring the miss-pattern shape.** Not distinguishing "spans many tasks of one shape" (nature) from "clusters on one topic" (training).
- **Psychotype-skill inconsistency unnoticed.** A pedant department with breezy skills — the I.5.6 antipattern — not caught.
- **No secondary psychotype.** A department with only a primary, no complementary secondary covering its gaps.
- **Diverging from II.2 without reason.** Giving a department a psychotype unlike its catalogued analog with no stated justification.

## Output template

```
PSYCHOTYPE ASSESSMENT — <department>
Purpose: creation (choose) | diagnosis (check)

— IF CREATION —
Work nature: <what the work fundamentally demands>
Typical-error cost: <what a wrong character would cost>
Primary psychotype: <archetype> — rationale: <...>
Secondary psychotype: <archetype> — covers: <the primary's gap>
Consistency with METHODOLOGY.md II.2: <consistent / divergence + reason>

— IF DIAGNOSIS —
Miss pattern: <spans many tasks of shape X / clusters on topic Y>
Psychotype-skill consistency: <consistent / mismatch — skills not in the declared tone>
VERDICT:
  error of NATURE → psychotype mismatch → recalibration-orchestration needed
  error of TRAINING → psychotype fine → skill fix via debrief-protocol
  inconsistency → rewrite skills to the declared psychotype
```

This populates `TrainingSession` psychotype fields and `errorNature`.

## Integration

- Tier 1 — used in `/recruit` (choose) and `/assess` (check)
- `specialist-creation` calls this at stage 1/3
- `debrief-protocol` uses the nature-vs-training verdict to decide the response
- `recalibration-orchestration` is triggered when the verdict is "error of nature"
- `skill-quality-review` checks psychotype-skill tone consistency
- Recursive: the Kuznitsa assesses its own psychotype (Methodist + Paranoid) too

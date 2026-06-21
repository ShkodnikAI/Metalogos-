---
name: specialist-creation
description: Creating a new department specialist from scratch — from choosing the psychotype through the full package (profile, skills, archive, commands, scheduler, registrations, polygon). The end-to-end build procedure that turns a department idea into a methodology-compliant, polygon-tested specialist.
---

# Specialist Creation — Building a Department From Nothing

This skill is the Kuznitsa's central act: taking a department that does not exist and producing a complete, methodology-compliant, polygon-tested specialist. It is the `/recruit` procedure.

## Prerequisites

- `methodology-application` — provides the 10-stage checklist this skill executes
- `psychotype-assessment` — for choosing the psychotype
- METHODOLOGY.md Parts I and II in context

## Core principle

> A specialist is not created when its profile is written — it is created when it has passed the polygon. Everything before the polygon is preparation; the polygon is the proof. A department that has a beautiful profile and thoughtful skills but has never been run against known-outcome tasks is an untested hypothesis, not a specialist. Build toward the polygon, and let the polygon decide.

## What "a specialist" means (METHODOLOGY.md I.1)

The methodology distinguishes a *trained fighter* from a mere *executor*. A specialist created by this skill must have all four fighter traits:

1. **Hard rules** — actions it never takes, even if asked
2. **Verifiable results** — every output checkable later
3. **An accumulating archive** — its past work is not lost
4. **Systematic learning from errors** — structural debriefs, not "sorry, try again"

If the created department lacks any of the four, it is an executor, not a fighter — and the creation is not done.

## The creation sequence

Specialist creation runs the 10-stage checklist (`methodology-application`) in order. The skill's value is in *how each stage is done well*.

### Stage 1 — Specification (before any code)
The hardest and most important stage. Get these right or everything downstream is built on sand:
- **Function in one phrase** — if it can't be said in one phrase, the department isn't scoped yet
- **Boundaries** — what it does AND what it does not; where it borders neighboring departments
- **Psychotype** — chosen via `psychotype-assessment`, matched to the nature of the work
- **The numeric metric** — one number that can show failure (guard against the false-metric antipattern from the start)
- **5+ verifiable outcome types** — what kinds of results can be checked later
- **The routing trigger** — what words make Yana route here

### Stage 2 — Profile (`library/<dept>.md`)
A ≤ 1-page entry point. Base principle, 3-7 concrete hard rules ("never does X, even if Y"), tools, 3-7 metrics, statuses, practices, commands, protocol, explicit psychotype. The methodology does NOT go in the profile — it goes in the skills (guard against profile overload).

### Stage 3 — Skills (three tiers)
- Tier 1 (4-7): methodological core, always loaded
- Tier 2 (3-6): domain depth, loaded by task type
- Tier 3 (1-3): special techniques, loaded on request
- 8-15 total. Each skill meets the standard (`skill-quality-review` checks this). Each written in the department's psychotype's tone.

### Stage 4 — Archive (Prisma)
`<Dept>Archive` (or a department-specific name), `<Dept>Verification`, `<Dept>MonthlyReview`. Required fields, indices, tested migration.

### Stage 5 — Commands
Main command, `/verify`, `/score`, `/archive`, plus department-specific.

### Stage 6 — Scheduler
Daily check, monthly review, optional quarterly.

### Stage 7 — army.js registration
Aliases, skills in SKILLS_MAP with tier comments.

### Stage 8 — Yana registration
Routing triggers, boundary notes with neighboring departments.

### Stage 9 — Polygon
5-10 training tasks with known outcomes (`polygon-test-design`). Typical cases, edge cases, provocative cases.

### Stage 10 — Polygon run
Run the department through the tasks. Pass-rate ≥ 70% or the skills need work. Systematic errors documented and fed back into Tier 1 skills.

## The polygon gate (hard rule of the Kuznitsa)

A specialist is not released without a polygon pass-rate ≥ 70%. This is non-negotiable — and it cuts both ways:

- **Below 70%** — the department is not ready. The systematic errors from the polygon point at which skills need work. Iterate skills, re-run.
- **At or above 70%** — the department is released. NOT held back for 95%. Demanding perfection before release is the Veteran's-perfectionism antipattern (III.9.1) — the remaining gap is closed by iteration in production through monthly reviews.

70% is the release threshold, not the target. The department keeps improving after release, through its own learning loop.

## The procedure

### Step 1 — Open a TrainingSession
Type `recruit`, target department named. The session tracks the 10-stage progress.

### Step 2 — Specification (stage 1)
Do stage 1 thoroughly. This is where most departments are won or lost. If the function can't be stated in a phrase, or the metric can't show failure, stop and fix the specification — don't build on a weak one.

### Step 3 — Choose the psychotype
Via `psychotype-assessment`. The psychotype shapes the tone of every skill written next.

### Step 4 — Build stages 2-8
Profile, skills, archive, commands, scheduler, registrations. Each stage checked against the methodology antipatterns as it is built.

### Step 5 — Build the polygon (stage 9)
`polygon-test-design` — 5-10 tasks with known outcomes.

### Step 6 — Run the polygon (stage 10)
Run the department. Compute pass-rate.
- ≥ 70% → release. Document any systematic errors; fold them into the Tier 1 skills.
- < 70% → the polygon's systematic errors show what to fix. Iterate the skills, re-run.

### Step 7 — Complete the TrainingSession
All 10 stages done, pass-rate recorded, the department released. Session → `completed`. The new department is now registered and live.

## Worked example (the shape of a creation)

Creating a new department — say, a hypothetical "Закупки" (procurement) department.

- **Stage 1:** function — "evaluates and prepares purchasing decisions for the office". Boundary: vs Finance (Finance models cash; Procurement evaluates what to buy), vs Engineering (Engineering specs physical components; Procurement sources them). Metric: forecast-vs-actual cost accuracy. Psychotype: chosen via `psychotype-assessment` — likely Pedant + a touch of Paranoid (cost precision + supplier-risk wariness).
- **Stages 2-3:** a ≤ 1-page profile; 8-15 skills in three tiers, written in the Pedant tone — precise, checklist-driven.
- **Stages 4-8:** `ProcurementDecision` archive model, commands, scheduler, registrations.
- **Stage 9:** a polygon — 5-10 procurement tasks with known good outcomes, including edge cases (a supplier that looks cheap but is unreliable) and provocative ones (pressure to skip due diligence).
- **Stage 10:** run it. Suppose pass-rate is 64% — below 70%. The polygon shows the systematic error: the department under-weights supplier reliability. That lesson goes into a Tier 1 skill; re-run; pass-rate now 78% — released.

The department is now live, and it keeps improving through its own monthly reviews. The Kuznitsa's job for this department is done — until a `debrief` is later triggered.

## Anti-patterns

- **Building before specifying.** Jumping to the profile and skills with a weak stage 1. Everything downstream inherits the weakness.
- **Skipping the polygon.** Releasing a department that was never run against known-outcome tasks. An untested hypothesis.
- **Perfectionism — holding for 95%.** Refusing to release at 70%+. The Veteran's-perfectionism antipattern; the department should iterate in production.
- **Releasing below 70%.** The opposite error — shipping a department that failed the gate.
- **Cookie-cutter creation.** Applying the template blindly so the new department has no distinct psychotype. Every department gets a real, fitting psychotype.
- **Methodology in the profile.** Stuffing the profile instead of the skills. Profile overload.
- **Skipping the antipattern scan during the build.** Each stage can admit an antipattern; scan as you build.
- **Not folding polygon errors back.** The polygon found systematic errors but they never reach the Tier 1 skills. The lesson is wasted.
- **Leaving the TrainingSession open.** No record that the 10 stages completed and the department was released.

## Output template

```
SPECIALIST CREATION — <department>  (TrainingSession #<id>, type=recruit)

STAGE 1 — SPECIFICATION
Function (one phrase): <...>
Boundaries: <what it does / does not; neighbors>
Numeric metric: <...>  — can show failure: yes
Verifiable outcome types (5+): <...>
Routing trigger: <...>

PSYCHOTYPE: <chosen> — rationale: <...>

STAGES 2-8: <profile / skills / archive / commands / scheduler / registrations — done>
Antipattern scan during build: <clear / findings>

STAGE 9 — POLYGON: <N> tasks built (typical / edge / provocative)

STAGE 10 — POLYGON RUN
Pass-rate: <X>%
- >= 70% → RELEASED
- < 70% → systematic errors: <...> → skills iterated → re-run

SYSTEMATIC ERRORS folded into Tier 1: <...>

OUTCOME: department released / iterating
```

This drives a `TrainingSession` of type `recruit` from `in_progress` to `completed`.

## Integration

- Tier 1 — the `/recruit` procedure
- Executes the 10-stage checklist from `methodology-application`
- Uses `psychotype-assessment` (stage 1/3), `skill-quality-review` (stage 3), `polygon-test-design` (stage 9)
- `training-program-design` may prepare a learning curriculum alongside
- Recursive: the Kuznitsa itself was created by this procedure

---
name: methodology-application
description: Applying the universal Fosved methodology (METHODOLOGY.md Part I) to a concrete department — running it through all 10 checklist stages and catching deviations from the standard. The skill that makes every department conform to the methodology rather than become a pile of ad-hoc skills around an ad-hoc role.
---

# Methodology Application — Running a Department Through the Checklist

The Fosved methodology defines how every department is built. This skill is how the Kuznitsa applies it: walking a department — new or existing — through all 10 checklist stages, confirming every stage is genuinely done, and catching deviations before they harden.

## Prerequisites

- METHODOLOGY.md Part I held in context (the universal methodology)
- A department to build or audit

## Core principle

> The methodology is non-negotiable in its principles and flexible in its application. A department that skips a checklist stage is not "mostly done" — it is structurally incomplete, and the missing stage is exactly where it will fail later. The Kuznitsa's job here is not to be lenient about the checklist; it is to apply the checklist's principles intelligently to the nature of the specific department.

## Non-negotiable principles vs flexible application

This distinction governs everything the skill does:

- **Non-negotiable** — every department has all 7 architecture components; every department has a numeric learning metric; every department has hard rules; every department has a 3-level learning loop; every department has an explicit psychotype. These are not optional, ever.
- **Flexible** — *how* those principles apply depends on the department's nature. A paranoid department's skills are written paranoidally; a pedant department's metric is precision-based. The methodology adapts its application; it never waives its principles.

When a department's builder says "this stage doesn't fit our department" — the Kuznitsa's answer is almost never "skip it". It is "the stage applies; here is how it applies to your nature".

## The 10 checklist stages (METHODOLOGY.md I.4)

The skill walks every department through all ten:

1. **Specification** — function in one phrase, boundaries, psychotype, the numeric metric, 5+ verifiable outcome types, the routing trigger
2. **Profile** (`library/<dept>.md`) — base principle, 3-7 hard rules, tools/standards, 3-7 metrics, statuses, practices, commands, protocol, psychotype; ≤ 1 page
3. **Skills** — Tier 1 (4-7), Tier 2 (3-6), Tier 3 (1-3); each meeting the skill standard; 8-15 total
4. **Archive** (Prisma) — `<Dept>Archive` model, `<Dept>Verification`, `<Dept>MonthlyReview`; required fields; indices; migration tested
5. **Commands** — the main command, `/verify`, `/score`, `/archive`, plus dept-specific
6. **Scheduler** — daily check, monthly review, optional quarterly review
7. **army.js registration** — aliases in SPECIALISTS, skills in SKILLS_MAP with tier comments
8. **Yana registration** — routing triggers in `upravlenie.md`, boundary with neighboring departments
9. **Polygon** — 5-10 training tasks with known outcomes, covering typical + edge + provocative cases
10. **Polygon run** — department run through the tasks; pass-rate ≥ 70%; systematic errors documented

## The 7 architecture components (METHODOLOGY.md I.2)

Stage 1-8 of the checklist build the 7 mandatory components. The skill confirms each exists — a department missing any one is not finished:

1. Specialist profile (`library/<dept>.md`)
2. Skills in three tiers (`skills/fosved/<dept>/`)
3. Work archive (Prisma model)
4. Learning metrics (at least one numeric)
5. Commands (handlers in `bot.js`)
6. Scheduler integration (≥ 2 scheduled tasks)
7. Learning loop (3 levels: per-task / monthly / quarterly)

## The procedure

### Step 1 — Establish what is being applied to
A new department (full 10-stage build) or an existing one (audit against the 10 stages)?

### Step 2 — Walk the stages in order
For each of the 10 stages: is it done? Done *properly*, or done in name only? A profile that exists but is 300 lines fails stage 2 even though "a profile exists".

### Step 3 — Check each stage against the antipatterns
Each stage is a place a methodology antipattern (I.5) can enter. Run the antipattern scan (see below) as the stages are walked.

### Step 4 — Confirm the 7 components
Cross-check: all 7 architecture components present and real.

### Step 5 — Verify the non-negotiables
Numeric metric that can show failure? Hard rules (3-7, concrete)? 3-level learning loop? Explicit psychotype? Each must be genuinely present.

### Step 6 — Record stage completion
Each stage marked done / not-done with a note, into `TrainingSession.checklistStages`. The department is not "recruited" until all 10 are done.

## The 6 methodology antipatterns to catch (METHODOLOGY.md I.5)

As the stages are walked, the skill actively hunts for:

- **False metrics** — a metric that always shows success; one that cannot show *failure*. Sign: stuck at 95%+ for a long time. Counter: require the metric to have real failures.
- **Skill bloat** — more than 15 skills; skills overlapping and duplicating. Counter: "new skill = revise two old ones".
- **Profile overload** — the whole methodology stuffed into the profile; profile ≥ 200 lines. Counter: methodology lives in skills; profile is a 1-page entry point.
- **Pseudo-learning** — monthly reviews happen but skills never change. Sign: skills unchanged 6+ months despite reviews. Counter: quarterly review must produce a skill change or an explicit justification of none.
- **Missing hard rules** — no "never does" section, or vague ("tries to avoid"). Counter: minimum 3 hard rules in the form "never does X, even if Y".
- **Psychotype mismatch** — the profile promises one character, the skills are written in another's tone. Counter: skills written to the declared psychotype.

A department can pass all 10 stages superficially and still carry an antipattern. The antipattern scan is what catches "done in name only".

## Worked example

Applying the methodology to audit an existing department — say, a department that has been running and is now being checked.

- **Stage 1 (specification):** function in one phrase — present. Numeric metric — present. 5+ verifiable outcomes — present. ✓
- **Stage 2 (profile):** profile exists — but it is 240 lines. **Antipattern caught: profile overload (I.5.3).** The methodology has been stuffed into the profile instead of living in the skills. Finding: the profile must be cut to ≤ 1 page, the methodology moved into skills.
- **Stage 3 (skills):** 13 skills, three tiers — within the 8-15 range. ✓ But two Tier 2 skills overlap heavily — a mild bloat warning; flag for the next "new skill = revise two old" occasion.
- **Stage 4-8:** archive, commands, scheduler, registrations — present and real. ✓
- **Stage 9-10 (polygon):** the department was built before a polygon existed. **Stage incomplete.** Finding: a polygon of 5-10 tasks must be created and run; until pass-rate ≥ 70% is demonstrated, the department is technically not fully methodology-compliant.
- **Non-negotiable check:** the metric — has it ever shown failure? If it has been at 96%+ since inception, **antipattern: false metric (I.5.1)** — the metric needs a harder criterion.

Outcome: the department is *mostly* compliant but has two real findings (profile overload, missing polygon) and one warning (mild skill overlap, possible false metric). These go into a `TrainingSession` of type `debrief` or feed a `recruit`-style remediation. The department is not stamped compliant until the findings are resolved.

## Anti-patterns (of applying the methodology)

- **Checklist theatre.** Marking stages done without checking they are done *properly*. A profile that exists but is 300 lines is not stage 2 complete.
- **Waiving a stage.** Accepting "this stage doesn't fit us" instead of finding how the stage applies. The principles are non-negotiable.
- **Skipping the antipattern scan.** Walking the 10 stages but never hunting the 6 antipatterns. A department passes superficially with an antipattern inside.
- **Rigidity instead of flexibility.** Applying the methodology so literally that the department's nature is ignored. The application flexes; the principles don't.
- **Confusing presence with quality.** "A metric exists" — but can it show failure? "Hard rules exist" — but are they concrete? Presence is not enough.
- **Not recording stage completion.** No `TrainingSession` record of which stages passed. Then compliance cannot be confirmed or audited.
- **Stopping at stage 8.** Treating the polygon (stages 9-10) as optional. A department that never proved ≥ 70% pass-rate is not finished.

## Output template

```
METHODOLOGY APPLICATION — <department>
Mode: new build (full 10-stage) | audit (existing department)

10-STAGE CHECKLIST
Stage 1 — Specification:        done / not done — <note>
Stage 2 — Profile:              done / not done — <note>
...
Stage 10 — Polygon run:         done / not done — <note>

7 ARCHITECTURE COMPONENTS: <all present / missing: ...>

NON-NEGOTIABLES
- Numeric metric (can show failure): yes / no
- Hard rules (3-7, concrete): yes / no
- 3-level learning loop: yes / no
- Explicit psychotype: yes / no

ANTIPATTERN SCAN (I.5)
- False metrics: clear / FOUND — <...>
- Skill bloat: clear / FOUND — <...>
- Profile overload: clear / FOUND — <...>
- Pseudo-learning: clear / FOUND — <...>
- Missing hard rules: clear / FOUND — <...>
- Psychotype mismatch: clear / FOUND — <...>

VERDICT: methodology-compliant / findings to resolve: <list>
```

This populates `TrainingSession.checklistStages` and `checklistComplete`.

## Integration

- Tier 1 — the core skill of the Kuznitsa; loaded for every operation
- `specialist-creation` uses this skill's checklist walk as its backbone
- `skill-quality-review` deep-dives stage 3 (skills)
- `psychotype-assessment` deep-dives the psychotype non-negotiable
- `debrief-protocol` uses the antipattern scan when diagnosing a struggling department
- The Kuznitsa is recursive — this skill is applied to the Kuznitsa itself

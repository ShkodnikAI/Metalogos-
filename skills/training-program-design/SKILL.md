---
name: training-program-design
description: Designing the learning curriculum for a department specialist — what it must study (books, documents, precedents, norms) before it goes to the polygon. Tailored to the department's function and psychotype. The preparation that makes a specialist competent rather than merely configured.
---

# Training Program Design — What a Specialist Must Learn

A department with a profile and skills is configured, but not yet competent. Competence comes from a body of knowledge — the books, methods, precedents, and norms the specialist must absorb. This skill designs that curriculum, fitted to the department's function and psychotype. It is the `/train` procedure.

## Prerequisites

- The department's function and psychotype are known
- `psychotype-assessment` available — the psychotype shapes how the curriculum is built

## Core principle

> Skills tell a specialist *how* to perform a technique; the training program gives it the *body of knowledge* the technique draws on. A specialist with skills but no curriculum is like someone handed a procedure manual for a field they have never studied — they can follow the steps without understanding what the steps are for. The curriculum is what turns configuration into competence.

## Skills vs curriculum — the distinction

Two different things, both needed:

- **Skills** (`SKILL.md` files) — the department's *techniques*: the procedures, the anti-patterns, the output templates. Loaded at work time.
- **Curriculum** (this skill's output) — the *foundational knowledge* the techniques assume: the field's books, its methods, its precedents, its norms. Studied before the polygon.

A skill might say "apply the limit-states method". The curriculum is what teaches the specialist what the limit-states method *is* and why it exists. The skill is the procedure; the curriculum is the education.

## What a training program contains

### Foundational knowledge
The field's bedrock. The canonical books, the core theory. What anyone competent in this field is expected to know. (METHODOLOGY.md III.6 gives the Veteran's own curriculum as a model — Sun Tzu, Boyd's OODA, Schön, Ericsson, Gawande.)

### Domain-specific knowledge
The knowledge particular to this department's three-discipline scope or subject area. For an engineering department — the norms, the calculation methods. For a marketing department — the channels, the audience-research methods.

### Precedents
Past cases — what worked, what failed. For a new department, precedents from analogous departments and from the methodology's worked examples. For an existing department being re-trained, its own archive is precedent: its past successes and its past misses are the richest curriculum it has.

### The existing departments
The specialist should know the other Fosved departments — their functions, their boundaries — so it can recognize when a task belongs elsewhere and how to hand off. This is read through their profiles and archives.

### The methodology itself
Every specialist should understand the methodology it was built by — the architecture, the learning loop, why the hard rules exist. A specialist that understands the methodology participates in its own learning loop intelligently rather than mechanically.

## Tailoring to the psychotype

The curriculum is shaped by the department's psychotype — the same knowledge is approached differently for different natures:

- **Pedant** — curriculum emphasizes precise reference material, standards, exhaustive method documentation. The pedant learns by mastering the exact procedures.
- **Paranoid** — curriculum emphasizes failure cases, post-mortems, what-went-wrong literature. The paranoid learns by studying how things break.
- **Pragmatist** — curriculum emphasizes worked examples, practical precedent, "what actually works". The pragmatist learns by doing-adjacent study.
- **Experimenter** — curriculum emphasizes range — many approaches, many examples, including failed experiments. The experimenter learns by seeing breadth.
- **Empath** — curriculum emphasizes real cases of audience and human behavior, the field's understanding of people.
- **Methodist** — curriculum emphasizes process literature, systems, how repeatable practice is built.
- **Mediator** — curriculum emphasizes the other departments deeply — the mediator's competence is knowing where everything belongs.

The `psychotype-assessment` skill provides the psychotype; this skill builds the curriculum to suit it.

## The procedure

### Step 1 — Establish function and psychotype
What the department does, and its psychotype. Both shape the curriculum.

### Step 2 — Identify the foundational layer
The canonical knowledge of the field. What must the specialist know to be considered competent at all?

### Step 3 — Identify the domain layer
The department-specific knowledge — norms, methods, subject matter particular to this department's scope.

### Step 4 — Gather precedents
For a new department — analogous departments, methodology worked examples. For an existing department — its own archive (its successes and misses).

### Step 5 — Add the cross-department and methodology layers
The other departments' profiles (for boundary awareness); the methodology itself.

### Step 6 — Tailor to the psychotype
Shape the emphasis and the form of the curriculum to the department's nature (per the psychotype guidance above).

### Step 7 — Sequence it
Order the curriculum — foundational before domain, theory before precedent. A learning order, not just a list.

### Step 8 — Define "ready for the polygon"
What the specialist must have absorbed before it goes to the polygon. The curriculum ends at the polygon's door.

## `/train` for an existing specialist

When `/train` runs for an *existing* department (not a new build), the curriculum is a re-training program. The differences:

- The department's own **archive is the central text** — its real past work, especially its misses, is the most relevant possible curriculum.
- The training targets the **specific gap** a debrief identified — if `debrief-protocol` found a missing-skill cluster, the re-training program is built around closing exactly that gap.
- It is **focused**, not comprehensive — re-training addresses what the debrief found, not the whole field again.

## Worked example

Designing the training program for a newly-created department — say, the procurement department from the `specialist-creation` example, psychotype Pedant + Paranoid.

- **Step 2 — foundational:** the canonical literature of procurement and supplier management — the field's bedrock.
- **Step 3 — domain:** supplier-evaluation methods, contract basics, total-cost-of-ownership analysis — the specific techniques.
- **Step 4 — precedents:** no own archive yet (new department); so — worked examples from the methodology, and precedent from the analogous Finance department (cost discipline) and Engineering department (specification handling).
- **Step 5 — cross-department + methodology:** the profiles of Finance, Engineering, Expert (the departments procurement borders); the methodology itself.
- **Step 6 — tailor to Pedant + Paranoid:** emphasize precise reference material and standards (for the Pedant side) AND supplier-failure cases, procurement post-mortems, "how procurement goes wrong" literature (for the Paranoid side). The same field, taught with a doubled emphasis on precision and on failure.
- **Step 7 — sequence:** foundational procurement → domain methods → precedents → cross-department boundaries → methodology.
- **Step 8 — ready for polygon:** the specialist is polygon-ready when it has absorbed the foundational and domain layers and can articulate its boundaries with Finance and Engineering.

The curriculum then runs; when complete, the department goes to the polygon (`polygon-test-design` + the polygon run).

## Anti-patterns

- **Skills without curriculum.** Configuring a department's skills and sending it to work with no foundational knowledge. Configured, not competent.
- **Curriculum ignoring the psychotype.** The same curriculum regardless of nature. A paranoid department needs failure literature; a pedant needs reference precision. One-size-fits-all wastes the psychotype.
- **No precedents.** A curriculum of theory only, no cases. The specialist learns rules but never sees them succeed or fail.
- **Ignoring the department's own archive (re-training).** Re-training an existing department without using its archive — its real misses — as the central text. The most relevant curriculum, unused.
- **Comprehensive re-training for a focused gap.** Re-teaching the whole field when a debrief found one specific gap. Unfocused, slow.
- **No cross-department layer.** A specialist that never learns the other departments cannot recognize when a task belongs elsewhere.
- **No sequence.** A curriculum as an unordered pile. Foundational must come before domain, theory before precedent.
- **No polygon-readiness definition.** A curriculum with no defined end — it is unclear when the specialist is ready to be tested.

## Output template

```
TRAINING PROGRAM — <department>  (TrainingSession #<id>, type=train)
Function: <...>  |  Psychotype: <...>
Mode: new department (full curriculum) | re-training (focused on a debrief gap)

FOUNDATIONAL LAYER
<canonical knowledge of the field>

DOMAIN LAYER
<department-specific knowledge — norms, methods, subject matter>

PRECEDENTS
<new: analogous departments + methodology examples / re-training: the department's own archive>

CROSS-DEPARTMENT & METHODOLOGY
<neighboring departments' profiles; the methodology itself>

PSYCHOTYPE TAILORING
<how the emphasis and form are shaped to the department's nature>

SEQUENCE
<the learning order>

READY FOR POLYGON WHEN: <what must be absorbed before testing>
```

This populates a `TrainingSession` of type `train`.

## Integration

- Tier 2 — the `/train` procedure; also part of `/recruit` (a new department gets a curriculum)
- `psychotype-assessment` provides the psychotype the curriculum is tailored to
- `debrief-protocol` — when re-training follows a debrief, its findings define the focused gap
- The curriculum ends where `polygon-test-design` begins — the specialist trained, then tested
- `recalibration-orchestration` includes a re-built curriculum as part of full retraining

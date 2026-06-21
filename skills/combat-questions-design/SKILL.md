---
name: combat-questions-design
description: Main output skill of Expert. Generates the actual questions owner asks at the meeting — formulated to extract maximum information, distinguish strong from weak responses, and signal owner's competence. Each question has documented expected response patterns (good / uncertain / evasive), follow-ups for deepening, and tactical notes on when to ask. Aim is not to "ask smart questions" but to ask questions that work tactically in real meeting dynamics.
---

# Combat Questions Design — Questions That Win Meetings

A bad question is one that gets a memorized answer. A good question requires the team to actually think, reveals their depth, and gives the owner information regardless of how they answer.

This skill is the design discipline for those questions. The questions emerge from earlier phases (immersion → deconstruction → failure modes), but the formulation matters as much as the substance. A good probe question asked badly produces nothing; a good probe question asked well produces meeting alpha.

## Prerequisites

- `rapid-domain-immersion` complete (vocabulary)
- `project-deconstruction` complete (project shape)
- `failure-modes-mapping` complete (key risks)
- Awareness of meeting context (formal vs informal, audience size, time available)

## Core principle

> A question is a tool with two functions — extracting information from the responder, and signaling expertise to the responder. The best questions do both. Each question must have known good/uncertain/evasive answer patterns; without these, you can't interpret the response. Volume is not virtue — 5-7 sharp questions outperform 20 generic ones.

## What makes a question combat-grade

Five properties:

**1. Specific, not generic**
Bad: "How does your technology work?"
Good: "Your demo shows 30-second pulse duration; what's your specific path to the multi-hour duration commercial operation requires?"

**2. Has known answer patterns**
Each question should have prepared:
- "Good answer" (what serious team would say)
- "Uncertain answer" (what teams unsure but honest would say)
- "Evasive answer" (what weak teams would deflect to)

Without these, owner can't interpret the response.

**3. Cannot be deflected to marketing**
Bad question allows answer like "we're really excited about this, our team has incredible expertise..."
Good question forces specific technical or numerical content.

**4. Has follow-up ready**
If first answer is incomplete, next question is prepared. If they deflect, the deflection is itself answered.

**5. Sounds natural from owner's voice**
Question must sound natural coming from a smart non-specialist. Overly technical jargon makes it sound rehearsed; too vague reveals lack of preparation.

## Question categories

Different types of questions serve different purposes. A good combat question set has mix.

**Type A — Technical depth probes**
Test specific technical claims with specific demands. From `failure-modes-mapping` — top failure modes become these.

**Type B — Numerical anchor tests**
Test claims against numerical anchors from immersion. "You quote $X per unit; the industry baseline is $Y — walk me through how you achieve that."

**Type C — Comparative framing**
Position project against alternatives. Tests their awareness of competitive landscape.

**Type D — Maturity probes**
Test claimed maturity vs actual. "You say production-ready; what's the largest deployment running today?"

**Type E — People probes**
Test team capability through specific competence questions. "Who specifically on your team has experience with [specific challenge]?"

**Type F — Marker questions**
Designed not for specific answer but to gauge team's overall competence. Ask the same kind of question regardless of project. Their handling reveals professionalism.

For a standard combat question set: 2 type A (failure modes), 1-2 type B (numbers), 1 type C (competition), 1 type D (maturity), 0-1 type E (team), 1 type F (marker).

## The procedure

### Step 1 — Receive inputs from prior phases

From immersion: schools, debates, numerical anchors
From deconstruction: real shape, demonstrated vs claimed
From failure modes: top critical failure modes with probe questions

### Step 2 — Identify which failure modes become primary questions

Top 2-3 failure modes from mapping become Type A questions. These are the most decisive — if responses are weak here, the project has serious problems.

### Step 3 — Add numerical anchor tests

From immersion Layer 4 numerical anchors, identify 1-2 specific numbers in their pitch that can be tested against industry baselines. Frame as questions.

### Step 4 — Add comparative framing question

What's the most credible alternative approach? Frame as: "Your approach is X; group Y argues Z is better because [reason]. How do you respond?"

This tests both their awareness of competition and their thinking quality.

### Step 5 — Add maturity probe

From deconstruction maturity assessment, find the gap between claimed and actual. Frame as specific question testing real maturity.

### Step 6 — Optional team probe

If meeting includes team members beyond pitcher, can ask specific competence question to specific person. Tests whether team is real or one-person show.

### Step 7 — Add marker question

A high-quality marker that works for most meetings:
- "What would convince you that you're wrong about [their core thesis]?"
- "If you had to bet against your own project, where would you place the bet?"
- "What's the strongest critique you've received and how have you responded?"

These reveal intellectual honesty (good responses engage seriously) vs marketing-mind (deflect to "no critique is fundamental").

### Step 8 — For each question, design full structure

For each question:
- Exact phrasing as owner would say it
- Expected good response (what to listen for)
- Expected uncertain response (what to listen for)
- Expected evasive response (what to listen for)
- Follow-up question ready
- Tactical timing notes (when in meeting to ask)

### Step 9 — Order and prioritize

Question order matters tactically:
- Start with **moderate** questions to establish expert framing without immediately confronting
- Build to **probing** questions in middle
- Close with **decisive** failure mode questions (they remember the end most)
- Marker question often best in middle (catches them off guard)

### Step 10 — Output combat cards

Each question on a "card" — owner can review during meeting, scan during their answer for matching pattern.

## Worked example — for hypothetical FusionCorp meeting

Combat questions for evaluating fusion startup.

### Question 1 (Type B — Numerical anchor):
**Phrasing:** "Your investor deck cites $3B per gigawatt for commercial deployment — current ITER scaling and similar projects suggest $5-7B for first-of-kind. Walk me through how you achieve $3B."

**Good response:** Specific cost reduction items with quantification (HTS magnets reduce cost by X, modular construction by Y, learning curve assumptions Z). Acknowledgment that first-of-kind is more expensive, with credible learning curve.

**Uncertain response:** Vague references to "we believe" with general reasoning but few specifics.

**Evasive response:** "Our financial model is proprietary" or pivot to talking about overall vision.

**Follow-up:** "Is your $3B for first-of-kind or nth-of-kind? What's the ratio you assume?"

### Question 2 (Type A — Failure mode):
**Phrasing:** "Walk me through your tritium supply for the first 5 years of operation. Global production is roughly 30 kilograms; commercial fusion needs 50-100 kilograms per year."

**Good response:** Specific plan including either tritium breeding integration from day one (with breeding ratio targets), specific supplier agreements, or DT-light operation with tritium accumulation strategy. Demonstrates awareness this is critical.

**Uncertain response:** "We're working on partnerships with [supplier]." Acknowledges issue but no concrete plan.

**Evasive response:** "Tritium supply is an industry-wide challenge that will be solved as commercial fusion scales." Pure deflection — they haven't engaged with the problem.

**Follow-up:** "What's your specific breeding blanket design? Has it been demonstrated?"

### Question 3 (Type A — Failure mode):
**Phrasing:** "What materials work for a first wall under 14 MeV neutron flux for 5 years? What's your specific material development partnership and timeline?"

**Good response:** Names specific candidate materials (RAFM steels, vanadium alloys, etc.), specific partner (national lab, university, materials company), realistic timeline acknowledging materials are 10-15 year challenge.

**Uncertain response:** Acknowledges challenge, mentions general materials work, no specific partnership.

**Evasive response:** "Materials science is advancing rapidly" or similar non-answer.

**Follow-up:** "How much testing time has the candidate material received under representative neutron spectrum?"

### Question 4 (Type C — Competitive framing):
**Phrasing:** "Helion Energy claims they don't need DT fusion — they're targeting D-He3 or p-B11. If they succeed, your tritium and materials problems become moot. How do you assess their probability of success?"

**Good response:** Engaged technical analysis of why DT remains the right choice (gain factor advantages, demonstrated physics) plus serious assessment of Helion's risk-reward (acknowledging if they succeed it changes industry).

**Uncertain response:** General comparison but vague on Helion-specific physics.

**Evasive response:** "We focus on our approach; they have their approach."

**Follow-up:** "What would have to be true for you to pivot to non-DT fuels?"

### Question 5 (Type D — Maturity probe):
**Phrasing:** "Your timeline shows commercial Q=30 operation by 2030. ITER's first plasma is now expected 2025-2030 with Q=10 demonstration after that. What gives you 5 years compression on something more ambitious than ITER?"

**Good response:** Specific reasoning about why their approach is faster — engineering simpler than ITER, modular construction, smaller machine reaches conditions faster, etc. Acknowledges aggressiveness of timeline.

**Uncertain response:** Generic claims about "private sector speed."

**Evasive response:** "Different scope, different program."

**Follow-up:** "What's the longest pulse duration you've achieved at your target plasma parameters?"

### Question 6 (Marker):
**Phrasing:** "If you had to bet against your own approach, where would you place the bet?"

**Good response:** Engages seriously, names actual technical concern, demonstrates they've thought about failure mode honestly.

**Uncertain response:** Hesitant but eventually offers concern.

**Evasive response:** "We don't bet against ourselves; we focus on solutions" or similar deflection.

**Follow-up:** "What probability would you give to that bet?"

### Question 7 (Type A — Failure mode):
**Phrasing:** "What's the longest pulse duration any tokamak has achieved at your target plasma parameters? What specifically is your path from there to continuous operation?"

**Good response:** Specific number (with citation to which tokamak achieved it), specific physics-based plan with intermediate milestones.

**Uncertain response:** Approximate number, vague plan.

**Evasive response:** Pivot to discussing other achievements, redirect to magnet capabilities, etc.

**Follow-up:** "What's the next 10x improvement you expect, and when?"

### Tactical ordering for meeting:

Questions 1-2 first (establish technical framing, force engagement with hard numbers and tritium). Then 3-4 (materials and competition — probing depth). Question 6 (marker) somewhere mid-late, catches them in flow. Questions 5 and 7 toward end (maturity and pulse duration — both hit decisive points to close on).

## Anti-patterns

- **Generic questions.** "How does it work?" extracts nothing useful. Be specific.
- **Questions without expected answers.** Without prepared good/uncertain/evasive patterns, can't interpret response.
- **Too many questions.** 5-7 sharp questions > 20 weak ones. Quality matters more than quantity.
- **Questions that sound rehearsed.** Use vocabulary natural to a smart non-specialist, not jargon overload.
- **No follow-ups.** Initial answer is rarely complete. Be ready to push.
- **All same question type.** Mix Type A through F for full coverage.
- **Wrong tactical order.** Hardest questions at start makes meeting adversarial; at end loses owner the chance to use information productively.
- **Missing marker question.** Easy to skip but reveals more than any specific technical question.

## Output template

```
─── COMBAT QUESTIONS ───

Meeting: <description, time>
Project: <identifier>
Number of questions: <count>

QUESTION 1
Type: [A | B | C | D | E | F]
Phrasing (exact): "<question as owner would say it>"
Good response signs: <what to listen for>
Uncertain response signs: <what to listen for>
Evasive response signs: <what to listen for>
Follow-up: "<follow-up question>"
Tactical timing: <when in meeting to ask>

QUESTION 2
[same structure]

[5-7 questions total]

TACTICAL ORDER (suggested):
1. <Question N>
2. <Question M>
[etc.]

OPENING/CLOSING NOTES:
- Opening: <how to enter the meeting tonally>
- Mid-meeting: <signals to watch for change in approach>
- Closing: <what to leave with>
```

## Integration with Expert protocol

This is **Phase 4** of the Expert protocol — the main output. Generated last after immersion, deconstruction, and failure modes feed into it.

Output is the primary deliverable to owner. Format is "combat cards" — printable/screenshot-able for quick reference during meeting.

Stored in ExpertBriefing record under `combatQuestions` field. Post-meeting debrief updates: which questions worked, which didn't, what evasions team used, what owner learned.

This is what makes Expert different from generic AI advice. The questions are designed for tactical use in real meeting dynamics, not for impressing on paper.

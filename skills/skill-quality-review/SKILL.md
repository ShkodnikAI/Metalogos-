---
name: skill-quality-review
description: Reviewing a SKILL.md against the Fosved skill standard (METHODOLOGY.md I.3) — checking the mandatory sections, the quality of anti-patterns, the procedure's concreteness, and consistency with the department's psychotype. The quality gate that keeps skills from degrading into vague inspiration.
---

# Skill Quality Review — Holding Skills to the Standard

A skill is a methodological document, not a motivational one. This skill is how the Kuznitsa checks that every `SKILL.md` meets the Fosved standard — has the mandatory sections, real anti-patterns, a concrete procedure, and a tone matching its department's psychotype.

## Prerequisites

- METHODOLOGY.md I.3 (the skill standard) in context
- The department's psychotype known (for the tone check)
- A `SKILL.md` to review

## Core principle

> A skill without anti-patterns is not methodology — it is inspiration. The difference between the two is the whole point of the Fosved skill system. Inspiration tells you what good looks like; methodology tells you what good looks like, what the failure modes are, and exactly how to avoid each one. The quality review's job is to refuse anything that has slipped from methodology back into inspiration.

## The skill standard (METHODOLOGY.md I.3)

Every `SKILL.md` has this structure:

**Mandatory sections (I.3.1):**
- `name`, `description` (frontmatter) — the description is the most important line; it is how Yana and the Veteran decide whether to load the skill
- A heading with a one-phrase statement of the skill's value
- `Core principle` — with an inversion of intuition where possible
- `The procedure` — numbered, concrete steps
- `Anti-patterns` — minimum 3
- `Output template`

**Recommended (I.3.2):**
- `Prerequisites` — if the skill depends on others
- `Worked example` — for skills where the technique is counter-intuitive
- `Reference cases / data` — if applicable
- `When NOT to use this skill` — boundaries

**Forbidden (I.3.3):**
- "Generic AI assistant" phrasing
- Descriptions with no concrete steps
- A list of tips with no procedure
- A skill with no anti-patterns (without them — not methodology, inspiration)

## What the review checks

### 1. Mandatory sections present
Frontmatter with name + description. Value-statement heading. Core principle. Numbered procedure. Anti-patterns. Output template. Any missing → the skill fails the standard.

### 2. The description line
This is the line Yana and the Veteran use to decide whether to load the skill. It must say, in one sentence, *what the skill does and when it applies*. A vague description ("helps with analysis") makes the skill un-routable. A sharp description is specific about the technique and the trigger.

### 3. Core principle quality
A real principle, ideally with an inversion of intuition — something that corrects a natural but wrong instinct. "Be careful" is not a principle. "The forgotten load case, not the calculated one, is what fails the structure" is a principle.

### 4. Procedure concreteness
Numbered steps that say what to *do*, not what to think about. "Consider the requirements" is not a step. "List every input parameter and tag each known / assumed / provisional" is a step. The forbidden pattern is a "list of tips with no procedure".

### 5. Anti-patterns — minimum 3, and real
Not just the count. Each anti-pattern must be a genuine failure mode — something a real practitioner would actually do wrong — ideally with *why it is wrong* and a counter-measure. Three anti-patterns that are all trivial restatements ("don't do it badly") fail the spirit of the standard even if they pass the count.

### 6. Output template present and usable
A structured template the parser can use to extract data into the archive. Not prose — a structured shape.

### 7. Psychotype consistency (METHODOLOGY.md I.5.6)
The skill's *tone* must match the department's declared psychotype. A pedant department's skill is written pedantically — precise, exhaustive. A paranoid department's skill hunts edge cases. A skill written in the wrong tone confuses the specialist and is the psychotype-mismatch antipattern. The reviewer reads the skill asking "does this sound like the declared psychotype wrote it?"

### 8. Forbidden phrasing absent
No "generic AI assistant" language. No vague descriptions. No tip-lists masquerading as procedures.

## The procedure

### Step 1 — Check the mandatory sections
Walk the I.3.1 list. Every mandatory section present? Any missing → fail, list what is missing.

### Step 2 — Judge the description line
Is it specific about technique and trigger? Could Yana route on it? Vague → fail.

### Step 3 — Judge the core principle
A real principle, with an intuition inversion if the topic allows? Or just a platitude?

### Step 4 — Judge the procedure
Concrete numbered actions, or a tip-list in disguise? Each step a *do*, not a *think about*.

### Step 5 — Judge the anti-patterns
At least 3. Each a real failure mode with a reason and ideally a counter-measure. Not trivial restatements.

### Step 6 — Check the output template
Present, structured, usable by a parser.

### Step 7 — Check psychotype consistency
Read the skill's tone against the department's psychotype. Match or mismatch?

### Step 8 — Scan for forbidden phrasing
Generic-assistant language, vague descriptions, tip-lists.

### Step 9 — Verdict
Meets the standard / fails — with the specific findings and what must be fixed.

## When this skill runs

- **During `/recruit`** — every skill of a new department is reviewed before the department is released (stage 3 of the checklist)
- **During quarterly methodology review** — skills are re-reviewed; degraded skills are caught
- **During `/debrief`** — when a department struggles, its skills are reviewed as a possible cause

## Worked example

Reviewing a Tier 2 skill submitted as part of a new department's creation.

- **Step 1 — mandatory sections:** frontmatter present, value-heading present, core principle present, procedure present, output template present. Anti-patterns section present. ✓ count-wise.
- **Step 2 — description:** "Helps the department analyze things effectively." **Fail.** This is vague — Yana cannot route on it; it does not say what technique or when. Finding: the description must name the specific technique and its trigger.
- **Step 3 — core principle:** "It is important to be thorough and accurate." **Fail.** This is a platitude, not a principle — no inversion, no correction of a wrong instinct. Finding: rewrite as a real principle.
- **Step 4 — procedure:** the steps say "Consider the context", "Think about the requirements", "Review the output". **Fail.** These are think-abouts, not do-actions — a tip-list in disguise (a forbidden pattern, I.3.3). Finding: rewrite as concrete actions.
- **Step 5 — anti-patterns:** there are exactly 3, but they read "Doing it carelessly", "Not being thorough", "Skipping steps". **Fail the spirit.** The count is met but these are trivial restatements, not real failure modes with reasons. Finding: replace with genuine, specific anti-patterns.
- **Step 7 — psychotype:** the department's psychotype is Pedant, but the skill's tone is loose and breezy. **Psychotype mismatch (I.5.6).** Finding: rewrite in the precise, exhaustive pedant tone.

**Verdict:** the skill fails the standard on multiple counts. It has the *shape* of a skill (sections present) but has slipped into inspiration — vague description, platitude principle, think-about procedure, trivial anti-patterns, wrong tone. It must be rewritten before the department is released. The review names each finding concretely so the rewrite is directed, not guesswork.

## Anti-patterns (of reviewing skills)

- **Counting, not judging.** Confirming "3 anti-patterns exist" without checking they are *real*. The count can pass while the spirit fails.
- **Passing the shape.** Approving a skill because all sections are present, without judging their quality. A skill can have every section and still be inspiration.
- **Skipping the psychotype check.** Reviewing a skill without reading it against the department's psychotype. The I.5.6 antipattern slips through.
- **Accepting a vague description.** The description is the routing line — a vague one breaks loading. It cannot be waved through.
- **Accepting platitude principles.** "Be careful and thorough" is not a core principle. A principle corrects an instinct.
- **Accepting think-about procedures.** "Consider X" is not a step. Steps are actions.
- **Vague findings.** Telling the author "this skill needs work" instead of naming each specific failure. The review must direct the fix.
- **Inconsistent strictness.** Holding new departments' skills to the standard but letting existing departments' degraded skills pass at quarterly review.

## Output template

```
SKILL QUALITY REVIEW — <skill name>  (department: <dept>, psychotype: <...>)

MANDATORY SECTIONS (I.3.1)
- Frontmatter (name + description): present / missing
- Value-statement heading: present / missing
- Core principle: present / missing
- Numbered procedure: present / missing
- Anti-patterns (>= 3): <count> — present / missing
- Output template: present / missing

QUALITY JUDGEMENTS
- Description line (routable, specific): pass / fail — <...>
- Core principle (real, intuition inversion): pass / fail — <...>
- Procedure (concrete actions, not think-abouts): pass / fail — <...>
- Anti-patterns (real failure modes, not restatements): pass / fail — <...>
- Output template (structured, usable): pass / fail — <...>
- Psychotype consistency (tone matches <psychotype>): pass / fail — <...>
- Forbidden phrasing absent: pass / fail — <...>

VERDICT: meets the standard / fails — findings:
- <specific finding + what to fix>
```

## Integration

- Tier 1 — runs during `/recruit` (stage 3), quarterly review, and `/debrief`
- `specialist-creation` calls this on every skill of a new department
- `methodology-application` — this skill deep-dives checklist stage 3
- `psychotype-assessment` provides the psychotype the tone is checked against
- Recursive: the Kuznitsa's own 8 skills are held to this same standard

---
name: user-task-analysis
description: Foundational discipline for design — every design starts with "what is the user trying to accomplish?" not "what should this screen look like?". Map user tasks before sketching pixels. Without this analysis, designs solve imaginary problems and miss real ones.
---

# User Task Analysis — Designing for Tasks, Not Screens

The most common design failure: designer asked "make a dashboard" produces beautiful dashboard nobody uses. Reason: nobody asked what the user is trying to accomplish. The dashboard answers questions nobody asked.

This skill is the prevention: every design begins with explicit user task analysis.

## Prerequisites

- Brief received (from owner, Yana, or Dev request)
- Target users identified (who, in what context)

## Core principle

> Users have tasks, not screens. They want to buy something, schedule something, find something, decide something. The interface is the means. Start with the task, derive the interface. Reversed order produces interfaces that look right but don't work.

## The five questions

For every screen/feature/flow, answer:

1. **Who** is the user? (role, expertise level, mental state)
2. **What** are they trying to accomplish? (specific task, in their words)
3. **Why** are they doing this? (motivation, broader goal)
4. **When/Where** — context of use? (mobile commute, desktop work, panic, exploration)
5. **How** would they describe success? ("done when I see X" or "done when I get Y")

Document answers before sketching. 10 minutes investment saves hours of rework.

## Task taxonomy

Categorize user tasks:

**Transactional tasks:** complete an action.
- Buy product
- Submit form
- Send message
- Confirm booking

**Informational tasks:** find/understand something.
- Check status
- Compare options
- Read article
- Look up reference

**Decisional tasks:** make a choice.
- Pick a plan
- Choose between options
- Approve / reject

**Creational tasks:** make something new.
- Write document
- Build configuration
- Compose message

**Maintenance tasks:** keep something working.
- Update settings
- Manage subscriptions
- Resolve issues

Each task type has different design implications. Transactional needs clear path; informational needs scannability; decisional needs comparison structure.

## Task hierarchy

User tasks form hierarchies:
- **Primary task:** main goal (e.g., "find a place to stay tonight")
- **Secondary tasks:** support primary (e.g., "filter by price", "check reviews", "compare locations")
- **Tertiary tasks:** edge cases (e.g., "modify dates after starting search")

Design serves primary first. Secondaries accessible without disrupting primary flow. Tertiaries reachable but not prominent.

Bad design: equal weight to all three levels. Primary task gets lost in noise.

## User mental models

Users come with mental models of how things work. Designs that fight mental models cause friction.

Examples of mental models:
- Trash can: dragging file there deletes
- Folder hierarchy: nested folders contain files
- Shopping cart: items added, reviewed, purchased
- Search: type query, see results

Design either:
- Match existing mental model (cheapest, fastest adoption)
- Introduce new model with strong scaffolding (expensive, only when payoff justifies)

Don't create novel mental models for problems with established patterns.

## Context of use

Context dramatically changes design needs:

**Mobile vs Desktop:**
- Mobile: short interactions, thumb-driven, frequent interruption
- Desktop: longer sessions, keyboard+mouse, focused work

**Time-pressured vs Exploratory:**
- Pressured: minimal friction, default-everything, big buttons
- Exploratory: rich info, controls visible, comparisons possible

**Expert vs Novice:**
- Expert: shortcuts, dense info, power features
- Novice: guided, defaults, explanations

**Stressed vs Calm:**
- Stressed: don't overwhelm, clear primary action, no decisions if avoidable
- Calm: can present options, education, customization

A design optimized for one context fails in another. Often multiple contexts must be supported — explicit decisions needed.

## Task flow mapping

For complex tasks, map the steps:

Example: "Submit expense report"
```
1. Open expense form
2. Select expense type
3. Enter amount and date
4. Attach receipt
5. Categorize
6. Add note (optional)
7. Submit
8. Confirmation
```

Each step is a design decision:
- Is step necessary? (Can step 5 be auto-categorized from type?)
- Can steps be combined?
- What's the failure mode of each step?
- What can be defaulted?

Goal: minimum steps + minimum friction per step + clear progress indicator.

## Edge cases analysis

For each task, ask:
- What if user has no data yet? (empty state)
- What if there's too much data? (overflow, pagination)
- What if network fails mid-task? (recovery)
- What if user enters invalid data? (validation, error messaging)
- What if user wants to cancel mid-task? (undo, escape)
- What if user is interrupted? (state preservation)

Each is a design opportunity, often overlooked.

## Writing task descriptions

Format:
```
[User role] needs to [task action] so that [outcome].

Context: [when, where, why now]
Frequency: [how often this happens]
Success criteria: [how user knows it worked]
Failure cost: [what happens if it goes wrong]
```

Example:
```
A busy executive needs to approve a vendor invoice so that the vendor gets paid on time.

Context: Likely on mobile during a commute, between meetings, with one hand available.
Frequency: 5-15 times per week.
Success criteria: Sees confirmation that invoice was approved and entered the AP queue.
Failure cost: Approves wrong invoice (financial loss) or fails to approve (vendor relationship strained).
```

This concrete description constrains the design. Inferring from "make an invoice approval screen" is guesswork.

## Anti-patterns

- **Skipping the analysis.** "Just make the screen". Designs solve random problems.
- **Designer-as-user.** Designer assumes own preferences are users'. Different mental models often.
- **One-size-fits-all.** Ignoring context differences. Mobile screen scaled down from desktop = fails on mobile.
- **All edge cases as primary.** Designing for the 1% case ruins the 99%. Edge cases are secondary.
- **No success criteria.** Without clear success definition, design completion is subjective.
- **Multiple primary tasks.** "User can do X or Y or Z" — pick one primary, others secondary.
- **Inventing tasks.** Adding features users didn't ask for because designer thinks "would be cool".
- **Imagining users.** Without data, assumptions about users are designer's projection. Better: ask, observe, test.

## Integration

- Applied first in every `design` workflow
- `design-system-thinking` uses task analysis to choose components
- `wireframe-production` uses task flow as backbone
- `interaction-states` derive from task failure modes
- `user-research-methods` validates task assumptions

## Output template

Every DesignArtifact starts with task analysis section:

```markdown
## User Tasks

**Primary task:** [one-sentence task statement]

**User profile:** [role, expertise, mental state]

**Context of use:** [device, time pressure, environment]

**Task flow:**
1. ...
2. ...

**Success criteria:** ...

**Failure modes to design for:**
- ...

**Secondary tasks:**
- ...

**Edge cases:**
- Empty state: ...
- Overflow: ...
- Errors: ...
- Cancellation: ...
```

This is the foundation everything else builds on.

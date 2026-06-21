---
name: architecture-decision-records
description: Foundational discipline for capturing architectural decisions as immutable historical records. Every non-trivial technical choice generates an ADR with context, alternatives considered, decision made, and consequences accepted. ADRs enable retrospective learning ("was this the right call?") and prevent re-litigating settled questions. Without ADRs the team forgets WHY decisions were made and repeats mistakes.
---

# Architecture Decision Records — Immutable Decision Trail

A codebase is a graveyard of decisions. Most are forgotten the moment they're made. When the next developer (or your future self) asks "why did we do it this way?" the answer is usually "I don't remember". This is how teams accumulate technical debt and repeat mistakes.

ADRs fix this. Every meaningful technical choice gets a short structured document. Six months later, you can answer the question.

## Prerequisites

- Active DevTask requiring a technical choice
- The choice is non-trivial (multiple plausible alternatives exist)
- Not for: typo fixes, formatting changes, obvious one-liners

## Core principle

> The value of an ADR is not in the decision recorded but in the **rationale and alternatives**. Anyone can see WHAT was decided by reading the code. Only an ADR captures WHY this and not the others. The why is what compounds.

## When you need an ADR

Triggers (any one):
- Choice between two or more plausible options (library X vs Y, REST vs GraphQL, SQL vs NoSQL)
- Pattern adopted that affects future code (architecture style, error handling convention, testing approach)
- Constraint accepted (latency budget, file size limit, deployment platform)
- Dependency introduced (new library, external service, infrastructure)
- Deviation from typical practice ("normally we'd do X, but here Y because...")

Skip ADR when:
- Following established team convention
- Trivial change (variable name, comment fix)
- Mechanical refactor without semantic change

## ADR format (minimal but complete)

Filename: `/docs/adr/<task_id>-<slug>.md` in the project repo.

```markdown
# ADR <task_id>-<slug>: <Short Decision Title>

**Status:** proposed | accepted | deprecated | superseded by ADR-XXX
**Date:** YYYY-MM-DD
**Deciders:** <who made the call>

## Context

What is the issue we're addressing? What constraints are in play? What background does the future reader need?
2-3 paragraphs maximum.

## Decision

What did we decide? One sentence headline, then 1-2 paragraphs of detail.

## Alternatives Considered

For each alternative seriously evaluated:

### Alternative 1: <Name>
- **Pros:** ...
- **Cons:** ...
- **Why rejected:** ...

### Alternative 2: <Name>
...

(Minimum 2 alternatives. If there really were no alternatives, an ADR isn't needed — it's not a decision.)

## Consequences

What did we accept? Trade-offs being made consciously.
- Positive consequences
- Negative consequences (yes, list them honestly)
- What this commits us to (dependency lock-in, paradigm constraints)

## Retrospective (filled in 1-3 months later)

(Empty at creation. Updated when retrospective happens.)
- Date:
- Was this the right call? yes / partially / no
- What we learned:
- Should we supersede?
```

## Minimum viable ADR (for small decisions)

For minor decisions where full format is overkill:

```markdown
# ADR <task_id>-<slug>

**Status:** accepted | **Date:** YYYY-MM-DD

## Context
[1 paragraph]

## Decision
[1-3 sentences]

## Alternatives
- X (rejected: <reason>)
- Y (rejected: <reason>)

## Trade-offs accepted
[2-3 bullet points]
```

3-5 minutes to write. Future self will thank you.

## The retrospective discipline

Each ADR has a `retrospective_date` 1-3 months out. When that date arrives, you revisit:

- Did the decision work?
- What did we underestimate or overestimate?
- Should we keep it, modify it, or supersede with new ADR?

This is the **compound learning step**. Without retrospectives, you make the same wrong calls repeatedly because you never check whether past calls worked.

Recording retrospective:
```javascript
await prisma.architectureDecision.update({
  where: { id: adrId },
  data: {
    retrospectiveDate: new Date(),
    retrospectiveNote: 'Postgres pooler with prepared statements was correct — handling 50 req/s with no issue, no need to migrate to PgBouncer. Decision validated.',
    wasRightCall: true
  }
});
```

If `wasRightCall: false` — analyze what info we lacked at decision time. This feeds back into `estimation-discipline` improvements.

## Decision quality vs Outcome quality

Important distinction:
- **Good decision:** best choice given the information available at the time
- **Good outcome:** turned out well in retrospect

These are different. A good decision can have bad outcome (unforeseen circumstances). A bad decision can have good outcome (luck). Retrospective evaluates **decision quality**, not outcome.

Example:
- Decision: choose Library X over Library Y based on benchmarks
- Outcome: 6 months later, Library X maintainer abandons it
- Retrospective: decision was correct given the data; outcome was bad due to external event; lesson is "check maintainer activity, not just benchmarks"

This nuance matters. Penalizing bad outcomes leads to risk-averse decision-making. Improving decision-making process is what compounds.

## Anti-patterns

- **No alternatives section.** ADR without alternatives is decision documentation, not decision record. The alternatives ARE the value.
- **Vague alternatives.** "Other libraries" — useless. Name them.
- **Decision in passive voice.** "It was decided that X" — who decided? When? Why? Use active voice and name deciders.
- **Skipping the negative consequences.** Every decision has trade-offs. Listing only positives means you didn't think through enough.
- **Forgetting retrospectives.** ADRs without retrospectives miss the compound learning.
- **ADR for everything.** ADR overhead must be justified by decision importance. Don't ADR a variable rename.
- **ADRs that nobody reads.** Make them discoverable: docs/adr/ directory, indexed in README, mentioned in commit messages.
- **Updating accepted ADR.** Once accepted, an ADR is immutable historical record. To change direction: write new ADR with `Supersedes ADR-XXX`.

## Storage

ADR file lives in repo at `/docs/adr/<task_id>-<slug>.md`.

Database record in `ArchitectureDecision` table for cross-referencing:
- Linked to DevTask
- Searchable across all projects
- Retrospective dates tracked centrally

Bot command `/dev-decide <task_id> <decision>` creates both — file in repo + DB record.

## Integration with other skills

- Used by every Dev workflow — `estimation-discipline` references ADRs for similar past decisions
- `tech-radar-maintenance` consults ADRs when changing tech state ("we adopted X in ADR-007, current usage is...")
- `iterative-implementation` requires ADR before non-trivial implementation begins
- Retrospective dates trigger reminders via scheduler

ADRs are the **memory of the engineering organization**. Without them, every project starts from zero.

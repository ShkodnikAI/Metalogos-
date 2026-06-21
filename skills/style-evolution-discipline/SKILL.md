---
name: style-evolution-discipline
description: How the Visual Department evolves its visual style over time without losing archive comparability. Quarterly review process — what to update, what to leave, how to version templates so historical visuals stay readable while new ones improve. The discipline that lets the department learn without breaking the archive.
---

# Style Evolution Discipline — Improving Without Breaking the Archive

The Visual Department is a learning system. New techniques emerge. Recall tests reveal which templates work poorly. Pattern scans surface effective approaches. The department must adapt — but adaptation without discipline destroys the archive's value.

This skill is the discipline for **how** to evolve.

## Prerequisites

- `infographic-templates` understood (template versioning concept)
- Recall test data accumulated (need ≥3 months of `VisualArtifact.recallSuccess` records)

## Core principle

> Visual style must improve to stay relevant, but the archive must remain comparable across time. The solution: **version templates explicitly, evolve them in discrete steps (quarterly), preserve old versions in archive**. New visuals use new versions, old visuals stay as they were — and the `templateVersion` field lets you see both data evolution AND style evolution simultaneously.

## The quarterly review cadence

Every quarter (Jan 1, Apr 1, Jul 1, Oct 1, at 10:30) the Visual Department runs:

```javascript
visual.quarterlyStyleEvolution();
```

Which performs:
1. **Aggregate recall stats** for last 90 days
2. **Compute coverage rate** (% of analyses with visuals)
3. **Review pattern scan observations** from last quarter
4. **Identify templates with low recall success** (< 60% in 90 days)
5. **Identify patterns worth adopting** from external scan
6. **Generate StyleEvolution record** with summary
7. **Notify owner** via Telegram

Owner reviews. If changes proposed:
- Bump template versions
- Update template specs in DB
- Add version history entry

## What can change in a quarterly review

### Tier 1 changes (template-level, OK to change)

- Layout adjustments within existing template (move panel positions)
- Add new optional field/slot to template
- Improve color encoding (e.g., add intensity gradient for severity)
- Adjust typography sizes if recall shows readability issues
- Change icon set within Lucide family
- Optimize spacing

These bump template version (v1.0 → v1.1) but don't break grammar.

### Tier 2 changes (brand-level, require explicit owner approval)

- Change brand color palette (add color, retire color, shift shade)
- Change primary typography
- Change canvas dimensions for a type
- Add new icon set beyond Lucide

These are RARE and require deliberate owner sign-off, not just quarterly automation.

### Tier 3 changes (architecture-level, very rare)

- Add new visual type (6th type)
- Retire existing visual type
- Change versioning scheme
- Change idempotence mechanism

Require full planning session, not quarterly review.

## What CANNOT change without owner approval

- Brand color semantics (gold = important, burgundy = risk, forest = positive) — these are sacred
- Template count (5 types, no creep)
- Visual grammar foundational rules (data-ink ratio, anti-chartjunk, F-pattern)
- Existing version history (no rewriting past versions)

## Template version protocol

When updating a template:

```javascript
// Inside quarterlyStyleEvolution:

// Get current
const template = await prisma.visualTemplate.findUnique({ where: { type: 'osp_analysis_card' } });
const current = template.currentVersion;  // e.g., "v1.0"
const next = bumpVersion(current);         // "v1.1"

// Capture history
const history = JSON.parse(template.versionHistory || '[]');
history.push({
  version: current,
  retiredOn: new Date().toISOString(),
  spec: template.spec,
  reasonForUpdate: '...recall rate < 60%, panel layout reorganized...'
});

// Update
await prisma.visualTemplate.update({
  where: { type: 'osp_analysis_card' },
  data: {
    currentVersion: next,
    spec: newSpecJSON,
    versionHistory: JSON.stringify(history)
  }
});
```

**All NEW VisualArtifact records** get `templateVersion: 'v1.1'`.
**Existing VisualArtifact records** keep `templateVersion: 'v1.0'`.

This is non-negotiable. Without preserving template_version on existing artifacts, you can't compare archive entries from different periods correctly.

## Pattern scan integration

Daily `dailyPatternScan()` accumulates observations in lightweight form (URL + brief description + applicable to template).

Quarterly review aggregates: which patterns appeared multiple times? Which were new and effective? Which became cliché?

**Examples of patterns from external scan:**
- "Isometric perspective for technology stack illustrations" (seen 5x in last quarter at Information is Beautiful, GitHub trending)
- "Color gradient as time-encoded data channel" (used effectively by Reuters Graphics, FT, NYT)
- "Micro-typography labels with hairline leader lines" (premium magazine pattern)

For each candidate pattern, decide:
- **Adopt:** add to relevant template, bump version
- **Watch:** keep observing, decide next quarter
- **Reject:** decoration without function, OR breaks brand grammar

Adoption requires: at least 2 quarters of observation, clear evidence of effectiveness, fits brand grammar.

## Recall-driven prioritization

Templates with lowest recall success get priority for review.

If `osp_analysis_card` has 85% recall success — keep it.
If `expert_briefing_card` has 45% recall success — investigate. Maybe:
- Combat questions too dense, hard to scan
- Failure modes color coding unclear
- Header takes too much space

Hypothesize fix, update template spec, bump version, observe next quarter.

This is **deliberate practice** — measure → identify weakness → adjust → re-measure.

## A/B testing for major changes

When changing a template significantly, optional A/B period:
- For one month, randomly assign new visuals to v1.0 (old) or v1.2 (new)
- Compare recall rates between groups
- Adopt v1.2 fully if shows improvement

In practice rarely needed — most changes are small enough that quarterly cadence is fine. A/B is for major redesigns.

## Annual portfolio review (separate but related)

Once per year (December), Annual Portfolio Review runs:
- Best 30 visuals of year (by recall + relevance + freshness)
- Style evolution summary (template version diffs)
- Pattern adoption record (what came in, what went out)
- Coverage trends (was 100% maintained?)
- Owner satisfaction overall

This compiles into a "Year in Visuals" Type 5 premium artifact (semi-manual).

## Anti-patterns

- **Frequent template changes.** Every change destabilizes archive comparability. Quarterly is the discipline.
- **Skipping version history.** Without version history, can't analyze what changed and why.
- **Pattern adoption without evidence.** "Looks cool" is not adoption criterion. Need observations + fit to grammar.
- **Brand color drift.** Adding "just a slightly different blue for this case" — strict no. Brand grammar protects archive.
- **Updating template silently.** All template changes must be logged in `versionHistory` with reason.
- **Forgetting to bump version.** Changing spec without bumping version means existing visuals are mis-tagged.
- **A/B paralysis.** A/B for every change is paralysis. Use A/B only for major redesigns (typically 1-2 per year).

## Quarterly review checklist

- [ ] Aggregate recall stats for last 90 days
- [ ] Compute coverage rate per source type
- [ ] Identify any template with recall <60% — list for update consideration
- [ ] Review pattern scan observations
- [ ] Identify 1-3 patterns for potential adoption
- [ ] For each candidate template update: write rationale, draft new spec, owner approval
- [ ] Apply updates with proper version bumping and history
- [ ] Generate StyleEvolution DB record
- [ ] Notify owner with summary

## Integration

- `lib/visual.js` `quarterlyStyleEvolution` function implements this
- Scheduler tasks run it on Jan/Apr/Jul/Oct 1
- StyleEvolution table records the history
- Pattern scan accumulates input data in lightweight format

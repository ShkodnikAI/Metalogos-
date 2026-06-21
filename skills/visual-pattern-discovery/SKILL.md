---
name: visual-pattern-discovery
description: How the Visual Department actively monitors the world's infographic landscape — daily scans of high-quality sources (Information is Beautiful, Reuters Graphics, Bloomberg, IMa Studio, Pudding, NYT graphics, r/dataisbeautiful), captures observations, accumulates them for quarterly pattern adoption review. The curator function — staying current without succumbing to trends.
---

# Visual Pattern Discovery — Active World Monitoring

The Visual Department's stylistic moat erodes if it doesn't evolve. New techniques appear constantly: novel encoding methods, color trends, layout innovations, interactive metaphors. This skill is **how** the department stays current — systematic observation, not random inspiration.

Frequency: daily lightweight scan, weekly aggregation, quarterly adoption decisions.

## Prerequisites

- Visual Department running for ≥30 days (need baseline to compare new patterns against)
- `style-evolution-discipline` understood (knows quarterly process for adoption)

## Core principle

> Trends are not patterns. A pattern is a technique that works across multiple sources, multiple subjects, multiple contexts — and works because of cognitive/perceptual mechanism, not because of fashion. The discovery process filters trends OUT and patterns IN. Department adopts the latter, ignores the former.

## Sources for monitoring (curated list)

These are high-signal sources. Daily scan rotates through them:

### Tier 1 — premium editorial graphics
- **Reuters Graphics** (graphics.reuters.com) — data journalism standards
- **Bloomberg Businessweek graphics** — premium magazine quality
- **NYT The Upshot** — analytical visualization
- **FT Graphics** — financial visualization
- **The Economist data** — concise visual analysis
- **Washington Post graphics** — explanatory journalism

### Tier 2 — design awards / showcases
- **Information is Beautiful Awards** annual winners
- **Malofiej Awards** (visual journalism)
- **SND (Society for News Design)** awards
- **Kantar Information is Beautiful** archive

### Tier 3 — community / experimental
- **r/dataisbeautiful** monthly top posts
- **Pudding.cool** essays
- **Observable** featured notebooks
- **#dataviz** Twitter trending

### Tier 4 — specialized infographic studios
- **IMa Studio** (referenced premium examples)
- **Visual Capitalist**
- **Information is Beautiful** website portfolio
- **Funkhaus** (architectural visualization)

### Tier 5 — academic / methodological
- **VIS Conference** proceedings (annual best papers)
- **Pew Research charts**
- **Our World in Data** charts

## Daily scan procedure

Each day, automated scan visits 2-3 sources from the rotation (different sources each day, full rotation in 7-10 days).

For each visited source, scan looks for:

**1. New visual encoding techniques**
- Novel chart types
- Hybrid visualizations
- Spatial encodings not previously seen

**2. Layout innovations**
- Section organization patterns
- Header/footer treatments
- Negative space usage

**3. Typography treatments**
- Font pairings
- Hierarchy techniques
- Annotation patterns

**4. Color usage patterns**
- Palette choices
- Semantic encoding
- Gradient usage as data channel

**5. Interaction patterns** (for inspiration, even if Visual Dept renders static)
- Progressive disclosure
- Comparison interactions
- Detail-on-demand

For each observation, lightweight record:

```javascript
{
  date: "2026-05-13",
  source: "reuters_graphics",
  url: "https://...",
  category: "encoding | layout | typography | color | interaction",
  description: "brief description of pattern",
  applicable_to: ["compression", "memorable", "card", "reference", "premium"],
  preliminary_assessment: "promising | unclear | reject",
  notes: "what's interesting about it"
}
```

Stored in JSON file or lightweight DB table. Quarterly review reads these.

## Weekly aggregation (Sunday)

Once per week, the department aggregates the 14-21 observations from the week:

- Group by category
- Identify recurring patterns (seen at 3+ sources)
- Flag innovations that seem highly applicable to Fosved visuals
- Filter out trendy-but-shallow (fad colors, current-year aesthetic clichés)

Sends summary to owner: "Week of [date]: 18 patterns observed, 3 promising:
1. [Pattern A] seen at Reuters, FT, NYT — could improve Type 3 cards
2. [Pattern B] from Information is Beautiful — applicable to Type 2 memorable
3. [Pattern C] from Pudding — interaction concept, can adapt for static"

Owner reviews. Discussion (optional). Patterns either marked for quarterly consideration or rejected.

## Quarterly adoption decision

At quarterly review, department considers patterns that:
- Were observed at 3+ independent sources
- Survived 2+ weekly assessments
- Have clear application to existing visual types
- Don't violate brand grammar (or violation is justified)

For each candidate, decision: **adopt / watch / reject**.

**Adopt:** add to relevant template, bump version, document in StyleEvolution

**Watch:** keep observing for another quarter, decide next time

**Reject:** document why (e.g., "fad — looks dated already", "violates brand grammar without compensating benefit", "applicable only to interactive contexts")

## Filtering trends from patterns

The hardest discipline. Some heuristics:

### Pattern signals
- Used across multiple unrelated subjects
- Works because of perceptual/cognitive mechanism
- Improves comprehension (measurable)
- Used by sources known for craft (not just trend-chasing)
- Survives 6+ months without becoming cliché

### Trend signals
- Concentrated in specific subject matter (e.g., crypto graphics 2021)
- Appears suddenly across many sources at once (suggests fad)
- Looks current-year — will look dated in 2 years
- Used heavily by single influential source only
- Stylistic without functional improvement

When unsure, watch for another quarter. No rush to adopt.

## Examples of patterns worth adopting (historical references)

**Pattern: Sparkline embedded in text** (Tufte, 1990s, still going)
- Tiny line charts inline with text
- Adds time series context to descriptions
- Adopted into Visual Dept reference sheets

**Pattern: Color gradient as quantitative encoding** (NYT, mid-2010s)
- Heat-map-style coloring on geographic or categorical encoding
- Works because gradient detection is innate
- Adopted into Visual Dept Type 1 compression visuals for showing distributions

**Pattern: Annotation-led narrative** (Bloomberg, Reuters, 2020s)
- Chart with text annotations directly on data points
- "Here's where X happened" pointing to spike
- Replaces separate caption with in-chart explanation
- Adopted into Visual Dept timeline elements

## Examples of trends to reject (historical references)

**Trend: Infographic illustration mania** (2010-2015)
- Cartoon characters explaining data
- Now looks dated
- Visual Dept rejected (kept restrained line illustration only)

**Trend: 3D charts everywhere** (2000s)
- 3D bars, pie charts, etc.
- Distorted data perception
- Rejected universally; Visual Dept maintains 2D discipline

**Trend: Big numbers with arbitrary fonts** (2018-2020 social media style)
- Display fonts for emphasis
- Often illegible at small sizes
- Rejected; Visual Dept stays with IBM Plex Mono

## Pattern adoption proposal template

When proposing a pattern for adoption at quarterly review:

```markdown
## Pattern: [Name]

**First observed:** [date] at [source]
**Subsequent sightings:** [count] across [N] sources
**Survives 6+ months:** [yes/no]

**Mechanism:** Why does this work? (cognitive/perceptual reason)

**Application to Visual Dept:**
- Affected types: [list]
- Affected templates: [list]
- Template changes needed: [bullet list]

**Brand grammar compliance:** [yes/no/with-adjustments]
- If violation: justification

**Risk if adopted:** what could go wrong

**Expected benefit:** measurable improvement (recall rate, comprehension)

**Recommendation:** adopt / watch / reject

**Decision (filled in by owner):** ___________
```

## Anti-patterns

- **Adopting trends because they're current.** Department's value is timelessness, not currency.
- **Ignoring world entirely.** Static styles erode. Need active monitoring.
- **Adopting too fast.** Quarterly cadence. Don't update templates mid-quarter based on one inspiring source.
- **Copying specific implementations.** Adopt the **principle**, not the **execution**. The principle adapts to brand grammar.
- **Treating r/dataisbeautiful as authoritative.** Reddit selection bias favors novelty over craft. Reuters/Bloomberg/FT are higher signal.
- **Adopting without measurable benefit.** Patterns must improve recall, scan-speed, or comprehension — not just look different.
- **Overcomplicating scan.** This is lightweight observation, not deep analysis. 15-30 minutes/day max.

## Lightweight implementation note

For v1.0 of Visual Department, `dailyPatternScan()` in `lib/visual.js` is a placeholder (just logs timestamp).

Full implementation in v1.1 involves:
- Web search + image search via Anthropic + Brave
- Storage of observations in JSON file (or new PatternObservation Prisma model)
- Aggregation scripts for weekly summary
- Integration with quarterly review

Start with manual mode: owner shares interesting visuals via chat, department logs them in observations file. Automate later when value proven.

## Integration

- Scheduler runs `dailyPatternScan` daily at 03:00
- Weekly aggregation manually triggered or auto-scheduled Sunday 18:00
- Feeds into `style-evolution-discipline` quarterly review
- Discoveries documented in `StyleEvolution.patternsAdded` field with provenance
- Owner notified of high-confidence pattern adoption recommendations

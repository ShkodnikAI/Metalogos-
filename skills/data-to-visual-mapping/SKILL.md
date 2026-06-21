---
name: data-to-visual-mapping
description: Rules for translating structured source data (Analysis fields, Briefing sections, Artifact properties) into visual elements. Defines what data goes where, what encoding to use for each data type, how to handle missing fields. Foundation for template-driven generation.
---

# Data-to-Visual Mapping — Source Fields to Visual Elements

Source data has structure (Analysis has leaves/branches/stem/roots/fruits + scenarios + actor forces; Briefing has combat questions + failure modes; etc.). The template has slots (header / hero / panels / footer). This skill is the rules for what fills what.

## Prerequisites

- `visual-grammar` loaded
- `infographic-templates` loaded
- Source data accessible via Prisma

## Core principle

> Each source field has a natural visual encoding. A probability is a gauge or bar. A timeline is a horizontal line with markers. A list of options sorted by importance is a bar chart. A categorical comparison is small multiples. Following natural encodings makes visuals scannable; fighting them confuses viewers.

## Mapping rules per source type

### OSP Analysis V2 → Analysis Card

| Source field | Visual element | Position |
|--------------|----------------|----------|
| `topic` | H1 headline, navy | Header center |
| `createdAt` | Date caption, grey | Header right |
| `confidence` | Color-coded badge | Header right (gold high / forest medium / burgundy low) |
| `leaves` (visible events) | Text panel with bullets, max 5 | Topology block, top row left |
| `branches` (active processes) | Text panel with bullets, max 5 | Topology block, top row right |
| `stem` (institutional structure) | Text panel with bullets, max 4 | Topology block, middle |
| `roots` (underlying conditions) | Text panel with bullets, max 4 | Topology block, lower |
| `fruits` (anticipated outcomes) | Highlighted text panel with gold border, max 3 | Topology block, bottom |
| `scenarios` (JSON array) | Sorted bar chart (probability) | Scenarios panel |
| `forcesAnalysis` (JSON) | Actor list with P value | Optional sub-panel |
| `bifurcationPoints` (JSON) | Fork diagram | Optional, only if depth=full |
| `verificationDate` | Date with countdown, mono font | Verification panel |
| `watchUntil` | Date, mono font | Verification panel |

### Expert Briefing → Briefing Card

| Source field | Visual element | Position |
|--------------|----------------|----------|
| `topic` | H1 headline | Header |
| `meetingDate` | Date + countdown ("через 4 часа") | Header right |
| `depthLevel` | Level badge (1/2/3) | Header left |
| `combatQuestions` (top 5) | Numbered list, dominant section | Combat questions panel |
| `failureModes` (top 3) | Color-coded panels (severity → burgundy intensity) | Failure modes panel |
| `bullshitDetection` flags | Red flag list with icons | Red flags panel |
| `understandingScore` | Gauge (post-debrief only) | Footer left |
| `bullshitDetected` | ✓/✗ icon | Footer center |

### Knowledge Artifact → ЛЗ Card

| Source field | Visual element | Position |
|--------------|----------------|----------|
| `topic` | H1 headline | Header |
| `status` | Status badge | Header right |
| `lastScannedAt` | Date caption | Header right |
| `hypePosition` | Gartner curve with marker | Hype cycle panel |
| `disruptionProbability` | Radial gauge (0-100%) | Disruption panel |
| `disruptionTimeline` | Text below gauge ("12-24 mo") | Disruption panel |
| `inflectionDetected` | Lightning icon if true | Footer |
| `fullProfile` (compressed) | Bullet list, max 6 | Findings panel |

### Client → Reference Sheet

| Source field | Visual element | Position |
|--------------|----------------|----------|
| `name` + `company` | H1 + H2 | Header |
| `tier` | Tier badge (1-5, color graded) | Header right |
| `status` | Status indicator | Header right |
| `email`, `phone` | Contact lines, mono font | Contact panel |
| `totalRevenue` (aggregated from invoices) | Large data number, gold if positive | Financial panel |
| `activeProjects` count | Number | Projects panel |
| `timeline` (last 10 events) | Horizontal timeline with event markers | Timeline strip |

### Project → Reference Sheet

Similar structure, fields: name, type, status, progress, team, deadline, tasks, client annotation.

## Encoding decisions by data type

| Data type | Best encoding | Avoid |
|-----------|---------------|-------|
| Single percentage | Gauge or bar | Pie chart for 1 value |
| Multiple percentages (3-7) | Sorted horizontal bars | Pie chart |
| Multiple percentages (8+) | Aggregate into 3-5 categories, then bars | Showing all individually |
| Time series (5+ points) | Sparkline | Table of values |
| Ranked list (top N) | Numbered list with size encoding | Unordered list |
| Categorical comparison (2-4 items) | Side-by-side small multiples | Stacked bars |
| Hierarchical structure | Tree or indented list | Network diagram for simple hierarchy |
| Geographic data | Map with markers | List of locations |
| Date/timeline | Horizontal line with markers | List of dates |
| Probability + outcome | Gauge with outcome label | Just text |
| Comparison vs baseline | Bar with reference line | Bar alone (loses context) |

## Handling missing data

Templates expect specific fields. When fields are missing:

**Strategy 1 — Graceful degradation.** Show placeholder ("data pending") in same position. Don't shift layout — comparability matters.

**Strategy 2 — Fallback to lower-tier visual.** If Analysis has no `bifurcationPoints` but template has a slot — leave slot empty with subtle "—" marker, not move other content.

**Strategy 3 — Mark template version mismatch.** If source is V1 schema and template is V2, render what's available and note version mismatch in footer.

**NEVER:**
- Invent data to fill missing fields
- Hide the missing slot (breaks template comparability)
- Crash silently — log the missing field

## Text compression rules

Many source fields are long text. Compression for visual:

- **Bullet points:** max 5 per panel, max 80 chars each
- **Headlines:** max 60 chars (else wraps awkwardly)
- **Data labels:** max 30 chars
- **Captions:** max 100 chars

Compression methods (in priority order):
1. Use source's own most-important sentence (often first)
2. Extract key noun phrases
3. Use ellipsis (...) for truncation
4. Last resort: Claude generates summary, but cite as "summarized"

## Anti-patterns

- **Inventing data to fill template.** If source has no scenarios, don't make up scenarios. Mark slot empty.
- **Stuffing all source text into visual.** Visual is compressed view. Text overflow = bad encoding choice.
- **Wrong encoding for data type.** Pie chart for 12 categories. Network diagram for simple list. Match encoding to data type.
- **Inconsistent encodings across cards.** If OSP card #1 shows scenarios as bars and card #2 as pie — comparability destroyed. Stick to template.
- **Translating one source to multiple cards inconsistently.** Same source data should produce same card output (modulo content hash). If it doesn't, generator has bug.

## Integration

Used by `lib/visual.js` `renderCardFromTemplate` — function reads template spec, applies mapping rules to fill template slots from source data.

Tested by `style-evolution-discipline` quarterly review — check if any source fields are systematically not mapped (suggests template needs update).

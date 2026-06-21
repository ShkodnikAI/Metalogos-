---
name: infographic-templates
description: Catalog of visual templates for Visual Department. Defines the 5 output types (Compression, Memorable, Analysis Card, Reference Sheet, Premium Spec) — when to use each, what data each requires, what layout each follows. Templates are versioned (v1.0, v1.1, ...) so style evolution doesn't break archive comparability.
---

# Infographic Templates — Catalog of Visual Types

The Visual Department produces 5 distinct types of visuals, not free-form pictures. Each type has a defined purpose, trigger, layout, and acceptance criteria. This skill is the catalog.

## Prerequisites

- `visual-grammar` loaded (defines brand colors, typography, grid)

## Core principle

> Free-form visuals don't accumulate value; templated visuals do. When every OSP Analysis is rendered with the same template, you can compare 50 analyses at a glance and see patterns. When every Expert Briefing has combat questions in the same position, you can scan a stack of briefings quickly. Templates ARE the value of the visual archive.

## The 5 templates — full reference

### Type 1: Compression Visual

**Purpose:** convert multi-page report into one image readable in 5-10 seconds.

**Trigger:** `/visual compression <source_type> <source_id>` or auto for deepexpert / full OSP analysis.

**Canvas:** 1080 × 1350 (portrait, mobile-friendly).

**Required sections (top to bottom):**
1. **Header band** (1080×120) — source title, date, source ID, type icon
2. **Hero metric** (1080×200) — single most important fact, large data number with caption
3. **4-6 content panels** (split grid) — each covers one aspect of source
4. **Footer band** (1080×80) — confidence level, provenance, hashtags

**Generation method:** Claude generates SVG via `claude_svg_generation`. Uses `buildCompressionPrompt`.

**Time:** 10-30 seconds.
**Cost:** ~5-10K Claude tokens.

**Acceptance criteria:**
- Entire source content represented (not selective excerpt)
- Hero metric is genuinely the most important
- All panels balanced (no panel dramatically larger than others)
- Brand grammar compliant

### Type 2: Memorable Infographic

**Purpose:** apply modern best-practices to make the key insight stick in memory.

**Trigger:** `/visual memorable <source_id>` — explicit request only (not auto). Optional `focus_insight` parameter centerpoints one specific insight.

**Canvas:** 1080 × 1350.

**Required techniques applied:**
- Golden ratio proportions (1.618:1) for key composition
- F-pattern or Z-pattern reading flow
- Color psychology applied semantically (gold/burgundy/forest per meaning)
- Visual hierarchy through size + contrast
- Emotional hook: scale comparison, surprise, or visual metaphor
- Gestalt principles: proximity, similarity, closure
- Negative space as structural element

**Generation:** Claude generates SVG with strong design discipline. Uses `buildMemorablePrompt`.

**Time:** 30-60 seconds.
**Cost:** ~10-20K tokens.

**Acceptance criteria:**
- One central insight clearly dominates the visual
- All elements support that insight (test: remove any element — is it weaker?)
- Scale comparison or visual metaphor used effectively
- Passes Visual Recall Test 30 days later

### Type 3: Analysis Card

**Purpose:** systematized card per analysis type for side-by-side comparison.

**Trigger:** **AUTOMATIC** for every `/analyze`, `/expert`, `/scan` completion. Most frequent template usage.

**Canvas:** 1080 × 1080 (square, balanced).

**Specific templates:**

**`osp_analysis_card`:**
```
Header (1080×120):        topic | date | confidence
5-level topology (1080×600): leaves / branches / stem / roots / fruits panels
Scenarios (540×280):       sorted by probability, top 3
Verification (540×280):    watch date, indicators count
Footer (1080×80):          id, hashtags
```

**`expert_briefing_card`:**
```
Header (1080×120):         topic | meeting date | depth level
Combat questions (1080×400): top 5 questions, numbered
Failure modes (540×400):    top 3 with severity color coding
Red flags (540×400):        manipulation patterns identified
Footer (1080×160):          understanding score (if debriefed) | hashtags
```

**`lz_artifact_card`:**
```
Header (1080×120):          topic | status | last scanned
Hype cycle position (540×400): visual gauge on Gartner curve
Disruption gauge (540×400):  probability % + timeline
Key findings (1080×480):    bullet list of top findings
Footer (1080×80):           inflection flag | hashtags
```

**Generation:** template-driven via `renderCardFromTemplate`. Loads template spec from `VisualTemplate.spec`, fills fields from source data, sends to Claude for SVG rendering with strict layout discipline.

**Time:** 2-5 seconds (templated).
**Cost:** ~2-5K tokens.

**Acceptance criteria:**
- Same template_version produces visually identical layouts (only data differs)
- All required template fields populated (no missing data sections)
- Side-by-side comparable with other cards of same type
- Brand grammar compliant

**This is the MOST important type** — covers all daily analyses.

### Type 4: Reference Sheet

**Purpose:** dense one-page reference card for product/project/system/company/technology.

**Trigger:** `/visual reference <entity_type> <id>` where entity_type ∈ {client, project, artifact}.

**Canvas:** 1200 × 900 (landscape, reference-card format).

**Specific templates:**

**`client_reference_sheet`:**
```
Header (1200×100):         name | company | tier | status
Contact panel (400×300):   email | phone | last contact
Financial panel (400×300): total revenue | active invoices | overdue
Projects panel (400×300):  active count | completed count | upcoming
Timeline (1200×400):       last 10 events compressed
Footer (1200×100):         last enrichment | tags
```

**`project_reference_sheet`:**
```
Header (1200×100):         name | type | status | progress
Team panel (400×300):      team members | roles
Deadlines panel (400×300): deadline | days remaining | task count
Client panel (400×300):    client | annotation
Tasks panel (1200×400):    open tasks sorted by priority
Footer (1200×100):         created | last update
```

**`technology_reference_sheet`** (from KnowledgeArtifact):
```
Header (1200×100):         technology name | category | status
Hype position (400×300):   Gartner curve with marker
Disruption (400×300):      probability | timeline | velocity
Key players (400×300):     top companies / labs
Profile (1200×400):        compressed technology profile
Footer (1200×100):         last scan | watchlist status
```

**Generation:** Claude generates SVG using aggregated entity data. Uses `buildReferenceSheetPrompt`.

**Time:** 30-60 seconds.
**Cost:** ~10-20K tokens.

**Acceptance criteria:**
- All key params present (entity should be fully understood from sheet alone)
- Dense but scannable (use whitespace to separate, not crowd)
- Print-ready (PDF export readable on A4)
- Numbers monospaced for column alignment

### Type 5: Premium Spec

**Purpose:** JSON specification for owner to produce publication-quality infographic manually using Midjourney + Figma.

**Trigger:** `/visual-spec <source_id>` — explicit request only.

**Output:** NOT an image. A structured JSON spec.

**Required spec sections:**
```json
{
  "title": "...",
  "subtitle": "...",
  "format": "portrait|landscape|square",
  "dimensions": "1080×1350|1200×900|1080×1080",
  "midjourney_base_prompt": "full MJ v7 prompt",
  "midjourney_aspect_ratio": "--ar 4:5",
  "midjourney_style_params": "--style raw --v 7",
  "color_palette": ["#hex1", "#hex2", "#hex3"],
  "text_overlays": [
    {
      "type": "headline|callout|label|annotation|caption|data_number",
      "text": "...",
      "position": "top-left|center|etc",
      "approximate_coordinates": {"x_percent": 50, "y_percent": 10},
      "font_family": "Inter Tight|Inter|IBM Plex Mono",
      "font_size_pt": 36,
      "font_weight": "regular|medium|bold",
      "color": "#hex"
    }
  ],
  "data_visualizations": [...],
  "callouts_and_lines": [...],
  "production_notes": ["specific instruction 1", "..."],
  "estimated_production_time_minutes": 90,
  "reference_examples": ["BMW blueprint", "SR-71 overview", "..."]
}
```

**Generation:** Claude generates spec. Uses `buildPremiumSpecPrompt`.

**Time:** 1-2 minutes for spec. Then 1-2 hours owner's manual work.

**Cost:** ~5-10K Claude tokens + Midjourney subscription ($30/mo) + Figma (free).

**Acceptance criteria:**
- Spec is executable without further questions (owner can produce visual following spec alone)
- Reference examples cited so owner has visual benchmark
- Specific Midjourney prompt provided (not "describe a fusion reactor")
- Text overlays positioned precisely (percentages, not vague "near top")

## Template versioning

Each template has `currentVersion` field (e.g., "v1.0", "v1.1").

When `quarterlyStyleEvolution` updates a template:
- Increment version (v1.0 → v1.1)
- Add entry to `versionHistory` with changes + date + reason
- All NEW visuals use new version
- Existing VisualArtifact records retain `templateVersion` field — preserve historical record

This way, comparing two Analysis cards from different quarters shows BOTH style evolution AND data difference — you can see how methodology evolved.

## When to update template version (rules)

Update version when:
- Recall test rate dropped >15% — template not working
- Pattern scan found significantly better approach (with evidence)
- Source data structure changed (e.g., new OSP V3 field needs to appear)
- Owner explicitly requests change

Do NOT update version for:
- Minor color tweaks ("a bit darker") — that's brand grammar update, separate process
- "Looks better" preference without evidence
- Single experimental visual want — use Type 5 spec instead

## Anti-patterns

- **Creating new types ad-hoc.** Stick to 5 types. If you think you need a 6th, propose to owner with rationale. Don't just create.
- **Mixing types in one visual.** A card that's also a reference sheet that's also memorable — chooses none well. Pick one type.
- **Bumping version for every change.** Versions should be quarterly events, not weekly. Continuous tweaking destroys archive comparability.
- **Templates without data validation.** If template expects `topology.leaves` but source doesn't have it, render fails silently. Add validation.
- **Forgetting `templateVersion` field.** Without it, can't analyze style evolution effects.

## Integration

- Used by `lib/visual.js` — every generator function loads template, applies its spec
- Tested by `style-evolution-discipline` — quarterly review checks each template's effectiveness
- Extended by `data-to-visual-mapping` — maps source fields to template slots
- Constrained by `visual-grammar` — templates must comply with brand grammar

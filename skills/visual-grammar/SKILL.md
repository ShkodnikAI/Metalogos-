---
name: visual-grammar
description: Foundational visual grammar for Fosved Visual Department. Defines the brand identity (palette, typography, grid, iconography) and the rules that apply to ALL generated visuals regardless of type. The "law of style" — without this, visuals lack consistency and lose comparability across the archive.
---

# Visual Grammar — Foundation of All Visuals

Every visual in Fosved Office speaks the same visual language. This is not aesthetic preference — this is **systematic discipline**. When two analyses look comparable (same proportions, same color meaning, same hierarchy logic), the brain can compare them. When each looks different, comparison is impossible and the archive loses value.

This skill defines the visual law. Every other Visual Department skill applies it.

## Prerequisites

- None — this is the foundational skill, loaded first

## Core principle

> Visual consistency is the substrate of visual comparability. If you can compare two analyses side-by-side without re-orienting, the visual grammar is doing its job. Comparison only works when style differences = data differences.

## The Brand Identity (immutable)

### Color palette (use ONLY these)

```
Primary navy     #1a1a2e  — dark backgrounds, headers, structural elements
Primary cream    #f5f1e8  — light backgrounds, body areas
Accent gold      #c9a961  — important data, hero metrics, highlighted insights
Accent burgundy  #8b2942  — risks, negative signals, alerts, failures
Accent forest    #2d5f3f  — positive signals, success, growth, validation
Neutral grey     #5a5a5a  — secondary text, captions
Light grey       #e0ddd5  — dividers, table backgrounds, subtle separators
```

**Color meaning is FIXED across all visuals:**
- Gold = THIS is important, look here
- Burgundy = risk, alert, warning
- Forest = positive, success, on-track
- Navy = structure, framework, container
- Cream = canvas, breathing room

**Violations forbidden:**
- Burgundy used decoratively (not as alert) — confuses semantics
- Gold used everywhere (loses meaning if overused — max 3 gold elements per visual)
- Introducing new colors not in palette
- Tinting/shading the brand colors arbitrarily

### Typography hierarchy

```
H1 (main title):       Inter Tight 700 / 56px / -0.02 letter-spacing
H2 (section header):   Inter Tight 700 / 36px / -0.01 letter-spacing
H3 (subsection):       Inter 600 / 24px / 0
Body:                  Inter 400 / 16px / 0
Data (numbers):        IBM Plex Mono 500 / variable / 0
  Large data hero:     IBM Plex Mono 500 / 48px
  Medium data:         IBM Plex Mono 500 / 28px
  Small data inline:   IBM Plex Mono 500 / 18px
Caption:               Inter 400 italic / 12px
```

**Why Mono for data:** numbers in monospace align perfectly in tables and lists, making comparison trivial. Body text in proportional (Inter) for natural reading.

**Discipline:**
- Same role = same style. A scenario probability is always 18px IBM Plex Mono regardless of which visual it's in.
- No "let me make this one bigger for emphasis" — emphasis is via position and color, not size override.

### Grid system

12-column grid with 24px gutters on canvas with 64px outer margins.

For canvas 1080×1080 (Tier 3 card):
- Outer margin 64px → working area 952×952
- 12 columns × ~71px + 11 gutters × 24px
- Use column counts: 1col, 2col, 3col (typical), 4col, 6col, 12col (full width)

**All elements snap to the grid.** Free-floating elements forbidden.

### Iconography

**Style:** Lucide line icons (open-source, 2px stroke, 24×24 base size).

**Why:** consistent line weight across all icons, recognizable visual family, royalty-free.

**Usage:**
- Always monochromatic in brand palette (typically navy for neutral, gold/burgundy/forest for semantic)
- Same size in same context (all section icons same size; all bullet icons same size)
- Icons SUPPORT text labels, do not replace them

**Forbidden:**
- Filled icons mixed with line icons
- Multi-color icons
- Skeumorphic icons (3D, gradients, shadows)
- Decorative icons without informational purpose

### Spacing scale

Use multiples of 8px for all spacing:
```
8, 16, 24, 32, 40, 48, 64, 80, 96, 128, 160
```

**No arbitrary values** (like 22px or 35px). Forces alignment, prevents drift.

### Border radius

```
0px   — for structural panels, data tables
4px   — for inline tags, buttons
8px   — for cards, badges
```

**Forbidden:** mixing radii in same visual; pill-shaped (100%) radii (looks toy-like).

## Universal layout principles

These apply to every visual:

### 1. F-pattern reading flow (top to bottom, left to right)

Most important element top-left. Eye scans:
- Horizontal sweep at top (headers, key metric)
- Vertical sweep down left edge (section markers)
- Horizontal sweeps shortening as eye descends

Design accordingly. **Never** put critical info bottom-right of an unscanned visual.

### 2. Visual hierarchy through SIZE and CONTRAST, not bold

- Largest element = most important
- Highest contrast = most attention-grabbing
- Bold is NOT a hierarchy mechanism (everything is one weight per role)

### 3. Negative space as structure

Empty space separates sections. Not decoration. **Crowded ≠ informative.**

Rule: at minimum 48px gap between distinct sections. At minimum 24px between related items within a section.

### 4. Anti-chartjunk (Tufte)

Every pixel must carry informational weight. Forbidden:
- Background patterns
- Drop shadows on text
- 3D effects on flat data
- Gradients that don't encode information
- Borders around things that are already separated by space
- Icons that just duplicate labels

### 5. Data-ink ratio maximized

Visualize data, not visualization itself. If you can remove a graphical element without losing information, remove it.

### 6. Glanceable hierarchy

Most important fact = readable in 2 seconds without focus.
Important supporting = readable in 5 seconds with focus.
Detail = readable in 10+ seconds with attention.

## Brand application checklist (run on every visual)

Before publishing any visual, verify:

- [ ] All colors used are from brand palette
- [ ] Color semantics consistent (gold/burgundy/forest used per meaning)
- [ ] Typography uses only specified fonts at specified sizes
- [ ] Elements snap to 12-column grid with 64px outer margins
- [ ] Icons all same style (Lucide line, 2px stroke)
- [ ] Spacing uses 8px scale multiples
- [ ] Hierarchy clear via size+contrast, not bold tricks
- [ ] No chartjunk elements
- [ ] F-pattern reading flow respected
- [ ] At minimum 48px gap between sections

If any fails — fix before publishing.

## Anti-patterns

- **"This visual needs a special color" temptation.** No it doesn't. If you need a new color to express something, you're encoding meaning ad-hoc instead of using established semantics.
- **"Just one decorative element" temptation.** Decoration kills data-ink ratio. Reject every decoration request.
- **Size as emphasis.** Bolding to highlight breaks hierarchy. Use position and gold color to highlight.
- **Tight spacing for "more info".** Crowding makes nothing readable. Less items, properly spaced > more items crammed.
- **Mixing fonts.** If you ever use a font outside Inter/IBM Plex Mono — you broke the grammar. Don't.
- **Free-floating elements.** Everything is on the grid. Off-grid placement looks amateur and breaks comparability.

## Integration with other Visual skills

This skill is **always loaded** when Visual Department activates. Every other skill assumes its rules. Specifically:

- `infographic-templates` extends grammar with type-specific layouts
- `svg-generation` enforces grammar at SVG-output level
- `data-to-visual-mapping` uses semantic colors per data type
- `style-evolution-discipline` can update template versions but **brand grammar is more conservative** — updates require explicit owner approval, not just quarterly review

If a visual violates grammar — that visual is rejected by quality gate regardless of other merits.

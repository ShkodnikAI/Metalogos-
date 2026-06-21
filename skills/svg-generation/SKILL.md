---
name: svg-generation
description: Techniques for generating SVG via Claude — when to use raw SVG vs Mermaid vs Excalidraw JSON. SVG output validation rules. Common patterns (responsive viewBox, accessibility attributes, font embedding). Anti-patterns specific to Claude SVG generation.
---

# SVG Generation — Production Techniques

The Visual Department outputs SVG by default (vector, scales, browser-renderable, Telegram-attachable, archivable as text). This skill covers production-quality SVG generation through Claude.

## Prerequisites

- `visual-grammar` loaded
- `infographic-templates` understood
- Source data structured for the visual

## Core principle

> SVG is text. Claude generates text. The combination is powerful — but Claude can produce invalid SVG, broken layouts, or off-brand styling. Discipline: structured prompts with explicit constraints, validation after generation, fallback strategies for failures.

## When to use what

**Raw SVG generation via Claude:**
- All Type 1 (compression visuals) — custom layouts per source
- All Type 2 (memorable infographics) — bespoke designs
- All Type 4 (reference sheets) — complex multi-panel layouts
- Type 3 cards rendered via template (template provides structure, Claude renders SVG)

**Mermaid syntax → SVG via kroki.io or mermaid.ink:**
- Quick flowcharts within textual responses
- Sequence diagrams for process flows
- Gantt charts for timelines
- State machines

**Excalidraw JSON:**
- Hand-drawn-feel diagrams for casual contexts
- Rarely used in Visual Department (off-brand, breaks formality)

**For Visual Department production, default to raw SVG generation.**

## SVG generation prompt structure

Every SVG generation prompt must include:

```
[CONTEXT]
What is being rendered, what type, what source.

[CONSTRAINTS]
- Brand palette (exact hex values)
- Typography rules (families + sizes)
- Canvas dimensions (viewBox)
- Grid alignment requirements
- Anti-chartjunk discipline

[SOURCE DATA]
JSON or structured representation of data to render.

[LAYOUT SPEC]
Either template spec or explicit layout description.

[OUTPUT REQUIREMENT]
Respond with ONLY the SVG code, no markdown wrappers, no preamble.
Start with <svg viewBox="0 0 W H" xmlns="http://www.w3.org/2000/svg">.
```

This structure is built into `lib/visual.js` prompt builders. Don't deviate ad-hoc.

## Required SVG attributes

Every generated SVG must have:

```xml
<svg viewBox="0 0 WIDTH HEIGHT"
     xmlns="http://www.w3.org/2000/svg"
     role="img"
     aria-label="<description for accessibility>">
  ...
</svg>
```

- `viewBox` (responsive scaling)
- `xmlns` (namespace declaration, required)
- `role="img"` (accessibility)
- `aria-label` (description for screen readers + AI indexing)

## Font handling

SVG has tricky font support. Approach:

**For inline rendering (Telegram, archive):**
Use `font-family` attribute with fallback chain:
```xml
<text font-family="Inter Tight, Inter, system-ui, -apple-system, sans-serif">
```

This guarantees readable text even if Inter not available.

**For numbers (mono font):**
```xml
<text font-family="IBM Plex Mono, Menlo, Consolas, monospace">
```

**Don't embed fonts as @font-face** in production SVG — file bloat. Rely on fallback chain.

## Color usage in SVG

Always use exact hex from brand palette:

```xml
<rect fill="#1a1a2e" />        <!-- primaryNavy -->
<text fill="#c9a961">42%</text> <!-- accentGold for important data -->
```

**Forbidden:**
- CSS variables in SVG (compatibility issues)
- Computed colors (hsl, rgb without explicit values)
- Color names ("blue", "gold") — imprecise

## Validation after generation

After Claude produces SVG, validate:

```javascript
function validateSvg(svg) {
  if (!svg) return { valid: false, reason: 'empty' };
  if (!svg.startsWith('<svg')) return { valid: false, reason: 'no svg tag' };
  if (!svg.endsWith('</svg>')) return { valid: false, reason: 'no closing svg' };
  if (!svg.includes('viewBox=')) return { valid: false, reason: 'no viewBox' };
  if (!svg.includes('xmlns="http://www.w3.org/2000/svg"')) {
    return { valid: false, reason: 'no xmlns' };
  }

  // Brand compliance check
  const allowedColors = ['#1a1a2e', '#f5f1e8', '#c9a961', '#8b2942',
                          '#2d5f3f', '#5a5a5a', '#e0ddd5', 'none', 'transparent'];
  const colorMatches = svg.match(/(?:fill|stroke)="(#[0-9a-fA-F]{3,6})"/g) || [];
  const usedColors = new Set(colorMatches.map(m => m.match(/#[0-9a-fA-F]+/)[0].toLowerCase()));
  const offBrand = [...usedColors].filter(c => !allowedColors.includes(c));
  if (offBrand.length > 0) {
    return { valid: false, reason: `off-brand colors: ${offBrand.join(', ')}` };
  }

  return { valid: true };
}
```

Validation failure handling:
- Log specific reason
- Retry generation with explicit error feedback to Claude
- Max 2 retries, then return null (fallback to template default or skip)

## Patterns for common visual elements

### Headlines

```xml
<text x="64" y="80" font-family="Inter Tight, system-ui, sans-serif"
      font-size="56" font-weight="700" fill="#1a1a2e">
  Topic Name
</text>
```

### Hero data number

```xml
<text x="540" y="320" text-anchor="middle"
      font-family="IBM Plex Mono, monospace"
      font-size="120" font-weight="500" fill="#c9a961">
  65%
</text>
<text x="540" y="380" text-anchor="middle"
      font-family="Inter, sans-serif" font-size="16" fill="#5a5a5a">
  probability of devaluation in 6 months
</text>
```

### Sorted bar chart

```xml
<g transform="translate(64, 600)">
  <!-- Bar 1 (largest) -->
  <rect x="0" y="0" width="600" height="32" fill="#c9a961" />
  <text x="610" y="22" font-family="IBM Plex Mono" font-size="18" fill="#1a1a2e">65%</text>
  <text x="0" y="-8" font-family="Inter" font-size="14" fill="#5a5a5a">Scenario A</text>
  <!-- Bar 2 -->
  <rect x="0" y="56" width="240" height="32" fill="#1a1a2e" />
  ...
</g>
```

### Gauge (radial probability)

```xml
<g transform="translate(540, 540)">
  <!-- Background arc -->
  <circle r="120" fill="none" stroke="#e0ddd5" stroke-width="20"
          stroke-dasharray="754" stroke-dashoffset="0" />
  <!-- Filled arc (75%) -->
  <circle r="120" fill="none" stroke="#c9a961" stroke-width="20"
          stroke-dasharray="565 754" transform="rotate(-90)" />
  <!-- Center number -->
  <text text-anchor="middle" dy="0.35em"
        font-family="IBM Plex Mono" font-size="48" font-weight="500" fill="#1a1a2e">75%</text>
</g>
```

### Color-coded panel (severity)

```xml
<!-- Catastrophic = burgundy -->
<g transform="translate(0, 720)">
  <rect width="540" height="200" fill="#f5f1e8" stroke="#8b2942" stroke-width="3" />
  <text x="20" y="40" font-family="Inter Tight" font-size="24" font-weight="700" fill="#8b2942">
    Failure Mode: Tritium Supply
  </text>
  <text x="20" y="70" font-family="Inter" font-size="16" fill="#1a1a2e">
    Catastrophic | Certain
  </text>
  ...
</g>
```

### Timeline (horizontal)

```xml
<g transform="translate(64, 700)">
  <line x1="0" y1="0" x2="952" y2="0" stroke="#5a5a5a" stroke-width="2" />
  <!-- Event marker -->
  <circle cx="120" cy="0" r="8" fill="#c9a961" />
  <text x="120" y="-20" text-anchor="middle" font-family="IBM Plex Mono" font-size="12">
    2026-03
  </text>
  <text x="120" y="30" text-anchor="middle" font-family="Inter" font-size="11" fill="#5a5a5a">
    First contact
  </text>
  ...
</g>
```

## Anti-patterns specific to SVG via Claude

- **Claude generates pretty SVG but wrong dimensions.** Always specify exact viewBox in prompt and validate.
- **Claude uses CSS not allowed in SVG.** Force `fill="..."` attributes, not `style="fill:..."`.
- **Claude invents colors.** Validation catches this; if frequent, sharpen prompt to repeat exact palette.
- **Claude omits xmlns namespace.** Without it, SVG won't render in many contexts. Validation catches.
- **Claude wraps SVG in markdown.** Strip markdown wrappers in extraction (`extractSvgFromResponse` does this).
- **Claude adds <html> or <body>.** Reject and retry — pure SVG only.
- **Inline `<style>` blocks that fail in archival.** Prefer attribute styling.
- **Forgetting accessibility attributes.** Add post-generation if missing.

## Integration

- Used by `lib/visual.js` all generator functions
- `extractSvgFromResponse` strips wrappers, returns clean SVG
- `validateSvg` runs after each generation
- On validation failure: retry once with feedback, then return null

## Storage format

SVG stored as text in `VisualArtifact.content` (PostgreSQL TEXT field).

Average SVG size: 5-30 KB. PostgreSQL handles easily.

For Telegram delivery: send as document with `.svg` extension. Telegram doesn't render SVG inline, but downloads work and most modern viewers display it.

For miniapp gallery: embed directly via `<div dangerouslySetInnerHTML={{__html: artifact.content}}>` (sanitized).

For PNG export (if needed for embedding): server-side render via sharp or canvas library — not in v1.0 of Visual Department, add if needed later.

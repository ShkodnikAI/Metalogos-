---
name: type-system-application
description: Detailed typographic rules for Visual Department. Goes beyond visual-grammar foundational rules into specifics — kerning, leading, alignment, hierarchy through type only, when to use which weight. Foundation for professional-looking visuals where typography itself carries information.
---

# Type System Application — Typography as Information

In Fosved visuals, typography is not decoration. It is **information encoding** — what's a number vs label vs caption vs annotation is determined by font choice, size, weight, color. Reading the type system is as fast as reading colors.

## Prerequisites

- `visual-grammar` loaded (defines font families + base sizes)

## Core principle

> The viewer should not have to think "what kind of text is this?" — the typographic treatment makes it obvious. Headlines look like headlines. Data looks like data. Captions look like captions. This obviousness comes from disciplined application of the type system, not from clever variations.

## The type system (formal definition)

```
ROLE              FAMILY              SIZE (px)  WEIGHT  COLOR              SPACING
─────────────────────────────────────────────────────────────────────────────────
H1 (main title)   Inter Tight         56         700     #1a1a2e (navy)     -0.02em
H2 (section)      Inter Tight         36         700     #1a1a2e            -0.01em
H3 (subsection)   Inter               24         600     #1a1a2e            0
Body              Inter               16         400     #1a1a2e            0
Caption           Inter               12         400     #5a5a5a (grey)     0    italic
Data large        IBM Plex Mono       48         500     varies             0
Data medium       IBM Plex Mono       28         500     varies             0
Data small        IBM Plex Mono       18         500     varies             0
Data label        Inter               11         500     #5a5a5a            0.05em
Annotation        Inter               13         400     #1a1a2e            0    italic
```

**Color for data values:** based on semantic meaning
- Gold #c9a961 — important / hero data
- Burgundy #8b2942 — negative / risk
- Forest #2d5f3f — positive / success
- Navy #1a1a2e — neutral / structural

## Hierarchy through type

Type alone signals importance — without changing size or color (color is reserved for semantic meaning).

**Three ways type creates hierarchy:**

### 1. Size hierarchy
H1 (56) > H2 (36) > H3 (24) > Body (16) > Caption (12)

Step ratio approximately 1.5×. Each step distinctly larger. NO intermediate sizes (no 32, no 42).

### 2. Weight hierarchy
Bold (700) for titles. Semibold (600) for subsections. Regular (400) for body. Italic for emphasis or captions.

Single weight per role. Don't use "extra bold" for emphasis — emphasis comes from position and color.

### 3. Family hierarchy
Inter Tight — only for titles. Establishes them as titles.
Inter — for everything readable.
IBM Plex Mono — only for data and numbers.

Family choice signals function:
- Tight + bold = title
- Regular sans = read this normally
- Mono = this is a number, scan/compare it

## Practical typesetting rules

### Letter spacing (tracking)

- Headlines (H1, H2): slight negative tracking (-0.01 to -0.02em). Looks tighter, more confident.
- Body: zero tracking (default).
- ALL CAPS labels: positive tracking (+0.05em). Caps need breathing room.
- Mono numbers: zero tracking (monospace handles spacing).

### Line height (leading)

- Headlines: 1.0–1.1× font size (tight, declarative)
- H3: 1.2×
- Body: 1.4× (comfortable reading)
- Captions: 1.3×

In SVG, set via `dy` between text lines or `<tspan>` positioning.

### Alignment

- Left-align for body text (easier to scan)
- Center-align for hero data numbers
- Right-align for data columns (so digits line up)
- Justify ONLY for very long body text blocks (rare in visuals)

### Line length

- Body text: 50-80 characters per line
- Captions: 30-60 characters

Long lines hard to read. Short lines look chopped. Stay in band.

## Numbers — special treatment

Numbers are the most read elements in data-heavy visuals. Special discipline:

### Always mono font

IBM Plex Mono for every number visible to viewer. Why:
- Digit width identical (1 takes same space as 8)
- Decimal points align between rows
- Visual comparison instantaneous

### Decimal precision discipline

- Percentages: 0 or 1 decimal (75% or 64.5%, never 64.523%)
- Money: 2 decimals or 0 ($1,250 or $1,250.00, consistency within visual)
- Probabilities: 2 decimals (0.65 or 65%, pick one and stick)
- Large numbers: use scale ($50M not $50,000,000)

### Sign and positivity convention

- Positive change: + prefix in forest color (+12% in #2d5f3f)
- Negative change: − prefix in burgundy color (−8% in #8b2942)
- No change: 0 or em-dash (—) in grey

### Currency placement

- Symbol before number: $50, €100, ¥3000
- For large amounts: $50M, $1.2B, $500K
- Currency code after for emphasis: 50 USD, 100 EUR (when comparing currencies)

## Captions and annotations

Often-neglected but critical for information density:

### Captions

- Always italic Inter 12pt grey
- 30-60 characters
- Direct relationship to element above/beside (proximity grouping)
- No period at end (it's a label, not a sentence)

### Annotations

- Italic Inter 13pt navy
- Connected to specific element via leader line
- Describes meaning: "first commercial deployment expected here"
- May include a number

### Leader line + annotation pattern

```xml
<g transform="translate(400, 300)">
  <line x1="0" y1="0" x2="80" y2="-40" stroke="#5a5a5a" stroke-width="1" />
  <circle cx="0" cy="0" r="3" fill="#5a5a5a" />
  <text x="85" y="-45" font-family="Inter" font-style="italic" font-size="13" fill="#1a1a2e">
    First commercial deployment
  </text>
  <text x="85" y="-28" font-family="Inter" font-size="11" fill="#5a5a5a">
    expected 2030 ± 2 years
  </text>
</g>
```

Single annotation pattern, reused throughout. Owner reads same way every time.

## Section headers

Section headers separate visual zones:

```
SECTION TITLE  ─────────────────────────────
                                              
Subsection                                    
```

Pattern:
- Title in H3 (Inter 24pt 600 navy)
- Horizontal hairline below or beside title (1pt grey, full width or section-width)
- ~24px below before content starts

Consistent across all visuals → builds recognition.

## Avoiding common typography mistakes

### 1. Don't mix similar sizes
H3 at 24 + H4 at 20 — hard to tell which is hierarchically higher. Stick to defined sizes.

### 2. Don't bold for emphasis
"This number is **important**" — visual hierarchy violated. Use color (gold) for emphasis, or position.

### 3. Don't italic for emphasis
Italic = caption, annotation, definition. Reserved usage. Bold body text creates noise.

### 4. Don't use ALL CAPS for body
ALL CAPS slows reading 13-20% (research). Use only for short labels (3-6 chars).

### 5. Don't decorate headlines
No drop shadows, gradients, decorative fills. Headlines stay flat, navy, declarative.

### 6. Don't use display fonts
Sticking to Inter family + IBM Plex Mono only. Don't introduce Bodoni, Helvetica, etc. — breaks system.

### 7. Don't fight monospace
Numbers in mono = wider than proportional. Don't shrink mono numbers to "fit" — re-layout instead.

## Vertical rhythm

All text positioned on 8px baseline grid:
- 8px gap between caption and element it captions
- 16px between body lines (10px leading)
- 24px between sections
- 32px between major zones
- 48px between distinct visual blocks

Never break vertical rhythm. Maintains alignment across visual.

## Worked example — Expert briefing card

**Top section:**
```
Topic: Compact tokamak FusionCorp investment
H1 (56pt Inter Tight 700 navy):
"FusionCorp Investment Briefing"
H3 below (24pt Inter 600 grey):
"Meeting tomorrow 14:00 · Level 3 deep dive"
```

**Combat questions:**
```
H2 (36pt Inter Tight 700 navy):
"Combat Questions"
─────────────────────────── hairline

[1] Body (16pt Inter 400 navy):
"What is your tritium supply plan for first 5 years?"
Caption (12pt italic grey):
"target: numerical answer with named partner"

[2] Body...
```

**Data hero:**
```
Data large (48pt IBM Plex Mono 500 gold):
"$3.0B"
Data label (11pt Inter 500 grey, +0.05em tracking):
"CLAIMED CAPEX PER GIGAWATT"
Data small (18pt IBM Plex Mono 500 burgundy):
"vs $5–7B industry baseline"
```

**Footer:**
```
Data label (11pt grey):
"BRIEFING ID"
Data small (18pt mono navy):
"#BRF-018"
```

Same structure every briefing → instantaneous recognition.

## Anti-patterns

- **Custom sizes for one visual.** Breaks system; visual feels off without viewer knowing why.
- **Color for hierarchy.** Color is for semantics, not hierarchy. Don't make headlines colorful.
- **Italic for emphasis.** Reserved for captions. Emphasis = position + accent color.
- **Mono for non-data text.** Mono is for numbers. Body in mono = unreadable.
- **Different fonts in same visual.** One sans family (Inter), one mono (IBM Plex Mono). Two families total.
- **Tracking variations.** Either zero, or -0.01–0.02 for tight titles, or +0.05 for caps. Nothing else.

## Integration

- Loaded after `visual-grammar` (extends it)
- Used by every other production skill
- Quality gate at SVG validation checks font-family attributes match approved list
- Enforced in template specs and prompt builders

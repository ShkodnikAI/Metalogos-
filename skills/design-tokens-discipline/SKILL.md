---
name: design-tokens-discipline
description: Design tokens as single source of truth for visual properties. Colors, spacing, typography, radii — every value lives in one place, referenced by name everywhere. Without token discipline, "spacing-md" means 12px here and 14px there. With it, design and code stay synchronized.
---

# Design Tokens Discipline — Single Source of Truth

A design token is a named value: `color-primary-500 = #1a1a2e`. Every component references the token, not the value directly. Change the token = change everywhere. This is the discipline that keeps design and code consistent.

## Prerequisites

- `design-system-thinking` understood
- Project's token system defined (or using Fosved brand as default)

## Core principle

> Never use raw values in components. Always use tokens. The token name encodes intent (`color-error-bg` says WHY, not just WHAT). Maintenance and consistency depend on this discipline. Five minutes of token reference saves hours of search-and-replace later.

## Token categories

### Color tokens

**Primitive tier (literal colors):**
```
color-navy-900: #1a1a2e
color-navy-700: #2d2d4a
color-cream-100: #f5f1e8
color-gold-500: #c9a961
color-burgundy-600: #8b2942
color-forest-700: #2d5f3f
color-gray-500: #5a5a5a
color-gray-100: #e0ddd5
```

**Semantic tier (role-based):**
```
color-text-primary: → color-navy-900
color-text-secondary: → color-gray-500
color-text-on-dark: → color-cream-100
color-bg-primary: → color-cream-100
color-bg-card: → white
color-border-default: → color-gray-100
color-action-primary: → color-navy-900
color-action-destructive: → color-burgundy-600
color-status-success: → color-forest-700
color-status-warning: → color-gold-500
color-status-error: → color-burgundy-600
color-status-info: → color-navy-700
color-accent: → color-gold-500
```

**Components reference semantic tier, not primitive.** Why: rebrand by reassigning semantic. Don't touch components.

### Typography tokens

```
font-family-display: "Inter Tight", system-ui, sans-serif
font-family-body: "Inter", system-ui, sans-serif
font-family-mono: "IBM Plex Mono", monospace

font-size-xs: 12px
font-size-sm: 14px
font-size-base: 16px
font-size-lg: 18px
font-size-xl: 20px
font-size-2xl: 24px
font-size-3xl: 32px
font-size-4xl: 40px
font-size-5xl: 56px

font-weight-regular: 400
font-weight-medium: 500
font-weight-semibold: 600
font-weight-bold: 700

line-height-tight: 1.1
line-height-snug: 1.25
line-height-normal: 1.5
line-height-relaxed: 1.75

letter-spacing-tight: -0.02em
letter-spacing-normal: 0
letter-spacing-wide: 0.05em
```

Semantic typography tokens build on these:
```
text-h1: font-family-display + font-size-5xl + font-weight-bold + line-height-tight + letter-spacing-tight
text-h2: font-family-display + font-size-3xl + font-weight-bold + line-height-snug
text-body: font-family-body + font-size-base + font-weight-regular + line-height-normal
text-caption: font-family-body + font-size-xs + font-style-italic
text-data: font-family-mono + font-size-base + font-weight-medium
```

### Spacing tokens (8px scale)

```
space-0: 0
space-1: 4px       (sub-base, use sparingly)
space-2: 8px       (base unit)
space-3: 12px      (half-step, use sparingly)
space-4: 16px
space-6: 24px
space-8: 32px
space-12: 48px
space-16: 64px
space-24: 96px
space-32: 128px
```

**Stick to even multiples of 8 typically.** Half-steps (4, 12, 20) only when 8-grid creates visual problems.

**Never use raw `px` for spacing in components.** Always `var(--space-N)`.

### Border radius tokens

```
radius-none: 0
radius-sm: 4px
radius-md: 8px
radius-lg: 16px
radius-full: 9999px (for pills, circles)
```

### Shadow tokens

```
shadow-none: none
shadow-sm: 0 1px 2px rgba(0,0,0,0.05)
shadow-md: 0 4px 6px rgba(0,0,0,0.1)
shadow-lg: 0 10px 15px rgba(0,0,0,0.1)
shadow-xl: 0 20px 25px rgba(0,0,0,0.15)
```

### Animation tokens

```
duration-fast: 150ms
duration-base: 250ms
duration-slow: 400ms

easing-default: cubic-bezier(0.4, 0, 0.2, 1)
easing-in: cubic-bezier(0.4, 0, 1, 1)
easing-out: cubic-bezier(0, 0, 0.2, 1)
```

### Z-index tokens

```
z-base: 0
z-dropdown: 100
z-sticky: 200
z-overlay: 300
z-modal: 400
z-popover: 500
z-toast: 600
```

Predefined z-index scale prevents "z-index: 9999" arms race.

## Token implementation

**CSS Variables:**
```css
:root {
  --color-navy-900: #1a1a2e;
  --color-text-primary: var(--color-navy-900);
  --space-4: 16px;
  /* ... */
}
```

**Tailwind config** (extending or overriding):
```javascript
// tailwind.config.ts
module.exports = {
  theme: {
    colors: {
      'navy-900': '#1a1a2e',
      'cream-100': '#f5f1e8',
      // ...
    },
    extend: {
      spacing: {
        // 8px scale
      }
    }
  }
};
```

For Fosved: Tailwind 4 with Fosved tokens extended.

## Naming conventions

**Token name describes intent, not appearance.**

Good:
- `color-text-primary` (what role)
- `color-action-destructive` (what action)
- `space-component-padding` (what use)

Bad:
- `color-dark` (describes appearance, not meaning)
- `space-medium` (relative without anchor)
- `red-color` (color name describes value)

Why intent over appearance: appearance can change in rebrand; intent stays.

## Token tiers and updates

Three tiers, different update rates:

**Primitive (rarely change):** colors palette, type stack. Brand-level decisions. Change once a year or less.

**Semantic (occasionally change):** which primitive plays which role. Refresh per quarter or design phase.

**Component-specific (frequently change):** local overrides for specific components. Day-to-day work.

When updating tokens:
- Primitive change → cascade through semantic + components
- Test broadly (visual regression)
- Bump design system version
- Document in CHANGELOG

## Tools

**Style Dictionary** (Amazon): cross-platform token system. Export to CSS, JS, iOS, Android.

**Tokens Studio** (Figma plugin): manage tokens in Figma, sync to code.

**Design tokens W3C draft spec:** emerging standard for token interchange.

For Fosved: keep simple. CSS variables in code + Tailwind config + documented in design system docs. Don't introduce Style Dictionary until justified by scale.

## Token usage discipline

**In components:**

Bad:
```tsx
<button style={{ padding: '12px', backgroundColor: '#c9a961' }}>
```

Good:
```tsx
<button className="px-3 py-2 bg-accent-gold">  // Tailwind classes mapping to tokens
```

Or:
```tsx
<button style={{ padding: 'var(--space-3)', backgroundColor: 'var(--color-accent)' }}>
```

**In design specs:** "padding: 8px (token: space-2)" — both value and token name. Dev knows what to use.

## When to introduce new token

Often: no.

If using existing token is awkward, ask:
- Is this a one-off? → Use closest existing token, accept slight imperfection.
- Is this a pattern? → New token, used 3+ places.

Avoid token proliferation. 200 tokens unmaintainable.

## Token documentation

For each token, document:
- Token name
- Current value
- Intent / role / when to use
- Anti-pattern / when NOT to use
- Visual sample
- Code reference

Living in design system docs.

## Audit existing usage

Periodically scan codebase for raw values:

```bash
# Find hardcoded color hex
grep -rn '#[0-9a-fA-F]\{6\}' src/

# Find hardcoded pixel values
grep -rn '[^-]\b[0-9]\+px' src/components/
```

Replace findings with token references. Run as monthly Dev hygiene.

## Anti-patterns

- **Raw values in components.** `padding: 12px` everywhere. Inconsistent, hard to refresh.
- **Token names = values.** `color-red-500` everywhere instead of `color-error`. Loses meaning, can't refactor.
- **Token proliferation.** 500 tokens for small app. Adds noise without signal.
- **Conflicting tokens.** `space-md` and `space-medium` mean different things. Pick one name.
- **Stale tokens.** Token defined, never used, never removed.
- **Off-scale values.** `padding: 11px` because "8 too small, 16 too big". Find way to use scale.
- **No version control on tokens.** Token changes silent, break design unexpectedly.
- **Component overrides cascading.** Token at component level overriding system. Document if intentional.
- **Tokens in code that disagree with Figma.** Designer in Figma uses different values than implemented. Sync.
- **No semantic tier.** Only primitive tier. Rebrand becomes find-and-replace nightmare.

## Integration

- Foundation for `design-system-thinking`
- Used by every component (`component-library-development`)
- Referenced in `dev-handoff-specs` (dev knows tokens to use)
- `wireframe-production` uses tokens for visual properties
- `responsive-design` uses tokens for breakpoints
- Token changes require ADR if breaking

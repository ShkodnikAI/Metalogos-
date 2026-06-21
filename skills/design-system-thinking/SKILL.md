---
name: design-system-thinking
description: Designing in terms of reusable systems rather than one-off screens. Each new component thinks about reuse, composition, consistency. Without system thinking, every project rebuilds the same wheels poorly. With it, the second project is 5x faster than the first.
---

# Design System Thinking — Components, Not Pixels

A designer who thinks in pixels for each screen produces beautiful inconsistent interfaces. A designer who thinks in systems produces consistent reusable interfaces. The system thinking is what makes designs scale.

## Prerequisites

- `user-task-analysis` completed for current design
- `design-tokens-discipline` understood
- Project type defined (uses existing system or creates new)

## Core principle

> Every design contributes to a system, not just a screen. Components are designed for reuse, even when needed for one place. The investment compounds: the third button is free; the tenth is built on prior. Without system thinking, each design is from scratch.

## Three-layer system

**Layer 1: Design tokens.** Atomic values.
- Colors (palette, semantic colors)
- Typography (sizes, weights, line heights)
- Spacing (8px scale: 8, 16, 24, 32, 48, 64)
- Border radii (0, 4, 8, 16, full)
- Shadows
- Z-indices
- Animation timings

**Layer 2: Components.** Primitives composed from tokens.
- Button (uses color, padding, radius, typography tokens)
- Input
- Card
- Modal
- Dropdown
- etc.

**Layer 3: Patterns.** Components composed for common use cases.
- Login form (input + button + link)
- Settings page (sidebar + content area)
- Data table (rows + filters + pagination)
- Confirmation flow (modal + buttons + state)

Design at the right layer:
- New color → token
- New widget → component
- New layout combining widgets → pattern

## Component design principles

### Composable

Components combine to make patterns. Each component does one thing well.

Bad: `<UserCardWithEditButtonAndDeleteAndStatusBadge>` — monolithic.
Good: `<Card>` + `<Button>` + `<Badge>` — composable.

### States explicit

Every interactive component has states:
- Default
- Hover
- Focus
- Active (pressed)
- Disabled
- Loading
- Error

All seven specified, even if some look similar.

### Variants are properties

Different sizes or types: variants of one component.
```
<Button size="sm|md|lg" variant="primary|secondary|destructive" />
```

Don't create `SmallButton`, `LargeButton`, `PrimaryButton` separately. Variants of one Button.

### Predictable

Same component used different places looks same and behaves same. No "I made this button slightly different here because it felt right" — undermines the system.

### Documented

Each component has:
- Purpose (when to use)
- Variants
- States
- Props
- Usage examples
- Anti-patterns (when NOT to use)

## Token taxonomy

**Color tokens, two levels:**

Primitive tokens:
- `color-blue-500`, `color-red-500`, `color-gray-100`
- These describe color literally

Semantic tokens:
- `color-text-primary`, `color-text-error`, `color-background-card`, `color-border-default`
- These describe role/meaning

**Use semantic tokens in components.** When you want to redesign, change semantic mapping, not every component.

For Fosved brand: defined in lib/visual.js `BRAND.colors`. Reused across visual artifacts and any custom design.

**Typography tokens:**
- `font-size-xs|sm|base|lg|xl|2xl|3xl`
- `font-weight-regular|medium|bold`
- `line-height-tight|base|relaxed`

**Spacing tokens (Fosved standard: 8px scale):**
- `space-1` = 8px
- `space-2` = 16px
- `space-3` = 24px
- `space-4` = 32px
- `space-6` = 48px
- `space-8` = 64px

Never inline `padding: 12px` — use a token. 12px is off-scale, breaks consistency.

## Component variants matrix

For each component, define variants:

```
Button variants:
- Size: sm, md, lg
- Style: primary, secondary, destructive, ghost
- State: default, hover, focus, active, disabled, loading

Total combinations: 3 × 4 × 6 = 72
Designed states: all
Implemented states: all
```

Decide explicitly. Don't implement subset of variants — looks complete then breaks when missing variant requested.

## When to create new component vs reuse

**Reuse existing:**
- Same purpose
- Same general shape
- Differences expressible as variants

**Create new:**
- Genuinely new purpose (e.g., progress indicator when no progress component exists)
- Cannot express variation as variants (e.g., shape fundamentally different)

**Bad:** create `Button2` because existing Button "doesn't fit". Either modify Button (add variant) or use Button differently. Don't multiply.

## Design system documentation

Every system has:

**Component pages:**
- Component name
- What it is (1 paragraph)
- When to use (bullets)
- When NOT to use (bullets — important!)
- Anatomy diagram (parts labeled)
- Variants (visual matrix)
- States (visual matrix)
- Props (developer reference)
- Code example (copy-pasteable)
- Accessibility notes
- Related components

**Pattern pages:**
- Pattern name
- Use case
- Required components
- Layout structure
- Variations
- Anti-patterns

**Token reference:**
- Color tokens (visual swatches with hex)
- Typography (rendered samples)
- Spacing (visual scale)
- Other tokens

**Voice and tone:**
- Writing style guide
- Microcopy patterns
- Error message conventions

Stored as Markdown in `docs/design-system/` in the project repo, or as separate documentation site for shared systems.

## Living vs static systems

**Static system:** designed once, frozen, used everywhere. Bad — world changes, system rots.

**Living system:** evolved deliberately, versioned, deprecations explicit. Good — but requires governance.

For Fosved:
- Quarterly review (per `library/design.md`)
- Component additions logged in CHANGELOG
- Breaking changes versioned (`button-v2`)
- Deprecations announced before removal

## Adoption strategy

**Greenfield project:** start with design system from day 1. Even minimal one (5 components) better than nothing.

**Existing project:** introduce incrementally. New code uses system. Refactor old code opportunistically. Don't big-bang migrate.

For Fosved Office: brand identity already defined (in Visual Department). Design Department extends with UI components built on that foundation.

## Shared vs project-specific

**Shared design system:**
- For client projects in same brand (Fosved's own)
- Lives in dedicated repo
- Versioned, published as package

**Project-specific:**
- For one-off client projects with their own brand
- Lives in project repo
- May reference shared system but customize

Decision: is this component going to be used in >1 project? If yes, shared. Else, project-specific.

## Anti-patterns

- **One-off components.** Designed for one screen, never reused, no system thinking applied.
- **Token proliferation.** Adding new token for one design instead of mapping to existing.
- **Inconsistent token usage.** Same purpose with different tokens randomly.
- **No documentation.** System exists in designer's head, not files.
- **Frozen system.** Never updated, becomes irrelevant to current needs.
- **Over-design.** 200 components for 5-screen app. Build only what's used.
- **Visual designs without component thinking.** Beautiful screens; impossible to implement consistently because no underlying system.
- **Code mismatched to design.** Designer says "button is 8px radius" but implemented as 6px. System breaks at handoff.
- **Naming chaos.** `btn-1`, `MainButton`, `primary-btn`, `submitButton`. Pick convention.
- **Versioning ignored.** Breaking change shipped without notice. Existing usage breaks.

## Integration

- `user-task-analysis` informs what components needed
- `design-tokens-discipline` enforces token usage
- `wireframe-production` references existing components
- `component-library-development` builds components per system
- `dev-handoff-specs` references system in dev specs
- `accessibility-first` constrains component design
- Each component is a DesignArtifact (type: component_spec)
- Quarterly review per `library/design.md` evolves system

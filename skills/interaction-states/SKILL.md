---
name: interaction-states
description: Defining all interactive states for every component — default, hover, focus, active, disabled, loading, error, success, empty. Without explicit state design, devs make assumptions that diverge from intent. Comprehensive state design is the difference between polished UX and broken edges.
---

# Interaction States — All Seven Specified

A button has more than one look. Hover, focus, click, disabled, loading — each is a distinct visual that signals what's happening. Designing only the default state and hoping for the best produces interfaces with broken edges.

## Prerequisites

- `wireframe-production` complete (basic structure)
- `design-system-thinking` understood
- Component identified (button, input, card, etc.)

## Core principle

> Every interactive element has seven possible states. Specify all seven, even if some look similar. Devs implementing without specs make assumptions; assumptions diverge from designer intent; final product has inconsistent state behavior across components.

## The seven states

### 1. Default

The element at rest. Not being interacted with. Most common state.

Considerations:
- Visually clear what type of element (button, link, input)
- Affordance — looks interactable
- Brand-consistent

### 2. Hover

Mouse cursor over element. Indicates "this is interactable".

Common patterns:
- Slight background color change (10-20% lightness shift)
- Subtle scale (1.02x)
- Cursor change (pointer for clickable)
- Drop shadow increase

Should be subtle. Hover that's dramatic feels twitchy.

**Note:** doesn't apply on touch devices. Don't put critical info in hover state only.

### 3. Focus

Element selected via keyboard or programmatically. Different from hover (which is mouse).

Required for accessibility:
- 2px outline minimum, with sufficient contrast
- Visible without obscuring content
- Distinct from hover (so both work simultaneously)

Standard pattern: `outline: 2px solid var(--color-focus); outline-offset: 2px;`

**Don't `outline: none` without replacement.** Removes keyboard accessibility.

### 4. Active (pressed)

Mid-click or mid-tap. The brief state during action.

Common patterns:
- Slightly darker background
- Scale down (0.98x) — "pushed in" feel
- Inset shadow

Should be quick visual feedback, even if action takes longer (then transition to loading).

### 5. Disabled

Element exists but can't be interacted. Used for:
- Form field not yet valid
- Action not yet available
- Permission restriction

Visual:
- Reduced opacity (40-60%)
- No hover/focus response
- Cursor: not-allowed
- Often muted color

**Programmatic:** `disabled` attribute (forms) or `aria-disabled="true"`. Both for screen readers + actual behavior.

Provide explanation when possible: "Submit (fill required fields first)" or tooltip on hover.

### 6. Loading

Action in progress.

Patterns:
- Spinner replacing icon
- Pulsing animation
- "Loading..." text replacing button label
- Skeleton screen for content areas

Important: prevent double-click while loading. Disable element during loading state.

For long operations (>3s): progress indicator or estimate.

### 7. Error / Success / Other status

Result feedback:

**Error:**
- Burgundy/red border or background
- Error icon
- Error message nearby
- Persist until user corrects

**Success:**
- Forest/green check briefly
- Often transitions back to default

**Warning:**
- Gold/yellow indicator
- Caution message

For inputs: validation state shown on blur, after submit attempt, or inline as user types (debounced).

## Bonus states

### Empty state

Component has no data. Visual: placeholder + helpful message + call-to-action.

Example: empty inbox shows "No messages yet" + illustration + "Compose your first" button.

Empty states are first impressions. Often skipped, often important.

### Partial state

Some data, not all. E.g., search results loading more, list with 3 of 50 items shown.

### Hover-disabled

Disabled element with hover effect (tooltip explaining why disabled).

### Read-only

Looks like input but uneditable. Different from disabled (which signals unavailable).

## State specification format

In DesignArtifact for component:

```yaml
component: PrimaryButton

states:
  default:
    background: color-action-primary
    text: color-text-on-dark
    padding: space-2 space-4
    radius: radius-sm
    border: none
    cursor: pointer
    transition: duration-fast easing-default

  hover:
    background: color-action-primary-hover (10% lighter)
    transform: translateY(-1px)
    shadow: shadow-sm

  focus:
    outline: 2px solid color-focus
    outline-offset: 2px

  active:
    background: color-action-primary-active (10% darker)
    transform: translateY(0)
    shadow: none

  disabled:
    background: color-action-primary
    opacity: 0.5
    cursor: not-allowed
    pointer-events: none

  loading:
    background: color-action-primary
    text: invisible
    cursor: wait
    additional: spinner icon (16px)

  error: not applicable (button has no error state — input does)
```

This level of detail is what dev needs.

## Common state mistakes

**Same focus and hover.** Keyboard user thinks they didn't focus — same as hover.

**No active state.** Click feels unresponsive (~100ms before action completes feels broken).

**Loading state same as default.** User clicks again, double-submits.

**Disabled looks clickable.** Users try, get nothing, frustrated.

**Error states only in red color.** Colorblind users miss. Combine color + icon + text.

**Success forever.** Saved state stays green. Cluttered UI.

**Empty state generic.** "No data" — useless. Tell user how to get data.

## Animation between states

State transitions feel better with brief animation:

- Hover → use 150ms transition on background-color, transform
- Active → snap (50ms or instant)
- Loading → fade in spinner
- Error → shake gently (telegraphs error)

Token-driven: `transition: duration-fast easing-default`.

Don't animate everything — fast states (active) should be instant. Slow states (loading) ok to animate.

## Touch vs mouse considerations

Touch devices:
- No hover (or hover = first tap, click = second — confusing)
- Active state shown briefly during tap
- Focus less critical (no keyboard)

Mouse devices:
- Hover important (signals affordance)
- Focus shown for keyboard users
- Active brief

For responsive designs: hover often weakened on mobile-equivalent or removed.

## State testing

For every component, verify all states by:
- Hover with mouse
- Tab to focus with keyboard
- Click to see active
- Set disabled attribute, verify visual
- Trigger loading
- Trigger error
- Empty data

Catch missing states before dev handoff. Each missing state = guess by dev = inconsistency.

## State documentation

Every component spec has state matrix:

```
            Default  Hover  Focus  Active  Disabled  Loading  Error
Button       ✓       ✓      ✓      ✓       ✓          ✓        n/a
Input        ✓       ✓      ✓      n/a     ✓          ✓        ✓
Card         ✓       ✓      ✓      ✓       n/a        n/a      n/a
Modal        ✓       n/a    ✓      n/a     n/a        ✓        n/a
```

Some states don't apply (modal doesn't hover). Mark `n/a` explicitly so dev knows it was considered.

## Anti-patterns

- **Default state only.** "Just make the button" → dev guesses other states. Inconsistency.
- **Hover and focus identical.** Keyboard accessibility broken.
- **No loading state.** User clicks, app appears broken during long operations.
- **Disabled looks active.** Users click in vain.
- **No error state.** Validation feedback unclear.
- **Custom focus that's invisible.** Removed default outline without replacement.
- **States designed in isolation.** Each component different state convention. System breaks.
- **Animation in active state slow.** Active feels laggy.
- **Loading without timeout.** User waits forever, no escape.
- **Error blames user.** "Invalid input" — user knows already. Tell what to do.
- **Empty state forgotten.** Designed for ideal case only.

## Integration

- Loaded after `wireframe-production` and before `dev-handoff-specs`
- Stored as `stateSpecs` JSON in DesignArtifact
- Reviewed by Dev during handoff (no surprises)
- `accessibility-first` enforces focus state
- `responsive-design` may add mobile-specific state variants

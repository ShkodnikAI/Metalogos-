---
name: dev-handoff-specs
description: How designs become executable specifications for Dev. The artifact format that bridges Design and Dev — explicit enough for implementation without questions, complete enough to verify implementation matches. Without good handoff, Dev fills gaps with assumptions and final implementation drifts from design intent.
---

# Dev Handoff Specs — The Contract Between Design and Dev

A design that looks great in Figma but Dev can't implement faithfully is a failed design. The handoff spec is the contract: what should be built, with enough detail that implementation is deterministic.

## Prerequisites

- Design approved (status: `approved`)
- `design-tokens-discipline` and `component-library-development` understood
- Dev team accessible for clarifying questions

## Core principle

> A spec is complete when Dev can implement without asking questions. Ambiguity is the enemy: every ambiguity becomes Dev's guess, which may or may not match designer's intent. The spec invests time upfront to save 10x time in implementation iterations and rework.

## Spec components

A complete handoff spec includes:

### 1. Visual reference

- Final mockup (PNG/SVG export from Figma)
- All states (default/hover/focus/active/disabled/error/loading/empty)
- Mobile / tablet / desktop variants
- Light + dark mode if applicable

### 2. Component breakdown

For each visual element, identify:

- **Existing component from library?** → reference component name + variant
- **New component?** → full spec (see below)
- **One-off / composition?** → use existing components in specific arrangement

This is where component library pays off. Most elements should be existing components, not new ones.

### 3. Design tokens used

Explicit list of which tokens apply:

```yaml
colors:
  background: --color-primary-cream
  border: --color-light-grey
  accent: --color-accent-gold

typography:
  heading: --type-h2
  body: --type-body
  caption: --type-caption

spacing:
  padding: --space-md
  gap: --space-sm
```

Never hardcoded values. Reference tokens.

### 4. Behavior specification

What happens when user interacts:

```
Button "Save":
- onClick → call save action
- During async save: show loading spinner, disable button
- On success: brief success state (2s green check), then return to default
- On error: error toast, button returns to default
- Keyboard: Enter triggers click when focused
- Esc: ignores

Form field "Email":
- onChange: validate format (RFC 5322)
- onBlur: show validation error if invalid
- After valid input: clear error
- aria-invalid="true" when error visible
- aria-describedby points to error message
```

### 5. Responsive behavior

How does layout change across breakpoints:

```
Breakpoints:
- mobile (< 768px): single column, full-width buttons, hamburger nav
- tablet (768-1024): two columns, side nav collapsed
- desktop (> 1024): three columns, side nav expanded

Component-specific:
- Card: full-width mobile, 50% width tablet, 33% width desktop
- Hero text: 32px mobile, 48px tablet, 64px desktop
- Sidebar: drawer mobile, fixed tablet, fixed desktop
```

### 6. Accessibility requirements

Explicit, not "make it accessible":

```
- All interactive elements keyboard accessible (Tab/Shift+Tab)
- Focus visible (outline or ring)
- Color contrast ≥ 4.5:1 (WCAG AA)
- Form errors announced to screen reader (aria-live)
- Modal traps focus, restores on close
- Skip-to-content link first focusable element
- Heading hierarchy: one h1 per page
```

### 7. Edge cases

What about:
- Empty state (no data)
- Loading state (data fetching)
- Error state (load failed)
- Very long text (overflow handling)
- Very short text (alignment)
- Many items (pagination/virtualization)
- Few items (no scroll)
- Slow network (skeleton screens)
- Offline (graceful degradation)

Each addressed in spec.

### 8. Data requirements

What data does the component need to render:

```typescript
interface UserCardProps {
  user: {
    id: string;
    name: string;
    email: string;
    avatar?: string;
    role: 'admin' | 'member' | 'guest';
    lastSeen: Date | null;
  };
  onMessage?: () => void;
  onRemove?: () => void;
}
```

This is the contract Dev implements against.

## Spec format

For complex projects: Figma file + spec doc + token reference.

For simple projects: markdown doc with embedded screenshots.

For Fosved: store as `DesignArtifact.content` (markdown or JSON spec), reference Figma file URL where applicable.

Example simplified spec:

```markdown
# User Profile Card — Spec v1.0

## Visual reference
[mockup-default.png]
[mockup-hover.png]
[mockup-loading.png]
[mockup-error.png]

## Components used
- Card (existing, variant: outlined)
- Avatar (existing, size: lg)
- Button (existing, variant: secondary)
- StatusBadge (NEW — see new components below)

## Design tokens
- Container: `--color-primary-cream`, `--space-lg` padding, `--radius-md`
- Border: `--color-light-grey`, 1px
- Heading: `--type-h3`, `--color-primary-navy`
- Subtext: `--type-caption`, `--color-neutral-grey`

## Behavior
- Click on card body: navigate to user detail
- Click on "Message" button: open message composer
- Click on "Remove" button: show confirm dialog, then remove
- Hover: subtle elevation (shadow-sm)
- Focus: ring around full card

## Responsive
- Mobile: full width, vertical layout
- Tablet+: 320px width, horizontal layout

## States
[document each state from mockups]

## Accessibility
- Card has `role="article"` and `aria-labelledby={userNameId}`
- Avatar has alt text with user's name
- Status badge has accessible text equivalent
- Keyboard: Enter on focused card = navigate

## Data
[TypeScript interface as above]

## Edge cases
- No avatar: show initials in colored circle
- Long name: ellipsis after 30 chars, full on hover (tooltip)
- "Last seen" null: show "—" instead
```

## New component specs (when needed)

If spec introduces NEW component (not in library), it needs full sub-spec:

```markdown
## NEW: StatusBadge

### API
\`\`\`typescript
interface StatusBadgeProps {
  status: 'online' | 'away' | 'offline';
  showLabel?: boolean;
}
\`\`\`

### Visual
- Online: forest circle, optional "Online" label
- Away: gold circle, optional "Away" label
- Offline: grey circle, optional "Offline" label

### Sizes
- Default: 8px circle
- With label: 8px circle + 4px gap + caption text

### Accessibility
- Circle has aria-label="Status: online"
- If showLabel, label text is the source of truth, circle is decorative
```

If new component will be used elsewhere → propose adding to library.

## Common spec gaps to avoid

**Loading states** often missing in mockups but always present in reality. Specify.

**Empty states** ("no posts yet") often missing. Specify what shows.

**Error states** (something failed) often missing. Specify recovery options.

**Edge cases for text length:** Designer used 5-word headline. Real data has 50-word headline. What happens?

**Time-based variations:** "Last login" — show absolute time, relative time, or both? Updates how often?

**Multi-language considerations:** if i18n planned, text length varies dramatically.

## The Q&A spec validation

Before handoff finalized, designer answers all of these or fills gaps:

- [ ] Every state of every component shown
- [ ] Every responsive variant shown
- [ ] Color tokens used (no hex literals)
- [ ] Typography tokens used (no inline px sizes)
- [ ] Spacing tokens used (no inline px)
- [ ] All interactive elements have hover/focus/active states
- [ ] All accessibility requirements explicit
- [ ] Loading/empty/error states defined
- [ ] Data interface defined
- [ ] Edge cases for text length addressed
- [ ] Time-sensitive elements specified

Checklist saved in spec.

## Iteration workflow

Spec is v1.0 initially. As Dev implements, things come up:
- Designer didn't consider a case → spec update → v1.1
- Tech constraint makes spec infeasible → discussion, modified spec → v1.1
- Better solution discovered during impl → spec update → v1.1

Track in `DesignArtifact.iterationCount` and `feedbackLog`.

After implementation: implementation drift score (`implementationDrift`) measured 2 weeks post-deploy.

## Implementation drift measurement

2 weeks after Dev ships implementation, designer reviews:

For each spec'd item:
- Implemented as specified? ✓
- Acceptable variation? Note why.
- Unacceptable variation? Bug or wrong impl?

Score: implemented_correctly / total_specd = drift_score

Target: ≥ 0.9 (90% match). Below means spec was unclear OR Dev took liberties.

Lessons:
- High drift → spec needs more detail
- Specific drift patterns → which areas of spec need work

## Anti-patterns

- **Spec without states.** "Here's the button design." What about hover, focus, disabled? Dev guesses.
- **Hex codes in spec.** Hardcoded values. Token system bypassed. Inconsistency emerges.
- **No data interface.** "It shows user info." What fields? Dev makes up structure.
- **Aspirational specs.** Spec what should be possible, not what's actually built. Dev wastes time.
- **Locked Figma file.** Dev can't inspect properties without designer's help. Friction.
- **No version on spec.** Implementation against old version when new exists.
- **One-off custom for every component.** Component library never grows. Inconsistency accumulates.
- **No acceptance criteria.** Dev says "done" but designer expected different. Conflict.
- **Spec finished after Dev started.** Dev guesses, then spec contradicts.
- **No edge cases.** Implementation breaks on real data.
- **Vague responsive.** "Mobile-friendly." Dev makes up breakpoints.
- **Inconsistent terminology.** "Card" in spec, "tile" in code, "block" in CSS. Pick term, stick with it.

## Integration

- Outputs feed into Dev's `lib/dev.js` createDevTask flow
- `DesignArtifact.handedOffToDevTaskId` links
- `qa/integration-testing-patterns` tests against the spec
- `implementationDrift` measured and reported in monthly Design review
- Design-to-code drift is core Design metric

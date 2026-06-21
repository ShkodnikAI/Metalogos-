---
name: accessibility-first
description: Accessibility (a11y) baked into design from the start, not retrofitted. WCAG AA compliance minimum (AAA for critical). Color contrast, keyboard navigation, screen reader support, focus management, error handling. Designs that work for users with disabilities work better for everyone.
---

# Accessibility First — Designing for Everyone

Accessibility isn't an afterthought or compliance checkbox. It's design discipline that produces better interfaces for all users, not just those with disabilities. Keyboard navigation works for power users. Clear contrast works in sunlight. Screen reader semantics improve SEO. Skipping a11y costs more later (lawsuits, rebuilds, lost users).

## Prerequisites

- Design starting (NOT after design done)
- Project's WCAG target level decided (AA default, AAA for critical)

## Core principle

> Accessibility designed-in is free; accessibility retrofitted is expensive. Plus: when you fix the edge cases (low vision, motor impairment, screen readers), you fix usability for everyone in degraded contexts (sunlight, one-handed, broken mouse).

## WCAG levels

**WCAG 2.2 standards** (current):

- **Level A:** basics. Without these, content unusable for many. Bare minimum.
- **Level AA:** practical accessibility. Standard for production.
- **Level AAA:** maximum. Sometimes constrains design.

**For Fosved projects:**
- Default: AA (per `library/design.md` hard rule)
- Critical projects: AAA
- Public-facing: at least AA (often legally required)

## Color and contrast

**Contrast ratios required:**

| Element | AA | AAA |
|---------|-----|-----|
| Normal text | 4.5:1 | 7:1 |
| Large text (18pt+/14pt+bold) | 3:1 | 4.5:1 |
| Non-text (icons, borders) | 3:1 | 4.5:1 |
| UI states | 3:1 | 3:1 |

Check every text-on-background combination.

Tools:
- WebAIM Contrast Checker
- Browser DevTools (accessibility panel)
- axe-core (automated)
- macOS/Windows built-in color picker

**Don't rely on color alone** for meaning. Add icon, text, or pattern.

Example:
- Bad: red error text, green success text. Colorblind users see neither.
- Good: red error + ⚠ icon + "Error:" prefix; green success + ✓ icon + "Success:" prefix.

**Brand color compliance:** check Fosved palette against AA standards:
- Navy on cream: ratio ~13:1 → AAA pass
- Gold on cream: ratio ~3.5:1 → AA pass for large only, FAIL for small text
- Burgundy on cream: ratio ~7:1 → AA pass

So: gold can't be used for body text on cream. Use only for large/decorative.

## Keyboard navigation

**Every interactive element keyboard-accessible:**
- Tab to reach
- Enter/Space to activate
- Escape to close (modals, dropdowns)
- Arrow keys for navigation within composite widgets

**Focus visible:**
- Browsers have default focus ring — never `outline: none` without replacement
- Custom focus ring must be visible (2px+ contrast with background)
- Don't rely on background change alone (color-blind issue)

**Logical tab order:**
- Follows visual flow (top to bottom, left to right in LTR)
- No tab traps (focus that can't leave)
- Modal opens → focus enters modal; closes → focus returns to trigger

**Skip links:**
- "Skip to main content" link at top
- Hidden visually but in focus order
- Lets keyboard users skip nav repetition

## Semantic HTML

Use right element for the job:
- `<button>` for buttons (not `<div onClick>`)
- `<a>` for navigation (not button-styled link)
- `<input type="...">` for form fields
- `<nav>`, `<main>`, `<article>`, `<aside>` for landmarks
- `<h1>` through `<h6>` for hierarchy (use sequentially, no skipping)

Semantic HTML gives accessibility for free:
- Screen readers announce role
- Keyboard navigation works
- Browser provides default behavior

When semantic HTML insufficient, use ARIA. But: **first rule of ARIA is don't use ARIA** if HTML can do it.

## ARIA when needed

**ARIA roles:** describe purpose (`role="button"`, `role="alert"`).

**ARIA states:** describe current state (`aria-expanded`, `aria-selected`, `aria-checked`).

**ARIA properties:** describe relationships (`aria-labelledby`, `aria-describedby`, `aria-controls`).

Common patterns:

**Modal:**
```html
<div role="dialog" aria-modal="true" aria-labelledby="modal-title">
  <h2 id="modal-title">Confirm action</h2>
  ...
</div>
```

**Dropdown:**
```html
<button aria-haspopup="true" aria-expanded="false" aria-controls="menu-1">
  Menu
</button>
<ul id="menu-1" role="menu" hidden>...</ul>
```

**Live region for dynamic updates:**
```html
<div aria-live="polite" aria-atomic="true">
  Cart updated: 3 items
</div>
```

Refer to ARIA Authoring Practices Guide (W3C) for established patterns. Don't invent.

## Touch targets

**Minimum size:** 44×44 px (per Apple HIG and WCAG AAA).

**Spacing between targets:** 8px minimum (prevents mis-tap).

Why: fat fingers, motor impairments, small screens. Tiny targets fail real users.

## Form accessibility

**Every input has a `<label>`** (associated via `for=id`).

Visible labels preferred over placeholder-only (placeholders disappear when typing).

**Group related inputs** with `<fieldset>` + `<legend>`.

**Error messaging:**
- Inline error after submit attempt
- Programmatically associated with input (`aria-describedby`)
- Clear remediation: "Email format invalid" not just "Invalid"

**Required indication:**
- Visual (asterisk + explanation)
- Programmatic (`required` attribute, `aria-required`)

## Images and media

**`alt` attribute on every `<img>`:**
- Descriptive: alt="Bar chart showing Q4 revenue up 20%"
- Empty if decorative: alt=""
- Never null/missing — screen readers will read filename

**Videos:**
- Captions for spoken content (required)
- Audio descriptions for visual-only content (when applicable)
- Transcripts for podcasts/audio

**Animations:**
- Respect `prefers-reduced-motion` media query
- Pause/disable on user preference
- No flashing >3 times/second (seizure risk)

## Screen reader testing

Designs should be reviewed against screen reader output:
- macOS VoiceOver (Cmd+F5)
- Windows NVDA (free)
- iOS VoiceOver
- Android TalkBack

What to verify:
- Page structure communicated (landmarks, headings)
- Content reads in logical order
- Interactive elements announced with role + state
- Form fields announced with label
- Errors announced when occur
- Dynamic updates announced (live regions)

Even one screen reader session reveals design issues invisible to sighted designer.

## Cognitive accessibility

**Plain language:**
- Active voice, present tense
- Short sentences
- Common words (avoid jargon, technical terms unless audience-appropriate)
- Read-aloud test: does it make sense spoken?

**Predictable interfaces:**
- Same action does same thing everywhere
- Patterns repeated (consistent button placement, navigation)
- No surprising behavior

**Error prevention:**
- Confirm destructive actions
- Allow undo where possible
- Validate before submission
- Auto-save to prevent data loss

**Clear navigation:**
- Always know where you are (breadcrumbs, page title)
- Always know how to get back
- Logical hierarchy

## Quick a11y design checklist

For every design before approval:

- [ ] All text-on-background combinations meet AA contrast
- [ ] Color not sole indicator of meaning
- [ ] Touch targets ≥44px with 8px spacing
- [ ] Form labels visible (not placeholder-only)
- [ ] Error messages clear and associated with inputs
- [ ] Focus states visible for all interactive elements
- [ ] Keyboard navigation works (mentally trace tab order)
- [ ] Semantic structure (headings, landmarks) planned
- [ ] Images have alt strategy (descriptive vs decorative)
- [ ] Modal/dropdown/dynamic patterns follow ARIA practices
- [ ] Animation respects reduced-motion preference
- [ ] No flashing >3Hz
- [ ] Reading level appropriate for audience

If any unchecked: revise design.

## Anti-patterns

- **Retrofit after.** Designing then "adding accessibility". Always more expensive and less elegant.
- **"Accessibility mode" toggle.** Should be accessible always. Separate mode = afterthought.
- **Tooltip-only labels.** Required info hidden behind hover. Hover doesn't work on touch.
- **Placeholder as label.** Disappears when typing. Users forget what field was for.
- **Color-only error.** Red border without text or icon. Colorblind users miss.
- **Outline:none without replacement.** Removes focus indicator. Keyboard users lost.
- **Icons without labels.** Recognition aid only. Screen readers announce "image" — useless.
- **Hover-only menus.** Drop-down on hover. Can't access via keyboard or touch.
- **CAPTCHAs without alternatives.** "I'm not a robot" tests fail many users.
- **Auto-play media.** Disrupts screen reader, annoys all users.
- **Modals trapping focus then dismissing without returning focus.** Disorientation.

## Tools

- **axe DevTools** (browser extension): catches many issues automatically
- **WAVE** (browser extension): visual a11y feedback
- **Lighthouse** (Chrome DevTools): includes a11y audit
- **Color contrast checkers:** WebAIM, Stark
- **Screen readers:** VoiceOver, NVDA, JAWS
- **Keyboard testing:** unplug mouse, navigate

Automated tools catch ~30-50% of issues. Manual testing catches the rest.

## Integration

- Applied during every design (`user-task-analysis` flows already a11y-considered)
- `design-system-thinking` enforces a11y at component level (every component a11y-tested)
- `interaction-states` includes focus state and screen reader states
- `dev-handoff-specs` includes a11y requirements
- `qa/security-testing-protocol` includes a11y audit
- WCAG level recorded on every DesignArtifact

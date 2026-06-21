---
name: wireframe-production
description: Producing wireframes — low-fidelity layout sketches before visual design. Start at lo-fi to validate structure, escalate to high-fi only after structure approved. Wireframes prevent expensive late-stage changes when fundamental layout doesn't work.
---

# Wireframe Production — Structure Before Style

A common failure: designer jumps to high-fidelity visuals, gets feedback "this layout doesn't work", redoes everything. Wireframes solve this — cheap iteration on structure first, expensive visuals after structure stable.

## Prerequisites

- `user-task-analysis` completed
- Project scope clear
- Existing design system identified (or template chosen)

## Core principle

> Wireframes test structure. Visual design tests style. Mixing them wastes both. Lo-fi first lets you iterate on layout cheaply (5 wireframes in time for 1 visual). Once layout approved, visual design has clear constraints.

## Fidelity levels

**Level 1: sketches (paper or rough digital)**
- Boxes, arrows, words
- 5-10 minutes per screen
- Use for: exploring options quickly

**Level 2: wireframes (lo-fi digital)**
- Cleaner boxes, placeholder text
- Component blocks identified
- 30-60 minutes per screen
- Use for: validating structure

**Level 3: mid-fi prototypes**
- Wireframes with basic styling
- Component states shown
- 2-4 hours per flow
- Use for: testing interactions

**Level 4: high-fidelity visuals**
- Full visual design
- Real content
- 4-8 hours per screen
- Use for: final approval before dev handoff

Escalate fidelity only after lower fidelity approved.

## What wireframes show

**Layout structure:**
- Header, sidebar, content area, footer (zones)
- Component placement and hierarchy
- Reading flow direction

**Content hierarchy:**
- What's most important (visually largest)
- What's secondary
- What's tertiary
- What's chrome (always there)

**Information architecture:**
- Navigation structure
- Page-to-page relationships
- Breadcrumbs / location indicators

**Interactive elements:**
- Buttons, inputs, dropdowns (as placeholder shapes)
- Click targets
- Form structure

## What wireframes don't show

- Actual colors (use grays, single accent at most)
- Specific typography (placeholder text in single font)
- Final imagery (placeholders: gray rectangles with X)
- Decorative elements
- Brand identity

These come in later fidelity. Including them early derails feedback to "I don't like this color" instead of "this layout doesn't work".

## Wireframe components vocabulary

Use simple shapes consistently:

```
Header: thick rectangle top
Navigation: row of labels
Content area: large rectangle
Sidebar: column
Card: rectangle with internal structure
Button: smaller rectangle with rounded corners
Input field: rectangle with thin border
Image placeholder: rectangle with X crossed through
Text: horizontal lines (heading: thick, body: thin)
List: stacked thin rectangles
Modal: rectangle on darkened background
Tabs: row of rectangles, one highlighted
```

These let viewer focus on structure not styling.

## Tools

**Quick sketches:** pen + paper, FigJam, Whimsical, Excalidraw

**Wireframes:** Figma, Sketch, Adobe XD

For Fosved: Figma is adopt-state. Excalidraw for quick collab sketches (lower commitment).

## Process

### 1. Sketch alternatives (10-20 min)

For one screen, sketch 3-5 layout alternatives. Hand-drawn or rough digital.

Don't get attached. Most go to trash.

Compare:
- Which serves primary task best?
- Which is simplest?
- Which fits existing patterns?
- Which is most flexible for variants?

Pick one or merge best aspects.

### 2. Low-fi wireframe (30-60 min)

Translate sketch to digital wireframe. Cleaner but still no styling.

Components identified, positions set, hierarchy established.

### 3. Walk through user tasks (5 min)

For each user task from `user-task-analysis`:
- Trace the path on wireframe
- Verify all needed elements present
- Check for friction points

If task can't be completed: redesign before continuing.

### 4. Get feedback at lo-fi (15-30 min)

Show wireframe to owner / stakeholders.

Feedback at lo-fi is about structure:
- "I'd want recent activity here"
- "Make checkout obvious"
- "This sidebar wastes space on mobile"

Feedback NOT about style yet. If style feedback comes ("make it warmer"), redirect: "We'll handle visual style after structure approved."

### 5. Iterate (30 min - 2 hours)

Revise based on feedback. Document iteration count in DesignArtifact.

If >3 iterations on lo-fi: structure has fundamental issue. Re-do task analysis.

### 6. Approve lo-fi (gate)

Get explicit approval on structure. Then escalate.

## Wireframe annotations

Annotations explain what visual can't:

- "Click here → opens modal X"
- "Validation runs on blur"
- "Real-time updates here"
- "Mobile: collapses to dropdown"

Use clear callout arrows. Annotation text 12-14pt minimum (must be legible at scale).

For complex flows: separate flow diagram alongside wireframe.

## Mobile vs desktop

For responsive projects: wireframe both mobile and desktop.

Order:
- Mobile first (constraints expose what's essential)
- Then desktop (more space, can add secondary content)

Don't design desktop, then "make it work on mobile" — usually fails on mobile.

## Multi-screen flows

For task flows spanning screens:

- Sequence screens left-to-right
- Show transitions between (arrow + brief label)
- Identify the steps from task analysis
- Verify each step is supported

Single-screen wireframe insufficient for transactional flows. Always show flow.

## Empty states, error states, loading states

Don't wireframe only happy path.

For every screen with dynamic content:
- Empty state (no data yet)
- Loading state (data fetching)
- Error state (something went wrong)
- Partial data (some content)

Each is a wireframe. Each is a real user experience.

Owner often surprised: "Oh I didn't think about empty state." That's the value.

## Wireframe documentation

Each wireframe (DesignArtifact) includes:
- Title (which screen)
- User task being served
- Annotations
- Notes (what isn't visually obvious)
- Open questions (decisions pending)
- Implementation notes for dev (not full handoff yet)

Stored as DesignArtifact `artifact_type: wireframe`.

## When to skip wireframes

For simple components (single button, single input, modal with 3 fields): wireframe overkill. Sketch + visual directly is fine.

For familiar patterns where existing design system covers everything: wireframe may add little. Use existing patterns.

For exploratory / experimental: lots of sketching, fewer formal wireframes. Iterate on rough explorations.

## Anti-patterns

- **Skip wireframes for "simple" projects.** Project that turns out to be 5 screens with complex interactions. Wasted time.
- **Wireframes with styling.** "Just adding a little color" — defeats purpose.
- **One wireframe, never iterate.** First idea isn't best. 2-3 iterations minimum.
- **Stuck at lo-fi forever.** Lo-fi is means, not end. Escalate when approved.
- **No annotations.** Wireframe alone leaves room for interpretation. Annotate.
- **Mobile as afterthought.** Hard to retrofit.
- **Happy path only.** Empty/error/loading states discovered at dev time.
- **Showing lo-fi to wrong audience.** Stakeholders sometimes can't read wireframes. Either escalate fidelity for them or train them.
- **Wireframes don't reflect technical reality.** Designer designs feature that's technically impossible. Loop with Dev early.

## Time budget

For typical project:

- Task analysis: 1-2 hours
- Sketches: 1-2 hours
- Lo-fi wireframes (5-10 screens): 4-8 hours
- Feedback + iteration: 2-4 hours
- **Total lo-fi: 1-2 days**

vs:

- High-fi straight: 8-16 hours per screen × 8 screens = 64-128 hours
- Discovery of layout issue at high-fi: another 64-128 hours redoing

Lo-fi first: 1-2 days. Hi-fi after approval: clean and fast. Total: significantly less.

## Integration

- Output: DesignArtifact `wireframe` records
- Followed by `interaction-states` (define states), then visual design (hi-fi)
- Component identification feeds `component-library-development`
- Wireframe approval is gate before progression
- Iteration count tracked in `iterationCount` field

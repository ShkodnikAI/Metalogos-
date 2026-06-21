---
name: component-library-development
description: Building and maintaining a component library — reusable, composable UI building blocks that scale across projects. From atoms (buttons, inputs) to molecules (forms, cards) to organisms (navigation, dashboards). The shared vocabulary that lets multiple projects use consistent UI without reimplementing every time.
---

# Component Library Development — Reusable UI Building Blocks

A component library is leverage. Build a button once, use it 100 times. Without library, every project reimplements buttons with slight inconsistencies. With library, every project gets the same battle-tested implementations.

## Prerequisites

- `design-system-thinking` loaded
- `design-tokens-discipline` loaded
- Design system exists OR being created in parallel

## Core principle

> A component is not a styled HTML element — it's a contract. The component promises: "here's the visual treatment, here are the interactive states, here are the accessibility behaviors, here's the API." Consumers depend on the contract. Breaking it breaks everyone.

## Atomic design hierarchy

Brad Frost's atomic design vocabulary (still useful):

**Atoms:** smallest indivisible UI elements
- Button, Input, Label, Icon, Badge, Spinner, Avatar

**Molecules:** combinations of atoms forming functional units
- FormField (Label + Input + Error), Card (Header + Body + Actions), SearchBar (Input + Button)

**Organisms:** complex components with own state/behavior
- NavigationBar, DataTable, Modal, Dashboard tile

**Templates:** layout patterns combining organisms
- DashboardLayout, FormLayout, DetailViewLayout

**Pages:** specific instances using templates
- UserDashboard, ProductDetail, SettingsPage

Build atoms first. Compose up. Don't start with pages — you'll re-implement atoms inconsistently.

## Component API design

Every component has a public API. Design it intentionally.

**Good component API principles:**

**Single responsibility.** Button is a button. Don't add "isLoading" + "icon" + "tooltip" + "confirm dialog" — those are separate components.

**Sensible defaults.** Component works with minimal props.
```tsx
<Button>Click me</Button>  // works, default variant
<Button variant="primary" size="large">Click me</Button>  // explicit
```

**Composition over configuration.** Pass children for flexibility, not 50 boolean props.
```tsx
// Bad: configuration explosion
<Card hasHeader hasFooter showBorder padding="large" />

// Good: composition
<Card>
  <Card.Header>Title</Card.Header>
  <Card.Body>Content</Card.Body>
  <Card.Footer>Actions</Card.Footer>
</Card>
```

**Predictable naming.**
- Boolean props: `disabled`, `loading`, `expanded`, not `isDisabled`, `isLoading`
- Variants: `variant="primary"` not `type="primary"` (type is reserved)
- Sizes: `size="sm" | "md" | "lg"` standard
- Event handlers: `onChange`, `onClick`, `onSubmit` standard

**TypeScript first.** Every component fully typed.
```typescript
interface ButtonProps {
  variant?: 'primary' | 'secondary' | 'destructive' | 'ghost';
  size?: 'sm' | 'md' | 'lg';
  disabled?: boolean;
  loading?: boolean;
  children: React.ReactNode;
  onClick?: () => void;
}

export function Button({ 
  variant = 'primary', 
  size = 'md', 
  disabled = false,
  loading = false,
  children,
  onClick 
}: ButtonProps) {
  // ...
}
```

## States — what every interactive component handles

Every interactive component must handle:

- **default** — at rest, ready for interaction
- **hover** — pointer over (desktop)
- **focus** — keyboard focused (essential for accessibility)
- **active** — currently being pressed/clicked
- **disabled** — cannot interact
- **loading** — async operation in progress (where applicable)
- **error** — invalid state (for inputs)

Missing states = incomplete component. Don't ship without all states designed AND tested.

## Polymorphism patterns

Components that can render as different elements:

**`as` prop pattern:**
```tsx
<Button as="a" href="/profile">Profile link styled as button</Button>
<Button as={Link} to="/profile">React Router link</Button>
```

**`asChild` pattern (Radix UI):**
```tsx
<Button asChild>
  <Link href="/profile">Profile</Link>
</Button>
```

Avoids wrapping divs, lets you use semantic HTML.

## Composition patterns

**Compound components:**
```tsx
<Tabs defaultValue="account">
  <Tabs.List>
    <Tabs.Trigger value="account">Account</Tabs.Trigger>
    <Tabs.Trigger value="password">Password</Tabs.Trigger>
  </Tabs.List>
  <Tabs.Content value="account">...</Tabs.Content>
  <Tabs.Content value="password">...</Tabs.Content>
</Tabs>
```

Shared state via Context internally. Consumers compose freely.

**Slots:**
```tsx
<Card
  header={<h2>Title</h2>}
  footer={<Button>Action</Button>}
>
  Body content
</Card>
```

Or via compound components above.

**Render props (less common):**
```tsx
<DataLoader>
  {(data, loading) => loading ? <Spinner/> : <DataView data={data}/>}
</DataLoader>
```

Flexible but verbose. Compound components usually cleaner.

## Modern library: shadcn/ui pattern

Current recommended approach (per Tech Radar):

shadcn/ui is **copy-paste, not npm install**. You own the component code.

```bash
npx shadcn-ui@latest add button
# Creates: components/ui/button.tsx in your project
```

**Pros:**
- No version conflicts
- Customize without forking
- Built on Radix (accessibility solid)
- Tailwind-based (no styling system to learn)
- Tree-shaken automatically

**Cons:**
- No automatic updates (you manage upgrades manually)
- Manual customization per project (less leverage)

For Fosved projects with consistent brand: shadcn/ui base + custom theming via design tokens. Best of both.

## Documentation

Each component needs:

**Code-level (docstrings):**
```tsx
/**
 * Primary call-to-action button.
 * 
 * @example
 * <Button onClick={save}>Save changes</Button>
 * <Button variant="destructive" onClick={delete}>Delete</Button>
 */
export function Button({ ... }) { ... }
```

**Storybook (or similar):**
- All variants demonstrated
- All states (default/hover/focus/etc.)
- All sizes
- All edge cases (long text, icons, etc.)
- Code examples

For Fosved: Storybook for any project with 10+ components. Smaller: README.md with examples.

## Versioning component library

When library is shared across projects (separate package):

- Strict SemVer
- Major bump for breaking API changes
- Minor for new components or new variants
- Patch for fixes

Internal-only libraries (one project): looser, but document changes.

## Component testing

Each component tested at multiple levels (see `qa/unit-testing-craft` and `qa/e2e-testing-with-playwright`):

**Rendering tests:**
```tsx
test('renders with text', () => {
  render(<Button>Click me</Button>);
  expect(screen.getByText('Click me')).toBeInTheDocument();
});
```

**Interaction tests:**
```tsx
test('calls onClick when clicked', async () => {
  const handleClick = jest.fn();
  render(<Button onClick={handleClick}>Click</Button>);
  await userEvent.click(screen.getByRole('button'));
  expect(handleClick).toHaveBeenCalled();
});
```

**Accessibility tests:**
```tsx
test('has no accessibility violations', async () => {
  const { container } = render(<Button>Click</Button>);
  const results = await axe(container);
  expect(results).toHaveNoViolations();
});
```

**Visual regression** (optional): Chromatic, Percy snapshot library renderings.

## Build vs buy decision

For each component category:

**Build** when:
- Brand-critical (buttons, cards — your style)
- Already mostly there with shadcn/ui starting point
- Customization needs justify maintenance

**Buy/use library** when:
- Complex behavior (data tables, calendars, rich text editors)
- Solved problem (Radix for primitives)
- Maintenance cost would exceed library cost

For Fosved 2026 recommended stack:
- Build: brand-specific buttons, cards, forms
- Use: Radix UI for primitives (accessibility solid)
- Use: TanStack Table for data tables
- Use: TanStack Form for complex forms
- Use: Recharts for charts (already on miniapp)

## Anti-patterns

- **God components.** One component handles 20 responsibilities via 30 props. Decompose.
- **Inconsistent APIs.** One component uses `disabled`, another `isDisabled`. Standardize.
- **Missing states.** Component without focus state — keyboard users can't see where they are.
- **No documentation.** Components without examples. Consumers guess at API.
- **Breaking changes without major bump.** Consumers' code breaks unexpectedly.
- **Tight coupling to specific data.** Component only works with one API shape. Not reusable.
- **Library too large.** 200 components, half unused. Trim.
- **Library too small.** Every project still implementing its own forms. Increase coverage.
- **No accessibility.** Components work for sighted mouse users only. Tests catch this — run them.
- **Style coupling.** Component dictates margins of its container. Use composition.
- **Hardcoded values.** Component has `padding: 12px` literal. Use design tokens.

## Maintenance discipline

Quarterly audit:
- Usage stats per component (which used often, which unused)
- Issue list per component (bugs, requested features)
- Breaking change candidates (need batching for major version)
- Accessibility re-test
- Visual regression run

Retire unused components. Their maintenance has cost.

## Integration

- `design-tokens-discipline` provides values components use
- `accessibility-first` is requirement for every component
- `interaction-states` defines required states
- `dev-handoff-specs` describes how Dev consumes library
- `qa/unit-testing-craft` tests components
- `architecture-decision-records` documents library choices

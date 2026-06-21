---
name: refactoring-discipline
description: Refactoring without breaking things — small steps, test-protected changes, when to refactor vs leave, recognizing code smells. Strangler-fig pattern for large refactors. The discipline that keeps codebases maintainable instead of accumulating debt until rewrites become necessary.
---

# Refactoring Discipline — Improve Without Breaking

Refactoring is changing structure without changing behavior. Done well, it keeps codebase healthy. Done poorly, it introduces bugs and wastes time. Discipline determines which.

## Prerequisites

- Code exists and has tests (refactoring without tests is just hoping)
- Specific reason to refactor (not "felt like it")
- Time budget for refactor

## Core principle

> Refactoring is structural change with behavioral preservation. If behavior changes, it's not refactoring — it's a feature change. Two distinct activities. Don't mix them. Refactor first (verify nothing broke), then add feature (verify works).

## When to refactor

**Yes, refactor:**
- Same logic duplicated 3+ times (DRY violation)
- Function exceeds 50 lines or 4 levels of nesting
- Class/module has 7+ responsibilities (single responsibility violation)
- Code requires re-reading every time to understand
- Adding feature is harder than it should be (design issue)
- Naming is misleading or inconsistent
- Tests are hard to write (often signals design problem)

**No, don't refactor:**
- "I would have done it differently" — aesthetic preference, not technical issue
- Code works and isn't being modified
- Major refactor near a deadline
- Without tests covering the changing area
- Just discovered code — let yourself understand it first
- Working code that runs once a year — leave it

**Boy Scout rule:** leave code slightly cleaner than you found it. Small improvements accumulate.

## Code smells (signals to refactor)

**Duplication:**
- Same code copy-pasted in multiple places. Extract function.
- Similar code with minor variations. Parameterize.
- Magic numbers/strings. Extract to constants.

**Long methods:**
- Function does too many things. Extract sub-functions.
- Method has comments separating "sections". Each section is candidate for separate function.

**Large classes/modules:**
- File >500 lines. Likely too much in one place.
- Class has 10+ methods of unrelated concern. Split.

**Long parameter lists:**
- Function takes 6+ params. Use object/struct.
- Boolean flags. Often signals function does two things.

**Divergent change:**
- Same file modified for different reasons frequently. Wrong boundary.

**Shotgun surgery:**
- One conceptual change requires editing 5 files. Wrong boundary other direction.

**Feature envy:**
- Method uses another object's data more than its own. Belongs there.

**Inappropriate intimacy:**
- Class accesses internals of another. Hide implementation.

**Comments explaining what:**
- "// loop through users to find admin" — name the function `findAdmin` instead.
- "// hack to work around bug" — fix or document at higher level.

**Speculative generality:**
- Abstract base classes with one concrete. Premature flexibility. Remove until needed.

## Small steps

Refactor in tiny verified increments:

1. Identify smell
2. Form refactor plan
3. **Run tests, confirm green**
4. Make smallest meaningful change
5. **Run tests, confirm still green**
6. Commit
7. Next small change
8. **Run tests**
9. Commit
10. ...

Never accumulate changes without test runs. Each step independently safe.

## Common refactor recipes

### Extract function

Before:
```typescript
function processOrder(order) {
  // ... 30 lines of validation ...
  // ... 40 lines of business logic ...
  // ... 20 lines of notifications ...
}
```

After:
```typescript
function processOrder(order) {
  validateOrder(order);
  applyBusinessRules(order);
  sendNotifications(order);
}

function validateOrder(order) { /* 30 lines */ }
function applyBusinessRules(order) { /* 40 lines */ }
function sendNotifications(order) { /* 20 lines */ }
```

Now each function testable independently. Names document intent.

### Extract type

Before:
```typescript
function createUser(name: string, email: string, age: number, country: string, marketing: boolean) {
  // ...
}
```

After:
```typescript
interface CreateUserInput {
  name: string;
  email: string;
  age: number;
  country: string;
  marketing: boolean;
}

function createUser(input: CreateUserInput) {
  // ...
}
```

Easier to extend (add field without breaking calls), better with destructuring.

### Replace conditional with polymorphism

Before:
```typescript
function area(shape) {
  if (shape.type === 'circle') return Math.PI * shape.radius ** 2;
  if (shape.type === 'square') return shape.side ** 2;
  if (shape.type === 'triangle') return 0.5 * shape.base * shape.height;
}
```

After:
```typescript
class Circle { area() { return Math.PI * this.radius ** 2; } }
class Square { area() { return this.side ** 2; } }
class Triangle { area() { return 0.5 * this.base * this.height; } }
```

Adding new shape: add class. Don't modify existing.

Use sparingly — sometimes if/else is fine.

### Replace magic number

Before:
```typescript
if (age >= 18) { allowAccess(); }
```

After:
```typescript
const LEGAL_ADULT_AGE = 18;
if (age >= LEGAL_ADULT_AGE) { allowAccess(); }
```

Intent clear. Easy to find all references.

### Introduce parameter object

Before:
```typescript
function fetchOrders(userId, status, limit, offset, sortBy, sortDirection) { }
```

After:
```typescript
interface FetchOrdersOptions {
  userId: string;
  status?: string;
  pagination?: { limit: number; offset: number };
  sort?: { by: string; direction: 'asc' | 'desc' };
}
function fetchOrders(options: FetchOrdersOptions) { }
```

### Replace nested conditional with guard clauses

Before:
```typescript
function process(user) {
  if (user) {
    if (user.active) {
      if (user.hasPermission) {
        // do work
      }
    }
  }
}
```

After:
```typescript
function process(user) {
  if (!user) return;
  if (!user.active) return;
  if (!user.hasPermission) return;
  // do work
}
```

Less nesting, intent clearer.

## Large refactors (strangler fig pattern)

For refactor too big for single sitting:

**Strangler fig** (from Martin Fowler): gradually replace old with new, route traffic gradually.

```
Stage 1: All traffic → Old system
Stage 2: New system built alongside old, no traffic
Stage 3: Some traffic → New (10%)
Stage 4: More traffic → New (50%)
Stage 5: All traffic → New
Stage 6: Old removed
```

Each stage shippable. Rollback easy at any stage. Never big-bang switchover.

**Branch by abstraction:** introduce interface, switch implementations.

```
Step 1: Define interface
Step 2: Wrap old code to fit interface
Step 3: Build new implementation
Step 4: Switch at runtime via flag
Step 5: Remove old
```

## Tests as refactor protection

**Before refactor:** ensure tests cover the area you're changing.

If coverage thin: **write tests first**. They protect from accidental behavior changes.

**Don't refactor without tests.** You'd be making changes blind. Mistakes compound.

If code is too tangled to test: that itself is the refactor. **Refactor to make testable first, then continue.**

## When tests fail during refactor

If you ran tests after each step:
- Last change broke them
- Revert last change
- Try smaller step
- Or understand why and fix

If you didn't run tests after each step:
- You don't know which change broke
- May need to undo entire session
- Lesson: smaller steps + tests every time

## Refactor commits

Separate commits from feature commits.

Bad:
```
feat: add user export with refactored validation
```

Good:
```
refactor: extract validation logic to validators module
feat: add user export endpoint
```

Why: reviewers can verify refactor is behavior-preserving by looking at just that commit. Feature added cleanly on top.

## Refactoring isn't free

Time cost. Risk of bugs. Opportunity cost.

**Trade-offs:**
- Code only edited by you, you understand it → low value to refactor
- Code read/edited by team, hard to understand → high value
- Code about to be replaced → don't refactor, replace
- Code in critical path → high cost of bug, high care needed

Match refactor effort to expected payoff.

## Anti-patterns

- **"Refactor" with behavior changes.** Mixing two activities. Bugs harder to attribute.
- **No tests, just hope.** Hope is not a strategy. Bugs WILL happen.
- **Big bang refactor.** Days of changes without commits. When it breaks, hard to find what.
- **Refactor before deadline.** Risk vs reward poor. Wait until after.
- **Beautify code.** Personal preferences, not technical improvements. Time better spent elsewhere.
- **Over-abstraction.** Creating frameworks for one use case. YAGNI.
- **Refactor stable code that's never edited.** Time wasted.
- **Refactor with broken tests.** Tests already broken — no protection.
- **Refactor away from existing pattern in codebase.** Now codebase is inconsistent. Either commit to migration or leave.
- **Public API changes without version bump.** Breaks consumers.

## Recognizing when NOT to refactor

Some code is intentionally complex because the problem is complex. Trying to "clean up" results in worse code or lost subtlety.

If you find:
- "This looks weird but probably has a reason"
- Comments explaining "must do X because Y"
- Code that worked through painful debugging history

Read git log, blame, talk to author. Understand WHY before refactoring. Sometimes answer is "leave it".

## Refactor documentation

For significant refactors:
- ADR explaining decision (what, why, alternatives)
- Update CHANGELOG: `refactor: <description>`
- PR description includes before/after explanation
- Mention in commit messages with `refactor:` prefix

This becomes the project's refactor history. Future you knows what happened and why.

## Automated refactors

Modern tools (TypeScript LSP, ESLint with rules, Prettier) automate trivial refactors:
- Rename symbol (changes all references)
- Extract function (selection → function)
- Move declaration
- Convert to template literal
- Remove unused imports

Use these. Trust them. They're faster than manual and less error-prone.

For complex refactors: manual, with tests.

## Integration

- `code-organization-standards` defines structure refactor should preserve
- `iterative-implementation` discipline applies (small steps)
- `qa/unit-testing-craft` provides test coverage for refactor protection
- `qa/regression-test-discipline` ensures regression caught
- ADR for significant refactors
- `architecture-decision-records` captures large refactor decisions

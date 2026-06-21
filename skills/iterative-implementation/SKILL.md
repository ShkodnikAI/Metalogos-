---
name: iterative-implementation
description: Discipline for implementing code in small verifiable iterations rather than big-bang attempts. Each iteration produces working code that passes basic tests, even if incomplete. Counter-pattern to "I'll write it all then test" which produces unrecoverable debugging marathons. Foundation for predictable delivery and continuous integration.
---

# Iterative Implementation — Small Working Pieces, Not Big Broken Wholes

The most common engineering failure mode: write 800 lines of code, run it once, it fails, can't tell where. Iterative implementation prevents this by demanding working code at every step — even tiny steps.

## Prerequisites

- DevTask with ADR (decision recorded)
- Repository set up per `code-organization-standards`
- Test framework configured (even minimal)

## Core principle

> Working code at every commit. Not "almost working", not "should work", but actually running tests passing. If you go more than 30-60 minutes without a green test run, you're doing it wrong. The discipline of working state forces small steps and prevents debt accumulation.

## The iteration cycle

```
1. Write smallest meaningful test (failing)
2. Write minimum code to make it pass
3. Refactor with test still passing
4. Commit
5. Repeat
```

This is TDD's red/green/refactor — but the discipline matters regardless of whether you actually write tests first. The point is: **working state at every commit**.

## What "smallest meaningful step" means

Examples of right-sized steps:

- Add one route handler that returns hard-coded response
- Add one model field with migration
- Add one component that renders one prop
- Add one utility function with one test case
- Wire up one API call that returns mock data

NOT:
- "Build the whole user system"
- "Implement authentication"
- "Add the admin panel"

If a step takes >30 minutes, it's too big. Break it down.

## Working state checklist (every commit)

Before committing:
- [ ] Code compiles (TypeScript: `tsc --noEmit`)
- [ ] Linter passes (`npm run lint`)
- [ ] Existing tests still pass (`npm test`)
- [ ] New tests pass (if added)
- [ ] Manual smoke check (if UI: page loads; if API: endpoint responds)

If any fails: fix before commit. Don't push broken state to feature branch.

## Commit frequency

Typical session: 5-15 commits per task.

- Big task (8 hours): ~15-20 commits, each 20-30 min of work
- Medium task (2-3 hours): ~5-8 commits
- Small task (30 min): 1-2 commits

If you have 1 commit for 8 hours of work, you didn't iterate — you batched. Bad.

## Conventional commits in iterative work

Each iteration commit follows convention:

```
feat(users): add User model with email/name fields
feat(users): add createUser service function
feat(users): wire createUser to POST /users endpoint
test(users): cover createUser validation cases
fix(users): handle duplicate email error
refactor(users): extract validation to schema
```

Notice: feature builds up across multiple commits. Each commit is itself green.

## Branch state management

Feature branch should be **rebaseable and reviewable**:

- Commits in logical sequence
- Each commit message describes what it adds
- Squash before merge if commits are too granular (5+ "fix typo" commits → squash)

But during active work, keep commits granular. Squashing happens at merge time, not during iteration.

## When work-in-progress is okay

Sometimes you need to commit non-working state (end of day, switching tasks). Convention:

```
WIP: implementing user search — partial, tests failing
```

WIP commits:
- Prefix message with `WIP:`
- Push to feature branch only (never merge)
- Either complete next session OR rebase to clean up before merge
- WIP allowed on feature branch, FORBIDDEN on main

## Smoke testing each iteration

Beyond automated tests, run manually:

- API change → curl the endpoint, verify response
- UI change → open in browser, verify renders + basic interaction
- DB migration → check schema applied, query the table
- Deploy hook → trigger deploy, verify service responds

This catches what tests miss (config issues, environment differences, integration bugs).

## Mock data strategy

Iterating doesn't mean waiting for everything to be real. Use mocks aggressively at first:

- API not built? Hardcode response in client
- DB not set up? In-memory mock
- Auth not done? Hardcoded user
- LLM not connected? Stub function returning canned text

Then replace mocks incrementally. Each replacement is its own iteration:
- "Replace mock with real DB call"
- "Replace stub with real LLM"
- "Add real auth in place of hardcoded user"

This lets you build the architecture before all pieces are real.

## The "if I died" test

If you stop working right now, can someone else (or future you) continue from this commit?

- Code compiles? ✓
- Tests describe what's expected? ✓
- README mentions WIP status? ✓
- TODO comment explains next step? ✓

If yes — safe to stop. If not — finish the iteration first.

## Pair this with PR-based review

Even for one-owner setup:

1. Work on feature branch (multiple small commits)
2. Open PR to main
3. Review your own diff (yes, seriously) — catches obvious bugs
4. CI must pass before merge
5. Merge with squash (or rebase) producing clean main history

Self-review surfaces issues you missed during writing. The cognitive shift from "writing" to "reviewing" reveals problems.

## Anti-patterns

- **Big bang implementation.** "I'll write the whole module then run it." When it fails, you have no idea where. Hours of debugging.
- **Skipping smoke tests.** "Tests pass, commit." But you didn't actually run the thing. Tests can be wrong or incomplete.
- **WIP on main.** Broken state on main breaks others (or future you).
- **Squashing too aggressively.** All 15 commits squashed to 1 = lost the iterative history. Squash similar/cleanup commits, keep logical milestones.
- **Vague commit messages.** "fix stuff", "wip", "updates" — useless. Always describe what changed.
- **Disabling tests to commit.** "I'll re-enable later" — never happens. Fix the test or fix the code.
- **Manual testing only.** No automated tests means no regression protection.
- **Mock that becomes permanent.** Mock without TODO for replacement = forgotten mock in production.
- **One PR with 50 commits.** Either iterate in branch and squash, or split into multiple PRs by logical milestone.

## When iterative is hard

Some tasks resist iteration:
- Complex algorithm where partial code is meaningless
- Schema migration that's all-or-nothing
- Replacing one library with another wholesale

For these:
- Iterate in **spike** branch (experimental, throwaway)
- Once approach is clear, **rewrite from scratch** in main branch with iteration
- Keep spike for reference, don't merge

## Time-boxing iterations

Each iteration: 20-60 minutes typical.

If a "single iteration" exceeds 90 minutes:
- Stop
- Identify what's making it big
- Decompose
- Resume

Spending 4 hours on "one iteration" means it was actually 6 iterations badly combined.

## End-of-task verification

When all iterations complete:
- All tests pass
- Coverage acceptable
- README updated if external API changed
- CHANGELOG.md entry added
- Manual smoke check on integrated whole
- Self-review of complete diff in PR

Then ready for QA handoff.

## Integration

- `iterative-implementation` enforced for every Tier 2 dev skill
- `estimation-discipline` decomposes large tasks into iterations
- `architecture-decision-records` written before iterations begin
- `lib/dev.js` `commitsCount` field tracks iteration adherence
- Low commit count + long duration triggers monthly review flag ("did we iterate or batch?")

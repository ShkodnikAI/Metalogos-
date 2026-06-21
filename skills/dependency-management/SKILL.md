---
name: dependency-management
description: Discipline for adopting, updating, and removing dependencies. Each dependency is a long-term liability — bundle size, attack surface, maintenance burden, abandonment risk. Adopt deliberately, update systematically (not reactively), remove aggressively. Without dependency discipline, projects accumulate dead weight that slows builds, increases security exposure, and creates upgrade hell.
---

# Dependency Management — Each Library is a Liability

A new dependency feels free. `npm install some-lib` and it works. But you've taken on long-term liabilities you don't see:
- Maintenance burden (security updates, version conflicts)
- Bundle size (every kb matters for web apps)
- Attack surface (supply chain compromises happen)
- Lock-in (some deps spread through codebase)
- Abandonment risk (maintainer disappears, library becomes orphan)

This skill is the discipline.

## Prerequisites

- `tech-radar-maintenance` — dependency decisions feed into Tech Radar

## Core principle

> Every dependency is a long-term marriage, not a one-night stand. Before installing, ask: am I willing to maintain awareness of this package for the next 3 years? If no, write the code yourself or find a smaller alternative.

## Three-question gate for new dependencies

Before adding any new dependency:

### 1. Can I write this myself in <50 lines?

A small utility (slugify, debounce, retry) is often 10-30 lines of code. Installing a library means accepting:
- Versioning concerns
- Bundle size
- Potentially deep dependency tree (one lib pulls in 20)
- Attack surface

If you can write it in <50 lines with reasonable understanding — write it. Save dependency budget for things you genuinely can't write (Prisma, React, Next.js).

### 2. Is this dependency healthy?

Healthy signals:
- Released within last 6 months
- Active maintainer responding to issues
- 500+ weekly downloads minimum (avoid abandoned packages)
- Repository visible (not yarn-lock-only)
- Clear license
- TypeScript support (for TS projects)
- No known critical vulnerabilities

Quick check:
```bash
npm view <package> versions --json | tail -5
npm view <package> time --json | head -20
```

Last published date matters. >2 years stale = abandoned.

### 3. What does it replace?

Adding a dep without removing/consolidating one is silent inflation. Track in dependency log:
- "Adding chalk for terminal colors → removing custom ansi-codes file"
- "Adding bcrypt → replaces home-rolled hash (which was wrong anyway)"

If you can't articulate what it replaces or enables — reconsider.

## Dependency types and policies

### Production dependencies (`dependencies`)

End up in deployed bundle. Strict discipline:
- Tech Radar adopt state required (or trial with explicit ADR)
- Audited for security
- Reviewed for bundle size impact
- Updated quarterly minimum

### Dev dependencies (`devDependencies`)

Build/test tooling. More forgiving:
- Doesn't ship to production
- Updated when convenient
- Still security-audited

### Optional/peer dependencies

Less common. Document the choice — peer dependency means consumers must install. Use cautiously.

### Transitive dependencies

You don't install them but they install via your direct deps. Hardest to manage. Tools:
- `npm ls <package>` — see who depends on what
- `npm audit` — security across full tree
- `npm dedupe` — flatten duplicates

## Update strategy

### Patch updates (1.2.3 → 1.2.4)

Auto-update via:
- Dependabot (GitHub native, free)
- Renovate (more configurable)

Configuration: auto-merge patch updates that pass CI. Bot opens PRs daily.

### Minor updates (1.2.3 → 1.3.0)

Manual review monthly:
- Skim CHANGELOG
- Check for behavioral changes
- Run full test suite
- Update if safe

### Major updates (1.x → 2.x)

Treat as architecture decision:
- Write ADR
- Read migration guide
- Allocate time (often non-trivial)
- Major updates may take days, not minutes

### Security updates

Immediate, regardless of timing:
- `npm audit fix` for automatic safe fixes
- For breaking security fixes: emergency deploy
- Track via `qa/security-testing-protocol` workflow

## Removal discipline

Quarterly: review what's unused.

```bash
# Find unused dependencies
npx depcheck

# Find what depends on a specific package
npm ls some-package
```

Aggressively remove:
- Packages with 0 usage
- Packages used only in deleted code
- Packages where you've migrated away but didn't remove

Each removal: smaller bundle, smaller attack surface, faster `npm install`.

## Lock file discipline

`package-lock.json` (npm) or `pnpm-lock.yaml` or `yarn.lock` — commit it. Never gitignore.

Lock files ensure:
- Reproducible builds (same versions everywhere)
- Visible transitive changes (PR shows what got updated)
- Audit trail of versions over time

Periodically: `npm shrinkwrap` or lockfile maintenance.

## Supply chain security

Real risks (not theoretical):
- Typosquatting: `react-domn` instead of `react-dom`
- Compromised packages: maintainer account hijacked
- Malicious updates: legitimate package becomes evil
- Postinstall scripts: arbitrary code runs on install

Mitigations:
- Verify package names carefully when installing
- Pin exact versions for production deps (`1.2.3`, not `^1.2.0`) for critical packages
- Review postinstall scripts: `npm install --ignore-scripts` for inspection
- Use `npm ci` (not `npm install`) in CI for strict lockfile use
- Snyk or similar for ongoing vulnerability monitoring

## Bundle size discipline (web apps)

Every dep counts. Tools:
- `bundlephobia.com` — check before installing
- `webpack-bundle-analyzer` — see what's in your bundle
- `import-cost` VS Code extension — see size at import time

Rules:
- Anything over 50kb minified: requires ADR
- Anything over 200kb: requires alternative analysis
- Anything over 500kb: alternative MUST be used unless absolutely critical

Heavy libraries to be cautious of:
- `moment` (use date-fns or native Intl)
- `lodash` (import individually or use native ES)
- Whole UI libraries (use composable headless: Radix, shadcn)

## Anti-patterns

- **`npm install` without thinking.** Each addition needs justification.
- **No lockfile.** Builds become non-reproducible. Disaster waiting.
- **Updating only when something breaks.** Then you have huge multi-version jumps. Update incrementally.
- **Ignoring security alerts.** Github + Dependabot will flag them. Ignoring is negligence.
- **Pinning everything tightly.** Pinning `^1.2.3` to `1.2.3` for every dep means missing patch fixes. Pin critical, range non-critical.
- **Never removing deps.** Add-only mode accumulates dead weight.
- **Installing into wrong category.** dev tools in `dependencies` ship to production unnecessarily.
- **Trusting popularity blindly.** `npm install left-pad` was a thing. Popularity ≠ quality ≠ security.

## Quarterly dependency audit

Every quarter:
1. `npm outdated` — list of update candidates
2. `npm audit` — security vulnerabilities
3. `npx depcheck` — unused packages
4. Review against Tech Radar — anything in `hold` state?
5. Update in batches (patch first, then minor, major separately)
6. Run full test suite after each batch
7. Document major changes in CHANGELOG

This is a 1-2 hour ritual per project. Prevents 1-2 month upgrade emergencies later.

## Integration with other skills

- `tech-radar-maintenance` — dependencies feed radar state
- `security-hardening` — vulnerability scanning ties in
- `devops-deployment` — CI runs `npm audit` on every build
- `qa/security-testing-protocol` — dependency audit part of every release
- `architecture-decision-records` — ADRs for major dependency choices and updates

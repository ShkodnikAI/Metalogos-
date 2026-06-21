---
name: code-organization-standards
description: Standards every Fosved repository follows — folder structure, file naming, commit conventions, branch strategy, required files (README, LICENSE, CHANGELOG, .gitignore, CI workflow). Standards exist not for purity but for navigation speed — when every repo follows the same pattern, finding things and onboarding is instant.
---

# Code Organization Standards — One Pattern Across All Projects

A repo with random structure forces you to relearn it every time you return. A repo that follows standards is navigable in 30 seconds. Multiply by N repos, the difference is enormous.

## Prerequisites

- Creating new repo, or auditing existing one for compliance
- Knowing repo's project type (web_app | site | ai_office | local_ai | library | other)

## Core principle

> Standards are not bureaucracy. They are the **shared muscle memory** that lets you work fast across many repos. The cost of standards is felt once (at adoption). The benefit compounds for years.

## Mandatory files (every repo has these)

### README.md (root)

Structure:
```markdown
# Project Name

One-sentence description.

## Quick start
[3-5 lines: install, run, test]

## What this is
[2-3 paragraphs: purpose, audience, current state]

## Tech stack
- Language: <name + version>
- Framework: <name + version>
- Database: <name + version> (if any)
- Key libraries: <bullet list>

## Project structure
[Tree showing main folders, with one-line description each]

## Development
- Setup: [commands]
- Run dev server: [command]
- Run tests: [command]
- Build: [command]
- Deploy: [command]

## Environment variables
[List with descriptions, mark required/optional]

## License
See LICENSE file.

## Project metadata
- Created: YYYY-MM-DD
- Last updated: YYYY-MM-DD
- Status: alpha | beta | stable | maintenance | deprecated
- Owner: @ShkodnikAI
```

README is **outwardly-facing**. Even if private repo, write as if a stranger needed to understand it.

### LICENSE

Standard: MIT for libraries/tools, proprietary for client work.

If MIT: full text in file.
If proprietary: explicit statement "All rights reserved. Not licensed for redistribution."

Don't omit LICENSE — legally ambiguous repos are toxic.

### CHANGELOG.md

Format: [Keep a Changelog](https://keepachangelog.com/) standard.

```markdown
# Changelog

## [Unreleased]
### Added
- feature in progress

## [1.2.0] - 2026-05-15
### Added
- New feature X (PR #42)
### Changed
- Updated dependency Y
### Fixed
- Bug Z (issue #38)

## [1.1.0] - 2026-04-20
...
```

Updated on every release. Not optional.

### .gitignore

Use language-appropriate template + project specifics:
- Node.js: `node_modules/`, `.env*`, `dist/`, `.next/`, `coverage/`, `.DS_Store`
- Python: `__pycache__/`, `*.pyc`, `.venv/`, `.env*`, `dist/`, `*.egg-info/`
- Always: `.env` (never commit secrets), IDE folders (`.vscode/`, `.idea/`)

Use `gitignore.io` for starting template.

### .github/workflows/ci.yml (minimum)

Even smallest project has CI. Minimum:
```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
      - run: npm ci
      - run: npm run lint
      - run: npm test
      - run: npm run build
```

If no tests yet: at minimum lint + build. CI catches "doesn't even build" before merge.

## Folder structure standards

### Web app (Next.js)

```
project/
├── README.md
├── LICENSE
├── CHANGELOG.md
├── .gitignore
├── .github/workflows/ci.yml
├── package.json
├── package-lock.json
├── tsconfig.json
├── next.config.js
├── tailwind.config.ts
├── .env.example     # template, never .env
├── docs/
│   ├── adr/        # Architecture Decision Records
│   └── api/        # API documentation
├── src/
│   ├── app/        # Next.js app router
│   ├── components/ # React components
│   ├── lib/        # Shared utilities
│   ├── hooks/      # React hooks
│   ├── types/      # TypeScript type definitions
│   └── styles/     # Global styles
├── prisma/         # If using Prisma
│   ├── schema.prisma
│   └── migrations/
├── public/         # Static assets
└── tests/
    ├── unit/
    ├── integration/
    └── e2e/
```

### Backend service (Node.js)

```
project/
├── README.md
├── LICENSE
├── CHANGELOG.md
├── .gitignore
├── .github/workflows/ci.yml
├── package.json
├── tsconfig.json
├── .env.example
├── docs/adr/
├── src/
│   ├── server.ts        # entry point
│   ├── routes/          # HTTP routes
│   ├── services/        # business logic
│   ├── lib/             # utilities
│   ├── middleware/
│   └── types/
├── prisma/
└── tests/
```

### Static site

```
site/
├── README.md
├── LICENSE
├── CHANGELOG.md
├── .gitignore
├── package.json
├── .github/workflows/ci.yml
├── docs/adr/
├── content/         # MDX or markdown
├── public/
├── src/
│   ├── app/         # or pages/
│   ├── components/
│   └── lib/
└── tests/
```

### AI office (multi-agent, like fosved-bot)

```
project/
├── README.md, LICENSE, CHANGELOG.md, .gitignore
├── .github/workflows/ci.yml
├── package.json
├── prisma/schema.prisma
├── docs/adr/
├── library/         # specialist profiles
├── skills/<org>/    # skill .md files
├── lib/             # functional modules (e.g., research.js, archive-publisher.js)
├── bot.js           # main bot logic (or src/bot.ts)
├── scheduler.js
└── tests/
```

### Library/package

```
library/
├── README.md, LICENSE, CHANGELOG.md
├── package.json     # with publish config
├── tsconfig.json
├── .github/workflows/{ci.yml, publish.yml}
├── src/
│   ├── index.ts     # public API
│   └── ...
├── docs/
│   ├── api.md
│   └── examples/
└── tests/
```

## File naming conventions

- **React components**: PascalCase — `UserCard.tsx`, `ProfilePage.tsx`
- **Hooks**: camelCase with `use` prefix — `useUser.ts`, `useAuth.ts`
- **Utilities**: camelCase — `formatDate.ts`, `parseConfig.ts`
- **Types**: camelCase — `userTypes.ts`, `apiResponses.ts`
- **Configs**: kebab-case — `next.config.js`, `tailwind.config.ts`
- **Skills**: kebab-case — `architecture-decision-records/SKILL.md`
- **Library .md profiles**: lowercase — `dev.md`, `design.md`, `qa.md`

Don't mix conventions in one project.

## Branch strategy

```
main           # production, protected, requires PR
├── feature/*  # new work
├── hotfix/*   # urgent production fixes (skip PR review if critical, retroactive review)
├── refactor/* # non-functional improvements
└── chore/*    # tooling, dependencies
```

For one-owner repo: PR yourself. Forces review of your own diff before merge. Catches obvious mistakes.

**Protect `main`:** require CI green + at least 1 review approval (even self-review). Configure in GitHub repo settings.

## Commit message convention

Conventional Commits format:

```
<type>(<scope>): <short description>

[optional body]

[optional footer]
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`, `build`, `ci`, `revert`

Examples:
```
feat(auth): add OAuth provider for GitHub
fix(api): handle null user in /me endpoint
docs(readme): update setup instructions
refactor(db): extract repository pattern from services
chore(deps): bump prisma to 6.12
```

Body: explain WHY when not obvious. Reference issues/tasks: `Fixes #42` or `Refs DevTask-7`.

Breaking changes: footer `BREAKING CHANGE: <description>` triggers major version.

## Versioning

SemVer strictly: `MAJOR.MINOR.PATCH`

- MAJOR: breaking API changes
- MINOR: backwards-compatible features
- PATCH: bug fixes

Tags: `v1.2.3` (with `v` prefix). Tag every release.

For pre-1.0: `0.x.y` where x is "minor" but anything can break. Document this in README.

## Required content in every repo

Even minimal repo has:
- README with working quick start
- LICENSE
- .gitignore
- CHANGELOG (even if just "0.1.0 - initial")
- One working test (smoke level minimum)
- CI workflow that runs lint + build

The "everything optional except code" mentality leads to maintenance nightmare. Demand the floor.

## Anti-patterns

- **Random folder names.** `stuff/`, `utils/`, `helpers/`, `misc/` — junk drawers. Be specific.
- **Multiple sources of truth.** Config in 3 places. Pick one.
- **Mixing concerns.** `components/` with API routes inside. Separate by role.
- **No .env.example.** New contributor doesn't know what variables needed.
- **Committed .env.** Critical security failure.
- **Missing README sections.** "I know what this is" — until 6 months pass and you don't.
- **Inconsistent naming.** `userCard.tsx` next to `ProfilePage.tsx`. Pick convention, enforce.
- **No CHANGELOG.** Releases without notes. Future you can't reconstruct what changed when.
- **Hardcoded paths.** Absolute paths from your machine. Breaks on others' systems.
- **TODO comments instead of issues.** TODOs in code rot. File issues instead.
- **No CI.** "Trust me it works" — until it doesn't.

## Initial setup script

For new repo creation, `lib/dev.js` `createRepository()` runs:

```
1. Create GitHub repo via Octokit
2. Initialize with template files (README, LICENSE, CHANGELOG, .gitignore, CI)
3. Create initial directory structure per project type
4. Configure branch protection on main
5. Set repo description, topics, visibility
6. Record CodeRepository entry in DB
7. Publish creation announcement to Telegram Archive
```

This enforces standards at creation. New repo can't violate standards.

## Audit existing repos

For pre-existing repos, periodic audit:
- Run `npm run audit-standards` (custom script) on each
- Score against checklist (README sections present? LICENSE? CHANGELOG up to date? CI passes? .env.example?)
- Generate report
- Create issues for non-compliance
- Track over time

Audit run is part of quarterly Dev review.

## Integration

- `code-organization-standards` is loaded for every Dev session at Tier 1
- `lib/dev.js` `createRepository()` uses this skill's templates
- `lib/dev.js` `auditRepository()` checks compliance
- Tech radar entries for tools/libraries reference standards (which tools fit the structure)

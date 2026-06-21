---
name: devops-deployment
description: CI/CD pipelines, deployment strategies, environment management, secrets handling for Fosved projects. From local dev → staging → production. GitHub Actions workflows. Default deployment targets per project type. Operational discipline.
---

# DevOps Deployment — From Code to Production Reliably

A working app on your laptop is not a deployed app. Deployment is engineering discipline that turns code into reliably-running services. This skill is the path.

## Prerequisites

- Repository follows `code-organization-standards`
- CI workflow file present
- Deployment target known (Render, Vercel, Cloudflare, self-hosted)

## Core principle

> Deployment is code. Manual deploys ("ssh into server and pull") don't scale and create heroes. Automated deploys are reproducible, auditable, and rollback-able. Investment in deployment automation pays back from project 1.

## Default deployment targets

By project type:

| Type | Default | Why |
|------|---------|-----|
| Next.js web app | Render or Vercel | Native support, easy setup |
| Static site | Cloudflare Pages | Free, fast CDN |
| Telegram bot (Node) | Render Background Worker | Long polling support, free tier viable |
| API service | Render or Cloudflare Workers | Auto-scaling |
| Database | Supabase | Postgres managed, free tier |
| AI office (multi-component) | Render (for bot) + Render Web (for miniapp) | Existing pattern |
| Local AI | Self-hosted | Hardware-dependent |

Tech Radar tracks: Render (adopt), Vercel (adopt), Cloudflare (adopt), Fly.io (assess), AWS/GCP (hold for simple — overkill).

## GitHub Actions baseline

Every repo has minimum CI:

```yaml
# .github/workflows/ci.yml
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
          cache: 'npm'
      - run: npm ci
      - run: npm run lint
      - run: npm test
      - run: npm run build
```

Required: passes before merge to main.

## Deployment pipeline (Next.js → Render)

```yaml
# .github/workflows/deploy.yml
name: Deploy

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Trigger Render deploy
        run: |
          curl -X POST \
            -H "Authorization: Bearer ${{ secrets.RENDER_API_KEY }}" \
            "https://api.render.com/v1/services/${{ secrets.RENDER_SERVICE_ID }}/deploys"
```

For Render with auto-deploy enabled, push to main triggers deploy automatically — no workflow needed.

## Environment strategy

**Local dev:** developer's machine. `.env` file. Real services possible (Supabase free DB) but use mocks where reasonable.

**Staging (optional):** separate Render service for testing before prod. Useful for risky changes. Optional for solo projects.

**Production:** Render service, real users.

**Branch → environment mapping:**
- `feature/*` → no deployment (only CI)
- `main` → production deployment

For more complex setups: `develop` branch → staging, `main` → production. Most Fosved projects don't need this complexity yet.

## Secret management

Secrets NEVER in code or repo:

**Local:** `.env` file (gitignored).

**CI:** GitHub Secrets (Settings → Secrets and variables → Actions).

**Production:** Render env vars (encrypted at rest, set via dashboard or API).

**Categories of secrets:**
- API keys (ANTHROPIC_API_KEY, GITHUB_TOKEN, etc.)
- Database URLs (with credentials)
- Webhook secrets
- Session secrets

**Rotation:** every 6-12 months, or immediately if compromised. Document rotation process.

**.env.example pattern:**
```
# Required
DATABASE_URL=
ANTHROPIC_API_KEY=

# Optional
GROK_API_KEY=    # falls back to Anthropic only if missing
LOG_LEVEL=info
```

New contributor copies `.env.example` to `.env`, fills with real values.

## Database migrations in CI/CD

Migrations are infrastructure changes. Apply with care:

**Strategy 1: Manual.** Owner applies before deploy.
```bash
DATABASE_URL=$PROD_URL npx prisma migrate deploy
```

**Strategy 2: Auto in CI.** Run migrate as part of deploy.
```yaml
- name: Run migrations
  run: npx prisma migrate deploy
  env:
    DATABASE_URL: ${{ secrets.PRODUCTION_DATABASE_URL }}
```

Auto is safer because deploy fails if migration fails — code with new schema doesn't go live without migration.

**Backward compatibility for migrations:**

Bad migration: drop column AND deploy code that doesn't use it in same deploy.
Risk: deploy starts, old code still running, queries old column, errors.

Good migration: two-deploy approach.
- Deploy 1: code stops using column. Migrate to drop. (Or migrate first, code already stopped using).
- Deploy 2: column dropped.

For early-stage projects with no users, just one deploy is fine. For production with users: be careful.

## Rollback strategy

Things go wrong. Plan for it.

**Code rollback:**
```bash
# On Render dashboard: rollback to previous deploy (single click)
# Or via git:
git revert <bad-commit>
git push origin main  # triggers new deploy
```

**Database rollback:**
- Don't rollback migrations in production (data loss risk)
- Deploy new code that's backward-compatible with rolled-back state
- Restore from backup if catastrophic

**Feature flags for safer rollouts:**
```typescript
if (process.env.FEATURE_NEW_THING === 'true') {
  // new behavior
} else {
  // old behavior
}
```

Deploy with flag off. Test. Turn flag on (env var change). Roll back instantly if issues (turn flag off).

## Monitoring after deploy

Don't deploy and forget. Watch metrics:

- Error rate spike → rollback signal
- Latency spike → investigate
- Memory growth → leak introduced

Render dashboard shows basic metrics. For more: Sentry for errors, Datadog/Grafana for system metrics (see `observability-setup`).

**Smoke test post-deploy:**
- Hit health check endpoint
- Verify basic flows work (login, key feature)
- Check logs for errors

Automate with GitHub Actions:
```yaml
- name: Smoke test
  run: |
    sleep 30  # wait for deploy
    curl -f https://yourapp.com/health || exit 1
    curl -f https://yourapp.com/api/healthcheck || exit 1
```

## Zero-downtime deploys

Render handles this by default (new instance up before old terminated).

For self-hosted:
- Blue/green: maintain two environments, switch traffic
- Rolling: replace instances one at a time
- Canary: deploy to 5% first, monitor, then 100%

Most Fosved projects don't need beyond Render default.

## Branch protection

GitHub repo settings → Branches → main → Protect:
- Require PR before merge (even self-merge)
- Require CI passing
- Require linear history (no merge commits)
- Restrict force-push

This catches mistakes before they hit production.

## Deployment hooks

**Pre-deploy:** run tests, type-check, build.

**During deploy:** apply migrations.

**Post-deploy:**
- Smoke test
- Notify Telegram archive ("v1.2.3 deployed")
- Update CHANGELOG if not done

## Multi-service deployment (Fosved bot)

fosved-bot + fosved-miniapp deploy independently:
- Different Render services
- Same Supabase DB (shared)
- Schema changes coordinate between them

Coordination:
- Schema change in bot first (additive — add columns)
- Both deploy
- Then miniapp uses new schema
- Old columns removed in separate migration after both stable

## Local development workflow

```bash
# Setup
git clone repo
cd repo
cp .env.example .env
# Fill in values
npm install
npx prisma migrate dev  # if schema present

# Run
npm run dev    # development server with hot reload

# Test
npm test
npm run lint

# Commit
git checkout -b feature/whatever
# work, commit, push
gh pr create  # GitHub CLI for PR
```

For Fosved bot specifically: run bot locally, test in Telegram via dev bot token.

## Anti-patterns

- **Manual deploys.** SSH into server, pull, restart. Doesn't scale, no audit trail.
- **No CI.** Push and hope. Bugs reach production.
- **Mismatched envs.** Local works, prod doesn't. Different versions, configs.
- **Hardcoded URLs.** `http://localhost:3000` in production code.
- **Secrets in git history.** Even if "deleted", git remembers. Rotate keys immediately.
- **No rollback plan.** When deploy fails, panic.
- **No smoke test.** Deploy and assume good.
- **All-or-nothing deploys.** Big bang release with 50 changes. When broken, hard to bisect.
- **No env separation.** Production DB accessible from local. One bad query destroys production.
- **Long-lived feature branches.** Drift from main, merge conflicts pile up.
- **No CHANGELOG.** Can't tell what was deployed when.

## Render-specific tips (current primary)

- Use Render Disks for persistent storage when needed
- Background Workers for non-HTTP services (Telegram polling)
- Web Services for HTTP
- Postgres add-on or external Supabase
- Free tier good for development; paid for production reliability
- Auto-deploy from GitHub branch (configure in dashboard)
- Custom domains via dashboard
- Environment variables encrypted at rest

## Integration

- `code-organization-standards` mandates CI workflow file
- `database-design` migrations apply in deploy pipeline
- `observability-setup` instruments deployed services
- `security-hardening` checks before deploy
- ADRs document deployment platform choice
- `lib/dev.js` `deployRepo()` triggers deployments

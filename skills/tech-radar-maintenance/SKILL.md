---
name: tech-radar-maintenance
description: Maintaining the Dev department's Tech Radar — a living catalog of technologies in four states (adopt/trial/assess/hold). Defines what we use, what we're piloting, what we're watching, what we avoid — with explicit rationale. Updates quarterly based on project experience + external signals. Without a radar, tech choices are ad-hoc and inconsistent.
---

# Tech Radar Maintenance — Living Catalog of Tech Choices

A team without a tech radar makes a different stack decision every project. "What do we use for X?" gets a different answer every time. Compound learning is impossible because nobody tracks which choices worked.

A tech radar fixes this. Every technology has a documented state, rationale, and history. Decisions become consistent. Anti-patterns (using `hold` items) are flagged automatically.

## Prerequisites

- `architecture-decision-records` skill loaded
- Quarterly review cadence active

## Core principle

> Your tech stack is the result of dozens of micro-decisions. Without a radar, those decisions are reactive and forgotten. With a radar, they're deliberate and revisable. The radar is the team's collective memory of "what works for us, what doesn't, and why".

## The four states

### Adopt
**Definition:** technology we use in production without hesitation.

**Criteria for entry:**
- Used in 2+ projects successfully
- Maintainer is active (commits/releases in last 6 months)
- No known security/performance blockers
- Team has working knowledge
- Documented usage examples in our codebase

**Examples for Fosved 2026:**
- TypeScript (universal type safety)
- Next.js 16 (default React framework)
- Prisma 6 (default ORM)
- Tailwind 4 + Radix UI (default styling)
- Postgres (default DB)
- Render (default deployment)
- Anthropic SDK (primary LLM)
- Playwright (default E2E testing)

### Trial
**Definition:** worth investing in on new projects but not yet default.

**Criteria for entry:**
- Solves real problem better than current adopt option
- Used in 1+ projects with positive results
- Maintainer active
- Acceptable risk for non-critical projects
- Team has explored documentation

**Examples for Fosved 2026:**
- Effect-TS (functional error handling)
- Hono (lighter alternative to Express for edge)
- Bun (alternative runtime, faster cold start)
- shadcn/ui patterns (specific components)
- LangSmith / Promptfoo (AI evals — trialing)
- Ollama (local model deployment — trialing)

### Assess
**Definition:** watching, may evaluate seriously later, not using.

**Criteria for entry:**
- Interesting capability or approach
- Backed by credible team/company
- Not yet mature enough for trial
- Not yet a fit for current projects

**Examples for Fosved 2026:**
- Astro (alternative web framework)
- Deno 2 (Node alternative)
- TauriJS for AI office desktop apps
- DuckDB (analytics)
- LiteLLM (LLM abstraction layer)
- AutoGen / CrewAI (multi-agent frameworks)
- Inngest (durable workflows)

### Hold
**Definition:** explicitly NOT using. Has issues or made bad fit.

**Criteria for entry:**
- Tried and abandoned with documented reasons
- Known critical issue (security, performance, maintenance abandoned)
- Replaced by better alternative
- Anti-pattern that compounds technical debt

**Examples for Fosved 2026:**
- jQuery (modern alternatives exist)
- MongoDB (we use Postgres, no need for two DBs)
- create-react-app (deprecated; use Next.js or Vite)
- Express body-parser (built into Express now)
- moment.js (use date-fns or dayjs)

**Hold is not "bad" — it's "not for us right now"**. Hold reasons should be specific.

## Radar entry data structure

In `TechRadarEntry` table:

```javascript
{
  techName: 'Next.js',
  category: 'framework',
  state: 'adopt',
  rationale: 'Default React framework. Used in fosved-miniapp and all new web projects. Server components + app router stable since 14.',
  usageCount: 3,  // # of repos using
  lastUsedAt: '2026-05-13',
  externalReferences: [
    { url: 'https://nextjs.org/docs', title: 'Official docs', why_relevant: 'API reference' },
    { url: 'https://github.com/vercel/next.js', title: 'GitHub repo', why_relevant: 'Active maintenance check' }
  ],
  previousStates: [
    { state: 'trial', since: '2025-03', until: '2025-09', reason_for_change: 'Promoted to adopt after fosved-miniapp success' }
  ],
  lastReviewedAt: '2026-04-01',
  nextReviewAt: '2026-07-01'
}
```

## State transitions

States can move in either direction:

- `assess → trial`: project requires it, willing to try
- `trial → adopt`: 2+ successful uses, comfortable
- `adopt → trial`: regression in maintenance or new concerns ("downgrade")
- `trial → hold`: tried and abandoned
- `adopt → hold`: critical issue emerged
- `hold → assess`: situation changed, worth re-evaluating
- `assess → hold`: decided no, document why

Every transition recorded in `previousStates` with reason.

## Categories

Organize radar by category for navigability:

- **language** — TypeScript, Python, Bash, Go, Rust
- **framework** — Next.js, Express, Fastify, FastAPI
- **library** — Prisma, Radix UI, React Query, lodash
- **tool** — Playwright, Vitest, Promptfoo, k6
- **service** — Anthropic API, Render, Vercel, Cloudflare, Supabase
- **pattern** — DDD, Clean Architecture, Functional Core, Event Sourcing
- **model** — Claude Opus, GPT-4, Grok, Llama 3, Mistral

## Quarterly review process

Every Jan/Apr/Jul/Oct 1st, scheduler runs `quarterlyTechRadarReview()`:

**1. Usage audit.** For each adopt/trial entry: how many projects used it last quarter? Decreasing usage → consider demotion.

**2. External scan.**
- ThoughtWorks Tech Radar (latest issue)
- State of JS / State of CSS survey
- GitHub trending in our areas
- Conference talks (NextConf, ReactConf, NeurIPS, KubeCon)
- Anthropic / OpenAI / Mistral releases

**3. Project debriefs.** From last quarter's completed DevTasks, gather feedback:
- What worked well? (adopt candidates)
- What had friction? (trial → hold candidates or just lessons)
- What got tried? (assess → trial candidates)

**4. Movement proposals.** For each candidate movement: write rationale, owner approves.

**5. Documentation update.** TechRadarEntry records updated, `previousStates` appended.

**6. Communicate.** Quarterly radar published to Telegram archive with diff vs previous quarter.

## How to use the radar in daily work

Before starting a DevTask:

```javascript
// Check tech stack proposal against radar
const proposedStack = ['Next.js', 'Prisma', 'Astro'];

for (const tech of proposedStack) {
  const entry = await prisma.techRadarEntry.findUnique({ where: { techName: tech } });
  if (!entry) {
    // Unknown — needs adding to radar first
    console.warn(`${tech} not in radar. Add via /dev-radar-add or choose known option.`);
  } else if (entry.state === 'hold') {
    // Blocked
    throw new Error(`${tech} is on hold (${entry.rationale}). Use alternative or revisit radar.`);
  } else if (entry.state === 'assess') {
    // Promote to trial first
    console.warn(`${tech} is assess-only. Promote to trial before using? /dev-radar-add ${tech} trial`);
  }
  // adopt | trial → proceed
}
```

This check runs in `lib/dev.js` automatically when DevTask records `techStack` field.

## Anti-patterns

- **Static radar.** Never updates. Tech world moves; static radar becomes irrelevant.
- **Constant updates.** Updates every week. Defeats the purpose of stable categorization.
- **Hold with vague reason.** "Doesn't work" — explain HOW it doesn't work for specifics.
- **Adopt by reputation.** "Big company uses it" — that's signal not decision. Try it first.
- **Trial forever.** Trial > 1 year without promotion or demotion = decision avoidance.
- **No usage tracking.** Radar without usage data lacks anchor.
- **Bypassing hold.** "I'll use moment.js just this once" — defeats the system. Either change the radar or use alternative.
- **External-only inputs.** Adopting because ThoughtWorks says so, without our own trial.
- **Internal-only inputs.** Ignoring external signals because "we know best".

## Commands

- `/dev-radar` — show full radar by category
- `/dev-radar <category>` — show one category
- `/dev-radar-add <tech> <state> <rationale>` — add or update entry
- `/dev-radar-move <tech> <new_state> <reason>` — explicit state transition

## Compound learning value

After 4 quarterly reviews (1 year), radar has:
- ~40-80 entries
- Documented rationale for every choice
- History of what was tried and result
- Calibration against external trends

This is the **engineering institutional memory**. Without it, every year starts fresh. With it, every year compounds.

## Integration

- `architecture-decision-records` references radar entries
- `dependency-management` enforces radar state on dependency installation
- `lib/dev.js` `weeklyTechRadarScan()` accumulates external signals
- `lib/dev.js` `quarterlyTechRadarReview()` runs review process
- DevTask `techStack` field cross-references radar

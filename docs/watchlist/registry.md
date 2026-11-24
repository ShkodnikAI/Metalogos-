# I&W watchlist — the seven-scenario registry (№993, gh#2036; WL-07 added by №997 — the owner's 23.11 verdict «Согласен с твое рекомендацией», the 23.11 pre-study r74/r75 cuts, outside the ADR-0200 line)

The observation-only lane born from the owner's 22.11 decision («поставим на
отслеживание, если нужно потом внедрим») about the six external repositories
studied 21–22.11. **Nothing here is adopted**: the watchlist watches, the
adoption happens by the Red-triggers of this registry only, as a separate
naryad. Not part of the ADR-0200 line — infrastructure observation.

The mechanics live in `.github/workflows/watchlist-weekly.yml` (cron Mon
09:00 MSK = 06:00 UTC, no DST in Moscow) + `scripts/watchlist_check.py`.
The weekly trace: the digest comment in the pinned issue «I&W watchlist
digest» (the №979 visibility rule — a scheduled workflow never leaves the
week unseen) + the report files that ride the `watchlist-digest` branch
(main is protected: enforce-admins, no bypass — a direct report commit to
main is impossible by design; AND GitHub Actions cannot create or approve
pull requests in this repo by the owner's setting — verified 22.11, the
create attempt fails with `createPullRequest` — so the digest PR opens
manually, by the executor, when the artifacts are ready to merge). The
issue pin goes through the GraphQL `pinIssue` mutation (the REST pins
endpoint is absent for this account — 404 even for an admin PAT).

## The level calibration (the same for every scenario)

| Level | Trigger |
|---|---|
| Yellow | ONE trigger fires: the stars +5..15%/week; the crates.io downloads ≥ +500/week; a NEW release vs the baseline; a push pause ≥ 14 days at a repo that pushed within the baseline week |
| Orange | TWO triggers at once, or the stars > 15%/week |
| Red | THREE+ triggers at once; or the stars > 25%/week combined with a new release; or a push pause ≥ 45 days at an active repo |

The anomaly rule: an Orange/Red week NEVER rewrites the baseline — the
baseline stays and the digest carries the flag until the owner's manual
decision. A quiet or Yellow week refreshes the baseline in the same
workflow commit (the smooth growth is the new normal).

## The scenarios

### WL-01 — erkamyaman/rust-anything (the Rust experiment surface)

- **Signals:** the GitHub stars/forks/issues/pushed_at, the latest release,
  the crates.io downloads (the API requires a User-Agent header).
- **The adoption interest:** the Rust patterns we may borrow for the
  interpreter backends; the crates.io curve is the real-usage proxy.
- **Horizons:** T-30 — the weekly growth; T-90 — the release cadence;
  T-180 — the downloads trend; T-365 — the stars trajectory.
- **Baseline 22.11:** 7708★, v3.2.0, 2770 downloads.

### WL-02 — TauricResearch/TradingAgents (the trading-agent framework)

- **Signals:** the stars, the releases, the push pulse.
- **The adoption interest:** the multi-agent orchestration patterns (the
  distill/attest analogies); the release cadence signals the API churn.
- **Horizons:** T-30 — the weekly growth; T-90 — the releases;
  T-180 — the fork ratio; T-365 — the stars trajectory.
- **Baseline 22.11:** 123441★, v0.7.1.

### WL-03 — stacklok/toolhive (the MCP server lifecycle toolkit)

- **Signals:** the stars, the releases, the push pulse.
- **The adoption interest:** the MCP tool-server lifecycle — the direct
  analog of our capability gates; a new release is the strongest signal.
- **Horizons:** T-30 — the weekly growth; T-90 — the releases;
  T-180 — the issues flow; T-365 — the stars trajectory.
- **Baseline 22.11:** 2349★, v0.67.1.

### WL-04 — NVIDIA/auto-ontology (the ontology automation)

- **Signals:** the stars, the push pulse (no releases recorded at the
  baseline capture — a first release is itself an event).
- **The adoption interest:** the ontology-driven schemas vs our fact
  classes; the first release marks the usability threshold.
- **Horizons:** T-30 — the weekly growth; T-90 — the first release watch;
  T-180 — the fork ratio; T-365 — the stars trajectory.
- **Baseline 22.11:** 14254★, no releases.

### WL-05 — deepset-ai/haystack (the RAG/pipeline framework)

- **Signals:** the stars, the releases, the push pulse.
- **The adoption interest:** the pipeline composition patterns vs the mlog
  flow graphs; the v3→v4 line is the compatibility-churn indicator.
- **Horizons:** T-30 — the weekly growth; T-90 — the releases;
  T-180 — the issues flow; T-365 — the stars trajectory.
- **Baseline 22.11:** 27073★, v3.5.0.

### WL-06 — MervinPraison/PraisonAI (the multi-agent low-code framework)

- **Signals:** the stars, the releases, the push pulse.
- **The adoption interest:** the low-code agent assembly vs the distill
  hub; the release cadence is the feature-churn proxy.
- **Horizons:** T-30 — the weekly growth; T-90 — the releases;
  T-180 — the fork ratio; T-365 — the stars trajectory.
- **Baseline 22.11:** 9379★, v4.8.0.

### WL-07 — obra/superpowers (the composable-skills methodology)

- **Signals:** the stars, the releases, the push pulse (crates.io N/A —
  not a crate).
- **The adoption interest:** the methodology over 15 composable skills
  (TDD, writing-plans, subagent-driven, two-stage review) — the donor
  patterns for the skill layer; the release cadence (2–6 weeks) is the
  tempo signal.
- **Horizons:** T-30 — the weekly growth; T-90 — the release cadence;
  T-180 — the fork ratio; T-365 — the stars trajectory.
- **Baseline 23.11:** 324691★, v7.0.2 (the pre-study 23.11: 28 805
  forks, 361 open issues, MIT, Shell, created 09.10.2025; the adoption
  stays Red-trigger only — the donor patterns (a)–(в) are named
  triggers, each a separate naryad).

## The monthly review (the first Monday of the month)

The digest of that week carries the relevance summary: which scenarios
still earn their lines, which are the retire candidates (a scenario that
stayed quiet AND irrelevant to any open naryad for a quarter goes to the
owner with the retire proposal). The retirement is the owner's call, never
the workflow's.

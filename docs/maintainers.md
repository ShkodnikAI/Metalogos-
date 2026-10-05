# Maintainers — the supply-chain perimeter and the second-maintainer authority (№471)

Decision 6-A (gate gh#680): the project's bus factor is 1 at ~9 PRs a
day; the second maintainer is recruited with a NARROW, real veto — not
"the core as a whole", and not a for-show role. This document is the
written authority: what the perimeter is, what the veto covers, what is
expected, and how conflicts escalate.

## The perimeter (the veto zone)

The second maintainer holds the right to **block a merge** into the
supply-chain perimeter, defined in `.github/CODEOWNERS` as:

- `.github/**` — the CI surface: workflows, templates, dependabot;
- `.gitleaks.toml` — the secret-scanner config;
- `.cargo/audit.toml` — the cargo-audit overrides (created on the first
  RUSTSEC exception);
- `Cargo.lock` — the dependency lock (the supply-chain fact itself);
- `scripts/ci/**` — the threshold gates and their checked-in baselines
  (the dedup family: the №462 dup-names gate, the №484 ops-pair gate
  (gh#732), the №502 vm-mirror gate (gh#785); the №463 generative
  stop-list, the №467 type-share floor, the №468 debt gate, the №469
  risk-review script; the №472 core→media ban (gh#693), the №482
  unfreeze-gate summary (gh#730), the №497 badge sync, the №505
  naryad-number check, the №525 gate-facts sync, the №529
  registry-bounds gate, the №535 blocking-checks sync — the blocking
  table `blocking_checks.tsv` is the machine-read record of what
  blocks).

The perimeter is FROZEN at this list: it does not grow with the codebase.
Everything outside it (the core, the domains, the docs) stays the
owner's lane — the second maintainer has no veto there.

## The veto, mechanically

- A PR touching the perimeter requires the code owner's review before
  merge (`Require review from Code Owners` on `main`'s protection).
  Today the code owner of the perimeter lines is `@ShkodnikAI` (the
  single-maintainer fact); the second maintainer's handle is added to
  the same lines on the day they join — the veto becomes real without
  any perimeter change.
- The veto is exercised as a request-for-changes or a blocking review
  comment with the checklist item or the supply-chain rule cited. A
  veto is never silent: the reason is written in the PR.
- The maintainer does NOT open their own perimeter changes without a
  second pair of eyes either: the perimeter has no self-merge lane.

## Onboarding (the first week, in order)

1. Read this document, `.github/CODEOWNERS`, and the threshold gates
   listed above (each gate's baseline header explains its
   only-down / only-up rule; `blocking_checks.tsv` records what
   blocks).
2. Run the gates locally: `python3 scripts/ci/count_duplicated_names.py
   --gate scripts/ci/tw_vm_dup_names_baseline.txt`,
   `python3 scripts/ci/ops_pair_counter.py --gate
   scripts/ci/ops_pair_baseline.txt`,
   `python3 scripts/ci/mirror_counter.py --gate
   scripts/ci/src_mirror_baseline.txt`,
   `scripts/ci/type_signature_share.py --gate ...`,
   `scripts/ci/debt_counters.py --gate ...`,
   `scripts/ci/generative_stop_list_gate.py` and
   `scripts/ci/risk_review.py --report`.
3. The protection toggle (the owner does it once, together): enable
   `Require review from Code Owners` on `main` — the veto becomes
   machine-enforced from that moment.
4. Take one of the `good-first-issue` onboarding tasks (see below) as
   the first PR — the step-by-step path (fork → branch → CI → PR →
   report) and the pool's health metric (the time to the first merge)
   are written down in `CONTRIBUTING.md` ("Your First PR — the
   Good-First-Issue Lane", naryad №541).

## Expectations

- A few hours a week (2–4) — the perimeter is narrow by design: the
  gates are automatic, the human duty is the review of the perimeter
  PRs and the periodic upkeep tasks below.
- No on-call, no core review duty, no roadmap authority — those stay
  with the owner.

## The upkeep tasks (the good-first-issue pool)

Nine self-contained tasks as of naryad №541 (each carries a full guide:
the reproducible check, the expected scope, the acceptance criteria and
the honest boundary — follow its issue text; the first-PR path lives in
`CONTRIBUTING.md`):

- The №468 debt-gate upkeep: review the `#[ignore]`/`dead_code`
  inventory drift at each release, keep the baseline honest (movement
  only down) — gh#716.
- The №463 stop-list upkeep: new generative-model candidates in the
  tree → the manifest row + the LOC baseline, before the merge — gh#717.
- The №462/№467 floors: regenerate the dup-names and type-share
  baselines at releases (the counters print the follow-up note), one PR
  per movement, the history line in the baseline header each time —
  gh#718.
- The cargo-audit override ledger: the release-time review of
  `.cargo/audit.toml` exceptions against the live advisory DB — gh#777.
- The maintainers × CODEOWNERS sync review: keep the written authority
  matching reality — gh#778.
- The book examples freshness pass: every fenced mlog snippet in
  `docs/book/src/` runs against the live binary — gh#872.
- The ADR index freshness review: `gen_adr_index.py` output vs
  `docs/adr/` at each release — gh#873.
- The limitations.md anchor walk: every claim keeps a live
  `file:line` anchor — gh#874.
- The fresh-clone quickstart pass: README and CONTRIBUTING commands
  work verbatim on a clean machine — gh#875.

## The honest-boundary protocol (№588, audit d63cc1d §8)

Any "honest boundary" marker a PR adds to the diff (a commit body or a code
comment: `honest boundary`, `pre-existing divergence`, `known divergence`)
obliges, in the SAME PR:

1. **A `docs/limitations.md` row** — the divergence leaves the git history
   and becomes a fact the release gate reads (the limitations page is part
   of the gate's fact surface).
2. **If the divergence is security-relevant** — the `release-block` label on
   the linking issue + an explicit flag line in the PR description (the
   ADR-0179 §4 discipline; `fact_open_high_server` reads the label).

The mechanical companion: the `Honest boundary protocol (advisory)` CI job
(`scripts/ci/honest_boundary_check.py`) scans the PR's ADDED lines for the
markers and warns loudly when `docs/limitations.md` is not among the changed
files. Advisory by fact (the naryad's warn-only precedent; the blocking
escalation is decided by the false-positive experience) — loudness is
mandatory, the block is by fact, never silent. The forward-only boundary:
the check reads added diff lines, never the landed history — the existing
in-tree markers already carry their ADR/limitations records and no
retrospective scan is performed (№588's boundary 4).

## When CI is down (the M-1 rule, №551)

**No merges while GitHub Actions is degraded. A merge without a green
required-check run is a process violation, not a judgement call.**

The 2026-09-30 event-delivery outage (14:30–19:37 UTC) let six PRs
merge with zero checks and shipped two defects to main unseen (the
hotfix gh#865: "the two shipped-unseen CI defects the dead event
delivery hid"). The rule above is written down so the next outage meets
a policy, not an improvisation:

- GitHub Actions degraded (an incident on
  [githubstatus.com](https://www.githubstatus.com/) or runs simply not
  appearing): merges STOP. The queue waits; no PR is "small enough to
  skip".
- A PR whose required checks have NOT RUN (no check-runs at all — the
  dead event delivery looks exactly like this) is UNVERIFIED, not
  green. "No red X" is not "green".
- If a merge is business-critical during an outage: document the
  decision in the PR, merge, and open a follow-up issue the moment
  Actions recovers. The weekly audit flags it anyway (below) — the
  honest paper trail is the difference between a recorded decision and
  a violation.

### The weekly detective control (the machine half of the rule)

`scripts/ci/merge_ci_audit.py` — scheduled weekly in the
`merge-ci-audit` workflow (Mondays 05:00 UTC, plus `workflow_dispatch`)
— re-checks every main merge of the last 7 days against a GREEN run of
the required set on its PR's head SHA. A divergence is a red workflow
run AND an automatically filed issue with the list. Honest boundaries
of the check: a `skipped` required check is NOT green; a check absent
from the head SHA's workflow set (a job born after the merge) is
flagged too — it cannot be proven retroactively, so every flag needs a
human verdict, and the issue is closed WITH that verdict (e.g., the
documented 30.09-outage merges predating the №525/№535 jobs). The
script only reports — it never merges, reverts, or edits history; the
required set is pinned by `tests/naryad_551_merge_ci_audit.rs` and
moves together with the checklist below, never alone.

### The good-first-issue pool policy (№561, the audit 02.10 §7.2 W-1)

The pool exists for the INCOMING external contributors — the №491/№471
onboarding lane (the "second maintainer" procedure prepared, the grant
wording honest). The audit's finding: the pool drains from the INSIDE —
of the №541 positions, five were closed by owner/agent commits within a
day (gh#778, gh#872–gh#875, closed 02.10 15:08 UTC), and a newcomer
arriving on 03.10 saw an EMPTY pool. The policy:

- **Internal closing is allowed, but the refill happens in the same
  wave**: closing a pool position by an owner/agent commit obligates
  the same wave to re-open enough good-first-issues to bring the pool
  back to **8–10 open positions** (the №541 size);
- **the pool-size counter is part of the wave report** (the dispatch
  summary states the open-pool count next to the wave progress);
- the `reserved-for-newcomer` label marks the pool's positions; per
  AGENTS.md §3, agents do NOT take labeled tasks — a recurring upkeep
  task an agent did internally is re-opened as a good-first-issue for
  the next external contributor (internal work trains nobody);
- the №491 filter holds for every new position: self-contained volume,
  test-helpers/doc passes over core changes, no context dependency
  beyond the repo; each issue carries the expected volume and the
  entry point (the #872–#875 shape);
- the health metric stays **the time to the first merge** (§ above) —
  an empty pool is an infinite time-to-first-merge, which is why the
  refill rule exists.

### The branch-protection checklist (the owner's admin toggle)

Applying the settings is an ADMIN action on the repository — this
document records the checklist; the executor cannot apply it. Settings
→ Branches → Branch protection rule for `main`:

- [ ] **Require a pull request before merging** (no direct pushes);
- [ ] **Require status checks to pass before merging** — the required
      set (the job display names as the check-runs API reports them,
      fact-checked against `.github/workflows/ci.yml` @ `25b375e` and
      the live-protection API read of 2026-10-05 (the №586 first
      audit — 22 required contexts on the live rule);
      `msrv (blocking)` joined by №555, `release-gap (blocking)` by
      №562; the further twelve joined from the live read):
      `test-lib (blocking)`, `test-integration (blocking)`,
      `crosscheck (blocking)`, `clippy (blocking)`, `fmt (blocking)`,
      `cargo-audit (blocking)`, `cargo-deny (blocking)`,
      `gitleaks (blocking)`, `gate-facts-sync (blocking)`,
      `blocking-checks-sync (blocking)`,
      `registry-arity-check (blocking)`, `msrv (blocking)`,
      `release-gap (blocking)`,
      `ADR numbering (blocking)`, `branch-freshness (blocking)`,
      `candle-tests (blocking)`, `doc-tests (blocking)`,
      `ledger-golden (blocking)`, `minimal-build (blocking)`,
      `module-size-guard (blocking)`,
      `test-llm-cache-contract (blocking)`,
      `video-tests (blocking)`, `vision-tests (blocking)`,
      `voice-tests (blocking)`, `vscode-extension (blocking)`;
- [ ] **Require branches to be up to date before merging** (no merge
      over a stale base);
- [ ] **Do not allow bypassing the above settings** — including
      administrators: an outage bypass defeats the whole rule;
- [ ] (the №471 lane, with the second maintainer) **Require review
      from Code Owners** — the veto becomes active with the same
      toggle.

The checklist above is machine-audited (naryad №586, audit d63cc1d
X-4): the `branch-protection-audit (weekly)` job reads the live
protection via the read-only `BRANCH_PROTECTION_TOKEN` repo secret
(Administration: read-only, 90-day rotation) and compares it to THIS
checklist — the state is fixated, not assumed. A documented required
check missing from the live rule, or one of the toggles above being
off, is a RED run; live checks not yet in the list are RECORDED
(doc-stale warning, never failed — over-protection is not a hole) and
re-enter the list by the fact-check procedure above. The state-on-date
report line lands in the run's summary and artifact; the job is
read-only and never modifies the protection. Applying the divergent
toggles remains the OWNER's admin action — the first audit run
(2026-10-05) is fixated in the №586 record (gh#1000).

## Escalation

A disagreement between the maintainer and a PR author that the two
cannot close escalates to the OWNER — the owner's call is final and is
recorded in the PR. A disagreement about the PERIMETER ITSELF (growing
or shrinking the veto zone) is an owner decision by definition; the
maintainer may propose, not decide.

## The NLnet application wording (the owner's draft)

> Narjad №491 (the audit 26.09 §3.10 correction): grant texts must say
> "procedure prepared", not "second maintainer exists" — the person
> cannot be created by a document, and a grant application claiming an
> existing second maintainer would be false until the day one joins.
> The honest form, current as of the Restack draft:
>
> Metalogos has the second-maintainer PROCEDURE fully prepared: a real,
> machine-enforceable veto over a deliberately narrow supply-chain
> perimeter (CI workflows, the dependency lock, secret scanning, the
> threshold gates) is written down in `docs/maintainers.md` and
> mapped to `.github/CODEOWNERS`; the onboarding plan, the first-PR
> pool (nine documented `good-first-issue` tasks with reproducible
> checks and acceptance criteria) and the escalation path
> are documented. The veto becomes active with one owner toggle
> (`Require review from Code Owners`) the day a second maintainer
> joins. The search runs through the NLnet/Restack community and the
> external-contributor pipeline; the supply-chain role is the first,
> deliberately narrow step of the bus-factor plan (bus factor today
> is 1, stated openly).

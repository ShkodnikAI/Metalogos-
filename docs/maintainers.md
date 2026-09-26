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
  (the №462 dup-names gate, the №463 generative stop-list, the №467
  type-share floor, the №468 debt gate, the №469 risk-review script).

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

1. Read this document, `.github/CODEOWNERS`, and the five threshold
   gates listed above (each gate's baseline header explains its
   only-down / only-up rule).
2. Run the gates locally: `python3 scripts/ci/count_duplicated_names.py
   --gate scripts/ci/tw_vm_dup_names_baseline.txt`,
   `scripts/ci/type_signature_share.py --gate ...`,
   `scripts/ci/debt_counters.py --gate ...`,
   `scripts/ci/generative_stop_list_gate.py` and
   `scripts/ci/risk_review.py --report`.
3. The protection toggle (the owner does it once, together): enable
   `Require review from Code Owners` on `main` — the veto becomes
   machine-enforced from that moment.
4. Take one of the `good-first-issue` onboarding tasks (see below) as
   the first PR.

## Expectations

- A few hours a week (2–4) — the perimeter is narrow by design: the
  gates are automatic, the human duty is the review of the perimeter
  PRs and the periodic upkeep tasks below.
- No on-call, no core review duty, no roadmap authority — those stay
  with the owner.

## The upkeep tasks (the good-first-issue pool)

- The №468 debt-gate upkeep: review the `#[ignore]`/`dead_code`
  inventory drift at each release, keep the baseline honest (movement
  only down).
- The №463 stop-list upkeep: new generative-model candidates in the
  tree → the manifest row + the LOC baseline, before the merge.
- The №462/№467 floors: regenerate the dup-names and type-share
  baselines at releases (the counters print the follow-up note), one PR
  per movement, the history line in the baseline header each time.

## Escalation

A disagreement between the maintainer and a PR author that the two
cannot close escalates to the OWNER — the owner's call is final and is
recorded in the PR. A disagreement about the PERIMETER ITSELF (growing
or shrinking the veto zone) is an owner decision by definition; the
maintainer may propose, not decide.

## The NLnet application wording (the owner's draft)

> Metalogos ships with a second maintainer holding a real, machine-
> enforced veto over the supply-chain perimeter (CI, dependency lock,
> secret scanning, the threshold gates) — written authority from day
> one, documented in `docs/maintainers.md` and `.github/CODEOWNERS`.
> The search for a second CORE reviewer continues in parallel through
> the NLnet/Restack community; the supply-chain role is the first,
> deliberately narrow step of the bus-factor plan.

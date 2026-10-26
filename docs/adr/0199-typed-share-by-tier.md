# ADR-0199: The Typed-Signature Metric Reads by Maturity Tier — the STABLE-Tier Share Is the PRIMARY Definition; the Full-Registry Figure Stays Beside (the C3-a T3 decision)

- Status: **Accepted** (the C3-a T3 owner decision of 2026-10-23,
  variant (a) — "уровни в редакции по плану"; this ADR is the
  definitional record the decision required)
- Date: 2026-10-23 (the decision); 2026-10-26 (the ADR record, №809)
- Deciders: the OWNER (the C3-a decision); the machine formats the record
- Naryad: №809 (issue #1565, Волна 66)
- Source: the audit-23.10 C3-a T3 (issue #1489); the owner's C3 decision
  (#1472) chose variant (a) — the levels in the plan's edition
- Related: [ADR-0179](0179-typed-signature-goal.md) (the 0.28 general
  gate — unchanged), №796 (the tier literal in every spec! row — the
  machine fact this definition reads), №799 (the parked gate — the
  parked rows refuse at check, which is WHY they leave the primary
  denominator)

## §1. Context — what the single-denominator definition hid

The typed-signature share (№467, ADR-0179) counted over the WHOLE
registry: 372 typed rows of 526 (70.72% at the definition change). The
№796 tier literal made the registry's maturity structure machine-
readable: 483 stable, 22 experimental, 21 parked. The single
denominator mixed three different contracts:

1. **stable** — what every user gets by default; the primary surface
   the reference documents as always-available;
2. **experimental** — present, maturing, may change;
3. **parked** — behind features; the №799 gate makes a parked builtin
   REFUSE at check time unless its feature is on — a parked row is not
   part of the primary contract by construction.

A parked/experimental row that stays untyped dilutes the primary
figure even though no default-surface user can reach it. The audit's
point: the headline metric should measure the CONTRACT, not the
catalog.

## §2. The decision (the definitional change)

The PRIMARY typed-signature definition changes to the **stable-tier
share**: typed∩stable / stable. At the definition date:
363/483 = 75.15% (the full-registry figure beside it: 372/526 =
70.72%).

The change is a re-aim, not an erasure:

- the full-registry share keeps its floor
  (`type_signature_baseline.txt`, ADR-0179's 0.28 gate) and stays
  REPORTED beside the primary figure — in the machine report, in
  docs/metrics.md, and in the wave records;
- the stable share gets its own only-up floor
  (`type_signature_stable_baseline.txt`, 7515 bp at the definition
  date) and its own CI gate step (the same blocking job — the №535
  cell does not grow);
- both floors move ONLY up (the №757 procedure verbatim: a raise
  happens in the same PR that types more rows).

The REFERENCE index reads by the same levels (the same №809): stable
is the primary reference, experimental is the opt-in tier, parked is
the appendix — the doc and the metric now measure the same three-way
contract.

## §3. Consequences

- The primary metric RISES mechanically when a parked/experimental row
  graduates to stable while typed (the denominator swap is a gain) —
  that is the intended incentive: graduation is the movement the
  0.30-cycle typed campaign wants.
- A stable row added untyped hurts the primary figure MORE than
  before (the denominator is smaller) — also intended: the primary
  contract is the strict one.
- The parked/experimental typing work does NOT move the primary
  figure until the rows graduate — the honest trade the decision
  accepted; the full-registry floor still guards that lane from
  rotting.
- The wave records and the release notes quote BOTH figures (the
  generator and the metrics table already do).

## §4. The parked-features CI lane (the same decision's T3) — REVISED 2026-10-26

The four feature-gated pillar jobs gain the dedicated
`parked-features.yml` lane: nightly (schedule) + pull_request with a
path filter on the pillar catalogs + workflow_dispatch; the
`nightly-failure-record` job files the fix issue on a red nightly.
**The release rule (soft)**: a red nightly parked-features run does
NOT block a tag, but it is RECORDED — the CHANGELOG/limitations line
("the parked-features nightly is red on X.Y.Z — <pillar>") plus the
fix issue.

**The revision (the №809 CI episode, honest):** the ORIGINAL plan
moved the jobs OUT of the every-push CI — and the first head went
RED on the mergeability itself: the branch protection requires the
four pillar contexts (`candle-tests (blocking)` and the siblings —
read via the admin API 2026-10-26), so removing the jobs made every
PR unmergeable FOREVER (a required context with no producing
workflow never turns green). The jobs STAY in ci.yml until the
owner's protection toggle (форма №755) drops the four contexts; the
parked lane runs ALONGSIDE as the nightly health signal. The
protection-toggle is the precondition for the move's final step.

**The audit found a second drift while there:** the actual protection
carries 27 required contexts; the №551 `REQUIRED_CHECKS` list (the
audit script + the driver test + maintainers.md) carried 13 — 14
contexts (branch-freshness, minimal-build, module-size-guard,
test-llm-cache-contract, vscode-extension, ledger-golden, doc-tests,
html-tests, xml-tests, ADR numbering + the four pillars) were outside
the detective's audit surface (the protection enforced them; the
weekly detective did not re-verify them). The sync is its own naryad
(the №551 rule: the script, the driver test and the docs move
together — and the protection audit's direction must cover the
reverse side too: no required context without a checklist row).

# The v0.40.1 release-commit-gate verification — the recorded act
# (№976 follow-up 2, issue #1985; the owner decision 21.11 «принимаю»)

### 6.82. The v0.40.1 gate — the patch-slice fact and the closure (№976 follow-up 2, issue #1985)

## The fact

The patch slice 0.40.1 (`release/0.40.1`, head `6232bf93`) is a cut
from the `v0.40.0` tag tree plus the `lettre` bump, the version stamp
and one cherry-picked `ci.yml` fix. Its blocking CI set is green
(54/55 — the single red is the `coverage (non-blocking)` flake the
`stage4_benchmark` 500, tracked as №991). The release-commit gate on
that commit REFUSED with the pre-follow-up gate: `required 26,
present 24, missing 2` — `Red-team gate (blocking on the security
class — №961)` and `rust-doc-tests (blocking)` became required AFTER
the `v0.40.0` tag instant and are not defined in the release tree's
workflow files — the patch tree cannot run them, physically.

## The closure (№976 follow-up 2, one PR on main)

The gate learned the patch slice: the context waiver fires ONLY under
the conjunction — the context is not defined in the release commit's
workflow files AND its `REQUIRED_SINCE` is later than the line moment
(the commit date of `v0.40.0`, `2026-11-16T13:22:21+03:00`); a context
without a `REQUIRED_SINCE` entry is never waived; a minor/rc release
has no waiver path; an unresolvable `vX.Y.0` means no waiver and the
log says so; every waiver prints both conditions and the moment; the
summary carries both numbers. The ref resolution is fail-closed (the
silent `github.sha` fallback removed; the checkout carries the full
history and the tags). The self-test grew to 11 traps plus the
two-sided mutation of the conjunction.

The pre-tag verification (measured, the executor's run on the branch
ref): `release_commit_gate.py --ref release/0.40.1` →
`required 26, waived 2, present 24, missing 0, not-green 0` → **OK**
with both waivers printed by name.

### 6.83. The v0.40.1 verdict — the act and the result place (the owner, after tagging)

## The act, as it must be performed (the owner, after tagging)

1. The owner pushes the tag `v0.40.1` (the publish moment — the tag
   push itself runs NO gate: the workflow files do not exist on the
   patch branch — a patch tag does not self-verify,
   docs/release-checklist.md carries the same sentence).
2. The owner runs `release-commit-gate` via `workflow_dispatch` from
   main with `ref=v0.40.1` — the recorded verdict against the TAG SHA
   (not the branch head; they are the same commit `6232bf93`).
3. The owner publishes the release; `release.yml` triggers on
   `release: published` and builds the binary, the SBOM and the
   attestations.

## The result place

| What | Where |
|---|---|
| The dispatch-run link (the gate against `v0.40.1`) | _the owner fills this in after the dispatch run_ |
| The verdict text (the summary line + the waivers) | _the owner fills this in after the dispatch run_ |
| The release URL | _the owner fills this in after the publish_ |

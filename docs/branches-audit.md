# Branch audit (branch hygiene) — SSOT

**Naryad:** №1002 (issue #2063, P3, [ci][process], the owner's directive of 24.11: «why does the Metalogos repo have a pile of new branches again instead of one main? Who creates them and why — this must be fixed»).
**Audit date:** 24.11 (R84–R85, GET-only). **Sweep addendum:** 25.11.
No product code touched. **The file is English on purpose:** the repo CI carries a Cyrillic lint (`no_md_file_above_cyrillic_threshold_outside_allowlist`, tests/docs_language_lint.rs) and this path is not on the allowlist; the allowlist growth requires an owner decision (never done silently). **The issued Russian verbatim of the table and the rule lives in the issue #2063 body** — that body is the record of authority; this file mirrors it in English and is the on-main SSOT. The previous `docs/branches-audit.md` had disappeared from main — the table below restores it (see History).

## Who creates branches and why

- **The executor** — one branch per naryad (`naryad-NNNN-*`): main is protected (enforce-admins, no bypass), everything goes through a PR + green CI — merge rule (a), the owner's machine.
- **dependabot** — `dependabot/cargo/*`.
- **The weekly watchlist-workflow** (№993) — the rolling `watchlist-digest` (Actions cannot create PRs in this repo — the digest artifacts are parked on the branch).
- **The release-train** — `release/*` for the duration of a train.

`delete_branch_on_merge` = **true** (verified 24.11): merged branches do not accumulate. **The hole:** closing a PR **without merging** does not delete the branch — the orphan heads hung for exactly that reason. The PR-"probe" #2056 (23.11) — an incident of the same class, extinguished the same day.

## The audit table, 24.11 (7 rows; the verdicts as of the R85–R86 deletions)

| Branch | Origin | Verdict 24.11 |
|---|---|---|
| `main` | — | protected (enforce-admins), the only durable one |
| `naryad-997-wl07-watchlist` | the executor, PR #2062 (opened 24.11, №997 delivered as part of the №1000 cleanup) | **held** — live work (merged 24.11, the branch auto-deleted — see the 25.11 sweep) |
| `watchlist-digest` | the weekly watchlist-workflow №993 (cron Monday 09:00 MSK) | **held** — sanctioned rolling infrastructure |
| `naryad-979-workflow-silent` | the executor: PR #1993 and #2020(-v2) closed WITHOUT a merge | deleted 24.11 (an orphan; the commits remain reachable via the closed PRs) |
| `naryad-976-release-gate` | the executor: PR #1992 closed without a merge | deleted 24.11 (an orphan) |
| `dependabot/cargo/uuid-1.29.3` | dependabot: PR #2010 closed without a merge | deleted 24.11 (dependabot recreates it on a new revision) |
| `release/0.40.1` | the release-train: PR #2017 closed without a merge | deleted by the dispatcher 24.11 (v0.40.1 published 22.11T05:01Z — the train has departed) |
| `release/0.40.1-base` | the release-train base (no PR at all) | deleted by the dispatcher 24.11 (the same verdict) |

## The branch-hygiene rule

- **(a)** Closed a PR without merging — delete the head branch in the same act (the auto-delete covers merges only).
- **(b)** Durable branches only: `main`, `watchlist-digest`, the heads of open PRs, the active release-train branches.
- **(c)** A monthly sweep — reconcile the branch list against this table.
- **(d)** No PR-"probes" (the #2056 lesson: a real PR from the payload).

## Sweep 25.11 (№1002, the completion)

- **The branch list on 25.11: `main` + `watchlist-digest`** — exactly the «How to check» of #2063: two durable branches, no open PRs, no release branches (the train departed 22.11), no orphans.
- `delete_branch_on_merge` confirmed in practice: the branches `naryad-997-wl07-watchlist` (PR #2062), `naryad-1007-gitattributes-eol` (PR #2079), `naryad-1008-w3-gates` (PR #2081), `naryad-1009-record-windows` (PR #2082) disappeared on merge with no manual deletions.
- No new orphans found (every 25.11 naryad went through a merge; no closed-without-merge PRs).
- The next sweep — per rule (c), monthly; an off-schedule one on any owner directive.

## History

- 24.11: the audit R84–R85 issued in the #2063 body (the table and the rule — the verbatim lives there; the two dispatcher corrections of 24.11 are documented in the issue comments: the `v0.40.1` tag, the unformatted `%s`).
- 25.11: the file restored on main (this document) by the №1002 completion; the sweep addendum above. The first restoration attempt (the same day, PR #2083, commit faaed504) was Cyrillic and went RED on the docs language lint + the stale gen_metrics counters — both fixed in the corrective commit: the file translated to English (the allowlist NOT grown — that requires an owner decision), all the regenerated counters committed, `gen_metrics --check` verified green BEFORE the push.

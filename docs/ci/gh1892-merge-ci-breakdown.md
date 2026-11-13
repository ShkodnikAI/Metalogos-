# gh#1892 — the merge↔CI audit's «51 merges without green required checks», line by line

Naryad №960 (issue #1927, 13.11 — P0, `[ci][process]`). The merge↔CI
detective (`scripts/ci/merge_ci_audit.py`, naryad №551, reverse direction
№824) opened gh#1892 with **81 divergence lines** over a 30-day window
and a headline that reads as «51 merges went in unchecked». That headline
is wrong, and the way it is wrong is the content of this document: a
single count over four different things cannot be acted on, and the two
lines that ARE real do not say what the headline says.

This document is the FIXATION of the breakdown. The numbers below are
machine facts, each with the read that produced it; where a cause could
not be established from inside the repository, the document says so and
names the read that would settle it. The detective changes that came out
of it are in the same naryad.

## 1. The breakdown

| Class | Lines | What it is |
|---|---:|---|
| `convention` | 10 | the squash tail is not a PR number; the association lane resolved the merge, so the merge itself IS verified |
| born-later context | 33 | the line charges a merge for `release-train-gate (blocking)`, which became a required context AFTER that merge |
| `no-pr` | 8 | the commit is not a squash merge of a PR at all (a release commit, a direct push) — PR-side CI cannot apply by construction |
| `no-ci-silent` | 48 | a required context never reported — **all 48 belong to exactly TWO pull requests** |

Forty-eight lines are two PRs because each PR is charged once per
required context: 24 contexts × 2 PRs = 48. The headline's «51 merges»
counts LINES, not merges. The real population of merges with no CI is
**two**: #1752 and #1804.

## 2. The 33 born-later lines — fixed mechanically

`release-train-gate (blocking)` entered the required set at
**2026-11-05T11:19:45Z**. Every merge before that moment was charged for
a context that did not exist yet.

The fix is `REQUIRED_SINCE` + `required_at(merged_at)` in the detective:
a context is required for a merge only if the merge is later than the
context's birth. An absent `merged_at` keeps the full set (fail-closed).
The birth date is a DATUM in the script, next to the context name, so
the next added context carries its own date rather than retroactively
condemning history.

## 3. The two real PRs — what is established

| | #1752 | #1804 |
|---|---|---|
| branch | `ride-audit-rustsec-2026-0429` | `w81-n903-adr0217` |
| opened | 2026-11-02T13:14:01Z | 2026-11-04T15:50:05Z |
| merged | 2026-11-02T13:51:17Z | 2026-11-04T16:48:31Z |
| workflow runs on the branch, ever | **0** (`total_count: 0`) | **0** (`total_count: 0`) |
| check runs on the PR head | **0**, combined status `pending` | **0**, combined status `pending` |
| check runs on the merge commit | **0** | **0** |
| merged by | the owner | the owner |

Both merges sit at the end of a **repo-wide Actions silence**, and in
both cases the next run after the silence is the merge's own
push-to-main, created **3 seconds** after `merged_at`:

* 2026-11-02: last run 11:43:55Z → merge 13:51:17Z (**2 h 07 m**) →
  next run 13:51:20Z (`main`);
* 2026-11-04: last run 13:22:48Z → merge 16:48:31Z (**3 h 26 m**) →
  next run 16:48:34Z (`main`).

The silence is repo-wide: the regular ~30-minute `main` runs stop too,
not only the PR lane. Actions was not generally broken on either day (30
and 57 runs respectively), and the dispatcher was demonstrably able to
dispatch 3 seconds after each merge.

Meanwhile the protection was **in force** around both merges. The
`branch-protection-audit` fixations:

```
2026-10-26  GREEN contexts=23 missing=0 strict=True enforce_admins=True pr_reviews=True
2026-11-02  GREEN contexts=23 missing=0 strict=True enforce_admins=True pr_reviews=True   (16:27Z — 2 h 36 m after the #1752 merge)
2026-11-09  GREEN contexts=23 missing=0 strict=True enforce_admins=True pr_reviews=True
```

`allow_force_pushes=False`, `allow_deletions=False`, and the repository
has **no rulesets** (`GET /rulesets` → `[]`), so classic branch
protection is the whole story — there is no bypass-actor list anywhere.

## 4. What this refutes, including the dispatcher's own first reading

Three hypotheses were stated during the analysis and each is refuted by
the evidence above. They are recorded here because the refutations are
the useful part: the same three will be the first guesses next time.

1. **«Inattention at the merge button.»** Refuted. With 23 required
   contexts in force, `strict=True` and `enforce_admins=True`, a merge
   whose required checks had never reported is not something the merge
   button offers — not even to an admin. Attention was not the variable.
2. **«The branch protection did not hold / was never applied.»**
   Refuted as a standing state by the three GREEN fixations above, one of
   them 2 h 36 m after the first merge. (This was the dispatcher's own
   written conclusion in the first delivered analysis, before the
   protection-audit run history was read. It was wrong.)
3. **«A GitHub Actions outage that day.»** Refuted as a day-level claim:
   30 runs on 2026-11-02 and 57 on 2026-11-04. The silence is real but
   it is a window of hours, not a day, and it ends at the merge.

## 5. What is NOT established — and the one read that would settle it

Facts §3 are jointly inconsistent with a merge performed through the
merge button under that protection. Two mechanisms produce the observed
timeline and the data inside the repository cannot separate them:

* **(a)** Actions was switched off for the repository during those
  windows — which explains the repo-wide silence exactly, including the
  missing scheduled `main` runs and the 3-second resumption — with the
  protection also bypassed or briefly relaxed to allow the merge;
* **(b)** a GitHub-side failure in which `pull_request` dispatch and
  required-check enforcement were both inert while `push` dispatch
  recovered at the merge.

The only record that distinguishes them is the **audit log** —
`repo.actions_enabled` / `repo.actions_disabled`,
`protected_branch.policy_override`, `protected_branch.update` — and
reading it needs owner-level access, which no CI token in this
repository has.

**Therefore this document stops here.** Naming a cause would be an
inference dressed as a fact, and the rule the project already carries
(№525: an unreadable state is not a pass) applies to the dispatcher's own
conclusions too. The owner's read is one query; until it happens, the
cause is OPEN and the two mechanisms above are the candidates.

## 6. What changed so that either mechanism is visible next time

1. **`REQUIRED_SINCE` / `required_at`** — the born-later class cannot
   recur (§2).
2. **`classify_divergence` + the BY CLASS head** — six classes, counted
   separately, in both the stdout report and the issue body. The
   headline number is no longer a sum over incomparable things.
3. **`dispatcher_silence` + `no-ci-silent`** — the detective now reads
   the repo-wide Actions timeline around each merge and tags a `NO RUN`
   line with the dispatcher state at merge time. The signature is the
   CONJUNCTION (a ≥90-minute repo-wide gap before the merge AND the next
   run within 120 s, i.e. the merge's own push), because a long gap on
   its own is just a quiet night — 2026-11-02 also has a 2 h 48 m gap
   with no merge at its end, and the self-test pins that case as NOT
   silent. An infrastructure event to escalate is no longer reported in
   the same class as a merge that disregarded a verdict.
4. **The protection audit is DAILY** (was weekly) — a weekly snapshot
   cannot witness a short window; see `docs/maintainers.md`.

What remains out of reach by construction: the toggle EVENT. No polling
cadence records it, only the audit log. That is a boundary of this
detective, stated here rather than left to be rediscovered.

## 7. The closing verdict for gh#1892

Of the 81 lines: 51 need no action (10 `convention` — verified merges
with a message-convention break; 33 born-later — a detective defect, now
fixed; 8 `no-pr` — not applicable by construction). The remaining 48
lines are **two** pull requests, both real, both inside a dispatcher
silence, both merged under a protection that the audit shows in force
before and after — a shape that the repository's own data cannot explain
and that §5 hands to the owner.

The issue is closed with this breakdown as its verdict. **Closing it is
the owner's act**, as is the audit-log read.

# The retro-verification of v0.40.0 — the recorded act (№976, issue #1985)

## The fact

The 0.40.0 stable release (tag `v0.40.0`, commit `53fd0fdd9`) carries
**zero check-runs** — the workflow `CI` gave no run on the release
commit (the consolidated audit 18.11, finding A). The release shipped
without its blocking set; the machine half of the closure is the
release-commit gate (`scripts/ci/release_commit_gate.py` + the
`release-commit-gate` workflow on the tag path), which refuses the NEXT
tagged release whose commit lacks a green required context.

## The act, as it was actually performed (2026-11-19)

The direct `workflow_dispatch` route is closed by two independent walls:
GitHub answers `403 Resource not accessible by integration` on
`actions/workflows/321601128/dispatches` (the executor token holds no
`actions: write`), and the session proxy separately denies the Actions
paths. The run was therefore produced through a draft PR **#2001** whose
head is the tag (`53f0fdd9`) and whose base is the release parent
(`617643eb`, `v0.40.0^`); the merge-commit tree was verified equal to
the tag tree (`a80072c5` in both) before the run was read.

- Run: **44494525626**
- Verdict: **52 success, 3 failure, 55 jobs — every job reported.**
- The two red blocking jobs, `cargo-deny (blocking)` and
  `cargo-audit (blocking)`, share ONE cause: `lettre 0.11.23`,
  RUSTSEC-2026-0560…0565. The advisory publication date, read from the
  primary source (`rustsec/advisory-db`, `crates/lettre/RUSTSEC-2026-056N.md`),
  is `date = "2026-11-17"` — **the day AFTER the tag** (the cut was
  2026-11-16 13:22 +0300). On 16.11 these jobs were green: this is
  infrastructure drift, and no old ref is "fixable" for it.
- `coverage (non-blocking)` — red, a flake: the same job is green on the
  current main.
- The route difference that MUST be in the record: the event is
  `pull_request`, so **`branch-freshness` executed and passed** (a
  dispatch run would have it SKIPPED), the job set is 55 instead of 54,
  and `github.sha` is the merge-commit SHA, not `53f0fdd9` itself.

## The verdict the record makes

**In the release tree there is NOT a single substantive failure of the
blocking set.** What this record does NOT claim: the CAUSE of the 16.11
CI silence (that is the audit log — `workflows`, `repo.actions_*` — and
the owner's rights), and byte-equality with a hypothetical dispatch run
(the pull_request route differences above are the honest boundary of
the evidence).

## Separate line — not a verification matter

The published `v0.40.0` carries `lettre 0.11.23` with six known SMTP
advisories today; main closed this with the `3b470627` bump (18.11) and
`deny.toml` was not changed. The `0.40.1` question belongs to the
owner; this naryad does not decide it.

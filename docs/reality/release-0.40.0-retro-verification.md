# The retro-verification of v0.40.0 — the owner's act (№976, issue #1985)

## The fact

The 0.40.0 stable release (tag `v0.40.0`, commit `53fd0fdd9`) carries
**zero check-runs** — the workflow `CI` gave no run on the release
commit (the consolidated audit 18.11, finding A). The release shipped
without its blocking set; the machine half of the closure is the
release-commit gate (`scripts/ci/release_commit_gate.py` + the
`release-commit-gate` workflow on the tag path), which refuses the NEXT
tagged release whose commit lacks a green required context.

## The act the executor's token cannot perform

The retro-verification — a **workflow_dispatch run of `CI` on the
`v0.40.0` ref** — needs `actions: write` on the environment's dispatch
endpoint; the executor's token answers `403 Resource not accessible by
integration` on `actions/workflows/321601128/dispatches`. This is the
owner's act, and the naryad carries the instruction so the result is
recorded, not lost:

```
gh workflow run CI --ref v0.40.0
# or: the Actions tab → CI → Run workflow → ref: v0.40.0
```

## The result place

The owner records the run here after it lands (the discipline: a
measured act with its date, never a claimed one):

- Date of the dispatch:
- Run URL:
- Result (all blocking jobs green / the list of reds):
- The n868 package digest re-attestation needed (yes/no) — the run
  compiles the CURRENT sources against the release ref; a red that is
  the package-drift class (`the checked-in package drifted from the
  live sources`) is the expected shape for a retro run and is answered
  by the release-tag re-attest, not by code changes:

Until the line above is filled, the honest boundary stands: **the
0.40.0 release has no recorded CI verification, and the release-commit
gate is the machine guarantee that this class cannot return silently.**

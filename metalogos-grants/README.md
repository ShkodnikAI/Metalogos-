# metalogos-grants — the grants lane (the owner-gated surface)

This directory holds the DRAFT grant-application packages. Everything
here is built from public repository facts (the §16.0-7 sanitization:
no market analysis, no forward bets) and prepared FOR the owner — the
release of anything external is the owner's gate alone (the №541
postulate; №549, issue #887).

## Contents

- `nlnet-traction/01-metrics-one-pager.md` — the honesty-metrics
  one-pager: repository facts with dates, machine-generated where a
  generator exists.
- `nlnet-traction/02-demo.md` — the demo narrative: the language →
  serve → taint-labels → action-ledger arc, reproduced by ONE command
  (`bash scripts/demo_traction.sh`), over the public examples
  verbatim.

## The gates

- The demo script runs in this repository's CI-less lane (a human-run
  script, not a CI job) — it must keep reproducing on `main` (the
  pull-request lanes run the full blocking set).
- Nothing in this directory is published anywhere by an executor.
  Wordings to external programs (NLnet/Restack) are the owner's
  (№541); M1 — 2026-11-03.

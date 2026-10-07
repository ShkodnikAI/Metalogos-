# ADR-0177 §4 — the unfreeze-criteria summary (№482 machine check)

The one-page machine verdict over the four unfreeze criteria. The right to lift the freeze belongs to the OWNER ONLY (ADR-0177 §4) — this summary collects evidence, it decides nothing. Release gate (ADR-0177 §6): **0.27.0 is NOT published while any §4 criterion is red.**

| § | Criterion | Verdict | Evidence |
|---|-----------|---------|----------|
| 4.1 | Types — the typed-signature share grows (№467) | **GREEN** | share typed signatures: 309/516 (59.88%); the enforced floor 5988 — №623 (gh#1086): the 7 parameterized rows (List<Tool>, bp; the latest recorded floor 5988 — №623 (gh#1086): the 7 parameterized rows (List<Tool>, bp |
| 4.2 | Dedup — the TW/VM duplicate names at/below the threshold (№462) | **GREEN** | count 0 (threshold 0); mirrors 6 (threshold 6) |
| 4.3 | Debt — the ignore/dead_code counters green (№468) | **GREEN** | ignore 11/11; ignore_todo 0/0; dead_code 15/15; example_uncovered 0/0 |
| 4.4 | Memory — the office E2E dogfood (office#373, FO-056) | **RED** | the office record verdict: GREEN (the §4.4 criterion evidence is complete: the in-repo | the in-repo twin (naryad_429_memory_office_path) FAILED on this commit |

> 4.4 (Memory): - the office repo is private — the live office CI status is not queryable from the Metalogos CI (no cross-repo token); the checked-in record is the machine-readable evidence, refreshed by the office-side naryads

**Overall: RED.**

## The v2 gate — the 0.30 ABSOLUTE goals (№615/№626/№630, ADR-0186 — OWNER-FIXED 2026-10-07, wired by №630)

| § | Goal | Verdict | Evidence |
|---|------|---------|----------|
| v2 | The absolute goals: parameterized share ≥ goal (the №623 third metric — the Z-2 successor of the scalar typed/precise parameters), 0 open High (server path), the domain quorum, the serve-e2e inventory | **RED** | parameterized share 3736 bp vs goal 5000 bp: NOT MET; open High (server path) 0 vs goal 0: MET; domain quorum 0/8 vs goal 1/3: MET; the serve-e2e inventory: all state-accumulating declarations done |

**Overall (§4 + v2 0.30): RED.**
The 0.30 release gate: **WIRED** (the DEFAULT gate target — №630, ADR-0186 §5, the owner's verdict 2026-10-07; the strict release-time read, --strict, exits 1 on RED). The branch-protection criterion stays a DRAFT line — gh#1000 was open at the fixation, the fact key lands via a micro-PR after its closure (№525: no checker, no fact).

<details><summary>the raw gate outputs</summary>

**4.1 (types), gate exit 0:**

```
typed signatures: 309/516 (59.88%)
precise signatures: 218/516 (42.24%) — №560: the two shares side by side
parameterized signatures: 34/91 (37.36%) — №623: the third share (among List/Struct)
field-label signatures: 3/20 (15.00%) — №627: the fourth share (among the parameterized Struct rows)
```

**4.2 (dedup), gate exit 0:**

```
tw-vm duplicated builtin names: 0 (threshold 0)

src/ mirror mentions: 6 (threshold 6)
```

**4.3 (debt), gate exit 0:**

```
ignore: 11 (threshold 11) OK
ignore_todo: 0 (threshold 0) OK
dead_code: 15 (threshold 15) OK
example_uncovered: 0 (threshold 0) OK
dup (via №462 gate): tw-vm duplicated builtin names: 0 (threshold 0) OK
DEBT GATE OK — every counter at or below its threshold.
```

**4.4 (memory), gate exit 1:**

```
# №482 (gh#730): the checked-in evidence record for the ADR-0177 §4.4
# criterion — the memory office E2E dogfood (office#373, FO-056).
#
# WHY A RECORD: the office repository (ShkodnikAI/FOSVED-office-v2) is
# PRIVATE — the Metalogos CI has no cross-repo token, so the office CI
# status cannot be queried from a Metalogos workflow. The record below
# is the checked-in fact of the last VERIFIED office-side run; it is
# refreshed by the office-side naryads (the №475 precedent) and by the
# office CI itself landing on main. The summary job reports this record
# verbatim and labels it "evidence-record" — it never guesses.
#
# The criterion is GREEN when:
#   (a) the in-repo twin (tests/naryad_429_memory_office_path.rs) is
#       green on the current commit — run live by the unfreeze-gate job;
#   (b) this record is present and well-formed (fail-closed otherwise).
#
# ── Record (refreshed: 2026-10-05, the gh#981 resolution recount; before that: 2026-10-04, the owner decision 4А recount; 2026-09-27, by naryad №482) ──
office_repo: ShkodnikAI/FOSVED-office-v2 (private)
naryad_issue: office#373 (FO-056, closed 2026-09-25)
landing_pr: office PR #375, squash ec1c048 (2026-09-25)
scenario: test_memory_fo056.mlog — 16 labels (put → recall w/ consent →
  negative recall w/o consent → derive → forget dry_run → forget apply
  w/ grant → MEMORY_POISONED on derived → ttl-expiry → retain-ttl
  live/poisoned → ledger_export + ledger_verify)
pytest_suite: tests/test_fo056_memory_e2e.py — 3 tests
verified_runs:
  - 0.25.0 (bin/mlog, 2026-09-25): scenario 16/16 PASS, rc=0; pytest 3/3
  - 0.24.0 guard (bin/mlog-024, 2026-09-25): scenario 16/16, labels
    pairwise identical to 0.25
  - 0.26.1 pinned (the Wave 18 recount, gh#680 `n475: fo056-evidence`):
    scenario 16/16 + pytest 3/3 — the evidence the ADR lift carried
  - the office-pinned lineage recount (the office repo's checked-in
    bin/mlog, the version string "mlog 0.27.1" — an OFFICE-SIDE build,
    2026-10-04, the owner decisi
```

</details>

ADR: docs/adr/0177-domain-freeze-until-027.md · generated by `scripts/ci/unfreeze_gate.py` (№482, gh#730) — the summary is machine-checkable, the lift is not a machine act.

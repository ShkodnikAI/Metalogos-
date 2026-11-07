# ADR-0221: Terminal Stop 1.0 — the 1.0 Assembly Record: the Terminal Stop of the Phase 8 Train, the 1.0 Gate Definition, and the Aggregate Machine Fact TERMINAL_STOP (the W-T Wave Record)

- Status: **Accepted** (the wave-85 acceptance — TERMINAL_STOP 13/13
  at the floor 13, gate_046; the dispatch gh#1853 closed completed
  2026-11-06 23:01Z with the acceptance comment)
- Date: 2026-11-06
- Deciders: the OWNER (the R38 train terminal stop — the owner's
  directive of 2026-11-06 «Заводи следующую волну нарядов согласно
  стратегического плана развития языка Металогос» = the gate pass on
  the start of the 1.0 assembly; the wave-85 dispatch gh#1853; the
  plan canon §23 and ADR-0201 §2.2 fix the terminal definition), the
  issuer (the wave-85 registration — the office main `d8bf00fe`, the
  §16.0-1 rule)
- Naryad: №923 (issue #1850, wave 85 — W-T); the machine lands by №924
  (issue #1851), the readiness report by №925 (issue #1852)
- Depends on: ADR-0201 (the Phase 8 through-line — §2.2 «the train
  ends at 1.0 (the plan's terminal stop)», §3.2 the external-audit
  boundary), the plan §23 (1.0 = all 13 facts GREEN + the external
  audit + ≥1 paying pilot), ADR-0189 §5 (the calendar train), ADR-0186
  §4.5 (the publication is the owner's act), ADR-0195 (the
  release-gate-criteria precedent), R38(3) (the audit budget
  reserved), R38(9) (the partner track — the owner's time), ADR-0219
  (the legal shield), ADR-0220 (the verticals)
- Blocks: №924 (the TERMINAL_STOP machine + gate_046), №925 (the
  terminal-stop readiness report), the 1.0.0 release path

## 1. Context

The R38 subphase train (ADR-0201 §2.2) is exhausted: 0.47 = 8H-2 (the
verticals, VERTICAL_PROFILES) closed with wave 84 — the acceptance
2026-11-06 05:27Z (the dispatch gh#1842 closed completed; the merges
№920–№922 — PR #1843/#1844/#1849, the squash heads b4526be6,
d1066e6a, 47f8cb92; 54 checks success on every merge commit). All 13
capability slots of the plan §23 carry GREEN machine facts — 14 of
them (C4 carries LICENSED_FLOOR and PASSPORT_FRESH; C13 carries
INSTITUTIONAL_READY and VERTICAL_PROFILES; C7/C12 share
REPRO_BITFORBIT) — each behind its own strict gate roster
(scripts/ci/gate_033_goals.txt … gate_045_goals.txt) and its own
composition sites (the №874/№912/№918/№921 fail-closed lineage).

What the terminal stop does NOT yet have:

1. **A single machine expression of "all 13 GREEN".** The 14 facts are
   each gated, but no fact answers the terminal question in one number.
   A consumer (the owner, an auditor, an external consumer) must read
   14 gate reads to state the obvious — and nothing prevents a prose
   «сборка готова» claim from outrunning the machine.
2. **The evidence shapes.** Two terminal components live OUTSIDE the
   repository by the ADR-0201 §3.2 boundary: the external audit (the
   procurement is the owner's act, the budget is reserved per R38(3) —
   «no figures, vendors, or scheduling live in this repository») and
   the paying pilot (the owner's business act, R38(9)). The repo must
   carry the digest-pinned evidence FORMS so the owner's acts, when
   they happen, verify mechanically — not a new prose promise.
3. **The release path for 1.0.0.** The calendar train (ADR-0189 §5)
   replaced the R38 «subphase = minor» rule; the 1.0.0 minor must state
   explicitly how it departs (the same Sat rc → Sun stand → Mon
   publication discipline; the publication stays the owner's act).

The wave-84 acceptance tail recorded the exact standing rule: the next
subphase — the 1.0 assembly — is BEHIND the owner's gate. The owner's
directive of 2026-11-06 is that gate pass. This record fixes the shape
of what the assembly builds before any of it is code (the N1 rule —
the ADR-0201 §2.4 discipline).

## 2. Decision

### 2.1 The 1.0 gate

**The 1.0 gate := all 13 capability slots GREEN (14 machine facts,
each through its own strict gate) + the external-audit evidence + the
paying-pilot evidence** (the plan §23 verbatim; ADR-0201 §2.2).

A slot is GREEN ⇔ ALL its machine facts pass their strict gates:

| Slot | Machine fact(s) | Gate |
|---|---|---|
| C1 | ATTEST_MAP ≥ 30 | gate_037 |
| C2 | GRANT_COVERAGE 10000 bp | gate_034 |
| C3 | PROVENANCE_SHARE 10000 bp | gate_038 |
| C4 | LICENSED_FLOOR ∧ PASSPORT_FRESH | gate_033 |
| C5 | OFFLINE_SUITE ≥ 8000 bp | gate_039 |
| C6 | SEMA_EXTERNAL ≥ 1 | gate_035 |
| C7 + C12 | REPRO_BITFORBIT 10000 bp (one fact, two slots) | gate_043 |
| C8 | REPLAY_MATCH 10000 bp | gate_041 |
| C9 | TAINT_SATURATION ≥ 10⁹ | gate_042 |
| C10 | JURISDICTION_MATRIX ≥ 3 | gate_040 |
| C11 | APPROVAL_COVERAGE 10000 bp | gate_036 |
| C13 | INSTITUTIONAL_READY ≥ 12 ∧ VERTICAL_PROFILES ≥ 12 | gate_044, gate_045 |

### 2.2 The aggregate machine fact

**`TERMINAL_STOP :=` the count (0–13) of GREEN capability slots** —
the single number of the 1.0 gate. The definition is fixed by this
record BEFORE the code (the №891/№903/№916 form — the ADR fixes the
definition, №924 builds the machine):

- The aggregate is built OVER the live checks (the №735 form — "the
  fact together with its checker"): a slot counts ONLY through a call
  to the live strict check of each of its facts (the same roster +
  composition-site logic the --check runs). The machine never
  re-implements a sub-check and never synthesizes a green.
- **Fail-closed:** a dead roster, a dead composition site, a thrown or
  errored sub-check → the slot does NOT count (0); an empty roster is
  0 (a vacuous class is a gate failure, never a pass — the №862
  discipline).
- **The floor 13, only-up:** the dispatch gh#1853 constant; a lowered
  floor is refused with a loud FLOOR LOWERING REFUSED (the №776
  transaction discipline — a re-pin is an owner decision, never a
  silent gate edit).
- The roster file scripts/ci/gate_046_goals.txt pins
  `goal_terminal_stop_min 13` and the slot→fact map (the №921 form —
  the machine and the human read the same comment); the GENERATED
  rows in metrics.md / PLAN-SUMMARY carry the number (the №900 form).

### 2.3 The evidence shapes — digest-pinned (the G5/№828 form)

1. **The external-audit evidence** = a manifest of the evidence bundle
   (the audit scope: the C1–C13 surfaces + the ADR chain 0201–0220 +
   the machine facts gate_033–046 — the №925 outline), the manifest's
   digest pinned explicitly. The vendor, the figures, the schedule,
   the report contents — outside the repo (the ADR-0201 §3.2
   boundary); the re-verification runs against the digest.
2. **The paying-pilot evidence** = the owner's record carrying the
   carrier digest (the №925 form). The contract, the invoice, the
   counterparty details — outside the repo (R38(9)); nothing in the
   tree names them.

Both evidences land in the owner's hands BEFORE the 1.0.0 minor
departs; the repo verifies PRESENCE and INTEGRITY of the digest pins,
never the substantive force of the audit or the business validity of
the pilot (§5).

### 2.4 The release path for 1.0.0

The 1.0.0 minor departs the calendar train (ADR-0189 §5): the Saturday
12:00 MSK rc candidate from main, the Sunday office-stand
verification, the Monday final tag ON THE SAME COMMIT; the publication
is the OWNER's act (ADR-0186 §4.5). The release_train_gate (the
consecutive-minors rule) stands unchanged; the empty-week rule (§2.3)
stands: no evidence, no content, no departure. The 1.0.0 notes list
the Phase 8 subphases per the §13.13.2 rule (the subphases are named
in the release notes of the week that carried them).

## 3. Parents consumed by reference

- **ADR-0201** §2.2 (the train ends at 1.0 — the plan's terminal
  stop) and §3.2 (the external audit happens before the terminal
  stop; no figures, vendors, or scheduling live in this repository) —
  consumed verbatim, not re-argued.
- **The plan §23** (the office canon) — the terminal definition «1.0 =
  все 13 фактов GREEN + внешний аудит + ≥1 платящий пилот»; the
  market sections stay in the canon (the §16.0-7 access mode).
- **R38(3)** — the external-audit budget is reserved; the procurement
  is the owner's act. **R38(9)** — the partner track (the audit
  firms → C1, the insurers → C2) is the owner's time, outside the
  naryads.
- **ADR-0189 §5** (the calendar train) and **ADR-0186 §4.5** (the
  publication is the owner's act) — the release path stands on them,
  unchanged.
- **ADR-0195** — the release-gate-criteria record precedent.
- **ADR-0219** (the legal shield) and **ADR-0220** (the verticals) —
  the terminal components consumed by reference; this record adds
  nothing to their surfaces.

## 4. Consumption contracts

- **№924** (issue #1851) lands the machine: the TERMINAL_STOP
  aggregate in scripts/gen_metrics.py + gate_046 (the roster file +
  the floor 13) + the tamper traps 55–58 (the classes (а)–(г): the
  forged slot counted without a live checker, the roster loss, the
  dead composition site, the floor lowering — the №921 lineage). The
  test carries the quote-literal `TERMINAL_STOP :=` from this ADR —
  the mutation contract: deleting the literal from this record makes
  the №924 test red (the quote-literal not found).
- **№925** (issue #1852) lands the terminal-stop readiness report
  (docs/audits/terminal-stop-readiness-2026-11.md — the №907/№913/
  №919/№922 form): the 13/13 roster verdicts, the audit-scope
  outline, the evidence-bundle manifest (the digest pinned at issue
  time), the repo↔owner-acts boundary, the 1.0.0 release path, the
  cross-links to the legal shield (№919), the verticals (№922), the
  C7 trigger (№913 — not ignited by the 1.0 gate).
- The 1.0.0 release naryad is NOT part of wave 85: it departs after
  the owner holds both evidences (§2.3) — a later wave, on the
  calendar train.

## 5. Honest limits

1. The machine verifies the SLOT COUNT and the digest pins' PRESENCE
   — not the audit's substantive force, not the pilot's business
   validity, and never the «quality as a service» of the product.
2. The 1.0 gate is NOT a certification, NOT a legal advice, NOT a
   regulator's acceptance (the C10 verbatim — the №885/ADR-0209
   form).
3. The audit procurement (the vendor, the figures, the schedule) and
   the pilot contract (the counterparty, the invoice) — the owner's
   acts, outside the repo, named here as boundaries: the R38(3)
   budget, the R38(9) partner track. The repo carries the digest
   forms only.
4. The C7 reproducible-build trigger (the first gov/bank pilot —
   R38(8), №913) is NOT ignited by the 1.0 gate or by a vertical
   profile (the ADR-0220 §2 rule stands); the trigger stays the
   owner's.
5. This record schedules nothing: the calendar train departs weeks
   only on content (ADR-0189 §2.3); the 1.0.0 minor departs when the
   owner holds the evidences — the machine cannot manufacture them.

## 6. Migration discipline

- A docs/ADR record: zero code behavior changes, zero grammar/builtin/
  golden changes, zero floor-gate drift (the 14 facts keep their
  floors; the share gates (type_signature/sensitive_surface) are NO
  DRIFT without a ride).
- The aggregate adds one GENERATED row (the №924 contract) — the
  metrics surface grows by one line, no existing row moves.
- The status wiring of ADR-0219/0220 (Proposed → Accepted by the
  wave-83/84 acceptances) follows the 0216–0219 precedent and stays
  outside this record's scope (a separate status pass).

## 7. Acceptance

- The PR lands this ADR + the docs/adr/README.md index row 0221 +
  llms.txt (208 → 209 ADR; the naryad range bump — the §6.79 lesson)
  + the CHANGELOG [Unreleased] entry.
- CI green on the merge commit; the docs-domain gates NO DRIFT.
- The mutation boundary: deleting the `TERMINAL_STOP :=` literal from
  this record makes the №924 test red (the quote-literal not found).
- The wave-85 acceptance fixes the status as Accepted (the
  ADR-0217–0220 form).

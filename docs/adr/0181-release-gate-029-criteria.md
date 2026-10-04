# ADR-0181: Release gate 0.29 criteria

- Status: Accepted (the parameters fixed by the OWNER, 2026-10-04)
- Date: 2026-10-03
- Deciders: the owner (the parameters fixation — 2026-10-04, the draft
  accepted verbatim; the authorization record: «1. А, 2.А, 3. Б.
  -выполняй»), the executor (the draft)
- Naryad: №570 (gh#934, Wave 26)
- Predecessor: [ADR-0179 — release-gate-028-criteria-v2](0179-release-gate-028-criteria-v2.md)

## §1. Context

The 0.28.0 release shipped on the ADR-0179 v2 gate (SATISFIED, 2026-10-03).
The release-gate line continues: the 0.29 cycle needs its next contour at
the moment the 0.28.0 cut lands. This ADR proposes the 0.29 ABSOLUTE goals
in the ADR-0179 shape — the machine record lives in
`scripts/ci/gate_029_goals.txt` (the same structure as
`gate_028_goals.txt`), the same fail-closed discipline (a fact without a
machine source is itself a failure, №525).

## §2. Succession with ADR-0179

Everything in ADR-0179 carries over unless this ADR says otherwise:

- the gate direction — ABSOLUTE goals, not no-regress ratchets (the
  0.27.0 lesson);
- the §6 window checklist shape (the release-block sync, the quorum sync,
  the serve-e2e inventory, the v2 run, the owner's read);
- the parameter authority — the OWNER fixes the parameters (§3 here,
  §3 of ADR-0179);
- the fail-closed fact discipline and the checker set (№525; this draft
  introduces NO new fact_* keys — the 0.29 record reuses the 0.28
  checkers verbatim).

What changes: the goal values (§3) and the debt-goal surface (the №569
re-locked floors become the 0.29 debt goals; they ride the §4.3 legacy
criterion — no duplication into fact keys).

## §3. The parameters (OWNER-FIXED 2026-10-04 — the draft accepted verbatim)

| Parameter | Draft value (№570) | The owner's fixation | Cost to the contour |
|---|---|---|---|
| goal_typed_share_bp | 3500 (+300bp over the achieved 3000; the live fact at the 0.28.0 cut is 3241) | **fixed: 3500 (verbatim)** | ≈+14 typed signatures over the 0.28.0 fact (165 → 179 of 509; 178 = 3497bp < 3500) — a fraction of one №543-class wave. The draft's "~+260 (165 → ~425 of 509)" was an arithmetic error (×20), corrected 2026-10-05, the fixed VALUES untouched; the live fact at the correction is 186/509 = 3654bp — the goal is already met (№573) |
| goal_open_high_server | 0 (unchanged) | **fixed: 0 (verbatim)** | none — the release-block label discipline holds |
| goal_quorum_num / den | 1 / 3 (the 0.28 form kept verbatim; the live fact is 0/8) | **fixed: 1 / 3 (verbatim)** | none — the headroom is already honest |
| debt goals (ignore / ignore_todo / dead_code) | the №569 floors: 26 / 0 / 33 (ride §4.3, only-down) | **fixed: the №569 floors 26 / 0 / 33 (verbatim)** | none — the floors are already the fact |

**The fixation happened on 2026-10-04 — the OWNER's gate (§5).** The
owner accepted the draft verbatim (the authorization record: «1. А,
2.А, 3. Б. -выполняй»; the executor ran the mechanical edit and owns
nothing about the values). The fixation column above is filled,
`owner_fixed: true` stands in `gate_029_goals.txt`, and the v2 reader
now reads the 0.29 record the same way it reads 0.28
(criterion_goals_029 → criterion_goals_028 verbatim). The typed-share
goal (3500 bp) is a target AHEAD of the live fact (3241 at the 0.28.0
cut) — an honest NOT MET until the typing waves land; the 0.29 gate
still blocks NOTHING until it is wired into the blocking CI (§6).

## §4. The 0.29 window checklist (the ADR-0179 §6 shape)

1. **The release-block sync** — `fact_open_high_server` = the count of
   the open issues labeled `release-block`; the issue numbers into the
   record; the syncing PR linked from the release notes.
2. **The quorum sync** — `count_duplicated_names.py --quorum` over the
   record.
3. **The serve-e2e inventory** — every state-accumulating declaration
   `done` in the PR that adds the both-backend test.
4. **The gate, v2 mode** —
   `python3 scripts/ci/unfreeze_gate.py --office-tests <pass|fail> --gate-target 0.29`;
   the summary attached to the release; a RED anywhere blocks the read.
5. **The owner's read** — the OWNER decides the publication; the machine
   never publishes.

## §5. The parameter-change rule

The parameters move ONLY by the owner's explicit fixation (an edit to
`gate_029_goals.txt` setting `owner_fixed: true` with the values, or a
direct instruction to the executor recorded in the gate record). The
executor never fixes, never loosens, never tightens a parameter on its
own. Any change after the fixation is a NEW draft cycle of this ADR.

## §6. Honest limits

- The 0.29 gate is NOT in the blocking CI — the wiring is a separate
  decision after the owner's fixation (the 0.28 pattern: the parameters
  first, the CI ratchet after).
- The draft typed-share step (+300bp) is derived from the achieved-goal
  shape, not from a new audit — the owner may re-derive it from the 0.29
  audit facts instead.
- The debt goals ride the §4.3 criterion; a future re-lock in
  `debt_baseline.txt` moves them automatically (only-down).

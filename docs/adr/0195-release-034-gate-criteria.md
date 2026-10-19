# ADR-0195: The 0.34 Release-Gate Criteria — the draft variants (PROPOSED; the owner's verbatim pending)

- Status: **Proposed** (the draft, №731, Волна 51; §6 «The verdict
  package» is under the owner's verbatim — zero self-capture, the
  OD-дисциплина №659-b; this ADR wires NOTHING: ноль CI-правок, the
  wiring rides the owner's verbatim — the №723/ADR-0193 §7 form)
- Date: 2026-10-20 (the draft, №731, Волна 51)
- Deciders: the executor (the draft); the OWNER fixes the parameters
- Naryad: №731 (issue #1360, Волна 51)
- Predecessors: [ADR-0193 — the 0.32 typed-source goal](0193-0.32-typed-source-goal.md)
  (Accepted; the endgame (A) floor fixation), [ADR-0191 — the 0.31
  release gate](0191-0.31-release-gate.md), ADR-0186 §4.5 (the
  machine does not publish)
- The lekala: №713 (gh#1320 — the prep contour), №719 (gh#1331 — the
  release line of two windows, the §2.5 package)

## §1. Context — why the 0.34 gate needs its criteria NOW

Both release windows of 19.10 were consumed early (v0.33.0 03:39Z +
v0.33.1 08:33Z, the owner's live directives), so the 26.10 window's
train prepares without a fixed figure (0.33.2 tail-ride vs 0.34.0
feature-cut — the §2.5 package of №731). Independently of THAT
per-window call, the 0.34 LINE needs its release-gate criteria: the
currently-live gates are the accumulated ratchets (the №644
sensitive-surface bp floor 6844, the goal032 BLOCKING floor 23/67 —
ADR-0193 Accepted, the №467 typed-share floor, the №468 debt gate, the
№462 mirror/dup gates, the №562 release-gap gate) — they keep
guarding, but the 0.34 release has no FIXED FIGURE of its own. The
S-3 lesson (ADR-0193 §1) applies to GOALS, not to ratchets — a
release gate without a figure is honest-but-thin: it measures health,
it pushes nothing.

## §2. The question (the one this ADR puts to the owner)

> **What gates the 0.34.0 publication?** The owner picks the form in
> §6 (Parameter 1) and, if the form includes figures, their values
> (§6 Parameters 2–3).

## §3. The machine facts at the draft time (2026-10-20, main 2610039)

- The floors live and enforced=recorded: sensitive 6844 bp (the
  №644 ratchet; №725/gh#1348); goal032 closed 23/67 BLOCKING
  (ADR-0193; the gate wired №723/gh#1343); typed share 360/526 =
  68.44%; debt 8/0/15 (the №468 gate GREEN); mirrors 6, dups 0
  (the №462 gate GREEN).
- The precise share 237/526 = 45.05% — no movement since Волна 49.
- The movement №6 honest outcome (№730, gh#1359): the residue wall —
  all 44 open fixed-list rows carry standing refusal postures; every
  growth path runs through an owner/semantics lane (the Secret
  spelling — the dictionary lane; the №543 union forms — the
  semantics lane; the stub handlers — separate naryads; query* —
  post-№734 eligibility). The wall is the designed endgame frontier
  (ADR-0193 endgame (A)).
- The unfreeze summary (№482) stays the aggregate gate; the
  release_gap_gate (№562) measures the pending-Security gap
  (currently: nothing stuck).

## §4. The variants — price and risks, honestly

### (a) The ratchet floors only — the status-quo form

The 0.34 gate = the existing floors, only-up (the №644 bp, the
goal032 23/67, the №467 typed share, the №468/№462/№562 gates). No
new wiring, no new figure.

- **Price:** zero wiring; the gate reads GREEN-or-RED exactly as
  today; the release rides the current health.
- **Risks:** the honest-but-thin shape — no NEW criterion, the 0.34
  line records no goal of its own; the S-3 cousin risk is bounded
  (the ratchets cannot be gamed by the denominator, the ADR-0193
  form closed that hole) but the release cadence stops pushing the
  semantics lanes entirely.

### (b) The goal032 endgame (A) floor fixation as the release figure

The 0.34 release carries the FIXED-LIST closure figure as a
release-record criterion: the owner fixes N (the endgame (A)
parameters of ADR-0193 — the verdict pending), the 0.34 notes record
`closed N/67` as the line's goal statement.

- **Price:** the owner must fix N; the honest N today is 23 (the
  wall — №730): until the owner lanes open, the figure freezes at the
  wiring value (the ADR-0193 pattern — weak-but-honest).
- **Risks:** if the figure is wired BLOCKING, the release cadence
  couples to the types program — the trains could stall on the
  semantics lanes; the recommended posture (if picked) is the
  RECORD form (the figure rides the release notes, not the blocking
  gate) — the blocking wiring would be a separate owner's verdict.

### (c) The precise-share floor and/or the №569 coverage threshold

Two NEW ratchets: the precise share (237/526 = 45.05% — the honest
wiring value) as a bp floor; and/or the №569 coverage floor (the
threshold is the owner's parameter — the gate has waited for the
owner's verdict since Волна 49).

- **Price:** real pressure on the semantics lanes — the precise
  movement runs through the dictionary/Secret spellings and the
  union forms; the №569 threshold finally gets its value.
- **Risks:** without movement capacity the floors freeze at the
  wiring values (the frozen-floor form — honest but inert); the
  precise floor cannot honestly be raised by the executor (the
  same-PR rule + the residue wall) — the floor would be the owner's
  act entirely.

## §5. The recommendation (NO decision — the §6 package decides)

The executor's recommendation, for the record: **(a) as the base
form** (zero price, zero risk — the floors keep gating), **plus the
(b) RECORD figure once the owner fixes the endgame (A) N** (the
notes-record, not a blocking wire), **(c) parked** until the owner
lanes open (the Secret spelling and the union forms are the
prerequisite for ANY precise movement — a floor without the movement
capacity is the frozen-floor form). The recommendation is a POSITION,
not a verdict: every parameter in §6 belongs to the owner.

## §6. The verdict package (the owner picks; zero self-capture)

### §6.1 Parameter 1 — the 0.34 gate form (the owner picks one)

- (a) the ratchet floors only (the status-quo);
- (b) (a) + the endgame (A) figure as the release RECORD;
- (c) (a) + the precise-share floor and/or the №569 threshold (the
  blocking wiring);
- a combo (the owner spells it).

### §6.2 Parameter 2 — if (b): the endgame (A) N (the owner fixes)

The honest N today = 23 (the residue wall, №730). The owner may fix
N=23 (the floor fixation — the ADR-0193 endgame (A) posture) or a
higher N with the lanes-opening plan (the Secret spelling, the union
forms, the stub naryads).

### §6.3 Parameter 3 — if (c): the precise floor and the №569 threshold

The honest wiring values: the precise share 237/526 = 45.05%; the
№569 threshold — the owner's parameter (pending since Волна 49).

### §6.4 The wiring rule (whatever the picks)

The floors land in the SAME PR that raises them (the №757/№613 form);
the CI wiring rides the owner's verbatim (the №723/ADR-0193 §7
one-PR form); this ADR's draft wires nothing — the §6.4 wiring is
the POST-verbatim act, executed by the naryad that carries the
owner's verbatim (the №734/№723 landing form).

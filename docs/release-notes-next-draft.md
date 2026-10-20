<!-- GENERATED prep snapshot (№737, 2026-10-20 — the wave-52 refresh
     of the №731 prep draft; the [Unreleased] rows No734/No735 landed
     after the No731 cut; the movement No7 (No736) is the honest zero —
     no [Unreleased] row) — do NOT hand-edit.
     This file is the PREP-DAY snapshot for the 26.10 window train: the
     content is `python3 scripts/ci/release_notes.py Unreleased` — the
     [Unreleased] section of CHANGELOG.md verbatim (No497: the notes and
     the changelog can never disagree by construction).
     THE FIGURE IS UNDECIDED (the SS2.5 package of No737): 0.33.2
     tail-ride vs 0.34.0 feature-cut is the OWNER's call to the window
     (the dispatch gh#1374 gate 1). At the window: the [Unreleased]
     rows ride the cut per the owner's verdict, the section is re-cut
     in CHANGELOG.md under the decided version, and THIS DRAFT
     REGENERATES from the re-cut CHANGELOG
     (`python3 scripts/ci/release_notes.py <decided-version>`) before
     publishing — a stale draft is a lie (the No713 rule). The
     publication itself is the OWNER's act (SS6.5/ADR-0186 SS4.5). -->

### Added

- **№729 (gh#1356, gh#1350 Ask 2 — the owner's verbatim (a),
  2026-10-19):** the sanctioned read-back pair recognition — a
  destructive lookup key originating from the paired Source of the
  same domain (`cron_list` → `cron_remove`; `mtree_retrieve` →
  `mtree_forget`, the table `semantic::READBACK_PAIRS`) clears the
  sink-clearance gate at the key position: the paired sinks are
  match-only, the key never persists new data into the state — the
  data returns to the domain it came from, no egress channel by
  construction. Programs green on ≤ 0.33.0 that the gh#1336 coverage
  extension (0.33.1) refused via the `each`-body visibility compile
  again without the masking bridge. The conservative boundary
  (fail-closed №523/№531): the provenance chain must be
  paired-source-only (field reads and `to_string` preserve the domain;
  ANY foreign admixture loses the exemption); the exemption covers the
  key POSITION only (the payload positions of multi-arg sinks stay
  gated); a poisoned label clears nothing anywhere (ADR-0154 §2.1).
  `mtree_store` is NOT a read-back pair: the store's arg 0 is the
  payload text (auto-assigned id, admission-scored) with no key
  position — the #1350 issue's «mtree reads → mtree_store» conjecture
  corrected by the source.

### Changed

- **№723 (gh#1343, Wave 50 — the owner's verbatim «вербатим принят»,
  2026-10-19):** the goal032 fixed-list gate is wired BLOCKING in CI —
  ADR-0193 Accepted: the figure ≥20/67 ratified as-reached-late, the
  deadline closed as-reached, the ПДн-first record fixated, the endgame
  (A) floor fixation. The operational floor = the live counter at the
  wiring (23/67), only-up: a closed row losing its signature goes RED;
  the lift rides the closure PR (the №757/№613 form). The gate_031 §5
  one-PR form: the fact key (`gate_032_untyped_sources.txt`, FROZEN)
  WITH its checker. The §535 blocking surface grows by the owner's
  explicit verbatim — no product behavior changed.
- **№734 (gh#1364, Wave 51 tail — the owner's verbatim 2026-10-20,
  packages A(c)+B(i)):** `query()` on mutations returns the
  affected-row count as a String on BOTH backends (the №474 ONE
  contract — ADR-0194 packages A(c)+B(i), №734). AFFECTS: the VM-lane
  mutations change `[]` → `"n"` (the legacy divergence closed — the TW
  shape stands since Наряд №7); the examples and the office contour
  ride `db_execute` for mutations — zero affected consumers (the ADR
  §3 inventory). The error paths: the loud-fail CONTRACT pinned on
  both lanes (the cell/row errors name the column — practically
  unreachable; the ADR §2 probe record remains the carrier). The
  ratification record: ADR-0194 §8 (the verbatim + the delegation
  lekalo В49).
- **№735 (gh#1369, wave-51 tail — the owner's verbatim 2026-10-20,
  «вербатим ADR-0195»):** the 0.34 release-gate criteria are FIXED —
  ADR-0195 Accepted: the accumulated ratchet floors stay the base
  (the №644 bp, the goal032 23/67 BLOCKING, the №467/№468/№462/№562
  gates) plus the endgame (A) figure `closed 23/67` recorded as the
  0.34 line's release RECORD (the notes-record, not a blocking wire);
  the precise-share floor and the №569 coverage threshold are PARKED
  until the owner lanes open (the Secret spelling, the №543 union
  forms). Any only-up raise (a higher N, the (c) unparking) rides a
  follow-up owner verbatim. The ratification record: ADR-0195 §7;
  zero CI wiring (the RECORD form wires notes, not CI).

### Fixed

- **№724 (gh#1344, Wave 50 — the №722 tail):** the `markup5ever_rcdom`
  version requirement carries no semver metadata (`"0.39.0+unofficial"`
  → `"0.39.0"`) — the only build warning is gone. The resolver
  compatibility and the dependency graph are unchanged (the metadata is
  ignored by cargo); Cargo.lock is untouched (the package version stays
  `0.39.0+unofficial` — the real published tag). The №707 line hygiene.

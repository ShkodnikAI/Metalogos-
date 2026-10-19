<!-- GENERATED prep snapshot (№731, 2026-10-20) — do NOT hand-edit.
     This file is the PREP-DAY snapshot for the 26.10 window train: the
     content is `python3 scripts/ci/release_notes.py Unreleased` — the
     [Unreleased] section of CHANGELOG.md verbatim (№497: the notes and
     the changelog can never disagree by construction).
     THE FIGURE IS UNDECIDED (the §2.5 package of №731): 0.33.2
     tail-ride vs 0.34.0 feature-cut is the OWNER's call to the window.
     At the window: the [Unreleased] rows ride the cut per the owner's
     verdict, the section is re-cut in CHANGELOG.md under the decided
     version, and THIS DRAFT REGENERATES from the re-cut CHANGELOG
     (`python3 scripts/ci/release_notes.py <decided-version>`) before
     publishing — a stale draft is a lie (the №713 rule). The
     publication itself is the OWNER's act (§6.5/ADR-0186 §4.5). -->

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

### Fixed

- **№724 (gh#1344, Wave 50 — the №722 tail):** the `markup5ever_rcdom`
  version requirement carries no semver metadata (`"0.39.0+unofficial"`
  → `"0.39.0"`) — the only build warning is gone. The resolver
  compatibility and the dependency graph are unchanged (the metadata is
  ignored by cargo); Cargo.lock is untouched (the package version stays
  `0.39.0+unofficial` — the real published tag). The №707 line hygiene.

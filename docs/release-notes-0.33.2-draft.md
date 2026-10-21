<!-- GENERATED publication snapshot (№754, 2026-10-21 — the
     prep-to-publication cut AFTER the wave 56 (main 13995d2): the
     [Unreleased] rows (EIGHT — №729 Added; №748/№744/№740/№723/№734/
     №735 Changed; №724 Fixed) re-cut under [0.33.2] with the window
     stamp 2026-10-26 (the №708 form; an earlier publication re-dates
     the stamp first — the №721 form); version.workspace 0.33.1 →
     0.33.2 — the lockstep same-PR) — do NOT hand-edit.
     This file is the PUBLICATION NOTES for the v0.33.2 tag: the
     content is `python3 scripts/ci/release_notes.py 0.33.2` — the
     [0.33.2] section of CHANGELOG.md verbatim (№497: the notes and
     the changelog can never disagree by construction).
     The publication itself is the OWNER's act (§6.5/ADR-0186 §4.5):
     the tag v0.33.2 → release.yml → publish. The RECORD figure rides
     the notes from the CHANGELOG rows (ADR-0195 §7: goal032 closed
     35/67 — the N=23 floor exceeded). -->

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

- **№748 (gh#1392, Wave 55):** the 0.32 fixed-list movement №10 — the
  lane α continuation (the В55 dispatch gh#1394): the machine walk of
  ALL 38 open rows on live main with the №736 intercept-walk lesson
  applied to the WHOLE wall (the registry comment is the REGISTRY arm,
  not the whole ground) closes SIX rows by the №644 criterion from the
  LIVE both-lane handler facts: `reflex_list` → `List<String>` (the
  shared reflex_list_dispatch is the live both-lane path — the model
  names sorted by ReflexId.0; the №744 vision_list form), `event_count`
  → `Float` and `event_sum` → `Float` (the ADR-0052 intercepts are the
  live handlers — the №717 "planned wrapper, no handler" anchors were
  factually STALE; the №757 decrypt single-shape form), `events_since`
  → `List<Struct<Event>>` (the same intercept — the query rows form:
  the type_name carries the shape, no field-meta claimed),
  `deny_event` → `Struct<DenyEvent>{reason:internal,sink:internal,
  class:internal,argument:untrusted,label:internal,line:internal,
  human:untrusted}` (the №392 on_deny intercepts are the live handlers
  — the listen_start fixed-projection form; outside an on_deny body
  the usage stays a COMPILE error [DENY_HANDLER_SCOPE] — the error
  channel, not a shape), `deny_reason` → `String` (the reason field
  make_event always inserts — the single reachable arm). The other 32
  rows refuse as a wall — the classes re-verified with fresh anchors
  (inspect — the Struct|Unit state union, the (β) lane, the owner gate
  7; find — the data-dependent first match; reflex_generate — the
  candle-gated duality, no house precedent for feature-conditional
  typing; reflex_load — the bare Reflex opaque is not covered-grade;
  the stale anchor ranges refreshed comment-only: session_poll_wake/
  session_take_interrupt/duplex_open). goal032 29 → 35/67; the floors
  raised in the SAME PR (№757/№613): sensitive 7172 → 7586 bp, typed
  6958 → 7072 bp, precise 4543 → 4600 bp, parameterized 7580 → 7637 bp
  (97/127 — the denominator grows WITH the rows), field-label 58/60 →
  59/61. Battery: tests/naryad_748_wall_walk.rs.

- **№744 (gh#1385, Wave 54):** the 0.32 fixed-list movement №9 — the
  lane α continuation through the №740 UNION vocabulary (the В54
  dispatch gh#1388): the ПДн prima-facie group (session_poll_wake/
  session_take_interrupt — the №543-form state-dependent unions,
  memory_read/memory_open/secret — the Secret/opaque-spelling ground
  parked by ADR-0195 §7 (c), stdin — the true stub, request_body — the
  dynamic Struct fields, device_open — the opaque Device handle)
  refuses as a wall (each row READ, the classes in the naryad report),
  and the machine-chosen next package closes THREE rows by the №644
  criterion from the LIVE both-lane handler facts:
  `graph_path` → `List<Struct<PathError>|Struct<PathNode>>` (the union
  INNER — the №740 query_row form: the result is ALWAYS a List, the
  elements are the PathNode structs or the single PathError — the №669
  fictitious-List concern dissolves with both element shapes named),
  `vision_list` → `List<String>` (the №240 intercept is the live
  both-lane path — the `[Vision#N]` String handles; the row keeps its
  loud-Err fallback — the query №740 posture), `vision_lora_load` →
  `String` (the №244 intercept's single Ok arm — the adapter name; the
  №757 decrypt form). goal032 26 → 29/67; the floors raised in the
  SAME PR (№757/№613): sensitive 6965 → 7172 bp, typed 6901 → 6958 bp,
  precise 4302 → 4543 bp (the FIRST-match triple restored to the live
  value — the №536 form), parameterized 7540 → 7580 bp (94/124 — the
  denominator grows WITH the two new List rows). Battery:
  tests/naryad_744_lane_alpha_spellings.rs.

- **№740 (gh#1378, Wave 53):** the 0.32 fixed-list movement №8 — the
  query* package through the №740 UNION vocabulary (lane α, the
  growth path №736/§6.37 named): the stage-0 dictionary gains the
  union spelling `arm|arm|...` (≥2 stage-0-clean arms, strictly
  ascending — the canonical form; a malformed union fails closed to
  Unknown) and THREE registry rows close by the №644 criterion from
  the LIVE handler facts: `query` → `List<Struct<Row>>|String` (the
  SQL-class union rows|count — the №474 ONE contract, the fields
  dynamic by construction, the rows-path NOT extended),
  `query_scalar` → `Float|String|Unit` (the cell union),
  `query_row` → `List<Float|String|Unit>` (the heterogeneous List
  through the union inner — the №669 fictitious-X concern dissolves).
  The №539 false-warn guard satisfied by construction: the union
  names EVERY reachable arm. The context-less calls still refuse
  loudly (the t6 pin — the intercept rows stay handlerless through
  the new `intercept` spec! variant). goal032 23 → 26/67 (the db
  group 3/3 — the first ПДн-class group fully closed); the floors
  raised in the SAME PR (№757/№613): sensitive 6758 → 6965 bp,
  typed 6844 → 6901 bp, parameterized 6893 → 7540 bp, the goal032
  gate figure 26 in ci.yml. Battery: tests/naryad_740_union_spellings.rs.

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

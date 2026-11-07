# Changelog

All notable changes to the Metalogos project.

## [Unreleased]

- №930 (gh#1868, P2, [hygiene][ci][vision]): the vision golden re-pin (the Block 2 procedure) — the candle-core 0.11.0 → 0.11.1 bump (PR #1847, the owner's «Рекомендация принимается») shifted the tensor-kernel numerics (last-ULP), flipping the №211 golden embedding hashes/anchor bits; the A/B proof: the golden test green on the pre-bump `ae49ef5c`, red on the post-bump tree — the metalogos encoder math, the PRNG SSOT and the seed contract UNCHANGED, only the candle kernel numerics under them moved; re-pinned after 3 bit-identical local runs (P1 `fb43c9d6…`, P2 `ad01610e…`, P3 `a61e7e8b…`; the local got-value == the CI got-value — the cross-check); the full vision lane green locally (98/98 over the 8 test files); the parked-features nightly caught it (the №809 rule — the red nightly is recorded at the next release line).

- №929 (gh#1864, wave 87 — W-R, №R2, P2, [audit][docs][process]): the reader-surface drift sweep — `docs/audits/reference-drift-2026-11.md` (the №907 form): the methodology (machines, not prose — every verdict one reproducible command), the 15-surface roster (REFERENCE head RED→fixed by №928 / README badges / llms.txt / the ADR-index bijection + status sync / AGENTS↔CLAUDE №763 / spec-tools №645 / CHANGELOG — GREEN with numbers), the «шапка вне пина» lesson (6 weeks of silent head drift under a green 57/57 CI — the №928 pin closes the class), the pin verified on the wave merge commit `abedee052` (positive 7/7 + the negative control reproduced: ADR 209→208 → the named red); NO DRIFT (docs domain), no floors moved.

- №928 (gh#1863, wave 87 — W-R, P1, [docs][reference][testing][ci]): the REFERENCE.md head re-synced with the machine — `Version: 0.20.0` → `0.39.0` (Cargo.toml `[workspace.package]`), `Synced with code` → `2026-11-07 (naryad №928) · 530 builtins · 209 ADR files` (the unverifiable «149 accepted + 7 reserved» split removed, the №900 form), the §4 coverage note stamp `(v0.20)` → `(v0.39)`; the head is now PINNED by `tests/reference_consistency.rs` (three asserts: Version == Cargo.toml, builtins == named `spec!` count, ADR == `docs/adr/*.md` − README index) — the «шапка вне пина» drift class (6 weeks of silent drift under a green 57/57 CI, the R67 find) is closed.

## [0.39.0] - 2026-11-09

The amended calendar train (ADR-0189 §5 — the first amended cycle, the
owner directive on gh#1793): the candidate `v0.39.0-rc.1` cut from main
on Saturday 2026-11-07 (the lockstep, the №646 form), the office stand
verifies the rc on Sunday 2026-11-08, the final tag sits on the SAME
commit the stand verified (the rc↔final SHA equality is
machine-enforced) and the publication is the OWNER act (ADR-0186
§4.5). The section body = the [Unreleased] rows (the waves 76–86)
moved verbatim — the two `### Changed` groups of [Unreleased] merged
into one (rows verbatim, the internal order preserved). The
«Вошедшие подфазы» block replaces the cancelled R38 subphase→version
binding (the plan-canon §13.13): the version number classifies the
CONTENT (Added/Changed → minor), the subphase identity lives here.

Вошедшие подфазы: 8A-1/C1 (ADR-0206, wave 76); 8C/C3 (ADR-0207, wave
77); 8E-1/C5 — the ADR-0208 §2.5/§4 continuation (wave 78); 8D-2/C10
(ADR-0209, wave 79); 8G/C8 (ADR-0210, wave 80); 8F-2/C9 (ADR-0217,
wave 81); 8E-2/C7+C12 (ADR-0218, wave 82); 8H-1/C13б (ADR-0219, wave
83); 8H-2/C13а (ADR-0220, wave 84); the terminal stop of the Phase-8
train (ADR-0221, wave 85 — TERMINAL_STOP 13/13, gate_046); the ADR
status pass (wave 86 — ten records Proposed → Accepted per their
executed wave acceptances; the record content re-evaluated by nobody).

Проверено на стенде офиса: not verified — the Sunday stand
(2026-11-08) is pending at the rc cut; «not verified» = the train does
not depart (the release-checklist form). The verdict enters this block
by the post-stand/post-release sync (the №646 form, item 10) — the
tagged rc commit stays pinned.

### Added

- **№925 (issue #1852, wave 85 — W-T, P2, [audit][process]):**
  the terminal-stop readiness report —
  docs/audits/terminal-stop-readiness-2026-11.md (the №907/№913/№919/
  №922 form; the closing artifact of the W-T wave, strictly last):
  the methodology (every verdict regenerable by the machine —
  gen_metrics --check, the gate_033–gate_046 rosters — or a verbatim
  quote of a live source, no prose verdicts); the 13/13 slot-verdict
  table (every slot = the AND of its facts — C4/C13 two facts each,
  C7/C12 the shared REPRO_BITFORBIT — every verdict quoting the
  GENERATED numbers block); the external-audit scope outline (the
  funnel structure: the C1–C13 surfaces, the ADR chain 0201–0221, the
  machine facts gate_033–046, the readiness criteria — no vendors, no
  figures, no schedule: the partner track R38(9) is the owner's time,
  ADR-0201 §3.2); the evidence-bundle manifest with the digest pin
  (the G5/№828 form — sha256 over the manifest block, the re-hash
  refuses a drift); the repo↔owner-acts boundary (the audit
  procurement and the paying pilot — the owner's acts, R38(3)/R38(9);
  the pilot-evidence form: the owner's record + the carrier digest,
  the contract/invoice details never in the repo); the 1.0.0 release
  path (the calendar train ADR-0189 §5, the v0.38.3 precedent, the
  publication — the owner's act ADR-0186 §4.5); the cross-links (the
  legal shield В83 — №919, the verticals В84 — №922, the C7 trigger —
  №913 not ignited by the 1.0 gate); the honest limits (the gate
  counts SLOT STATES — not the audit force, not the pilot validity,
  not a certification); the sync rows in docs/limitations.md (the
  terminal-stop section) and docs/threat-model.md (the terminal-stop
  bullet — the fake-slot/lost-roster/dead-definition/floor-lowering
  classes each named); llms.txt — the naryad range bump.

- **№924 (issue #1851, wave 85 — W-T, P1, [core][testing][ci]):**
  the TERMINAL_STOP machine fact in gen_metrics.py + gate_046 — the
  13-slot aggregate over the live Phase-8 checks (ADR-0221 §2 — the
  №923 record's definition lands in code; the №735 form «факт вместе
  со своим чекистом»): `TERMINAL_STOP :=` the count (0–13) of GREEN
  capability slots, each slot answered by the REGISTERED live fact
  function (the identity pin — a substituted checker never counts, the
  fake-slot trap; the aggregate calls the same functions `--check`
  uses and never re-implements a fact), the slot GREEN ⇔ every machine
  fact of the slot holds its floor through its own LIVE strict check —
  the slot's own roster (gate_033–gate_045) strict-loaded (a
  dead/empty/corrupted roster is a refusal — the lost-roster class)
  and the roster floor holding the owner constant (only-up) and the
  live value holding the roster floor, plus the ADR-0221 definition
  literal `TERMINAL_STOP :=` standing in the record file (a dead
  definition → NOTHING is anchored → the aggregate reads 0 — the №923
  mutation contract); fail-closed everywhere (an exception, a dead
  anchor, a dead roster → the slot is NOT counted; an empty roster is
  a gate failure, never a pass; no synthesized green); the floor 13
  only-up (the wave-85 dispatch constant gh#1853, the TERMINAL_STOP_
  FLOOR_OWNER constant — the №776 discipline; the lowering is the loud
  FLOOR LOWERING REFUSED); gate_046 (scripts/ci/gate_046_goals.txt —
  the explicit roster: the floor line machine-read by
  load_floors_strict, the C1–C13 slot map as the human-readable twin);
  the tamper traps №55–№58 prove the dispatch classes (а)–(г) red: the
  fake-slot checker (the identity pin refuses a synthetic green), the
  lost gate_046 roster (the strict loader refuses the vacuum), the
  dead ADR-0221 definition (the aggregate zeroes — nothing is
  anchored), the floor lowering (the loud refusal); the
  blocking `gate-046` CI job (the double --check — the determinism
  contract — plus the tamper traps; the blocking_checks.tsv row, the
  fact_blocking_check_cells 35 → 36, the README badge 49 → 50); the
  test naryad_924_terminal_stop.rs pins the machine (the check green +
  the TERMINAL_STOP 13/13 verdict, the double --check byte-for-byte,
  the tamper-test green, the ADR-0221 quote-literal, the gate_046
  owner floor — the №535 row binds by name n924_terminal_stop_gate);
  the GENERATED rows in docs/metrics.md and docs/PLAN-SUMMARY.md (the
  №900 form «13/13»); the №900 denominators ride every fact row —
  the aggregate adds the slot count row (n/t).

- **№923 (issue #1850, wave 85 — W-T, P1, [process][adr][governance]):**
  ADR-0221 Terminal Stop 1.0 — the 1.0 assembly record (ADR before the
  code, the ADR-0201 §2.4 rule; the №891/№903/№916 form): the 1.0 gate
  definition — all 13 capability slots GREEN (14 machine facts, each
  through its own strict gate; a slot = AND over its facts: C4 =
  LICENSED_FLOOR ∧ PASSPORT_FRESH, C13 = INSTITUTIONAL_READY ∧
  VERTICAL_PROFILES, C7/C12 — the shared REPRO_BITFORBIT) + the
  external-audit evidence + the paying-pilot evidence (the plan §23
  verbatim; the R38 train exhausted — 0.47 = 8H-2 closed by wave 84,
  the owner's directive of 2026-11-06 = the start-of-assembly gate
  pass); the aggregate machine fact `TERMINAL_STOP :=` the count
  (0–13) of GREEN slots — defined BEFORE the code, built OVER the
  live strict checks (the №735 form — the fact together with its
  checker), fail-closed (a dead roster/composition/checker → the slot
  does not count; an empty roster is a gate failure, never a pass),
  the floor 13 only-up (the dispatch gh#1853 constant; the №776
  discipline); the digest-pinned evidence shapes (the G5/№828 form —
  the audit-evidence bundle manifest and the pilot-evidence record;
  the vendor/figures/schedule and the contract/invoice details stay
  outside the repo — ADR-0201 §3.2, R38(3)/R38(9)); the release path
  for 1.0.0 — the calendar train ADR-0189 §5 (Sat rc → Sun stand →
  Mon final tag on the same commit; the publication is the owner's
  act ADR-0186 §4.5; the empty-week rule stands); the honesty frame
  (the 1.0 gate ≠ a service-quality guarantee ≠ a certification ≠ a
  legal advice — the C10/В79 verbatim; the C7 trigger is not ignited
  by the 1.0 gate — R38(8)/№913). The mutation contract for №924: the
  test carries the quote-literal `TERMINAL_STOP :=` — deleting the
  literal from the ADR makes the №924 test red. The index row 0221 in
  docs/adr/README.md; llms.txt 208 → 209 ADR, the naryad range bump.

- **№922 (issue #1841, wave 84 — W-L2, P2, [docs][audit]):**
  the verticals-readiness report — docs/audits/verticals-readiness-2026-11.md
  (the №907/№913/№919 form; the closing artifact of the 8H-2 subphase,
  strictly last in the wave): the methodology (the gate_045 machine +
  the live kernel), the 12/12 roster-verdict table (every row = the
  CHECKED anchor pair in its live seed region), the two verticals
  audited from the sources (payments + public-sector — the six
  mandatory contours consumed by reference, the self-describing digest
  pins, the real-binary e2e verdicts), the profile ≠ certification ≠
  owner-acts boundary (the owner's acts named per R38(9): the
  PSP/acquiring integrations, the government contacts, the
  certification audits — each with its status «the owner's act, outside
  the repo» and what the repo carries instead; medicine — the backlog
  until the PHI expertise, R38(5)), the vertical distribution track as
  a REGISTRY STRUCTURE (the columns + the readiness criteria — no stub
  records: the contact registry is empty by design), the cross-links
  (the C7 trigger referenced but not ignited — R38(8)/№913; the
  institutional shield preceding the verticals — ADR-0219/№919; the
  Sigstore keyless release chain — ADR-0218; the package-path-only
  consumption — the honest §5(4) limit), the honest limits (what the
  vertical coverage does NOT close); the sync rows in
  docs/limitations.md (the vertical-contour section) and
  docs/threat-model.md (the known-boundaries bullet). The mutation
  boundary: the report is an ARTIFACT — deleting its boundary section
  does not red the machine (the fact lives in the kernel №921 and the
  machine gate_045), but the wave acceptance requires the report.

- **№921 (issue #1840, wave 84 — W-L2, P1, [core][registry][testing][ci]):**
  the vertical-profiles kernel in code — the 8H-2/C13а kernel + the
  VERTICAL_PROFILES fact machine (the 14th machine fact of the №841
  family): src/vertical.rs — VerticalProfile (the fail-closed
  constructor: a profile without the FULL six mandatory contours or
  with a drifted roster needle refuses at construction, the №879/№888
  form), the append-only registry (payments + public-sector per
  R38(5), the seed-region literals are the gate_045 roster needles),
  the carrier digest pin (the G5/№828 form — SHA-256 over the
  canonical serialization, self-describing) and vertical_verdict (the
  fail-closed consumption: an unknown id / a digest mismatch / an
  unverifiable contour refuses BEFORE any effect with the divergence
  NAMED — «vertical: unknown vertical id» / «vertical: vertical digest
  pin mismatch» / «vertical: mandatory contour not verifiable»; no
  default attach — a manifest without `[vertical]` verifies with ZERO
  behavioral mutations); the mlogpkg package path inherits №911 — the
  `[vertical]` manifest table (id + digest, the №910 [registry] shape)
  is consumed in build (after the integrity pass, before the advisory)
  and verify (after the signature chain); gate_045
  (scripts/ci/gate_045_goals.txt — the explicit 12-element roster: id,
  carrier, seed region, needle = the profile-scoped contour anchor,
  sub = the typed contour variant; the blocking `gate-045` CI job with
  the double --check + the tamper traps; the blocking_checks.tsv row —
  fact_blocking_check_cells 34 → 35 in gate_028/029/030, badge
  48 → 49); the tamper traps №51–№54 prove the dispatch classes
  (а)–(г) red: the removed seeding, the emptied mandatory_contours,
  the dead digest-pin consumption, the hand-edited GENERATED row (+
  the dead ADR-0220 definition — the №920 mutation contract); the
  tests tests/naryad_921_vertical_profiles.rs (the gate binding + the
  kernel fail-closed probes) and mlogpkg/tests/naryad_921_vertical_e2e.rs
  (the real-binary pack/sign/verify e2e: the live pin GREEN, the
  broken digest refused named, the unknown id refused named, the
  undeclared path zero-mutation). VERTICAL_PROFILES 12/12 — the 0.47
  gate is up; the machine verifies the PRESENCE and STRUCTURE of the
  profiles, not their certification force (ADR-0220 §5). The
  environment ride: the №868 compliance package re-attested in the
  same PR (the gate_028/029/030 rows re-pinned after the cells 34 → 35
  — the №863 form, the compiler replication proven byte-for-byte); the
  №12 httpbin tests made hermetic — a local 127.0.0.1 POST listener
  replaces the third-party endpoint (it rate-limits the datacenter
  runner IPs and flaked the blocking lane in the 02:11Z window; the
  builtin's request-construction / header-application / response-
  parsing path is exercised identically — no third party left).

- **№920 (issue #1839, wave 84 — W-L2, P1, [process][adr][governance]):**
  ADR-0220 Vertical Profiles — the 8H-2/C13а subphase record (ADR before
  the code, the ADR-0201 §2.4 rule; the №891/№916 form): the fact
  definition `VERTICAL_PROFILES :=` the count of CHECKED anchors of the
  12-element vertical roster (2 profiles — payments + public-sector per
  R38(5) — × 6 mandatory contour anchors: jurisdiction coverage,
  human-oversight/approval, decision-trace verify, taint floor,
  license class, the frozen №385 refusal vocabulary as the version-pin;
  medicine — backlog until the PHI expertise), every anchor CHECKED,
  not declared (the №862/№894/№918 form), the floor 12 only-up (the
  dispatch gh#1842 constant); fail-closed consumption — a package
  declaring a vertical in mlog.toml `[vertical]` (id + digest, the
  №910 [registry] shape) with an unknown id / a digest mismatch / an
  unverifiable contour refuses BEFORE any effect with the divergence
  named (the №874/№845/№911 form), no default attach — a manifest
  without `[vertical]` never gets a profile (zero behavioral
  mutations); the carrier digest pin (the G5/№828 form — SHA-256 over
  the canonical serialization); the C10 honesty frame (the profile is
  a configuration of limits and mandatory contours — NOT
  certification, NOT legal advice, NOT a regulator's acceptance; the
  C7 trigger is not ignited by a profile — R38(8)/№913); the run-path
  composition — the honest §5 limit, outside this wave.

- **№919 (issue #1833, wave 83 — W-L1, P2, [governance][docs][audit]):**
  the institutional-readiness report — docs/audits/institutional-readiness-2026-11.md
  (the №907/№913 form; the closing artifact of the 8H-1 subphase): the
  methodology (the gate_044 machine + the live sources), the
  12/12 roster-verdict table (every row = the CHECKED anchor pair of
  the live carrier), the three shield parts audited from the sources
  (the trademark policy, the succession protocol, the neutral-structure
  frame), the repo↔owner-acts boundary (the owner's acts named per
  R38(9): the trademark filing, the incorporation, the banking, the
  partner contacts — each with its status «the owner's act, outside
  the repo» and what the repo carries instead), the partner-contact
  REGISTRY STRUCTURE (the columns + the readiness criteria — no
  stub records: the contact registry is empty by design), the honest
  limits (what the succession coverage does NOT close — the legal
  force, the single-holder fact, the never-walked journal); the sync
  rows in docs/limitations.md (the institutional-contour section) and
  docs/threat-model.md (the known-boundaries bullet). The mutation
  boundary: the report is an ARTIFACT — deleting its boundary section
  does not red the machine (the fact lives in the carriers №917 and
  the machine №918), but the wave acceptance requires the report.

- **№918 (issue #1832, wave 83 — W-L1, P1, [testing][ci]):** the
  INSTITUTIONAL_READY fact machine in gen_metrics.py + gate_044 — the
  13th machine fact of the №841 family: the count of CHECKED items
  (0–12) over the explicit 12-item institutional roster
  (scripts/ci/gate_044_goals.txt, the №862/№894 form: id, carrier,
  needle = the section heading, sub = the mandatory content-bearing
  literal — a row counts only when BOTH stand in the named live file);
  the composition site is the ADR-0219 definition literal
  (`INSTITUTIONAL_READY :=` — the №916 mutation contract: a dead
  definition zeroes the count); the floor 12 only-up (the wave-83
  dispatch gh#1834 constant — a lowering below the owner constant is a
  loud refusal); the blocking `gate-044` CI job (the double --check —
  two consecutive runs must agree byte-for-byte, the determinism
  contract; + the tamper traps); the blocking_checks.tsv row
  (check|all|institutional-roster, bound to the
  naryad_918_institutional_gate test; fact_blocking_check_cells 33 → 34
  in the gate_028/029/030 records — the growth is explicit, the check
  landed in the same PR); the GENERATED rows in docs/metrics.md +
  docs/PLAN-SUMMARY.md (the №900 denominator form — «12/12»); the
  tamper traps №46–№50 prove the dispatch classes (а)–(г) red: the
  removed section, the emptied section, the broken cross-reference,
  the dead ADR definition, the lowered floor. INSTITUTIONAL_READY
  12/12 — the 0.46 gate is up; the machine verifies the PRESENCE and
  STRUCTURE of the policies, not their legal force (ADR-0219 §5).

- **№917 (issue #1831, wave 83 — W-L1, P1, [governance][docs]):** the
  institutional corpus in the tree — the three carriers of the
  ADR-0219 roster: TRADEMARK.md (the neutral-use policy of the
  name/logo — the scope, the permitted use, the forbidden use + the
  anti-endorsement rule, the permission-request channel on the
  repo's existing surfaces — a GitHub issue titled `trademark
  request:`; no invented addresses; the MIT/Apache-2.0 code license
  ≠ the right to the name), docs/SUCCESSION.md (the written authority
  for a bus-factor event — the activation triggers T1/T2/T3, the
  authority chain consuming maintainers.md №471 + CODEOWNERS BY
  REFERENCE, the recovery matrix incl. the Sigstore keyless release
  keys (ADR-0218), the quorum read from the LIVE holders (the lane
  №774 verified-wait reality, no fantasy council), the append-only
  activation journal — empty by design), and the GOVERNANCE.md
  Phase-2 neutral-structure frame (APPENDED after the existing
  content — the Phase-1 BDFL text byte-identical; the three
  criteria, the cross-consistency with TRADEMARK/SUCCESSION, the
  honest-limits owner-acts boundary). No stubs — every section a
  working policy; the language surface — 0 mutations.

- **№916 (issue #1830, wave 83 — W-L1, P1, [process][adr][governance]):**
  ADR-0219 — the institutional contour & the legal shield (the
  8H-1/C13б subphase record, ADR before the code — the №891 form): the
  fact is INSTITUTIONAL_READY := the CHECKED count (0–12) over the
  12-item institutional roster — TRADEMARK.md (the name/logo scope, the
  permitted use, the forbidden use + the anti-endorsement rule, the
  request channel), docs/SUCCESSION.md (the activation triggers, the
  authority chain → maintainers.md №471/CODEOWNERS by reference, the
  recovery matrix, the quorum, the append-only activation journal),
  the GOVERNANCE.md Phase-2 neutral-structure frame (the criteria, the
  cross-consistency, the owner-acts boundary); the floor 12 only-up
  (the dispatch gh#1834 constant); the three shield parts (the TM
  policy + the succession protocol + the neutral-structure FRAME — not
  a legal entity, not legal advice, the C10 verbatim) and the contour
  boundary (the trademark filing, the incorporation, the banking, the
  partner contacts — the owner's acts outside the repo, R38(9); the
  renaming №503 strictly last); the consumption contracts №917–№919;
  status Proposed — the wave-83 acceptance is the ACCEPTED path; the
  ADR index and llms.txt regenerated (207 ADRs, the counters
  regenerated).


### Changed

- **№926 (issue #1857, wave 86 — W-P, P1, [process][adr][docs]):**
  the ADR status pass — the separate status pass named by ADR-0221 §6
  (the scope widened by the R65-бис full-ADR sweep): TEN records flip
  Proposed → Accepted per their executed wave acceptances — ADR-0205
  (wave 75), 0206 (76), 0207 (77), 0209 (79), 0210 (80), 0217 (81),
  0218 (82), 0219 (83), 0220 (84), 0221 (85) — the dispatches
  gh#1699/gh#1714/gh#1729/gh#1764/gh#1779/gh#1803/gh#1817/gh#1834/
  gh#1842/gh#1853 closed with the acceptance comments; the wave-83
  verbatim: the ADR file status lines wait «до отдельной статусной
  проводки». The status lines only — the record bodies byte-identical;
  the docs/adr/README.md index rows follow; llms.txt — the naryad
  range №1-№926, the ADR count 209 unchanged. HONESTLY OUT OF SCOPE:
  ADR-0187 (owner_fixed: false) and ADR-0211 (the owner's variant
  choice, №901/gh#1795) stay Proposed — their acceptance gates are the
  owner's and have not fired. The pass fixes the executed acceptances
  — it re-evaluates nothing and adds no language surface.

- **№899 (issue #1793, P1, [process][release][adr]):** the calendar
  release train — the ADR-0189 amendment (§5): the rc candidate is cut
  on SATURDAY 12:00 MSK from main (the machine: the `unfreeze_gate.py
  --strict` read, the lockstep, the rc tag, the DRAFT prerelease with
  the assets), the office stand verifies on SUNDAY (the e2e battery +
  the `mlog check` baseline; a defect → the fix → `rc.N+1`), the final
  tag sits on the SAME commit on MONDAY and the owner publishes. The
  rules: an empty week departs nothing; a red Sunday skips the week
  (no Tuesday carry); the minor numbers run consecutively (the
  0.37-class skip is now a machine-caught impossibility —
  `scripts/ci/release_train_gate.py`, blocking in ci.yml and in the
  release build, the `--self-test` fixtures); the final tag must sit on
  the SAME commit as the last stand-verified rc candidate (the
  rc↔final SHA equality, enforced before any asset is built);
  security/P0 patch cuts stay legal on any day. The `-rc.N` tags are
  first-class in release.yml (the same assets/SBOM/attestations,
  `BUILD-INFO.txt` records the prerelease posture, the draft is
  created by the machine and published by the owner). The R38
  «subphase = minor version» rule is REPLACED (the plan-canon
  §13.13): the subphases are enumerated in the release notes of the
  train that carries them (the notes template «Вошедшие подфазы» +
  «Проверено на стенде офиса» in docs/release-checklist.md); the
  version bindings are stripped from PLAN-SUMMARY, the limitations
  rows and the wave audits (the subphase names stay).
- **№900 (issue #1794, P2, [metrics][docs]):** the denominator rides
  every bp/% fact — «10000 bp (1/1)» instead of a bare «100%» (the
  N3 decision): gen_metrics exposes the (numerator, denominator) pair
  for every machine fact (the eight bp facts + the jurisdiction
  matrix), the generated tables (docs/metrics.md, the PLAN-SUMMARY
  numbers block) print the fact WITH its denominator, and the
  DENOMINATOR GATE refuses to emit a bp fact without the (n/N) —
  the mutation test: strip a denominator from an emitter → red
  (verified: the PROVENANCE_SHARE strip → the gate names the fact and
  exits 1); the pair↔bp consistency machine
  (`--denominator-self-test`) re-derives every bp from its pair on the
  live tree — a pair edited apart from its bp function is a red
  drift. The floor mentions in the prose (the gate floor constants)
  are not facts and stay as they were.
- **№902 (issue #1796, P2, [process][hygiene]):** the process hygiene —
  the docs-naryad «strictly last» clause is MACHINE-ENFORCED retired:
  the wave-record generator (`gen_wave_record.py`) reds a dispatch
  carrying the retired clause (the C4 rule, gh#1473; the record belongs
  to the weekly generator run — the waves 81–82 repetition ends); the
  placeholder gate (`scripts/ci/changelog_placeholder_gate.py`,
  blocking in ci.yml) refuses the unfilled template-token family in
  CHANGELOG.md and the release-notes documents — the issue-number slot
  (the exact token the №897 row shipped with), the unrendered
  GitHub-expression form, the dispatch and naryad-number slots — the
  №897/0.38.2 shipped-placeholder lesson is now a machine-caught
  impossibility; the gh#1751 M-1 verdict: the
  four fast-path merges (10.31–11.02) closed with the root cause (the
  executor merged before the run registered), the containment (the
  `mergeable_state == clean` gate since 02.11 — the proof by
  measurement: PR #1750 50/50 success) and the mechanical elimination
  (the GitHub auto-merge button disabled on the repo — merges go
  through the executor's verified green-state path only); the 0.37
  skip recorded in docs/upgrade.md («v0.37 was never tagged — the
  criteria shipped inside v0.38.0»; the numbering gate rides №899).### Added
- **№901 (issue #1795, P1, [security][consent][adr]):** ADR-0211 — the
  verifiable mandate actor (ADR BEFORE the code, the owner's N4
  decision): the fact — `approval_issue(actor, target, ttl)` is a
  normal builtin whose `actor` is any non-empty String, so a program
  can issue a mandate naming any human and the fail-closed approval
  gate honors it (ADR-0205 §5's honest «not a signature» record; the
  APPROVAL_COVERAGE 10000 bp does not distinguish «a human approved»
  from «a program wrote a name»); the three variants compared over the
  trust anchor, the reachable surface, the runtime delta, the
  verified-mode path and the honest limits — (а) the Ed25519-signed
  mandate (the key in the host environment, the runtime verifies),
  (б) the out-of-program issuance (the host CLI `mlog approve …`/the
  secret endpoint; the program only reads), (в) the
  `actor_verified=false` marking that never counts; **the
  recommendation: (б) with (в)'s marking as the mechanic** — the trust
  anchor stays outside the machine (the ADR-0186 §4.5 lineage), the
  fail-closed direction is structural (the counted class is born
  outside the program's reach), the coverage becomes a checked fact
  about humans, not about strings; (а) stays the strengthening lane
  for the multi-host contour (the trigger, not a schedule). Status
  **Proposed** — the owner's variant choice closes the ADR as Accepted
  and opens the code naryad (the test mutation: `approval_issue` from
  `.mlog` does not pass the `cron_run` gate in the verified mode).### Added
- **№914 (issue #1816, wave 82 — W-K, P2, [process][ci], the
  №863/№889/№895/№908 form):** the baseline re-pin ride under the
  wave-82 signed-package kernel — the verification pass over ALL
  surface gates on the wave's merge chain (the head `408b2cc6`, re-run
  on the post-patch head `54f8c041` after the out-of-train №915):
  `type_signature_baseline.txt` (377/530, 71.13%),
  `type_signature_precise_baseline.txt` (244/530, 46.03%),
  `type_signature_parameterized_baseline.txt` (100/130, 76.92%),
  `--enforce-fieldmeta` (62/64, 96.87%),
  `type_signature_stable_baseline.txt` (366/486, 75.30% — the PRIMARY
  definition, ADR-0199), `sensitive_surface_baseline.txt` (112/147,
  7619 bp, the №644 fixed-denominator share), the src/ mirror gate
  (6, at its threshold 6 — movement only down) and the debt gate (dup
  0/0, ignore 8/8, ignore_todo 0, dead_code 15/15, example_uncovered
  0, read_file_soft_missing 0). The honest outcome — the №889/№895/№908
  precedent form: NO DRIFT. The wave-82 naryads (№909 the ADR-0218
  record, №910 the mlogpkg kernel in a SEPARATE crate — the trust
  domain №475, №911 the build integrity gate, №912 the gen_metrics
  machine + gate_043, №913 the C7 readiness report; the out-of-train
  patch №915 — the serve/mcp-serve per-module suppression) add ZERO
  `BUILTIN_REGISTRY` rows and zero grammar/golden surface — the
  type-signature and sensitive-surface counters read the REGISTRY, not
  the module tree, and the mlogpkg diff does not touch the counted
  surface — so every floor already equals the fact and no baseline
  file changed; the double `gen_metrics --check` run on both heads is
  green and deterministic (the №912 REPRO_BITFORBIT gate_043 floor
  10000 bp holds — the wave's own machine rides the same pass); no
  floor lowering — the floors stand, the ride's product is the
  verified record itself.

- **№913 (issue #1815, wave 82 — W-K, P2, [security][docs]):** the C7
  trigger-readiness report — docs/audits/repro-readiness-2026-11.md
  (the №907 form; the «отчёт аудита» artifact of the 0.45 line): the
  roster table with the machine verdicts (REPRO_BITFORBIT 10000 bp,
  11/11 live-wired — every number regenerable by gen_metrics), the
  release digest chain audited from the workflow source (the locked
  build, the BUILD-INFO toolchain record, the SBOM-for-exactly-that-
  binary binding, the SHA256SUMS attach, the Sigstore keyless
  attestations), the C7 trigger contract VERBATIM (R38(8): the first
  gov/bank pilot fires the bit-for-bit toolchain lane; Nix stays closed
  until then), the honest limits (ADR-0218 §5), the prior art (the
  kamerton cycle №618, the deterministic lockfile №198). The sync:
  docs/limitations.md — the Package Registry Integrity section (the
  pinned-chain model, NOT a PKI; the trigger-gated toolchain lane);
  docs/threat-model.md — the supply-chain boundary (what the signed
  registry catches / what remains). The external audit is NOT procured
  by this wave — the kamerton form stays the owner-forwarded channel.

- **№912 (issue #1814, wave 82 — W-K, P1, [testing][ci]):** the
  REPRO_BITFORBIT machine fact + gate_043 (ADR-0218 §2 — the machine
  builds the ADR's definition, never its own). The roster
  (scripts/ci/gate_043_goals.txt, the №862/№894 form): 11 registered
  reproduction carriers — the mlogpkg package path (the pack-twice byte
  identity, sign→verify happy, the digest-verify tamper/foreign-key
  refusals, the lockfile determinism, the pinned-happy build, the
  tamper-before-semantic-check order, the advisory-after-integrity
  order) plus the release digest chain (the SHA256SUMS attach, the
  CycloneDX SBOM, the Sigstore keyless attestations in release.yml).
  The count is CHECKED, not declared: a row counts ONLY on its needle
  literal in the named live source AND both composition sites
  (`fn verify_dependency_integrity` + `fn build_digest_document`, the
  №874 form) standing in mlogpkg/src/main.rs. The floor 10000 bp
  only-up (the wave-82 dispatch constant gh#1817; FLOOR LOWERING
  REFUSED / FLOOR BREACH in --check); the tamper traps 40–45 (the
  clean tree, the vacuous roster, the forged row, the row loss — the
  №894 Trap-37 honest covered-count observable, the dead composition,
  the floor lowering); the GENERATED rows in docs/metrics.md +
  docs/PLAN-SUMMARY.md; the double --check run deterministic.
  The honest scope (ADR-0218 §5): the registry package path + the
  release digest chain — NOT the bit-for-bit toolchain build (the C7
  trigger R38(8)).

- **№911 (issue #1813, wave 82 — W-K, P1, [registry][core]):** the build
  integrity gate (ADR-0218 §4.2) — the fail-closed coverage rule:
  `mlog.toml [dependencies]` grows the pinned form
  `{ version = "..", digest = "<64 hex>" }` (the №828 form); `mlogpkg
  build` runs the integrity pass BEFORE any effect — every dependency
  (transitively) must pin its digest, the registry tree must re-derive
  byte-for-byte, and a pinned `[registry] verify_key` demands a VERIFIED
  signature; the advisory consult (№198) rides strictly AFTER the
  integrity pass — a tampered package refuses as INTEGRITY, never as an
  advisory hit. BREAKING (the honest migration of the local-registry
  era): a bare version (`lib = "1.0.0"`) refuses at build with the
  missing-pin message naming the pinned form; `add` records the version
  only — pin before build. Refusals: digest pin missing / digest pin
  mismatch (pinned vs re-derived) / signature required / signature
  mismatch. The №198 suite runs pinned (the topological pack→pin order);
  the new naryad_911_integrity_gate.rs battery: bare-refusal, pinned
  happy, tamper-before-semantic-check, advisory-after-integrity,
  verify_key contract (no doc / unsigned / signed / foreign key),
  transitive pin + tamper-through-the-chain.

- **№910 (issue #1812, wave 82 — W-K, P1, [core][registry]):** the signed
  package kernel (ADR-0218 §2/§4.1) — `mlogpkg pack` (the canonical digest
  document: sorted keys, LF, no timestamps; re-pack of the same tree is
  byte-identical), `mlogpkg keygen` (32 bytes from /dev/urandom, hex,
  0o600), `mlogpkg sign --key` (HMAC-SHA256 over the 32 digest bytes; the
  signature trailer is excluded from the hashed line — a signature never
  signs itself), `mlogpkg verify --key` (fail-closed: the tree
  re-derivation is compared byte-for-byte, the HMAC is recomputed in
  constant time, and every divergence names itself — file missing, file
  digest divergence, manifest divergence, no signature, signature
  mismatch, stale document). Init/Add/Build/Info/Audit untouched (the
  №198 suite runs unmodified). Cargo.lock grows by ZERO entries (hmac/
  sha2/hex already in the workspace graph). Honest limits in the crate
  header (ADR-0218 §5): an integrity chain, not a PKI; /dev/urandom
  unix-only; the tar/toolchain lane stays behind the C7 trigger.

- **№909 (issue #1811, wave 82 — W-K, P1, [process][adr][registry]):**
  ADR-0218 Reproducible Build & Signed Package Registry — the 8E-2/C7+C12
  subphase record. The fact definition BEFORE code (the №891/№903 form):
  REPRO_BITFORBIT := the verified share of the explicit reproduction
  roster (gate_043) — the registry package path (the pack-twice byte
  identity, the lockfile determinism, the digest-verify tamper refusals)
  plus the release digest chain (the SHA256SUMS / SBOM-binding / Sigstore
  keyless wiring literals, CHECKED not declared — the №894 form). The
  signature covers the digest (the №828 form): the canonical digest
  document (sorted keys, LF, no timestamps), sign = HMAC-SHA256 over the
  32 digest bytes, verify = re-derive + byte-for-byte + constant-time.
  The contour boundary — the registry refusals are mlogpkg CLI errors
  (the №475 trust domain); the №385 vocabulary and the language surface
  do not move. The C7 trigger contract verbatim (R38(8)): the bit-for-bit
  toolchain lane (and Nix) opens ONLY at the first gov/bank pilot — the
  fact counts what IS verifiable today, honest-limits №588. ADR before
  the code (N1); llms.txt 204 → 205.

- **№908 (issue #1802, wave 81 — W-J, P2, [process][ci], the
  №863/№889/№895 form):** the baseline re-pin ride under the wave-81
  taint lane — the verification pass over ALL surface gates on the
  wave's merge chain (the head `340d928f`):
  `type_signature_baseline.txt` (377/530, 71.13%),
  `type_signature_precise_baseline.txt` (244/530, 46.03%),
  `type_signature_parameterized_baseline.txt` (100/130, 76.92%),
  `--enforce-fieldmeta` (62/64, 96.87%),
  `type_signature_stable_baseline.txt` (366/486, 75.30% — the PRIMARY
  definition, ADR-0199), `sensitive_surface_baseline.txt` (112/147,
  7619 bp, the №644 fixed-denominator share), the src/ mirror gate
  (6, at its threshold 6 — movement only down) and the debt gate (dup
  0/0, ignore 8/8, ignore_todo 0, dead_code 15/15, example_uncovered
  0, read_file_soft_missing 0). The honest outcome — the №889/№895
  precedent form: NO DRIFT. The wave-81 naryads (№904 the test-only
  taint lane in the differential fuzzer, №905 the triples ratchet,
  №906 the gen_metrics machine + gate_042, №907 the audit report +
  the limitations/threat-model sync) add ZERO `BUILTIN_REGISTRY` rows
  and zero grammar/golden surface — the type-signature and
  sensitive-surface counters read the REGISTRY, not the module tree —
  so every floor already equals the fact and no baseline file
  changed; the double `gen_metrics --check` run on the wave's head is
  green and deterministic; no floor lowering — the floors stand, the
  ride's product is the verified record itself.

- **№896 (issue #1778, wave 80 — W-I, P2, [docs], strictly last):** the
  weekly record of wave 80 — the generated §6.80 in
  docs/reality/2026-11.md (the №789 generator, the C4-a T1 form: the
  verbatim counters, the merge-CI verdicts — №891 `accef188`/PR #1780,
  №892 `ee719bf6`/PR #1781, №893 `5a580d80`/PR #1782, №894
  `38430258`/PR #1783, №895 `7d125d6f`/PR #1784; the №896
  self-reference row in the №877/№884 form); the honest-findings
  section carries the chain executor's HONEST-FINDING markers (the
  agent-index range-bump pattern confirmed again, the verify
  forged-claim catch, the Ledger-row args-hash race lesson, the
  composition-site generic-paren trap, the share-form trap
  re-derivation, the CHANGELOG-size metrics re-pin). The sanitized
  0.43 phase line in PLAN-SUMMARY (8G/C8, the dispatch gh#1779, the
  REPLAY_MATCH fact). The parked-features nightly RED on 0.38.0
  (93c18673) recorded honestly per the C3-a T3 rule — the release
  line: the parked-features nightly was RED on 0.38.0 (93c18673, run
  41415574569); the follow-up nights ran
  GREEN on `feed2030`, `2e4725a8`, `06ede970` — transient, tracked by
  gh#1737, closed dated at this wave's edition. llms.txt range
  №895 → №896.
- **№891 (issue #1773, wave 80 — W-I, P1, [process][adr][replay]):**
  ADR-0210 Deterministic Replay & Forensic Ledger — the decision trace
  as a typed, append-only, verifiable artifact; the frame R38(5):
  REPLAY_MATCH is the deterministic DECISION TRACE reconciliation, the
  bit-for-bit replay struck as unfulfillable (honest-limits №588); the
  trace lives IN the Ledger ACTION contour (ADR-0167) — never a
  parallel store; the versions read through the ProviderContract
  (№879); the refusal classes stay the frozen №385 set; the privacy
  boundary — digests, never raw prompts; ADR before the code.
- **№892 (issue #1774, wave 80 — W-I, P1, [core][replay]):** the
  DecisionTrace kernel (the №886 form) — the fail-closed constructor
  (a trace without input digests refuses; the Language carrier without
  the №879 version line refuses; raw payloads where a digest belongs
  refuse; foreign refusal words refuse — the №385 vocabulary does not
  grow), the canonical serialization + the G5 row digest (№828), the
  append-only DecisionTraceLog (the exact-duplicate refusal, supersede
  = a new row), the REPLAY_MATCH verify_replay (the claim-side and
  record-side digest re-derivation, the field-by-field divergence
  naming, the fail-closed no-recorded-trace refusal); `record_trace`
  rides the LIVE Ledger ACTION contour — the 1:1 migration, the
  zero-surface-drift rule.
- **№893 (issue #1775, wave 80 — W-I, P1, [replay][core]):** the
  coverage rule — the fail-closed decision-trace gate `replay_gate`
  (the fire_on_deny form of №874, the №845 rule): the irreversible
  decision without a live verify-passing trace refuses BEFORE the
  effect (SINK_CLEARANCE — the frozen class); every refused gate
  journals ONE Ledger deny row through the EXISTING action path; the
  happy path journals nothing; the only Ok exit IS the verified
  recorded row; the verify-side rule — the outputless claim and the
  outputless recorded row refuse at verify; the e2e on both execution
  roads byte-in-byte (the №880 form).
- **№894 (issue #1776, wave 80 — W-I, P1, [testing][ci]):** the
  REPLAY_MATCH machine fact + gate_041 — the explicit decision-carrier
  registry (scripts/ci/gate_041_goals.txt, the №862 roster form; the
  count CHECKED, not declared — the quoted decision literal in the
  live e2e wiring source + the alive `replay_gate` composition site,
  the №874 form); the seeds `run_program@tw` + `run_program@vm` (both
  live-wired, 10000 bp); the GENERATED rows in docs/metrics.md +
  docs/PLAN-SUMMARY.md; the floor enforcement in --check (FLOOR
  LOWERING REFUSED / FLOOR BREACH); the tamper traps 34–39; the double
  --check run deterministic.
- **№895 (issue #1777, wave 80 — W-I, P2, ride ygiene][ci], the
  №863/№889 form):** the baseline re-pin ride under the wave-80 replay
  kernel — the verification pass over ALL five surface gates on the
  wave's merge chain (the base `38430258`):
  `type_signature_baseline.txt` (377/530, 71.13%),
  `type_signature_precise_baseline.txt` (244/530, 46.03%),
  `type_signature_stable_baseline.txt` (366/486, 75.30% — the PRIMARY
  definition, ADR-0199), `--enforce-fieldmeta` (62/64, 96.87%) and
  `sensitive_surface_baseline.txt` (112/147, 7619 bp, the №644
  fixed-denominator share). The honest outcome — the №889/№882/№876
  precedent form: NO DRIFT. The wave-80 naryads (№891 the ADR-0210
  record, №892 the `src/decision_trace.rs` kernel, №893 the coverage
  gate + the e2e, №894 the metrics machine + gate_041) add ZERO
  `BUILTIN_REGISTRY` rows (the decision-trace kernel lives in
  `src/decision_trace.rs` + the python metrics machine + the e2e
  suites — the typed-signature and sensitive-surface counters read the
  REGISTRY, not the module tree), so every floor already equals the
  fact and the advisory `::warning` is absent from main; the double
  `gen_metrics --check` run on the wave's tree is green and
  deterministic. No baseline file changes in this ride — the floors
  stand; the ride's product is the verified record itself (the gates
  re-run green on the wave's tree, cited in the PR).
- **№890 (issue #1763, wave 79 — W-H, P2, [docs], strictly last):** the
  weekly record of wave 79 — the generated §6.79 in
  docs/reality/2026-11.md (the №789 generator, the C4-a T1 form: the
  verbatim counters, the merge-CI verdicts — №885 `9568a39e`/PR #1765,
  №886 `8a842e76`/PR #1768, №887 `206e785b`/PR #1769, №888
  `5f411679`/PR #1770, №889 `d9b87957`/PR #1771 — all GREEN on the
  merge commits; the №890 self-reference row in the №877/№884 form);
  the honest-findings section carries the three HONEST-FINDING markers
  from the chain executor's acceptance comments (the agent-index
  range-bump lesson, the clippy/fmt first-round catches, the №868
  re-attest on the pinned-file extension). The REALITY stub follows
  (2026-11: 4 sections, total 63). The sanitized 0.42 phase line in
  PLAN-SUMMARY (8D-2/C10, the dispatch gh#1764, the JURISDICTION_MATRIX
  fact). llms.txt range №889 → №890.
- **№889 (issue #1762, wave 79 — W-H, P2, ride [hygiene][ci], the
  №863/№882 form):** the baseline re-pin ride under the wave-79
  jurisdiction kernel — the verification pass over ALL five surface
  gates on the wave's merge chain: `type_signature_baseline.txt`
  (377/530, 71.13%), `type_signature_precise_baseline.txt` (244/530,
  46.03%), `type_signature_stable_baseline.txt` (366/486, 75.30% — the
  PRIMARY definition, ADR-0199), `--enforce-fieldmeta` (62/64, 96.87%)
  and `sensitive_surface_baseline.txt` (112/147, 7619 bp, the №644
  fixed-denominator share). The honest outcome — the №882/№876
  precedent form: NO DRIFT. The wave-79 naryads (№886/№887/№888) add
  ZERO `BUILTIN_REGISTRY` rows (the jurisdiction kernel lives in
  `src/jurisdiction.rs` + the server-side composition in
  `metalogos-server/src/mcp_policy.rs` + the python metrics machine —
  the typed-signature and sensitive-surface counters read the REGISTRY,
  not the module tree), so every floor already equals the fact and the
  advisory `::warning` is absent from main. No baseline file changes in
  this ride — the floors stand; the ride's product is the verified
  record itself (the gates re-run green on the wave's tree, cited in
  the PR).
- **№888 (issue #1761, wave 79 — W-H, P2, [testing][ci]):** the
  JURISDICTION_MATRIX fact machine + gate_040 — the explicit registry
  (scripts/ci/gate_040_goals.txt: the goal_jurisdiction_matrix 3 floor
  + the EU/RU/US-CA rows, the №862 roster form); a row counts ONLY on
  the code's PRESENCE in the live seed_matrix constructor (the count is
  checked, not declared; a forged row is uncovered; an empty registry
  is 0 — a vacuous class is a failure, never a pass); the floor twin
  ≥3 only-up (the wave-79 dispatch constant); the GENERATED rows in
  PLAN-SUMMARY + docs/metrics.md (the №881 form); the tamper traps
  29–33 (the clean-tree assertion, the vacuous registry, the forged
  row, the row loss, the floor lowering); load_floors_strict skips the
  jurisdiction roster rows (the two-class data file, the №881 form).
- **№887 (issue #1760, wave 79 — W-H, P1):** the coverage rule — the
  fail-closed jurisdiction gate at the consumption site:
  jurisdiction_verdict refuses an uncovered carrier («молчаливый
  пропуск невозможен», the №845 form; the KPI C10 on the consumed
  path); the MCP composition (ADR-0196 × ADR-0209) — jurisdiction_gate
  rides the compiled ToolPolicy's audited sinks BEFORE any run (a deny
  refuses, a limit rides as the mark the consumer honors); the e2e
  trace jurisdiction → rule (norm+date) → compilation → ToolPolicy
  decision → carrier verdict on BOTH backends (TW + VM byte-identical,
  the №880/№829 form); the gate report double-run byte-identity; the
  refusals reuse the kernel's jurisdiction classes (the frozen №385
  vocabulary not extended). The №868 re-attest ride included (the
  mcp_policy extension re-pins the gateway-policy evidence row).
- **№886 (issue #1759, wave 79 — W-H, P1):** the jurisdiction kernel
  in code — JurisdictionId (the validated code), JurisdictionRule
  (norm_ref + checked_at pinned by the canonical-row digest, the
  №828/№868 form) + the append-only JurisdictionMatrix
  (issue/query/digest; the exact duplicate refuses; the supersede is a
  new row, strictest-wins) + the fail-closed compile site (an unknown
  jurisdiction refuses; a deserialized incomplete rule refuses — the
  KPI C10 is a machine check on the compiled path) + the seed matrix
  ≥3 jurisdictions (EU/RU/US-CA — real norm pointers: GDPR Art. 17,
  152-ФЗ ст. 21, Cal. Civ. Code §1798.105 (CPRA), GDPR Art. 5(1)(c);
  the code data, never attached to a runtime by default — the ADR-0209
  frame: a configuration of limits, not legal advice); the 1:1
  migration — no grammar, no builtin-registry, no golden change.
- **№885 (issue #1758, wave 79 — W-H, P1, ADR before the code):**
  ADR-0209 Jurisdiction-Aware Policy Compilation — the jurisdiction
  limit-configuration as a typed, verifiable artifact (the frame: a
  configuration of limits, NOT legal advice — the contents of the
  norms are never interpreted, the rules carry references only); the
  package rule = JurisdictionRule {jurisdiction, carrier, effect,
  norm_ref, checked_at, source}; the KPI C10 (100% of the package
  rules carry a norm reference and a verification date) as a
  fail-closed compile-site check; the parents consumed by reference
  (feature-gates №676, mcp_policy ADR-0196, capability ADR-0203,
  passports ADR-0202, attestations ADR-0206, egress ADR-0207 — never
  duplicated); the refusal vocabulary stays frozen (№385). The
  agent-index lesson: every chain naryad carries its own llms.txt
  range bump.
- **№884 (issue #1744, wave 78 — W-G, P2, [docs], strictly last):** the
  weekly record of wave 78 — the generated §6.78 in
  docs/reality/2026-11.md (the №789 generator, the C4-a T1 form: the
  verbatim counters, the merge-CI verdicts — №878 `3af43db2`/PR #1750,
  №879 `a1f76ded`/PR #1747, №880 `3bc702c0`/PR #1748, №881
  `c3492d3e`/PR #1749, №882 `e17c02df`/PR #1754, №883 `b2eccbd8`/PR
  #1755 — all GREEN on the merge commits; the №884 self-reference row
  in the №877 form); the honest-findings section carries the seven
  HONEST-FINDING markers from the chain executor's acceptance comments
  (the parallel-iteration supersession, the terminal `[m` display
  artifact, the GLOBAL_SMART_ROUTER persistence, the never-CI'd stack
  liability, the goals-file two-class loader, the no-drift projection
  lesson, the dependabot blind-spot class). The REALITY stub follows
  (2026-11: 3 sections, total 62). The sanitized 0.41 phase line in
  PLAN-SUMMARY (8E-1/C5, the dispatch gh#1745, the OFFLINE_SUITE
  fact). llms.txt range №883 → №884.
- **№882 (issue #1742, wave 78 — W-G, P2, ride [hygiene][ci], the №863
  form):** the baseline re-pin ride under the wave-78 llm-contract types
  — the verification pass over ALL five surface gates on the wave's
  merge chain: `type_signature_baseline.txt` (377/530, 71.13%),
  `type_signature_precise_baseline.txt` (244/530, 46.03%),
  `type_signature_stable_baseline.txt` (366/486, 75.30% — the PRIMARY
  definition, ADR-0199), `--enforce-fieldmeta` (62/64, 96.87%) and
  `sensitive_surface_baseline.txt` (112/147, 7619 bp, the №644
  fixed-denominator share). The honest outcome — the №876 precedent
  form: NO DRIFT. The wave-78 naryads (№879/№880/№881) add ZERO
  `BUILTIN_REGISTRY` rows (the llm contract lives in
  `src/provider_contract.rs`/`src/llm.rs` — the typed-signature and
  sensitive-surface counters read the REGISTRY, not the module tree),
  so every floor already equals the fact and the advisory `::warning`
  is absent from main. No baseline file changes in this ride — the
  floors stand; the ride's product is the verified record itself (the
  gates re-run green on the wave's tree, cited in the PR).
- **№881 (issue #1741, wave 78 — W-G, P1, [testing][ci], ADR-0208 §2.5):**
  the OFFLINE_SUITE machine fact + gate_039 (the 0.41 gate): the share,
  in basis points, of the REGISTERED LLM-surface checks that execute
  WITHOUT the external network — the explicit registry
  `scripts/ci/gate_039_goals.txt` (12 checks, the offline=yes/no field
  per row: the local provider of №879, the local rung of №880, the
  deterministic mock fault seam №385, the loopback stub servers of
  №819/n812 — every row offline=yes, the №401 test hygiene), counted by
  `offline_suite_bp()` in `gen_metrics.py` (the count is CHECKED, not
  declared: a row counts ONLY on offline=yes AND the check file's
  EXISTENCE — tests/<name>.rs or metalogos-server/tests/<name>.rs; a
  forged row is uncovered, an empty registry reads 0 bp). The
  GENERATED rows land in docs/metrics.md + the PLAN-SUMMARY numbers
  block (the №875 form); the gate_039 floor is the wave-78 dispatch
  constant — 8000 bp, only-up (the owner-pinned lowering refusal); the
  tamper traps 25–28 (the field flip, the forged row, the
  vacuous-registry control, the floor lowering — the gate_036/037/038
  лекало); the `load_floors_strict` loader skips the registry rows
  (roster data, never floors — the strictness for goal_ lines
  unchanged). The wiring rides the EXISTING docs-metrics job (the cell
  does not grow — №535): `gen_metrics.py --check` + `--tamper-test`.
  The double `--check` run is deterministic.
- **№880 (issue #1740, wave 78 — W-G, P1, [llm][core], ADR-0208 §4):** the
  coverage rule — the fail-closed gate on provider dependence: an LLM
  call addressed to an external provider must END in a covered local
  fallback path (a `local` rung in the failover roster —
  `SmartRouter::has_covered_local_fallback()`: a local rung OR the
  empty roster, the №879 local-first default IS coverage) or in the
  honest refusal (the EXISTING exhaustion class — the loud "All LLM
  providers failed" / `LLM_PROVIDER_UNAVAILABLE`, the frozen №385
  vocabulary: no new deny codes); never a silent degradation. The
  `local` rung dispatches inside `call_provider` BEFORE any HTTP client
  is built (the frozen deterministic marker — no network, no key); the
  stream path refuses the local rung LOUDLY (`STREAM_UNSUPPORTED` —
  the local provider answers in one piece via `call()`); the
  pre-network refusal half — a keyless `anthropic` rung refuses BEFORE
  any HTTP wait (the RealLlm early-key-check semantics, lifted to the
  router dispatch on BOTH the call and the stream loops; the keyless
  OpenAI-compatible rungs keep the proxy use case untouched). The
  e2e trace green on BOTH backends (TW + VM, the №825 two-lane form):
  request → the deterministic external refusal (the unreachable
  loopback stub) → the failover chain → the local fallback → the
  result; the double run byte-identical. The №880 refinement: the
  contract view's `deterministic_answer` reads the ALL-LOCAL roster
  honestly. security: no — the egress contour of №874/ADR-0207 is NOT
  duplicated (the rule governs the LLM fallback paths, not the network
  boundary); the media domain untouched. Verified by
  `tests/naryad_880_failover_coverage.rs`.
- **№879 (issue #1739, wave 78 — W-G, P1, [core][llm], ADR-0208):** the
  provider-agnostic contract in code — `src/provider_contract.rs`:
  `ProviderContract` (capabilities + limits + health semantics — the
  NAMED VIEW over the `Provider`/`RealLlm`/`MockLlm`/`ProviderHealth`/
  `SmartRouter` parents, the dedup guard keeps it apart from the
  `Provider` roster enum), the closed-roster lookup
  `contract_for_name` (an unaddressable name refuses LOUDLY through the
  EXISTING `LLM_PROVIDER_UNAVAILABLE` class — №385 frozen), the
  deterministic canonical `report()` (the double-run is byte-identical
  by construction), and the local-first composition predicate
  `external_llm_declared()`. The negative controls:
  `tests/naryad_879_provider_contract.rs` (the local-first default on
  BOTH lanes — TW and the VM — over the learnable and `call_llm`
  seams; the empty `llm { }` block at the router's empty-chain seam;
  the declared-external fail-loud lane; the report determinism; the
  roster refusal) and the module unit tests (the config view honors
  the block's `circuit_breaker`/`timeout`/`max_tokens`/`failover`
  fields). The №454/№478 pins re-pinned to the ADR-0208 contract
  (see Changed) — the №454 SECURITY essence is preserved and stronger:
  the prompt material NEVER reaches the caller in ANY mode. security:
  no — the media/perception lanes ride the untouched
  `mock_llm_requested()` predicate; the egress contour of №874 is not
  duplicated (the rule layer is №880).
- **№865 (issue #1707, P0 — the migration honesty 0.35–0.38, out-of-wave
  on the owner's directive 2026-11-01):** `docs/upgrade.md` — the
  0.34 → 0.38 migration recipe (the inventory, the minimal
  `allow_carrier` path, the strict grant path, the `cron_run` mandate
  example, the exec host permit, the 0.35 model-passport check), the
  repeat-gate `scripts/ci/breaking_change_gate.py` (the blocking
  `breaking-change-gate` CI job): a tag range that adds a deny-path
  name to `GRANT_ALWAYS` / `PROFILE_PERMIT_SHARP` / `HUMAN_OVERSIGHT`
  without a `### Changed` BREAKING row naming it in the pending
  CHANGELOG section is RED (the mutation self-test pins the red), and
  `tests/naryad_865_migration_path.rs` — the 0.34.1-shaped program
  (http_post + send_message + cron_run) run unmigrated (the named
  refusal) and migrated (both gates pass) on BOTH backends. The README
  Quick Start and [docs/limitations.md](docs/limitations.md) carry the
  one-line pointer. security: no — docs/process (the capability and
  approval rule code is not touched; the refusals already named the
  carrier and the resolution paths — verified, cited in the PR).

- **BREAKING — №898 (issue #1792, P1, [core][llm][process]): `call_llm`
  without any LLM configuration now REFUSES at the first call (the
  silent mock default is rolled back).** The `create_llm_backend`
  composition no longer answers with the local mock marker when
  NOTHING is declared (the №879 local-first default is rolled back —
  the owner decision 04.11): a program calling `call_llm` with no
  `METALOGOS_LLM_PROVIDER` / `METALOGOS_API_KEY` /
  `METALOGOS_OPENAI_BASE_URL` and no `llm {}` block gets a loud
  refusal naming every configuration surface — a value a program
  builds on is never a mock marker mistaken for a model answer (the
  №454 lesson). **What to check on upgrade:** every program that calls
  `call_llm` must now declare ONE of — `METALOGOS_LLM_PROVIDER` (+
  `METALOGOS_API_KEY` for anthropic/openai), `METALOGOS_OPENAI_BASE_URL`,
  an `llm { providers: [...] }` block, or the EXPLICIT offline mock:
  `METALOGOS_MOCK_LLM=1` (test fixtures) / the in-program explicit
  local rung `llm { providers: [{alias: mock, provider: local}] }`.
  `mlog serve` prints the `[serve] LLM UNCONFIGURED` line at startup
  and the built-in `/health` carries the `x-llm-mode` header (mock /
  declared / unconfigured). A DECLARED provider without its key keeps
  the №454 loud failure — no change on the declared lane.

- **№883 (issue #1743, wave 78 — W-G, P2, ride [deps][ci]):** rusqlite
  0.40.2 → 0.41.0 (the dependabot PR #1658, the owner's verdict of
  2026-11-02 «Рекомендация принимается» — the bump rides the wave's
  merge chain with the full CI: libsqlite3-sys 0.38.2 → 0.39.1 rides
  the same lock entry; the msrv floor 1.93.1 and the docker build
  re-verified by the blocking jobs). The rustyline blocker (#1544)
  re-checked on the execution date 2026-11-03: crates.io still lists
  ONLY 19.0.0 (published 2026-10-17, not yanked, no stable-msrv
  follow-up) — the nightly-only `cfg_select!` build stands, the pin
  stays at 18.0.1, and #1544 remains open with the dated check
  comment (the honest re-date form). The 0.40→0.41 migration honesty:
  `Statement::column_names()` now returns an iterator (was
  `Vec<&str>`) — the THREE internal call sites in `src/db_ops.rs`
  (db_ops.rs:279/623/841) adapted (`.iter()` dropped; zero observable
  surface change, the language/registry rows untouched); no BREAKING
  row — the deny-path rosters are untouched (the №865 breaking-change
  gate stays green) and the language surface is byte-identical.
- **№879 (ADR-0208 §2.2/§5 — the ONE intentional posture change):**
  `create_llm_backend()` is LOCAL-FIRST: with NOTHING declared (no
  `METALOGOS_MOCK_LLM`, no `METALOGOS_LLM_PROVIDER`, no
  `METALOGOS_API_KEY`, no `METALOGOS_OPENAI_BASE_URL`) the composition
  resolves to the LOCAL provider — `MockLlm`, a full-right member of
  the implementation roster: the deterministic, non-echo, offline
  `[mock-llm:<hex>]` marker (№454/№487), VISIBLE as a local answer.
  Every DECLARED configuration keeps its exact pre-№879 semantics: the
  explicit mock flag (the `[MOCK: …]` lane of `call_llm`), the env
  provider/key/base-url (the REAL path — fail-loud without keys, the
  №454 posture preserved ON THE DECLARED LANE), the `llm {}` block
  (the SmartRouter). The re-pinned pins:
  `tests/naryad_478_mock_unification.rs` (both surfaces — `call_llm`
  and office-human) and
  `metalogos-server/tests/mock_llm_not_default_in_serve.rs` (the serve
  repro + the VM flow parity; the declared-external cases added).
- **№865 (retroactive):** the [0.36.0] and [0.38.0] sections carry
  their `### Changed` BREAKING rows (the №845 sharp set, the №861
  `cron_run` mandate) — the deny rules themselves are NOT changed and
  the Added rows stay verbatim; each BREAKING row cross-references its
  Added row (the closed is not deleted silently, it is read together).

### Fixed

- **№865:** the [0.34.0] date-stamp re-cut to the actual publication
  date 2026-10-28 (the форма №721 re-cut that never ran; the
  planned-window 2026-11-02 stamp was a lie the release notes carried).

## [0.38.3] - 2026-11-05

The unscheduled patch cut (the 0.28.1/0.30.1/0.38.1/0.38.2 shape —
ADR-0189: an unscheduled cut is a PERMITTED exception when a defect
cannot wait for the next Monday window, with the reason recorded in
the release notes). The reason: the №897 fix stopped at the RUN gate —
the №523 serve gate (metalogos-server/src/server.rs:811) still
filtered against the ENTRY source text only, and the №557 mcp-serve
gate (metalogos-server/src/mcp_server.rs `enforce_semantic_startup`)
held the UNFILTERED refusal surface (the №802-era "no source threading
through the MCP entrypoints" note): a program clean under
`mlog check` — and running under `mlog run` since 0.38.2 — refused at
`mlog serve`/mcp-serve startup when the allow-comment sat in an
IMPORTED module (the same consumer evidence as №897: fosved-acc
carries 22 sanctioned allow-comments in 10 imported modules; on
v0.38.2 the serve-path polygons stay down). Fixed-only: the
[Unreleased] wave rows stay pending for the next minor train — the
patch cut does not carry them (the 0.30.1 form). The publication is
the OWNER act (ADR-0186 §4.5); release.yml attaches the binary, the
SBOM, the attestations and SHA256SUMS on publish.

### Fixed

- **№915 (issue #1819 — [process][bugfix]):** the №897 per-module
  suppression surface completes the check ↔ run ↔ serve triangle —
  every startup gate now applies the same
  `filter_suppressed_unchecked_per_module` over the same
  `resolve_import_groups` landing: the №523 serve gate
  (metalogos-server/src/server.rs) swaps the entry-only filter for the
  groups form, and the №557 mcp-serve gate
  (metalogos-server/src/mcp_server.rs `enforce_semantic_startup`) takes
  the entry source threaded through all three entrypoints (the stdio
  loop, the http/sse network transport, the test harness — the bin
  caller reads the file before parse, so the threading is a parameter,
  not an I/O change) and applies the same per-module filter — the
  №802-era fail-closed asymmetry note ("no source threading through
  the MCP entrypoints in this naryad") is CLOSED. The filter itself is
  unchanged (the per-line quota, the non-empty-reason rule); one
  module's allow-comment cannot launder another module's access (the
  №802 laundering rule, now pinned on the serve path too); no-import
  programs behave byte-identically to 0.38.2 (the empty-group case
  degenerates to the old entry-only filter). Pinned by the serve/mcp
  regression battery
  (`metalogos-server/tests/naryad_915_serve_gate_suppression.rs`,
  6 tests) with the RED-before run on main 8bcee429: the three
  positive arms refused with "semantic findings block serve
  startup"/"block mcp-serve startup", the negative arms and the
  laundering pin held. The process honesty ride: the №897 row in
  [0.38.2] shipped with the issue-number placeholder left unresolved —
  pinned retroactively to gh#1789 (the index must not carry lies; the
  №902 gate now refuses the class).

## [0.38.2] - 2026-11-04

The unscheduled patch cut (the 0.28.1/0.30.1/0.38.1 shape — ADR-0189:
an unscheduled cut is a PERMITTED exception when a defect cannot wait
for the next Monday window, with the reason recorded in the release
notes). The reason: the №523 run/serve gate disagrees with the №802
check surface on imported modules — a program clean under `mlog check`
refuses at `mlog run` when the allow-comment sits in an IMPORTED
module (the consumer evidence: fosved-acc carries 22 sanctioned
allow-comments in 10 imported modules — guards written through the
`redact(…,"hash_only")` barrier because the native `if r.ok` form on
`parse_json(read_file/http)` is refused by UNTRUSTED_DECISION; on
v0.38.1 app.mlog and all polygons fail to start, the consumer stays
pinned at v0.33.2). Fixed-only: the [Unreleased] wave-80 rows stay
pending for the next minor train — the patch cut does not carry them
(the 0.30.1 form). The publication is the OWNER act (ADR-0186 §4.5);
release.yml attaches the binary, the SBOM, the attestations and
SHA256SUMS on publish.

### Fixed

- **№897 (issue #1789 — [process][bugfix]):** the №523 run/serve
  gate applies the №802 allow-comment suppression PER MODULE — each
  finding is sanctioned only by its own module's source text. The bug:
  `enforce_run_gates` checked the MERGED surface (entry ++
  statically-resolved imports) but ran `filter_suppressed_unchecked`
  against the ENTRY source only, so an
  `[UNCHECKED_RESULT]`/`[UNCHECKED_OPTION]` finding inside an imported
  module never matched its comment and blocked `mlog run`/`mlog serve`
  while `mlog check` passed the same program (the lib.rs №802 note
  states the opposite intent: "the checked surface and the run gate
  must agree — a suppressed-at-check program must not refuse at run").
  The fix is exact, not heuristic: the UNCHECKED walk is
  per-declaration (a fresh flow per statement-bearing body, the
  callables map read-only), so a module's findings computed under the
  MERGED callables surface are byte-identical to its segment of the
  merged check's UNCHECKED block; the unchanged №802 filter (the
  per-line quota, the non-empty-reason rule) runs per module source and
  the gate's own error list keeps its order — one module's
  allow-comment cannot launder another module's access (the №802
  multi-line laundering rule, now also cross-file). The no-import
  programs behave byte-identically to 0.38.1 (the empty-group case
  degenerates to the old filter). Pinned by the regression pair and
  the laundering/quota/dedicated-form pins
  (`naryad_897_per_module_suppression.rs`, 6 tests).

## [0.38.1] - 2026-11-03

The unscheduled patch cut (the 0.28.1/0.30.1 shape — ADR-0189: an
unscheduled cut is a PERMITTED exception when a defect cannot wait for
the next Monday window, with the reason recorded in the release
notes). The reason: issue #1735 — P0, out of the release train (the
owner's decision 03.11). The office production (v0.38.0) had its
native cron ticks down since the 0.38 switch: every `http_post` from a
tick refused `IRREVERSIBLE_NO_GRANT` on a VALID
`profile capability { allow_carrier: … }` declaration (the night
cycle, night report, morning brief, admin reply and LZ daily slots
silently dead), and interpreter-backend route bodies refused the same
way. The owner ruled the office stays on v0.38.0 and waits for this
fix (no rollback to 0.34.1). Fixed-only: the [Unreleased] wave-78 rows
(Added/Changed) stay pending for the next minor train — the patch cut
does not carry them (the 0.30.1 form). The publication is the OWNER
act (ADR-0186 §4.5); release.yml attaches the binary, the SBOM, the
attestations and SHA256SUMS on publish.

### Fixed

- **issue #1735 (P0 — out of the release train, the owner's decision
  03.11):** the entry's `profile capability { allow_carrier: … }` roster
  is applied again by the serve paths that lost it — the permit roster
  propagates into the serve-boot interpreter merge
  (`merge_interpreter` → `clone_definitions_into`, the UNION add-only
  discipline of the №332 definition class) and into the fresh program
  context (`fresh_program_context` — the №426 native cron-tick executor
  AND the TW route bodies). On 0.38.0 neither leg carried
  `Interpreter::capability_permits`, so a correctly declared program
  refused every sharp carrier from a native cron tick with
  IRREVERSIBLE_NO_GRANT under BOTH serve backends (the tick executes on
  the tree-walking interpreter whatever the backend is), and the TW
  route bodies refused the same way — the issue's original 500 row.
  The consumer evidence: the FOSVED office night cycle, night report,
  morning brief, admin reply and LZ daily were silently down since the
  0.38 switch. The gate is untouched: no new grant source (the roster
  originates only in `permits_from_declarations` / the VM wire field),
  PROFILE_PERMIT_SHARP / GRANT_ALWAYS / the refusal vocabulary do not
  move (ADR-0203 §2.4) — the same program WITHOUT the declaration still
  refuses IRREVERSIBLE_NO_GRANT, fail-closed by default. Pinned on the
  REAL `mlog serve` process, BOTH backends
  (`naryad_1735_tick_capability_context.rs`: the declared tick and the
  route round-trip; the undeclared tick and route refuse with the
  local receiver seeing NO ping — the refusal is BEFORE the action),
  plus the merge-union unit pin (add-only, empty-safe). **AFFECTS serve
  consumers: native cron ticks and interpreter-backend routes regain
  the declared carriers on 0.38.0 → 0.38.1; the office pins the 0.38.1
  patch in a separate office naryad after the release (the owner's
  decision 03.11).**

## [0.38.0] - 2026-11-01

The scheduled train (ADR-0189): the 0.38 window — the 8B-2 subphase
of Phase 8 (the C11 «Typed Consent & Human Oversight» line, ADR-0205,
the wave-75 naryads №859-№864). The honest-tag note: this cut carries
TWO pending phase lines — [Unreleased] held the wave-74 row (the 0.37
line, №853-№858, ADR-0204) and the wave-75 row (the 0.38 line,
№859-№864); no 0.37.0 tag was published (the owner directive
2026-11-01 «Публикуй 0.38» named 0.38 only), so BOTH rows ride in
this cut, moved verbatim — the train order recorded here (the
[0.35.0] form). The publication is the OWNER act (ADR-0186 §4.5)
executed by the office machine on the explicit directive (the 0.36.0
precedent).

### Changed

- **BREAKING (№865 — the migration-honesty verdict 2026-11-01; the rule
  is №861, ADR-0205 §2.6):** `cron_run` is a human-oversight carrier
  (`HUMAN_OVERSIGHT`, `src/approval.rs`): a call without a LIVE
  approval mandate refuses BEFORE the action with
  `IRREVERSIBLE_NO_GRANT` — record the human's decision with
  `approval_issue(actor, "carrier:cron_run", ttl_seconds)` (the
  mandate is bound to the CARRIER, not the call digest; it expires by
  ttl; `approval_revoke("carrier:cron_run")` withdraws it). A 0.34-era
  program that fires `cron_run` refuses at the first call on 0.38.0.
  The migration recipe: [docs/upgrade.md](docs/upgrade.md) (§
  "0.34 → 0.38", the cron_run step). The `### Added` row below stays
  verbatim — read the two rows together (the rule shipped in the Added
  prose without the BREAKING mark; that omission is what this row
  fixes).

### Added

- **№859–№864 (Wave 75, P1/P2 — the 8B-2 subphase, the C11
  "Typed Consent & Human Oversight" line, ADR-0205):** the human
  mandate becomes a typed VALUE. №859 — ADR-0205 before the code: the
  mandate is a typed carrier of the human consent/oversight decision
  (actor, target digest, the window, revocation) — a condition of
  action for a grant of the human-oversight class, NOT a new grant
  mechanism and NOT a replacement of the ADR-0203/ADR-0155 contour; the
  refusal value comes from the frozen W-C algebra (ADR-0204), the
  existing `IRREVERSIBLE_NO_GRANT` class, no new codes (№385).
  №860 — the carrier in code: the pure `TypedApproval` (the portable-
  kernel boundary of ADR-0204 §3), the issue / verify / revoke
  constructors, the «2 negative controls» (the foreign digest, the
  expired/revoked mandate — both deny BEFORE the action); the 1:1
  migration (the DenyEvent surface and the Ledger records unchanged).
  №861 — the coverage rule: the fail-closed approval gate composed into
  `capability::authorize_call` on BOTH backends (one decision surface —
  the TW/VM parity by construction), the explicit `HUMAN_OVERSIGHT`
  registry transaction opens with `cron_run` (the autonomous-run
  trigger), the consent surface `approval_issue` / `approval_revoke`
  (registry 528→530, append-only), the e2e trace request → approve →
  execute → revoke/expire → deny green on both lanes, the Ledger
  journals the existing deny form. №862 — the `APPROVAL_COVERAGE`
  machine fact (the share of the human-oversight carriers covered by
  the gate, over the live roster and the composition site) with the
  gate_036 only-up floor 10000 bp (the plan §23 number) and the tamper
  traps (the forged name, the removed site, the substituted definition,
  the lowered floor). №863 — the coverage-floor record re-pinned by the
  fact (78.04% @ `8e40e02e` — the «the floor follows the fact» form).
  №864 — the catch-up weekly records for waves 67–74 (the generator
  №789) and this sanitized 0.38 phase line. Honest limits (ADR-0205 §5):
  the mandate is not a signature, not a legal act, not an audit
  mechanism, not IO/UI; the class extension is an explicit registry
  transaction.

- **№853–№858 (Wave 74, P1/P2 — the 8F-1 subphase, the C6
  "Portable Refusal Algebra" line, ADR-0204):** the refusal becomes a
  portable VALUE. №853 — ADR-0204 before the code: the refusal is a
  value of a finite lattice over the frozen 14-word `DENY_REASONS`
  vocabulary (№392), `join` = the more severe cause / `meet` = the less
  severe, the verdict `Allow | Deny { code, cause }` with the frozen
  codes BY REFERENCE (№385/ADR-0169 — no new codes); the
  portable-kernel boundary (in: the lattice, the codes, the verdict,
  the JSON rendering; out: IO, the Ledger, the network, the language
  compiler) and the `SEMA_EXTERNAL` machine-fact definition. №854 — the
  algebra in code: `src/refusal_algebra.rs` (the dependency-free SSOT —
  the vocabulary table moves there, `deny.rs` re-exports it), the
  `DenyEvent` construction migrated 1:1 through the algebra (the same
  events, the same Ledger records — the parity test pins all 14 words
  byte-identical; the ledger-golden and conformance suites green with
  zero golden edits). №855 — the WASM contour: the `refusal-wasm`
  facade compiles the SAME dependency-free kernel source for
  `wasm32-unknown-unknown` behind a C-ABI/JSON boundary
  (`rw_alloc`/`rw_eval`/`rw_reset`, the JSON verdict contract); the
  `wasm-facade (blocking)` CI job builds it and the host smoke consumer
  (`node refusal-wasm/smoke.mjs` — no Rust in the loop) drives it on
  every push. №856 — the reusable `policy-check` Action (the root
  `action.yml`): any repository evaluates a refusal policy (a JSON of
  lattice requests) through the algebra in its own CI; the outputs are
  `verdict` + `code` (the join — the MOST severe cause — across the
  denying requests, computed by the algebra itself); the
  `policy-check-dogfood (blocking)` job is the living external consumer.
  №857 — the machine fact: `SEMA_EXTERNAL` in `gen_metrics --check`
  (the live external-consumption artifacts, each counted ONLY on the
  real existence + the live blocking-CI wiring) with the gate_035
  only-up floor ≥ 1 (the plan §23 number); the tamper traps catch the
  placeholder, deletion and definition-substitution classes. №858 — the
  spec topic 7 (`docs/spec/refusal-algebra.md`): the norms S-RA-001
  (the handled refusal degrades, the refused action never executes) and
  S-RA-002 (the frozen codes stamped at position 0) with the
  conformance pairs on BOTH backends, the consumption contract
  (WASM/Action/SEMA), the honest-limits section; the sanitized 0.37
  phase line (this entry and the PLAN-SUMMARY row).

## [0.36.0] - 2026-10-31

The scheduled train (ADR-0189): the 0.36 window — the 8B-1 subphase
of Phase 8 (the C2 «Capability/Grant Runtime» line, ADR-0203, the
wave-72 naryads №843-№848). The honest-tag note: the №844-№847 code
was already present in the 0.35.0 tree (the wave-72 merge preceded
the 0.35.0 tag — the train order recorded in [0.35.0]); the 0.36.0
tag differs from 0.35.0 by the version bump and this CHANGELOG row —
0.36.0 is the first release whose notes document the capability
token (the №497 rule: the notes and the changelog can never
disagree).

### Changed

- **BREAKING (№865 — the migration-honesty verdict 2026-11-01; the rule
  is №845, ADR-0203 §2.4):** the sharp set refuses BEFORE the run —
  12 carriers (`exec`, `exec_argv`, `delete_file`, `git_push`,
  `http_post`, `http_post_multipart`, `send_message`, `send_photo`,
  `send_document`, `smtp_send`, `smtp_send_html`, `tts_send`) fail
  with `IRREVERSIBLE_NO_GRANT` without one of the THREE legal paths:
  (1) a covering capability grant (issue with scope `builtin:<name>`
  — the strict path, TTL/quota metered at the gate), (2) `profile
  capability { allow_carrier: "<name>" }` (the minimal path),
  (3) (exec/exec_argv) the host permit — `METALOGOS_ALLOW_EXEC=1` in
  the process context, `METALOGOS_SERVE_ALLOW_EXEC=1` in a
  serve-route body. `profile legacy` does NOT bypass the rule. A
  0.34-era program calling any sharp carrier refuses at the first
  call on 0.36.0. The migration recipe:
  [docs/upgrade.md](docs/upgrade.md) (§ "0.34 → 0.38"). The `### Added`
  row below stays verbatim: its "existing programs byte-unchanged"
  clause refers to the №844 grant-carrier migration (the typed
  Capability view over the same ledger), NOT to the №845 coverage
  rule — read the two rows together (the rule shipped in the Added
  prose without the BREAKING mark; that omission is what this row
  fixes).

### Added

- **№844–№847 (Wave 72, P1/P2 — the 8B-1 subphase, the C2
  "Capability/Grant Runtime" line, ADR-0203):** the capability token
  lands. №844 — the TYPED carrier: `grant_issue`/`grant_subgrant` mint
  the read-only `Capability` view `{scope, class, ttl, quota, revoked,
  grant}` over the grant ledger (the ledger stays the SSOT — the token
  is an untrusted cache); the new `capability_state(c)` builtin is the
  LIVE ledger view; every grant-argument extractor accepts both the
  opaque Grant and the typed Capability (the 1:1 migration — existing
  programs byte-unchanged). №845 — the coverage RULE: every
  irreversible carrier passes the capability decision BEFORE the run on
  both executors; the named standard baseline posture permits the
  runtime-state and own-channel classes; the sharp set (exec, exec_argv,
  delete_file, git_push, http_post, http_post_multipart, send_message,
  send_photo, send_document, smtp_send, smtp_send_html, tts_send)
  refuses without a covering capability grant (scope `builtin:<name>`),
  an explicit `profile capability { allow_carrier: "..." }` permit, or
  (exec) the host permit — the refusal rides the existing
  `IRREVERSIBLE_NO_GRANT` deny class (the vocabulary stays 14); the
  `memory_forget_cascade`/`forget` linear actions keep their
  grant-always contract (no profile can weaken it). №846 — the machine
  fact: `GRANT_COVERAGE` in `gen_metrics --check` with the gate_034
  only-up floor `10000 bp` (the plan §23 number); the strict loader
  refuses a corrupted floor file, the checker refuses a floor lowering.
  №847 — the acceptance e2e: issue → use → revoke → deny with the
  ordered Ledger golden (`examples/w72_capability_trace.mlog`), the
  negatives (no grant / expired token / Once linearity / N(2) quota at
  the gate) on both executors. The e2e surfaced the §2.2 consume
  semantics: the gate's grant route consumes one use through the single
  consume point (`grant_use`) — the Once/N quota is metered at the gate.


## [0.35.0] - 2026-10-31

The scheduled train (ADR-0189): the 0.35 window — the 8D-1 subphase
of Phase 8 (the C4 «License-Typed Model Layer» line, ADR-0201 +
ADR-0202, the wave-71 naryads №836-№842) plus the wave-68 feature
rows that waited for the train (№823/№825/№826 — ADR-0200/ADR-0216,
№824 — the CI hygiene fix). The train-reality note: the wave-72
capability-token work (№844-№847) was merged to main BEFORE the
0.35.0 tag was cut — the 0.35.0 binary carries that code, and its
CHANGELOG row ships with [0.36.0] (the honest-notes rule: the notes
and the changelog can never disagree, №497).

### Added

- **№826 (Wave 68, P1 — the W2′ metric registry, ADR-0216):** the
  `rollback_if` gate reads honest quality signals. `METRIC_REGISTRY`
  grows `macro_f1` and `min_class_recall` beside `accuracy` (the same
  `MetricSpec` form, no new `Value` variants); the math is the
  `metalogos-reflex` SSOT (№545) — the class-space bodies behind both
  the registry adapters and the gate's label conversion (an unanswered
  held-out task is the residual `<no-answer>` class: no metric looks
  good by ignoring the tasks the backend failed to answer). The
  grammar accepts a metric NAME: `rollback_if: macro_f1 > 0.8` — and
  the old literal form `rollback_if: accuracy OP x` stays valid
  (`accuracy` is a registered name; the golden corpus output is
  byte-unchanged, pinned). An unregistered name is a LOUD semantic
  refusal (`mutate: unknown metric '...'. Registered metrics: ...` —
  the `reflex_train` convention), on both lanes by construction (the
  check runs on the AST before any backend compiles it). Every GATED
  decision is accompanied by the **majority baseline** — the same
  metric evaluated on the always-majority predictor over the same
  held-out labels (the №485 distill-gate pattern; the computed
  on-the-spot surface, not a stale-prone grammar field) — and a gating
  rule with NO held-out evidence has no baseline: a LOUD refusal on
  both lanes, never a keep. The message builder is ONE body for both
  lanes (byte parity pinned); for the default `accuracy` the string is
  byte-identical to the pre-№826 formats — the №375 mock pins hold
  without edits; the mock stub stays 0.95 for any metric (the
  mechanism exercise, ADR-0112) with no baseline segment (no evidence
  surface in mock mode — fabricating one would be a silent lie).
  Contract tests: `tests/naryad_826_metric_registry.rs` (the six
  contracts, RED-first per форма №772 — 9/11 red on the unpatched
  main). security: no new egress or taint surface — the registry is a
  closed name set validated at the check; the gate refuses
  fail-closed (no baseline → no keep); the mock-mode stub is loudly
  documented and unchanged.

- **№825 (Wave 68, P1 — the W1 Reflex attestation, ADR-0200):** the
  provenance certificate for trained Reflex weights. `reflex_save`
  now writes a canonical-JSON certificate (keys sorted: the
  attestation_version, model, pattern, spec_hash, data_hash,
  embed_signature, weights_sha256, teacher — null with the reason
  until W3b, dataset{rows, filter:"none", provenance}, eval —
  `uncalibrated` until E3, rollback_if — null with the reason until
  W5, toolchain, ledger_seq) + the `reflex.attest` ledger anchor
  (cert_sha256 in the Ed25519 chain — tamper-EVIDENT per ADR-0167)
  in the SAME transaction as the weights row; a ledger write failure
  is a LOUD save error (the artifact rule — ADR-0200 §4), never a
  silent save. The `reflex_attestation` history table (append-only,
  + the spec_json snapshot column the changed-field contract
  consumes); `reflex_load` verifies before deserialize: weights/embed
  mismatches always refuse, a stale spec refuses with the CHANGED
  FIELD NAMED unless `--allow-stale`/`METALOGOS_ATTEST_ALLOW_STALE`
  (loudly audited + a ledger record), data changes are informational
  (`[ATTESTATION_DATA_CHANGED]`), pre-W1 weights warn and continue
  with `cert: none`, and `METALOGOS_ATTEST_STRICT=1` refuses both the
  missing and the `unsigned` certificates (owner decision 4; the
  default flips strict one release after W1). The new
  `attest_verify(model) -> Struct{ok, level, spec_hash, reason}`
  builtin (tier `experimental`, поправка (г); Source × Internal ×
  Pure) answers on BOTH lanes identically (the VM intercept joins the
  reflex-group shape; `ok: true` means provenance consistency —
  NEVER quality, decision 5; `level: unfiltered` while every W1
  dataset is unfiltered). The new `mlog attest verify <file.mlog>
  [--model=Name] [--allow-stale] [--json]` CLI — pure parse + SQLite
  read, the program is NOT executed. Contract tests:
  `tests/naryad_825_attest.rs` (the eight contracts). security: the
  attestation is a trust surface — the honest boundary is ADR-0200 §2
  (not a quality proof, not tamper-PROOF); the REFERENCE rows and the
  ADR-0200 implementation record carry the boundary verbatim.

- **№823 (gh#1600, Wave 68 — the E1–E4 measured baseline for the
  W4–W6 line, ADR-0200):** the distillation experiments, measurements
  only — the embed speed (the SSOT TF-IDF backend: ~11–12 µs/line
  steady-state; the vocabulary growth drives ~8.6× per-line cost at
  10k distinct tokens — the W4 corpus-vocabulary argument), the
  reflex_train time/memory curve (linear in N, ~3.2–4.2 µs per
  example per epoch; the per-C convergence budget measured), the
  confidence calibration (ECE ≈ 0.044 over 3 procedural seeds — the
  softmax head is consumable on the clean shape; the per-corpus
  re-measure is the W6 gate input), the teacher-agreement harness
  (0.95 vs the 0.58 majority baseline on the 60-text corpus through
  the serve distill lane). The scripts live in `scripts/research/`,
  the raw CSVs + the document in `docs/research/`
  (`distill_experiments_2026-11.md`). security: no — measurement
  artifacts only, no product surface change; the honest findings (the
  serve route lane's smart-router gap, the VM list-clone loop
  artifact) are recorded in the document, no behavior changed.

- **№836-№842 (Wave 71, P1/P2 — the 8D-1 subphase, the C4
  "License-Typed Model Layer" line, ADR-0201 + ADR-0202):** the model
  passport lands. №836 — the through-line ADR: Phase 8 «Verifiable
  Compliance & Trust», the train 0.35-0.47, the ADR booking 0201-0215
  (the owner's R38 gate: the ADR ships after the 0.34.0 window).
  №837 — the passport ADR: one typed artifact instead of the
  scattered license-class / digest / provenance holders. №838 — the
  CODE: `src/model_passport.rs` introduces the `ModelPassport` type
  (identity, digest, license class + note, the freshness/verification
  date) and migrates the two existing holders 1:1 —
  `BackendEntry`/`WeightsFile`/`WeightsSource` (`src/backends.rs`) and
  `WeightsManifest` (`src/vision/weights.rs`) — no behavior growth
  (the №545 stop-list). №839 — the RULE: the license-class x profile
  permit coupling on the compile path — `non-osi`/`restrictive`
  backends load ONLY in a profile that explicitly permits the class,
  the refusal happens BEFORE the run (fail-closed; the negative
  controls ride the leak suite). №840 — the distillation firewall,
  shadow-first (the owner decision R38(4)): the distill events (the
  donor, the acceptor, the weights/passports involved) are observed
  and logged, the contour is NON-blocking by construction until
  validated — the №581 lesson (hardening without negative controls
  breaks legitimate paths) is the design constraint. №841 — the
  machine facts: `LICENSED_FLOOR` (the explicit-class + filled-note
  share of the backend registry, bps) and `PASSPORT_FRESH` (the share
  of the №838 passports within `PASSPORT_FRESHNESS_DAYS = 180`) in
  `gen_metrics --check`, the gate_033 only-up floors. №842 — the
  hygiene: the passport freshness/revocation reads are fail-closed
  («passport digest mismatch» refuses; the re-pin goes through an
  explicit update only — the G5 digest-pinning pattern, №828).
  security: yes — the license-typed layer is a trust boundary of the
  model supply chain; the default-deny posture `MODEL_WEIGHTS_UNSAFE`
  (№334) stays, the rule adds the class dimension the posture lacked.

### Fixed

- **№824 (gh#1592, Wave 68 — the CI hygiene, the №809 follow-up):**
  the weekly merge↔CI detective carried only 13 of the enforced
  contexts — `REQUIRED_CHECKS` is now the LIVE branch protection
  verbatim (23 contexts, the 2026-10-28 admin read; the four pillar
  contexts the owner's №1593 toggle dropped stay out of the set),
  the reverse direction diffs the live rule against the set (an
  enforced-but-unaudited context is a named M-1 divergence; an
  unreadable protection is exit 2 fail-closed, №525 — the
  `BRANCH_PROTECTION_TOKEN` secret rides the workflow env, the №586
  form), and the №809-class tripwire requires a pull_request-producing
  workflow job for every required context (the offline
  `--tripwire-check` mode rides every CI lane via the driver test).
  Script + driver test + the maintainers.md checklist move together
  (the №551 rule). security: no — the audit tooling hardening, no
  product security surface changes.


## [0.34.1] - 2026-10-30

The unscheduled fix-release (ADR-0189 §2.6): the published 0.34.0 build
carries the VM mis-dispatch at `profile legacy` (the H-1 of the merged
audit `METALOGOS_MERGED_AUDIT_24fde8a`) and the SQL gate covered 2 of
the 5 classified carriers (the H-2) — the defects cannot wait for the
next Monday window (the office prod-transition contour runs on this
line). No new features: the wave-68 feature rows (№825/№826) stay in
[Unreleased] for the next scheduled train.

### Security

- **№831 (gh#1625, Wave 70, P1 — the H-2 of the merged audit
  `METALOGOS_MERGED_AUDIT_24fde8a`; the behavior change, fail-closed;
  AFFECTS programs that pass non-literal SQL to `query_row` /
  `query_scalar` / `db_execute_with_grant` — they compile clean on
  0.34.0 and refuse at check on 0.34.1; the audit guard scenario: the
  SQL_DYNAMIC gate read exactly `name == "query" || name == "db_execute"`
  (src/audit.rs:881 @ `24fde8a`) — 2 of the 5 classified SQL carriers,
  so `query_row("SELECT * FROM u WHERE name='" + query_param("n") + "'")`
  passed `mlog check` without findings):** the gate now reads the SQL
  position from the classification SSOT (`builtins_classification::
  SQL_ARG_POSITIONS`) — `query` / `query_row` / `query_scalar` /
  `db_execute` at argument 0, `db_execute_with_grant` at argument 1
  (the grant handle occupies arg 0) — 5/5 carriers, both contours by
  construction (the gate lives in `audit.rs`; the classification table
  is GENERATED into the `gen_classification.py` template — the №777
  freshness gate refuses hand edits). `db_insert` carries the separate
  identifier arm: the table must be a string literal (the identifier
  substitutes into the SQL text — a one-operator modification vector);
  the column structure refuses when it derives from an untrusted source
  (`json_body()` / `query_param()` / … — mass assignment + column-name
  injection), the struct literal stays the whitelist, and the trusted
  dynamic shape rides the runtime `quote_identifier` hardening
  (the SQLite double-quote escaping in `db_ops.rs`) as the second
  layer. The leak-suite negatives grow 49 → 54 (n29 `query_row`, n39
  `query_scalar`, n40 `db_execute_with_grant` — all compiled clean on
  unpatched main; n41 dynamic `db_insert` table, n42 `db_insert`
  `json_body()` struct). Pins: `tests/naryad_831_sql_gate.rs` (11
  contracts, RED-first per форма №772). The fix lands 2026-10-29 (this
  PR; the published v0.34.0 tag is affected — no re-tag, the №620
  precedent). security: yes — the H-2 closure; the language promise
  («нелитеральный SQL — отказ до запуска») now covers the whole SQL
  contour.

### Fixed

- **№829 (gh#1622, Wave 69, P1 — the H-1 of the merged audit
  `METALOGOS_MERGED_AUDIT_24fde8a`, gh#1616 + gh#1618 — one class, one
  hearth; the behavior change, fail-closed; AFFECTS 0.34.0 AND EARLIER
  on the VM backend; the audit guard scenario: after a top-level
  `profile legacy` declaration a call to a pattern defined EARLIER in
  the file could execute a DIFFERENT pattern — typically the one
  defined immediately before the callee; matching arity silently
  routed the arguments and completed with WRONG RESULTS and NO error,
  mismatching arity named the wrong pattern (`pattern _Mark expects 2
  args, got 0`); `mlog check` did not flag it — the divergence was
  runtime-only and VM-only, the TW oracle dispatched by name
  correctly):** the VM pattern dispatch no longer depends on the
  declaration-order artifacts — the body-count index allocator assigns
  stable dispatch indices, the dispatch-table gate is LOUD (a table
  inconsistency refuses at compile time instead of the silent
  corruption), and the last-match name resolution restores the
  by-name callee identity for backward-reference calls. Red-before:
  the office v3 battery 16/19 files red on the VM lane; the public
  repros (test_n553v3 / test_n556v3 / test_n565v3 / test_n566v3 /
  test_n559v3 / test_n588_night / test_forge3v3) green on both lanes
  after the fix; two negative controls (the profile does not affect
  the dispatch; the TW lane unchanged). Pins: `tests/` per the №829
  PR (gh#1629). The fix merged to main before this release; the
  published v0.34.0 build predates it — the [!WARNING] block on the
  0.34.0 release notes (№830) is stamped fixed-in-0.34.1, the
  limitations row closed with this release link. The fix lands
  2026-10-29 (this release; no re-tag — the №620 precedent). security:
  no — the defect mis-delivers a VALUE in a pattern call (the
  integrity class), it neither widens a sink nor weakens a gate (the
  №800 row verdict form).

## [0.34.0] - 2026-10-28

### Migration

Changes in [Unreleased] that can break existing programs, each with the
repair:

- **№800 (gh#1547, V3):** `run` / `eval` / the REPL execute on the VM —
  the single user-command executor; the TW stays the reference oracle
  behind the transitional `--reference` flag. A program that diverged
  between the backends now surfaces the VM's verdict; pin the expected
  outputs (the examples carry `.expected` pairs) and report a divergence
  as an issue, do not rely on the TW parity by default.
- **№799 (gh#1546, C3-a T2):** a parked builtin now refuses at `check`
  with `[BUILTIN_PARKED] <name> requires --features <f>` instead of
  failing at runtime — enable the feature in the build (or the run
  profile) before shipping a program that names a parked builtin.
- **№781 (gh#1492, A2):** `escape_html` no longer declassifies —
  escaping a Secret now produces the masked form instead of the secret
  bytes. Templates that escaped a secret to embed it must switch to the
  explicit declassification points (the redaction pipeline or the
  sanctioned fingerprint), not the HTML escaper.
- **№787 (gh#1498, I-c):** the integer precision loss beyond 2⁵³ at the
  SQLite/JSON borders is a loud refusal — retype the affected fields to
  the text lane (or the hashed comparison) if a program relied on the
  silent truncation.

### Added

- **№763 (gh#1416, Wave 58 — the agent-index honesty, the R-01
  finding):** the llms.txt drift (uncovered by any generator gate since
  its creation) is closed by a BLOCKING gate. (1) llms.txt refreshed
  FROM the generators — the builtins count 409 → 526 (the registry
  scan), the examples 214 → 253, the ADRs 132 → 187 (the gen_metrics
  machines), the naryad range №1-№289+ → №1-№763 (the merged-history
  №505-grammar scan — the naryad sequence machine), and the three
  missing canonical rows added (docs/spec/tools.md, docs/limitations.md,
  docs/PLAN-SUMMARY.md). (2) CLAUDE.md — the declared "bridge copy of
  AGENTS.md" — restored byte-identical to AGENTS.md (was 8958 vs 10703
  bytes; the №676 A5 content-source form: AGENTS.md is the source, the
  copy is one). (3) NEW `scripts/ci/agent_index_gate.py` (the
  gen_metrics --check form): the llms.txt counters == the generator
  machines, the bridge byte-identity, every llms.txt link target
  exists; `--self-test` carries the tamper traps (the stale counter,
  the broken bridge, the dangling link — all spring). Wired as the
  `agent-index (blocking)` CI job (fetch-depth: 0 — the git-log
  machine) + the blocking_checks.tsv cell (33 cells) pinned by
  `tests/naryad_763_agent_index_gate.rs`. Stale agent indexes are
  false facts on the first screen of IDE agents and RAG pipelines —
  the same honesty family as the stale-draft rule (№713); the drift
  now refuses the merge.

- **№788 (gh#1511, Wave 63 — audit-23.10 C4-a T3, the owner decision
  23.10):** the honest-boundary CHANGELOG verdict — `security: no —
  <reason>` on the row itself is the THIRD legal path (next to the
  `### Security` section and the `release-block` evidence); the gate
  accepts it and prints it for the review. The rephrase-circumvention
  detector rides the same blocking job: a dictionary-matching line
  removed from CHANGELOG/limitations whose replacement matches neither
  the dictionary nor the verdict exits 1 («rephrase without verdict»)
  — the №762.1/№771 synonym lane is closed, the dictionary unweakened,
  the hunk-scoped pairing keeps multi-line entries together. Self-test:
  six new cases (the verdict accepted, the bare `security: no.`
  rejection, the №771 shape caught, the verdict-carrying rewrite
  lawful, the dict-vocabulary lane, the limitations shape). The
  AGENTS.md rule: a naryad whose only content is a rephrase under the
  gate is not issued.

- **№789 (gh#1512, Wave 63 — audit-23.10 C4-a T1, the owner decision
  23.10):** `scripts/gen_wave_record.py` — the wave-record generator:
  by the `wave-N` label it finds the dispatch, parses the naryad
  composition table, and assembles the REALITY §-record (per naryad:
  the issue, the source issue, the merge commit from the git-history
  scan, the PR, the CI verdict ON the merge commit) + the PLAN-SUMMARY
  line. The machine counters ride VERBATIM from the scripts/ci
  machines — nothing paraphrased; a broken machine prints
  `MACHINE UNAVAILABLE` into the record (never silently dropped, never
  invented). The prose is ONLY the `HONEST-FINDING:` marked lines from
  the naryad audit comments (no markers → an empty section — the
  generator writes no prose, the C4 form). The weekly flow (one PR for
  all the waves of the week, manual until the owner decides on the
  schedule) replaces the per-wave «strictly last» docs naryad. The
  verification: the wave-62 and wave-61 records generated — the
  structured fields reproduce the git/API facts 1:1 (the comparison
  with the manual §6.46 rides the naryad report); the wave-61 sweep
  surfaced №778 as closed-without-a-merge-commit (the honest
  no-surface rendering).

- **№790 (gh#1513, Wave 63 — audit-23.10 C1 (b), the owner decision
  23.10):** ADR-0196 **Accepted** — the MCP policy gateway is the
  FLAGSHIP PILOT: the language stays a product until the fork; the
  pilot scope G0 (#1458 landed as №780, #1460) → G1–G4 (#1480–#1483,
  waves 64–66); the MEASURABLE exit criteria (the published G4 demo
  with the reproducible script; the Fosved Office dogfood; ≥ 3
  external installation attempts) — conjunctive; the fork by the
  results is a NEW owner act (the expansion (a) or the course
  unchanged (c)); the boundaries: no builtin expansion beyond the
  gateway's need, the domain quota and the stop-list in force, the
  language roadmap untouched. The full owner verbatim lives in #1470;
  the ADR quotes the posture line (the docs-language budget — the
  lint is the honest machine). The ADR index 187 → 188.

- **№791 (gh#1514, Wave 63 — audit-23.10 C5-b V1, the owner decision
  23.10):** ADR-0197 **Accepted** — the VM is the ONLY executor of the
  user commands (`run`, `serve`, `mcp-serve`, `eval`, REPL); TW is the
  reference oracle in CI (the crosscheck, the diff-fuzzer, the
  doc-tests) and behind the transitional `--reference` flag (two
  release windows by default, an extension is a new owner act). The
  same-PR lineage edits: ADR-0105 — the remainder SUPERSEDED (the
  TW-default posture; ADR-0141 had already superseded the readiness
  reservations), ADR-0171 — EXTENDED from `serve` to ALL the
  user-command lanes. The oracle's contract stays: the core-semantics
  criterion, the shared-RuntimeContext dedup, the CI oracle surfaces.
  The landing order (the owner's verbatim): the precondition #1458
  landed (№780), V2 rides with the gateway pilot, then V3, then the
  V4 gate closes the lane. The ADR index 188 → 189.

- **№792 (gh#1515, Wave 63 — audit-23.10 C6 R-b 1, the owner decision
  23.10):** ADR-0198 **Accepted** (supersedes ADR-0106) — the error
  model is the LOUD failure with a code + the explicit `_or`
  quietness + `try` for the handling; in the type layer `Result<T>`
  (the try result) and `Option<T>` (= `T|Unit`) are the ALIASES of the
  existing shapes — **no new `Value` variants, the grammar of values
  and the 526 signatures unchanged, no full sum types** (`Ok/Err/
  Some/None`, `?` are not introduced). The same-PR lineage edits:
  ADR-0106 → Superseded (the soft-failure premise expired; the
  "not without demonstrated need" principle survives), AGENTS.md §3/§6
  re-pointed (the CLAUDE.md bridge byte-identical). The I-c Int piece
  (the loud `[PRECISION_LOSS]`) already landed as №787; the static
  use-check rides #1505. The ADR index 189 → 190.

- **№793 (gh#1531, Wave 64 — the gateway G1, the C1 (b) owner
  decision, ADR-0196):** the MCP contour gains the SESSION layer —
  long-lived upstream sessions for the gateway. The new
  `src/mcp_session.rs` module carries the connection primitives
  (moved verbatim in behavior from the builtins module — the
  stateless `mcp_call`/`mcp_list_tools` contract is unchanged, the
  back-compat form the naryad requires) plus `McpSession`: ONE live
  process per declared upstream for the gateway's lifetime, the
  loud reconnect on an upstream death (the deterministic stream-EOF
  death signal + the reaped-exit check; a failed call is never
  auto-retried — the side-effect decision stays with the caller).
  The gateway reads `METALOGOS_MCP_UPSTREAMS` once at startup (the
  №263 discipline; entries `name=argv0,arg1,...` separated by `;`),
  aggregates the upstream `tools/list` under the `name.` prefix and
  proxies `tools/call name.tool` to the session — the response
  shape, the size cap and the error envelope are byte-identical to
  the local path; the LOCAL tools win a name collision (pinned
  loudly at startup); the allowlist gate rides every declaration
  fail-closed. The protocol: the client declares the CURRENT
  revision `2025-11-25` (the legacy `2025-03-26` declaration is
  superseded), the server negotiates the client's known revision
  (the hardcoded `2024-11-05` answer is gone), a mismatch is a loud
  refusal naming both versions. The 6-pin gateway battery +
  the 4-pin session battery verified RED-before on the pre-№793
  revision: 4/6 gateway pins failed (no upstream aggregation, the
  hardcoded version, no declaration gate), 2 regression pins green.
  AFFECTS: gateway deployments can now serve stateful upstreams
  (a browser, a db session); the stateless builtins unchanged.

- **№796 (gh#1534, Wave 64 — audit-23.10 C3-a T1, the C3 (a) owner
  decision, ADR-0196-adjacent):** the maturity tier SSOT — every
  `spec!` registration carries an EXPLICIT `tier`
  (`"stable" | "experimental" | "parked"`, the owner's rule table
  23.10). The gate IS the grammar: the spec! macro requires the
  trailing tier literal on every arm — a registration without one is
  a compile error (the №796 gate form, stronger than a CI script).
  The classification (the owner's rule): `stable` = the working,
  test-covered core (the labels, the grants, the ledger, MCP, the
  memory, the media handles, the working perception backends, the
  provider voice); `experimental` = the embodied family
  (device_*/pose_make/proof_*), the forecast family (series_*/forecast_*),
  the not-implemented stub seams (provisional — the dispute list);
  `parked` = the ADR-0178 generative contour (reflex-generate,
  vision-generate/edit/lora, voice_design/tts_speak/voice_enroll,
  video-render/extend/interp + the weights-fetch infrastructure).
  The compiled default build: 476 stable / 22 experimental / 11
  parked (509 = 526 source rows minus the 69 feature-gated); the
  source-level 526/526 ride gen_reference --check. REFERENCE's
  generated index gains the Tier column (the generator re-validates
  the vocabulary loudly — a tier outside the three-word set is
  exit 2). The BEHAVIOR IS UNTOUCHED (the naryad boundary: the
  marking only). The 3-pin battery (the vocabulary coverage, the
  owner-rule spot checks, the census) — new-surface pins. The
  disputable rows ride the naryad report for the owner's review.

- **№802 (gh#1549, Wave 65 — the C6 R-b 2 static check, the ADR-0198
  queue):** the try result and the `T|Unit` union are verified BEFORE use —
  the flow-sensitive semantic pass refuses `[UNCHECKED_RESULT]` (a
  `.value` read on a try-bound variable outside a proven `.ok` guard: the
  `if r.ok { … }` then-branch, the early-exit `r.ok == false { return … }`,
  the short-circuit `r.ok and …`, the `match r.ok { "true" then { … } }`
  arm) and `[UNCHECKED_OPTION]` (a union-with-Unit value — `query_scalar`'s
  `Float|String|Unit`, a declared `-> T|Unit` — used where `T` is required:
  the arithmetic/ordering operands, the `each` iterable, the string-key
  index, a declared precise parameter; `to_string` is the sanctioned total
  projection). Conservative posture: an unprovable guard refuses; the
  sanctioned escape is the explicit suppression comment with a non-empty
  reason (`// allow(unchecked_result): <reason>` — a bare allow is NOT a
  suppression; one comment sanctions one access line, several reads on the
  same line are a unit). The refusal fires on `mlog check` and blocks the
  №523 run gate and the serve startup on BOTH backends (the shared
  semantic pass; the mcp-serve startup keeps the unfiltered refusal — the
  fail-closed asymmetry is recorded). The registry SSOT carries the raw
  return-type spelling (`return_type_raw`) so the union arms are readable
  without a shadow table. Examples migrated: p30 projects the union cell
  through `to_string` (the honest fix); w385_office_branch and the №670
  conformance specimens carry the suppression comments with reasons (the
  redact-obfuscated guards and the verbatim shape pins are deliberate).
  RED-before 4/12 (the №772 form). security: no — the check widens the
  refusal surface only (fail-closed), no gate, allowlist or taint floor
  moves; the honest-boundary dictionary is untouched. AFFECTS: programs
  reading `r.value` without an `.ok` guard or consuming `T|Unit` values
  raw now refuse at check/run time — guard them, project through
  `to_string`, or suppress with a reason.

- **№805 (gh#1561, Wave 66 — the gateway G4, the pilot's exit criterion):**
  the demo + e2e — `examples/gateway/policy.mlog` (the 30-line gateway
  policy over two upstream MCP servers: file + fetch) and
  `examples/gateway/README.md` (the 5-minute guide for Claude Desktop /
  any MCP client) ship as CI-verified artifacts: the e2e
  (`naryad_805_gateway_demo`, 4 scenarios) reads the SHIPPED policy file,
  pools BOTH upstreams in ONE gateway process (the №793 pool's first
  two-upstream exercise), and runs the canonical event — the agent reads
  `.env` (Source private joins the session label), the POST through the
  fetch upstream is refused with `[SINK_CLEARANCE]` BEFORE the upstream
  sees the call, the refusal is a signed `mcp.call.denied` ledger record
  and the export verifies externally (`ledger_verify` ok); plus the
  allowed path (a fresh session posts), the once-grant unblock (№801) and
  the audit tap (`ledger_dump.dump` → the export round-trips through MCP
  and verifies). The fixture upstreams (`tests/fixtures/mcp_file_server.py`,
  `tests/fixtures/mcp_fetch_server.py`) speak the newline-delimited
  JSON-RPC the fixtures already use. `examples/gateway/screencast.sh`
  drives the REAL gateway with live JSON-RPC for the 60–90 s take (the
  video cut itself — verified-wait за вербатимом владельца, форма №755).
  RED-before: the naryad lands no product change (additive examples +
  tests + fixtures only) — the pins are the new two-upstream coverage and
  ran on the pre-№805 revision; the result is recorded in the PR body.
  security: no — the demo adds no executable product surface; the demo
  policy is conservative (Source private on the file upstream), the
  ledger/allowlist/label contracts are untouched. AFFECTS: none (the
  gateway surface is unchanged; the example is the new entry point).

- **№819 (gh#1587, Wave 67 — the #1302 per-provider model, waiting since
  16.10):** the `llm {}` provider entry carries the optional `model:`
  field (`alias, provider, key?, url?, model?` — the #1302 canonical
  order), and the model resolution moved INSIDE the failover loops: the
  call model (`call_llm(p, i, "model")`) → the provider's own `model:`
  → `default_model`, per rung, on both the call and the streaming paths.
  Before, the model was resolved once and handed to EVERY provider — a
  mixed provider set made the 2nd/3rd failover rungs ask for a foreign
  model name and fail (the office OSP refusal class,
  FOSVED-office-v2 #505). The honest-found companion gap closes too: the
  VM user lanes (`mlog run`/`mlog eval`/the VM REPL) silently ignored
  the declared `llm {}` — every `call_llm` fell back to the legacy
  env-var backend; the router install rides the lane preparation now
  (the №521 conversation_config class). RED-before: the pins
  (`tests/naryad_819_provider_model.rs`, over a real local socket) do
  not compile on unpatched main — the `model:` surface does not exist;
  the #1302 parse refusal (`expected RBRACE`) is the historical form.
  security: no — a routing/config surface addition; no label, secret, or
  sink surface changes; the provider set semantics only tightens.
  AFFECTS: mixed-provider failover works end-to-end; programs without
  the new field behave exactly as before (the back-compat pin).

### Changed

- **№767 (gh#1420, Wave 59 — the audit 21.10 M-1, the exemption's paper
  trail):** the read-back pair exemption (№729 — `cron_list` →
  `cron_remove`; `mtree_retrieve` → `mtree_forget`, the table
  `semantic::READBACK_PAIRS`) leaves the documentation trace the audit
  methodology demands: a limitations row (the Static Analysis section —
  the live anchor, the conservative boundary, the revisit condition) +
  the risk-review verdict recorded in ADR-0154 §11 (accepted,
  position-only, domain-matched, quarantine-proof; a third pair or a
  recognition-shape change re-opens the review). The implementation is
  NOT changed — the docs-only honesty pass; every future Category-A
  exemption follows the same protocol (the row + the verdict in the
  same PR).

- **№777 (gh#1452, Wave 61 — the №772 honest finding closed, the №744
  class):** the classification-SSOT generator is now drift-proof and
  gate-covered. A bare `gen_classification.py` run on a clean HEAD
  produced a non-default diff (builtins_classification.rs +8/−26 — the
  template's static fragments were not in the fmt-gate style;
  REFERENCE.md +1 — the block carried a trailing newline the tail after
  the END marker already provides, one empty line per run). (1) The
  template fragments now carry the fmt-gate style and the emitted Rust
  is normalized through rustfmt when it is on PATH — the generator
  cannot drift from the fmt gate by a formatter-default change. (2) The
  REFERENCE block no longer grows an empty line per run (the replace
  path emits END without the doubled newline). (3) The argv is loudly
  validated via argparse: unknown flags are a loud exit 2 — the
  `--goal032 --list` spelling from the №772 report was never a mode
  (the flags were dropped and the run REGENERATED the tree). (4) NEW
  `--check` mode (the gen_metrics --check form): both artifacts are
  rebuilt in memory and compared against the tree — exit 1 on drift,
  nothing written; wired BLOCKING into the fmt job right after
  `cargo fmt --all --check` (the fact key WITH its checker, №735).
  Verification: a one-token classification tamper springs `--check`
  (exit 1); `--goal032 --list` exits 2 with a named refusal; the bare
  run on a clean HEAD is diff-empty (idempotent). AFFECTS: the
  regeneration workflow and the CI surface only — no product behavior
  change; the classification data (526 entries, 289 non-Pure) is
  byte-identical; the floors stand (sensitive 7586 bp, goal032 35/67).

- **№794 (gh#1532, Wave 64 — the gateway G3, the C1 (b) owner
  decision, ADR-0196):** the ledger entry on EVERY gateway
  `tools/call` — `mcp.call` (allowed: the local ok, the local body
  error, the upstream proxy ok/proxy error) and `mcp.call.denied`
  (the gateway refusals: the unknown tool, the not-in-allowlist
  miss), the tool in the record scope, the reason/outcome in the
  detail. The detail rides the ledger schema's `args_hash` BY
  CONSTRUCTION — the arguments and the reasons never appear in the
  journal in the open form (the ledger must not become a leak
  channel; the pin replays a `LEAKME` argument and reads the stored
  record back). The actor is the gateway process; the floating
  session label extends the record with G2 (#1481) — the
  `label=pre-G2` marker is the explicit extension point, not
  silence. The 2-pin battery verified RED-before via the
  hooks-stash run (форма №772): 2/2 failed on the pre-№794
  hooks (no mcp.* records); the 5-call session (3 allowed / 2
  denied) leaves 5 signed records, `mlog ledger verify --expect-head`
  is GREEN on the export and RED after a one-letter tamper.
  AFFECTS: gateway deployments gain the audit trail the G4 demo
  needs; the stateless builtins' audit-log contract unchanged
  (№268).

- **№799 (gh#1546, Wave 65 — the C3 (a) T2 owner decision, the maturity
  tiers):** the parked builtins refuse at CHECK time — the tier-`parked`
  rows (the №796 census) compile ONLY under the owning feature
  (`vision`/`voice`/`video`/`candle`; 11 rows ungated before — the vision
  family, `reflex_generate`, the №526 erasure rows), and a build without
  the feature answers `mlog check` with
  `[BUILTIN_PARKED] <name> requires --features <f> (parked: <basis>)`
  instead of the old split (the ungated rows passed check and refused
  only at the call; the gated rows read the lie UNDEFINED_FUNCTION).
  The always-compiled parked table (`src/builtins/parked.rs`) is the
  name/basis SSOT — the parity lock pins table ↔ rows set-equality over
  the registry SOURCE (the №467 two-locks discipline; the gated rows
  ride the source metric only, the REFERENCE 526-row surface and the
  typed floors unchanged). The run/serve gates block on the same
  finding (№523, the position-0 stamp), the VM compile path stamps the
  coded `[BUILTIN_PARKED]` origin (№479 shape), the TW eval/REPL lane
  reads it branchably through `try r.error.code` (№413 —
  `split_origin_stamp`); the environment-parked exception
  (`reflex_bpe_load`, no feature to gate with — the never-Ok handler,
  №717/№195) stays registered. The tier-`experimental` calls now WARN
  on check (`[BUILTIN_EXPERIMENTAL]`, exit-0 — never an error; the
  №523 blocking gates never see it). The 8-pin battery (форма №772)
  ran against the pre-№799 main: 6 behavioral REDs (the ungated parked
  calls PASSED check, the gated name read UNDEFINED_FUNCTION, no
  warning, the run/compile lanes didn't name the contract) + 2 guards
  (stable calls clean, truly-unknown stays UNDEFINED_FUNCTION); the
  parity lock caught the paren-in-comment scan hazard and
  the environment-parked exception by construction. AFFECTS: programs
  calling parked builtins stop passing check in the default build
  (the fix: the feature build, or the documented alternative); the
  №483 media-handle pin now reads registry ∪ parked (the names stay
  in the language); the №526 erasure pair rides the `voice` build with
  its rows — a build without `voice` collects no prints, so the erasure
  right holds by construction (the voice CI job carries the pair's
  tests; the dead-code debt floor is untouched). The office e2e proxies
  (probe021cron/probe021qp/probe318qp) check clean; the leak-suite
  classes unchanged (cat_a precedes the parked consult).

- **№800 (gh#1547, Wave 65 — the C5 (b) V3 owner decision, ADR-0197):**
  `run`/`eval`/REPL execute on the VM — the single user-command executor
  (the same executor `serve` and `mcp-serve` already use; TW becomes the
  REFERENCE ORACLE: it stays in CI — the crosscheck, the diff-fuzzer, the
  doc-tests — and behind the transitional `--reference` flag, which
  announces itself with the loud banner and answers `[backend:
  reference-tw]` under `METALOGOS_BACKEND_ECHO=1`; two release windows
  per ADR-0197 §4.3). The VM run lane carries the SAME gate set as the
  TW run path — the shared `enforce_run_gates` body (Category A + the
  №523 semantic gate over the statically-import-merged declarations):
  the №535 pins caught the VM lane refusing a missing import with a raw
  io error and no №523 stamp — a gate divergence, fixed by the shared
  body, not a wording nit. The REPL session is a persistent VM: the
  session chunk compiler (`Compiler::with_session` + `Vm::run_chunk`)
  compiles each line against the live session tables — globals keep
  their slots, patterns/learnables/rules join them, memory persists
  (the phase72 carry-over pinned on the VM lane; the cross-line pattern
  call, the redefinition shadowing, and the loud `[UNDEFINED_VARIABLE]`
  on unknown names — the silent Unit read in `LoadGlobalByName` is
  retired on BOTH loops, the №780 one-contract-both-loops form).
  `mlog eval` scores through the VM harness (`Vm::run_eval_blocks` — the
  same `call_llm` path a user call takes; the aggregation lives in the
  shared `EvalResult::from_examples`, one body for both backends).
  Programs where TW and VM diverged now behave in `run` as they do in
  `serve`; the examples corpus reproduces its `.expected` outputs
  through the VM lane unchanged (191/191 runnable). AFFECTS: consumers
  running `mlog run` on TW-specific behavior — the `--reference` lane
  keeps it during the transition; a doubly-invalid program (an unknown
  function AND an unknown variable) surfaces the compile-time
  `[UNDEFINED_FUNCTION]` first on the VM lane, the reference lane keeps
  the TW runtime order (the №535 pins state both lanes; the
  single-defect classes — undefined variable, undefined function —
  refuse with the same codes on both). The flip SURFACED one
  pre-existing VM defect: the self-host parser self-parse
  (ParseWhile→ParseBlock→ParseStmt→ParseIfStmt, deep) replaces a
  pattern-call result with an AST string — the №197 bootstrap rides the
  `--reference` oracle lane until the repair (tracked gh#1555; the
  corpus unaffected; the №762 (a1) name-miss soft fallback and the №287
  doc-test broken-example code follow the contract — loud
  `[UNDEFINED_VARIABLE]` on both).

- **№801 (gh#1548, Wave 65 — the C1 (b) G2 owner decision, the gateway
  pilot, ADR-0196):** the MCP gateway carries a FLOATING session label
  (LIO current-label form) — one current label per agent session that
  the upstream tool classes move. The policy is a first-class `.mlog`
  declaration: `mcp_policy { tool "upstream.name" source "private";
  tool "upstream.mail" sink "public"; }` — a Source entry JOINs its
  label into the session on a successful call; a Sink entry demands the
  session label FLOW TO it before the call (`Label::flows_to` — the
  conf ≤ required, the integrity ≥ required, the consent scopes cover;
  Poisoned never flows). A sink refusal is a TYPED error to the agent
  (`[SINK_CLEARANCE]` naming the tool, the requirement and the current
  label) + the №794 ledger record; the ONE downgrade path is the owner
  grant — `grant_issue(scope "mcp:<tool>", class "once")` unblocks the
  call EXACTLY ONCE (the grant table consumes it; the second call
  refuses again). The session label rides every `tools/call` result
  `_meta` (`metalogos.dev/session-label`). `mlog check` validates the
  policy (an unknown Label-annotation word or a bare local-namespace
  tool name refuse with `[MCP_POLICY]`). Conservative posture: an
  upstream tool WITHOUT a policy entry passes UNCLASSIFIED (the №793
  posture — the pilot is opt-in; the startup prints the loaded-entry
  count, the first call of an unpoliced tool prints a one-time loud
  note). AFFECTS: gateway operators — the policy-less deployments
  behave exactly as №793; the classified ones gain the inter-call data
  flow the LIO model demands. The per-connection label split for the
  network transports rides with the http session work (the stdio
  transport IS one session; the pilot documents this).

- **№803 (gh#1550, Wave 65 — the C2 showcase, the gh#1486 registry task):**
  the leak-suite showcase reads in ENGLISH — the `// Threat:/Attack:`
  headers of `examples/leak/*.mlog` and the explanatory parts of the
  `EXPECTED: <CLASS> — <scenario>` lines are translated (66 .mlog + 47
  .error files); the failure CLASS before the em-dash is byte-identical
  (the runner compares ONLY the class — `run_leak_suite` green after the
  translation, the corpus BLOCKING gate intact). The program payload
  strings (the realistic PII/secret fixtures the ok_*.expected outputs
  pin) stay verbatim — they are behavior, not comments. The showcase is
  the first-screen surface for the C2 positioning (№804 rides this). The
  documented acceptance: no Cyrillic in the comments/explanations of
  `examples/leak/` (the payload strings are exempt by the naryad's own
  wording). security: no — the refusal classes, the runner comparison
  and every gate input are unchanged; a text-only translation. AFFECTS:
  none — the suite output, the classes and the .expected pins are
  identical; the English text makes the showcase legible to the
  non-Russian-speaking AI-agent audience (the C2 decision).

- **№804 (gh#1551, Wave 65 — the C2 positioning, the gh#1485 registry
  task, the owner decision 23.10):** the first screen carries ONE
  formulation across the three entry points — **README**, the GitHub
  description (prepared for the owner's verbatim — the repo-settings
  act) and **llms.txt**: *"A language for AI agents where data cannot
  reach actions it is not cleared for: secret leaks, injection and
  unauthorized irreversible actions are rejected before the program
  runs."* The irreversible-actions claim is PINNED — the static
  `[IRREVERSIBLE_NO_GRANT]` refusal with the leak-suite test link
  (n18). «A Taste» is now the n02 showcase: the leaking program, the
  REAL `mlog check` output (shot from the release binary v0.33.2,
  sha256 48b98167…) and the `redact(…, "secrets")` fix that passes;
  Quick Start's first command is `mlog check
  examples/leak/n02_secret_respond.mlog`; the new «What it stops»
  section lists the five refusal classes, each linked to its pinned
  negative. **Metrics moved to [docs/metrics.md](docs/metrics.md)** —
  the full generated table (the one honest place, + the new Leak Suite
  row: 48 negatives / 27 positives); the README front page carries
  three rows max (version, leak suite, the pointer); `gen_metrics.py`
  and `tests/readme_consistency.rs` re-pointed (the size/count rows are
  checked in the new place). Honest status preserved verbatim. security:
  no — the surface text and the metrics placement; every promise on the
  first screen is pinned by a test or a leak-suite negative, nothing
  new promised. AFFECTS: the README-facing docs only; the GitHub
  description/topics flip is the owner's repo-settings act (the
  prepared string sits in gh#1551).

- **№806 (gh#1562, Wave 66 — the audit-23.10 B1, the secret-comparison
  recipe):** the unkeyed `hash_only` redact policy no longer DECLASSIFIES —
  the fingerprint keeps the source confidentiality and restores only
  integrity (the №327 decision lift survives on non-secret data: the
  error-code branching form stays legal). The hole being closed: the old
  contract shipped a 64-bit UNKEYED SHA-256 prefix labelled `public` — a
  low-entropy secret (a PIN, a short code) is brute-forced offline in
  seconds, so the sanctioned recipe was a leak channel dressed as hygiene.
  The changes: (1) `hash_only` now emits the FULL 256-bit hex digest
  (`[HASH:<64hex>] hex`, was `[HASH:<16hex>]`) and its registry target is
  `private`; (2) the NEW `hash_keyed` policy — one-way HMAC-SHA256 under
  the process-scoped `METALOGOS_REDACT_KEY` (`[HMAC:<64hex>] hex`,
  process-scoped comparability, declassifies to public; the missing/empty
  key is a LOUD fail-closed refusal, never a silent unkeyed fallback — a
  one-word env-keyed design was rejected because the check-time and
  run-time environments could diverge and the static label would lie,
  №525); (3) the static label mapping (semantic.rs) reads the amended
  registry — `hash_only` keeps the input conf and restores integrity,
  `hash_keyed` lands at bottom; (4) the secret-comparison recipe in the
  `UNTRUSTED_DECISION` refusal, limitations.md and threat-model.md is now
  `ct_eq(provided, expected)` (constant-time) or the keyed fingerprint on
  BOTH sides; (5) ADR-0154 §10 amended in place (the two-word form keeps
  the static label truthful with zero env dependence). Consumers: the
  dogfood contour migrates to `hash_keyed` (l1_dogfood, l1_redact), the
  leak suite gains the `n806_unkeyed_fingerprint_respond` refusal and the
  `ok_806_secret_keyed_fingerprint_respond` fence (the pre-№806
  `ok_781_secret_hash_only_respond` is retired — the unkeyed digest
  respond is a leak now), the office's `hash_only`-BOTH-sides comparison
  keeps WORKING (deterministic comparability is preserved) but its
  digests stop declassifying — migrate the egressing fingerprints to
  `hash_keyed` with a configured process key. RED-before (форма №772):
  the n806 leak-suite refusal was RED on the pre-№806 revision (the
  program compiled and the digest flowed to respond) — the fix turns it
  into the pinned refusal; the keyed refusal/comparability pins went
  RED (arity/shape) before the implementation landed. security: YES —
  this is the B1 security fix; the refusal surface widens (fail-closed),
  no gate, allowlist or taint floor moves; the honest-boundary dictionary
  is untouched. AFFECTS: programs egressing unkeyed `hash_only`
  fingerprints of secret-derived values now refuse at check/run — use
  `ct_eq` for comparisons or `hash_keyed` with a configured
  `METALOGOS_REDACT_KEY` for public fingerprints; digest outputs are
  longer (64 hex) and `[HMAC:…]`-shaped for the keyed policy.

- **№807 (gh#1563, Wave 66 — the audit-23.10 B2, the Secret zeroize
  discipline):** the Secret material no longer lands in plain unsecured
  String copies on the crypto/masking paths — `hash_password` and
  `verify_password` BORROW the Secret (argon2 reads the Zeroizing buffer
  directly), `redact` reads it in place, and `hmac_sha256` now accepts
  the opaque Secret as the KEY (the recommended path — the material never
  leaves the Zeroizing buffer; the String key stays accepted for backward
  compat, both borrowed). The same-class find: the CalDAV session
  credential (`cal_connect` pass) lives in a `Zeroizing<String>` field —
  the copy is unavoidable there (the session stores it for the connection
  lifetime), so it round-trips inside the zeroizing buffer and is wiped on
  drop. The classification rationale (builtins_classification.rs) records
  the arg0-Private recommendation for `hmac_sha256` (the REFERENCE row
  follows the registry regen). The zero-copy invariant is pinned
  STRUCTURALLY (the №604 source-scan form — a heap copy is not observable
  at runtime, the source IS the observable): the named sites must not
  carry `as_str().to_string()` on the Secret arms. RED-before (форма
  №772): `hmac_sha256(secret("K"), msg)` was run against the pre-№807
  revision — the static check passed (the opaque posture) and the run
  refused LOUD (`hmac_sha256() key must be String`); the pins flip that
  refusal into the working contract on BOTH backends (the digest is
  carrier-independent: Secret and String keys produce the identical tag).
  security: YES — the B2 fix; the memory posture only tightens (fewer
  unsecured copies), no gate, allowlist or taint floor moves; the
  honest-boundary dictionary is untouched. AFFECTS: none functionally —
  every previously-working call keeps working; programs passing a Secret
  key to `hmac_sha256` (previously refused at runtime) now work.

- **№808 (gh#1564, Wave 66 — the audit-23.10 C5-b V4, the TW-oracle
  boundary gate):** the C5 (b) contract is now ENFORCED by a blocking
  CI gate — the VM executes everything a user runs; the tree-walking
  interpreter (TW) is the reference oracle. `scripts/ci/
  tw_user_path_gate.py` scans the user-path sources (`src/lib.rs`,
  `metalogos-server/src/server.rs`, `metalogos-server/src/bin/mlog.rs`)
  for the TW-entry calls (`run_program*`, `eval_program*`,
  `execute_route_body`) and demands an explicit `[TW-ORACLE]` mark in
  the source or a checked-in baseline row (`tw_user_path_baseline.txt`,
  `path :: anchor :: reason`); a new unmarked TW entry is RED
  (fail-closed: an unreadable file or a missing baseline is RED too —
  №525). The inventory at the landing: 9 entries, ALL marked in-source
  (the oracle runners `run_program`/`eval_program` + the `--reference`
  lanes in the CLI + the serve `interpreter` opt-out dispatch + the
  in-file test harnesses) — the baseline is EMPTY BY DESIGN (the marks
  ARE the documentation). The gate self-test (the №535 form) pins the
  shipped GREEN and the synthetic unmarked RED, wired as the new
  blocking job `tw-oracle-gate`. The crosscheck report wording moves to
  «VM vs the TW oracle» (the step name and the crosscheck header; the
  JOB names are untouched — the branch-protection contract, №471).
  `METALOGOS_SERVE_BACKEND=interpreter` stays transitional with the
  ADR-0197 banner (unchanged — the same term as `--reference`).
  security: no — the gate widens the refusal surface only (a CI
  boundary), no runtime gate, allowlist or taint floor moves. AFFECTS:
  none at runtime; a PR adding an unmarked TW entry now fails CI with
  the named remedy.

- **№809 (gh#1565, Wave 66 — the C3-a T3 levels, ADR-0199):** the
  typed-signature metric reads by MATURITY TIER — the PRIMARY
  definition is the stable-tier share (typed∩stable / stable:
  363/483 = 75.15% at the definition date); the full-registry figure
  (372/526 = 70.72%) stays reported beside it with its own untouched
  floor (ADR-0179), and the new stable floor
  (`type_signature_stable_baseline.txt`, 7515 bp) moves only up (the
  №757 procedure). The REFERENCE §6 index reads by the same levels —
  stable is the primary reference, experimental the opt-in tier,
  parked the appendix (the counts pinned by the new
  `reference_consistency` test: the headline triple, the per-tier
  headers, every row in its own tier section, the name sets equal to
  the registry per tier). The four feature-gated pillar jobs
  (candle/vision/voice/video) move out of the every-push CI into the
  `parked-features` lane (nightly + PR path filter on the pillar
  catalogs + dispatch) as the NIGHTLY health signal — the jobs stay in
  the every-push CI until the owner's protection toggle (the branch
  protection requires their four contexts; removing the jobs made
  every PR unmergeable — the episode caught it on the first head, the
  revision is in ADR-0199 §4), and the soft release rule lands in
  docs/maintainers.md: a red nightly does NOT block a tag but is
  recorded (the auto-filed fix issue + the CHANGELOG/limitations
  line). security: no — a docs/metrics/CI-lane re-aim; no gate loses
  its verdict power (the general floor is untouched, the new floor
  only adds). AFFECTS: the primary metric figure changes definition —
  both figures are quoted side by side from now on; the parked CI
  latency leaves the per-PR path.

- **№810 (gh#1566, Wave 66 — the C4-a T2 owner decision):** docs/REALITY.md
  is the CURRENT TRUTH only (≤30 KB, machine-pinned) — the §6.x
  wave-record history moved VERBATIM to the monthly archive
  docs/reality/2026-09.md + 2026-10.md (50 sections, the
  reconciliation figure is machine-pinned by
  tests/naryad_810_reality_split.rs — «закрытое не удаляется молча» is
  preserved by the transfer). REALITY.md carries the generated stub:
  the verbatim machine-truth block (the same collect_machines SSOT the
  records use) + the archive map (the map is checked against the
  archive files). The wave-record generator gains `--archive`: the
  records APPEND to the record-run month archive (idempotent) and the
  stub refreshes (scripts/gen_reality_current.py, --check is the CI
  gate). The language-lint allowlist carries the two archive files
  (the Russian prose is the RECORD content — the owner decision's
  sanctioned growth). security: no — a docs-layout change; the gates
  only strengthen (the size ceiling, the reconciliation counter, the
  map check are new only-up pins). AFFECTS: the wave records append to
  docs/reality/ from №810 on; docs/REALITY.md regenerates via
  gen_reality_current (≤30 KB enforced); the README links are
  unchanged (the current-truth entry stays the entry).

- **№811 (gh#1567, Wave 66 — the audit-23.10 B3, SSRF):** the V6 arm of
  the SSRF guard covers the two remaining IPv4-smuggling shapes — NAT64
  `64:ff9b::/96` (RFC 6052) and the legacy IPv4-compatible `::a.b.c.d`
  both UNFOLD and re-check through the V4 branch (the parity with the
  №261 mapped-form fix: `64:ff9b::a00:1` is 10.0.0.1 behind a NAT64
  gateway and used to pass as a "public" V6), and the documentation
  range `2001:db8::/32` (RFC 3849) is blocked outright (nowhere-
  routable; an egress there is a configuration against the operator).
  The special forms keep their standard lanes: `::` (unspecified) and
  `::1` (loopback) stay on the standard checks — the unfold requires a
  nonzero sixth segment (unfolding `::1` would produce `0.0.0.1`, which
  the V4 branch does not block). The №261 regression pin moves
  `2001:db8::1` out of the public set (the honest contract change, the
  replacement pin rides the neighboring routable prefix). Documented in
  threat-model.md and REFERENCE.md. RED-before (форма №772): the new
  unit pins ran against the pre-№811 revision — the NAT64/compatible
  private forms passed the guard (RED), the fix turns them into the
  pinned blocks. security: YES — the B3 fix; the block-list only
  widens, allowlist-конвенции №259 не тронуты. AFFECTS: programs
  egressing to `64:ff9b::…`/`::a.b.c.d`/`2001:db8::…` addresses now
  refuse with the loud `SSRF guard` error (they were silent passes
  before).

- **№812 (gh#1568, Wave 66 — the audit-23.10 B4, the assert crash):**
  `assert_contains` never PANICS on non-ASCII — the failure preview
  slices by CHARS (the byte slice `&haystack[..min(len, 80)]` crashed
  with `byte index is not a char boundary` on a Cyrillic haystack past
  80 bytes: the assert refusal was a program crash), and the ARGUMENTS
  in the error text go through the masking pipeline (`redact(…,
  "secrets")` — a token-like secret in the needle/haystack lands in the
  error MASKED: the error text is data-controlled). The char-safe
  preview helper carries an explicit ellipsis on truncation. RED-before
  (форма №772): the Cyrillic pin was run against the pre-№812 revision
  and PANICKED (the byte-boundary crash) instead of returning the
  refusal — the fix turns the crash into the pinned refusal. security:
  no — a crash-to-refusal fix plus redaction of error text; no gate,
  allowlist or taint floor moves. AFFECTS: programs relying on the raw
  (unredacted) error preview of assert_contains now see the masked
  form; the ASCII refusal shape is pinned unchanged.

- **№814 (gh#1579, hygiene — the wave-record PR-citation class, issue
  #1577):** the wave-record generator VERIFIES the PR citation — the
  squash-merge tail `(#N)` is a candidate, not a fact. A 404 (an issue
  number in the tail — the #1577 message-convention class), an unmerged
  record, or a missing tail resolves through the commit-association
  lane (exactly one merged PR) and the record carries a VISIBLE
  tail-ref-correction marker; unresolvable stays a loud `PR unresolved`
  marker — the citation is never an unverified number riding the audit
  trail, never a guess. The merged §6.65 record is corrected in place
  (the №762.1 form): №802 cites PR #1557 and №803 cites PR #1558 (the
  record run had swallowed the issue-number tails #1549/#1550). The
  acceptance ran live: the generator on wave 65 cites #1557/#1558 with
  the correction markers, no hand-editing. security: no — a
  reporting-path fix in a read-only generator; the weekly form
  (C4-a T1) is unchanged. AFFECTS: the wave records cite verified PRs;
  the convention breaks stay visible in the audit trail.

### Removed

- **№827 (gh#1602, Wave 68 — the owner decision 27.10):** the
  `waker.yml` keep-alive workflow is removed. The production office
  service runs on a paid Render plan and does not sleep, so the
  10-minute `/health` ping had no job left; its blocking prod probe
  only painted `main` red when the free staging instance failed to
  start. Instance health is now Render's own Health Check (`/health`,
  configured by the owner in the Render dashboard). security: no —
  a CI keep-alive ping, no product surface.

- **№786 (gh#1484, Wave 63 — the grant-application sweep, the owner's
  directive 23.10, repeated 24.10):** the funding-programme mentions
  (the milestone M1 is closed by the owner — #1135) are swept from
  REALITY (17 lines) and PLAN-SUMMARY (6 lines); the sweep itself is
  documented in the current REALITY section (closed milestones are not
  silently deleted — the record keeps the fact, drops the programme
  names). The recurrence root — every wave dispatch re-planting the
  stale funding-programme owner gate — is closed by the NEW blocking
  docs-metrics step `scripts/ci/no_stale_grant_refs.py` (git grep over
  the doc surfaces; red on an artificially planted mention —
  self-tested). The language capability grants (`grant_issue`,
  `src/grants.rs`, ADR-0167) and the Open Collective sponsorship are
  untouched. AFFECTS: documentation only — no runtime, grammar, or
  registry surfaces.

### Fixed

- **№756 (gh#1408, Wave 57 — the two lane divergences from the №751
  (α)-walk):** the TW/VM lanes answer ONE contract on both discoveries.
  (1) `resolve_skill_index`: the VM lane answers the dept-qualified
  type_name and carries the truncation field now — `CompiledSkillIndex`
  carries `truncation` through the compile (bytecode.rs; the compiler
  maps the declaration's `TruncationMode`), the VM arm inserts the field
  in the TW spelling (`whole_skill_only` / `truncate_at_boundary`) and
  answers `format!("SkillIndex_{dept}")` (was: the fixed `"SkillIndex"`,
  the truncation TW-only — the ADR-0075 divergence the №751 walk pinned).
  (2) The absent-field READ is LOUD on both lanes (the ADR-0194 package
  A = (c) loud-fail contract): the VM `GetField` `Unit` fallback is
  removed on BOTH execution loops (`run()` and `execute_code_inner()`
  — was `unwrap_or(Value::Unit)`, the error masked as a value — the
  №604 class-4 shape); the `get_field` Err propagates — the same
  string the TW
  `FieldAccess` always propagated (`field 'x' not found on struct` /
  `cannot access field 'x' on non-struct value (...)`). The registry
  comment carries the resolution anchors; the №751 divergence pin
  re-pinned to the parity fact (форма №740, same-PR). Pins:
  `tests/naryad_756_lane_divergences.rs` (8: the dept-qualified
  type_name probe + the negative fixed-form control, the truncation
  both-modes + the absent-stays-absent, the absent-field and non-struct
  loud refusals on both lanes, the present-field happy path). AFFECTS:
  the VM lane semantics — absent-field reads now refuse loudly (the
  programs that relied on the silent Unit read the loud refusal; the
  TW lane unchanged); the `resolve_skill_index` answer gains the
  dept-suffixed type_name and the truncation field on the VM lane (the
  shape parity).

- **№762 (gh#1415, Wave 58 — the census/classification of the 129 quiet
  `unwrap_or(Value::Unit)` fallbacks in `vm.rs`, the №756 scope tail):**
  the loud operand contract (the ADR-0194 package A = (c) RATIFIED) is
  pinned for the reachable class. The census split every site by the
  live handler/opcode facts: **(a) 47 pinned, semantics unchanged** —
  (a1) 39 loud-through-result (the fallback feeds an evaluator that
  rejects `Unit` loudly on every path: the binop family through
  `eval_binop`'s coded mismatch, `Contains` on the run loop through
  `eval_contains`, `CmpGt/Lt/Ge/Le` through the `eval_cmp` catch-all —
  only `CmpEq`/`CmpNe` answer `Unit` silently, `GetField` through
  `get_field` (the №756 contract), the rule-value `FieldAccess` read,
  and `LoadGlobalByName` — every valid-program emission is a
  name-reference position whose miss lands the builtin's own loud
  `model not declared` text, which stays UNCHANGED); (a2) 6
  documented-soft language semantics (the `IndexAccess` in-value
  lookups — out-of-range index / missing key are valid-program events
  with the documented `Unit` answer); (a3) 2 internal register-stack
  pops (`EndValueExpr` over `value_registers`). **(b) 82
  reachable-masking — fixed loud:** the `.mbc` delivery path
  (`deserialize` — no version/integrity gate) makes stack underflow and
  slot-bounds misses reachable via a desynchronized or corrupt program,
  and the silent `Unit` fallback flowed into results, stores, branches
  and memory as the №604 class-4 shape (the error masked as a value) —
  every class-(b) site now refuses loudly naming the instruction and the fact
  (`VM: stack underflow at …`, the slot/range/name spelled out). Zero
  observable change on valid bytecode: the compiler guarantees every
  operand (`expr; Return` in every context, the №582 epilogue, the
  constructor-paired slots), all fixes are line-count-neutral (the
  registry `vm.rs:` anchors hold). Pins:
  `tests/naryad_762_unit_fallback_census.rs` (7: the run-loop and
  inner-loop underflow pins with the named refusals, the slot-bounds
  pins, the inner `Return` + the body-epilogue tail, the (a1)
  loud-through-result behavioral pins (Add/CmpGt/GetField refuse with
  the evaluator's text, no underflow text), the (a2) documented-soft
  pins, and the happy-path negative controls). AFFECTS: the VM lane
  semantics — a desynchronized or corrupt `.mbc` program now fails
  loudly at the first operand-stack violation instead of silently
  producing `Unit`-poisoned results/branches/memory (the programs that
  relied on the silent fallback read the loud refusal); valid programs
  are byte-for-byte unchanged; the TW lane unchanged (structurally
  incapable of stack underflow).

- **№765 (gh#1418, Wave 59 — the S-1b XSS closure, the external audit
  §3 H-1):** the let-form of render() is no longer a gap. The audit
  scenario — `let page = render("Comment", j.author, j.text)` →
  `respond_html(page)` — produced no finding and the raw slot carried
  the request data straight into the page. Three layers move together:
  (1) `builtin_render` computes the RAW-slot param names from the
  template body (the SAME `template_scan::raw_param_positions` the
  audit taint layer and the html_label walk read — ONE scan, four
  consumers) and requires `Value::Html` for them exactly like the safe
  slots since №695 — the fail-closed refusal `RAW_SLOT_NOT_HTML` names
  the slot and the fact (the ungated dynamics: the VM lane refuses in
  the real pipeline; the TW lane's checked programs hit the static
  implicit-coercion gate first). (2) `html_label::walk` — the BINDING
  walk resolves the raw positions through the shared resolver (a
  StringLit name or a bare Ident that names a DECLARED template —
  render is a name-reference builtin; a variable-carried name stays
  the conservative unknown, the №685-E pin) and answers
  `Labeled(String, Untrusted)` when a labeled argument sits at a raw
  position; the sink walk keeps its №685 contract. (3) render is OUT of
  the blanket sanitizer lists (the binary predicates: the memory-taint
  layer, the interp state walker) and POSITION-AWARE in the label
  engines (the raw map decides per slot: the all-escaped template keeps
  the sanitizer semantics — the №685-D/n98 pins; any raw slot conducts
  the join of its raw args; the dynamic name stays the conservative
  conductor). The check path seeds the raw map now (the same
  re-seeding discipline the audit entries follow). Pins: NEW
  `tests/naryad_765_s1b_let_form.rs` (7: the audit scenario flagged on
  BOTH surfaces — red before the fix, green after; query_param into a
  raw slot; render→memory→respond_html keeps the taint fact; the TW
  static coercion refusal + the VM runtime backstop refusal; the Html
  producer composition runs; the render(render()) negative; the
  all-escaped interlock) + 2 inline units in http.rs (the raw-slot
  backstop at the pub(crate) boundary). The interlocks hold: №685
  24/24, №695 4/4, №686 3/3, №376 4/4, №448 11/11, №201 8/8,
  template_integration 8/8, n98 8/8. AFFECTS: programs that relied on
  render-as-sanitizer for RAW slots now get findings (the static layers)
  and runtime refusals (a plain String in a `{{{ }}}` slot); the
  all-escaped templates keep the previous behavior byte-for-byte; the
  escape_html status untouched; the raw semantics of №685/№695/№696 not
  regressed.

- **№766 (gh#1419, Wave 59 — the diff-fuzzer H-2, the external audit
  §3):** the fuzzer's statement generator is RECURSIVE now — the old
  `gen_stmts` carried a `_depth` parameter it never used, and the
  «form A inside form B» class (№369: the nested match; gh#1336: the
  nested each — both found by CONSUMERS, not by the fuzzer) was
  unmeasurable by construction. (1) `gen_stmts(depth)` at every level
  picks among the SIX nesting forms (if/else, bounded while, each,
  each-with-index, match, the flat pool) and recurses into the body at
  depth-1 (the audit's п.1). (2) Every generated block appends a
  deterministic TAG to the trace accumulator the program prints in its
  return — any silently dropped block is a TW/VM divergence by
  construction, not by luck (п.2). (3) The each-in-match-arm pins land
  in `tests/n1336_vm_each_in_if.rs` (the parity + the liveness — the
  accumulator is read AFTER the match; the each-with-index form too) —
  the last row of the audit's §6 п.3 list (п.3). (4) The nesting-pairs
  metric (форма-родитель × форма-ребёнок): {top, if, while, each,
  each_idx, match} × {flat, if, while, each, each_idx, match} = 36
  cells, all reachable — the achieved coverage is 36/36 over the seed
  sweep; the floor lives in
  `scripts/ci/fuzz_nesting_pairs_baseline.txt` (the only-up ratchet:
  a lower count is a regression, a higher count DEMANDS the bump in
  the same PR) and the CI log carries the 6×6 table (п.4). (5) The
  mutation probe: the Each-arm sensitivity is pinned BY CONSTRUCTION —
  the sweep programs whose output carries an each-trace tag diverge the
  moment the Each arm stops emitting code; the probe asserts the
  sensitive share (≥40% of the executed sweep) — disabling the Each arm
  in `compile_stmt_with_locals` sends the run red (п.5; the manual
  procedure — the №503/№618 form — in the naryad report). The
  generator's liveness holds: the undefined-ident error-class lane
  scaled 1/8 → 1/48 per leaf pick (the recursion multiplies the leaf
  picks; the PER-PROGRAM refusal rate returns under the №621 sweep
  floor — the lane keeps its coverage). The audit's «Не делать» held:
  no new fixed fragment — the coverage is systemic. Pins: the fuzzer
  suite 13/13 (the legacy n1336 shape-production probe re-pinned to the
  pairs (if, each)/(while, each)), n1336 9/9. AFFECTS: none for the
  production code — the change is the test generator + the metric (the
  compiler/VM semantics untouched).

- **№768 (gh#1421, Wave 59 — the audit 21.10 §3 M-2 + L-1):** the 0.31
  narrative lands (the third carry — the audit's «это пять минут
  работы») and the mirror counter stops lying about its CWD. (1) The
  version-line narrative — «0.31 was never released; its goal folded
  into 0.32.0» — a Note subsection in the historical [0.32.0] section
  (the gh#1350 Note form; the tags confirm: `v0.30.1` → `v0.32.0`, no
  `v0.31.*`) + the same note in the REALITY §6.29 record (the
  historical sections gain a line, not a rewrite). (2)
  `scripts/ci/mirror_counter.py` builds the scan root from
  `Path(__file__).parents[2]` — the old CWD-relative glob printed a
  bare `0` (exit 0) when run from `scripts/ci/`, a false green for a
  metric that can only move DOWN (the audit L-1, reproduced 1-в-1);
  now any CWD prints the honest 6 (verified from the root,
  `scripts/ci/` and `scripts/`), and a missing `src/` tree fails LOUD
  (exit 2) — never a quiet zero. The CI invocation from the root is
  unchanged; the metric value (6, threshold 6) is untouched.

- **№772 (gh#1435, Wave 60 — the prod bug gh#1430, the fosved-acc
  АСС-1.9 finding on v0.33.2):** catching a pattern-call error with
  `try` no longer wipes the caller's frame locals on the VM. The
  unwound CallPattern left its CallFrame installed (the `?` propagation
  skipped the cleanup the success path performs), so every later
  LoadLocal/StoreLocal resolved against the stale base_bp — the pre-try
  locals read back as Unit and the reported `Unit Sub Float` /
  `Float Sub Unit` killed the route (the fosved-acc site monitor went
  dark during an outage on the serve VM lane; TW kept the values — the
  divergence the consumer found before the machine, the gh#1336 class
  again). (1) The fix: BOTH TryEval arms (the run loop and the inner
  loop — форма №762) snapshot the stack and call_stack depths before
  the guarded code runs and restore both on catch — the same discipline
  `run_deny_handler` (№392) applies to its own body on every path;
  nothing is invented, the loud contract (ADR-0194 (c)) is unchanged —
  the error still prints once and still lands in the result struct; TW
  (the reference) untouched. (2) The pins:
  `tests/n1430_try_pattern_unwind.rs` — 6 tests, 4 RED before the fix
  on unpatched main (the issue's p3 shape 1-в-1 with TW, the consumer
  ms shape, the nested-frames shape, the route-body lane through
  `execute_route_code`) + 2 negative controls that were green before
  and must stay (the inline-builtin boundary, the try success path).
  (3) The differential fuzzer (the bug's ask 3) carries the class shape
  now — `try boom()` → catch → a trace append reading a PRE-try local;
  verified honest: with the fix stashed the fuzzer diverged loudly
  (`Unit + String` on the VM), with the fix it is green 13/13 — the
  class cannot return quietly. AFFECTS: VM programs that relied on
  reading pre-try locals as Unit after a caught pattern-call error now
  see the TW values (the divergence was the defect; TW behavior
  unchanged byte-for-byte; the inline-builtin try boundary unchanged).

- **№776 (gh#1445, Wave 60 hygiene tail — the №771 form):** the
  gate-facts-sync verdict read survives a GitHub API filter flap. The
  `status=completed` runs-list filter answered `total_count=0` on the
  0-side of a server-side flap while the same endpoint without the
  filter showed all five completed GREEN runs (the evidence log in
  gh#1445: 0,0,0,5,5,5,0,5 on identical queries ~20s apart); two main
  runs failed fail-closed INFRA (№525 — the honest behavior, the gate
  did not pass an unreadable state). The empty-list answer is retried
  now within a bounded window (6 attempts × 20s ≈ 2 min; the observed
  0-side of the flap is ~20-80s); the verdict still comes ONLY from
  completed runs, an API exception stays the loud immediate exit 2,
  and the exhausted retries still fail closed — the №525 contract is
  unchanged (the retry never turns an unreadable state into a pass).

- **№783 (gh#1494, Wave 62 — the audit-23.10 A4/S-3+N-2, P1):** the
  query* family no longer loses parameters silently, and `each` over a
  non-List refuses identically on both backends. (1) The params SSOT —
  `bind_params(who, args, idx)` in db_ops.rs now serves ALL SIX db
  functions (query, query_vm, query_row, query_scalar, db_execute,
  db_execute_with_grant): the pre-№783 query* family answered a
  non-List second argument with `Vec::new()` — SILENTLY
  (`query("… WHERE id = ?1", user_id)` bound ZERO parameters and,
  without a mandatory placeholder, passed quietly with the caller's
  data never reaching the engine); the refusal NAMES the `[x, y]`
  form, the №553 loudness contract moves into the SSOT (its pins ride
  the landed wording). (2) The VM `each` over a NON-List answered 0
  iterations silently (ListLen → 0.0) while the TW refused — a live
  TW/VM divergence of the gh#1336 class (a client-string
  `json_body()` made the VM lane skip the loop); both ListLen arms
  now carry the byte-identical TW coded refusal
  (`[TYPE_MISMATCH] each: expected List, got <type>`). (3) The №766
  diff-fuzzer carries the class shape — one in 31 generated iterables
  is a String literal (the refusal-parity shape; the executed-counter
  floors of the №621/№766 probes hold — the form stays rare at the
  statement level). The 7-pin battery (4 functions × both backends +
  the TW/VM each-parity pair + 2 happy-path controls) verified
  RED-before on the pre-№783 engines: 5/7 failed (the quiet Ok
  instead of the refusal, the old №553 wording, the VM divergence);
  the №553 wording pins re-spelled to the landed form. AFFECTS:
  programs that passed a non-List params argument now catch the loud
  refusal (the redact-equivalent `[x, y]` List form stays the legal
  path); the each-over-non-List programs now refuse on the VM lane
  exactly as the TW lane always did.

- **№785 (gh#1507, Wave 62 hygiene tail — the №776 form):** the
  cargo-audit/cargo-deny RED on main (`e77a93c`) — `RUSTSEC-2026-0385`
  (ctutils 0.4.2: the non-constant-time mask generation / `u128`
  comparisons) landed in the RustSec advisory database BETWEEN the CI
  rounds (the same commit graph was GREEN an hour earlier on the
  tokio bump — the base flap, not a diff regression; the №780 diff
  does not touch Cargo.lock). The bump: ctutils 0.4.2 → 0.4.4
  (`cargo update --precise`, the single Cargo.lock hunk) — the
  constant-time comparison stack (the №1002/gh#1211 `ct_eq` contour)
  rides the fixed release. 0 vulnerabilities after (5 allowed
  unmaintained/yanked warnings, the advisory class); the
  0367/0368/0369 zip ignores are untouched (UPSTREAM-BLOCKED, the
  2026-11-17 expiry review stands).

- **№787 (gh#1510, Wave 63 — audit-23.10 C6 I-c):** the integer
  precision-loss boundary — a SQLite/JSON integer f64 cannot represent
  exactly (|n| > 2^53: 64-bit ids, snowflakes) is a LOUD
  `[PRECISION_LOSS]` coded refusal naming the column (the four
  SQLite row-conversion sites — query/query_scalar/query_row on both
  lanes through the ONE shared `value_ref_to_value`) or the document
  path (`parse_json`/`config_load`/MCP tool schemas/the VM LlmResponse
  — the JSON `unwrap_or(0.0)` arm is gone). The exactly-representable
  integers (|n| ≤ 2^53 and the exact powers of two beyond it, e.g.
  i64::MIN) ride the UNCHANGED Float behavior — the boundary battery
  pins 2^53 and i64::MIN as value-comparisons (RED-before: the reads
  returned the distorted 9007199254740992). AFFECTS: programs that
  relied on the quiet round-trip now catch `[PRECISION_LOSS]`
  (try-classified on both backends); re-read lossy columns as TEXT or
  CAST in SQL.

- **№797 (gh#1535, Wave 64 — audit-23.10 B5/D-1, P2):** the AGENTS.md
  stale facts (the file every coding agent reads first) — three
  corrections: (1) the §5 coverage claim ("~59%, not 100%") is
  re-spelled to the current machine truth (the GENERATED index covers
  100% of the registry; the curated prose may lag — the registry grep
  stays the SSOT); (2) the §6 hardcoded "all 110 in a row" ADR count
  is REMOVED per the docs-metrics form (the number is regenerated by
  the machines, never hand-maintained — the header says so); (3) the
  §6 "VM parity with TW … deliberately not implemented (ADR-0105)"
  is re-pointed to the current architecture: the parity is a BLOCKING
  contract, the VM is the single executor of the user commands and TW
  is the reference oracle in CI (`ADR-0197` supersedes the `ADR-0105`
  remainder — the C5 owner decision). The §8.5 site-files audit item
  (secret_label.rs / sql_label.rs missing from the sanitizer-layer
  list) had ALREADY landed with №781's A2 additions — verified, no
  edit needed. CLAUDE.md re-synced byte-for-byte (the №763 identity;
  the agent-index gate green). AFFECTS: documentation only.

- **№798 (gh#1536, Wave 64 — audit-23.10 B6/D-2, P3):** `llms.txt` is
  ENGLISH — the owner's 2026-09-17 "documentation in English only"
  directive is now machine-enforced for the file too. The pre-№798
  state: the file carried Russian prose wholesale (1298 Cyrillic
  letters / 39.4% — the verified scan) while
  `tests/docs_language_lint.rs` walked `.md` files only, so the
  directive was declared but not enforced for the agent-facing index.
  Three halves: (1) the full translation (the structure, the links
  and the machine-readable counters preserved — the agent-index gate
  counters identical); (2) the lint's scanned set gains the explicit
  NON-`.md` agent inputs (`llms.txt`) — the coverage pin + the
  scanner's negative control (a synthetic Russian document is
  flagged, an English one passes; no repo file is mutated); the RED
  form verified honestly: the extended lint against the pre-№798
  Russian `llms.txt` FAILS with the 39.4% verdict (stash-run, форма
  №772); (3) the agent-index gate's two Russian-coupled llms patterns
  (`рабочих .mlog`, `наряды №1-№N`) move to the English spellings —
  a regression back to Russian now trips the gate twice (the missing
  statement + the lint). The `.rs` Cyrillic comments stay OUT of
  scope (the owner's gray-zone decision, the audit's own note).
  AFFECTS: documentation + the lint surface only.

- **№813 (gh#1578, hygiene — the weekly merge-ci-audit exit-2 class, issue
  #1577):** the detective treats a 404 on the squash-merge tail ref as the
  DETERMINATE message-convention finding it is — the association lane
  (`GET /commits/{sha}/pulls`, exactly one merged PR) resolves the real PR
  and the break stays a VISIBLE divergence line (a convention break never
  heals silently); the transient lane (5xx, throttling, network) stays
  fail-closed exit 2 (№525: an unreadable state is not a pass). The first
  trigger: №802/№803 carried the ISSUE numbers (#1549/#1550) in the tails —
  the scheduled run 40309903631 died exit 2 without a single verdict; the
  manual verdict (issue #1577) also surfaced six tail-less merges
  (№762/№762.1/№763/№765/№766/№729-tail) — all eight resolved via
  associations, all heads 13/13 success, zero M-1 findings. The self-test
  carries the resolver cases (exactly-one-merged resolves;
  zero/several/none-merged stay unresolvable). security: no — a
  reporting-path fix in a read-only detective; the required set, the
  fail-closed posture and the report-only boundary are pinned unchanged.
  AFFECTS: the weekly audit reports convention breaks as exit-1 divergence
  lines instead of dying unreadable; the merge-convention tail `(#PR)` is
  enforced by visibility, not by silence.

- **№815 (gh#1580, hygiene — the conflict-marker class):** `llms.txt`
  on main carried a RAW git conflict block (landed with №806/0e4ce59,
  carried through four merges and every gate — the agent-index gate
  covers the machine-generated index lines, not the hand-maintained
  region; the language lint checks the language, not the structure).
  The block is resolved to its HEAD side (the CHANGELOG reference
  line); the tripwire lands in `docs_language_lint` — a stateful
  scanner (the seven-equals separator counts only inside an open
  marker block, so the setext underlines stay content) pins that no
  tracked doc carries conflict markers, and the synthetic pin proves
  the shape is caught. security: no — a docs-hygiene fix + a structural
  tripwire; no gate loses coverage. AFFECTS: the conflict-marker class
  is grounded — the silent pass-through is impossible from now on.

- **№816 (gh#1584, Wave 67 — the #1555 self-parse divergence, the
  release-block):** the bytecode compiler no longer rolls the frame-slot
  counter back after an if/match/loop arm — an arm's `let` bindings stay
  live in the function-level name→slot map (the p30 scoping contract),
  so the rolled-back counter could hand a NEW binding the stack cell of
  a LIVE local: two locals shared one cell and a pattern-call result was
  replaced by an unrelated value deep in the recursion. The #1555
  symptom (`self-host/parser.mlog` self-parse: `get() requires List as
  first argument` in the ParseWhile→ParseBlock→ParseStmt→ParseIfStmt
  chain) is closed — the self-host parser now self-parses identically on
  both lanes, byte-for-byte. The class pin (a new binding after an
  if-arm must not share the arm binding's slot) rides in
  `tests/naryad_816_vm_frame_slot_alias.rs`, and the minimal repro
  joined the differential-fuzzer corpus as
  `tests/fuzz_corpus/seed_frame_slots.mlog` (the №783 form — the fuzzer
  carries the class shape). security: no — a compiler-correctness fix in
  the frame-slot allocation; no label, secret, or sink surface changes;
  the both-lane parity only tightens. AFFECTS: programs with bindings
  after if/match/loop arms stop reading stale neighbor cells; the TW
  lane is unchanged (it never had slots).

- **№818 (gh#1586, Wave 67 — the #1573 silent-500 class):** every serve
  route-handler error now prints ONE unconditional stderr line —
  `[ROUTE_HANDLER_ERROR] <method> <path> -> 500: <error>` — on all four
  Err(e) paths that build the 500 response (the production dispatch and
  the three test-harness probes); no opt-in, the ledger untouched (not a
  security decision). The office webhook class (17 days silent, found
  only by local reproduction) is closed: the failure mode is observable
  from the process stderr alone. The pin spawns the REAL `mlog serve`
  process and asserts the line and the 500 on BOTH backend lanes
  (`tests/naryad_818_route_handler_stderr.rs`; the fixture
  `tests/fixtures/naryad_818_route_error.mlog`). security: no — an
  observability fix; no gate, label, or secret surface changes.
  AFFECTS: serve operators see handler errors in stderr immediately;
  the response bodies are unchanged.

- **№820 (gh#1588, Wave 67 — the #1528 stub-audit follow-up):** the
  `db_insert` registry row stops lying. The #1528 anchor ("no handler on
  any lane; REFERENCE promises without code") was factually STALE — the
  real parameterized INSERT landed with №409 (the lazy-open VM lane) and
  carries the №783 params SSOT: `db_insert(table, struct)` writes
  through src/db_ops.rs and returns last_insert_rowid, special-cased on
  BOTH lanes before the registry dispatch (the №736 query_scalar
  stale-anchor correction class). The row moves out of the `stub` class
  into `db` (arity 2 exact, the `Float` type, the tier stays
  experimental — the №809 only-up floor is not this naryad's call), and
  the registered fallback (the db_execute context-less lane form) now
  answers the loud "no database connection" refusal instead of the
  generic "no handler". The REFERENCE sink row drops the "(registry-only
  stub)" claim; the curated INSERT rows were already accurate and stay.
  RED-before: the registry-honesty pin
  (`tests/naryad_820_db_insert_surface.rs`) fails on unpatched main
  (arity 0 / stub / no handler) while the two-lane behavior pins pass —
  the behavior existed, the record lied. security: no — a metadata/
  honesty sync over an existing, unchanged surface; no label, secret, or
  sink behavior changes. AFFECTS: the tier/arity machines and the
  REFERENCE §6 row describe the shipped surface; wrong-arity calls are
  caught at the semantic gate now (the arity-0 skip is gone).

### Security

- **№780 (gh#1491, Wave 62 — the audit-23.10 A1, P0):** the runtime
  label gates (`LabelJoin`/`SinkCheck` — the №325/ADR-0156 runtime
  twin, the `[SINK_CLEARANCE_RUNTIME]` verdict) execute in the
  pattern/route bodies too. Two halves of one hole: the compiler did
  NOT arm the body's sink sites (`compile_pattern_body_with_locals`
  had no `emit_armed_sink_checks` on `ExprStmt`/`Return`), and the VM
  inner loop (the pattern/route executor) dropped both label
  instructions into the wildcard tail — the serve route bodies (the
  server code) never ran the sink-clearance check, the label env
  stayed unseeded there. The fix: the compiler arms the body sinks
  symmetrically to `compile_stmt_with_locals` (the deny path №392
  included), the VM inner loop executes the label instructions from
  the SAME executors as the main loop, and the main-only instructions
  answer LOUD in a body
  (`VM: instruction <X> is main-only — compiler/VM skew`) instead of
  the quiet skip. The split (inner vs main-only, 46+12) is pinned by
  the exhaustive enumeration test — a new `Instruction` variant
  without a class is a compile error; ADR-0156 §1 carries the
  №780 note (the contract sentence is now the fact);
  docs/wildcard-tail-ledger.md updated. Red-before: 5/6 of the new
  pins fail on the unpatched main (verified by a stash-run). The
  verification sweep also caught the COMPILER half of the hole (the
  body sink sites compiled WITHOUT the armed check at all — the
  debug byte dump) — both halves are closed by this naryad. The
  variable sink arguments carry their local slot (serde-default,
  old .mbc files deserialize cleanly) so the VM reads the value
  behind the name and keeps the typed materialization canon
  (№355/№691: a WorldState handle refuses `WORLD_STATE_PRIVATE`,
  the sst_001 pair, on BOTH backends) even when the twin fires
  before the shared builtin leg.
  AFFECTS (форма №756): programs relying on the missing body-level
  runtime gate (a private label reaching a sink inside a
  pattern/route body) are now refused loudly on the VM — the TW
  static gate always refused; the serve lane now agrees with the
  static verdict. The main-only skew error is a new loud class for
  hand-built bytecode only (the compiler emits none).

- **№781 (gh#1492, Wave 62 — the audit-23.10 A2, P0):** escaping a
  secret is still a secret. The `escape_html` builtin and the ESCAPED
  slots of `render` restored the INTEGRITY of the data but carried an
  unconditional declassification — the result was treated as
  `Public`, so `respond("200 OK", escape_html(env("API_TOKEN")))`
  passed every gate on all three label lanes. The fix (the N2/
  gh#1211 ct_eq form, one shape in every lane): the INTEGRITY is
  restored (the escape still cleans untrusted markup — the №685/
  №695/№765 XSS contract is NOT weakened), the CONFIDENTIALITY is
  the join of the arguments. The lanes: `secret_label.rs` — the
  render/escape_html overrides are removed from `label_of_expr` and
  `binding_label` (the argument join decides); the label lattice
  (`semantic.rs` `label_source` + the `render` position-aware arm,
  `audit.rs` `interp_expr_label` — both engines symmetric) — an
  escaped slot restores the integrity only, a raw slot conducts
  everything (№765 unchanged), an unknown/dynamic template name
  stays the conservative conductor. The audit A5-class stakes:
  `secret_label.rs`/`sql_label.rs` join the risk-review site set
  (№686, the item-10 reminder fires on their diffs; AGENTS.md §8.5
  updated, the self-test set-pin moves 4 → 6). The threat-model
  Boundary 3 carries the corollary verbatim (escape restores
  integrity only, never lowers confidentiality; the redact-policy
  registry stays the only path down). The leak-suite quartet pins
  the hole RED-before (verified on the unpatched main: 4/4 compiled
  — NOT CAUGHT; a stash-run of the mutation probe shows every pin
  losing the SECRET_LEAK class to the PII_EGRESS_* mismatch —
  the pins are hard-wired to the lane); the A2 fences
  (`ok_781_escape_html_untrusted_respond`, `ok_781_secret_hash_only_
  respond`) keep the false-positive side green. AFFECTS: programs
  that sent an escaped env-secret to a sink now catch `SECRET_LEAK`
  at compile time (the redact one-way policies — `hash_only`/
  `secrets`/`all` — stay the legal path); everything else compiles
  unchanged. security: yes — the compile-time gate grew teeth on the
  escaped-secret egress class.

- **№782 (gh#1493, Wave 62 — the audit-23.10 A3, P0):** the executor
  resource limits — `serve` can no longer be killed by a runaway
  program. Three halves: (1) the string capacity — ONE SSOT
  (`src/limits.rs::MAX_STRING_LENGTH`, the old two drift-prone local
  copies in vm.rs/execution.rs are gone); the multiplicative builtins
  (`repeat`, `pad_left`, `pad_right`) check `unit_len * count` BEFORE
  the allocation — the old `s.repeat(1e12)` died in the allocator
  (an abort, not a panic — `spawn_blocking` cannot catch it, the whole
  serve process went down); `join` computes the exact need up front,
  `format`/`word_wrap` carry the loud input/result checks (their
  growth is input-proportional). (2) the native-recursion depth cap —
  the CallPattern recursion (`pattern R(n) { return R(n+1) }`) used to
  overflow the thread stack (in serve the route bodies run inside
  `spawn_blocking` with the 2 MiB stack — the overflow ABORTED the
  serve process); BOTH executors guard the entry now (`[CALL_DEPTH_EXCEEDED]`,
  default 512, `METALOGOS_VM_MAX_DEPTH` is the deployment knob, the
  VM run path executes on a 256 MB thread (the №197 form), the serve
  blocking pool runs 8 MiB stacks). (3) the main VM loop
  (`execute_main_code`) carries the iteration counter mirrored from
  `execute_code_inner` — an eternal top-level loop hung `mlog run`
  forever. The subprocess pin battery (6 tests, both backends) verified
  the RED-before on the unpatched main: the alloc abort + the stack
  overflow, byte-for-byte the audit's symptoms; the `while`-shape pin
  patches the compiled Halt into a self-jump (the top-level grammar
  cannot express the eternal main loop — the patch is the honest
  construction). The new section in docs/limitations.md: the defaults,
  the knob, what the caps do NOT cover (the opaque subsystems'
  allocations). AFFECTS: programs that relied on the unlimited
  recursion/string growth (below the limits the semantics is
  unchanged — crosscheck green).

- **№784 (gh#1495, Wave 62 — the audit-23.10 A5/S-8, P1):** the
  authenticate mock answered `Ok(Unit)` while the REFERENCE row
  promised "In a server context, checks the credentials" — a promise
  with no code behind it (no server user-store exists on either
  backend). Every call now refuses with the coded `[NOT_IMPLEMENTED]`
  refusal naming the real credential path —
  `verify_password(password, hash)` against the stored hash (the
  ADR-0198 loud-failure line); the code joins `ORIGIN_STAMPED_CODES`
  (the №413 mechanics) so `try authenticate(...)` can branch on it.
  The 5-pin battery (the two loud-refusal pins, the two
  validation-refusal regression pins, the try-code pin) verified
  RED-before on the pre-№784 engines: 3/5 failed (the printable Unit
  instead of the refusal; no try code). The REFERENCE row (curated +
  the generated index) is re-spelled honestly, and the regeneration
  surfaced a generator defect fixed in the same PR: the curated-row
  harvest split table cells on EVERY pipe, so the Markdown-escaped
  union signatures (`String\|String`) were cut mid-cell and 17
  generated rows (authenticate, len, to_float, to_int, parse_json,
  call_llm, http paths, send_message, send_photo, render, inspect,
  reflex_metrics, vec_store, vec_search) carried truncated signatures
  with the return-type column riding in as the description; the
  harvest now splits on unescaped pipes and the cell escaping is
  idempotent — all 526 rows render intact (gen_reference --check
  526/526, TODO(doc) 0, curated sections untouched). AFFECTS:
  programs calling authenticate now catch the loud refusal (the
  migration is the named verify_password path); hash_password /
  verify_password untouched.

- **№795 (gh#1533, Wave 64 — audit-23.10 C5-b V2, the C5 (b) owner
  decision, ADR-0197; fix date 2026-10-24):** `mcp-serve` executes tool methods on the VM —
  the SAME compile-path as serve (the strict VM compile at startup; a
  non-compilable file refuses the gateway loudly, where the pre-№795
  TW path silently ignored the non-tool declarations' run errors).
  The capability the switch buys: the RUNTIME label gates
  (`[SINK_CLEARANCE_RUNTIME]`, the №780/ADR-0156 machinery) fire IN
  TOOL BODIES — a private value routed into a sink at runtime is
  refused loudly through the tools/call envelope, where the pre-№795
  TW path answered green (the leak went through; the TW lane has no
  runtime label twin). The invocation rides the new
  `Vm::invoke_pattern` (the CallPattern convention: the JSON args
  bind positionally per the declaration order; the per-request depth
  budget and the backend-tag contract match `execute_route_code`).
  The gates are unchanged: `ServeRouteExecGuard` (№253-А/№259),
  the Category-A startup gate, the allowlist, the exec/env posture.
  The 3-pin battery (the runtime-gate pin + the honest parity pins)
  verified RED-before via the changes-stash run (форма №772): the
  runtime pin FAILED on the pre-№795 TW path (the green answer —
  the leak), the parity pins green. The existing mcp suites
  (№394/№477/№536/№793/№794) stay green on the VM lane. AFFECTS:
  gateway tool files must compile on the VM lane (the strict form);
  the tool bodies gain the runtime label enforcement.

## [0.33.2] - 2026-10-21

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

## [0.33.1] - 2026-10-19

### Changed

- **№707 (gh#1307, Wave 48 — the №700 candidate-наряд):** the extraction
  family moved to the markup5ever 0.39 line — html5ever 0.27 → 0.39,
  xml5ever 0.18 → 0.39, markup5ever_rcdom 0.3 → 0.39.0+unofficial.
  The 0.39 line is the newest COMPILABLE pair: no rcdom release exists
  under markup5ever 0.40 (the №700 reports gh#1286/gh#1288), and the
  0.39 members unify on ONE markup5ever 0.39 graph (verified by
  `cargo tree -e features`) — the double-graph mismatch that broke the
  0.40 attempt disappears by construction. The RcDom/TreeSink
  integration needed NO rework (the surface — parse_document +
  Handle/NodeData walking — is stable across the line). The `html`/`xml`
  features stay off-by-default; DTD/external entities NEVER (XXE-safe by
  construction); the n5/n6 both-backend suites (14+14), the
  rss_decomposition smoke, the conformance goldens, the leak-suite, the
  №465 diff-fuzzer, and the crosscheck are green under the features.

- **№716 (gh#1323, Wave 49 — the gh#1317 resolution):** the
  `UNTRUSTED_DECISION` anti-injection refusal now names the sanctioned
  secret-comparison recipe — `redact(x, "hash_only")` on BOTH sides.
  The honest probe on the release binary v0.32.0 REFUTED the
  inbound-path hypothesis: serve delivers header/query bytes UNCHANGED
  (the raw-value decision block is the intended Category-A integrity
  gate). The production 401/403 (FOSVED-office-v2 #536) came from
  `redact(x, "all")`: its entropy net (ADR-0136) masks token-like runs
  (≥24 base64-class chars with a digit AND a hex letter) into
  `[REDACTED:entropy…last4]`, silently breaking the comparison — the
  gh#1317 table is explained 7/7 by that filter and is now
  golden-locked (`tests/n716_serve_secret_compare.rs`: the table, the
  recipe's comparability on both backends, the trap shape). The
  contract lives in `docs/limitations.md` (§ Serve Secret Comparison).
  No runtime semantics changed — the masking stays, the guidance gets
  loud. AFFECTS: none (no language bug); the office-side program fix is
  the hash_only recipe.

### Fixed

- **gh#1336 (the fosved-acc serve report, АСС-1.3/1.6):** the VM
  silently DROPPED an `each`/`each_with_index` nested in an `if`/`else`
  branch, a `while` body, another `each`, or a match arm — 0 iterations,
  no error, no warning — while the tree-walking interpreter executed the
  same program. Root cause: the loop lowering lived ONLY in the
  pattern/route-body TOP-LEVEL compiler; the SHARED statement compiler
  (the path of every nested context) carried an explicit NO-OP for the
  forms (the №510 "explicit no-op set"; a wildcard `_ => {}` before it —
  the posture is older than the report, present at least since v0.28.0;
  the exact first-bad release not bisected). In the consumer the shape
  guards `db_execute`/`http_get` (the incident-confirmation monitor
  stayed silent on an outage) — the №476 blocking class. The lowering is
  ONE shared helper now (`compile_each_stmt_with_locals` in
  `src/compiler.rs`); the №658 break/continue fixups resolve to the
  NEAREST loop (break/continue inside the nested each pinned). Tests:
  `tests/n1336_vm_each_in_if.rs` (7 shapes, both backends, liveness
  pinned) and the serve-lane route parity
  (`test_n1336_each_inside_if_body_vm_parity` — the issue's program
  verbatim). The differential fuzzer's nested-loop shapes carry
  observable accumulators now (the pre-fix `len(item)` bodies could not
  see the drop — the class escaped the run). AFFECTS: the serve VM lane
  (the ADR-0171 default) on the affected releases — consumer mitigation
  `METALOGOS_SERVE_BACKEND=interpreter` (АСС-1.7) until the release
  trains.

### Note (added 2026-10-19, gh#1350)

- **Gate coverage (the gh#1336 follow-up):** the shared statement lowering
  (the serve-VM fix above) also made `each` bodies visible to the
  compile-time sink-clearance pass — programs green on ≤ 0.33.0 may fail
  `mlog check` from 0.33.1 with previously-invisible findings (e.g.
  `PII_EGRESS_NETWORK` on a `cron_list()` → `cron_remove` read-back).
  Interim bridge: `redact(…, "all")` (the documented trap: the ADR-0136
  entropy net masks token-like key runs). The value-preserving recipe is
  ratified in gh#1350 (the read-back pair recognition, Ask 2 (a)) — the
  implementation rides the next release train.


## [0.33.0] - 2026-10-19

### Added

- **№695 (gh#1274, Wave 46 — ADR-0188 phase 2):** the template pipe
  grammar lands — the safe stage `{{ param | safe }}` in template
  declarations. Without the pipe the placeholder auto-escapes (the
  contract is unchanged); with the pipe the value rides RAW and the
  raw landing is honest by construction: the parameter must be
  declared `Html` (`SAFE_PIPE_NOT_HTML` refuses at check time on BOTH
  backends — the №685 mirror for the escape-exempt slot), the runtime
  value must be the opaque `Html` (the `render()` backstop refuses a
  plain String fail-closed), and the taint layer CONDUCTS through a
  safe slot (a raw position — it never answers «sanitized»). Every
  unrecognized pipe form (an unknown stage, multiple pipes, a pipe in
  the raw triple-brace form, an expression that is not a declared
  parameter) refuses loudly (`UNSAFE_PIPE_FORM`) — the N9 refusal
  (gh#1219) narrows to exactly these forms and shifts left to the
  semantic gate; the registration refusal stays as the runtime
  backstop. The last Known-Issue lane (gh#967 §7) closes with the
  lift; №115 closes executed.

## [0.32.0] - 2026-10-15

### Note (added 2026-10-22, №768 — the external audit 21.10 §3 M-2)

The version line skips 0.31: **0.31 was never released** — its goal
(the gate_031 typing movement, ADR-0191) folded into 0.32.0, and the
release number 0.32.0 is the owner's verdict (the lockstep
0.30.1 → 0.32.0; the REALITY §6.29 record). The tag line confirms:
`v0.30.1` → `v0.32.0`, no `v0.31.*` tag exists.

### Security


- **№689 (gh#1256, В44 — the R-1 audit finding; fix date 2026-10-15):**
  the mail ПДн (personal data) learned to SPEAK at the flow layer. Before
  the fix the `private` mark lived on exactly ONE registry row
  (`canary_check`, №661), `imap_read` carried all 9 fields `untrusted`,
  and the label engine was NAME-based only — the registry field-meta
  never reached the №323/№325 flow machinery: a program could send the
  mail correspondence (from/to/subject/body_text/body_html/attachments)
  straight into `smtp_send`/`http_post`/`respond` with ZERO findings —
  the silent ПДн egress the audit's R-1 described («the machine does not
  check the private → sink flow because there are almost no marks»).
  The repair, RED-first (tests/naryad_689_private_field_flow.rs): (1)
  the REGISTRY marks — the ImapEmail ПДн fields are `private` now (the
  shape read off the impl, №650; uid/date/seen stay `untrusted` — the
  server metadata); (2) the BRIDGE —
  `semantic::registry_private_flow_label`: a row whose field-meta
  carries `private` seeds the flow layer with the type-layer secret
  projection (conf=private, integrity=trusted, the ADR-0154 §2 form), so
  the №325 sink gate fires the PII_EGRESS_NETWORK / PII_EGRESS_OUTPUT
  classes on the let-bound AND the direct-call paths; `redact` stays the
  only legal downward cure (№326, tested). The honest boundaries: the
  bridge is deliberately NARROW (rows without a private mark — the
  untrusted/integrity vocabulary — keep the old path; whole-value
  fail-closed posture, the per-field precision is the stage-2 contour);
  `card_contacts`/`cal_events` keep the honest JSON-String spelling with
  the reason in the row comments (the stage-0 scalar vocabulary carries
  no label — a `String<private>` row would erase to Unknown and regress
  the typed floor; the structural typing is the №688-cycle tranche).
  Metrics (the actual recount): the field-label share 46/48 = 95.83% —
  unchanged; the sensitive surface 73/145 = 5034 bp — unchanged (the
  marks changed values, not coverage). Affected: programs egressing the
  imap_read fields into the egress sinks — they now refuse at compile
  with the PII class named (advisory under `profile legacy`, the №325
  posture).

- **№685 (gh#1252, В44 — the S-1 audit's High finding; fix date 2026-10-15):**
  the RAW template insert `{{{ var }}}` in `render()` is FAIL-CLOSED now.
  Before the fix the N9 raw form (gh#1219) landed its argument VERBATIM in
  the page while every static layer answered «sanitized» at the render
  boundary — `render("Comment", "author", body.author, "text", body.text)`
  with `{{{ text }}}` in a `text: String` template passed `mlog check`,
  `run` and `serve` with zero findings: the XSS the audit demonstrated,
  invisible to the HTML_INJECTION category. The repair is the audit's
  prescription, three gates deep: (1) the DECLARATION gate — a template
  parameter used in `{{{ }}}` must be declared `Html`, anything else is
  the `RAW_INSERT_NOT_HTML` stable-code refusal BEFORE execution on BOTH
  backends (check_program + the compile_program gate block, the №673
  contour shape; the code joins the №385/ADR-0169 classification, nothing
  renamed); (2) the CALL-SITE coercion — a String-shaped argument into an
  Html-typed parameter of a statically-known template refuses (the №662
  rule-1 twin, the implicit String→Html the runtime accepted silently);
  (3) the TAINT/LABEL layers — `render()` is now a sanitizer for the
  ESCAPED positions and a CONDUCTOR for the RAW ones (the template body
  decides from the program's declarations; a dynamic/unknown name is a
  conservative conductor; the UserInput/LLM egress gates
  OPEN_REDIRECT/HTML_INJECTION/TELEGRAM_HTML_INJECTION read the conducted
  taint instead of a fake lift). The honest path is unchanged: an
  Html-typed parameter built by an explicit Html producer
  (`render("Outer", render("Inner", s))`) renders verbatim on both
  backends. Affected: programs that fed String values into raw
  `{{{ }}}` positions — they now refuse at start with the parameter and
  the fix named in the message (the class-4 dictionary entry, the guard
  scenario per the №653 form; the audit's «do not cut 0.31.0 before S-1»
  is satisfied). `template_render`'s raw=true trusted-author contract is
  unchanged (the limitations row now tells the two contracts apart).


- **№673 (gh#1200, В41 — BREAK_OUTSIDE_LOOP, the R-2 audit's two
  «молча неверные» break/continue edges; fix date 2026-10-14):** a `break`/`continue` (а)
  OUTSIDE any loop (№658-b) or (б) crossing the VALUE channel — an arm
  body of a match/if-as-value (№658-a) — now refuses as a SEMANTIC error
  BEFORE execution on BOTH backends, with the stable
  `BREAK_OUTSIDE_LOOP` code stamped first (the №523 shape; the code
  joins the №385/ADR-0169 classification, no existing code renamed).
  Before the fix both edges were the class-4 dictionary entries (№653)
  the honest-limits records pinned: the TW refused at RUNTIME
  («break/continue used outside of a loop»), while the VM either
  compiled the bare statement to NOTHING and continued (the audit's
  import_batch shape — `break` on the error path instead of `return`,
  and the code after it RAN: the recorded pre-fix VM answer was
  `Ok("published")`) or silently ABSORBED the signal inside a
  value-channel arm and ran the loop to its condition (the recorded
  pre-fix VM answer was `Ok("matched;matched;matched;")` where the TW
  refused). The repair is the audit's recommendation — a semantic gate,
  not a runtime mechanism: ONE shared walker
  (`semantic::break_outside_loop_violations`) walks every executable
  body (pattern, tool method, server route, hook, on_deny handler,
  test) with the loop/value-channel context model — loop bodies open
  the loop context, statement-position branches preserve it (the
  №658 contract the norms S-BLK-011/012 pin stays UNCHANGED — a break
  nested through if/match inside a loop keeps working), value-channel
  arm bodies forbid the signal outright, a loop inside an arm re-opens
  the legal context for its own body. Wired into BOTH production gates:
  `check_program` (the TW run/serve path) and — for the first time for
  a semantic finding class — `compile_program` (the VM compile→run .mbc
  path, which had no semantic gate; only THIS class is wired there, not
  the whole check_program — the widening is a separate conversation).
  The runtime mirrors stay untouched as backstops. Two conformance
  pairs landed (the sblk_011/012 form): `sblk_015_break_value_arm`
  (№658-a) and `sblk_016_break_outside_loop` (№658-b), both
  `STATUS: error, CODE: BREAK_OUTSIDE_LOOP` — RED before the fix on
  the VM (the silent answers above), green after. Honest limits:
  the blocks.md №658-a/№658-b records moved to the REPAIRED form
  (the №652-a sample; the edge history kept). Affected: programs that
  RELIED on the silent absorption (the №653 class-4 guard scenario) —
  they now refuse loudly at startup and must use `return`.
  Size: M; risks: low-medium (any pre-existing program carrying the
  illegal shapes now blocks at startup — that is the repair's purpose;
  the legal break/continue surface is contract-pinned unchanged).


### Added


- **№688 (gh#1255, В44 — the audit f92c61c S-3; the draft, the owner's
  verdict pending):** the 0.32 typed-source goal moves to the
  FIXED-LIST closure form (ADR-0193 **Proposed**) — «of the 67 untyped
  sensitive sources recorded at the anchor `f92c61c`, at least 20
  closed, the ПДн-class first; the denominator is OUT of the goal».
  The machine half: `scripts/ci/gate_032_untyped_sources.txt` (the 67
  ids with the registry lines at the anchor, generated by the anchor
  worktree's own №644 inventory) + the `--goal032` report-only counter
  (GREEN at the honest 0/20 start; the `--gate032 N` floor mode exists
  mechanically and wires ONLY with the owner's §5 fixation — the
  gate_031 one-PR form). The X-3 fraud shape (the 0.31 goal met by the
  denominator growth 66/138 → 73/145 while `untyped 67` did not move)
  is closed mechanically: only closing REAL rows of the FROZEN list
  moves the goal.
- **№691 (gh#1258, В44 — the spec topics):** the specification grew two
  topics — **routes.md** (Topic 4: S-RT-001 the route declaration
  vocabulary, S-RT-002 the direct respond* early answer, S-RT-003 the
  depth-2 RESPOND_SWALLOWED refusal, S-RT-004 the raw-insert Html-only
  rule, S-RT-005 the protective-header ownership — the middleware's,
  not set_header's, S-RT-006 the respond_html dual contract) and
  **state.md** (Topic 5: S-ST-001 the world_state private-by-default
  materialization refusals, S-ST-002 the private label gates
  materialization — not the handle flow — plus the typed device chain,
  S-ST-003 the literal-key memory round-trip). NINE new conformance
  pairs (both backends, the №645 runner; the №685 raw-insert pairs
  referenced, not duplicated); the probes found NO cross-backend
  divergence — the route/state refusals are shared-code builtins and
  semantic checks. The normative core: 33 → 42 norms. The №674
  mirror-scan IN SYNC (the №668 typed-coarse honesty got its №588-form
  limitations row in the same PR); REFERENCE cross-references the
  topics from the server and the world_state rows.

- **№674 (gh#1201, В41, process/ci — the honest-boundary machine sees
  the spec):** `honest_boundary_check.py` now scans the «Honest limits»
  sections of `docs/spec/*.md` — every ACTIVE record (the №-id bullets)
  MUST have its mirror row in `docs/limitations.md`; REPAIRED records
  are exempt (the repair closed the edge — the №652-a sample). The scan
  is repo-state based and runs in the SAME blocking №604 job on PRs AND
  on push/main (the job's push/main branch runs the full retrospective
  now instead of exiting early — no new CI cell, №535); the `--diff`
  marker/dictionary modes stay unchanged. The gap it closes: the №659-x
  /№659-y records lived in `docs/spec/blocks.md` ONLY — limitations.md
  had no mirror (the X-1 class: honest knowledge outside the machine's
  control; grep 658|659 was empty). In THIS PR the mirrors landed (the
  two OPEN rows in the TW/VM Divergences section, the №588 form), the
  self-test gained the hermetic collector/mirror cases, and the demo
  probe (a temporary unmirrored record → exit 1 with the exact list;
  removed → IN SYNC) is recorded in the naryad report. The same run
  surfaced a REAL classification gap: the №673 CHANGELOG entry (the
  №653 class-4 repair) sat outside `### Security` — moved
  there in THIS PR per the №643 machine (the №629 precedent; the №658
  fix of the same break/continue line landed under `### Security` of
  0.30.1). Spec counters (the Topics table, 33 norms) verified in sync.

- **N7 (gh#1217, PE-2, serve) — the application response headers:
  `headers` on `HttpResponse` + `set_header(resp, name, value)`.**
  The pure value transform: it takes the response value, attaches the
  allowlisted header and returns the new response; the route returns it
  and the server applies the set at the conversion boundary (the
  `curl -i`-visible result). **The allowlist** (case-insensitive, the
  canonical spelling stored): Cache-Control, ETag, Content-Type,
  Location, Content-Disposition — the PROTECTIVE headers (CSP, XCTO,
  HSTS) are the `security_headers` middleware's duty (N8) and cannot be
  overridden from the route. **The injection gate**: CR, LF or NUL in
  the name or the value → the typed `[HTTP_HEADER_INJECTION]` refusal
  (the response-splitting vector) — the runtime gate on both backends
  AND the static Category-A compile lane on the literals (the injection
  verdict comes before the allowlist's, so a smuggled name never leaks
  the allowlist's wording). **The allowlist gate**: any other name →
  the typed `[HTTP_HEADER_NOT_ALLOWED]` refusal naming the offender and
  the allowlist. **The taint gate**: a UserInput-tainted VALUE →
  `[HTTP_HEADER_TAINTED]` at COMPILE time (the Category-A lane; the
  header channel is the server's own voice — echo the client into the
  escaped body, never into a response header); clean computed values
  (an in-program ETag) pass. `set` semantics: the same name
  (case-insensitive) REPLACES, the others keep their order; a
  `set_header("Content-Type", ...)` overrides the response's
  content_type field at the boundary (the map-insert contract). The
  signature ships parameterized-coarse
  `Struct<HttpResponse>{4 internal}` WITH the field-meta (the №661
  anti-dilution rule; every field is program-computed at construction).
  The floors raised in the same PR (№757): general 6317 → 6326 bp,
  parameterized 6893 → 6923 bp, fieldmeta 9565 → 9574 bp; precise
  4307 bp (the dilution absorbs within the 4302 floor on this branch).
  REFERENCE + the classification regenerated (519 → 520 builtins);
  registry_arity re-blessed; the gate facts updated. 11 tests (9
  integration + 2 server-boundary), RED first. Size: M. Risks: the
  allowlist is deliberately narrow (a need outside the five names is an
  explicit ADR conversation, not a silent widen).
- **N9 (gh#1219, PE-2, compiler/bug) — `render()` and `| safe`: no more
  silent escaping.** The owner's finding: `render()` always escaped,
  the Jinja-style `{{ var | safe }}` was NEVER implemented (the №115
  substitution left the placeholder VERBATIM in the output, the №86
  engine silently rendered it empty), and the composition example
  pinned the silent mismatch with an EMPTY .expected. The fix combines
  the naryad's menu options 1+2+3. (1) The filter pipe is a LOUD
  refusal — a compile error on BOTH backends (the №250 compiler
  registration gate and the interpreter declaration walk share the
  failing `register_template`) and a loud parse refusal in
  `template_render`, each naming the supported raw form; a pipe inside
  a condition string literal (`{{#if x == "a|b"}}`) stays legal — the
  construct grammar is not a filter. (2) `render()` gains the RAW
  insert — the triple-brace form `{{{ var }}}`, the same contract
  `template_render` already had; safe-by-construction: the parameters
  are program-provided, the egress gates still apply downstream. (3)
  The composition example (p6_template_composition) actually renders
  now — the data rides the ESCAPED default (`{{ title }}`, the
  script-neutralization is visible in the .expected), the trusted
  program-built fragment rides raw (`{{{ content }}}`); the empty
  .expected (the pin of the silent empty render) is replaced by the
  real output. REFERENCE §4.14 documents the raw form and the refusal.
- **N6 (gh#1216, PE-2, ADR-0192) — the `xml` capability feature:
  `feed_parse` / `sitemap_parse` / `xml_parse`, the feed/XML family.**
  Off-by-default behind `xml` (implies `html` — the shared text/limit
  helpers; xml5ever 0.18, the same MIT/Apache-2.0 servo stack, promoted
  from the transitive markup5ever_rcdom dep to a direct optional). The
  registry surface is feature-INDEPENDENT: without the feature the
  three handlers refuse LOUDLY naming ADR-0192. `feed_parse(xml)` —
  RSS 2.0 (`rss > channel > item`) and Atom 1.0 (`feed > entry`) detected
  by the root element; every item → `List<FeedItem>{title, link,
  published, summary, id}`; a missing field is the EMPTY string
  (fail-closed items — one ragged entry never kills the batch): pubDate
  falls back to dc:date, published to updated, summary to content, guid
  to link; the Atom link prefers `rel="alternate"`. `sitemap_parse(xml)`
  — the `urlset > url` and `sitemapindex > sitemap` shapes uniformly →
  `List<SitemapEntry>{loc, lastmod}` (lastmod often absent → "").
  `xml_parse(xml)` — the general tree → `Struct<XmlNode>{tag, attrs,
  text, children}`: the local name is CASE-SENSITIVE (XML is), attrs is
  a Dict (namespace DECLARATIONS are not attributes — the XML5 grammar
  carries them out-of-band), text is the DIRECT text (normalized,
  CDATA contributes), children recurses. **Security: DTD and external
  entities are NEVER expanded** — the XML5 grammar has no custom-entity
  mechanism, so XXE degrades to literal text and billion-laughs stays
  bounded (both pinned by explicit negative tests). The loud limits:
  input ≤ 5 MiB, result ≤ 10 000 nodes, tree depth ≤ 256 (the recursive
  build must not blow the stack) — all loud, never a silent truncate.
  EVERY result string carries the untrusted taint (the N2/N5 source
  lists). The golden pair `examples/n6_feed_parse.mlog(+.expected)` is
  byte-identical on TW and VM; the default golden sweeps skip it (the
  №373 parity gate: the xml-gate list joins the frozen capability
  classes). New blocking CI job `xml-tests` (the off-by-default guard +
  the feature-on suite + clippy); the feature rides `portable`. The
  №757 precise compensation IN THE SAME PR: `decrypt` and
  `reflex_bpe_decode` ship the verified precise `String` facts (227/525
  = 4323 bp; the floor 4302 → 4323), the parameterized share 76/108 =
  7037 bp (floor 6893 → 7037), the general 336/525 = 6400 bp, the
  field-meta 45/47 with `Struct<XmlNode>` carrying the honest
  all-untrusted labels. REFERENCE + the classification regenerated (522
  → 525 builtins). 15 tests (14 feature-on + 1 off-feature refusal),
  RED first. Size: M. Risks: the RSS/Atom real-world variance is wider
  than the two fixtures (the fail-closed item shape absorbs the ragged
  forms); XXE hardening is by-grammar + negative-tested.
- **N5 (gh#1215, PE-2, ADR-0192) — the `html` capability feature:
  `html_select` / `html_text` / `html_links`, the DOM extraction family
  the tender agent needs.** Off-by-default behind `html` (html5ever 0.27
  + markup5ever_rcdom 0.3 + url 2 — MIT/Apache-2.0/Unicode-3.0 only, the
  №518 license gate verified by the N5-pre probe; the MPL-2.0 `scraper`
  stack rejected). The registry surface is feature-INDEPENDENT (the N559
  rhythm): without the feature the three handlers refuse LOUDLY naming
  ADR-0192, never silently. `html_select(html, css)` returns
  `List<HtmlMatch>{tag, text, attrs, html}` — the selector subset is
  tag/`*`/`.class`/`#id`/`[attr]`/`[attr=value]`/descendant/direct-child/
  comma (anything else a loud parse error naming the subset and the
  position); document order; attrs is a Dict. `html_text(html, css?)`
  returns the whitespace-normalized text — script/style subtrees NEVER
  contribute. `html_links(html, base_url)` returns
  `List<HtmlLink>{href, text}` — hrefs are ABSOLUTE (the WHATWG
  resolution via the url crate), only http/https survive, `javascript:`
  and every other scheme is DROPPED by contract, never an error. The
  loud limits (fail-loud discipline): input ≤ 5 MiB
  (`HTML_INPUT_TOO_LARGE` shape — the error names the limit), result ≤
  10 000 nodes (never a silent truncate). Broken-HTML tolerance is
  html5ever's spec error-recovery. EVERY result string carries the
  untrusted taint — the audit/semantic/html-label source lists mirror
  `request_header` (N2): scraped HTML/text must not reach `respond_html`
  without the escape/validate step. The golden pair
  `examples/n5_html_tender.mlog(+.expected)` (the synthetic Cyrillic
  tender page) is byte-identical on TW and VM (the feature-on contract
  test H8); the default golden sweeps skip it (the html-gate list, the
  №373 parity gate re-justified: three sanctioned exclusion classes).
  New blocking CI job `html-tests` (the off-by-default guard + the
  feature-on suite + clippy); the feature rides `portable` per the
  Cargo.toml feature-block rule. REFERENCE + the classification
  regenerated (519 → 522 builtins); the precise-share fact 4315 → 4310
  bp (html_text ships precise String; the two coarse `List<...>` rows
  dilute — the floor raise is the №757 follow-up), the parameterized
  fact 6893 → 6952 bp. 15 tests (14 feature-on + 1 off-feature
  refusal), RED first. Size: L. Risks: the minimal selector engine is
  deliberately NOT a CSS engine (the unsupported syntax is loud);
  fuzzing on broken HTML rides the existing fuzz-smoke lane.
- **N8 (gh#1218, PE-2, serve/bug) — the `security_headers` middleware
  actually applies the protective headers.** The owner's finding: the
  declaration was VALIDATED (the semantic name check + the "no
  security_headers" recommendation warning) but never APPLIED — a plain
  GET showed no CSP, no XCTO, no HSTS. The fix: the middleware applies
  the three protective headers to EVERY response — the route responses,
  the axum fallback 404 and the direct-dispatch 404 (the OWASP
  every-response shape): `Content-Security-Policy: default-src 'self'`,
  `X-Content-Type-Options: nosniff`,
  `Strict-Transport-Security: max-age=31536000; includeSubDomains`. The
  middleware OWNS the set: unconditional inserts, and the `set_header`
  builtin (N7) does not carry these names — a route cannot override the
  protective set (the N7 allowlist is the boundary: application headers
  belong to the route, protective headers belong to the middleware).
  The opt-in contract is byte-exact: WITHOUT the middleware in the list
  the headers are ABSENT (pinned by the negative legs). The wire-level
  contract (the №892 precedent — the defect lived on the wire, not at
  the builtin): a REAL `run_test_server_with_backend` HTTP round-trip,
  BOTH serve backends × with/without the middleware × route/404 = 4
  tests, RED first (the with-middleware legs failed on the missing
  headers before the fix). REFERENCE documents the applied set + the
  ownership boundary. Size: S/M. Risks: the CSP baseline
  (`default-src 'self'`) is strict by design — a route serving inline
  scripts/styles needs an explicit config conversation, not a silent
  exception.

- **N3 (gh#1212, PE-1) — the send_message opts + send_photo: the
  Telegram delivery family grows up.** `send_message(chat_id, text,
  reply_markup?, opts?)` — the optional last `opts: Struct` carries
  `parse_mode` (`"HTML"` | `"MarkdownV2"`, anything else loud), `bot`
  (the bot NAME — the token comes from `TELEGRAM_BOT_TOKEN_<NAME>`, the
  name uppercased; an explicitly named bot with no token env is a loud
  error naming the ENV VAR, never a value; without `bot` the legacy
  `TELEGRAM_BOT_TOKEN` is read), `disable_preview` (maps to the Telegram
  `disable_web_page_preview`), `chunk` (default TRUE — the text routes
  through text_chunk, the "paragraph" strategy, budget 4000, overlap 0;
  every chunk leaves as its own sendMessage call, `reply_markup` rides
  ONLY the last chunk, `parse_mode` rides every chunk; a text without
  paragraph boundaries degrades to the hard 4000-char windows), and
  `retry` `{max_retries, base_delay}` (base_delay in milliseconds, the
  METALOGOS_HTTP_MIN_INTERVAL_MS convention; a 429 honors the server's
  retry_after, a 5xx backs off base_delay·2^attempt, BOTH capped at
  30 s — `metalogos::builtins::TELEGRAM_RETRY_CAP_MS`; up to
  max_retries (default 3) retries after the initial call; other 4xx are
  loud immediately). A 3-argument Struct WITHOUT any known opts key
  stays the legacy `reply_markup` (backward compat by contract). The
  new `send_photo(chat_id, photo, caption?, opts?)` — the photo is an
  HTTPS URL or a file_id (the API passthrough), the caption is refused
  loud beyond 1024 characters BEFORE any wire traffic, `chunk` /
  `disable_preview` are documented no-ops for a photo. The secret
  discipline is family-wide: every diagnostic is token-redacted (the
  reqwest URL Display and a hostile endpoint's echo body carry no token
  fragments; send_message / send_photo / answer_callback_query /
  edit_message_text / send_document all redact). The compile gate
  `TELEGRAM_HTML_INJECTION` (the HTML_INJECTION analogy the naryad
  pins): untrusted text into the text/caption position with the literal
  opts `parse_mode: "HTML"` is a Category-A compile error naming the
  fix (escape_html) — opts-aware, both taint lanes, ordered BEFORE the
  №325 sink clearance so the shared site names the scenario; the
  lattice stays the general gate (`MarkdownV2` / opts-less keep
  `UNTRUSTED_EGRESS_NETWORK`). `METALOGOS_TELEGRAM_API_BASE` overrides
  the API origin — the test/proxy seam mirroring
  METALOGOS_OPENAI_BASE_URL (№12). 30 tests
  (`tests/naryad_n3_send_opts.rs`) over a canned-response Telegram mock
  (bind → env-lock → serve, early-exit after the expected request
  count — the parallel-safe mock discipline): the wire shapes, the
  chunk boundaries + the last-chunk markup, the 429/5xx retry budgets
  and the 30 s cap, the never-leaks pack, the bot-name env resolution,
  the loud opts validation, the caption limit boundary (1024/1025),
  the TW↔VM parity leg, and the compile-gate pack. RED first (23/30
  behavioral failures on main; the helper-level test did not compile —
  no such API). Golden pairs: `telegram_send_opts.mlog` (the no-token
  Unit stubs with opts accepted) +
  `err_send_photo_caption.mlog` (the off-server caption refusal).
  REFERENCE regenerated (518 → 519 builtins), the classification row
  added, the arity export re-blessed, the gate facts synced
  (fact_precise_share_bp 4324 → 4315 — the precise share is
  count-stable over a bigger denominator, the goal ratchet 3000 bp
  untouched). Size: M/L (the largest naryad of PE-1, per the issue);
  risks: medium — the chunk=true default is an INTENDED behavior
  change for >4000-char texts (documented), the opts recognition is
  key-based in the 3-argument position (a Struct without opts keys is
  a reply_markup — mixed structs are loud), and chunk boundaries may
  carry a leading paragraph separator (text_chunk keeps separators for
  lossless reassembly — delivered as-is, v1-min).

- **N4 (gh#1213, PE-1) — the outgoing-HTTP hardening: the host
  allowlist, the User-Agent identity, the per-host pacing, the robots
  politeness gate.** All four egress builtins (`http_get`,
  `http_post`, `http_download`, `http_post_multipart`) now sit behind
  the env-driven gate stack, read at CALL time (the №130
  `ssrf_guard_enabled()` convention). `METALOGOS_HTTP_ALLOW_HOSTS`
  (comma list, `*.example.by` wildcards anchored at the dot,
  case-insensitive, whitespace-tolerant) refuses any non-listed host
  with the typed `[HTTP_HOST_NOT_ALLOWED]` BEFORE any DNS lookup or
  connect; per redirect hop by construction — №261's `Policy::none`
  means every hop is a new program-level builtin call, so the gate
  re-runs on each hop (the №261 SSRF-pin-per-hop argument replayed for
  the allowlist; unset/empty = gate off, backward compatible).
  `METALOGOS_HTTP_USER_AGENT` overrides the new default identity
  `Metalogos/<version>`; an explicit caller `User-Agent` header wins
  over both (any-case compare). `METALOGOS_HTTP_MIN_INTERVAL_MS` paces
  the requests per host:port bucket (different endpoints never wait on
  each other; retries are paced too). The `{robots: true}` call option
  (http_get / http_download — retrieval is crawl semantics, a POST is
  an action) fetches robots.txt once per host over the same gate
  stack, caches the parsed rules for exactly 24 h
  (`metalogos::builtins::ROBOTS_TTL`), enforces the minimal
  `User-agent: *` ruleset with the RFC 9309 longest-match verdict (a
  tie goes to Allow; the empty `Disallow:` allows everything; no
  `*`/`$` inside rule paths — the documented minimal scope) and
  refuses a denied path with the typed `[HTTP_ROBOTS_DENIED]`; a
  missing (4xx) or unreachable robots file fails OPEN and the empty
  ruleset is cached the same way (an unreachable host is not
  re-probed on every call). Two new whitelisted origin-stamped codes
  (`HTTP_HOST_NOT_ALLOWED`, `HTTP_ROBOTS_DENIED`) join the №592/№385
  `try{}`-branchable taxonomy. 18 tests:
  `tests/naryad_n4_http_hardening.rs` — the matching table, the
  typed-before-connect refusals on all four builtins, the TW↔VM
  refusal parity, the UA default/override/caller-wins over a live
  mock, the per-host pacing (and its absence across hosts), the
  robots allow/deny/cache-24h/fail-open/longest-match pack. RED
  first (the refusal tests observed the request GO THROUGH on main).
  No golden pair: the surface is env-driven — a `.expected` would
  pin nothing off-server («где применимо» per the DOD). Size: M;
  risks: medium-low — no existing shape changed (the gates are no-ops
  with the env unset, verified by the №130/№261/№71/№592/№76/№260
  regression pack green), the visible deltas are the new UA header on
  every request and the pacing sleeps when the interval is configured.

- **N2 (gh#1211, PE-1) — `request_header(name)` + `ct_eq(a, b)`: the
  webhook-secret builtins:** `request_header` reads a request header
  from the current route body by a case-insensitive name (the server
  injects the per-request `HeaderMap` with lowercased names on BOTH
  backends; multi-value headers join with `", "`); an absent header or
  a missing route context answers the empty string — the query_param
  №40 parity contract, so a secret check fails CLOSED off the server
  instead of erroring. The result carries `UserInput` taint (the
  semantic/audit/html-label source lists). `ct_eq` is the
  constant-time string equality for secret comparisons: the XOR/OR
  accumulator never short-circuits on a differing byte (no `==`
  prefix-timing leak; the length check leaves only public metadata out
  of the accumulator path). The verdict restores INTEGRITY for
  decisions (comparing an untrusted token against the expected secret
  IS the validate-before-decide step the №327 gate asks for) while its
  CONFIDENTIALITY follows the compared values (a verdict over private
  material stays private for egress). The canonical example
  `examples/telegram_webhook_secret.mlog` pins the Telegram
  `X-Telegram-Bot-Api-Secret-Token` check shape. REFERENCE regenerated
  (curated §4 crypto/web rows + the generated index, 516 → 518).
  16 tests: `tests/n2_request_header_ct_eq.rs` (10 — the off-server
  shape, the ct_eq correctness table, the loud arg errors, TW↔VM
  parity) and `metalogos-server/tests/naryad_n2_request_header.rs`
  (6 — the live-header route surface on both backends with the strict
  TW↔VM cross-check). RED first. Size: M; risks: low-medium — two new
  builtins (no existing surface touched), the taint-lane additions
  mirror the query_param source treatment verbatim, and the ct_eq
  sanitizer semantics are the verify_password mirror (integrity
  restored, confidentiality preserved).

- **№670 (gh#1185) — the specification topic 3 «Errors and the try
  result» (docs/spec/errors.md — docs-only, the behavior is RECORDED,
  not changed):** six norms S-ERR-001…006 pin the existing try machinery
  on BOTH backends per the №645 protocol — the TryResult shape
  {ok, value, error} (the shared `try_result_struct`), the caught
  runtime refusal (ok=false, value=Unit, execution continues), the
  TryError{code, message} struct, the №385 classification contract (the
  whitelisted position-0 origin stamp, else the honest RUNTIME_ERROR —
  mid-message markers are content, not stamps), the semantic-blocking
  boundary (an undefined name refuses BEFORE execution regardless of
  try), and the first-class inner-try value (nesting). Seven conformance
  pairs (`tests/conformance/serr_*`) ride the existing
  `conformance (blocking)` job (the №535 rule — the cage does not grow);
  the runner demands the cross-backend agreement first. The README
  Topics table is synced with the actual fact (topic 2 → S-BLK-001…014 —
  the №658/№659 norms; the normative core 23 → 33). The topic-3 probe
  found NO cross-backend divergence (the shape and the classification
  are shared-code parity by construction); the stable refusal codes are
  referenced, never redefined (№385/ADR-0169).

- **№640 (gh#1123, В36, process/adr) — ADR-0188 «The Template
  Semantics Composition» (docs-only, the ledger gh#967 §7 lane plan):
  the compile-time port re-priced, the phased recommendation —
  Proposed at the merge (`bddc4cc5`, ADR 179 → 180) and
  **Accepted — option (б)** by the owner's verdict 2026-10-08
  (gh#1123); the option (б) compile-time checker landed by №662
  (gh#1163), the pipe syntax stays the documented gap (№115).


### Changed


- **№686 (gh#1253, В44 — the audit f92c61c S-1/S-2):** the risk-review
  checklist grew item 10 — SANITIZERS, SOURCES, AND SINKS. A PR changing
  the taint/label sites (`audit.rs`, `html_label.rs`, the `semantic.rs`
  lattice, `builtins_classification.rs`), the sanitize/source/sink name
  sets in them, or the `render`/`template_render`/`set_header`-class
  contracts must carry the sanitizer section in the PR description: the
  touched sites, the before/after for every affected `source → sink`
  path, and the regression pair (or the justification of its absence).
  The reminder is mechanical — the risk-review report prints the item-10
  section whenever the diff touches a site file, and a site hit triggers
  the report on its own (no new CI job; the existing advisory job grew
  the scan, the №604-class form; the guard's own check is a fixture
  self-test pinned by the CI-side driver
  `tests/naryad_686_sanitizer_guard.rs`).
- **№687 (gh#1254, В44 — the audit f92c61c S-2):** the naryad-class
  counter counts the commits OUTSIDE the naryads — a commit without a
  recognized naryad number (`Naryad №…`/`Наряд №…`) that touches
  `src/builtins/registry.rs` or ADDS a `Cargo.toml` feature/dependency
  key is a red-line finding the counter cannot «not notice»; the window
  verdict FAILS (the three-line report: the naryad quota, the off-naryad
  findings with sha + files, the combined verdict — the dispatch `--line`
  format unchanged). Docs-only and test-only diffs outside naryads stay
  legal (they grow no surface); a dependency version bump modifies, it
  does not add. The live S-2 window (the `pe2: n5`/`pe2: n6` HTML/XML
  PE commits) reproduces the audit's finding verbatim — the registry
  growth plus the `html`/`xml`/`url` Cargo keys, exit 1. The rule lives
  in AGENTS.md §8 (the agent entry point); the guard's own check is a
  fixture self-test pinned by `tests/naryad_687_off_naryad_quota.rs`.
- **№690 (gh#1257, В44 — the audit f92c61c D5):** the README CI badge
  is the REAL GitHub workflow badge now
  (`actions/workflows/ci.yml/badge.svg?branch=main` — it turns RED
  when CI fails; the static brightgreen shields picture never did, and
  its count had already drifted from the machine count). The №676 D5
  honesty survives: the blocking-job count still RENDERS from ci.yml
  into the text beside the badge (`badge_sync.py` — the machine
  renders, the human never hand-edits; the SSOT count 44 pinned by the
  render/verify round-trip). The badge is theme-independent by
  construction (white text on opaque gradients — legible on both the
  light and the dark README). The README stays at 149 lines (the D2
  gate). The live proof of the D5 point: the badge currently shows
  FAILING on main — the honest-boundary retrospective (the push/main
  scope that №685 turned on) flags the three boundary rows pending the
  ADR-0179 §6 release-time label sync — the badge surfaces what the
  static picture hid.

- **№676 (gh#1203, В41, docs/hygiene — the audit 7c3456b §4 checklist,
  all 11 rows):** the repo-tree cleanliness is now machine-enforced — a
  blocking `git status --porcelain` step after the Library tests AND the
  Integration tests (no new job; on its first full-suite run it caught
  two committed repo-tree writers, both lanes moved to throwaway
  `METALOGOS_DATA_DIR` roots); the CHANGELOG archive — the releases
  0.5.0→0.28.2 moved byte-exact into `docs/changelog/
  CHANGELOG-2025-06_2026-10.md` (42 sections, no record lost; the live
  root keeps [Unreleased] + 0.29.0/0.30.0/0.30.1 with their Security
  records; 727 KB → 57 KB); the README front-door reworked (Quick Start
  into the first 60 lines, 149 lines total, the feature-gate table →
  docs/feature-gates.md, the Newcomer Contract deep-dive → the link with
  the machine-pinned §3/§5 claims kept); the repo root is 8 .md files
  (AI_USAGE/MEMORY_ROADMAP/FEATURE_INTAKE/GOVERNANCE → docs/, the
  byte-identical GEMINI.md duplicate deduped); the generated
  naryad-number stamp left the README (the generator no longer stamps
  it, badge-sync greps it); the CI badge is RENDERED from ci.yml
  (28 → 44 — the static badge had drifted); the MSRV claims synced
  (1.93.1 everywhere); `frames/` and the leaked data/ fixtures left the
  tracked tree; the repo-writing test suites run in the №387 sandbox
  form (the №402 ledger export and the №394 tool write now land under
  throwaway `METALOGOS_DATA_DIR` roots; the №337 GENERATE leg's latent
  shared-scratch race with the CAPTURE leg separated).
- **№660 §5 (gh#1161 — the owner's verdict 2026-10-13) — the 0.31
  release gate OWNER-FIXED (the §5 mechanical transition, the №630
  shape):** the OD divergence (ADR-0191 §3/§4) closed by the variant
  (а) — the live-fact fixation **66/138 = 4782 bp** (the №644 machine
  read on the fixation anchor 34a6370 agrees exactly; the movement
  №668/№669 landed before the fixation, so the §5 read starts MET by
  the reached number and the gate holds as the ONLY-UP ratchet — the
  X-3 posture recorded, not hidden). ONE PR: gate_031_goals.txt
  `owner_fixed: true` + `goal_sensitive_surface_bp: 4782`, the fact key
  `fact_sensitive_surface_bp` landed WITH its checker (№525 — synced
  blocking by sync_gate_facts.py), the DEFAULT unfreeze target switched
  0.30 → 0.31 (the 0.31 read = the 0.30 v2 form verbatim + the
  sensitive-surface goal, fail-closed; the 0.28/0.29/0.30/legacy
  targets stay explicit — no record deleted), ADR-0191 → Accepted
  (§7). The 4130 / 4420–4493 draft variants retired (the history kept
  verbatim in the record); the 4 eternal honest refusals (query /
  query_row / query_scalar / request_body) remain the separate
  owner-decided dictionary-expansion line. **The landing re-bless
  (№757/№613):** the §5 merge brought the PE-2 platform-enabler wave
  (n5–n9) and the В43 nuclear №684 onto the same tree — every new
  sensitive source landed fully covered, the machine read rose
  66/138 = 4782 bp → **73/145 = 5034 bp** (the denominator moved by new
  sources only — Internal 102 → 109, the definition unchanged); the
  fact twin and the floor raised in the SAME PR (5034), the
  OWNER-FIXED goal line stays 4782 until the owner's later §5
  decision — the ratchet reads 5034 ≥ 4782: MET.

- **N13 (gh#1221, platform-enabler) — reasoning_content: the thinking
  block never leaves the contour, the usage does.** GLM reasoning
  models (thinking enabled) answer with `message.reasoning_content`
  alongside the final `message.content` on the OpenAI-compatible wire.
  The purity contract is now pinned on all three paths — the
  non-stream legacy parse, the SmartRouter parse, and the streaming
  delta extraction (`delta.reasoning_content` yields NO text, only
  `delta.content` deltas flow): the user sees ONLY the final content,
  never a reasoning fragment. The usage accounting gap is CLOSED: the
  legacy (no-`llm config`) path used to drop the provider's usage
  block on the floor — the trace line carried no token counts at all.
  Now the usage the provider reported for THIS call
  (`prompt_tokens` / `completion_tokens`; per the GLM/OpenAI contract
  `completion_tokens` INCLUDES the reasoning tokens) rides a
  thread-local from the provider branch into the caller's
  `METALOGOS_LLM_TRACE` line (`gen_ai.usage.input_tokens` /
  `gen_ai.usage.output_tokens`), cleared at every RealLlm entry and
  consumed once — a stale value can never be attributed to a later
  call; a response without a usage block stays honestly absent.
  Verified-loud edge: a reasoning-only response with `content: null`
  (the answer cut mid-reasoning) fails with the loud
  `Unexpected OpenAI response format` error — never a silent empty
  string. A plain response without a reasoning block is unchanged.

- **N12 (gh#1220, platform-enabler) — per-call model choice on
  `call_llm`:** the builtin grows an optional third String argument —
  `call_llm(prompt, input, model)` — naming the model for THIS call:
  a direct provider model name (`"glm-5.3"`) or a named profile alias
  resolved via `METALOGOS_LLM_MODEL_{ALIAS}` (the ADR-0048
  `resolve_model` order: env profile first, pass-through second). The
  override rides BOTH call paths — the SmartRouter `model_override`
  slot and the legacy backend's `call_with_model` (ADR-0048) — so
  `glm-5.3-flash` and `glm-5.3` work in ONE process (the owner's
  platform decision: the primary default is `glm-5.3-flash`). The
  legacy-path LLM trace names the model THIS call actually asked for
  (override first, env default second — honest per-call data, №276).
  Two-argument calls are byte-for-byte unchanged (the default leg is
  pinned by a test). A non-String third argument is a loud
  argument-type error, never a silent ignore. Registry arity 1..2 →
  1..3; REFERENCE §4.5 + the env-var table (METALOGOS_LLM_PROVIDER /
  METALOGOS_LLM_MODEL / METALOGOS_LLM_MODEL_{ALIAS}) updated.

- **№669 (gh#1184) — the typing movement 0.31 №1 (docs-adjacent,
  registry-locked; the behavior unchanged):** the forecast/graph
  untyped package — 4 of 6 registry rows get honest signatures against
  the verified handler facts (forecast_state Struct<ForecastState>{10
  fields — the prov-block metadata, 8 internal + horizon/label
  untrusted}, forecast_points Struct<ForecastPoints>{4 internal},
  graph_neighbors List<Neighbor>, graph_query List<GraphEntry>); **2
  honest refusals** (the №650 form): forecast_next — the opaque
  ForecastHandle (the stage-0 OpaqueKind has NO Forecast kind; a
  fictitious spelling is forbidden — the dictionary line is a separate
  owner decision), graph_path — the heterogeneous List (PathNode vs
  the PathError singleton). The sequential recount on the post-№668
  base: typed 326/516 = 6317 bp, parameterized 71/103 = 6893 bp,
  field-label 44/46 = 9565 bp, sensitive 66/138 = 4782 bp (only-up,
  №757); the compiled pins 322/68/99/43; the
  fact_parameterized_share_bp twin 6893 (the gate-facts-sync).
- **№668 (gh#1183) — the Q-2 movement №2 «memory → recognition»
  (docs-adjacent, registry-locked; the behavior unchanged):** 8 of 9
  gate_031 rows closed against the verified handler facts — the four
  typed-coarse rows parameterized (human_personas List<PersonaSummary>,
  human_recall List<Memory>, check_reminders List<DueReminder>,
  list_reminders List<Reminder>), semantic_search List<SearchResult>,
  mtree_retrieve List<MTreeEntry>, mtree_stats
  Struct<MTreeStats>{7 internal} + mtree_summarize
  Struct<MTreeSummarize>{7 internal} (both WITH the field-meta — the
  anti-dilution rule); the ninth row (world_state) is the honest
  OPAQUE WorldState handle (№355/ADR-0159 — not a scalar, not a
  parameterized List/Struct; the dictionary line is a separate owner
  decision). Sensitive 54/138 = 3913 → 62/138 = 4492 bp (only-up);
  the floors in the same PR.
- **№662 (gh#1163) — the ADR-0188 option (б) compile-time port (the
  behavior change, fail-closed; lands in the 0.31.0 semantics — NOT in
  the 0.30.1 train):** the SHARED semantic checker refuses two
  late-failure classes at CHECK time on BOTH backends (the owner's
  verdict gh#1123, 2026-10-08; the parity postulate — the checker is the
  backend-neutral front door): (1) the IMPLICIT String→opaque coercion —
  a String-shaped value (a literal or a `+` chain) flowing into a
  target declared with an opaque type (Html/…) at an assignment
  (entity initializer) or an argument position; the explicit path (an
  Html producer — `render()`) stays legal (№114 resolved); (2) the
  unknown template names — a `render("Name", …)` whose
  statically-resolvable name is absent from the program's template
  declarations (the №250 registration set; the request-time 500 moves
  left to check time; ONLY a string-literal name is statically
  resolvable — a bare Ident is a runtime-evaluated name (the CI-verified
  ok_448 leak-flow shape: render(raw) passes the variable holding the
  name) and every non-literal form stays the runtime's backstop). The
  corpus is clean (the recursive dry-run inventory: 370 .mlog files
  across examples/compat/tests, 0 refusals); the diagnostics move left
  for NEW code only. The pipe syntax stays runtime-absent (№115 — the
  documented gap, option (а) is later).

- **№637 (gh#1120, В36, types) — the parameterized movement 0.30
  №3 (the registry surface):** the PDF contour, 21 verified rows (the
  20 Struct rows WITH field_meta — the anti-dilution rule), the
  parameterized share 34/91 = 3736 bp → **55/91 = 6043 bp — the 0.30
  owner-fixed goal (≥ 5000 bp) REACHED**; the floors raised in the
  same PR (the parameterized baseline 6043, PARAM_FLOOR 52).
  (`e7894a1e`)

- **№638 (gh#1121, В36, types) — the field-labels №2 (the registry
  surface):** the 15 existing parameterized Struct rows labeled per
  the №627 table read off the handlers, the field-label metric
  5750 → **9500 bp (38/40 = 95%)** (FIELDMETA_FLOOR 37).
  (`48301708`)


### Fixed



- **№684 (gh#1244, В43 nuclear — VM tool-method resolution):** a
  `tool` method call (`calc.double(21.0)`) now COMPILES and RUNS on
  the VM — before the fix the bytecode pipeline refused every
  qualified tool call (`error: compile: qualified call
  'calc.double()' — not found`) while `mlog check` passed and the
  tree-walking backend returned `42` (the В42 no-progress flag,
  gh#1233 comment 6146003277). The compiler now follows ADR-0054
  Implementation Strategy verbatim: tool methods compile as REGULAR
  patterns under qualified keys (`tool.method`) into the existing
  `Program::patterns` table (both passes walk `all_decls`
  top-to-bottom, so the №415 positional contract holds; no new
  opcode, the .mbc schema is untouched — a tool call never compiled
  before, so old files are unaffected), and the `Expr::QualifiedCall`
  resolve gains the tool-namespace branch that mirrors the
  interpreter's dispatch order EXACTLY (execution.rs): runtime-op
  intercepts and plain builtins resolve BY FUNCTION NAME first
  (plain builtins shadow tool methods on TW — pinned by contract),
  then the qualified method, then a loud
  `compile: undefined method '<m>' in tool '<t>'` — never a
  fallthrough to flat patterns / the global-builtin fallback (TW
  would not reach those either). Two latent compiler holes closed on
  the way: the QualifiedCall arm never emitted its ARGUMENTS (no
  example in examples/ used qualified calls, so the gap was
  unreachable until the tool branch made them reachable — the flag's
  literal `21.0` arg exposed it on the first run), and the
  arity/purity metadata now rides the same `CompiledFn` shape as
  ordinary patterns. 13 both-backend contracts in
  `tests/tool_abstraction_contract.rs` (the 9 ADR-0054 contracts get
  their VM leg + the flag's minimal repro as the golden pair
  `42 == 42` + the shadowing/intercept mirrors); crosscheck,
  naryad_373_parity_gate, vm_golden, golden, conformance and the
  866-test lib suite stay green. Known pre-existing residue (out of
  scope, flagged): the wrong-typed `query()` diagnostic suffix
  diverges between the TW and VM db-op dispatch paths
  (db_ops.rs:139 vs :702) — reachable through plain calls long
  before this fix; behavioral parity is contract-pinned. Size: M;
  risks: low (the tool branch is gated on declared tool names; the
  resolution order mirrors the interpreter, so no new backend
  divergence). Unlocks the showcase App C (№682) after the next
  pinned release.

## [0.30.1] - 2026-10-12

### Security


- **№658 (gh#1159) — the VM break/continue parity (the behavior change,
  fail-closed; AFFECTS 0.30.0 AND EARLIER on the VM backend; the audit
  guard scenario: a `break`/`continue` inside a loop body — top-level,
  inside an `if` guard, inside a match arm, in a nested loop — was
  silently IGNORED by the VM: the body ran to the condition cap as if
  the statement were absent, and a top-level form kept an unresolved
  `Jump(0)` placeholder that tripped the infinite-loop guard, while the
  tree-walking backend honored both statements; the X-1/Y-1/Q-1
  silently-wrong control-flow class, the №652-a finding):** the VM loop
  path mirrors the TW `ControlFlow` contract 1:1 — every
  `break`/`continue` reaches its NEAREST enclosing loop on BOTH backends
  (`break` exits the loop; `continue` starts the next iteration — the
  condition re-test for `while`, the next item for
  `each`/`each_with_index`). The compiler now resolves every loop jump
  from the loop-stack entry: the shared statement compiler registers a
  `Jump(0)` placeholder in the NEAREST entry at any nesting depth and
  the loop compiler patches it from the POPPED entry (the stale-local
  patch loops — the pre-№658 root cause — are gone). The norms
  S-BLK-011/012 + five conformance pairs pin the record; the TW is
  unchanged. Found by №652 (the spec topic 2 probe); the repair follows
  the №651/№629 precedent. The fix lands 2026-10-09 (this PR; after the
  v0.30.0 tag b4b9abb lineage — the published tag is affected on the VM
  backend; no re-tag — the №620 precedent).

- **№651 (gh#1145) — the TW/VM condition parity (the behavior change,
  fail-closed; AFFECTS 0.30.0 AND EARLIER on the VM backend; the audit
  guard scenario: a composite or an opaque value in a condition — e.g.
  `if [items] { … }` or `if hash_password(pw) { … }` — silently chose a
  branch on the VM (a non-empty List took THEN, an opaque handle took
  ELSE) while the tree-walking backend refused; the X-1/Y-1/Q-1
  silently-wrong-guard class):** the VM condition path (the new
  `Instruction::JumpIfNotCond`, emitted for if / else-if / while / the
  match guards) mirrors the TW `Value::as_bool` 1:1 — the truthy set of
  a condition is exactly {true Bool, Float ≠ 0.0, non-empty String},
  Unit is falsy, and every composite / opaque / Fluid value refuses
  with the stable `[TYPE_MISMATCH]` code on BOTH backends (the norm
  S-VAL-013; the №629 loud-refusal class). The soft truthiness of
  `&&`/`||` (the №532 twin, compiled to the legacy `JumpIfNot`) is
  unchanged and stays pinned. Found by №645 (the spec-carcase
  conformance build, the finding №645-a); the repair follows the №629
  precedent. The fix lands 2026-10-08 (this PR; after the v0.30.0 tag
  6eafee1 — the published tag is affected on the VM backend; no re-tag
  — the №620 precedent).

### Changed

- **N1 (gh#1210, PE-1) — `slugify`: the deterministic Russian/Belarusian
  transliteration (the behavior change per the owner's dispatch):**
  Cyrillic letters no longer drop — a per-character table maps them to
  ASCII (а-a … ж-zh, х-kh, ц-ts, ш-sh, щ-shch, ъ/ь contribute nothing,
  ы-y, э-e, ю-yu, я-ya; Belarusian і-i, ў-u, ґ-g; the apostrophe drops
  with no separator). The output stays lowercase; ASCII unchanged;
  OTHER non-ASCII (CJK, Greek, accented Latin) still drop — the table
  is the contract, locale-free. The owner's four contract cases:
  `slugify("Ремонт офисов")` = `remont-ofisov`, `slugify("Привет мир")`
  = `privet-mir`, `slugify("Шамякина Мінск")` = `shamyakina-minsk`,
  `slugify("Щётка")` = `shchetka`. The p117 golden `slug_cyr` case
  (was `""`) now carries `privet-mir`; REFERENCE's `slugify` entry
  regenerated. Four tests in `tests/n1_slugify_translit.rs` (RED
  first). Size: S; risks: low — a pure builtin behavior change, no
  other builtin shares the code path.


### Fixed


- **#1169/#1170 (the office VM-pilot staging, FO-050; PR — this fix) —
  the VM serve-boot db bootstrap (the interpreter-parity boot path; the
  behavior change, fail-closed at boot; Fixed-class → rides the
  0.30.1 train):** `db { url: env("NAME") }` on the VM serve lane
  resolved the URL inside EVERY route handler (the №758 lazy
  first-db-access semantics), so the same read went through the №259
  serve-route gate — without `METALOGOS_ENV_ALLOWLIST` every request
  500'd with ENV_NOT_PERMITTED (the interpreter resolves the url
  expression at BOOT in the Process context and serves the same file +
  env with 200), and the schema-DDL bootstrap only ran when a request
  first touched the db — on the office's real corpus (30 routes) a
  fresh DB stayed at 0 tables and the first request died with an
  unrelated arity 500 instead of an honest bootstrap diagnostic
  (`[vm/db] Connected` was already on the boot log — the TW
  registration lane opened the file — while the VM request lane kept
  re-resolving and re-refusing per request). The fix: at VM serve boot
  (`run_server` + the test harness, on the compiled Program BEFORE it
  is shared) `boot_resolve_db_url` resolves the env name ONCE in the
  Process (boot) context — the resolved literal replaces `db_url_env`,
  so the request path performs NO env() reads at all — and replays the
  schema DDL eagerly on a boot connection (same open + WAL +
  DDL-tolerance steps as the lazy open), so a fresh DB carries the
  schema BEFORE the first request (the #1169 interpreter parity); an
  unset variable is a LOUD BOOT error naming the variable and the
  remedy (was: a silent OK boot — the TW registration error line was
  swallowed by the merge, then per-request 500s); a non-sqlite URL is
  the same LOUD boot error it used to be at the first access (№758
  text moved to boot time). The №259 serve-route env gate is
  UNCHANGED for program-level `env()` calls — the boot read is the
  Process-context class (the №259 contract, the №457 registration
  zone). The lazy per-request connection open stays for requests that
  never touch the db (№409 cost contract); `reset_for_reuse` semantics
  unchanged (№403 4/4). Contract tests
  `metalogos-server/tests/issue_1169_1170_vm_boot_db.rs` (T1: the
  db-touching route serves WITHOUT the allowlist — was 500
  ENV_NOT_PERMITTED per request; T2: the fresh DB carries the schema
  table immediately after boot; T3: the unset variable fails the BOOT
  loudly naming the variable — was a silent OK boot; T4: the
  literal-URL twin — no regression on the №758 surface); №758 5/5, №259
  7/7, №160 23/23 (1 ignored), №403 4/4, №457 5/5, №399 4/4 GREEN.
  The `TeamSkillWired expects 2 args` first-request symptom of #1169
  (the office corpus) is NOT reproduced minimally and is NOT claimed
  fixed — it is expected to disappear with the empty-schema state gone;
  the minimal-repro ask stands in the issue thread.


## [0.30.0] - 2026-10-08

### Security

- **№629 (gh#1096) — the VM comparison parity (the behavior change,
  fail-closed; AFFECTS 0.29.0 AND EARLIER on the VM backend — all six
  comparison operators, including `!=`; the audit guard scenario
  `if owner != current_user { … }` with `owner = Unit` PASSED on the
  VM — the X-1/Y-1/Q-1 silently-wrong-guard class):** the VM's
  `==`/`!=`/`>`/`<`/`>=`/`<=` now mirror the TW matrix 1:1 — the
  heterogeneous equality (`"abc" == 5.0`, `"5" == 5.0`), the ordering
  on non-Float pairs (`"b" > "a"`, `true > false`) and the
  incomparable types (Struct/List/Html) REFUSE with the stable
  `[TYPE_MISMATCH]` code on BOTH backends; the legacy VM answered a
  silent `false` (and `true` for numeric strings) — the
  silently-wrong class; `Unit == Unit` is `true` on both now. Found by
  the №621 fuzzer sweep the day it stopped being vacuous; the №479
  `+` precedent. The fix landed on main 2026-10-07 (22d62fa — after
  the v0.29.0 tag, 4be0f72: the published tag is affected; no re-tag
  — the №620 precedent); the affected-release audit 2026-10-08 (the
  unified audit of 48301708, the dispatch gh#1136, Q-1).

### Changed

- **№630 (gh#1098) — the 0.30 release-gate §5 mechanical transition
  (process, the owner-fixed merge):** `owner_fixed: true` in
  `gate_030_goals.txt` (ADR-0186), ADR-0186 Proposed → Accepted, the
  DEFAULT gate target of `unfreeze_gate.py` 0.29 → **0.30**; the 0.28
  and 0.29 records stay verifiable explicit targets (`--gate-target`).
  The typed parameter of the 0.30 gate = the parameterized share
  (≥ 5000 bp, the owner's 2026-10-07 verdict); the machine read is
  RED-loud until the movement completes — by design.
- **№631 (gh#1099) — the parameterized movement 0.30 №2 (internal):**
  the second honest package of 27 verified rows — the registry-order
  contour carries `List<T>` / `Struct<Name>` spellings now (the
  memory/todo/goal/profile/mood families; the polymorphic list ops stay
  honestly coarse — the element type is the caller's). The parameter
  share 7/91 = 769 bp → **34/91 = 3736 bp**; the floors raised in the
  same PR (the parameterized baseline 3736, the in-tree PARAM_FLOOR 31).
- **№627 (gh#1110) — the field-label metadata (internal, the stage-2
  preparation per ADR-0178):** the registry specs may carry the
  `Struct<Name>{field:label,...}` form; the labels live in the registry
  side-table (`BuiltinSpec.field_meta`), the `Type` enum stays erased;
  a malformed section is a loud `Unknown` (fail-closed). The FOURTH
  metric — the field-label share among the parameterized Struct rows
  (3/20 = 1500 bp, the first honest package of 28 verified rows:
  GeoLocation 9, Weather 13, LlmUsage 6) with the checked-in floor +
  the in-tree lock. The parser mirrors (gen_metrics, gen_reference, the
  two Rust pins) synced — the generated blocks regenerated.
- **№633 (gh#1101) — the dead_code burn (internal):** 33 → 15 (18
  places removed or justified), the floor re-locked in the same PR.
- **№634 (gh#1102) — the self-host lexer lane lifted green
  (internal):** the §5 test fixture had drifted behind the language
  (let-mut, the no-entry flow, the newline in the quote class, the
  stale keywords) — repaired; the №617 artifact documented-accepted
  with the mutation-verified pin; the ignore floor 14 → 13.
- **№635 (gh#1103) — the VM template_render lane lifted green
  (internal):** both vm_golden corpus sweeps 189/189 (the template gap
  was closed by №250; the harness facts fixed — the env sidecars №385,
  the candle-gated exclusion mirror, the browser-gated p88); the ignore
  floor 13 → 11.

- **№622 (gh#1085) — `return` inside a `match`/`if` EXPRESSION arm is
  captured as the arm's value on BOTH backends:** the TW documented
  contract (the expression channel carries values, not control signals)
  now holds on the VM — `let v = match x { "a" then { return 5.0 } }`
  binds `5` and runs the tail instead of terminating the pattern with
  `"5"`. The consumer scan (match arms / if-without-else / if-else /
  MatchExpr — the implicit block value never becomes an early return;
  sandbox/flow have no statement bodies in the grammar).
- **№623 (gh#1086) — the parameterized signature vocabulary stage-0
  (internal):** the registry specs carry `List<T>` / `Struct<Name>`
  spellings (the enum erases the parameter); the THIRD metric — the
  parameterized share among the List/Struct rows (7/91 = 769 bp, the
  first honest package of 7 verified rows; the general typed share 5988
  bp, the precise share 4224 bp untouched per the Z-2 verdict).
- **№621 (gh#1084) — the differential fuzzer executes again (internal):
**  the sweep was vacuously green (0/150 programs ran — the stale
  builtin call, the undefined idents, the heterogeneous arithmetic); the
  generator produces the №612 loop class (the bare-call bodies, the
  mixtures, the tails, the nested shapes) and the permanent LIVENESS
  ratchet pins the executability (≥ 60% of the sweep on both backends).
- **№617 (gh#1075) — the semantic checker grows three compile-time
  refusals (the gh#967 §3 lane):** the variable-scope walk (the TW env
  model verbatim — flat, never popped, the entities as globals, the
  render/reflex_* name-reference slots exempt), the undefined-variable
  read refusal (the runtime message verbatim + the stable
  `UNDEFINED_VARIABLE` code), the assignment-to-undefined refusal, and
  the static opaque-concat check (the declared-fact twin of the runtime
  guard — `cannot concatenate opaque type …`). The three `#[ignore]`d
  gh#967 §3 tests lifted green; the debt floor 17 → 14; the №465 fuzzer
  VM lane now carries the №523 semantic gate (the production order).
- **№616 (gh#1077) — the PRECISE movement 0.30 №1:** 61 registry rows
  typed against the verified single-shape handler facts — the precise
  share 3042 → **4224 bp** (218/516), the general share 4670 →
  **5852 bp** (302/516), TYPED_FLOOR 237 → 298; the floors raised in the
  same PR (the №757 procedure); the 0.29 gate fact synced (the
  machine-read rule).
- **№615 (gh#1078) — the 0.30 gate DRAFT (Proposed, ADR-0186):**
  `gate_030_goals.txt` with `owner_fixed: false` — the draft parameters
  strictly above the live facts (typed 6100 > 5852, precise 4500 >
  4224, the X-3 rule); the gate reads it honestly RED on the fixation
  fact; the DEFAULT gate target stays 0.29 (the blocking CI never reads
  a draft) — **the fixation is the OWNER's gate**.
- **№618 (gh#1076) — the Камертон cycle closure report:** the external
  review report artifact (`docs/audits/kamerton-report-v0.29.0.md`) —
  the SHA256 of the v0.29.0 assets re-run on the downloaded artifacts
  (3/3 OK against the attached SHA256SUMS), the R1–R6 verdicts from real
  runs of the repro corpus (`scripts/repro_kamerton/`, the t92
  reconstruction) on the fixed main — every probe now behaves as its
  contract demands (the mutation probe goes red: `assert_eq failed:
  3 != 999`), the D1/D2 → PR/commits map, the affected releases
  (v0.28.1, v0.28.2 — update to v0.29.0), and the honest protocol
  corrections verbatim from t92. The forwarding to the external
  reviewer is the owner's gate (the external channel).

## [0.29.0] - 2026-10-06

### Security (ADVISORY — restart your tests)

**0.28.x and earlier are affected on the TW interpreter — the fix ships IN
0.29.0; users of earlier versions should re-run their existing tests on
≥ 0.29.0.** The Камертон matrix finding (the unified audit of `e40ce8e` /
v0.29.0, the dispatch gh#1089) traced the №612 loop defect to its
user-facing consequences: until 0.29.0, a call with a non-Unit result at
the END of a loop body was read as an early `Return` of the ENCLOSING
function — the loop ran exactly ONE iteration, the tail after the loop
was silently cut, and in `mlog test` bodies the harness flattened the
fabricated `Return` into `Ok` — a GREEN run that never executed the
checks. The `return` here was FABRICATED by the implicit block-value
mechanics, not written by the author — the dictionary class «a
respond*/deny*/return being IGNORED» (№604), mirrored: a return being
INVENTED.

> `each row in rows { db_execute("INSERT …", row) }` — with the INSERT
> call last in the body, the first iteration returned from the function:
> one row inserted, the rest skipped, exit code 0.

- **The guard patterns with an audit loop:** a loop that re-checks a
  condition (audit/retry/scan) stopped after the first iteration — the
  code after the loop, the second half of the guard, never ran.
- **The batch inserts/updates:** a loop over rows executed exactly ONE
  element — the remaining rows were silently skipped (no error, no
  warning, exit code 0).
- **The false confidence of vacuous green runs:** in `mlog test`, a test
  whose body tripped the shape reported GREEN while its asserts never
  executed. The harness reads ControlFlow NOW (№612): an early
  termination is a LOUD failure — a green run on ≥ 0.29.0 means the body
  ran to completion.

- **Affected:** `mlog run`, `mlog test`, and `mlog serve` with
  `METALOGOS_SERVE_BACKEND=interpreter` (the fallback backend; the serve
  DEFAULT is the VM) — on every published release BEFORE 0.29.0. The VM
  backend is NOT affected. The fix landed in 0.29.0 (№612, PR #1068).
- **The user action:** restart your existing tests on ≥ 0.29.0; re-check
  the batch insert/update loops and the guard patterns with audit loops
  written before 0.29.0. The fix/audit date: **2026-10-06/07**.

### Fixed

- **№612 (gh#1061, PR #1068) — the loop semantics: a bare call in the
  body NEVER terminates the loop.** The Камертон matrix finding (TW):
  the loop arms read the block's №13 implicit-return value from the body
  as an early `Return(v)` of the ENCLOSING function — the first iteration
  became the last one, the tail after the loop was silently cut, and in
  test bodies the harness flattened the fabricated Return into Ok — a
  GREEN VACUOUS test (a deliberately false `assert_eq` stayed green; the
  asserts never ran). The fix (the honest statement semantics): the loop
  arms discard the body's implicit value; a loop statement, a binding and
  an assignment produce NO value (the implicit value resets — the poison
  class); the sub-block implicit value is replaced unconditionally (the
  staleness class); the test harness evaluates bodies
  ControlFlow-preservingly — an explicit `return` in a test body is a
  LOUD failure, a test that terminated early proves nothing (the corpus
  scan: zero test bodies use explicit return). Demonstrated pre/post on
  the honest-count program: pre-fix output 1 (the fabricated Return
  carried the FIRST `db_insert` id; the COUNT query never ran), post-fix
  output 3. 8 regression tests + the TW↔VM parity e2e (the VM's jump
  machinery never fabricated the Return — the honest count 3 on BOTH
  backends, the №607 precedent).
- **№611 (gh#1060, PR #1067) — the schema-as-code references: the
  qualified name parsed in EVERY spacing (the dot is the separator).**
  The parser's split kept the dot inside a token — `references(parent.id)`
  stayed one identifier and the modifier was dropped SILENTLY (no
  REFERENCES clause in the applied DDL), while `references( parent . id )`
  yielded the broken `REFERENCES parent(.)` clause (SQL_ERROR at apply,
  check green in every form). The fix (two halves, one contract): the
  parser tokenizes the qualified name with the dot as the TABLE/FIELD
  separator — every spacing of `t.f` yields exactly the pair `(t, f)`,
  anything else is a LOUD parse error, never a silent drop; `mlog check`
  validates the generated DDL on an in-memory SQLite dry-run — a DDL
  SQLite rejects fails the check LOUDLY, never the apply. The SSOT:
  `ast::schema_table_ddl` — the ONE renderer both the interpreter apply
  and the check dry-run execute. 8 regression tests.

### The 0.29 gate (the release evidence, v2)

- **№605 (gh#1046, PR #1062) — the X-3 precise-share goal, OWNER-FIXED
  3000 bp (ADR-0181 §3.1).** The audit's X-3 finding: the coarse goal
  (3500 bp) was already exceeded before the wiring, so it demanded no
  movement — "the achieved goal measures nothing". The PRECISE share
  (types without Unknown, №560) is the honest 0.29 target; the coarse
  goal stays the floor. The between-releases enforcement is the
  precise-baseline ratchet (the share may ONLY GROW; a raise happens in
  the same PR that types more rows precisely, the №565/№757 procedure).
  The honest RED record landed first (the v2 verdict NOT MET on precisely
  this parameter, 2131 bp — PR #1069) BEFORE the movement.
- **№613 (gh#1070, PR #1071) — the precise-share movement: 47 verified
  PRECISE rows, 2131 → 3042 bp ≥ 3000; the 0.29 v2 gate turns GREEN.**
  The full verification table in the naryad; the honest Unknowns kept
  (№560: `__first`/`__last`, the telegram trio, `poll_wake`/
  `take_interrupt`, `hash_password`, `query`, `respond`, the coarse
  families). The floors raised in the same PR per the №757 procedure:
  precise 2131 → 3042, general 3759 → 4670, TYPED_FLOOR 190 → 237. The
  fresh gate read at the release cut (unfreeze_gate.py --gate-target
  0.29): §4 GREEN, v2 GREEN — typed 4670/3500, precise 3042/3000, open
  High (server path) 0, domain quorum 0/8, the serve-e2e inventory done.

### The owner's decisions (the records)

- **ADR-0184 (№603, gh#1044, PR #1063) — the depth-≥2 respond boundary:
  REFUSED FOREVER, variant Б.** The owner's decision («1-Б, 2-3000,
  3-отложить и добивай остальное», the chat authorization 2026-10-06):
  `RESPOND_SWALLOWED` stays forever — the bare `respond*`/`respond_html*`
  form at depth ≥ 2 is refused at startup on BOTH backends, no early
  answer at any depth will ever be reintroduced; the migration remains
  one word (`return respond(...)`).
- **ADR-0185 (№609, gh#1050, PR #1066) — the memory_forget container
  form DEFERRED to the memory-phase planning.** The owner's decision:
  postponed, NOT rejected; the return trigger is the memory-phase
  planning, the container-form sketch stays in the ADR.

### Process

- **№608 (gh#1049, PR #1064) — the Y-4 quota line in the wave report:**
  the counter learns the real formats of the audited reports, the
  `--line` mode renders the one-line quota cell for the wave dispatch
  template; the Y-4 quota (the audit finding) is now machine-counted on
  every wave report instead of hand-asserted.
- **№610 (gh#1051, PR #1065) — the newcomer path: the broken first step
  fixed** (the clone URLs point at the real repository), the live pool
  list rendered from the good-first-issue pool (gh#951–gh#959), 9/9
  acceptance criteria.
- **The post-publication sync of 0.28.2 (PR #1059):** the
  `fact_open_high_server` trajectory closed at 0 on the live tag
  (the В31 corrective fact 0 → 1 → 0), REALITY §6.17 release state →
  PUBLISHED, both goals files carry the machine-verified facts.

## [Archived releases — 0.5.0 → 0.28.2]

Every release older than 0.29.0 lives in
[docs/changelog/CHANGELOG-2025-06_2026-10.md](docs/changelog/CHANGELOG-2025-06_2026-10.md)
— moved byte-exact by Naryad №676 (A7), no record lost, the №588
Security records of the live releases stay in this file.

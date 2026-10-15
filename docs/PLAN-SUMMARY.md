# Metalogos — public digest of the work plan

> Publisher's derived artifact (access mode §16.0-7 of plan canon v2): the list of phases,
> the current wave, criteria. The plan canon is held by the coordinator (private office repo); the public
> repository does not contain the canon. Updated by the publisher in sync with canon edits.
> Updated: 2026-10-03.

## Status

| Wave | Phase | State |
|---|---|---|
| Wave 0 (dispatch #408; naryads №316–№321) | Phase 0 "Foundation and inventory" | Executed and accepted: PR #409–#415, merge 2026-09-14, CI green |
| Wave 1 (dispatch #424) | Phase 1 "Label-checker on existing types" | Executed: the lattice, inference, sink gate, parity, dogfood (PRs merged, CI green) |
| Wave 1.5 (dispatch #446) | Audit-fixes: VM Stage 1+parity, error-protocol, adapt metric | Executed (2026-09-16) |
| Wave 2 (dispatch #465) | Phase 2 "Media handles and the backend registry" | Executed (2026-09-16) |
| Wave 3 (dispatch #491) | Phase 3 "Capability / Action security": grants, DenyEvent, Ledger v1, MCP policy | Executed (2026-09-16) |
| Wave 4 (dispatch #529) | Audit 2026-09-19 | Executed; the issue is held OPEN as the conveyor SSOT thread |
| Wave 5 (dispatch #553) | Audit 2026-09-20: RSS re-gate, version discipline | Executed (2026-09-20) |
| Wave 6 (dispatch #566) | Ledger verify hook, retained memory class, dogfood | Executed (2026-09-21) |
| Wave 7 (dispatch #570) | Native cron: core, office integration, waker | Executed (2026-09-21) |
| Wave 8 (dispatch #585) | Audit 2026-09-21 fixes: README truth-up, docs sync | Executed (2026-09-21) |
| Wave 9 (dispatch #598) | Phase 4 "Always-on, memory, forgetting": №348–№352 | Executed (2026-09-22) |
| Wave 10 (dispatch #620) | Audit 2026-09-22 + release 0.22.0 + Phase 5 start | Executed (2026-09-22) |
| Wave 11 (dispatch #633) | Audit 2026-09-23: waker repair, examples library, doc sync | Executed (2026-09-23) |
| Wave 12 (naryads №440/№441, issues #635/#636) | Phase "Forecast domain": SeriesHandle, taint passthrough, the timeseries registry class, red/green examples | Executed (PRs #640/#641, 2026-09-24) |
| Wave 13 (dispatch #645) | Phase 4 continuation: recall + honest recount + release 0.23.0 | Executed (2026-09-24, tag v0.23.0) |
| Wave 14 (dispatch #652) | The forgetting memory: forget, decay/boost/retain-ttl + recount + release 0.24.0 | Executed and accepted (2026-09-24, tag v0.24.0; the verifier acceptance in #660) |
| Wave 15 (dispatch #660) | Phase 1 continuation: the static contour — full statement-kind inference + the effects module + recount + release 0.25.0 | Executed and accepted (2026-09-25, tag v0.25.0; the verifier acceptance in #681) |
| Strategic gate (gh#680, decision 1-A, ADR-0177) | The domain freeze until 0.27 — no new subsystems and no domain extensions; the capacity goes to the core (types, dedup, debt); 0.27.0 is gated on the ADR's unfreeze criteria | In force since 2026-09-25 |
| Wave 16 (dispatch #681) | The audit 25.09 reaction: mock-LLM fail-loud, the read_file gate, the strict serve context, HARDCODED_SECRET category A, the audit synthetics, the lean front door + release 0.26.0 | Executed (PRs #697–#705, tag v0.26.0 on `01bec90`, 2026-09-26) |
| Wave 17 (dispatch #695) | The strategic decisions of the gate gh#680: the domain freeze (ADR-0177), the generative stop-list, the CI gates (dup-names, debt, type-share), the enum Type stage 0, the media isolation (the physical core→media ban), the naryad classes and the quota counter, the second maintainer's perimeter, release 0.26.1; the №466 dedup transfer continues into В18 (threshold 35) | Executed (PRs #698–#721, 2026-09-26; the closing recount — REALITY §6.11) |
| Wave 18 (dispatch #745) | The unfreeze path (ADR-0177 §4): №466 completed — the groups 5–7 leave the audit-ledger, recipe and server/runtime names into the shared live modules (threshold 35→20; the media/vision 20 ride the 0.27 split), №474 — the enum Type stage 1 (the let-type inference, warn-only), №475 — the FO-056 evidence on 0.26.1; all four §4 criteria read GREEN; the lift is the owner's; release 0.26.2 | Executed (PRs #746–#749, 2026-09-27; the closing recount — REALITY §6.12) |
| Wave 19 (dispatch #793) | The audit 28.09 (v0.27.0) reaction: distillation in serve, the fs_gate bypass via lopdf, the P0 semantic-error blocking, the voiceprint honesty, release 0.27.1 | Executed (13 naryads merged; the release shipped as v0.27.1, tag on `5b79996`, 2026-09-30) |
| Wave 20 | The reserve №498/№499 (skills/, demos/) | Reserve — not dispatched yet |
| Wave 21 (dispatch #804) | The consolidated audit over four independent runs (А/Б/В/Г): the `!=`→`==` compiler defect, the Dockerfile repair, the voice honesty, the soft-fail inventory, the registry isolation, cargo-deny, privacy.md | Executed (naryads №510–№519, PRs #824–#831, 2026-09-28–30) |
| Wave 22 (dispatch #845) | The consolidated audit А+Д 30.09 (v0.27.1) — corrective: semantic findings block run/serve (P0 №523), the voiceprint erasure path (GDPR Art. 17), the AAD binding, the LRU registry eviction, MSRV, the wildcard lint, the blocking-checks table | Executed (naryads №520–№535, PRs #853–#866, 2026-09-30) |
| Wave 23 (dispatch #852) | The typing wave to the 0.28 gate: the string/list/db/memory packages leave stage 0 (floor 998→2110 bp), the first two ops pairs collapse over DbAccess, the second-maintainer lane goes operational | Executed (naryads №536–№541, PRs #867–#871, #876, 2026-09-30) |
| Wave 24 (dispatch #888) | Development per ПСРМ after В22/В23: the cycle sync + the actuality audit №2, the typing top-up to the 3000 bp gate, the typed-stage 2 strictly behind the gate, the crate split, the 0.28 window prep | Executed (naryads №542–№549, PRs #889–#902, 2026-10-01; the 0.28 gate floor reached — 3195 bp) |
| Wave 25 (gh#911–928) | The 0.28.0 release prep: the CHANGELOG sectioning, the audit 02.10 reaction (the explicit respond_html forms, the registry-vs-corpus validator, the precise-share second floor, the tag↔main gap gate, the read_file loud refusal, the mirror widening, the generated grantors' digest, the GFI pool policy), the release cut | Executed (17 naryads, PRs #937–#963; tag v0.28.0, 2026-10-03) |
| Wave 26 (gh#931–936) | The post-0.28.0 development: the crate split stage 2 (metalogos-server lands — the transport contour + the mlog bin leave the language crate), the html_label walks unify (stage 3), the debt-honesty classification (the counter strictified, the ledger gh#967), the 0.29 gate draft + the OWNER FIXATION (ADR-0181, 2026-10-04), the coverage floor 76% line, the limitations archive fold | Executed (5 naryads, PRs #968–#982, 2026-10-03–04) |
| Wave 27 (dispatch gh#978) | Development per ПСРМ after В25+В26: the release-pipeline repair (P0 — the release workflows follow the bin, the release-bin-guard), the typing step 0.29 №1, the VM json_body serve contract, the recall lane ranking, the post-wave docs | Executed (5 naryads, PRs #984–#988, 2026-10-04; the 0.29 draft goal 3500 bp overdelivered — 3654 bp) |
| Wave 28 (dispatch gh#993) | The pre-M1 line + the domain-line reopening: the pre-M1 external-reporting sync (№577), ADR-0182 — the media-handle family + the backend-registry interface contour (№578, docs-only; the owner's §7 answers are the Wave-29 gate), the post-wave docs (№579, strictly last) | Executed (3 naryads, PRs #1006/#1012, 2026-10-05; the counters unchanged — the honest wave fact) |
| Wave 29 (dispatch gh#1004) | The security wave per the audit d63cc1d + the release line: the 0.28.1 preparation and the post-publication contour (№580–№583), the VM respond-terminality + the gate retired to advisory (№584), the route-body differential fuzzer (№585), the squash-body rule X-5 (№587), the honest-boundary protocol (№588), the post-wave docs (№589 — this row); the owner package rode in the wave's frame: v0.28.1 PUBLISHED, the 0.29 gate wired into blocking CI (№597), the ADR-0182 §7 answers (№598, the ADR → Accepted), the Image bridge (№599, the step 1 of 3); №586 stays blocked by the owner's repo secret | Executed (№580–№585, №587–№589 merged; №586 blocked — the owner's secret; the counters unchanged, the release state: v0.28.1 live) |
| Wave 30 (dispatch gh#1022) | The language enrichment for the Камертон consumer: the spectral contour (№591), the per-call HTTP deadline taxonomy (№592), the migration-rollback boundary ADR-0183 — the OWNER GATE resolved by delegation, variant B (№593), the UTC calendar arithmetic (№595), the Box–Muller normal sampler (№590, merged last — the merge-order rebase with the cumulative baselines), the tick↔route parity — the third diff axis, tests-only (№594), the post-wave docs (№596 — this row) | Executed (№590–№595 merged, PRs #1034–#1039; the registry 509→516, ALL seven new rows typed; typed 3654 → 3759 bp, precise 2043 → 2131 bp; the classification generator's OVERRIDES drift repaired) |

## List of phases (no rationale, no timelines)

1. Phase 0 — Foundation and inventory (executed).
2. Phase 1 — Label-checker on existing types (executed; the continuation landed in Wave 15).
3. Phase 2 — Media handles and the backend registry (executed).
4. Phase 3 — Capability / Action security (executed).
5. Phase 4 — Always-on, memory, forgetting (executed; the continuation in Waves 13–14).
6. Phase 5 — Embodied, sim-only (behind the entry gate: the GPU budget).
7. Phase 6 — Spatial / XR.
8. Phase 7 — Edge deepening + verifier.

The recent waves (16–27) run inside the strategic frame: the domain freeze
(ADR-0177, gh#680 — LIFTED by the owner's decision 2026-09-27, the
unfreeze gate GREEN per №568) and the release gate succession
(ADR-0179 → ADR-0181) — the capacity goes to the
core (types, dedup, debt, parity); the Phase-2 domain lines remain the
owner's call, not a wave's.

The sequence, the phase rationale, and the composition of subsequent waves are not subject to publication —
they reside in the canon held by the coordinator (§16.0-7).

## Wave 0 results

- Classification of 421 builtins (role × label × reversibility) — `src/builtins_classification.rs` (#316).
- Leak-suite: corpus of negative/positive programs and a runner — `tests/run_leak_suite.rs` (#317).
- Honest readiness report — `docs/REALITY.md`: P0 ≈ 26%, subsystem weights marked UNVERIFIED (#318).
- Synchronization of `REFERENCE.md` (421 builtins) + ADR-0154–0161 reserve (#319).
- C2PA mini-slice: manifest read/write, Art 50 labeling — a narrow slice (#320).
- Completion-audits of Wave 0 — accepted (gh#404–#406).

## Executed — Wave 41 (the honest-boundaries-machine wave, the dispatch gh#1205, 2026-10-14)

The dispatch gh#1205 queue — the honest boundary line as one wave.
Executed: №673 (gh#1200, PR #1247 → 9b8cdc5) — the BREAK_OUTSIDE_LOOP
semantic gate: the two silent-wrong edges of the №658 probe refuse
BEFORE execution on both backends (BREAK_OUTSIDE_LOOP /
CONTINUE_OUTSIDE_LOOP; the value channel included), the №658-a/№658-b
spec records REPAIRED with the anchors; №674 (gh#1201, PR #1248 →
250ef31) — the Honest-limits spec scan rides the SAME blocking job
(retrospective mode): every ACTIVE record must have a limitations.md
mirror by id — 7 records, 2 active (№659-x/№659-y), IN SYNC; the
659-x/659-y mirrors are the №588-form rows (the repairs stay outside
the wave); №675 (gh#1202, PR #1249 → 8912c9d) — the ignore floor 11 →
9 (the machine fact) PLUS the below-floor rule in the debt gate (a
fact below the floor = exit 1 naming the old/new floors, the floor
lowered in the same PR); №676 (gh#1203, PR #1250 → b072748) — the
audit 7c3456b §4 hygiene checklist, all 11 rows (A3 the porcelain
gates in test-lib AND test-integration; D2 the README front-door 149
lines, Quick Start at line 29; A7 the CHANGELOG archive 727 → 57 KB,
42 sections byte-exact, the Security records of the live releases
stay; A1 frames/ + the two committed data/ leaks out; A6 the root at 8
.md; A5 the GEMINI byte-duplicate deduped; D3 the link-not-duplicate
with the §3/§5 machine-pinned claims kept; D1 the generated-stamp off
+ the grep floor; A2 the repo-writing tests on the №387 sandbox form;
C1 the MSRV claims synced; D5 the CI badge rendered from ci.yml 28 →
44) — and the gate caught two real writers ON ITS FIRST FULL SUITE
(the №402 ledger export re-committed weekly; the masked №394 file);
№677 (this record; REALITY §6.26; the quota line 0/15 PASS; the
dispatch gh#1205 closes). The counters at b072748: typed 337/526 =
6406 bp, precise 227/526 = 4315, parameterized 77/109 = 7064,
fieldmeta 46/48 = 9583, sensitive 73/145 = 5034 (the §6.25 → b072748
deltas belong to the PE-1/PE-2 lanes), norms 33 (13+14+6) with 40
pairs, the Honest-limits scan NEW (7 records, 2 active, IN SYNC),
ignore 9 AT the floor 9 / dead_code 15, ADR 184, dups 0, blocking
cells 32, release-gap OK, the 0.30 gate --strict exit 0. The owner
gates remaining: the ADR-0191 §-answers (the OD variants), the train
№2 candidate (19.10 — the [Unreleased] classification = the 0.31.0
MINOR), №639 (15.10) / №648 (01.12) outside the wave.

## Current wave — the open machine tail (2026-10-15)

The in-flight machine surface: **№682 (gh#1233)** — verified-wait
(the App C leg is proven on the fixed core; the landing gates on the
owner's release act → the pin bump; see the issue's machine trail);
**№648 (gh#1135)** — the dated plan audit (2026-12-01). The В36 tail
is closed (№639 executed 2026-10-15, №641 this sync).
The showcase wave's record: the «Executed — Wave 42» section below and
REALITY §6.27.

The 0.32 goal DRAFT (№688, ADR-0193 **Proposed**): the typed-source
goal moves to the FIXED-LIST closure form — «≥ 20 of the 67 untyped
sources recorded at `f92c61c`, the denominator out of the goal» — the
machine list `scripts/ci/gate_032_untyped_sources.txt` + the
`--goal032` counter (GREEN at the honest 0/20 start). The figure, the
priority, and the gate wiring await the OWNER's verdict (the №525
rule — nothing self-captures).

## Executed — Wave 42 (the showcase wave, the dispatch gh#1235, 2026-10-13)

The dispatch gh#1235 queue — the showcase charter (ADR-0190) executed
in the OWNER's separate public repository `ShkodnikAI/metalogos-examples`
(the cross-repo lane, parallel to the PE line; the core untouched
except this docs sync, the №683 exception). Executed: №679 (gh#1230,
showcase PR #1 → `fcd369a`) — the scaffold: the charter README (the
three apps, the honesty rule §2.4, the pin policy §3(а) «pin +
scheduled drift-check», the boundary with the core's `examples/` and
GFI #951), the licenses verbatim from the pin v0.30.1, the
loudly-empty layout, the blocking both-backend CI + the weekly
drift-check (the write-grant refusal was flagged no-progress per the
naryad's premise, then the owner unlocked Contents: R&W — the same
commit pushed unchanged); №680 (gh#1231, showcase PR #2 → `e9155a3`)
— App A the stateful serve application: 62 lines, the typed-memory
arc, **the restart arc persist → query → restart → query green on
BOTH backends** (the fresh-process persistence through
`METALOGOS_MEMORY_DB` + `METALOGOS_MEMORY_MASTER`), the transcripts
byte-identical TW == VM, the blocking CI + the nightly stress leg
(5 notes × 3 restarts × both backends); №681 (gh#1232, showcase PR
#3 → `bef13e6`) — App B the bot on mlogpkg: the mlogpkg package
(manifest + `src/main.mlog`), the HOOK + POLL intake over the
per-chat dialog state machine, the outbound behind the deterministic
`[AUDIT]` stub, the hook+poll transcripts identical TW == VM; the
mlogpkg binary is not among the pinned release assets — the leg
exercises the surface the pin carries (the manifest shape + `mlog
check`, the documented boundary). **The deviation:** №682 App C —
**STOPPED no-progress** (the flag gh#1233): the VM compiler cannot
resolve tool-method qualified calls (the compile-time divergence: TW
runs `42`, `mlog compile` fails `qualified call 'calc.double()' —
not found`; the compiler skips `Declaration::Tool`, the QualifiedCall
resolution has no tool branch, the tool contract tests are TW-only).
Per the parity rule §2.4 — STOP, the oracle not adjusted, the
divergent code did not enter the showcase; the unlock is the
conveyor's decision (the core naryad for the VM tool resolution, or
the charter amendment for the App C subset). №683 (gh#1234, PR → this
merge) — the wave tail: REALITY §6.27 + this section + the core
README's showcase link (the №676-D2 line-budget protocol in the
naryad report). The counters: the core machines on the base `d403832`
— typed 328/519, precise 224/519, parameterized 71/103, fieldmeta
44/46, sensitive 67/139 (the movements by the parallel PE line are
attributed to it, never to В42); ADR 183, dups 0, mirrors 6, blocking
cells 32, release-gap OK. The owner gates remaining: the №682 unlock
decision, the ADR-0191 §-answers, the train №2 (19.10).

## Executed — Wave 39 (the verdict-execution + repair, the dispatch gh#1167, 2026-10-09)

**Executed and accepted** (the post-wave sync №665 → `dcb9324`; the wave record: REALITY §6.24; 7/7 naryads incl. the train №1 at the window). The dispatch gh#1167 queue —

The dispatch gh#1167 queue — the owner's verdict session 08.10 as
workorders + the №652-a/b/c repair line + the train №1. Executed:
№658 (gh#1159, PR #1168 → 3f9fee3) — the №652-a repair: the VM
break/continue parity (the loop-stack fixups on BOTH backends; the
norms S-BLK-011/012 + five pairs; the ### Security entry rides the
0.30.1 cut); №659 (gh#1160, PR #1171 → 5c5a1a1) — the №652-b/c
repairs: the trailing let in a value arm is Unit, the arm lets do not
leak (S-BLK-013/014); №660 (gh#1161, PR #1173 → aea5cc5) — ADR-0191
the 0.31 gate skeleton (Proposed; gate_031_goals.txt DRAFT, both OD
variants); №661 (gh#1162, PR #1174 → 2da1e50) — the К-Б enforcement
stage-2: the blocking field-meta rule + the FIRST PRIVATE label
(canary_check) + the floors in the same PR; №662 (gh#1163, PR #1175 →
ddca7ca) — ADR-0188 option (b): the SHARED compile-time checker (the
implicit String→opaque coercion + the unknown template names) —
UNRELEASED, the 0.31.0 material (the [Unreleased] Changed; the
#[ignore] debt 11 → 9); №663 (gh#1164, PR #1172 → c1b1a72) —
ADR-0189/0190 → Accepted, the The-train checklist section, the train
№1 activation; №664 (gh#1165, PR #1193 → 27877b8) — **the train №1 at
the window 2026-10-12: the 0.30.1 cut ([0.30.1] - 2026-10-12 —
### Security №658+№651, ### Fixed #1169/#1170; the lockstep ×5; the
gate --strict exit 0; CI 48/48), the tag v0.30.1 ON THE MERGE COMMIT —
the publication is the OWNER's act, §6.5, PENDING**. The post-wave
docs sync: №665 (this section; REALITY §6.24; the quota line 0/24
PASS). The counters at 9f5cee7: typed 326/516 = 6317 bp, precise 4302,
parameterized 71/103 = 6893, fieldmeta 44/46 = 9565, sensitive
66/138 = 4782, norms 33 (13+14+6) with 40 pairs, ignore 9 / dead_code
15, ADR 183, dups 0, blocking cells 32, release-gap OK, the 0.30 gate
--strict exit 0. The owner gates remaining: **the v0.30.1 publication
(§6.5)**, the ADR-0191 §-answers (the OD variants), the В36 tail
(№639 15.10, №641), №648 (01.12), the showcase
repository creation (owner).

## Executed — Wave 38 (the parity-and-spec development, the dispatch gh#1151, 2026-10-08)

**Executed and accepted** (the post-wave sync №656 → `b4b9abb`; the wave record: REALITY §6.23; the window carried the №652-a/b/c repair naryads — see Wave 39). The dispatch gh#1151 queue —

The dispatch gh#1151 queue — the ПСРМ development after the 0.30.0
publication. Executed: №651 (gh#1145, PR #1152 → 3ac294a) — the TW/VM
condition parity: the №645-a divergence repaired fail-closed (the new
append-compat JumpIfNotCond; both VM loops call the SAME Value::as_bool;
the TW refusal stamped [TYPE_MISMATCH]; the escape-hatch not applied —
no live dependencies found), the norm S-VAL-013 + 3 pairs + the №503
condition class + the ### Security entry (AFFECTS 0.30.0 AND EARLIER,
VM — the №562 gate counts); №650 (gh#1144, PR #1153 → a854c2b) — the
Q-2 movement №1 in the §6.3 audit order: 7 of 11 rows closed against
the verified handler facts, 4 honest refusals recorded, **the
sensitive surface 46/138 = 3333 bp → 53/138 = 3840 bp** (the floors
rose with the rows: general 6124, parameterized 6170 (58/94),
fieldmeta 9512 (39/41), precise 4302; the locks 312/55/38; the fact
twins synced across both gate records; the 0.31 DRAFT not touched,
№525); №652 (gh#1146, PR #1154 → a521432) — the spec topic 2: 10 norms
S-BLK-001..010 (the normative core 13 + 10 = 23), the probe found
THREE honest divergences (№652-a the VM ignores break/continue in
while; №652-b the trailing-let value; №652-c the VM arm-let leak) —
recorded in the Honest limits as the candidate repair naryads; №653
(gh#1147, PR #1155 → 55b95a6) — the №604 dictionary calibration: the
class 4 (silently-wrong) — the literal №629 fixture CAUGHT now, the
retro IN SYNC (limitations 79 rows, CHANGELOG 1238 entries, 0
[Unreleased] matches), zero false positives; №654 (gh#1148, PR #1156 →
9fb190b) — ADR-0189 the release train (Proposed): the Monday cadence
(the first candidate 2026-10-12), the nine window steps in the
checklist, the honest-content rule (no empty trains), the
version-number semantics, the release-block hold; №655 (gh#1149, PR
#1157 → 7036e8e) — ADR-0190 the examples showcase charter (Proposed):
three apps with e2e, the parity-as-display rule, the placement
§-question to the owner. №656 (gh#1150) — this sync, strictly last.
The counters: sensitive 3840 bp, typed 6124, precise 4302,
parameterized 6170, fieldmeta 9512, the normative core 23 norms, the
dictionary classes 1–4, ignore 11 / dead_code 15, ADR 182, dups 0,
blocking cells 32, release-gap OK, the 0.30 gate --strict exit 0,
v0.30.0 PUBLISHED. The owner gates remaining: the ADR-0189 §-answer
(the cadence), the ADR-0190 §-answers (the placement + the app list),
the К-Б stage-2 verdict, the №652-a/b/c repair naryads (the semantics
gate), the Камертон forwarding, the В36 tail
(№639 15.10, №641), №648 (01.12).

## Executed — Wave 37 (the unified-audit execution, the dispatch gh#1136, 2026-10-08)

The dispatch gh#1136 queue — the unified audit of 48301708 (the sources
А + К, §6.6) landed as workorders. Executed: №642 (gh#1129, PR #1137 →
81177c8) — the `### Security` section in [Unreleased]: the №629 entry
with the AFFECTS-0.29.0-AND-EARLIER stamp and the date stamps, the №562
gate moved from no-security-part to age-counting; №643 (gh#1130, PR
#1140 → 55bce5bb) — the №604 security dictionary extends to CHANGELOG.md
(a dictionary entry in [Unreleased] outside ### Security without the
release-block evidence = exit 1; the released sections pass by the
release — the M-3 scope calibrated with the fact; the retro 1237
entries → IN SYNC; self-test 17/17); №644 (gh#1131, PR #1138 →
636b3cbb) — the sensitive-surface coverage metric, the fixed
denominator 138 (Source ∧ ≠ Public), covered 46/138 = 3333 bp, the
only-up floor, the RECORD line + the 0.31 DRAFT (4130 bp) in the gate
record — the CI wiring advisory (№535); №645 (gh#1132, PR #1139 →
8a5d13d5) — the normative spec + the conformance layer: docs/spec/
(12 norms S-VAL-001..012 with the TW+VM anchors), tests/conformance/
(12 pairs, the cross-backend agreement as the oracle), the BLOCKING
conformance CI job, the №535 cell 31 → 32; №646 (gh#1133, PR #1141 →
6eafee1c) — the release lockstep 0.30.0: the version lockstep in one
PR, the CHANGELOG cut [0.30.0] - 2026-10-08 with ### Security first,
release_gap_gate OK (the №629 tag↔main gap closed), the fresh gate read
GREEN + --strict exit 0, the tag v0.30.0 on 6eafee1c — **THE
PUBLICATION IS THE OWNER'S ACT (§6.5): the release is STAGED; the
Z-3 recommendation attached — the branch-protection switches (gh#1000)
BEFORE publishing**; №647 (gh#1134) — this sync, strictly last. The
dated workorder №648 (gh#1135, the plan-relevance audit release №4) —
2026-12-01 (± 1 day), verified-wait. The counters: blocking cells 32,
sensitive-surface 3333 bp (new), release-gap OK, the 0.30 gate --strict
exit 0, parameterized 6043 bp / field-label 9500 bp (the 0.30 goal
MET), typed 5988 bp / precise 4224 bp, ignore 11 / dead_code 15, ADR
180, dups 0. The owner gates remaining: the 0.30.0 publication, gh#1000
(the branch protection — then the fact-key micro-PR, №525), the stage-2
enforcement (К-Б), the Камертон forwarding, the В36 tail (№639 the ledger revision 2026-10-15, №641
the В36 docs sync), №648 (01.12).

## Executed — Wave 36 (the parameterized movement №3, the dispatch gh#1125, 2026-10-08)

The dispatch gh#1125 queue (the ПСРМ development after В35). Executed:
№637 (gh#1120, PR #1126 → e7894a1) — the parameterized movement 0.30
№3: the PDF contour, 21 verified rows (the 20 Struct rows WITH
field_meta — the anti-dilution rule), the share 34/91 = 3736 bp →
**55/91 = 6043 bp — THE 0.30 OWNER-FIXED GOAL (≥ 5000 bp) REACHED**
(the floors raised in the same PR: the parameterized baseline 6043,
PARAM_FLOOR 52); №638 (gh#1121, PR #1128 → 4830170) — the field-labels
№2: the 15 existing parameterized Struct rows labeled per the №627
table read off the handlers, 5750 → **9500 bp (38/40 = 95%)**
(FIELDMETA_FLOOR 37); №640 (gh#1123, PR #1127 → bddc4cc) — the ADR-0188
DRAFT (the template-semantics composition, the ledger §7 lane):
Proposed, owner_fixed: false, no implementations, ADR 179 → 180. The
dated tail EXECUTED: №639 (gh#1122) — the ledger gh#967 revision
executed at its dated window 2026-10-15 (the §3 book-sync restored,
the §2 path fixed, the §4 parked with evidence, the §7 partially
resolved — the closing act carries the marker
`<!-- w36: ledger-revision -->`); №641 (gh#1124) — the В36 docs sync,
executed 2026-10-15 (REALITY §6.28; the dispatches gh#1089 and
gh#1125 closed).
The counters: parameterized 6043 bp (moved), field-label 9500 bp
(moved), ADR 180, the rest unchanged from В35.

## Executed — Wave 35 (the ПСРМ development after В34, the dispatch gh#1105, 2026-10-07)

The dispatch gh#1105 queue (7 workorders) plus the registered
candidate №627 (the owner's К-А verdict execution, 2026-10-07).
Executed: №630 (gh#1098) — the §5 MECHANICAL TRANSITION of the 0.30
gate in one PR: `owner_fixed: true`, ADR-0186 Accepted, the DEFAULT
gate target 0.29 → 0.30 (the №597 pattern; the typed parameter = the
parameterized share ≥ 5000 bp — the machine read on the merged main is
RED-loud, the movement is the requirement); №631 (gh#1099) — the
parameterized movement №2: the second honest package of 27 verified
rows, the share 7/91 = 769 bp → 34/91 = 3736 bp, the floors raised in
the same PR (PARAM_FLOOR 7 → 31); №633 (gh#1101) — the dead_code burn
33 → 15, the floor re-locked; №634 (gh#1102) — the self-host lane
(the gh#967 §5) lifted green: the FIXTURE had drifted, not the parser;
the ignore floor 14 → 13; №635 (gh#1103) — the VM template_render lane
(the gh#967 §6): both vm_golden sweeps 189/189 green, the ignore floor
13 → 11; №627 (gh#1110) — the field-label METADATA (the stage-2 prep,
ADR-0178): the form `Struct<Name>{field:label,...}`, the registry
side-table `BuiltinSpec.field_meta` (the enum NOT extended), the
fail-closed parser armor, the FOURTH metric — the field-label share
3/20 = 1500 bp (the first honest package of 28 verified rows:
GeoLocation 9, Weather 13, LlmUsage 6; the parser mirrors synced in
the same PR — the gen_metrics regex drift caught); №636 (gh#1104) —
this sync, strictly last. The dated workorder №632 (gh#1100, the
plan-relevance audit release №3) — the execution date 2026-11-01
(± 1 day), verified-wait. The counters: ignore debt 11, dead_code 15,
typed 5988 bp / precise 4224 bp unchanged, parameterized 3736 bp
(moved), field-label 1500 bp (new), ADR 179, mirrors 6, dups 0,
blocking cells 31. The owner gates remaining: gh#1000 (the branch
protection), the parameterized movement to ≥ 5000 bp, the stage-2
enforcement (К-Б — not filed), the ledger gh#967 §4/§7 (the revision
2026-10-15), the Камертон forwarding.

## Executed — Wave 34 (the verdict-execution wave, the dispatch gh#1089, 2026-10-07)

The OWNER's 2026-10-07 verdict executed («принимаю форму довеска В34
как основу…» — the verbatim record in the 0.30 gate file) plus the
dispatch queue. Executed: №626 (gh#1093) — the 0.30 gate draft synced
with the В34 addendum (the typed parameter = the parameterized share
among List/Struct, the benchmark ≥ 50% of 84; the Z-2 ratchet demotion;
the owner's verdict verbatim); №620 (gh#1083) — the Security ADVISORY
appended to the published 0.29.0 (the CHANGELOG + the release body
verbatim; the tag/assets untouched); №621 (gh#1084) — the fuzzer
produces the №612 class AND the honest finding: the sweep was vacuously
green (0/150 executed) — repaired + the permanent LIVENESS ratchet (≥
60% both-backend); №629 (gh#1096) — the repair born from that finding:
the VM comparison parity (the heterogeneous Eq and the non-Float
ordering refuse on the VM — the silent `false` closed fail-closed);
№622 (gh#1085) — the implicit-block-value class scan (sandbox/flow —
N/A: no statement body in the grammar) + the parity fix: `return`
inside a value-channel arm is CAPTURED on the VM now
(`Instruction::SetValueReg`); №623 (gh#1086) — the parameterized
signatures stage-0 (`List<T>`/`Struct<Name>`; the 7 verified rows; the
THIRD metric 7/91 = 769 bp; the floors raised: general 5988, precise
untouched per Z-2); №624 (gh#1087) — ADR-0187 (Proposed): the
memory-phase plan; №625 (gh#1088) — this sync, strictly last. The owner
gates remaining: gh#1000 (branch protection — blocks №628, the §5
fixation), the stage-2 enforcement (ADR-0178), the ledger gh#967
§4–§7, the Камертон forwarding. The wave-35
registered candidates: №627 (gh#1110), №628 (gh#1111).

## Executed — Wave 33 (the ПСРМ development after the 0.29.0 release, the dispatch gh#1080, 2026-10-06)

The first DEVELOPMENT wave after the В31 corrective + В32 release line
(the owner's trigger: «Возвращаемся к Металогос, пиши следующую волну
нарядов согласно ПСРМ, заливай в issues»). Executed: №617 (gh#1075) —
the gh#967 §3 lane in the implementation: the variable-scope walk + the
static opaque-concat check in the semantic checker (the three
checker-failure tests lifted green; the debt floor 17 → 14; the fuzzer
lane honesty fix — run_vm carries the №523 gate); №618 (gh#1076) — the
Камертон closure report (the SHA256 re-run 3/3 OK, the R1–R6 verdicts
from real runs, the mutation probe goes red; the forwarding — the
owner's gate); №616 (gh#1077) — the PRECISE movement 0.30 №1: 61
verified rows, precise 3042 → 4224 bp, general 4670 → 5852 bp,
TYPED_FLOOR 237 → 298; №615 (gh#1078) — the 0.30 gate DRAFT
(gate_030_goals.txt, owner_fixed: false, the X-3 floors strictly above
the live facts; the default gate target stays 0.29) + ADR-0186
(Proposed); №619 (gh#1079) — this sync, strictly last. The owner gates
remaining: the 0.30 parameter fixation (№615/ADR-0186 — owner_fixed:
false → true), the Камертон report forwarding (№618), the type stage 2
(ADR-0178), branch protection (gh#1000), the Phase-2 domain lines.

## Executed — Wave 32 (the 0.29.0 release, the owner's verdict 2026-10-06)

The wave executes the release on the owner's verbatim chat verdict
(«Делай релизный наряд 0.29.0», 2026-10-06; the publication itself is
the owner's act per §6.5, the v0.28.2 precedent). The executor's chain:
№614 (gh#1072) — the version lockstep 0.29.0 in ONE PR (the №583
pattern: workspace Cargo.toml + Cargo.lock ×5 members + the README
badge + the generated Version row and the SSOT blocks), the CHANGELOG
[0.29.0] cut, the fresh 0.29 v2 gate read GREEN (unfreeze_gate.py
--gate-target 0.29, exit 0), the tag v0.29.0 on the squash commit, the
release notes per the v0.28.2 shape, the four assets verified (the SBOM
describes `mlog 0.29.0`), the post-publication syncing PR (the №583/PR
#1059 pattern — REALITY §6.18 STAGED → PUBLISHED). The wave so far:
№605 (the X-3 precise goal 3000 bp OWNER-FIXED, ADR-0181 §3.1) → the
honest RED record (#1069) → №613 (gh#1070) — 47 verified PRECISE rows,
2131 → 3042 bp ≥ 3000, the 0.29 v2 gate GREEN (the ratchet: precise
3042 / general 4670 / TYPED_FLOOR 237). The owner gates remaining:
branch protection (gh#1000), the LLM-line sequencing (ADR-0182 §7.5),
the 0.30 string deprecation, №503 (strictly last).

## Executed — Wave 31 corrective (dispatch gh#1052, 2026-10-05)

The wave is the CORRECTIVE line over the unified audit of `25b375e`
(v0.28.1): №600 (gh#1041, P0, release-block) — the audit §3 Y-1 regression
closed: a bare respond* NESTED under a top-level block-form if/else branch
(the guard-bypass shape the audit reproduced) refuses run/serve at startup
again with the blocking `RESPOND_SWALLOWED` error — the fail-closed posture
№584 had retired for ALL forms at once; №601 (gh#1042, P0) — the swallow
test contract rewired, the audit scenario pinned end-to-end on both
backends; the execution-honesty rider — the №601-mandated test exposed the
PRE-EXISTING TW divergence on the migration path (a nested explicit
`return respond(...)` was silently discarded while the VM answered it),
repaired in the same PR (the honest-boundary marker, the №588 protocol).
№586 (gh#1000, P2) — the X-4 branch-protection audit job landed; the first
read fixated the real divergences (the owner's admin toggles, gh#1000).
The release line: the CHANGELOG [0.28.2] Security section cut (this
naryad, №602) — the publication is the OWNER's act (§6.5); the
`fact_open_high_server` trajectory 0 → 1 (the honest №600 carrier) → 0
(the №602 closure on the live tag). The remaining wave naryads: №604 ∥
№605 (the owner's precise-goal value) ∥ №606 ∥ №607 → №608 ∥ №609 (the
owner's №503 decision) ∥ №610; №603 (the depth-≥2 semantics fate) — the
owner's ADR gate.

## Executed — Wave 30 (dispatch gh#1022, 2026-10-05)

<!-- BEGIN GENERATED NUMBERS (scripts/gen_metrics.py — do not edit inside) -->
| Machine fact (generated — do not hand-edit) | Value |
| --- | --- |
| Typed-signature floor — the 0.28-gate line (ADR-0179) | 6577 bp — 346/526 = 65.77% (precise 234/526 = 44.48%) |
| BUILTIN_REGISTRY rows | 526 |
<!-- END GENERATED NUMBERS -->

## Acceptance criteria (public part of the methodology)

- The naryad contract — AGENTS.md §8: branch → PR → blocking CI green on the merge commit.
- "No stubs" (#16.0-D): `grep todo!|unimplemented!|SKELETON` over new files — 0.
- Completion-audit before closing: the evidence for every "Done, when" item
  is reproduced on the merge commit.
- Reporting: progress / verified-wait / no-progress; three no-progress in a row → blocked-audit.

## Where to look in the repo

- `docs/REALITY.md` — the honest readiness report (updated by the naryads of the wave).
- `REFERENCE.md`, `CHANGELOG.md`, `docs/adr/` — language documentation and decisions.
- `docs/PLAN-SUMMARY.md` — this file (updated by the publisher).

## What is not in the public repo (§16.0-7 sanitization)

Market analysis, the rationale for the sequence and the priorities, forward-looking bets, the risk register,
the full wave registry — not published; questions about them go through the owner/coordinator.

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
| Wave 24 (dispatch #888) | Development per ПСРМ after В22/В23: the cycle sync + the actuality audit №2, the typing top-up to the 3000 bp gate, the typed-stage 2 strictly behind the gate, the crate split, the 0.28 window prep, the NLnet traction | Executed (naryads №542–№549, PRs #889–#902, 2026-10-01; the 0.28 gate floor reached — 3195 bp) |
| Wave 25 (gh#911–928) | The 0.28.0 release prep: the CHANGELOG sectioning, the audit 02.10 reaction (the explicit respond_html forms, the registry-vs-corpus validator, the precise-share second floor, the tag↔main gap gate, the read_file loud refusal, the mirror widening, the generated grantors' digest, the GFI pool policy), the release cut | Executed (17 naryads, PRs #937–#963; tag v0.28.0, 2026-10-03) |
| Wave 26 (gh#931–936) | The post-0.28.0 development: the crate split stage 2 (metalogos-server lands — the transport contour + the mlog bin leave the language crate), the html_label walks unify (stage 3), the debt-honesty classification (the counter strictified, the ledger gh#967), the 0.29 gate draft + the OWNER FIXATION (ADR-0181, 2026-10-04), the coverage floor 76% line, the limitations archive fold | Executed (5 naryads, PRs #968–#982, 2026-10-03–04) |
| Wave 27 (dispatch gh#978) | Development per ПСРМ after В25+В26: the release-pipeline repair (P0 — the release workflows follow the bin, the release-bin-guard), the typing step 0.29 №1, the VM json_body serve contract, the recall lane ranking, the post-wave docs | Executed (5 naryads, PRs #984–#988, 2026-10-04; the 0.29 draft goal 3500 bp overdelivered — 3654 bp) |
| Wave 28 (dispatch gh#993) | The pre-M1 line + the domain-line reopening: the pre-M1 NLnet/Restack grants sync (№577 — the delivery 15 days before the 2026-10-20 deadline, M1 2026-11-03), ADR-0182 — the media-handle family + the backend-registry interface contour (№578, docs-only; the owner's §7 answers are the Wave-29 gate), the post-wave docs (№579, strictly last) | Executed (3 naryads, PRs #1006/#1012, 2026-10-05; the counters unchanged — the honest wave fact) |
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

## Current wave — Wave 33 (the ПСРМ development after the 0.29.0 release, the dispatch gh#1080, 2026-10-06)

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
(ADR-0178), branch protection (gh#1000), the Phase-2 domain lines,
NLnet M1 03.11.2026.

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
| Typed-signature floor — the 0.28-gate line (ADR-0179) | 5988 bp — 309/516 = 59.88% (precise 218/516 = 42.24%, №560) |
| BUILTIN_REGISTRY rows | 516 |
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

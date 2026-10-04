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
| Wave 27 (dispatch gh#978) | Development per ПСРМ after В25+В26: the release-pipeline repair (P0 — the release workflows follow the bin, the release-bin-guard), the typing step 0.29 №1, the VM json_body serve contract, the recall lane ranking, the post-wave docs | In force since 2026-10-04 |

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

## Current wave — Wave 27 (dispatch gh#978, 2026-10-04)

The development cycle per ПСРМ after the executed В25+В26 (the verification
conveyor t91_state.py: В25 17/17 closed with cross-ref PRs, В26 5/5 closed
with completion-audits, the tag v0.28.0 published with the binary + the
CycloneDX SBOM + Sigstore): the release-pipeline repair (№572, P0 — the
release workflows follow the mlog bin into metalogos-server, the
release-bin-guard blocking job closes the "release workflows are covered by
nothing" gap the №567 regression exposed), the typing step 0.29 №1 (№573,
P1 — the №543 line toward the owner-fixed 3500 bp goal of ADR-0181), the
VM json_body serve contract (№574, P1 — the ledger gh#967 §2 lane), the
recall lane ranking and the forget/recall consistency (№575, P2 — the
ledger gh#967 §1 lane), and the post-wave docs sync (№576, P2 — strictly
last). Naryads: №572–№576 (gh#973–#977). The wave rules: the typing floor
moves only up; the debt/pairs/mirrors/stop-list only down; the 9 ignore
lifts (№574+№575) re-lock the floor down per the №569 procedure; no domain
opens by a naryad.

<!-- BEGIN GENERATED NUMBERS (scripts/gen_metrics.py — do not edit inside) -->
| Machine fact (generated — do not hand-edit) | Value |
| --- | --- |
| Typed-signature floor — the 0.28-gate line (ADR-0179) | 3654 bp — 186/509 = 36.54% (precise 104/509 = 20.43%, №560) |
| BUILTIN_REGISTRY rows | 509 |
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

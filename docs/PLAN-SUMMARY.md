# Metalogos — public digest of the work plan

> Publisher's derived artifact (access mode §16.0-7 of plan canon v2): the list of phases,
> the current wave, criteria. The plan canon is held by the coordinator (private office repo); the public
> repository does not contain the canon. Updated by the publisher in sync with canon edits.
> Updated: 2026-09-25.

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

## List of phases (no rationale, no timelines)

1. Phase 0 — Foundation and inventory (executed).
2. Phase 1 — Label-checker on existing types (current).
3. Phase 2 — Media handles and the backend registry.
4. Phase 3 — Capability / Action security.
5. Phase 4 — Always-on, memory, forgetting.
6. Phase 5 — Embodied, sim-only.
7. Phase 6 — Spatial / XR.
8. Phase 7 — Edge deepening + verifier.

The sequence, the phase rationale, and the composition of subsequent waves are not subject to publication —
they reside in the canon held by the coordinator (§16.0-7).

## Wave 0 results

- Classification of 421 builtins (role × label × reversibility) — `src/builtins_classification.rs` (#316).
- Leak-suite: corpus of negative/positive programs and a runner — `tests/run_leak_suite.rs` (#317).
- Honest readiness report — `docs/REALITY.md`: P0 ≈ 26%, subsystem weights marked UNVERIFIED (#318).
- Synchronization of `REFERENCE.md` (421 builtins) + ADR-0154–0161 reserve (#319).
- C2PA mini-slice: manifest read/write, Art 50 labeling — a narrow slice (#320).
- Completion-audits of Wave 0 — accepted (gh#404–#406).

## Current wave — Phase 1 "Label-checker"

Phase goal: static information-flow labels (conf, integrity, consent-scope) on
the existing types of the language — a label lattice, control-flow inference, a sink gate with
the `legacy` compatibility profile, redact/declassify, anti-injection, runtime parity,
dogfooding of the office loop. Naryads: #322–#329 (gh#416–#423), the dispatch with rules
and acceptance — gh#424. The public Go criteria of the phase — in dispatch gh#424.

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

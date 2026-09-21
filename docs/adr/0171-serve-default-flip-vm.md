# ADR-0171: `mlog serve` default backend flip — the VM becomes the default (Stage 5 executed)

**Status:** Accepted
**Date:** 2026-09-21
**Naryad:** №404 Stage 5 (issue #527 — the owner-decision issue; the flip directive is recorded there as `w6: n404-flip-directive`, comment 5754930273)
**Depends on:** ADR-0088 (the serve backend switch), ADR-0141 (the staged plan — Stages 1–4 executed, Stage 5 gated on the owner), ADR-0105 (reservations superseded per ADR-0141 D7), the №398 re-gate protocol series (№404, №410, №415), №402 (the `Arc<Program>` shared snapshot), №409 (the per-request footprint compression), №415/№416 (the retained-representation compression)

## 1. Context

ADR-0088 shipped the VM serve backend as opt-in (`METALOGOS_SERVE_BACKEND=vm`) with the tree-walking interpreter as the default. ADR-0141 (D5–D6) made the default flip conditional on the staged plan: Stage 1 (the four language gaps) closed by №369–№372, Stage 2 (the parity gate) green by №373, Stage 4 — a real-load benchmark meeting the two-threshold gate — and Stage 5 (the flip itself) reserved for an explicit owner decision, to land as a separate ADR with the ADR-0088 status amendment recorded here, not inside ADR-0088.

Stage 4 took three re-gate series under the №398 protocol (thresholds fixed by rule 2: p95(VM/TW) ≥ ×1.5 AND peak RSS(VM/TW) ≤ ×1.1, 3/3 pinned runs, rounds=30, divisor 14 declared before the runs):

| Series | Base | Latency (p95 ratio) | Memory (peak RSS VM/TW) | Verdict |
|---|---|---|---|---|
| №404 (re-gate №1) | `fe89aa3` | ×3.35/×3.10/×3.04 PASS | ×1.136–×1.143 FAIL | NOT flip-ready |
| №410 (re-gate №2, after №409) | `434a871` | ×3.32/×3.83/×3.09 PASS | ×1.126/×1.128/×1.129 FAIL | NOT flip-ready |
| №415 (re-gate №3, after the retained-representation compression) | `5f9da64` | ×3.00/×3.49/×3.11 PASS | ×0.95/×1.07/×0.98 PASS | **flip-ready** |

The two red memory series localized the remaining delta to a RETAINED class (the compiled `Arc<Program>` + the pattern-body duplicates + the shared-cache snapshots), and the owner chose **path A** (one wave, decision comment 5751899756): compress exactly that class, then re-gate under the SAME thresholds. The compression (№415/№416 — the pattern-body TABLE with zero duplicates, the boxed `Instruction` payloads 160 → ≤ 32 B, the zero-clone snapshot) moved the memory gate from 0/6 to 3/3 within exactly one wave — the diagnosis confirmed by action. Runs: [35538598882](https://github.com/ShkodnikAI/Metalogos-/actions/runs/35538598882), [35538606322](https://github.com/ShkodnikAI/Metalogos-/actions/runs/35538606322), [35538613574](https://github.com/ShkodnikAI/Metalogos-/actions/runs/35538613574); claim `w6: n415-claim` (5752763580, BEFORE the runs), verdict `w6: n415-verdict` (5752838305).

## 2. Decision

The owner executed Stage 5 with the directive **«Флипай»** (2026-09-21, issue #527): the flip-ready data is accepted as sufficient, and the `mlog serve` default backend is flipped.

1. **Default = VM.** Absent `METALOGOS_SERVE_BACKEND` selects the bytecode VM. The startup line stays loud and carries the decision trace: `[server] backend: vm (bytecode VM, default per ADR-0171 — owner flip decision 2026-09-21 on the re-gate №3 3/3 GREEN evidence)`.
2. **The interpreter opt-out is preserved** (back-compat per ADR-0141 D6): `METALOGOS_SERVE_BACKEND=interpreter` is an explicit, loudly-logged override. Unknown values fall back to the VM (the new default) with a loud WARN — the fallback follows the default, not the legacy default.
3. **ADR-0088 status update** (recorded here per ADR-0141 D6, historical accuracy of ADR-0088 preserved): `Implemented (default remains interpreter)` → `Implemented (default flipped to vm per ADR-0171, opt-out via METALOGOS_SERVE_BACKEND=interpreter)`.
4. **The VM pool posture is untouched**: the warm per-request pool (№403, ADR-0141) remains default-OFF — the flip changes which engine executes routes, not how request state is pooled.
5. **The benchmark protocol is untouched**: the stage-4 bench and the soak select both variants explicitly; the re-gate series remain reproducible on the flipped tree.
6. **TW remains the guaranteed full-language backend** (ADR-0141 D7) — now as the explicit opt-out; `crosscheck_backends` (№373) remains the blocking parity gate.

**Honest boundary (recorded, not hidden):** Stage 3's sprint-length staging-soak criterion (vm-gaps-inventory §5.3, ориентир ~2026-09-30) was not formally complete at flip time. What was green: the parity gate, the nightly soak workflow's 24 h-parity accumulation across waves 3–5, and the full local test suite. The owner's explicit directive supersedes the schedule on the strength of the Stage-4 evidence; the soak workflow keeps accumulating parity evidence post-flip, and the interpreter opt-out keeps the rollback path one env var away.

## 3. Consequences

- The VM path (startup route compilation per №40, per-request `Vm::new` + `load_program` over the shared `Arc<Program>` snapshot per №402/№415) becomes the serve default; the per-request and retained memory work (№403/№409/№415) is now on the hot path for every deployment, not only opt-ins.
- Deployments that pinned `METALOGOS_SERVE_BACKEND` explicitly are unaffected; deployments relying on the absent-var default move TW → VM and inherit the ×3+ p95 improvement and the ≤ ×1.1 peak-RSS envelope measured by the series above.
- The docs-consistency newcomer contract (tests/docs_consistency.rs) now pins the flipped default: the section must state "VM by default" and name the `METALOGOS_SERVE_BACKEND=interpreter` opt-out.
- ADR-0105 receives a one-line addendum (its §Decision 1–4 remain in force — TW as the guaranteed full-language backend, now the explicit opt-out).

## 4. Verification

- `src/server.rs`: the №40 env ladder flipped (default → `ServeBackend::Vm`, unknown → VM fallback with WARN, `=interpreter` → explicit opt-out) and pinned by `test_n40_backend_env_default_is_vm` + `test_n40_backend_env_unknown_falls_back`.
- `tests/docs_consistency.rs`: the needles updated to the flipped contract (`METALOGOS_SERVE_BACKEND=interpreter`, "VM by default").
- Full local `cargo test --workspace` green; blocking CI green on the PR head; the merge auto-closes issue #527 (`Closes: #527`).
- The benchmark/soak surfaces are byte-identical in variant selection (explicit backends only).

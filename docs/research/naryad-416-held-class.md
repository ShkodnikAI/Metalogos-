# Naryad №416 — the held-class decomposition (gh#568, Волна 6)

**Executor:** Super Z · **Date:** 2026-09-21 · **Base:** main @ `148f013` · **Issue:** gh#568
**Scope note:** the compression levers of this naryad were executed under **№417**
(gh#569 — created by the owner's path-A decision on gh#527, comment 5751899756;
PRs #574/#575) BEFORE this naryad's turn in the queue came up. This doc delivers
the remaining №416 item — the measured decomposition table — and reconciles every
№416 task against where it actually landed. Nothing is re-executed; nothing is
double-counted.

## 1. The held class, decomposed (pre-№417, the №410 residual ≈4.3–4.6 MB)

Measured by `examples/retained_diag.rs` on the pinned Stage-4 bench corpus
(`benches/fixtures/production_workload.mlog`, 2 344 lines, 14 routes) at main
`b8a5976` — the raw numbers in `naryad-415-retained-compression.md` §1, re-aggregated
here per №416's component list:

| Component | Serialized (wire) | In-RAM | ≈ % of the held class (midpoint 4.5 MB) |
|---|---|---|---|
| Pattern bodies INLINE in `main_code` (`RegisterPattern(CompiledFn)`) | 131 KiB (9 243 instructions) | ≈ 1.48 MB (9 243 × 160 B) | ≈ 33% |
| The №402 shared snapshot `pre_registered_patterns` — a full CLONE of the same bodies | — (a clone, not wire) | ≈ 1.48 MB | ≈ 33% |
| **→ the duplicate pair together** | 131 KiB ×2 copies | **≈ 2.96 MB** | **≈ 66%** |
| Program top-level bytecode (`main_code` minus inline bodies; 178 instructions) | ~4 KiB | ≤ 0.01 MB | < 1% |
| The RegisterPattern snapshot AS A SEPARATE STRUCTURE | — | — | — (it WAS the clone above; №416's "компакция RegisterPattern-снапшота" targets exactly this) |
| Shared-cache snapshots: rules / deny_handlers / skill_indices / globals / schema_ddl / learnables | 8 B each (0 entries on this corpus) | ~0 | ≈ 0% |
| Route bodies (compiled at serve startup) | 2 715 B (135 instructions) | ≈ 0.02 MB at the 160 B scale | < 1% |
| The remainder of the ×1.126–×1.129 gap (≈ 1.3–1.6 MB) | — | spike-amplified representation mass: the Stage-4 harness VmHWM holds 5 × compile spikes (STARTUP_RUNS), 5 × (parse+compile+routes) spikes, 20 × whole-`Program` clones, and the in-process server carrying BOTH representations — every spike reads the per-instruction size (160 B) | ≈ 30% |

**Honest reading.** The gate measures the HARNESS process VmHWM, not a steady-state
server. That is why the residual read ×1.126–×1.129 while the largest single
retained structure was "only" the ≈ 2.96 MB duplicate: the duplicate mass is
re-materialized in every compile/clone spike, so shrinking BOTH the copy count
(zero-dup) AND the per-instruction size (160 → ≤ 32 B) attacks the spike ceilings
too. A decomposition that only summed retained structures would have under-predicted
the measured effect — and did (the №410 projection).

## 2. Step A — lazy route-body compilation: assessed N/A (recorded, not implemented)

- **Numbers:** 14 routes, 135 instructions, 2 715 B serialized — ≈ 22 KiB in-RAM at
  the 160 B scale pre-№417, ≈ 4.4 KiB post (≤ 32 B/instruction). The Stage-4 bench
  plan hits ALL 14 routes EVERY round, so laziness moves no watermark on this gate.
- **The documented semantics contract (for a future gate where route mass matters):
  a lazy body compiles at the FIRST call of its route, cached by route-id; the
  compile-failure point moves from load time to first call — it must surface as a
  LOUD fail-fast AT THE CALL (no silent degradation), pinned by a test at
  implementation time.** Not implemented in this wave — recorded here per the naryad.

## 3. Step B — RegisterPattern-snapshot compaction: DELIVERED (№417, owner path A)

- Before: the snapshot was a full second copy of every body (≈ 1.48 MB).
- After (PR #574, `5f9da64`): `Program::patterns` (`Arc<Vec<CompiledFn>>`, serde `rc`,
  wire-identical) holds the SINGLE canonical copy (175 entries); the №402 snapshot is
  an `Arc` increment — zero clone (`Arc::ptr_eq` pinned by
  `n415_snapshot_is_zero_clone_over_the_table`); `size_of::<Instruction>()` 160 → ≤ 32 B
  (pinned); the legacy inline variant stays for old-.mbc (boxed, wire-transparent) with
  the scan fallback; the out-of-range `RegisterPatternRef` fails loudly (№264 backstop).
- Measured delta: serve duplicate mass ≈ 2.96 MB → **~0**; in-RAM body mass 2 copies
  → 1; `.mbc` wire 132 → 133 KiB (+1 KiB for the refs + table header).

## 4. Re-gate №3 (protocol №398) — the shared series record

3/3 GREEN on BOTH thresholds (p95 ×3.00/×3.49/×3.11 ≥ ×1.5; peak RSS ×0.95/×1.07/×0.98
≤ ×1.1; runs [35538598882](https://github.com/ShkodnikAI/Metalogos-/actions/runs/35538598882) /
[35538606322](https://github.com/ShkodnikAI/Metalogos-/actions/runs/35538606322) /
[35538613574](https://github.com/ShkodnikAI/Metalogos-/actions/runs/35538613574) on main @
`5f9da64`; claim `w6: n415-claim` 5752763580 declared BEFORE the runs per rule 2, verdict
`w6: n415-verdict` 5752838305) — shared with this naryad per `w6: regate3-shared`
(gh#568 comment 5752215576). The memory gate was green for the first time in three series
(№404: 0/3; №410: 0/3; №417: 3/3). The flip was decided AND executed by the OWNER
(№404 Stage 5, ADR-0171, PR #576) — never by the executor (the №398/№404 boundary).

## 5. Reconciliation — every №416 item → where it landed

| №416 item | Status | Where it lives |
|---|---|---|
| (1) held-class decomposition table (bytes + %) | **THIS DOC** — measured pre/post on the pinned corpus | `docs/research/naryad-416-held-class.md` §1, §6 |
| (2) Step A — lazy route-body compilation | assessed **N/A** for this gate; semantics documented for the future | §2 above |
| (3) Step B — RegisterPattern-snapshot compaction | **DELIVERED** by №417 (PR #574) | `naryad-415-retained-compression.md` §2–3; pinned by `tests/naryad_415_retained.rs` (7 tests) |
| (4) re-gate №3 data (claim → 3 runs → verdict; thresholds untouched) | **EXECUTED, 3/3 GREEN** | the shared record: gh#527 5752763580/5752838305; PR #575; ADR-0141 Addendum 6 |
| (5) docs (ADR-0141 Addendum 6, limitations, CHANGELOG, README anchor) | **DONE** (by №417 + the №404 flip PR) | ADR-0141 Addendum 6; limitations VM row — the "VM is not the default" row CLOSED (№404 Stage 5); CHANGELOG Unreleased/Added; README §Dual Execution Backend |

## 6. The live state — the diag re-run on current main (@ `148f013`)

| Probe (post-№417) | Value |
|---|---|
| Inline bodies in `main_code` | **0** (main_code = 178 instructions, all top-level) |
| `Program.patterns` table | **175 entries** — the single canonical copy |
| rules / deny_handlers / skill_indices / globals / schema_ddl / learnables | 0 entries each (8 B each on the wire) |
| Route bodies | 14 routes, 135 instructions, 2 715 B |
| `.mbc` serialized Program | 136 244 B (133 KiB) |
| Leftover inline+snapshot duplicate mass | **~0** |
| instruction-slot proxy | main_code 178 (0 from inline bodies), heap-units ≈ 356 |

## References

№409 (per-request compression, PR #560) · №410 (re-gate №2 + the retained-class
diagnosis, PR #561) · №417 (gh#569 — path A, PRs #574/#575) · gh#527 (the owner
decision + the verdict packages + the flip directive) · №404 Stage 5 (PR #576,
ADR-0171) · ADR-0141 (Addenda 4–7) · gh#566 (the Wave-6 dispatch) · protocol №398
(`docs/benchmark-protocol.md`).

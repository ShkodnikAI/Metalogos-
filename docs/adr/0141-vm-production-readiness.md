# ADR-0141: VM production-readiness — staged gap closure + parity-gated default flip

**Status:** Accepted (owner decision 2026-09-14 — supersedes the ADR-0105 caveat)
**Date:** 2026-09-14
**Naryad:** #293 (issue #356, P0/adr — VM_COMPLETE)
**Amends / Supersedes (partial):** ADR-0105 (`Bytecode VM — experimental scope`) — supersedes the caveat "Do not implement Match/BlockIfElse in the VM under this ADR — revisit only under a real strategic need"; ADR-0105 §Decision 1-4 remain in force (TW = guaranteed full-language backend; VM = experimental for full-language use until Stage 5).
**Precedent:** ADR-0088 (VM for `mlog serve`, opt-in default interpreter — the default flip remains gated), Naryad #91 (TryEval — the template for closing one VM gap), ADR-0073 (JIT experimental scaffold).

## Context

ADR-0105 (Accepted 2026-08-21) fixed two confirmed gaps in the bytecode VM: `Match` is not compiled at all; `Expr::BlockIfElse` (if/else as a value in `let`/`return`) is not compiled — a loud error since naryad #129 (previously it silently compiled to Unit). ADR-0105 stated outright: "Do not implement Match/BlockIfElse in the VM under this ADR — revisit only under a real strategic need (demonstrated load where VM throughput matters), not because leaving the gaps feels incomplete". FOSVED runs on TW; ADR-0088: the `mlog serve` default is interpreter, VM is opt-in.

The external Metalogos audit of 2026-09-13 (state of main post-PR #353) proposed revisiting that decision. **Owner decision 2026-09-14**: supersede the ADR-0105 caveat — staged gap closure; the default flip (ADR-0088) remains gated: 100% parity + full crosscheck + soak + real load.

The detailed gap inventory and closure costs — `docs/research/vm-gaps-inventory.md` (this ADR summarizes it).

## Decision (staged plan)

### D1. Stage 0 — research (this naryad, #293)

Zero code. Research inventory (`docs/research/vm-gaps-inventory.md`) + this ADR + a README update (VM experimental — note "staged closure in progress per ADR-0141"). The owner decision is recorded.

### D2. Stage 1 — closing the 4 gaps, following precedent #91

Each gap — a separate naryad, a separate PR, separate tests + crosscheck exclusion removal. Order (by parity effect):

| Naryad (proposed) | Gap | Cost (LOC) | crosscheck exclusion removal |
|---|---|---|---|
| #294 | Match statement + expression (`Statement::Match` + `match_expr`) | ~400 | `p_match_switch.mlog` |
| #295 | `Expr::BlockIfElse` (if/else as a value) | ~205 | (no direct exclusion; covered by the VM-excluded examples in `p5_if_else.mlog`-style programs — add a crosscheck if needed) |
| #296 | Binop coercion (heterogeneous List+String) | ~80 | `p118_collection_utils.mlog` |
| #297 | PRNG state (`random_seed`/`random`) + Bool→String formatting | ~60 | `reflex_math.mlog` |

**Structure of each naryad** (following the #91 pattern — TryEval):
1. A new bytecode instruction in `src/bytecode.rs` (enum variant).
2. A compiler arm in `src/compiler.rs` (compile expression/statement → emit instruction).
3. VM dispatch in `src/vm.rs` — both loops (`run` for the main program + `execute_route_code` for route handlers).
4. Tests — `tests/naryad_<N>_*.rs` covering success path + edge cases + regression.
5. crosscheck_backends.rs — remove the corresponding exclusion (1 line).

**No ADR required** per gap — this is a VM extension within the already-accepted language semantics (Match/BlockIfElse/binop/random are already defined in the grammar and work in TW). Extending the VM bytecode is implementation work, not an architectural decision.

### D3. Stage 2 — parity gate

After Stage 1 (all 4 naryads merged): `tests/crosscheck_backends.rs` without a single `continue;` VM-uncovered exclusion (except the negative-test contracts — `p50_unknown_fn`, `p2_wrong_types` — designed-to-fail, not a parity concern). If parity is 100% — proceed to Stage 3. If regressions — additional naryads to close them before proceeding.

### D4. Stage 3 — soak

FOSVED on the VM in staging for **1 sprint (≈2 weeks)**, with no panic/regression. Currently FOSVED runs on TW; the VM is opt-in for experiments only. Soak is a mandatory period; on panic/regression — extend or roll back.

### D5. Stage 4 — real-load benchmark

Benchmark on a production-class .mlog file (≥2000 lines, with LLM calls, DB, vision — a representative FOSVED workload). The VM must show:
- **≥2× latency improvement** (via bytecode dispatch + no AST traversal overhead), OR
- **Equivalent latency with a memory/CPU win** (if latency is not 2×, but the memory footprint is significantly smaller — acceptable for restricted envs).

Without one of these conditions — no default flip (Stage 5 is blocked).

### D6. Stage 5 — ADR-0088 default flip (only if Stages 2-4 are green)

Flip the `METALOGOS_SERVE_BACKEND` default from `interpreter` to `vm`. A separate ADR (new number — `0142` or higher). With the opt-out via `METALOGOS_SERVE_BACKEND=interpreter` preserved for back-compat (old deployments, edge cases, debugging).

**ADR-0088 status update**: `Implemented (default remains interpreter)` → `Implemented (default flipped to vm per ADR-0XXX, opt-out via METALOGOS_SERVE_BACKEND=interpreter)`. A separate amend-ADR — not done inside the existing ADR-0088 (historical accuracy).

### D7. ADR-0105 amend

ADR-0105 §Decision 1-4 remain in force:
- TW = guaranteed full-language backend (Stage 5 does not cancel this — TW remains as the opt-out).
- VM = experimental for full-language use — but the status changes: "experimental" → "production-ready after Stage 1-4" (after the gaps are closed).

The caveat "Do not implement Match/BlockIfElse in the VM under this ADR" is **superseded** by this ADR (Stage 1 closes the gaps).

## Consequences

- **Stage 1 (naryads #294-#297)**: VM bytecode coverage grows from ~95% to 100% of language constructs. crosscheck_backends.rs becomes exclusion-free (for VM-uncovered constructs).
- **Stages 2-4**: parity + soak + benchmark — the gates before the flip. If any of them fails — the plan is revisited (possibly keeping the VM opt-in as is, without the default flip).
- **Stage 5 (if green)**: the `mlog serve` default = VM. FOSVED gets an automatic perf boost. TW remains for debugging / back-compat.
- **Risk**: each stage may surface hidden gaps (not covered in `docs/research/vm-gaps-inventory.md`). That is normal — the inventory is verified on `main fdfbfb7`, but is no proof against future constructs.
- **No regression risk** for existing VM usage — Stage 1 only adds coverage (new opcodes + compiler arms); existing bytecode stays valid.

## Addendum: What this ADR does NOT do

- **Does not close the gaps in this naryad** — Stage 0 = research only. Stage 1 = naryads #294-#297.
- **Does not change the backend default** — Stage 5 (a separate ADR) after Stages 2-4.
- **Does not remove ADR-0105** — only supersedes the caveat "Do not implement…". ADR-0105 remains in force for §Decision 1-4.
- **Does not add new constructs to the language** — Match/BlockIfElse/binop/random are already defined in the grammar and work in TW. Extending the VM bytecode is implementation work.

## Addendum 2 — Step B executed: the warm VM pool config decision (naryad #403, 2026-09-20)

Step B (§4 of the Stage-5 evidence, MEDIUM risk, separate naryad) is implemented on main as an
ENV OPT-IN — **the default remains pool OFF** until the #404 re-gate:

- `METALOGOS_VM_POOL=1` enables the pool (`ServerState.vm_pool`, read once at startup, the #263
  read-once discipline); `METALOGOS_VM_POOL_MAX` caps the idle set (default 8; beyond-capacity
  checkins are dropped — a bounded pool, the #263 no-unbounded-growth discipline).
- The reset protocol is contract-tested FAIL-CLOSED as §4 demanded: `Vm::reset_for_reuse` clears
  every mutable state class (the db connection first — the #381 shared-DB precedent) and
  `load_program` rebuilds the program-scoped tables wholesale; the field enumeration is
  compiler-enforced (a `Vm` field added without a reset story breaks the build); the HTTP-level
  tests pin per-request db content, grant linearity, the deny surface and the fail-closed discard
  via the pool's own counters.
- Honest indicative delta (debug profile, sequential minimal route, one box): 1090 → 895 µs per
  request (1.22x) — the `Vm::new` share. This is NOT the flip evidence: the #404 re-gate runs the
  #398 protocol (3x pinned, release profile) and owns the decision.
- Correction of a stale premise: the #40-era note "Vm is !Send" (server.rs) was wrong — `Vm` is
  `Send` (compile-time probe pinned in `src/vm_pool.rs`), which is what makes a shared in-process
  pool legal at all.

## Addendum 3 — Step C executed: the Stage 5 re-gate series (naryad #404, 2026-09-20)

The #398-protocol re-run on main @ `fe89aa3` (preconditions: #402 in main — mandatory;
#403 in main — desired, pool default-OFF): 3 consecutive pinned runs (rounds=30, all success),
divisor declared before the runs = corpus route count 14, no env flags. The series record and
the raw numbers: [`docs/research/naryad-404-stage5-rerun.md`](../research/naryad-404-stage5-rerun.md).

- Latency gate (p95 ≥ ×1.5): **3/3 PASS** — ×3.04 / ×2.50 / ×3.56 (№388 was ×1.66/×1.62/×1.48,
  2/3): the #402 `Arc<Program>` divisor elimination is confirmed at the p95 level.
- Memory gate (peak RSS ≤ ×1.1): **0/3 FAIL** — ×1.136 / ×1.143 / ×1.129 (№388 was
  1.10/1.09/1.12, 2/3): the clone removal is a LATENCY lever, not a resident-state lever, and
  the pool is likewise a latency lever (idle VMs stay resident).
- **Verdict: NOT flip-ready** by the fixed two-threshold gate (audit 2026-09-19 P0-1, thresholds
  unchanged per protocol rule 2). What holds: parity (Stages 1–2), soak (3 green nights, №388),
  and now the latency criterion with a wide margin. What does not hold: the memory criterion —
  the VM's per-request resident footprint (in-memory sqlite + per-VM state classes) sits at
  ×1.13–1.14 of TW.
- Decision ownership: the flip is the owner's call — the executor does not flip. The honest next
  lever is a VM memory-footprint naryad (the per-request resident state), then a fresh re-gate
  under the SAME thresholds. The default remains `interpreter`; the pool remains opt-in.

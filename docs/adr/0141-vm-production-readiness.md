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

## Addendum 4 — Step D executed: the VM-serve footprint compressed (naryad №409, 2026-09-20)

The memory gate of Addendum 3 failed because the VM's per-request peak sat at ×1.13–1.14 of TW.
№409 (issue #554, audit 2026-09-20 P0-1 steps A+B) decomposed that footprint by state class and
applied the candidates that measured effective. Thresholds were NOT touched (protocol №398 rule 2);
the serve default is NOT flipped (the owner decides after the fresh re-gate №410).

### Step A — the peak-RSS decomposition (method + table)

Method: one probe process per class (`examples/rss_decomposition.rs`, driver
`scripts/rss_decomposition.sh`); inside the process the probe allocates N live instances of the
class and reports the RSS slope (ΔVmRSS / N); the RSS source is `/proc/self/status` — the same
source the pinned Stage 4 benchmark uses. Fixture: the №398/№404 production-class corpus
(`benches/fixtures/production_workload.mlog`, 2344 lines, 14 routes, `db { sqlite::memory: }`,
8 entity globals, no `reflex`/`vision` DECLARATIONS — vision/reflex appear only as mock builtins).

| State class (per instance) | Before №409 | After №409 | Δ |
|---|---|---|---|
| `Vm::new()` — builtins registry + name table | ~70 KB | ~17 KB | −76% |
| `load_program` without db (globals slots + shared snapshots) | ~76 KB | ~19 KB | −75% |
| `load_program` with db declared (probe never touches the db) | ~161 KB | ~21 KB | −87% |
| per-request db class (in-memory sqlite open + schema DDL) — paid by db routes only, on first access | ~86 KB (every request) | ~86 KB (db-touching requests only: 2/14 fixture routes) | moved, not removed |
| pool idle set (per idle VM, resident) | ~228–236 KB | ~56 KB | −76% |
| reflex model registration (1 dense model, synthetic probe) | ~89 KB per VM | ~34 KB per VM | −62% (registry share effect) |
| per-request execution scratch (db route, sequential loop) | ~1 MB transient peak | ~1 MB transient peak | unchanged (freed per request; retained-per-request ≈ 0) |

Read of the table (honest): the dominant per-request classes were the builtins registry rebuild
(~70 KB) and the unconditional in-memory sqlite open + DDL (~86 KB) — both paid by EVERY request,
including the 12/14 fixture routes that never touch the db. The pool's idle set held a LIVE sqlite
connection per idle VM (the №403 reset re-opened it eagerly).

### Step B — the applied candidates (measured before/after on the pinned fixture)

- **C1 — share the immutable builtins registry (applied).** `Builtins` is a pure derivation of
  `BUILTIN_REGISTRY` (№170 SSOT) and is read-only on every VM path — the only mutation affordance
  (`override_handler`, №287) is Interpreter-only, and the interpreter keeps its own owned instance
  (the TW baseline is untouched). `Vm::builtins` and `Vm::builtin_names` are now process-wide
  `Arc` (one `OnceLock` each): `Vm::new()` pays two atomic increments instead of rebuilding the
  ~460-entry map + ~460 `String`s. Measured: the `Vm::new` class 70 → 17 KB.
- **C2 — lazy db open (applied).** `load_program` records the declared URL and takes the SHARED
  schema-DDL snapshot (`Program::schema_ddl_shared`, one Arc increment — program-immutable data
  previously deep-copied implicitly per request); the connection opens on the FIRST db access
  (`Vm::ensure_db_open`) with semantics identical to the eager open: same WAL pragma, same DDL
  application (log + continue), same "Connected"/"Failed to connect" lines, same legacy
  "no database connection" access error, fail-fast per VM generation on connect failure
  (`db_open_failed` resets on `reset_for_reuse` — a fresh VM = a fresh attempt, exactly the eager
  per-request retry semantics). The №381 isolation class is UNCHANGED: the connection is still
  per-VM, never shared across requests; `reset_for_reuse` still drops it FIRST. Measured: the
  per-request db class moved from "every request" to "db-touching requests only"; the pool idle
  set dropped 236 → 56 KB per idle VM (no live sqlite in the idle set).
- **C3 — share vision/origin decl maps (measured, not applied).** The fixture declares no
  `vision {}`/`origin {}` blocks, so the candidate's pinned-fixture effect is definitionally
  ~0; per the naryad's rule ("apply only what measurably reduces RSS on the pinned fixture")
  it is recorded, not merged. The benefit would accrue to decl-bearing workloads.
- **C4 — lazy reflex/vision model construction (measured, not applied).** Same reasoning: the
  pinned fixture declares no reflex/vision models (the synthetic probe prices the class at
  ~13 KB per declared dense model), so the fixture effect is ~0. Not merged.
- **C5/C6 — pool reset and `METALOGOS_VM_POOL_MAX` (covered by C2; default kept).** The
  "defer heavy tables out of the pooled checkout" candidate IS C2 (a pooled checkout no longer
  opens sqlite; the reset leaves the VM connection-free). The default idle cap stays 8: with C2
  the measured idle residency is ~56 KB/VM → 8 idle VMs ≈ 0.45 MB, which does not justify a
  behavioral default change (data-driven decision, recorded here).

### The full-bench before/after (same machine, pinned command `cargo bench --bench stage4_benchmark -- --rounds 30`)

| Run | TW p95 µs | VM p95 µs | p95 ratio | TW RSS MB | VM RSS MB | RSS VM/TW |
|---|---|---|---|---|---|---|
| before 1 | 25427 | 5199 | ×4.89 | 36.2 | 39.1 | 1.08 |
| before 2 | 23443 | 5021 | ×4.67 | 36.5 | 40.4 | 1.11 |
| after 1 | 35572 | 4069 | ×8.74 | 39.2 | 37.5 | 0.96 |
| after 2 | 24962 | 3632 | ×6.87 | 38.9 | 39.2 | 1.01 |
| after 3 | 40548 | 4192 | ×9.67 | 36.4 | 37.6 | 1.03 |

- The VM p95 improved (5.0–5.2 → 3.6–4.2 ms local): the removed per-request registry rebuild
  and eager open are latency, not just memory. The p95 ≥ ×1.5 criterion holds with a WIDER margin.
- The peak-RSS ratio moved from ×1.08–1.11 (local) to ×0.96–1.03 (local). The CI-runner readings
  (Addendum 3: ×1.129–1.143) are the ones the GATE measures — the fresh re-gate under the SAME
  thresholds is naryad №410's verdict, not this addendum. No threshold was renegotiated; no
  default was flipped; fail-closed pool and TW/VM parity (Stages 1–2) are unchanged.
- Tests (red→green, mutation-verified per the №382 protocol): `mod n409_tests` pins the lazy-open
  invariants (load defers; db-free route never opens; db route opens + DDL on first access;
  fail-fast with the legacy message), the registry sharing (`Arc::ptr_eq` process-wide) and the
  new discard invariant (a pooled reset rests CONNECTION-FREE; the next generation's first access
  re-opens a fresh db — request A's content never survives). Mutations M1 (re-eager the open),
  M2 (un-share the registry), M3 (keep the connection across a reset) each made the named test
  fall.

## Addendum 5 — Step E executed: the re-gate №2 series (naryad №410, 2026-09-20)

The fresh re-gate Addendum 3 promised, on main @ `434a871` (precondition №409 in main — the
footprint compression, Addendum 4): 3 consecutive pinned runs (rounds=30, all success), divisor
declared before the runs = 14 (claim comment `w5: n410-claim`, issue #527, 2026-09-20), no env
flags. The series record and the raw numbers:
[`docs/research/naryad-410-stage5-regate2.md`](../research/naryad-410-stage5-regate2.md).

- Latency gate (p95 ≥ ×1.5): **3/3 PASS** — ×3.32 / ×3.83 / ×3.09 (the margin WIDENED vs
  Addendum 3's ×3.04/×2.50/×3.56: the №409 per-request compression is also a latency lever).
- Memory gate (peak RSS ≤ ×1.1): **0/3 FAIL** — ×1.126 / ×1.128 / ×1.129 (Addendum 3 was
  ×1.136/×1.143/×1.129).
- **Verdict: NOT flip-ready** by the fixed two-threshold gate (thresholds unchanged per protocol
  rule 2). The default remains `interpreter`; the pool remains opt-in.
- Where the remaining delta lives (Addendum 4's decomposition + this series): the CI-runner peak
  gap (VM − TW ≈ 4.3–4.6 MB) is a RETAINED class — the compiled `Arc<Program>` bytecode + the 14
  compiled route bodies + the shared-cache snapshots vs TW's AST — which per-request compression
  cannot move (the local bench ratio moved ×1.08–1.11 → ×0.96–1.03 precisely because the local
  peak was per-request-dominated; the CI peak is representation-dominated).
- Next lever (a candidate naryad, owner's call, issue #527): shrink or share the retained
  bytecode representation — candidates: price the AST-vs-bytecode retained delta precisely on
  the fixture; route-body compilation laziness; dropping/compacting the `RegisterPattern`
  shared-cache snapshot (it duplicates pattern bytecode). A fresh re-gate under the SAME fixed
  thresholds remains the honest path after any such lever. The alternative — accepting the
  latency-vs-retained-memory trade — is the owner's decision, not the executor's.

## Addendum 6 — the retained-representation lever executed (naryad №415, 2026-09-21)

The owner took **path A** of Addendum 5's decision menu (gh#527, comment 5751899756,
one-wave limit): shrink the retained bytecode representation, then re-gate №3 under the
SAME fixed thresholds. The compression record and the before/after probe:
[`docs/research/naryad-415-retained-compression.md`](../research/naryad-415-retained-compression.md).

- The diagnostic closed Addendum 5's question about where the retained delta lives:
  175 pattern bodies (9 243 instructions) sat INLINE in `main_code` AND were cloned once
  into the №402 shared snapshot (`Program::patterns` was a dead, never-filled field) —
  ≈ 2.96 MB in-RAM of pure duplicate at `size_of::<Instruction>() = 160 B` (the enum's
  size was dictated by its fattest payloads, not by what programs contain).
- The lever: the compiler fills the `Program::patterns` TABLE exactly once and emits the
  append-only `RegisterPatternRef(u32)`; the shared snapshot becomes a zero-clone Arc
  increment; fat `Instruction` payloads are boxed (wire-transparent for bincode — the
  .mbc format of every existing variant is byte-identical; old .mbc keeps loading via
  the legacy boxed variant + scan fallback). `size_of::<Instruction>()` 160 → ≤ 32 B.
- Lazy route-body compilation: evaluated, **N/A for this gate** — the Stage-4 plan hits
  all 14 routes every round (route bodies total ~2 KiB), so laziness moves no watermark.
- Re-gate №3 EXECUTED on the merged main @ `5f9da64` (claim `w6: n415-claim` BEFORE the
  runs, protocol rule 2): **3/3 GREEN on BOTH thresholds** — latency ×3.00/×3.49/×3.11
  (≥ ×1.5), peak RSS ×0.95/×1.07/×0.98 (≤ ×1.1; in 2 of 3 runs the VM peak is BELOW the
  TW peak). The memory gate is green for the first time in three series (№404: ×1.136–
  ×1.143; №410: ×1.126–×1.129) — the №410 "retained class" diagnosis confirmed by action.
  Series record: the research doc §6; verdict package `w6: n415-verdict` in gh#527
  (comment 5752838305). The default flip remains the OWNER's decision — flip-ready data,
  not a flip.

# Stage 5 evidence — memory/perf refresh + the per-request divisor profile (naryad №388)

> Naryad №388 (issue #482, P2 testing/vm): the entry to the ADR-0141 Stage 4/5
> default-flip re-gate. Parent verdict: №381's INSUFFICIENT DATA (mean ×1.67–×1.95,
> no memory win, RSS ≈ 1.0). The №381 report named the common divisor: the
> per-request `load_program`/clone chain in the serve path. This document
> (1) confirms the divisor at code level, (2) refreshes the RSS/latency numbers
> on a pinned runner + a local sandbox run, (3) checks the re-gate thresholds,
> and (4) proposes the elimination ADR WITHOUT implementing it.

## 1. The per-request divisor — confirmed at code level

After startup, BOTH backends still pay a whole-state cost per request. The
startup route compilation (`server.rs`, the №40 block — `vm_routes` + the
compiled program are built ONCE) did remove compile-per-request; what remains
is the per-request STATE construction:

VM path — `execute_route_body_vm` (src/server.rs, the №40 dispatch):

```rust
let program = match state.vm_program.as_ref() {
    Some(p) => p.clone(),          // FULL Program clone PER REQUEST
    None => return Err(...),
};
...
let (audit_entries, result) = tokio::task::spawn_blocking(move || {
    let _serve_exec_guard = ServeRouteExecGuard::new();
    let mut vm = Vm::new();        // fresh VM PER REQUEST
    vm.load_program(&program)      // full load PER REQUEST
        .map_err(|e| format!("VM route init: {}", e))?;
    vm.clear_server_context();
    ...
```

- `Program::clone()` deep-clones the whole compiled program (instructions,
  constants, route tables) for every request — the closure needs `'static + Send`.
- `Vm::new()` + `load_program()` re-instantiate and re-install the program per
  request; the VM never survives across requests.

TW path — `execute_route_body` (the parity twin):

```rust
let shared = state.shared_interp.clone();      // the SHARED interpreter handle
...
shared.clone_definitions_into(&mut interp);    // definitions re-cloned PER REQUEST
```

- The shared interpreter's definitions (the corpus' 14 routes + entities) are
  cloned into a fresh per-request interpreter — same shape of cost, one level up.

Why it remains: `spawn_blocking` closures are `'static + Send`; the naive fix
(sharing the VM/interpreter across requests) crosses state-isolation concerns —
per-request taint/labels/sessions/DB handles must not leak between requests
(the №253 serve-route context, the №391 bridge, the session/DB merge chain —
№381's Fixed section documents how fragile the shared-DB path already is).

## 2. Fresh numbers (2026-09-18)

### Pinned runner (ubuntu-latest via stage4-benchmark.yml, rounds=30, N=3)

Runs (all `success`, dispatched consecutively on main @ 307f506, rounds=30,
raw JSON per run in the artifacts):
[35366524331](https://github.com/ShkodnikAI/Metalogos-/actions/runs/35366524331) ·
[35367692167](https://github.com/ShkodnikAI/Metalogos-/actions/runs/35367692167) ·
[35368447002](https://github.com/ShkodnikAI/Metalogos-/actions/runs/35368447002)

| Metric | TW | VM | Run 1 | Run 2 | Run 3 |
|---|---|---|---|---|---|
| cycle p50 ratio | — | — | ×1.58 | ×1.70 | ×1.51 |
| cycle p95 ratio | — | — | **×1.66** | **×1.62** | **×1.48** |
| cycle mean ratio | — | — | ×1.51 | ×1.69 | ×1.50 |
| peak RSS VM/TW | — | — | **1.10** | **1.09** | **1.12** |

Startup: TW parse+semantic 192 ms; VM compile 190 + routes 0 ms (the route
compilation at startup is already amortized — the №40 block).

### Local sandbox run (NOT pinned — supplementary)

```
cargo bench --bench stage4_benchmark -- --rounds 30
```

(numbers recorded in the issue report; the local run is a noise-prone
single-sandbox machine — per docs/benchmark-protocol.md rule 2 the RAW values
are reported with the environment caveat, not normalized into the re-gate.)

### Thresholds (fixed by the re-gate guide, checked against the pinned run)

| Threshold | Value | Run 1 | Run 2 | Run 3 | Verdict |
|---|---|---|---|---|---|
| p95 request-cycle speedup | ≥ ×1.5 | ×1.66 | ×1.62 | ×1.48 | NOT STABLE — 1 of 3 runs below |
| peak RSS ratio VM/TW | ≤ 1.1 | 1.10 | 1.09 | 1.12 | NOT STABLE — straddles the boundary |
| runs | ≥ 3 | 3 dispatched | | | met |

**Verdict: NOT flip-ready.** Both numeric thresholds pass on 2 of 3 pinned
runs and fail on the third — with the scheduler noise documented by №381
(+24–29 %) and №398, the current per-request cost keeps the serve-path gap
inside the noise band. This is exactly the outcome the divisor hypothesis
predicts: as long as every request pays a full program clone + VM init (VM)
or a full definitions clone (TW), the ratio wobbles with the machine's
scheduler. The elimination (§4) must land and re-run this protocol before
the re-gate can consume stable numbers.

## 3. Soak evidence

- `soak.yml` cron `0 2 * * *` — ACTIVE.
- Green nights on the date of this naryad: **3** (2026-09-16, 2026-09-17,
  2026-09-18 — all `success`).
- Each green run is a full-parity pass (lib + crosscheck + №373 parity gate +
  the whole integration suite + doc-tests) — the Stage 2 evidence keeps
  accumulating independently of this naryad.

## 4. ADR-proposal (NO implementation in this naryad)

**Title (reserved slot on owner acceptance): ADR-0170 — serve-path state reuse:
program cache + VM warm-pool.**

**Proposal.** Two independent, separately-scoped changes to
`execute_route_body_vm` / `execute_route_body`:

1. **Program cache (VM).** `state.vm_program` is ALREADY a startup-built
   `Arc`-able value; the per-request `p.clone()` exists only because
   `spawn_blocking` needs `'static + Send`. Wrap the compiled program in
   `Arc<Program>` (load_program takes `&Program`; the clone becomes an
   Arc increment). Scope: `ServerState` field type + 2 call sites. Risk:
   LOW — the program is immutable after startup; no request state crosses.
   Expected win: the entire per-request deep-clone of the compiled program
   disappears; on the 2344-line corpus this is the dominant constant term
   the №381 profile could not explain from route compilation alone.
2. **Warm VM pool (VM) / definition-cache (TW).** A bounded pool of
   pre-loaded VMs (or pre-cloned interpreter definition sets) recycled
   across requests, with a hard per-request reset of the server context
   (the existing `clear_server_context` + the per-request injection calls
   already form the reset protocol). Scope: a pool type + the borrow path in
   the two dispatch fns; middleware unaffected. Risk: MEDIUM — state leakage
   between requests is the whole danger (sessions, DB handles, taint, the
   №253 exec guard); the reset protocol must be contract-tested per request
   shape (the №381 shared-DB bug is the cautionary precedent). A leak here
   is a security regression, not a perf bug — the pool must fail CLOSED
   (reset-verify before reuse; a failed reset refuses the request loudly).

**Expected win (order-of-magnitude, to be validated by the implementation
naryad's own benchmark):** removes the per-request O(program) clone+load from
the VM path and the O(definitions) clone from the TW path — the constant term
that compresses the TW↔VM gap at the small-request end (where the ×1.67–×1.95
spread lives) and the RSS overhead of duplicate program instances under
concurrency. The re-gate thresholds (RSS ≤ 1.1, p95 ≥ ×1.5) are achievable
ONLY with this elimination on the current evidence; without it the verdict
stays INSUFFICIENT DATA.

**Out of scope / risks:** a shared mutable VM (cross-request state), JIT
(ADR-0073 scaffold), interpreter-level caching of builtin dispatch (the №170
SSOT stays), any API/grammar change (none — the whole proposal is runtime
plumbing).

**Implementation path:** a separate naryad per the owner's decision; the pool
variant MUST land with its own concurrency contract tests (reset-verify, leak
canary, the №253 guard interplay) before the flip re-gate consumes its numbers.

## 5. The N-run protocol for the re-gate (deferred, documented)

The single pinned run of this naryad validates the harness and refreshes the
memory picture; the noise documented by №381 (+24–29 % cross-run) and the
№398 protocol require the final re-gate evidence to be N ≥ 3 pinned runs on
consecutive days (workflow_dispatch, rounds=30, artifacts attached to the
re-gate issue) — the runs are CHEAP to trigger and stay with the owner.

# Stage 5 re-run — the №398 protocol series on the post-divisor VM (naryad №404)

> Naryad №404 (issue #527, P1 owner-decision) — the entry to the Stage 3 re-gate
> (orientation ~2026-09-30, owner directive). Preconditions in main: №402
> `Arc<Program>` shared snapshots (`9dc7570` — mandatory) and №403 warm VM pool
> (`92b3372` — desired). The pool is ENV OPT-IN with the default OFF (ADR-0141
> Addendum 2, the pre-gate posture) and the run contract uses no env flags, so
> the frozen variant measures **VM WITHOUT the pool** — stated in the claim
> comment (2026-09-20, issue #527) BEFORE the runs, per protocol rule 2.

## The series record (the protocol's report form)

```
decision: does VM-serve (main @ fe89aa3 = Arc<Program> №402, pool default-OFF per ADR-0141)
          meet the Stage 3 re-gate thresholds (p95 >= x1.5 AND RSS <= 1.1) on 3/3 pinned runs?
divisor (declared BEFORE the runs): benchmark plan's route count = 14
          (identical to the №398 first series; the ratios below are RAW — a divisor may not absorb them)
run command (identical, no env flags): cargo bench --bench stage4_benchmark -- --rounds 30
          via stage4-benchmark.yml, pinned runner (ubuntu-latest), ref = main @ fe89aa3
variants: single frozen variant — VM (Arc<Program>, pool OFF) vs TW, both in-process
          on the SAME machine (same-environment by construction)
tree: root -> stage5-rerun-fe89aa3 (x3 consecutive dispatches, N=3 for the 3/3 requirement)
repair policy: OOM/timeout/infra failures count as repairs (rule 3); none occurred — 3/3 answered
```

## Raw numbers (runs 1–3, all `success`)

| Run | TW p95 µs | VM p95 µs | p95 ratio TW/VM | ≥ ×1.5? | TW RSS KB | VM RSS KB | RSS VM/TW | ≤ 1.1? |
|---|---|---|---|---|---|---|---|---|
| [35496980866](https://github.com/ShkodnikAI/Metalogos-/actions/runs/35496980866) | 17184 | 5647 | ×3.04 | PASS | 36312 | 41260 | 1.136 | FAIL |
| [35496984047](https://github.com/ShkodnikAI/Metalogos-/actions/runs/35496984047) | 12713 | 5092 | ×2.50 | PASS | 36260 | 41428 | 1.143 | FAIL |
| [35496987315](https://github.com/ShkodnikAI/Metalogos-/actions/runs/35496987315) | 18092 | 5086 | ×3.56 | PASS | 36440 | 41156 | 1.129 | FAIL |

Cycle p50/mean ratios for the same runs: ×3.04/×3.04, ×2.54/×2.53, ×3.53/×3.49 —
the p95 reading is not an outlier. Run 2's absolute numbers are uniformly lower
(runner noise); the ratios are the metric (same-environment by construction).
Raw JSON per run is in the workflow artifacts; the harness log's verdict line
prints the latency-only §D5 first criterion ("GO-DATA: VM shows >= 2x latency
improvement") — that is NOT the re-gate verdict (the re-gate is a
two-threshold gate; the honesty precedent is the №388 verdict).

## Threshold verdict (the audit 2026-09-19 P0-1 gate, unchanged per protocol rule 2)

- Latency gate (p95 ≥ ×1.5): **3/3 PASS** with a wide margin (×2.50–×3.56).
- Memory gate (peak RSS ≤ ×1.1): **0/3 FAIL** (×1.129–×1.143, all above the gate).
- **Verdict: NOT flip-ready.** The `mlog serve` default remains the tree-walking
  interpreter; the VM pool remains opt-in (ADR-0141 Addendum 2). Thresholds were
  not renegotiated.

## Comparison with №388 (the 2/3 baseline)

| Metric | №388 (main @ 307f506, per-request divisor) | №404 (main @ fe89aa3, Arc<Program>) |
|---|---|---|
| cycle p95 ratio | ×1.66 / ×1.62 / ×1.48 (2/3 ≥ 1.5) | ×3.04 / ×2.50 / ×3.56 (3/3 ≥ 1.5) |
| peak RSS VM/TW | 1.10 / 1.09 / 1.12 (2/3 ≤ 1.1) | 1.136 / 1.143 / 1.129 (0/3 ≤ 1.1) |

- The №402 divisor elimination is CONFIRMED at the p95 level: the latency
  criterion went from marginal (2/3, one run below the gate) to met on all
  three runs with a ×1.7–2.4 margin over the threshold.
- The memory ratio did NOT improve — expected: the clone removal is a LATENCY
  lever, not a resident-state lever. The VM's peak RSS sits at ×1.13–1.14 of TW
  (VM ~41.2–41.4 MB vs TW ~36.3 MB — the per-request in-memory sqlite plus the
  per-VM state classes).
- The pool (№403) is also a LATENCY lever (idle VMs stay resident): enabling it
  cannot be expected to move the RSS gate. The blocking lever is the VM's
  resident footprint — a separate engineering direction (a candidate naryad),
  after which a fresh re-gate under the SAME fixed thresholds is the honest
  path. The flip decision itself stays with the owner.

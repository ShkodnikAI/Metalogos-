# Stage 5 re-gate №2 — the №398 protocol series on the compressed VM (naryad №410)

> Naryad №410 (issue #555, P0 vm) — the SECOND re-gate after the memory
> criterion failed the first series (№404: latency 3/3, memory 0/3,
> NOT flip-ready). Precondition: №409 in main (the footprint compression:
> shared builtin registry + lazy per-request db open, PR #560 → `434a871`).
> The thresholds are UNCHANGED (protocol rule 2; the audit 2026-09-20 P0-1
> step C fixed them: p95 ≥ ×1.5 AND peak RSS ≤ ×1.1 on ALL THREE runs).
> The run contract uses no env flags, so the frozen variant measures
> **VM (№402 Arc<Program> + №409 compression, pool default-OFF)** —
> stated in the claim comment `w5: n410-claim` (2026-09-20, issue #527)
> BEFORE the runs, per protocol rule 2.

## The series record (the protocol's report form)

```
decision: does VM-serve (main @ 434a871 = Arc<Program> №402, pool default-OFF per ADR-0141
          Addendum 2, footprint compression №409) meet the Stage 3 re-gate thresholds
          (p95 >= x1.5 AND RSS <= 1.1) on 3/3 pinned runs?
divisor (declared BEFORE the runs): benchmark plan's route count = 14
          (identical to the №398 first series and the №404 series; the ratios below are RAW —
          a divisor may not absorb them)
run command (identical, no env flags): cargo bench --bench stage4_benchmark -- --rounds 30
          via stage4-benchmark.yml (repo-versioned, unmodified since №404),
          pinned runner (ubuntu-latest), ref = main @ 434a871
variants: single frozen variant — VM (№402 + №409) vs TW, both in-process
          on the SAME machine (same-environment by construction)
tree: root -> stage5-regate2-434a871 (x3 consecutive dispatches, N=3 for the 3/3 requirement)
repair policy: OOM/timeout/infra failures count as repairs (rule 3); none occurred — 3/3 answered
```

## Raw numbers (runs 1–3, all `success`)

| Run | TW p95 µs | VM p95 µs | p95 ratio TW/VM | ≥ ×1.5? | TW RSS KB | VM RSS KB | RSS VM/TW | ≤ 1.1? |
|---|---|---|---|---|---|---|---|---|
| [35520926850](https://github.com/ShkodnikAI/Metalogos-/actions/runs/35520926850) | 12922 | 3888 | ×3.32 | PASS | 36012 | 40536 | 1.126 | FAIL |
| [35520933026](https://github.com/ShkodnikAI/Metalogos-/actions/runs/35520933026) | 17319 | 4525 | ×3.83 | PASS | 36524 | 41188 | 1.128 | FAIL |
| [35520940149](https://github.com/ShkodnikAI/Metalogos-/actions/runs/35520940149) | 19238 | 6230 | ×3.09 | PASS | 36272 | 40940 | 1.129 | FAIL |

(RSS KB are the exact `rss_peak_kb` values from each run's
`stage4_benchmark_report.json` artifact; the ratio is the harness's
`rss_ratio_vm_over_interp`. The log's MB printout — 35.2/39.6, 35.7/40.2,
35.4/40.0 — is the same reading at lower precision. Cycle p50 ratios for
the same runs: ×3.58 / ×3.91 / ×3.37 — the p95 reading is not an outlier.
Raw JSON per run is in the workflow artifacts; the harness log's §D5
verdict line is the latency-only reading and is NOT the re-gate verdict —
the re-gate is a two-threshold gate.)

## Threshold verdict (the fixed two-threshold gate, unchanged per protocol rule 2)

- Latency gate (p95 ≥ ×1.5): **3/3 PASS** (×3.09–×3.83; the margin widened vs №404's
  ×2.50–×3.56 — the №409 per-request compression is also a latency lever).
- Memory gate (peak RSS ≤ ×1.1): **0/3 FAIL** (×1.126–×1.129 — a hair under №404's
  ×1.129–×1.143, still above the gate).
- **Verdict: NOT flip-ready.** The `mlog serve` default remains the tree-walking
  interpreter; the VM pool remains opt-in (ADR-0141 Addendum 2). Thresholds were
  not renegotiated. The flip decision stays with the owner (issue #527).

## Where the remaining delta lives (the honest next-lever analysis)

The №409 compression removed the PER-REQUEST classes (measured on the pinned
fixture, same machine, ADR-0141 Addendum 4: `Vm::new` 70→17 KB, load 76→19 KB,
db-free requests pay no sqlite at all; local bench RSS ratio ×1.08–1.11 →
×0.96–1.03). The pinned CI series shows the RATIO barely moved (×1.13 both
before and after) — which pins down WHERE the CI-runner peak actually lives:

- **The retained program-representation delta.** The VM process holds the
  compiled `Arc<Program>` (bytecode of the 2344-line corpus) + the 14 compiled
  route bodies + the shared-cache snapshots (the pre-registered-pattern table is
  a full bytecode copy); the TW process holds the parsed AST. The VM−TW peak gap
  on CI is ~4.3–4.6 MB (39.6–40.2 vs 35.2–35.7) — of the same magnitude as
  №404's ~4.9–5.1 MB. Per-request compression cannot move a RETAINED class.
- The per-request classes the series still pays are small against that gap
  (≤ ~25 KB for a db-free request + ~86 KB transient sqlite for the 2/14
  db-touching routes, freed per request).
- **Next lever (a candidate naryad, owner's call):** shrink or share the
  retained bytecode representation — candidates: measure AST-vs-bytecode
  retained size on the fixture to price the class precisely; route-body
  compilation laziness (compile a route body on its first request); a compact
  bytecode encoding (the shared-cache snapshot of `RegisterPattern` duplicating
  pattern bytecode is measurable and may be droppable). A fresh re-gate under
  the SAME fixed thresholds remains the honest path AFTER any such lever.
- The alternative is the honest trade: the VM's latency win (×3.1–×3.8 on the
  pinned workload) is bought with a retained-representation premium the memory
  gate refuses. Whether the gate is worth pursuing further is an owner
  decision (issue #527), not an executor's.

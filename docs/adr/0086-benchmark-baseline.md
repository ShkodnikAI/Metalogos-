# ADR-0086: Performance baseline benchmarks

**Date:** 2026-08-01
**Status:** accepted
**Context:** Naryad №34 Block 6

## Benchmark program

```
pattern Add(n: Float, m: Float) -> Float { return n + m }
pattern Chain(n: Float) -> Float {
  let mut a = n
  a = Add(a, 1.0)  // ×10
  return a
}
entity start: Float = 0.0
flow Main { input: Float = start -> Chain -> output }
```

Pattern call chain: 10 additions through pattern calls.

## Baseline (release build, criterion)

| Component | Benchmark | Median |
|---|---|---|
| Parser | `parse_chain_program` | **177.7 µs** |
| Interpreter | `run_chain_program` | **272.0 µs** |
| Compiler | `compile_chain_program` | **218.2 µs** |
| VM | `run_bytecode_chain` | **36.1 µs** |

## Key ratios

- **VM is 7.5× faster than interpreter** for this workload.
- Compile + VM = 218µs + 36µs = 254µs — comparable to interpreter (272µs).
- Parser overhead is 178µs — ~65% of interpreter time.
- VM execution is only 13% of interpreter time.

## Environment

- Rust 1.97.1, release profile, criterion 0.5
- Single-threaded, no CPU throttling
- Metalogos v0.12.0, commit `4eef23d`

## Tool

`benches/core_benchmarks.rs` — criterion-based, runs on same program for fair comparison.
Run: `cargo bench`.

## Addendum: Stage 4 real-load benchmark (2026-09-17, naryad №381 / issue #467)

ADR-0141 §D5 requires a benchmark on a production-class workload before the
`mlog serve` default flip. That benchmark now exists and has run.

**Corpus** — `benches/fixtures/production_workload.mlog` (2344 lines,
FOSVED-like helpdesk): 14 routes over `mlogserver` (match dispatch, mock-LLM
classify/summary, mock vision/voice, in-memory SQLite CRUD, kv sessions, a
mixed pipeline, a path-template route, try/error-protocol routes, report and
stats workloads; ~106 helper patterns). I/O is deterministic-mock only —
what is measured is DSL execution, not network I/O. Sanitize gate: 0 hits.
The deterministic request plan lives in `benches/fixtures/stage4_routes.json`.

**Harness** — `benches/stage4_benchmark.rs` (`cargo bench --bench
stage4_benchmark`): one process per backend (clean peak-RSS HWM, identical
machine), 30 request cycles × 14 routes per backend over loopback HTTP,
per-route p50/p95/mean, per-cycle totals, peak RSS (VmHWM), startup split
into parse+semantic (both backends) vs bytecode compile (VM-only), plus a
DSL-only diagnostic (the corpus Bench flow through `run()`/`run_bytecode()`,
reported net of the measured per-run whole-program clone baseline). A raw
JSON report is written to `target/bench-reports/`.

**Results** (3 consecutive runs, 30 rounds each; fastest/typical below):
request-cycle mean speedup interpreter→vm **×1.67–×1.95** (p50 ×1.82–×2.00);
DSL-only net **×1.1–×1.5**; peak RSS ratio vm/interpreter **~1.0** (no
memory win); startup: parse+semantic ~135–150 ms, VM compile +routes
~133–147 ms. The loopback HTTP layer adds an identical constant to both
backends and compresses the visible ratio.

**§D5 verdict (loud): INSUFFICIENT DATA — флип не обоснован.** The VM is
consistently ~1.8–2× faster per request cycle, but the §D5 bar (≥2× latency
win, OR equivalent latency with a significant memory win) is not met on the
cycle-mean reading and there is no memory win. Per ADR-0141 §D5/§D6: the
default remains `interpreter`; the flip decision goes back to the owner
re-gate (~2026-09-30) with this data.

Side effects of building the corpus (fixed in the same naryad, parity
pinned by `tests/naryad_381_stage4_corpus.rs`): the VM dropped the `query()`
params list entirely (now typed-binds via the shared `convert_params` SSOT,
as do `db_execute()`/`query_scalar()` which previously stringified), and
the server-startup merge chain clobbered the shared in-memory `db_conn`
(every `query()` in route bodies failed with "no database connection" on
`db { url: "sqlite::memory:" }` apps).

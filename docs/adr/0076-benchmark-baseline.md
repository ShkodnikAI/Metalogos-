# ADR-0076: Performance baseline benchmarks

**Date:** 2026-08-01
**Status:** accepted
**Context:** Наряд №34 Block 6

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

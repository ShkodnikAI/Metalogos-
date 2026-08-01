// ── Core Benchmarks: parser, interpreter, VM ─────────────────────
//
// Usage: cargo bench
//
// All benchmarks run on the same .mlog program for fair comparison.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

/// Benchmark program: 10 chained additions through pattern calls.
/// Exercises: entity, pattern, flow, function call, arithmetic.
const PROGRAM: &str = r#"
pattern Add(n: Float, m: Float) -> Float {
  return n + m
}
pattern Chain(n: Float) -> Float {
  let mut a = n
  a = Add(a, 1.0)
  a = Add(a, 1.0)
  a = Add(a, 1.0)
  a = Add(a, 1.0)
  a = Add(a, 1.0)
  a = Add(a, 1.0)
  a = Add(a, 1.0)
  a = Add(a, 1.0)
  a = Add(a, 1.0)
  a = Add(a, 1.0)
  return a
}
entity start: Float = 0.0
flow Main { input: Float = start -> Chain -> output }
"#;

fn bench_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser");
    group.bench_function("parse_chain_program", |b| {
        b.iter(|| {
            let result = metalogos::parser::parse(black_box(PROGRAM));
            black_box(result)
        })
    });
    group.finish();
}

fn bench_interpreter(c: &mut Criterion) {
    let mut group = c.benchmark_group("interpreter");
    group.bench_function("run_chain_program", |b| {
        b.iter(|| {
            let result = metalogos::run_program(black_box(PROGRAM));
            black_box(result)
        })
    });
    group.bench_function("compile_chain_program", |b| {
        b.iter(|| {
            let result = metalogos::compile_program(black_box(PROGRAM));
            black_box(result)
        })
    });
    group.finish();
}

fn bench_vm(c: &mut Criterion) {
    let mut group = c.benchmark_group("vm");
    // Compile once, benchmark only execution
    let program = metalogos::compile_program(PROGRAM).expect("compile failed");
    group.bench_function("run_bytecode_chain", |b| {
        b.iter(|| {
            let result = metalogos::run_bytecode(black_box(program.clone()));
            black_box(result)
        })
    });
    group.finish();
}

criterion_group!(benches, bench_parser, bench_interpreter, bench_vm);
criterion_main!(benches);

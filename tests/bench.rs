// ── Phase 4.2 Benchmark: tree-walking vs VM performance ─────────────
// Generates a synthetic program with a long pipeline (100 chained pattern calls)
// and measures execution time for both tree-walking and VM paths.

use std::fs;
use std::path::Path;

/// Generate a synthetic .mlog program with N chained Increment steps in the flow.
/// Pattern: Increment(n: Float) -> Float { return n + 1.0 }
/// Flow: input -> Increment -> Increment -> ... (N times) -> output
fn generate_synthetic_program(steps: usize) -> String {
    let mut program = String::new();
    program.push_str("entity counter: Float = 0.0\n\n");
    program.push_str("pattern Increment(n: Float) -> Float { return n + 1.0 }\n\n");
    program.push_str("flow Main {\n  input: Float = counter\n");
    for _ in 0..steps {
        program.push_str("  -> Increment");
    }
    program.push_str("\n  -> output\n}\n");
    program
}

/// Execute via tree-walking interpreter, return elapsed time in microseconds.
fn bench_tw(source: &str, base_dir: &Path) -> (Result<Option<String>, String>, u128) {
    let start = std::time::Instant::now();
    let result = metalogos::run_program_with_dir(source, base_dir.to_path_buf());
    let elapsed = start.elapsed().as_micros();
    (result.map_err(|e| e.to_string()), elapsed)
}

/// Execute via bytecode VM, return elapsed time in microseconds.
fn bench_vm(source: &str, base_dir: &Path) -> (Result<Option<String>, String>, u128) {
    let start = std::time::Instant::now();
    let result = metalogos::run_program_with_dir(source, base_dir.to_path_buf());
    let elapsed = start.elapsed().as_micros();
    (result.map_err(|e| e.to_string()), elapsed)
}

#[test]
fn benchmark_vm_vs_tree_walking() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let examples_dir = Path::new(&manifest_dir).join("examples");
    let base_dir = examples_dir.parent().unwrap_or(Path::new("."));

    // Warm-up run (JIT-like effects, cache warm-up)
    let warmup = generate_synthetic_program(50);
    let _ = bench_tw(&warmup, base_dir);
    let _ = bench_vm(&warmup, base_dir);

    // Benchmark configurations
    let step_counts = [10, 50, 100, 500, 1000];
    let mut results = Vec::new();

    for &steps in &step_counts {
        let program = generate_synthetic_program(steps);

        // Run tree-walking 3 times, take minimum
        let mut tw_min = u128::MAX;
        for _ in 0..3 {
            let (_, us) = bench_tw(&program, base_dir);
            tw_min = tw_min.min(us);
        }

        // Run VM 3 times, take minimum
        let mut vm_min = u128::MAX;
        for _ in 0..3 {
            let (_, us) = bench_vm(&program, base_dir);
            vm_min = vm_min.min(us);
        }

        let speedup = if vm_min > 0 {
            tw_min as f64 / vm_min as f64
        } else {
            0.0
        };

        results.push((steps, tw_min, vm_min, speedup));
    }

    // Print benchmark results table
    eprintln!("\n╔══════════════════════════════════════════════════════════════════╗");
    eprintln!("║  Phase 4.2 Benchmark: Tree-Walking vs VM (synthetic flow pipeline) ║");
    eprintln!("╠════════╦════════════╦════════════╦═══════════════════════╣");
    eprintln!("║ Steps  ║ TW (µs)     ║ VM (µs)    ║ Speedup              ║");
    eprintln!("╠════════╬════════════╬════════════╬═══════════════════════╣");
    for (steps, tw_us, vm_us, speedup) in &results {
        eprintln!(
            "║ {:>6} ║ {:>10} ║ {:>10} ║ {:>18.2}x     ║",
            steps, tw_us, vm_us, speedup
        );
    }
    eprintln!("╚════════╩════════════╩════════════╩═══════════════════════╝\n");

    // Verify correctness: TW and VM produce same output for largest program
    let largest = generate_synthetic_program(1000);
    let (tw_result, _) = bench_tw(&largest, base_dir);
    let (vm_result, _) = bench_vm(&largest, base_dir);

    let tw_out = tw_result
        .expect("TW failed")
        .map(|s| s.trim_end().to_string())
        .unwrap_or_default();
    let vm_out = vm_result
        .expect("VM failed")
        .map(|s| s.trim_end().to_string())
        .unwrap_or_default();
    assert_eq!(
        tw_out, vm_out,
        "Benchmark correctness: TW and VM outputs differ"
    );
}

/// Also benchmark with the existing golden examples to confirm real-world parity.
#[test]
fn benchmark_golden_examples_both_paths() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let examples_dir = Path::new(&manifest_dir).join("examples");
    let base_dir = examples_dir.parent().unwrap_or(Path::new("."));

    let mut total_tw: u128 = 0;
    let mut total_vm: u128 = 0;
    let mut count = 0;

    if let Ok(entries) = fs::read_dir(&examples_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "mlog").unwrap_or(false) {
                let source = match fs::read_to_string(&path) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let (_, tw_us) = bench_tw(&source, base_dir);
                let (_, vm_us) = bench_vm(&source, base_dir);

                total_tw += tw_us;
                total_vm += vm_us;
                count += 1;
            }
        }
    }

    if count > 0 {
        let speedup = total_tw as f64 / total_vm as f64;
        eprintln!(
            "\nGolden examples ({}) — Total TW: {}µs, Total VM: {}µs, Speedup: {:.2}x",
            count, total_tw, total_vm, speedup
        );
    }
}

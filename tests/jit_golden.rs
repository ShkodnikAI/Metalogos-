// ── Phase 4.3 JIT Tests: correctness + hot-path benchmarks ──────────

use std::path::Path;

/// Generate a .mlog program with N chained Increment steps.
/// Pattern: Increment(n: Float) -> Float { return n + 1.0 }
/// Flow: counter -> Increment -> ... (N times) -> output
fn generate_increment_program(steps: usize) -> String {
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

// /// Execute via VM with JIT enabled (threshold=1 for testing).
fn run_jit(
    source: &str,
    base_dir: &Path,
    threshold: usize,
) -> (Result<Option<String>, String>, u128) {
    // TODO: restore when Vm::with_jit is available
    let _ = (source, base_dir, threshold);
    (Err("JIT not available".to_string()), 0)
}

/// Execute via VM without JIT.
fn run_vm(source: &str, base_dir: &Path) -> (Result<Option<String>, String>, u128) {
    // TODO: restore when run_program_vm_with_base is available
    let _ = (source, base_dir);
    (Err("VM base runner not available".to_string()), 0)
}

/// Execute via tree-walking interpreter.
fn run_tw(source: &str, base_dir: &Path) -> (Result<Option<String>, String>, u128) {
    // TODO: restore when run_program_with_base is available
    let _ = (source, base_dir);
    (Err("TW base runner not available".to_string()), 0)
}

/// Helper: trim trailing whitespace from Option<String>.
fn trim_opt(s: &Option<String>) -> String {
    s.as_deref().map(|v| v.trim_end()).unwrap_or("").to_string()
}

// ── JIT Correctness Tests ─────────────────────────────────────────

#[test]
#[ignore = "TODO: JIT not yet integrated — Vm::with_jit unavailable (ADR-0073)"]
/// Category: VM Unimplemented (JIT not yet integrated)
// TODO: restore when Vm::with_jit and run_program_* are available
fn jit_hot_pattern_correctness() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let examples_dir = Path::new(&manifest_dir).join("examples");
    let base_dir = examples_dir.parent().unwrap_or(Path::new("."));

    // Use threshold=1 so JIT kicks in immediately
    let program = generate_increment_program(10);

    let (tw_result, _) = run_tw(&program, base_dir);
    let (vm_result, _) = run_vm(&program, base_dir);
    let (jit_result, _) = run_jit(&program, base_dir, 1);

    let tw_out = tw_result.expect("TW failed");
    let vm_out = vm_result.expect("VM failed");
    let jit_out = jit_result.expect("JIT failed");

    assert_eq!(trim_opt(&tw_out), trim_opt(&vm_out), "VM vs TW mismatch");
    assert_eq!(trim_opt(&tw_out), trim_opt(&jit_out), "JIT vs TW mismatch");
}

#[test]
#[ignore = "TODO: JIT not yet integrated — Vm::with_jit unavailable (ADR-0073)"]
/// Category: VM Unimplemented (JIT not yet integrated)
// TODO: restore when Vm::with_jit and run_program_* are available
fn jit_large_program_correctness() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let examples_dir = Path::new(&manifest_dir).join("examples");
    let base_dir = examples_dir.parent().unwrap_or(Path::new("."));

    // 100 Increment steps: counter goes from 0.0 to 100.0
    let program = generate_increment_program(100);

    let (tw_result, _) = run_tw(&program, base_dir);
    let (jit_result, _) = run_jit(&program, base_dir, 1);

    let tw_out = tw_result.expect("TW failed");
    let jit_out = jit_result.expect("JIT failed");

    assert_eq!(
        trim_opt(&tw_out),
        trim_opt(&jit_out),
        "JIT vs TW mismatch for 100-step program"
    );
}

#[test]
#[ignore = "TODO: JIT not yet integrated — Vm::with_jit unavailable (ADR-0073)"]
/// Category: VM Unimplemented (JIT not yet integrated)
// TODO: restore when Vm::with_jit is available
fn jit_compilation_actually_happens() { // TODO: restore when Vm::with_jit is available    // let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());    // let examples_dir = Path::new(&manifest_dir).join("examples");    // let base_dir = examples_dir.parent().unwrap_or(Path::new("."));    // let program = generate_increment_program(10);    // let decls = metalogos::parser::parse(&program).expect("parse failed");    // let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir.to_path_buf());    // let prog = comp.compile(decls).expect("compile failed");    // let mut vm = metalogos::vm::Vm::with_jit(1).expect("JIT init failed");    // let _result = vm.run(prog).expect("run failed");    // let compiled_count = vm.jit_compiled_count();    // assert!(compiled_count >= 1,    //     "Expected at least 1 JIT-compiled pattern, got {}", compiled_count);
}

// ── JIT Benchmark Tests ────────────────────────────────────────────

#[test]
#[ignore = "TODO: JIT not yet integrated — Vm::with_jit unavailable (ADR-0073)"]
/// Category: VM Unimplemented (JIT not yet integrated)
// TODO: restore when Vm::with_jit and run_program_vm_with_base are available
fn benchmark_vm_vs_jit() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let examples_dir = Path::new(&manifest_dir).join("examples");
    let base_dir = examples_dir.parent().unwrap_or(Path::new("."));

    // Warm-up
    let warmup = generate_increment_program(50);
    let _ = run_vm(&warmup, base_dir);
    let _ = run_jit(&warmup, base_dir, 1);

    let step_counts = [10, 50, 100, 500, 1000];
    let mut results = Vec::new();

    for &steps in &step_counts {
        let program = generate_increment_program(steps);

        // VM-only: 3 runs, take minimum
        let mut vm_min = u128::MAX;
        for _ in 0..3 {
            let (_, us) = run_vm(&program, base_dir);
            vm_min = vm_min.min(us);
        }

        // VM+JIT (threshold=1): 3 runs, take minimum
        let mut jit_min = u128::MAX;
        for _ in 0..3 {
            let (_, us) = run_jit(&program, base_dir, 1);
            jit_min = jit_min.min(us);
        }

        let speedup = if jit_min > 0 {
            vm_min as f64 / jit_min as f64
        } else {
            0.0
        };

        results.push((steps, vm_min, jit_min, speedup));
    }

    // Print benchmark results
    eprintln!("\n╔═══════════════════════════════════════════════════════════════════╗");
    eprintln!("║  Phase 4.3 Benchmark: VM-only vs VM+JIT (Cranelift native code)     ║");
    eprintln!("╠════════╦════════════╦════════════╦═════════════════════════════╣");
    eprintln!("║ Steps  ║ VM (µs)    ║ JIT (µs)   ║ JIT Speedup vs VM         ║");
    eprintln!("╠════════╬════════════╬════════════╬═════════════════════════════╣");
    for (steps, vm_us, jit_us, speedup) in &results {
        eprintln!(
            "║ {:>6} ║ {:>10} ║ {:>10} ║ {:>24.2}x       ║",
            steps, vm_us, jit_us, speedup
        );
    }
    eprintln!("╚════════╩════════════╩════════════╩═════════════════════════════╝\n");

    // Verify correctness for the largest program
    let largest = generate_increment_program(1000);
    let (tw_result, _) = run_tw(&largest, base_dir);
    let (jit_result_largest, _) = run_jit(&largest, base_dir, 1);
    let tw_out = tw_result
        .expect("TW failed")
        .map(|s| s.trim_end().to_string())
        .unwrap_or_default();
    let jit_out = jit_result_largest
        .expect("JIT failed")
        .map(|s| s.trim_end().to_string())
        .unwrap_or_default();
    assert_eq!(
        tw_out, jit_out,
        "Benchmark correctness: TW and JIT outputs differ"
    );
}

/// Triple-mode benchmark: TW vs VM vs JIT for all golden examples.
#[test]
#[ignore = "TODO: JIT not yet integrated — Vm::with_jit unavailable (ADR-0073)"]
/// Category: VM Unimplemented (JIT not yet integrated)
// TODO: restore when Vm::with_jit and run_program_* are available
fn benchmark_triple_mode_golden() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let examples_dir = Path::new(&manifest_dir).join("examples");
    let base_dir = examples_dir.parent().unwrap_or(Path::new("."));

    let mut total_tw: u128 = 0;
    let mut total_vm: u128 = 0;
    let mut total_jit: u128 = 0;
    let mut count = 0;

    if let Ok(entries) = std::fs::read_dir(&examples_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "mlog").unwrap_or(false) {
                let source = match std::fs::read_to_string(&path) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let (_, tw_us) = run_tw(&source, base_dir);
                let (_, vm_us) = run_vm(&source, base_dir);
                let (_, jit_us) = run_jit(&source, base_dir, 1);

                total_tw += tw_us;
                total_vm += vm_us;
                total_jit += jit_us;
                count += 1;
            }
        }
    }

    if count > 0 {
        let jit_vs_tw = total_tw as f64 / total_jit as f64;
        let jit_vs_vm = total_vm as f64 / total_jit as f64;
        let vm_vs_tw = total_tw as f64 / total_vm as f64;
        eprintln!(
            "\nGolden examples ({}) — TW: {}µs, VM: {}µs, JIT: {}µs",
            count, total_tw, total_vm, total_jit
        );
        eprintln!("  VM vs TW:    {:.2}x", vm_vs_tw);
        eprintln!("  JIT vs TW:   {:.2}x", jit_vs_tw);
        eprintln!("  JIT vs VM:   {:.2}x", jit_vs_vm);
    }
}

#[test]
#[ignore = "TODO: JIT not yet integrated — Vm::with_jit unavailable (ADR-0073)"]
/// Category: VM Unimplemented (JIT not yet integrated)
// TODO: restore when Vm::with_jit and run_program_* are available
fn jit_p5_golden_example() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let examples_dir = Path::new(&manifest_dir).join("examples");
    let base_dir = examples_dir.parent().unwrap_or(Path::new("."));

    let source = std::fs::read_to_string(examples_dir.join("p5_jit_hot.mlog"))
        .expect("cannot read p5_jit_hot.mlog");
    let expected = std::fs::read_to_string(examples_dir.join("p5_jit_hot.expected"))
        .expect("cannot read p5_jit_hot.expected");

    // All three modes must produce the same output
    let tw_result = run_tw(&source, base_dir).0.expect("TW failed");
    let vm_result = run_vm(&source, base_dir).0.expect("VM failed");
    let jit_result = run_jit(&source, base_dir, 1).0.expect("JIT failed");

    let tw_out = trim_opt(&tw_result);
    let vm_out = trim_opt(&vm_result);
    let jit_out = trim_opt(&jit_result);
    let expected_trimmed = expected.trim_end();

    assert_eq!(tw_out, expected_trimmed, "TW output differs from expected");
    assert_eq!(vm_out, expected_trimmed, "VM output differs from expected");
    assert_eq!(
        jit_out, expected_trimmed,
        "JIT output differs from expected"
    );
}

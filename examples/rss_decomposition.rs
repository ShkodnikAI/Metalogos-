// ── Naryad №409 (issue #554), Step A — VM-serve peak-RSS decomposition ──
//
// ADR-0141 Addendum 4 requires the resident/peak footprint of the VM-serve
// request path decomposed by state class: the builtins registry (Vm::new),
// the program-table slots (load_program without db), the per-request db
// connection (in-memory sqlite + schema DDL), the reflex/vision model
// registration, the pool idle set, and the per-request execution scratch.
//
// Method (mechanical, reproducible): ONE probe process per class; inside the
// process the probe allocates N live instances of that class and reports the
// RSS slope (delta VmRSS / N) plus the VmHWM high-water mark. The RSS source
// is /proc/self/status — the same source the pinned Stage 4 benchmark uses
// (benches/stage4_benchmark.rs peak_rss_kb), so the numbers are comparable.
// The fixture is the production-class corpus of the №398/№404 protocol
// (benches/fixtures/production_workload.mlog, 2344 lines, 14 routes).
//
// Usage: cargo run --release --example rss_decomposition -- --probe <name> [--n N]
//   probes: vm_new | load_nodb | load_db | pool_idle | exec_loop | reflex_model | summary_env
//
// The driver script scripts/rss_decomposition.sh runs every probe in its own
// process and assembles the decomposition table.

use metalogos::bytecode::Program;
use metalogos::vm::Vm;
use std::sync::Arc;

/// Resident set size right now (kB) from /proc/self/status — the same
/// source as the pinned benchmark's peak_rss_kb.
fn rss_now_kb() -> u64 {
    let path = "/proc/self/status";
    match std::fs::read_to_string(path) {
        Ok(s) => {
            for line in s.lines() {
                if let Some(rest) = line.strip_prefix("VmRSS:") {
                    let kib = rest
                        .trim()
                        .trim_end_matches("kB")
                        .trim()
                        .parse::<u64>()
                        .unwrap_or(0);
                    return kib;
                }
            }
            0
        }
        Err(_) => 0,
    }
}

/// Peak resident set size high-water mark (kB) from /proc/self/status.
fn hwm_now_kb() -> u64 {
    let path = "/proc/self/status";
    match std::fs::read_to_string(path) {
        Ok(s) => {
            for line in s.lines() {
                if let Some(rest) = line.strip_prefix("VmHWM:") {
                    let kib = rest
                        .trim()
                        .trim_end_matches("kB")
                        .trim()
                        .parse::<u64>()
                        .unwrap_or(0);
                    return kib;
                }
            }
            0
        }
        Err(_) => 0,
    }
}

fn manifest_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture_corpus() -> String {
    let p = manifest_dir().join("benches/fixtures/production_workload.mlog");
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {}", p.display(), e))
}

/// Compile the fixture corpus exactly like the pinned benchmark child does:
/// parse → Compiler::with_std_root(manifest_dir) → compile → compile_routes.
fn compile_fixture() -> (Arc<Program>, Vec<metalogos::bytecode::CompiledRoute>) {
    let corpus = fixture_corpus();
    let decls = metalogos::parser::parse(&corpus).expect("parse fixture");
    let server_cfg = decls
        .iter()
        .find_map(|d| match d {
            metalogos::ast::Declaration::MlogServer(s) => Some(s.clone()),
            _ => None,
        })
        .expect("mlogserver block in fixture");
    let mut comp = metalogos::compiler::Compiler::with_std_root(manifest_dir());
    let program = comp.compile(decls).expect("compile fixture");
    let compiled = comp
        .compile_routes(&server_cfg.routes)
        .expect("compile fixture routes");
    (Arc::new(program), compiled)
}

/// A small synthetic program WITH a reflex declaration (dense only —
/// no candle feature), used by the reflex_model probe to price the
/// per-declaration model construction class that load_program pays
/// when the program declares reflex models.
fn reflex_program_source() -> String {
    // Same shape as examples/reflex_persist.mlog (naryad №180/№204 canon):
    // reflex declaration + a trivial pattern — no flow needed to compile.
    "reflex ProbeClassifier {\n\
     \x20 input: embedding(2)\n\
     \x20 layers: [dense(8, \"relu\"), dense(2, \"softmax\")]\n\
     \x20 labels: [\"near\", \"far\"]\n\
     \x20 seed: 42\n\
     }\n\
     pattern Probe(_x: String) -> String { return \"ok\" }\n"
        .to_string()
}

fn compile_source(source: &str) -> Arc<Program> {
    let decls = metalogos::parser::parse(source).expect("parse probe source");
    let mut comp = metalogos::compiler::Compiler::with_std_root(manifest_dir());
    Arc::new(comp.compile(decls).expect("compile probe source"))
}

fn print_probe(name: &str, n: usize, rss_before: u64, rss_after: u64, hwm_before: u64) {
    let hwm_after = hwm_now_kb();
    let per_instance_bytes = if n > 0 {
        (rss_after.saturating_sub(rss_before)) * 1024 / n as u64
    } else {
        0
    };
    println!(
        "{{\"probe\":\"{}\",\"n\":{},\"rss_before_kb\":{},\"rss_after_kb\":{},\"per_instance_bytes\":{},\"hwm_before_kb\":{},\"hwm_after_kb\":{},\"hwm_delta_kb\":{}}}",
        name,
        n,
        rss_before,
        rss_after,
        per_instance_bytes,
        hwm_before,
        hwm_after,
        hwm_after.saturating_sub(hwm_before)
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut probe = String::new();
    let mut n: usize = 100;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--probe" => {
                i += 1;
                probe = args.get(i).cloned().unwrap_or_default();
            }
            "--n" => {
                i += 1;
                n = args.get(i).and_then(|v| v.parse().ok()).unwrap_or(100);
            }
            other => panic!("unknown arg {}", other),
        }
        i += 1;
    }
    match probe.as_str() {
        // ── Class: Vm::new() — builtins registry + builtin_names ──
        "vm_new" => {
            let before = rss_now_kb();
            let hwm = hwm_now_kb();
            let mut vms: Vec<Vm> = Vec::with_capacity(n);
            for _ in 0..n {
                vms.push(Vm::new());
            }
            let after = rss_now_kb();
            std::hint::black_box(&vms);
            print_probe("vm_new", n, before, after, hwm);
        }
        // ── Class: load_program WITHOUT db — globals slots + shared
        //    snapshot Arcs + decl maps (program tables per-VM share) ──
        "load_nodb" => {
            let (program, _) = compile_fixture();
            let mut nodb_src: Program = (*program).clone();
            nodb_src.db_url = None; // isolate the non-db load class
            let nodb = Arc::new(nodb_src);
            let before = rss_now_kb();
            let hwm = hwm_now_kb();
            let mut vms: Vec<Vm> = Vec::with_capacity(n);
            for _ in 0..n {
                let mut vm = Vm::new();
                vm.load_program(&nodb).expect("load nodb");
                vms.push(vm);
            }
            let after = rss_now_kb();
            std::hint::black_box(&vms);
            print_probe("load_nodb", n, before, after, hwm);
        }
        // ── Class: load_program WITH db — adds the in-memory sqlite
        //    connection + schema DDL per instance (the №381 class) ──
        "load_db" => {
            let (program, _) = compile_fixture();
            let before = rss_now_kb();
            let hwm = hwm_now_kb();
            let mut vms: Vec<Vm> = Vec::with_capacity(n);
            for _ in 0..n {
                let mut vm = Vm::new();
                vm.load_program(&program).expect("load db");
                vms.push(vm);
            }
            let after = rss_now_kb();
            std::hint::black_box(&vms);
            print_probe("load_db", n, before, after, hwm);
        }
        // ── Class: pool idle set — N reset-and-reloaded VMs resident in
        //    the idle set (№403), the residency a pool-ON deployment pays ──
        "pool_idle" => {
            let (program, _) = compile_fixture();
            let before = rss_now_kb();
            let hwm = hwm_now_kb();
            let pool = metalogos::vm_pool::VmPool::with_max(program.clone(), n);
            // Hold ALL N VMs outside the pool first (each is a cold build),
            // then check them in — only then does the idle set hold N.
            let mut held: Vec<metalogos::vm::Vm> = Vec::with_capacity(n);
            for _ in 0..n {
                held.push(pool.checkout().expect("cold checkout"));
            }
            for vm in held {
                pool.checkin(vm, true);
            }
            let after = rss_now_kb();
            let s = pool.stats();
            assert_eq!(s.idle_now, n as u64, "idle set must hold N");
            std::hint::black_box(&pool);
            print_probe("pool_idle", n, before, after, hwm);
        }
        // ── Class: per-request execution scratch — fresh VM + load +
        //    one real fixture route execution, repeated; the HWM delta
        //    over the loop is the per-request transient peak observed ──
        "exec_loop" => {
            let (program, compiled) = compile_fixture();
            // The db-shaped route exercises the heaviest per-request class
            // stack (load + sqlite + DDL + execution scratch).
            let route = compiled
                .iter()
                .find(|r| r.path == "/db/orders")
                .expect("/db/orders route")
                .clone();
            let hwm = hwm_now_kb();
            // Warm once (allocator arenas grow to steady state).
            {
                let mut vm = Vm::new();
                vm.load_program(&program).expect("warm load");
                let _ = vm.execute_route_code(&route, &program);
            }
            let rss_start = rss_now_kb();
            for _ in 0..n {
                let mut vm = Vm::new();
                vm.load_program(&program).expect("loop load");
                let _ = vm.execute_route_code(&route, &program);
            }
            let rss_end = rss_now_kb();
            let hwm_end = hwm_now_kb();
            println!(
                "{{\"probe\":\"exec_loop\",\"n\":{},\"rss_start_kb\":{},\"rss_end_kb\":{},\"retained_per_request_bytes\":{},\"hwm_before_kb\":{},\"hwm_after_kb\":{},\"hwm_delta_kb\":{}}}",
                n,
                rss_start,
                rss_end,
                (rss_end.saturating_sub(rss_start)) * 1024 / n.max(1) as u64,
                hwm,
                hwm_end,
                hwm_end.saturating_sub(hwm)
            );
        }
        // ── Class: reflex model construction (per declared model) ──
        "reflex_model" => {
            let program = compile_source(&reflex_program_source());
            assert!(
                !program.reflex_decls.is_empty(),
                "probe program must declare a reflex model"
            );
            let before = rss_now_kb();
            let hwm = hwm_now_kb();
            let mut vms: Vec<Vm> = Vec::with_capacity(n);
            for _ in 0..n {
                let mut vm = Vm::new();
                vm.load_program(&program).expect("load reflex");
                vms.push(vm);
            }
            let after = rss_now_kb();
            std::hint::black_box(&vms);
            print_probe("reflex_model", n, before, after, hwm);
        }
        other => panic!(
            "unknown probe '{}' (vm_new|load_nodb|load_db|pool_idle|exec_loop|reflex_model)",
            other
        ),
    }
}

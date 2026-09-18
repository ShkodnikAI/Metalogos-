// ── Stage 4 real-load benchmark — naryad #381 (issue #467) ─────────
//
// ADR-0141 §D5: benchmark on a production-class .mlog workload
// (>= 2000 lines, LLM/vision/DB-shaped I/O via DETERMINISTIC MOCKS)
// before the `mlog serve` default flip. Verdict criterion (§D5):
//   - >= 2x latency improvement, OR
//   - equivalent latency with a significant memory/CPU win.
// Without one of these — the default flip does not happen.
//
// Usage: cargo bench --bench stage4_benchmark
//
// The parent process (default) spawns ITSELF twice — once per backend
// (`--child interpreter|vm`) — so each backend runs in its own process
// (clean peak-RSS high-water mark per backend, identical machine/env).
// Each child:
//   1. measures startup separately: parse (both backends) and the
//      bytecode compile (VM-only startup cost), median of 5 runs;
//   2. starts the test server on 127.0.0.1:0 with the given backend;
//   3. warms every route up, then runs ROUNDS request cycles (>= 5,
//      default 30) over real loopback HTTP against ALL benchmark
//      routes of the deterministic request plan
//      (benches/fixtures/stage4_routes.json);
//   4. reports per-route latency samples (p50/p95/mean, µs), the
//      per-cycle total, and the process peak RSS (VmHWM);
//   5. prints one JSON object on stdout.
// The parent assembles the comparison table + the loud §D5 verdict and
// writes the combined raw JSON report.
//
// WHAT IS MEASURED (loud, per the naryad): DSL execution — call_llm /
// vision_understand / stt_transcribe run in deterministic mock mode
// (no network, no keys), the DB is in-process sqlite::memory:, kv is
// in-process. Real LLM traffic would mask the backend delta. The
// loopback HTTP layer is identical for both backends.

#[cfg(feature = "server")]
mod stage4 {
    use metalogos::server::{run_test_server_with_backend, ServeBackend};
    use serde_json::Value;
    use std::io::Read;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    pub const CORPUS_REL: &str = "benches/fixtures/production_workload.mlog";
    pub const PLAN_REL: &str = "benches/fixtures/stage4_routes.json";
    pub const DEFAULT_ROUNDS: u32 = 30;
    /// Latency statistics are only meaningful on a stable machine; the
    /// parent runs each backend in its own process and the naryad
    /// requires >= 5 iterations — 30 rounds x 14 routes is the default.
    pub const STARTUP_RUNS: u32 = 5;

    // ── Banned substrings (sanitize gate, mirrors the generator) ──
    pub const BANNED: [&str; 11] = [
        "sk-",
        "api_key",
        "apikey",
        "bearer ",
        "private key",
        "akia",
        "ghp_",
        "xox",
        "secret:",
        "token:",
        "authorization:",
    ];

    pub fn manifest_dir() -> PathBuf {
        std::env::var("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().expect("cwd"))
    }

    pub fn read_corpus() -> String {
        let path = manifest_dir().join(CORPUS_REL);
        std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read corpus {}: {}", path.display(), e))
    }

    pub fn read_plan() -> Value {
        let path = manifest_dir().join(PLAN_REL);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read plan {}: {}", path.display(), e));
        serde_json::from_str(&text).expect("plan JSON")
    }

    pub fn sanitize_hits(text: &str) -> Vec<&'static str> {
        let lower = text.to_lowercase();
        BANNED
            .iter()
            .copied()
            .filter(|b| lower.contains(b))
            .collect()
    }

    /// Peak resident set size (kB) from /proc/self/status — the same
    /// number `/usr/bin/time -v` reports as "Maximum resident set size"
    /// (this is the documented analog, Linux /proc is always present on
    /// the pinned ubuntu-latest runner).
    pub fn peak_rss_kb() -> u64 {
        let path = "/proc/self/status";
        let text = std::fs::read_to_string(path).unwrap_or_default();
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("VmHWM:") {
                let num: String = rest.chars().filter(|c| c.is_ascii_digit()).collect();
                return num.parse().unwrap_or(0);
            }
        }
        0
    }

    fn percentile(sorted: &[f64], q: f64) -> f64 {
        // Nearest-rank percentile.
        if sorted.is_empty() {
            return 0.0;
        }
        let n = sorted.len();
        let idx = ((q * n as f64).ceil() as usize).clamp(1, n) - 1;
        sorted[idx]
    }

    pub fn summarize(samples_us: &[u64]) -> (f64, f64, f64) {
        let mut s: Vec<f64> = samples_us.iter().map(|x| *x as f64).collect();
        s.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p50 = percentile(&s, 0.50);
        let p95 = percentile(&s, 0.95);
        let mean = if s.is_empty() {
            0.0
        } else {
            s.iter().sum::<f64>() / s.len() as f64
        };
        (p50, p95, mean)
    }

    fn median_of<F: FnMut() -> f64>(mut f: F, runs: u32) -> f64 {
        let mut v: Vec<f64> = (0..runs).map(|_| f()).collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        if v.is_empty() {
            0.0
        } else {
            v[v.len() / 2]
        }
    }

    // ── Child mode: measure one backend end-to-end ─────────────────
    pub fn run_child(backend: ServeBackend, rounds: u32) -> Value {
        let corpus = read_corpus();
        assert!(
            corpus.lines().count() >= 2000,
            "corpus must be >= 2000 lines (ADR-0141 §D5), got {}",
            corpus.lines().count()
        );
        let plan = read_plan();
        let routes = plan["routes"].as_array().expect("plan.routes").clone();

        // Deterministic mocks (loud in the report: DSL execution, not I/O).
        std::env::set_var("METALOGOS_MOCK_LLM", "true");
        std::env::set_var("METALOGOS_LLM_MOCK", "true");

        // ── Startup: parse (both backends) and compile (VM only) ──
        let parse_ms = median_of(
            || {
                let t = Instant::now();
                let decls = metalogos::parser::parse(&corpus).expect("parse");
                let _ = metalogos::semantic::check_program(&decls);
                t.elapsed().as_secs_f64() * 1000.0
            },
            STARTUP_RUNS,
        );
        let (vm_compile_ms, vm_routes_ms) = if backend == ServeBackend::Vm {
            let compile_ms = median_of(
                || {
                    let t = Instant::now();
                    let program = metalogos::compile_program(&corpus).expect("vm compile");
                    std::hint::black_box(&program);
                    t.elapsed().as_secs_f64() * 1000.0
                },
                STARTUP_RUNS,
            );
            // Route-only compilation mirrors what the server does on top
            // of the program compile (run_test_server_with_backend):
            // the program must be compiled into the SAME Compiler first,
            // otherwise route bodies cannot resolve the patterns.
            let routes_ms = median_of(
                || {
                    let decls = metalogos::parser::parse(&corpus).expect("parse");
                    let server_cfg = decls.iter().find_map(|d| match d {
                        metalogos::ast::Declaration::MlogServer(s) => Some(s.clone()),
                        _ => None,
                    });
                    let mut comp = metalogos::compiler::Compiler::with_std_root(manifest_dir());
                    let program = comp.compile(decls).expect("vm compile");
                    std::hint::black_box(&program);
                    let t = Instant::now();
                    let compiled = comp
                        .compile_routes(&server_cfg.expect("mlogserver block").routes)
                        .expect("vm route compile");
                    std::hint::black_box(&compiled);
                    t.elapsed().as_secs_f64() * 1000.0
                },
                STARTUP_RUNS,
            );
            (compile_ms, routes_ms)
        } else {
            (0.0, 0.0)
        };

        // ── DSL-only measurement (no HTTP constant) ────────────────
        // §D5 asks about the BACKEND delta on DSL execution; the loopback
        // HTTP layer adds an identical ~constant to both backends and
        // compresses the ratio. This second measurement runs the corpus
        // Bench flow (the heaviest stats chain) directly through the
        // backend — run() for TW, run_bytecode() for VM — no HTTP, no
        // server. Documented biases: TW pays a declarations clone per run
        // (Vec consumed by run()), VM pays a Program clone per run
        // (run_bytecode takes ownership) — both are whole-program clone
        // costs, symmetric and conservative.
        const DSL_RUNS: u32 = 20;
        // Per-run whole-program clone/setup baseline (measured, subtracted
        // in the report): TW pays a full AST Vec clone (run() consumes it),
        // VM pays a full bytecode Program clone (run_bytecode consumes it).
        // Without this baseline both DSL-only readings are clone-dominated
        // and the §D5 execution delta would be hidden.
        let clone_baseline_us: Vec<u64> = if backend == ServeBackend::Vm {
            let program = metalogos::compile_program(&corpus).expect("vm compile (baseline)");
            (0..10)
                .map(|_| {
                    let t = Instant::now();
                    std::hint::black_box(program.clone());
                    t.elapsed().as_micros() as u64
                })
                .collect()
        } else {
            let decls = metalogos::parser::parse(&corpus).expect("parse (baseline)");
            (0..10)
                .map(|_| {
                    let t = Instant::now();
                    std::hint::black_box(decls.clone());
                    t.elapsed().as_micros() as u64
                })
                .collect()
        };
        let (_, _, clone_mean) = summarize(&clone_baseline_us);
        let dsl_samples: Vec<u64> = if backend == ServeBackend::Vm {
            let program = metalogos::compile_program(&corpus).expect("vm compile (dsl-only)");
            (0..DSL_RUNS)
                .map(|_| {
                    let t = Instant::now();
                    let out = metalogos::run_bytecode(program.clone()).expect("vm flow run");
                    std::hint::black_box(&out);
                    t.elapsed().as_micros() as u64
                })
                .collect()
        } else {
            let decls = metalogos::parser::parse(&corpus).expect("parse (dsl-only)");
            (0..DSL_RUNS)
                .map(|_| {
                    let mut interp = metalogos::interpreter::Interpreter::new();
                    interp.set_base_dir(manifest_dir());
                    let t = Instant::now();
                    let out = interp.run(decls.clone()).expect("tw flow run");
                    std::hint::black_box(&out);
                    t.elapsed().as_micros() as u64
                })
                .collect()
        };
        let (dp50, dp95, dmean) = summarize(&dsl_samples);

        // ── Start the server on the given backend ──
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .expect("tokio runtime");
        let (port, handle) = rt
            .block_on(run_test_server_with_backend(&corpus, backend))
            .expect("test server should start");

        // Requests run on a plain thread (no tokio context): the blocking
        // reqwest client requires that (ADR-0096 family lesson).
        let plan_routes = routes.clone();
        let measurer = std::thread::spawn(move || -> Result<Value, String> {
            let client = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|e| format!("client: {}", e))?;

            let url_for = |r: &Value| -> String {
                let mut url = format!(
                    "http://127.0.0.1:{}{}",
                    port,
                    r["path"].as_str().unwrap_or("/")
                );
                if let Some(q) = r["query"].as_object() {
                    let qs: Vec<String> = q
                        .iter()
                        .map(|(k, v)| format!("{}={}", k, v.as_str().unwrap_or_default()))
                        .collect();
                    if !qs.is_empty() {
                        url.push('?');
                        url.push_str(&qs.join("&"));
                    }
                }
                url
            };

            let hit = |r: &Value| -> Result<u64, String> {
                let url = url_for(r);
                let t = Instant::now();
                let resp = match r["method"].as_str() {
                    Some("POST") => client
                        .post(&url)
                        .json(r["body"].as_object().unwrap_or(&serde_json::Map::new()))
                        .send()
                        .map_err(|e| format!("POST {}: {}", url, e))?,
                    _ => client
                        .get(&url)
                        .send()
                        .map_err(|e| format!("GET {}: {}", url, e))?,
                };
                let status = resp.status().as_u16();
                let body_text = resp.text().map_err(|e| format!("body: {}", e))?;
                if status != 200 {
                    return Err(format!(
                        "{} {} -> HTTP {}: {}",
                        r["path"], url, status, body_text
                    ));
                }
                Ok(t.elapsed().as_micros() as u64)
            };

            // Warm-up: 2 full cycles (not measured).
            for _ in 0..2 {
                for r in &plan_routes {
                    hit(r)?;
                }
            }

            // Measured rounds.
            let n = plan_routes.len();
            let mut per_route: Vec<Vec<u64>> = vec![Vec::new(); n];
            let mut cycle_totals: Vec<u64> = Vec::new();
            for _ in 0..rounds {
                let mut cycle = 0u64;
                for (i, r) in plan_routes.iter().enumerate() {
                    let us = hit(r)?;
                    cycle += us;
                    per_route[i].push(us);
                }
                cycle_totals.push(cycle);
            }

            let mut route_stats = Vec::new();
            for (i, r) in plan_routes.iter().enumerate() {
                let (p50, p95, mean) = summarize(&per_route[i]);
                route_stats.push(serde_json::json!({
                    "name": r["name"],
                    "path": r["path"],
                    "method": r["method"],
                    "samples_us": per_route[i],
                    "p50_us": p50,
                    "p95_us": p95,
                    "mean_us": mean,
                }));
            }
            let (cp50, cp95, cmean) = summarize(&cycle_totals);
            Ok(serde_json::json!({
                "cycle": { "p50_us": cp50, "p95_us": cp95, "mean_us": cmean },
                "routes": route_stats,
            }))
        });

        let measured = measurer.join().expect("measure thread panicked");
        handle.abort();
        rt.shutdown_timeout(Duration::from_secs(2));

        let measured = measured.expect("request loop failed (loud fail per naryad #381)");
        let rss_peak_kb = peak_rss_kb();

        let mut out = serde_json::json!({
            "backend": match backend {
                ServeBackend::Vm => "vm",
                _ => "interpreter",
            },
            "corpus": CORPUS_REL,
            "corpus_lines": corpus.lines().count(),
            "rounds": rounds,
            "startup": {
                "parse_semantic_ms": parse_ms,
                "vm_compile_ms": vm_compile_ms,
                "vm_route_compile_ms": vm_routes_ms,
            },
            "dsl_only": {
                "runs": DSL_RUNS,
                "p50_us": dp50,
                "p95_us": dp95,
                "mean_us": dmean,
                "samples_us": dsl_samples,
                "clone_baseline_mean_us": clone_mean,
                "net_mean_us": (dmean - clone_mean).max(0.0),
                "note": "Bench flow via run()/run_bytecode(); totals include the per-run whole-program clone (measured baseline reported alongside); net = total - baseline",
            },
            "rss_peak_kb": rss_peak_kb,
            "note": "deterministic mocks: call_llm/vision_understand/stt_transcribe mocked; db = sqlite::memory:; measured = DSL execution over loopback HTTP",
        });
        if let (Some(base), Some(m)) = (out.as_object_mut(), measured.as_object()) {
            for (k, v) in m {
                base.insert(k.clone(), v.clone());
            }
        }
        out
    }

    // ── Parent mode: spawn children, compare, verdict ──────────────
    pub fn run_parent(rounds: u32) -> i32 {
        // Sanitize gate + corpus size gate (loud, §Сделано-когда 1).
        let corpus = read_corpus();
        let lines = corpus.lines().count();
        let hits = sanitize_hits(&corpus);
        println!("corpus: {} ({} lines)", CORPUS_REL, lines);
        println!(
            "sanitize: {}",
            if hits.is_empty() {
                "0 hits (clean)".to_string()
            } else {
                format!("BANNED PATTERN PRESENT: {:?}", hits)
            }
        );
        if lines < 2000 || !hits.is_empty() {
            eprintln!("FAIL: corpus gate (>= 2000 lines, sanitize 0) violated");
            return 2;
        }

        // Same-process determinism env for children (inherited).
        std::env::set_var("METALOGOS_MOCK_LLM", "true");
        std::env::set_var("METALOGOS_LLM_MOCK", "true");

        let self_exe = std::env::current_exe().expect("current_exe");
        let mut results = Vec::new();
        for backend in ["interpreter", "vm"] {
            println!(
                "measuring backend: {} ({} rounds x 14 routes)...",
                backend, rounds
            );
            let out = std::process::Command::new(&self_exe)
                .args(["--bench-child", backend, "--rounds", &rounds.to_string()])
                .output()
                .expect("spawn child");
            let mut stdout = String::new();
            let _ = std::io::Cursor::new(&out.stdout).read_to_string(&mut stdout);
            if !out.status.success() {
                eprintln!(
                    "LOUD FAIL (naryad #381, outcome 4): backend {} child failed:\n{}{}",
                    backend,
                    stdout,
                    String::from_utf8_lossy(&out.stderr)
                );
                return 3;
            }
            // The child prints exactly one JSON object on stdout — take from
            // the FIRST '{' to the LAST '}' (nested objects make rfind('{')
            // wrong; the child's stdout carries no other braces).
            let text = String::from_utf8_lossy(&out.stdout);
            let start = text.find('{').expect("child JSON start");
            let end = text.rfind('}').expect("child JSON end") + 1;
            let raw_tail = &text[start..end];
            let v: Value = serde_json::from_str(raw_tail.trim())
                .unwrap_or_else(|e| panic!("child JSON parse: {} — raw: {}", e, raw_tail));
            results.push(v);
        }

        let interp = &results[0];
        let vm = &results[1];
        let i_cycle = interp["cycle"]["mean_us"].as_f64().unwrap_or(0.0);
        let v_cycle = vm["cycle"]["mean_us"].as_f64().unwrap_or(0.0);
        let i_p50 = interp["cycle"]["p50_us"].as_f64().unwrap_or(0.0);
        let v_p50 = vm["cycle"]["p50_us"].as_f64().unwrap_or(0.0);
        let i_p95 = interp["cycle"]["p95_us"].as_f64().unwrap_or(0.0);
        let v_p95 = vm["cycle"]["p95_us"].as_f64().unwrap_or(0.0);
        let i_rss = interp["rss_peak_kb"].as_f64().unwrap_or(0.0);
        let v_rss = vm["rss_peak_kb"].as_f64().unwrap_or(0.0);
        let speedup = if v_cycle > 0.0 {
            i_cycle / v_cycle
        } else {
            0.0
        };
        let i_dsl = interp["dsl_only"]["mean_us"].as_f64().unwrap_or(0.0);
        let v_dsl = vm["dsl_only"]["mean_us"].as_f64().unwrap_or(0.0);
        let dsl_speedup = if v_dsl > 0.0 { i_dsl / v_dsl } else { 0.0 };
        let rss_ratio = if i_rss > 0.0 { v_rss / i_rss } else { 1.0 };

        // ── §D5 verdict (loud, three outcomes) ─────────────────────
        // Primary reading: the REQUEST CYCLE — the metric the naryad
        // literally specifies ("latency p50/p95/mean на representative
        // запрос-цикл"); its I/O is deterministic-mock, so the backend
        // delta is DSL execution. The DSL-only flow measurement is
        // reported as diagnostics (net of the measured clone baseline);
        // both totals carry whole-program per-run setup that dilutes the
        // pure dispatch delta.
        let verdict = if speedup >= 2.0 {
            "GO-DATA: VM shows >= 2x latency improvement on the representative workload (§D5 first criterion) — данные за флип".to_string()
        } else if speedup >= 0.95 && rss_ratio <= 0.8 {
            "GO-DATA: equivalent latency with a significant memory win (§D5 second criterion) — данные за флип".to_string()
        } else if speedup < 0.95 {
            "NO-GO DATA: VM degrades (or does not improve) on the representative workload — флип не обоснован".to_string()
        } else {
            "INSUFFICIENT DATA: neither §D5 criterion met (latency win below 2x, no significant memory win) — флип не обоснован".to_string()
        };

        // ── Table ──────────────────────────────────────────────────
        println!();
        println!("═══ Stage 4 benchmark (naryad #381, ADR-0141 §D5) ═══");
        println!("{:<34} {:>14} {:>14}", "metric", "interpreter", "vm");
        println!(
            "{:<34} {:>12.0}us {:>12.0}us",
            "request cycle p50", i_p50, v_p50
        );
        println!(
            "{:<34} {:>12.0}us {:>12.0}us",
            "request cycle p95", i_p95, v_p95
        );
        println!(
            "{:<34} {:>12.0}us {:>12.0}us",
            "request cycle mean", i_cycle, v_cycle
        );
        println!(
            "{:<34} {:>12.1}MB {:>12.1}MB",
            "peak RSS (VmHWM)",
            i_rss / 1024.0,
            v_rss / 1024.0
        );
        println!(
            "{:<34} {:>12.1}ms {:>12.1}ms",
            "startup: parse+semantic",
            interp["startup"]["parse_semantic_ms"]
                .as_f64()
                .unwrap_or(0.0),
            vm["startup"]["parse_semantic_ms"].as_f64().unwrap_or(0.0)
        );
        println!(
            "{:<34} {:>12} {:>12.1}ms",
            "startup: vm compile (+routes)",
            "n/a",
            vm["startup"]["vm_compile_ms"].as_f64().unwrap_or(0.0)
                + vm["startup"]["vm_route_compile_ms"].as_f64().unwrap_or(0.0)
        );
        println!();
        println!("per-route p50 (µs), interpreter → vm:");
        let i_routes = interp["routes"].as_array().cloned().unwrap_or_default();
        let v_routes = vm["routes"].as_array().cloned().unwrap_or_default();
        for (ir, vr) in i_routes.iter().zip(v_routes.iter()) {
            println!(
                "  {:<24} {:>10.0} → {:>10.0}  (x{:.2})",
                format!(
                    "{} {}",
                    ir["method"].as_str().unwrap_or("?"),
                    ir["path"].as_str().unwrap_or("?")
                ),
                ir["p50_us"].as_f64().unwrap_or(0.0),
                vr["p50_us"].as_f64().unwrap_or(0.0),
                ir["p50_us"].as_f64().unwrap_or(1.0) / vr["p50_us"].as_f64().unwrap_or(1.0)
            );
        }
        println!();
        println!("latency speedup (DSL-only flow mean, interpreter/vm): x{:.2}  [diagnostic, net of clone baseline]", dsl_speedup);
        println!("latency speedup (HTTP request-cycle mean, interpreter/vm): x{:.2}  [§D5 primary reading]", speedup);
        println!(
            "DSL-only p50: {:.0}us -> {:.0}us; p95: {:.0}us -> {:.0}us",
            interp["dsl_only"]["p50_us"].as_f64().unwrap_or(0.0),
            vm["dsl_only"]["p50_us"].as_f64().unwrap_or(0.0),
            interp["dsl_only"]["p95_us"].as_f64().unwrap_or(0.0),
            vm["dsl_only"]["p95_us"].as_f64().unwrap_or(0.0)
        );
        println!("peak RSS ratio (vm/interpreter): {:.2}", rss_ratio);
        println!();
        println!("VERDICT (§D5): {}", verdict);
        println!();

        // ── Raw combined report (JSON, attached to the naryad) ─────
        let report = serde_json::json!({
            "naryad": "381",
            "issue": "467",
            "adr": "0141 §D5",
            "generated_at_utc": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            "corpus": CORPUS_REL,
            "corpus_lines": lines,
            "sanitize_hits": hits,
            "rounds": rounds,
            "environment": {
                "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
                "mocks": "call_llm / vision_understand / stt_transcribe deterministic mock; sqlite::memory:; kv in-process",
            },
            "interpreter": interp,
            "vm": vm,
            "verdict": {
                "dsl_speedup_mean": dsl_speedup,
                "http_cycle_speedup_mean": speedup,
                "rss_ratio_vm_over_interp": rss_ratio,
                "verdict": verdict,
            },
        });
        let out_dir = manifest_dir().join("target").join("bench-reports");
        let _ = std::fs::create_dir_all(&out_dir);
        let out_path = out_dir.join("stage4_benchmark_report.json");
        std::fs::write(
            &out_path,
            serde_json::to_string_pretty(&report).unwrap_or_default(),
        )
        .unwrap_or_else(|e| panic!("write report: {}", e));
        println!("raw JSON report: {}", out_path.display());
        println!("(§D5 verdict recorded; the flip decision itself is a separate naryad after the re-gate)");

        0
    }

    pub fn main() -> i32 {
        let args: Vec<String> = std::env::args().collect();
        // `cargo bench` passes --bench <name>; custom flags come from STAGE4_ARGS
        // or directly on the command line.
        let mut backend: Option<ServeBackend> = None;
        let mut rounds = DEFAULT_ROUNDS;
        let mut it = args.iter().peekable();
        let mut bench_child = false;
        while let Some(a) = it.next() {
            match a.as_str() {
                "--bench-child" => {
                    bench_child = true;
                    backend = it.next().map(|b| match b.as_str() {
                        "vm" => ServeBackend::Vm,
                        _ => ServeBackend::Interpreter,
                    });
                }
                "--rounds" => {
                    rounds = it
                        .next()
                        .and_then(|r| r.parse().ok())
                        .unwrap_or(DEFAULT_ROUNDS);
                }
                _ => {}
            }
        }
        if bench_child {
            let b = backend.expect("--bench-child <interpreter|vm>");
            let out = run_child(b, rounds);
            println!("{}", serde_json::to_string(&out).expect("serialize"));
            return 0;
        }
        run_parent(rounds)
    }
}

#[cfg(feature = "server")]
fn main() -> std::process::ExitCode {
    std::process::exit(stage4::main())
}

#[cfg(not(feature = "server"))]
fn main() {
    eprintln!("stage4_benchmark requires the `server` feature (on by default)");
    std::process::exit(1);
}

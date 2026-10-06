//! Наряд №607 (Волна 31, аудиt §3 Y-3): the WORK budget of the spectral
//! contour — a server route must not spend seconds of CPU on one call.
//!
//! Contracts under test (all pinned on BOTH backends — the handlers are
//! shared, the parity is by construction and asserted end-to-end):
//! 1. THE WORK BUDGET: the natural grid for near-uniform data is
//!    M ≈ N²/2 (the work N × M grows as N³/2 — the audit's finding: up to
//!    ~1.3·10⁸ operations per call before №607). Past the budget
//!    (10⁷ sine/cosine pairs) the grid is COARSENED over the SAME band and
//!    the Spectrum carries `degraded: true` + the explicit
//!    `degraded_reason` — loud, never a silent truncation. The load-test
//!    row (the DoD): a route computing `lomb_scargle` on N ≈ 2000
//!    request-data points FITS the budget and answers.
//! 2. The BUDGET SEAM (the №546 contour budget, the per-request/tick
//!    scope): the call charges the work-based units; exhaustion is the
//!    loud `[CONTOUR_BUDGET_EXCEEDED]` refusal of the call — fail-closed.
//! 3. The №584/№591 surfaces stay untouched: a small-N spectrum keeps the
//!    natural data-derived grid (degraded=false, no reason), the golden
//!    pins of naryad_591_spectral.rs are unchanged.
#![cfg(feature = "server")]
#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

// ── Backend harness (the №590/№591 files' proven shape) ─────────────

fn run_tw(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, base_dir.to_path_buf())
}

fn run_vm(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir.to_path_buf());
    let program = comp
        .compile(declarations)
        .map_err(|e| format!("compile error: {}", e))?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program).map_err(|e| e.to_string())
}

fn assert_parity(name: &str, source: &str) -> String {
    let base_dir = PathBuf::from("examples");
    let tw =
        run_tw(source, &base_dir).unwrap_or_else(|e| panic!("{}: TW must succeed: {}", name, e));
    let vm =
        run_vm(source, &base_dir).unwrap_or_else(|e| panic!("{}: VM must succeed: {}", name, e));
    let tw_out = tw
        .as_deref()
        .map(str::trim_end)
        .unwrap_or_default()
        .to_string();
    let vm_out = vm
        .as_deref()
        .map(str::trim_end)
        .unwrap_or_default()
        .to_string();
    assert_eq!(tw_out, vm_out, "{}: TW and VM outputs diverge", name);
    tw_out
}

fn program(body: &str) -> String {
    format!(
        "pattern T(_input: String) -> String {{\n{}\n}}\nflow Main {{ input: String = \"s\" -> T -> output }}",
        body
    )
}

// ── The N ≈ 2000 series (the audit's load-test magnitude) ───────────

/// A deterministic near-uniform series of `n` points: times = 0.0..n
/// (step 1.0), values = a 7.3-period sinusoid over unit noise-free
/// amplitude — the dominant frequency is known (1/7.3), so the test can
/// also assert the coarsened grid still RESOLVES the peak (the honest
/// degradation: fewer bins, the same band, the signal survives).
fn series(n: usize) -> (String, String) {
    let mut ts = String::with_capacity(n * 8);
    let mut vs = String::with_capacity(n * 8);
    for i in 0..n {
        if i > 0 {
            ts.push_str(", ");
            vs.push_str(", ");
        }
        ts.push_str(&format!("{}.0", i));
        let v = (i as f64 * std::f64::consts::TAU / 7.3).sin() * 10.0;
        vs.push_str(&format!("{:.4}", v));
    }
    (ts, vs)
}

const BUDGET_UNITS_PER_MILLION: usize = 1_000_000;

// ── 1: the work budget — the coarsening is loud and honest ──────────

static N_POINTS: AtomicUsize = AtomicUsize::new(2000);

#[test]
#[serial_test::serial]
fn n607_load_test_n2000_fits_the_budget_and_degrades_loudly() {
    let n = N_POINTS.load(Ordering::SeqCst);
    let (ts, vs) = series(n);
    let src = program(&format!(
        "  let ts = [{}]\n  let vs = [{}]\n  let sp = lomb_scargle(ts, vs)\n  return to_string(sp.degraded) + \",\" + to_string(sp.grid_size) + \",\" + to_string(sp.n_points) + \",\" + sp.degraded_reason",
        ts, vs
    ));
    // The DoD row: the route-equivalent computation on N ≈ 2000 FITS the
    // budget (the call completes — the run does not spend unbounded CPU)
    // and the result is LOUDLY degraded.
    let out = assert_parity("n607 load test", &src);
    let parts: Vec<&str> = out.splitn(4, ',').collect();
    assert_eq!(
        parts.len(),
        4,
        "the CSV shape: degraded,grid_size,n_points,reason"
    );
    assert_eq!(
        parts[0], "true",
        "the coarsened spectrum must carry degraded=true (loud, never silent)"
    );
    let grid_size: usize = parts[1].parse().expect("grid_size must be a number");
    let budget_grid = 10_000_000 / n; // the work budget / N
    assert!(
        grid_size <= budget_grid,
        "the coarsened grid ({}) must fit the work budget ({} bins for N={})",
        grid_size,
        budget_grid,
        n
    );
    assert!(
        grid_size >= 3,
        "the coarsened grid stays usable (>= 3 bins), got {}",
        grid_size
    );
    let reason = parts[3];
    assert!(
        reason.contains("coarsened") && reason.contains("work budget"),
        "the explicit reason must name the coarsening and the budget, got: {}",
        reason
    );
}

#[test]
#[serial_test::serial]
fn n607_coarsened_band_still_resolves_the_known_peak() {
    // The honest degradation: fewer bins over the SAME band — the known
    // 7.3-period signal still lands at the peak (within 2%, the №591 rule).
    let (ts, vs) = series(2000);
    let src = program(&format!(
        "  let ts = [{}]\n  let vs = [{}]\n  let sp = lomb_scargle(ts, vs)\n  let pk = spectral_peak(sp)\n  return to_string(pk.freq) + \",\" + to_string(pk.degraded)",
        ts, vs
    ));
    let out = assert_parity("n607 peak on the coarsened grid", &src);
    let parts: Vec<&str> = out.split(',').collect();
    assert_eq!(parts[1], "false", "the peak itself is not degraded");
    let freq: f64 = parts[0].parse().expect("the peak freq must be a number");
    let truth = 1.0 / 7.3;
    let rel = (freq - truth).abs() / truth;
    assert!(
        rel < 0.02,
        "the coarsened grid must still resolve the 7.3-period peak within 2%, got {} (rel {})",
        freq,
        rel
    );
}

#[test]
#[serial_test::serial]
fn n607_small_n_keeps_the_natural_grid() {
    // The №584/№591 surfaces stay untouched: a small series keeps the
    // data-derived grid — degraded=false, no reason (the golden pins of
    // naryad_591_spectral.rs are unchanged by №607).
    let src = program(
        "  let ts = [0.0, 0.7, 1.3, 1.9, 2.8, 3.4, 4.1, 4.9, 5.5, 6.3, 7.1, 7.8, 8.4, 9.2, 9.9, 10.6, 11.4, 12.2, 12.9, 13.6, 14.3, 15.1, 15.8, 16.6]\n  let vs = [1.0, 2.0, -1.0, 3.0, 0.5, -2.0, 2.5, 1.0, -0.5, 2.0, -1.5, 0.0, 1.5, 2.2, -0.8, 3.1, -2.2, 0.3, 1.1, -1.7, 2.8, 0.9, -0.2, 1.4]\n  let sp = lomb_scargle(ts, vs)\n  return to_string(sp.degraded) + \",\" + to_string(sp.grid_size) + \",\" + to_string(sp.degraded_reason == \"\")",
    );
    let out = assert_parity("n607 small-N natural grid", &src);
    let parts: Vec<&str> = out.split(',').collect();
    assert_eq!(
        parts[0], "false",
        "a small series keeps the natural grid — no degradation"
    );
    assert_eq!(
        parts[2], "true",
        "the natural-grid spectrum carries an EMPTY degraded_reason"
    );
}

// ── 2: the contour budget seam — the loud refusal ───────────────────

struct EnvGuard(&'static str);
impl EnvGuard {
    fn set(var: &'static str, val: &str) -> Self {
        std::env::set_var(var, val);
        EnvGuard(var)
    }
}
impl Drop for EnvGuard {
    fn drop(&mut self) {
        std::env::remove_var(self.0);
    }
}

fn fresh_scope() {
    // The №546 shape: the RAII scope resets the thread-local counter at
    // the request/tick boundary; the tests call it between runs.
    let _ = metalogos::builtins::embed_seam::ContourBudgetScope::new();
}

#[test]
#[serial_test::serial]
fn n607_budget_exhaustion_refuses_the_call_loudly_on_tw_and_vm() {
    let _budget = EnvGuard::set("METALOGOS_CONTOUR_BUDGET", "1");
    fresh_scope();

    // One N=2000 call costs 10 units (10⁷ work / 10⁶ per unit) against a
    // 1-unit budget: the seam refuses BEFORE the compute, loudly.
    let (ts, vs) = series(2000);
    let src = program(&format!(
        "  let ts = [{}]\n  let vs = [{}]\n  let sp = lomb_scargle(ts, vs)\n  return to_string(sp.grid_size)",
        ts, vs
    ));
    let base_dir = PathBuf::from("examples");
    let tw_err = run_tw(&src, &base_dir).expect_err("TW: the budget must refuse the call");
    assert!(
        tw_err.contains("[CONTOUR_BUDGET_EXCEEDED]"),
        "TW: the loud budget refusal expected, got: {}",
        tw_err
    );
    let vm_err = run_vm(&src, &base_dir).expect_err("VM: the budget must refuse the call");
    assert!(
        vm_err.contains("[CONTOUR_BUDGET_EXCEEDED]"),
        "VM: the loud budget refusal expected, got: {}",
        vm_err
    );
}

#[test]
#[serial_test::serial]
fn n607_budget_within_the_limit_stays_green() {
    let _budget = EnvGuard::set("METALOGOS_CONTOUR_BUDGET", "100");
    fresh_scope();

    // The same N=2000 call against a 100-unit budget: the call happens,
    // the degraded (coarsened) spectrum returns — the budget caps the
    // COUNT of heavy calls per request, not their honesty.
    let (ts, vs) = series(2000);
    let src = program(&format!(
        "  let ts = [{}]\n  let vs = [{}]\n  let sp = lomb_scargle(ts, vs)\n  return to_string(sp.degraded)",
        ts, vs
    ));
    assert_parity("n607 within the budget", &src);
}

// ── 3: the route e2e — a serve route with the heavy call (both backends) ──

#[tokio::test]
#[serial_test::serial]
async fn n607_route_with_lomb_scargle_serves_degraded_on_both_backends() {
    let (ts, vs) = series(2000);
    let src = format!(
        r#"
mlogserver {{
  port: 8094
  route "/spectrum" method=GET {{
    let ts = [{ts}]
    let vs = [{vs}]
    let sp = lomb_scargle(ts, vs)
    respond("200", to_string(sp.degraded) + "," + to_string(sp.grid_size))
  }}
}}
"#,
        ts = ts,
        vs = vs
    );
    for (backend_name, backend) in [
        ("VM", metalogos_server::server::ServeBackend::Vm),
        ("TW", metalogos_server::server::ServeBackend::Interpreter),
    ] {
        let (port, _handle) = metalogos_server::server::run_test_server_with_backend(&src, backend)
            .await
            .expect("the route server should start");
        let url = format!("http://127.0.0.1:{}/spectrum", port);
        let resp = reqwest::get(&url).await.expect("GET should succeed");
        let status = resp.status().as_u16();
        let body = resp.text().await.expect("the body should be readable");
        assert_eq!(
            status, 200,
            "{}: the route answers with the degraded spectrum",
            backend_name
        );
        let degraded = body.split(',').next().unwrap_or("");
        assert_eq!(
            degraded, "true",
            "{}: the route-level result is loudly degraded",
            backend_name
        );
    }
}

// The budget-units arithmetic pin: the 10⁷ work budget over N=2000 gives
// 5000 bins — the coarsened call costs exactly 10 units (10⁷ / 10⁶).
#[test]
#[serial_test::serial]
fn n607_units_arithmetic_pin() {
    let n = 2000usize;
    let budget_grid = 10_000_000 / n;
    assert_eq!(budget_grid, 5000);
    let units = (n * budget_grid) / BUDGET_UNITS_PER_MILLION;
    assert_eq!(units, 10, "the load-test call costs 10 budget units");
}

//! Наряд №591 (Волна 30, Камертон Н1-05/Н1-07, инвариант 3): the spectral
//! contour — `lomb_scargle(times, values) -> Spectrum` and
//! `spectral_peak(spectrum) -> SpectralPeak`.
//!
//! Contracts under test (all pinned on BOTH backends):
//! 1. GOLDEN PIN (the DoD row «второй независимый метод»): the fixed
//!    irregular 24-point dataset below is computed by
//!    `scripts/n591_golden.py` — a plain-Python implementation written FROM
//!    THE CONTRACT (no Metalogos code involved) and cross-checked at
//!    authoring time against scipy.signal.lombscargle (a THIRD
//!    implementation: after the exact (N−1)/2 normalization conversion the
//!    max relative deviation was 1.3e−13, argmax identical — see the
//!    script's printed verdict). The pinned subset (grid/baseline, the peak
//!    triple, 13 sampled powers) must reproduce with rel tolerance 1e-9 —
//!    any constant or algorithm mutation in src (2π → 6.28, the τ offset
//!    dropped, the ddof flipped, the grid mis-derived) goes RED.
//! 2. The DoD row «пик в пределах 2% от истины» at the LANGUAGE level: the
//!    golden dataset's dominant period is 6.1 (freq 1/6.1); the measured
//!    peak freq must land within 2% — through the interpreter, not just the
//!    direct-handler unit tests in src/builtins/spectral.rs.
//! 3. TW↔VM parity: both backends share the handlers, so the outputs are
//!    byte-identical — asserted on every program below (the №372/ADR-0141
//!    posture, extended to the spectral contour).
//! 4. Degraded-loud (§16.0-D): too-few-points input returns the typed
//!    `Degraded` struct (degraded=true) and `spectral_peak` PROPAGATES it —
//!    never a quiet result, never a synthesized peak.
//! 5. The loud [SPECTRAL_INPUT] domain gate: a length mismatch refuses with
//!    the position-0 origin stamp on both backends, and `try{}` classifies
//!    it to the typed code SPECTRAL_INPUT (№385/ADR-0169).
//!
//! The statistical DoD rows (the gapped-sinusoid 10-seed 2% sweep and the
//! white-noise false-positive band on 100 seeds) live as deterministic
//! unit tests in src/builtins/spectral.rs — direct handler access keeps
//! them in milliseconds.

use std::path::{Path, PathBuf};

// ── Backend harness (the №590 file's proven shape) ───────────────────

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

// ── The fixed golden dataset (scripts/n591_golden.py) ────────────────

const GOLDEN_TIMES: &str = "0.0, 0.7, 1.3, 1.9, 2.8, 3.4, 4.1, 4.9, 5.5, 6.3, 7.1, 7.8, 8.4, 9.2, 9.9, 10.6, 11.4, 12.2, 12.9, 13.6, 14.3, 15.1, 15.8, 16.6";
const GOLDEN_VALUES: &str = "0.5, 0.631079706215994, 0.47664519482941103, 0.7829223518959496, 0.7411935555311409, -0.3818245010043322, -1.3792979771965186, -0.7464493457640825, -0.0928986629146516, -0.045447933873798696, 0.5141938091419642, 1.3668201731330043, 1.0809663763258326, -0.46922266047681177, -0.9479441547663447, -0.5502012062868219, -0.6470612775987332, -0.49661917887097234, 0.7469762095046965, 1.4782451094260138, 0.6862753971970732, -0.2638533290024082, -0.23811689814020798, -0.6852186557221218";

/// The golden program: compute the spectrum, take the peak, render a CSV
/// of the pinned numbers (the powers are indexed out into a local first —
/// the field-then-index chain stays inside one statement shape).
fn golden_program() -> String {
    program(&format!(
        "  let ts = [{}]\n  let vs = [{}]\n  let sp = lomb_scargle(ts, vs)\n  let pk = spectral_peak(sp)\n  let pw = sp.powers\n  return to_string(sp.degraded) + \",\" + to_string(pk.degraded) + \",\" + to_string(sp.grid_size) + \",\" + to_string(sp.baseline) + \",\" + to_string(pk.freq) + \",\" + to_string(pk.power_fraction) + \",\" + to_string(pk.p_value) + \",\" + to_string(pw[0]) + \",\" + to_string(pw[25]) + \",\" + to_string(pw[41]) + \",\" + to_string(pw[50]) + \",\" + to_string(pw[75]) + \",\" + to_string(pw[100]) + \",\" + to_string(pw[125]) + \",\" + to_string(pw[150]) + \",\" + to_string(pw[175]) + \",\" + to_string(pw[200]) + \",\" + to_string(pw[225]) + \",\" + to_string(pw[250]) + \",\" + to_string(pw[260])",
        GOLDEN_TIMES, GOLDEN_VALUES
    ))
}

/// The pinned golden numbers (scripts/n591_golden.py output, full f64).
/// Layout: (degraded_spectrum, degraded_peak) then grid_size, baseline,
/// freq, power_fraction, p_value, then powers at [0, 25, 41, 50, 75, 100,
/// 125, 150, 175, 200, 225, 250, 260].
const GOLDEN: [f64; 20] = [
    0.0, // sp.degraded = false
    0.0, // pk.degraded = false
    261.0,
    16.6,
    0.16315261044176707,
    0.7505684238091509,
    0.034113189136006694,
    0.21359036647063037,  // pw[0]
    1.6882372652362645,   // pw[25]
    8.925353972022611,    // pw[41] — the peak
    5.716646702650531,    // pw[50]
    0.22283148140226902,  // pw[75]
    0.3025003355506484,   // pw[100]
    1.5469834318106606,   // pw[125]
    0.027973139307631196, // pw[150]
    0.024751706971109882, // pw[175]
    0.11949253482793269,  // pw[200]
    0.15643601613993033,  // pw[225]
    0.3102555988680316,   // pw[250]
    0.12280908146328813,  // pw[260]
];

fn assert_close(got: f64, want: f64, what: &str) {
    if want == 0.0 {
        assert_eq!(got, want, "{}: got {}, want 0", what, got);
        return;
    }
    let rel = ((got - want) / want).abs();
    assert!(
        rel <= 1e-9,
        "{}: got {}, want {} (rel err {:.3e} > 1e-9)",
        what,
        got,
        want,
        rel
    );
}

// ── 1 + 2: the golden pin and the 2% language-level peak row ────────

#[test]
fn n591_golden_reference_pinned_on_both_backends() {
    let out = assert_parity("n591_golden", &golden_program());
    let pieces: Vec<&str> = out.split(',').map(str::trim).collect();
    assert_eq!(pieces.len(), GOLDEN.len(), "the CSV layout drifted");
    assert_eq!(
        pieces[0], "false",
        "the golden spectrum must NOT be degraded"
    );
    assert_eq!(pieces[1], "false", "the golden peak must NOT be degraded");
    let got: Vec<f64> = pieces[2..]
        .iter()
        .enumerate()
        .map(|(i, piece)| {
            piece.parse::<f64>().unwrap_or_else(|e| {
                panic!("n591_golden: piece {} {:?} must parse: {}", i + 2, piece, e)
            })
        })
        .collect();
    for (i, (g, w)) in got.iter().zip(GOLDEN[2..].iter()).enumerate() {
        assert_close(*g, *w, &format!("golden[{}]", i + 2));
    }
    // The DoD row at the language level: the dominant period of the golden
    // dataset is 6.1 → freq 1/6.1; the measured peak must sit within 2%.
    let true_freq = 1.0 / 6.1;
    let rel = (got[2] - true_freq).abs() / true_freq;
    assert!(
        rel <= 0.02,
        "the peak must land within 2% of the true frequency: got {} vs {} (rel {:.4})",
        got[2],
        true_freq,
        rel
    );
    // The peak bin is the strongest of the sampled pins (the peak index 41
    // holds the maximum power) — a structural guard against an argmax drift.
    assert!(
        got[7] > got[5] && got[7] > got[6] && got[7] > got[8] && got[7] > got[17],
        "pw[41] must dominate the sampled neighbors"
    );
}

// ── 4: degraded-loud through the language ────────────────────────────

#[test]
fn n591_degraded_spectrum_and_peak_propagation_language_level() {
    // 11 points — below the SPECTRAL_MIN_POINTS floor: the Spectrum is
    // Degraded, and spectral_peak propagates the SAME Degraded (never a
    // synthesized peak, never a quiet zero).
    let src = program(
        "  let ts = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]\n  let vs = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0]\n  let sp = lomb_scargle(ts, vs)\n  let pk = spectral_peak(sp)\n  return to_string(sp.degraded) + \"|\" + to_string(pk.degraded) + \"|\" + pk.reason",
    );
    let out = assert_parity("n591_degraded", &src);
    let parts: Vec<&str> = out.split('|').collect();
    assert_eq!(
        parts.len(),
        3,
        "layout: degraded|degraded|reason, got {}",
        out
    );
    assert_eq!(parts[0], "true", "the spectrum must be degraded=true");
    assert_eq!(
        parts[1], "true",
        "the propagated peak must be degraded=true"
    );
    assert!(
        parts[2].contains("insufficient points"),
        "the reason must name the shortfall, got: {}",
        parts[2]
    );
}

// ── 5: the loud [SPECTRAL_INPUT] gate ────────────────────────────────

#[test]
fn n591_length_mismatch_loud_on_both_backends() {
    let src = program(
        "  let ts = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0]\n  let vs = [1.0, 2.0, 3.0]\n  return to_string(lomb_scargle(ts, vs))",
    );
    let base_dir = PathBuf::from("examples");
    for (backend, res) in [
        ("TW", run_tw(&src, &base_dir)),
        ("VM", run_vm(&src, &base_dir)),
    ] {
        let err = res.expect_err("a length mismatch must be a loud refusal");
        assert!(
            err.starts_with("[SPECTRAL_INPUT] "),
            "{}: the refusal must carry the origin stamp at position 0, got: {}",
            backend,
            err
        );
    }
}

#[test]
fn n591_non_numeric_element_loud_on_both_backends() {
    // A non-numeric element is refused at the gate with the typed stamp.
    // (The language cannot construct NaN/Inf by ordinary arithmetic — the
    // interpreter guards 0/0 and friends — so the non-finite arms of the
    // gate are covered by the direct-handler unit tests in
    // src/builtins/spectral.rs; this is the language-reachable arm.)
    let src = program(
        "  let ts = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0]\n  let vs = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, \"x\"]\n  return to_string(lomb_scargle(ts, vs))",
    );
    let base_dir = PathBuf::from("examples");
    for (backend, res) in [
        ("TW", run_tw(&src, &base_dir)),
        ("VM", run_vm(&src, &base_dir)),
    ] {
        let err = res.expect_err("a non-numeric element must be a loud refusal");
        assert!(
            err.starts_with("[SPECTRAL_INPUT] "),
            "{}: got: {}",
            backend,
            err
        );
    }
}

#[test]
fn n591_try_classification_typed_code_both_backends() {
    // `try` classifies the stamped refusal to the typed code on BOTH
    // backends (the №385 classifier reads the position-0 stamp).
    let src = r#"
pattern Probe(x: String) -> String {
  let ts = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0]
  let vs = [1.0, 2.0, 3.0]
  let r = try lomb_scargle(ts, vs)
  return r.error.code
}
flow Main { input: String = "x" -> Probe -> output }
"#;
    let base_dir = PathBuf::from("examples");
    for (backend, res) in [
        ("TW", run_tw(src, &base_dir)),
        ("VM", run_vm(src, &base_dir)),
    ] {
        let out = res
            .unwrap_or_else(|e| panic!("{}: try must capture the refusal: {}", backend, e))
            .unwrap_or_else(|| panic!("{}: the pattern must return", backend));
        assert_eq!(
            out.trim(),
            "SPECTRAL_INPUT",
            "{}: typed code mismatch",
            backend
        );
    }
}

#[test]
fn n591_spectral_peak_refuses_foreign_input_on_both_backends() {
    // A non-Spectrum argument is a typed loud refusal, not a silent guess.
    let src = program("  return to_string(spectral_peak(42.0))");
    let base_dir = PathBuf::from("examples");
    for (backend, res) in [
        ("TW", run_tw(&src, &base_dir)),
        ("VM", run_vm(&src, &base_dir)),
    ] {
        let err = res.expect_err("a foreign input must be refused");
        assert!(
            err.starts_with("[SPECTRAL_INPUT] "),
            "{}: got: {}",
            backend,
            err
        );
    }
}

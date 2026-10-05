//! Наряд №590 (Волна 30, Камертон Н1-05): `normal_sample(mean, stddev)` —
//! the Box–Muller transform over the SHARED deterministic PRNG.
//!
//! Contracts under test (all pinned on BOTH backends):
//! 1. Fixed seed → deterministic sequence; TW and VM read the same shared
//!    handler and thread-local xorshift64 state, so the sequences are
//!    byte-identical (the №372/ADR-0141 Stage 1.4 posture, extended to the
//!    normal sampler).
//! 2. MUTATION PIN: `GoldenReplication` below is an INDEPENDENT second
//!    implementation of the exact contracted state machine
//!    (seed_to_state → xorshift64 → u64_to_float → Box–Muller with
//!    u1 = 1−u ∈ (0,1], u2 ∈ [0,1), TAU), living entirely in this test
//!    file. The builtin must match it bit-exactly (f64::to_bits) — any
//!    constant mutation in src (2π → 6.0, −2 → −1, the 1−u domain mapping,
//!    the seed mapping, the draw order) goes RED. In-process replication
//!    keeps the pin portable (no cross-libm brittleness); the machine
//!    cross-check against a third implementation — the Python replication
//!    scripts/n590_golden.py — was run at authoring time and matched the
//!    printed sequences to full precision.
//! 3. Loud domain gate: stddev <= 0 refuses on both backends with the
//!    stable [NORMAL_SAMPLE_STDDEV] origin stamp at position 0, and `try`
//!    classifies it to the typed code NORMAL_SAMPLE_STDDEV (№385/ADR-0169)
//!    — never a NaN result.
//! 4. Mixed stream: random() and normal_sample() consume ONE stream — the
//!    interleaving is pinned bit-exact on both backends.
//! 5. Statistical shape (the 10⁶ DoD row) lives as a unit test in
//!    src/builtins/math.rs — direct handler access, no interpreter
//!    overhead; deterministic seed, so the bounds are stable, not flaky.

use std::path::{Path, PathBuf};

// ── The independent replication (the mutation tripwire) ─────────────

fn seed_to_state(seed: f64) -> u64 {
    let bits = seed.to_bits();
    let state = bits ^ 0x9E3779B97F4A7C15;
    if state == 0 {
        0x9E3779B97F4A7C15
    } else {
        state
    }
}

fn xorshift64(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn u64_to_float(bits: u64) -> f64 {
    let mantissa = bits >> 11;
    (mantissa as f64) / ((1u64 << 53) as f64)
}

/// One normal draw per the №590 contract: u1 = 1−u ∈ (0,1], u2 = u ∈ [0,1),
/// z = sqrt(−2·ln u1)·cos(2π·u2), result = mean + stddev·z.
/// Deliberately written from the CONTRACT (issue #1015), not from the src.
struct GoldenReplication {
    state: u64,
}

impl GoldenReplication {
    fn seeded(seed: f64) -> Self {
        Self {
            state: seed_to_state(seed),
        }
    }

    fn next_uniform(&mut self) -> f64 {
        u64_to_float(xorshift64(&mut self.state))
    }

    fn normal(&mut self, mean: f64, stddev: f64) -> f64 {
        let u1 = 1.0 - self.next_uniform();
        let u2 = self.next_uniform();
        let z = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
        mean + stddev * z
    }
}

// ── Backend harness ──────────────────────────────────────────────────

/// Execute via tree-walking interpreter.
fn run_tw(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, base_dir.to_path_buf())
}

/// Execute via bytecode VM.
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

fn five_draws_program() -> String {
    program(
        "  random_seed(42.0)\n  let mut s = \"\"\n  s = s + to_string(normal_sample(0.0, 1.0)) + \",\"\n  s = s + to_string(normal_sample(0.0, 1.0)) + \",\"\n  s = s + to_string(normal_sample(0.0, 1.0)) + \",\"\n  s = s + to_string(normal_sample(0.0, 1.0)) + \",\"\n  s = s + to_string(normal_sample(0.0, 1.0))\n  return s",
    )
}

/// Parse a comma-separated to_string(Float) render back to exact bits.
/// `to_string` prints via Rust's shortest round-trip Display, so parsing
/// recovers the exact f64 — formatting cannot mask a mutation.
fn parse_bits(csv: &str, name: &str) -> Vec<u64> {
    csv.split(',')
        .map(|piece| {
            piece
                .trim()
                .parse::<f64>()
                .unwrap_or_else(|e| panic!("{}: piece {:?} must parse as f64: {}", name, piece, e))
        })
        .map(|f| f.to_bits())
        .collect()
}

// ── 1 + 2: parity and the bit-exact mutation pin ────────────────────

#[test]
fn n590_fixed_seed_parity_both_backends() {
    let (tw, _vm) = {
        let base_dir = PathBuf::from("examples");
        let tw = run_tw(&five_draws_program(), &base_dir).expect("TW run");
        let vm = run_vm(&five_draws_program(), &base_dir).expect("VM run");
        assert_eq!(
            tw.as_deref().map(str::trim_end),
            vm.as_deref().map(str::trim_end)
        );
        (tw, vm)
    };
    assert_eq!(
        tw.as_deref().unwrap_or_default().split(',').count(),
        5,
        "five draws must be rendered"
    );
}

#[test]
fn n590_mutation_pin_bit_exact_seed42() {
    let tw = assert_parity("n590_pin", &five_draws_program());
    let got = parse_bits(&tw, "n590_pin");
    let mut golden = GoldenReplication::seeded(42.0);
    for (i, g) in got.iter().enumerate().take(5) {
        let expected = golden.normal(0.0, 1.0).to_bits();
        assert_eq!(
            g, &expected,
            "n590_pin: draw {} diverges from the independent replication",
            i
        );
    }
}

#[test]
fn n590_mutation_pin_bit_exact_mixed_stream() {
    // seed 7.0 → random() then normal_sample(50, 2): ONE stream, pinned.
    let src = program(
        "  random_seed(7.0)\n  let r = random()\n  return to_string(r) + \",\" + to_string(normal_sample(50.0, 2.0))",
    );
    let tw = assert_parity("n590_mixed", &src);
    let got = parse_bits(&tw, "n590_mixed");
    let mut golden = GoldenReplication::seeded(7.0);
    let expected_r = golden.next_uniform().to_bits();
    let expected_n = golden.normal(50.0, 2.0).to_bits();
    assert_eq!(got.len(), 2);
    assert_eq!(got[0], expected_r, "random() draw diverges");
    assert_eq!(got[1], expected_n, "normal_sample draw diverges");
}

#[test]
fn n590_shift_scale_contract() {
    // mean/stddev are honored: the builtin's output must equal the affine
    // projection of the SAME golden standard draws — 100 + 15·z_i, bit-exact.
    let src = program(
        "  random_seed(42.0)\n  let mut s = \"\"\n  s = s + to_string(normal_sample(100.0, 15.0)) + \",\"\n  s = s + to_string(normal_sample(100.0, 15.0))\n  return s",
    );
    let tw = assert_parity("n590_affine", &src);
    let got = parse_bits(&tw, "n590_affine");
    let mut golden = GoldenReplication::seeded(42.0);
    let z0 = golden.normal(0.0, 1.0);
    let z1 = golden.normal(0.0, 1.0);
    assert_eq!(got[0], (100.0 + 15.0 * z0).to_bits());
    assert_eq!(got[1], (100.0 + 15.0 * z1).to_bits());
}

// ── 3: the loud domain gate ──────────────────────────────────────────

#[test]
fn n590_stddev_zero_loud_on_both_backends() {
    let src = program("  return to_string(normal_sample(0.0, 0.0))");
    let base_dir = PathBuf::from("examples");
    for (backend, res) in [
        ("TW", run_tw(&src, &base_dir)),
        ("VM", run_vm(&src, &base_dir)),
    ] {
        let err = res.expect_err("stddev = 0 must be a loud refusal");
        assert!(
            err.starts_with("[NORMAL_SAMPLE_STDDEV] "),
            "{}: the refusal must carry the origin stamp at position 0, got: {}",
            backend,
            err
        );
    }
}

#[test]
fn n590_stddev_negative_loud_on_both_backends() {
    let src = program("  return to_string(normal_sample(10.0, -3.0))");
    let base_dir = PathBuf::from("examples");
    for (backend, res) in [
        ("TW", run_tw(&src, &base_dir)),
        ("VM", run_vm(&src, &base_dir)),
    ] {
        let err = res.expect_err("stddev < 0 must be a loud refusal");
        assert!(
            err.starts_with("[NORMAL_SAMPLE_STDDEV] "),
            "{}: the refusal must carry the origin stamp at position 0, got: {}",
            backend,
            err
        );
    }
}

#[test]
fn n590_try_classification_typed_code_both_backends() {
    // `try` classifies the stamped refusal to the typed code on BOTH
    // backends (the №385 classifier reads the position-0 stamp).
    let src = r#"
pattern Probe(x: String) -> String {
  let r = try normal_sample(0.0, 0.0)
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
            .expect("{}: the pattern must return");
        assert_eq!(
            out.trim(),
            "NORMAL_SAMPLE_STDDEV",
            "{}: typed code mismatch",
            backend
        );
    }
}

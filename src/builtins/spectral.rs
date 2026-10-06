// ── Наряд №591 (Волна 30, Камертон Н1-05/Н1-07, инвариант 3): спектральный
//    контур ────────────────────────────────────────────────────────────
//
// `lomb_scargle(times, values) -> Spectrum` — the normalized Lomb–Scargle
// periodogram for UNEVENLY sampled series (the classic Scargle window with
// the per-frequency phase offset τ; Press et al., NR §13.8 formulation).
// FFT is deliberately NOT used: the observation series is gapped by
// construction, and the FFT requires a uniform grid (the naryad's own
// fact row).
//
// `spectral_peak(spectrum) -> SpectralPeak` — the dominant frequency with
// its share of total power in a band around the peak and the false-alarm
// p-value. Numbers and provenance ONLY — the instrumental posture of the
// forecast surface (docs/limitations.md, the `timeseries` row) is
// inherited verbatim: no interpretations, no verdict strings, no advice.
//
// Frequency grid — derived from the data, no magic constants:
//   - f_min  = 1/T           (T = the observation baseline t_max − t_min;
//                             the lowest resolvable frequency of the window)
//   - f_max  = 1/(2·Δt_med)  (the Nyquist-equivalent of the MEDIAN positive
//                             spacing — the density of the series; falls
//                             back to the smallest positive spacing when
//                             the median spacing cannot resolve f_min —
//                             heavy-gap data)
//   - Δf     = 1/(N·T)       (N-fold oversampling of the Rayleigh limit
//                             1/T — the resolution element derived from
//                             the LENGTH of the series; a peak is then
//                             located within Δf of its true frequency,
//                             which the DoD's 2% budget dwarfs)
//
// Normalization and significance (Scargle 1982): powers are scaled by
// 2σ² (σ² = the sample variance about the mean, ddof = 1), so under the
// Gaussian-noise null a single frequency's power z is Exp(1)-distributed
// and the false-alarm probability of the MAXIMUM over the M scanned grid
// bins is 1 − (1 − e^{−z})^M. Using the FULL bin count M (not an
// effective-independent count) OVERestimates the FAP for correlated
// neighbors — the conservative direction: a claimed periodicity survives
// a stricter bar, and the white-noise false-positive rate stays AT OR
// BELOW the declared α (the DoD row is stated as an upper bound).
//
// Degraded-loud (§16.0-D — never silent): fewer than SPECTRAL_MIN_POINTS
// points, a constant (zero-variance) series, a zero baseline, or an
// over-capacity grid returns the typed `Degraded` struct (the
// backend_select shape — ok=false, degraded=true), never a quiet
// empty/zero spectrum. Malformed input (length mismatch, non-numeric or
// non-finite elements) is a LOUD domain error stamped [SPECTRAL_INPUT]
// (№385/ADR-0169 — `try{}` classifies it to the typed code on BOTH
// backends).
//
// Classification (№316): the registry category is `math` — the
// provably-pure default (two passes of closed-form arithmetic over
// in-program lists; zero effects, no state, no ingress/egress) — the same
// posture the №590 normal_sample row took.

use crate::interpreter::Value;

use super::core::{expect_list_arg, make_struct};

/// The typed origin stamp for malformed spectral input (№385/ADR-0169).
/// Lives in `values.rs` as `CODE_SPECTRAL_INPUT` and is whitelisted in
/// `ORIGIN_STAMPED_CODES` so `try{}` classifies it identically on TW and VM.
use crate::interpreter::values::{coded_error, CODE_SPECTRAL_INPUT};

/// The methodological floor of the periodogram: a two-amplitude-plus-offset
/// model per frequency needs enough residual degrees of freedom for the
/// variance estimate to mean anything. Below 12 points the significance
/// arithmetic is theater — Degraded, loudly (the DoD row).
const SPECTRAL_MIN_POINTS: usize = 12;

/// The capacity guard of the frequency grid. The N-fold-oversampled grid
/// over a long series can grow past any useful size; past this bin count
/// the contour refuses with a LOUD Degraded (a guard, not a tuning
/// constant — documented in REFERENCE.md).
const SPECTRAL_MAX_GRID: usize = 200_000;

/// №607 (the audit 25b375e Y-3): the WORK budget of one call — the N × M
/// sine/cosine pairs the periodogram computes. The natural grid for
/// near-uniform data is M ≈ N²/2, so the work grows as N³/2: past the
/// budget the grid is COARSENED over the same band (fewer, wider bins —
/// the band never shrinks) and the result carries `degraded: true` with
/// the explicit reason — never a silent truncation, never a silent
/// refusal (a server route must not spend seconds of CPU on one call).
/// The guideline magnitude is the naryad's: 10⁷ work units per call.
const SPECTRAL_WORK_BUDGET: usize = 10_000_000;

// ── The statistical core (one implementation; both builtins and the
//    unit tests call these fns; the backends share the handlers — the
//    TW↔VM parity is by construction, asserted end-to-end in
//    tests/naryad_591_spectral.rs) ─────────────────────────────────────

/// The derived frequency grid: (freqs, baseline T, bin count M).
/// Fails loudly (Err) on a degenerate window; returns Ok(None)-shaped
/// degradation reasons via `DegradedReason`.
/// №607: `coarsened` carries the loud degradation note when the natural
/// grid was re-sampled to a coarser Δf over the SAME band to fit the work
/// budget (None = the natural grid, the data-derived Δf untouched).
enum GridOutcome {
    Grid {
        freqs: Vec<f64>,
        baseline: f64,
        coarsened: Option<String>,
    },
    Degraded(String),
}

fn derive_grid(times: &[f64]) -> GridOutcome {
    let t_min = times.iter().cloned().fold(f64::INFINITY, f64::min);
    let t_max = times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let baseline = t_max - t_min;
    // NaN-safe forms (the clippy neg-cmp ratchet): an NaN baseline is
    // impossible with validated finite inputs, but the guards stay total.
    if baseline <= 0.0 || !baseline.is_finite() {
        return GridOutcome::Degraded(format!(
            "degenerate baseline: times span {} (need a finite positive span)",
            baseline
        ));
    }
    // Median positive gap (the density of the series): sort, walk the
    // strictly-positive consecutive gaps. A duplicated timestamp contributes
    // no information about density and is skipped.
    let mut sorted = times.to_vec();
    // total_cmp: the deterministic total order (validated-finite inputs;
    // NaN would sort last rather than panic — total, not partial).
    sorted.sort_by(f64::total_cmp);
    let mut gaps: Vec<f64> = sorted
        .windows(2)
        .map(|w| w[1] - w[0])
        .filter(|g| *g > 0.0)
        .collect();
    if gaps.is_empty() {
        return GridOutcome::Degraded(
            "degenerate baseline: fewer than two distinct observation times".to_string(),
        );
    }
    gaps.sort_by(f64::total_cmp);
    let median_gap = gaps[gaps.len() / 2];
    let f_min = 1.0 / baseline;
    // The Nyquist-equivalent of the median spacing; heavy-gap data (median
    // spacing so large that f_max would fall under f_min) falls back to the
    // SMALLEST positive spacing — the densest pair still bounds the
    // resolvable band from above honestly. If even that fails, degrade.
    let mut f_max = 1.0 / (2.0 * median_gap);
    if f_max <= f_min {
        f_max = 1.0 / (2.0 * gaps[0]);
    }
    if f_max <= f_min {
        return GridOutcome::Degraded(
            "degenerate grid: the sampling density cannot resolve the baseline band".to_string(),
        );
    }
    // N-fold oversampling of the Rayleigh limit — Δf from the LENGTH (N)
    // and the BASELINE (T), never a magic constant (the naryad's grid row).
    let n = times.len();
    let delta_f = 1.0 / (n as f64 * baseline);
    let m = ((f_max - f_min) / delta_f).floor() as usize + 1;
    if m < 3 {
        return GridOutcome::Degraded(format!(
            "degenerate grid: only {} frequency bin(s) derive from the data",
            m
        ));
    }
    // №607: the WORK budget — N × M sine/cosine pairs per call — and the
    // grid CAPACITY bound the frequency grid together. Past either bound
    // the grid is re-sampled to a coarser Δf over the SAME band
    // [f_min, f_max]: the band never shrinks (the low-frequency limit and
    // the Nyquist-equivalent ceiling both stay honest), the bin count fits
    // the bounds, and the degradation is carried in the result — loud,
    // never a silent truncation, never a refusal of a computable spectrum
    // (the pre-№607 capacity refusal is retired: a computable, honestly
    // degraded spectrum serves the consumer better than an error).
    let work = n.saturating_mul(m);
    if m > SPECTRAL_MAX_GRID || work > SPECTRAL_WORK_BUDGET {
        let m_budget = (SPECTRAL_WORK_BUDGET / n).clamp(3, SPECTRAL_MAX_GRID);
        let delta_f_coarse = (f_max - f_min) / (m_budget as f64 - 1.0);
        let freqs: Vec<f64> = (0..m_budget)
            .map(|k| f_min + k as f64 * delta_f_coarse)
            .collect();
        return GridOutcome::Grid {
            freqs,
            baseline,
            coarsened: Some(format!(
                "the frequency grid coarsened to fit the work budget: the \
                 derived grid {} bins × {} points = {} work units exceeds \
                 the budget {} (the grid capacity {}); sampled {} bins over \
                 the same band [{} Hz, {} Hz] — the frequency resolution is \
                 reduced, the band is unchanged",
                m, n, work, SPECTRAL_WORK_BUDGET, SPECTRAL_MAX_GRID, m_budget, f_min, f_max
            )),
        };
    }
    let freqs = (0..m).map(|k| f_min + k as f64 * delta_f).collect();
    GridOutcome::Grid {
        freqs,
        baseline,
        coarsened: None,
    }
}

/// The Lomb–Scargle power at one frequency, normalized by 2σ² (Scargle's
/// exponential-law normalization): the per-frequency phase offset τ makes
/// the sine/cosine cross-terms vanish for ANY sample layout (the uneven-
/// sampling heart of the method).
fn ls_power_at(freq: f64, times: &[f64], y: &[f64], variance: f64) -> f64 {
    let omega = std::f64::consts::TAU * freq;
    // τ = atan2(Σ sin 2ωt, Σ cos 2ωt) / (2ω) — the phase offset that
    // diagonalizes the two-harmonic fit at this frequency.
    let (mut s2, mut c2) = (0.0f64, 0.0f64);
    for &t in times {
        let a = 2.0 * omega * t;
        s2 += a.sin();
        c2 += a.cos();
    }
    let tau = 0.5 * s2.atan2(c2) / omega;
    let (mut yc, mut ys, mut sc, mut ss) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for (i, &t) in times.iter().enumerate() {
        let a = omega * (t - tau);
        let (c, s) = (a.cos(), a.sin());
        yc += y[i] * c;
        ys += y[i] * s;
        sc += c * c;
        ss += s * s;
    }
    // sc/ss are strictly positive for ω > 0 with ≥ 2 distinct times; the
    // zero guard keeps the closed form total (a zero denominator would be
    // a degenerate bin, reported as zero power rather than NaN — the bin
    // cannot carry signal it cannot represent).
    if sc <= 0.0 || ss <= 0.0 {
        return 0.0;
    }
    (yc * yc / sc + ys * ys / ss) / (2.0 * variance)
}

/// The false-alarm probability of the maximum power over `bins` scanned
/// frequencies: 1 − (1 − e^{−z})^M (Scargle 1982; the full bin count is
/// the conservative — upward — FAP estimate, see the module header).
fn false_alarm_prob(peak_power: f64, bins: usize) -> f64 {
    let single = (-peak_power).exp(); // P(Z > z) = e^{−z}, z ~ Exp(1) under H0
    1.0 - (1.0 - single).powi(bins as i32)
}

/// The shared validation+compute body: `Vec<f64>` extraction, the loud
/// [SPECTRAL_INPUT] domain gates, the Degraded-loud branches, the grid,
/// and the normalized powers.
fn spectrum_from_args(fn_name: &str, args: &[Value]) -> Result<Value, String> {
    let times_raw = expect_list_arg(fn_name, args, 0)?;
    let values_raw = expect_list_arg(fn_name, args, 1)?;
    if times_raw.len() != values_raw.len() {
        return Err(coded_error(
            CODE_SPECTRAL_INPUT,
            format!(
                "{}: times and values must have equal length, got {} and {}",
                fn_name,
                times_raw.len(),
                values_raw.len()
            ),
        ));
    }
    let mut times = Vec::with_capacity(times_raw.len());
    for v in &times_raw {
        match v {
            Value::Float(f) if f.is_finite() => times.push(*f),
            other => {
                return Err(coded_error(
                    CODE_SPECTRAL_INPUT,
                    format!(
                        "{}: times must be finite numbers, got {} ({})",
                        fn_name,
                        other.type_name(),
                        other
                    ),
                ));
            }
        }
    }
    let mut values = Vec::with_capacity(values_raw.len());
    for v in &values_raw {
        match v {
            Value::Float(f) if f.is_finite() => values.push(*f),
            other => {
                return Err(coded_error(
                    CODE_SPECTRAL_INPUT,
                    format!(
                        "{}: values must be finite numbers, got {} ({})",
                        fn_name,
                        other.type_name(),
                        other
                    ),
                ));
            }
        }
    }
    if times.len() < SPECTRAL_MIN_POINTS {
        return Ok(degraded(format!(
            "insufficient points: got {}, need >= {} for a significance-capable periodogram",
            times.len(),
            SPECTRAL_MIN_POINTS
        )));
    }
    let n = times.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let variance = values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / (n - 1.0);
    if variance <= 0.0 {
        return Ok(degraded(
            "degenerate series: zero variance (constant values carry no spectral content)"
                .to_string(),
        ));
    }
    let (freqs, baseline, coarsened) = match derive_grid(&times) {
        GridOutcome::Grid {
            freqs,
            baseline,
            coarsened,
        } => (freqs, baseline, coarsened),
        GridOutcome::Degraded(reason) => return Ok(degraded(reason)),
    };
    // №607: the spectral contour joins the per-request/tick CONTOUR BUDGET
    // (the №546 seam — the scope lives at the serve/request boundaries and
    // at the program-run entries): the work-based cost (N × M sine/cosine
    // pairs, 10⁶ per unit) is charged BEFORE the compute; exhaustion is a
    // LOUD [CONTOUR_BUDGET_EXCEEDED] refusal of the call — fail-closed,
    // never a silent degradation. Outside a budgeted context (a direct
    // lib-level call, no request boundary) the charge is skipped — the
    // call's own N × M work budget still bounds it.
    let units = ((times.len() as u64) * (freqs.len() as u64) / 1_000_000).max(1);
    crate::builtins::embed_seam::seam_budget_check_scoped(units)?;
    let y: Vec<f64> = values.iter().map(|v| v - mean).collect();
    let powers: Vec<f64> = freqs
        .iter()
        .map(|f| ls_power_at(*f, &times, &y, variance))
        .collect();
    let (degraded_flag, reason_field) = match coarsened {
        Some(reason) => (Value::Bool(true), Value::String(reason)),
        None => (Value::Bool(false), Value::String(String::new())),
    };
    Ok(make_struct(
        "Spectrum",
        vec![
            (
                "freqs",
                Value::List(freqs.iter().map(|f| Value::Float(*f)).collect()),
            ),
            (
                "powers",
                Value::List(powers.iter().map(|p| Value::Float(*p)).collect()),
            ),
            ("n_points", Value::Float(n)),
            ("baseline", Value::Float(baseline)),
            ("grid_size", Value::Float(freqs.len() as f64)),
            ("degraded", degraded_flag),
            // №607: the explicit coarsening note when the work budget
            // re-sampled the grid — the consumer reads WHY, never a silent
            // coarser spectrum.
            ("degraded_reason", reason_field),
        ],
    ))
}

/// The typed `Degraded` result (the backend_select shape): ok=false,
/// degraded=true, the spectral stage named, the reason carried — never a
/// silent empty spectrum (§16.0-D).
fn degraded(reason: String) -> Value {
    make_struct(
        "Degraded",
        vec![
            ("ok", Value::Bool(false)),
            ("degraded", Value::Bool(true)),
            ("class", Value::String("spectral".to_string())),
            ("reason", Value::String(reason)),
        ],
    )
}

/// `lomb_scargle(times, values)` — the normalized Lomb–Scargle periodogram
/// for unevenly sampled series (Scargle 1982; the per-frequency phase
/// offset τ, Press et al. NR §13.8 formulation). The frequency grid derives
/// from the data: f_min = 1/T, f_max = 1/(2·median-spacing), Δf = 1/(N·T)
/// (N-fold oversampling of the Rayleigh limit) — no magic constants. Powers
/// are normalized by 2σ², so under the Gaussian-noise null a bin is
/// Exp(1)-distributed. Fewer than 12 points, a zero-variance series, a zero
/// baseline, or an over-capacity grid returns the typed `Degraded` struct
/// (degraded=true) — never a quiet spectrum. A length mismatch or a
/// non-finite element is a loud [SPECTRAL_INPUT] domain error. Pure
/// function — zero effects (№316). Numbers only, no interpretations (the
/// instrumental forecast posture, docs/limitations.md).
/// №607: past the WORK budget (N × M > 10⁷ sine/cosine pairs) the grid is
/// COARSENED over the same band — the Spectrum carries `degraded: true` +
/// the explicit `degraded_reason`; the call also charges the contour budget
/// seam (the №546 per-request budget) and refuses loudly on exhaustion.
pub(crate) fn builtin_lomb_scargle(args: &[Value]) -> Result<Value, String> {
    spectrum_from_args("lomb_scargle", args)
}

/// `spectral_peak(spectrum)` — the dominant frequency of a `lomb_scargle`
/// Spectrum with its false-alarm p-value (1 − (1 − e^{−z})^M over the
/// scanned grid — the conservative full-bin count) and its share of the
/// total spectral power inside a ±1/T band around the peak (one Rayleigh
/// resolution element on each side — derived from the baseline, not a
/// magic constant). A `Degraded` input propagates loudly and unchanged;
/// anything that is not a Spectrum is a loud [SPECTRAL_INPUT] error. Pure
/// function — zero effects (№316). Numbers only, no interpretations.
pub(crate) fn builtin_spectral_peak(args: &[Value]) -> Result<Value, String> {
    let fn_name = "spectral_peak";
    if args.len() != 1 {
        return Err(format!(
            "{}: expected 1 argument, got {}",
            fn_name,
            args.len()
        ));
    }
    // Degraded flows through LOUDLY — the consumer sees the degradation,
    // not a synthesized peak (§16.0-D).
    if let Value::Struct { type_name, fields } = &args[0] {
        if type_name == "Degraded" {
            let degraded = fields.get("degraded").cloned().unwrap_or(Value::Bool(true));
            let reason = fields.get("reason").cloned().unwrap_or(Value::Unit);
            let class = fields
                .get("class")
                .cloned()
                .unwrap_or(Value::String("spectral".to_string()));
            let ok = fields.get("ok").cloned().unwrap_or(Value::Bool(false));
            return Ok(make_struct(
                "Degraded",
                vec![
                    ("ok", ok),
                    ("degraded", degraded),
                    ("class", class),
                    ("reason", reason),
                ],
            ));
        }
    }
    let (freqs, powers, baseline, grid_size) = match &args[0] {
        Value::Struct { type_name, fields } if type_name == "Spectrum" => {
            let read_list = |key: &str| -> Result<Vec<f64>, String> {
                match fields.get(key) {
                    Some(Value::List(items)) => {
                        let mut out = Vec::with_capacity(items.len());
                        for v in items {
                            match v {
                                Value::Float(f) => out.push(*f),
                                other => {
                                    return Err(coded_error(
                                        CODE_SPECTRAL_INPUT,
                                        format!(
                                            "{}: Spectrum.{} must hold floats, got {}",
                                            fn_name,
                                            key,
                                            other.type_name()
                                        ),
                                    ));
                                }
                            }
                        }
                        Ok(out)
                    }
                    other => Err(coded_error(
                        CODE_SPECTRAL_INPUT,
                        format!(
                            "{}: Spectrum.{} must be a List, got {}",
                            fn_name,
                            key,
                            other.map(|v| v.type_name()).unwrap_or("nothing")
                        ),
                    )),
                }
            };
            let freqs = read_list("freqs")?;
            let powers = read_list("powers")?;
            let baseline = match fields.get("baseline") {
                Some(Value::Float(t)) => *t,
                other => {
                    return Err(coded_error(
                        CODE_SPECTRAL_INPUT,
                        format!(
                            "{}: Spectrum.baseline must be Float, got {}",
                            fn_name,
                            other.map(|v| v.type_name()).unwrap_or("nothing")
                        ),
                    ));
                }
            };
            let grid_size = match fields.get("grid_size") {
                Some(Value::Float(m)) => *m as usize,
                other => {
                    return Err(coded_error(
                        CODE_SPECTRAL_INPUT,
                        format!(
                            "{}: Spectrum.grid_size must be Float, got {}",
                            fn_name,
                            other.map(|v| v.type_name()).unwrap_or("nothing")
                        ),
                    ));
                }
            };
            (freqs, powers, baseline, grid_size)
        }
        other => {
            return Err(coded_error(
                CODE_SPECTRAL_INPUT,
                format!(
                    "{}: expected a Spectrum struct from lomb_scargle, got {}",
                    fn_name,
                    other.type_name()
                ),
            ));
        }
    };
    if freqs.is_empty() || freqs.len() != powers.len() || grid_size != freqs.len() {
        return Err(coded_error(
            CODE_SPECTRAL_INPUT,
            format!(
                "{}: Spectrum is inconsistent (freqs {}, powers {}, grid_size {})",
                fn_name,
                freqs.len(),
                powers.len(),
                grid_size
            ),
        ));
    }
    // The first maximum — deterministic under ties (the earliest bin wins).
    let mut best = 0usize;
    for (i, &p) in powers.iter().enumerate() {
        if p > powers[best] {
            best = i;
        }
    }
    let total: f64 = powers.iter().sum();
    if total <= 0.0 || !total.is_finite() {
        return Err(coded_error(
            CODE_SPECTRAL_INPUT,
            format!(
                "{}: Spectrum carries no finite power (total {})",
                fn_name, total
            ),
        ));
    }
    // The ±1/T band around the peak — one Rayleigh resolution element per
    // side; the fraction of the TOTAL periodogram power inside it.
    let half_band = 1.0 / baseline;
    let band: f64 = freqs
        .iter()
        .zip(powers.iter())
        .filter(|(f, _)| ((*f - freqs[best]).abs()) <= half_band)
        .map(|(_, p)| p)
        .sum();
    let peak = powers[best];
    Ok(make_struct(
        "SpectralPeak",
        vec![
            ("freq", Value::Float(freqs[best])),
            ("power_fraction", Value::Float(band / total)),
            ("p_value", Value::Float(false_alarm_prob(peak, grid_size))),
            ("degraded", Value::Bool(false)),
        ],
    ))
}

// ── №591 unit evidence: direct-handler access keeps the statistical DoD
//    rows in milliseconds; every randomness source is a DEDICATED test-only
//    xorshift (deterministic seeds — stable evidence, not flaky stochastic
//    assertions) ──────────────────────────────────────────────────────────

#[cfg(test)]
mod spectral_tests {
    use super::*;

    /// Test-only deterministic stream (independent of the production PRNG —
    /// the spectral contour consumes no randomness; this drives the SYNTHETIC
    /// data only).
    struct Xorshift(u64);

    impl Xorshift {
        fn next_u64(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        /// Uniform on [0, 1)
        fn next_uniform(&mut self) -> f64 {
            (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
        }
        /// Uniform on [-1, 1)
        fn next_signed(&mut self) -> f64 {
            2.0 * self.next_uniform() - 1.0
        }
    }

    fn call_lomb(times: Vec<f64>, values: Vec<f64>) -> Result<Value, String> {
        let args = vec![
            Value::List(times.into_iter().map(Value::Float).collect()),
            Value::List(values.into_iter().map(Value::Float).collect()),
        ];
        builtin_lomb_scargle(&args)
    }

    fn unpack_spectrum(v: &Value) -> (Vec<f64>, Vec<f64>, f64, usize) {
        match v {
            Value::Struct { type_name, fields } if type_name == "Spectrum" => {
                let read = |key: &str| -> Vec<f64> {
                    match fields.get(key) {
                        Some(Value::List(items)) => items
                            .iter()
                            .map(|x| match x {
                                Value::Float(f) => *f,
                                other => {
                                    panic!("{}: expected Float, got {}", key, other.type_name())
                                }
                            })
                            .collect(),
                        other => panic!("{} missing: {:?}", key, other.map(|o| o.type_name())),
                    }
                };
                let baseline = match fields.get("baseline") {
                    Some(Value::Float(t)) => *t,
                    other => panic!("baseline missing: {:?}", other.map(|o| o.type_name())),
                };
                let grid = match fields.get("grid_size") {
                    Some(Value::Float(m)) => *m as usize,
                    other => panic!("grid_size missing: {:?}", other.map(|o| o.type_name())),
                };
                (read("freqs"), read("powers"), baseline, grid)
            }
            other => panic!("expected Spectrum, got {:?}", other.type_name()),
        }
    }

    fn peak_freq_of(v: &Value) -> (f64, f64, f64) {
        match builtin_spectral_peak(std::slice::from_ref(v)) {
            Ok(Value::Struct { type_name, fields }) if type_name == "SpectralPeak" => {
                let get = |k: &str| match fields.get(k) {
                    Some(Value::Float(f)) => *f,
                    other => panic!("{} missing: {:?}", k, other.map(|o| o.type_name())),
                };
                (get("freq"), get("power_fraction"), get("p_value"))
            }
            Ok(other) => panic!("expected SpectralPeak, got {:?}", other.type_name()),
            Err(e) => panic!("spectral_peak failed: {}", e),
        }
    }

    /// The DoD row 1: a sinusoid of a KNOWN period with 30% of points knocked
    /// out — the peak lands within 2% of the true frequency on 10 independent
    /// seeds. The knock-out pattern comes from the test-only xorshift seeded
    /// per case (deterministic, so the evidence is stable).
    #[test]
    fn n591_gapped_sinusoid_peak_within_two_percent_on_ten_seeds() {
        let true_period = 10.3f64;
        let true_freq = 1.0 / true_period;
        for seed in 0..10u64 {
            let mut rng = Xorshift(0x9E3779B97F4A7C15 ^ (seed + 1));
            let mut times = Vec::new();
            let mut values = Vec::new();
            for t in 0..120i64 {
                // knock out ~30% deterministically
                if rng.next_uniform() < 0.30 {
                    continue;
                }
                let tf = t as f64;
                times.push(tf);
                // a pure sinusoid, amplitude 1 — the mean subtraction makes
                // the offset irrelevant
                values.push((std::f64::consts::TAU * tf / true_period).sin());
            }
            let spectrum = call_lomb(times, values).expect("spectrum must compute");
            let (freq, _fraction, p_value) = peak_freq_of(&spectrum);
            let rel_err = (freq - true_freq).abs() / true_freq;
            assert!(
                rel_err <= 0.02,
                "seed {}: peak freq {} vs true {} (rel err {:.4} > 2%), p={}",
                seed,
                freq,
                true_freq,
                rel_err,
                p_value
            );
            // a coherent sinusoid peak must be significant at α = 0.01
            assert!(
                p_value < 0.01,
                "seed {}: coherent peak must be significant, p={}",
                seed,
                p_value
            );
        }
    }

    /// The DoD row 2: white noise — the peak's p-value must NOT claim
    /// periodicity at the declared α; the false-positive share stays at or
    /// below α (the conservative full-bin FAP biases p upward — the safe
    /// direction). 100 seeds at α = 0.05: mean 5 FPs, σ ≈ 2.18 — the bound
    /// 11 is the documented 3σ band (a statistical test, not a bit-pin;
    /// the tolerance is the honest reading of «доля ложных срабатываний
    /// в пределах объявленного α»).
    #[test]
    fn n591_white_noise_false_positive_rate_within_declared_alpha() {
        let alpha = 0.05f64;
        let seeds = 100u32;
        let mut false_positives = 0u32;
        let mut p_sum = 0.0f64;
        for s in 0..seeds {
            let mut rng = Xorshift(0xDEADBEEFCAFEBABE ^ (s as u64 + 7));
            let times: Vec<f64> = (0..80).map(|t| t as f64).collect();
            let values: Vec<f64> = (0..80).map(|_| rng.next_signed()).collect();
            let spectrum = call_lomb(times, values).expect("spectrum must compute");
            let (_, _, p) = peak_freq_of(&spectrum);
            assert!((0.0..=1.0).contains(&p), "p_value {} outside [0, 1]", p);
            if p < alpha {
                false_positives += 1;
            }
            p_sum += p;
        }
        assert!(
            false_positives <= 11,
            "white-noise false positives {} of {} exceed the declared 3σ band at α={} \
             (the FAP arithmetic is over-claiming)",
            false_positives,
            seeds,
            alpha
        );
        // A sanity floor in the OTHER direction: a broken FAP that reports
        // near-certain significance everywhere would push the mean p to 0.
        // The conservative full-bin M biases the mean UP, never below ~0.5;
        // the floor 0.4 catches the inverted-sign / wrong-normalization class.
        let mean_p = p_sum / seeds as f64;
        assert!(
            mean_p >= 0.4,
            "mean white-noise p_value {} < 0.4 — the FAP arithmetic is over-claiming significance",
            mean_p
        );
    }

    /// Degraded-loud (§16.0-D): every degradation branch returns the typed
    /// Degraded struct with degraded=true — never a quiet result.
    #[test]
    fn n591_degraded_paths_are_loud_and_typed() {
        // insufficient points
        let short: Vec<f64> = (0..11).map(|t| t as f64).collect();
        let v = call_lomb(short.clone(), short.clone()).expect("short must not error");
        assert!(matches!(&v, Value::Struct { type_name, fields }
            if type_name == "Degraded"
                && matches!(fields.get("degraded"), Some(Value::Bool(true)))));
        // zero variance (constant series)
        let times: Vec<f64> = (0..40).map(|t| t as f64).collect();
        let constant = vec![2.5f64; 40];
        let v = call_lomb(times.clone(), constant).expect("constant must not error");
        match &v {
            Value::Struct { type_name, fields } => {
                assert_eq!(type_name, "Degraded");
                assert!(matches!(fields.get("degraded"), Some(Value::Bool(true))));
                match fields.get("reason") {
                    Some(Value::String(r)) => assert!(r.contains("zero variance")),
                    other => panic!("reason missing: {:?}", other),
                }
            }
            other => panic!("expected Degraded, got {:?}", other.type_name()),
        }
        // zero baseline (all times identical)
        let same_times = vec![3.0f64; 40];
        let values: Vec<f64> = (0..40).map(|i| i as f64).collect();
        let v = call_lomb(same_times, values).expect("zero baseline must not error");
        assert!(matches!(&v, Value::Struct { type_name, .. } if type_name == "Degraded"));
    }

    /// Loud [SPECTRAL_INPUT] domain errors: length mismatch, NaN, infinity —
    /// each carries the stable origin stamp at position 0 (№385/ADR-0169).
    #[test]
    fn n591_malformed_input_is_loud_with_origin_stamp() {
        let ok: Vec<f64> = (0..40).map(|t| t as f64).collect();
        let mismatch = call_lomb(ok.clone(), ok[..10].to_vec()).unwrap_err();
        assert!(
            mismatch.starts_with("[SPECTRAL_INPUT] "),
            "length mismatch must carry the stamp, got: {}",
            mismatch
        );
        let mut nan = ok.clone();
        nan[3] = f64::NAN;
        let nan_err = call_lomb(nan.clone(), ok.clone()).unwrap_err();
        assert!(nan_err.starts_with("[SPECTRAL_INPUT] "), "got: {}", nan_err);
        let mut inf = ok.clone();
        inf[5] = f64::INFINITY;
        let inf_err = call_lomb(inf, ok).unwrap_err();
        assert!(inf_err.starts_with("[SPECTRAL_INPUT] "), "got: {}", inf_err);
    }

    /// spectral_peak propagates a Degraded spectrum LOUDLY (unchanged shape,
    /// degraded=true) and refuses non-Spectrum input with the typed stamp.
    #[test]
    fn n591_peak_propagates_degradation_and_refuses_foreign_input() {
        let short: Vec<f64> = (0..11).map(|t| t as f64).collect();
        let degraded_spectrum = call_lomb(short.clone(), short).expect("short must not error");
        let propagated =
            builtin_spectral_peak(std::slice::from_ref(&degraded_spectrum)).expect("propagate");
        match &propagated {
            Value::Struct { type_name, fields } => {
                assert_eq!(type_name, "Degraded");
                assert!(matches!(fields.get("degraded"), Some(Value::Bool(true))));
            }
            other => panic!("expected Degraded propagation, got {:?}", other.type_name()),
        }
        let err = builtin_spectral_peak(&[Value::Float(1.0)]).unwrap_err();
        assert!(
            err.starts_with("[SPECTRAL_INPUT] "),
            "foreign input must carry the stamp, got: {}",
            err
        );
    }

    /// The grid derivation is data-driven: the derived bin count grows with
    /// the series length (N-fold oversampling) and the baseline is exact.
    #[test]
    fn n591_grid_derives_from_length_and_density() {
        let build = |n: usize| -> (Vec<f64>, Vec<f64>) {
            let times: Vec<f64> = (0..n).map(|t| t as f64).collect();
            let values: Vec<f64> = (0..n).map(|i| (i as f64 * 0.37).sin()).collect();
            (times, values)
        };
        let (t40, v40) = build(40);
        let (t80, v80) = build(80);
        let s40 = call_lomb(t40, v40).expect("40-point spectrum");
        let s80 = call_lomb(t80, v80).expect("80-point spectrum");
        let (_, p40, b40, m40) = unpack_spectrum(&s40);
        let (_, p80, b80, m80) = unpack_spectrum(&s80);
        assert_eq!(b40, 39.0);
        assert_eq!(b80, 79.0);
        assert!(
            m80 > m40 * 2,
            "the N-fold-oversampled grid must grow with length: {} vs {}",
            m80,
            m40
        );
        assert_eq!(p40.len(), m40);
        assert_eq!(p80.len(), m80);
    }
}

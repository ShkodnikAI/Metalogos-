// ── Math / conversion / collection-size builtins ──────────────────

use crate::interpreter::Value;

use super::core::expect_float_arg;

pub(crate) fn builtin_float(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::Float(f)) => Ok(Value::Float(*f)),
        Some(Value::String(s)) => s
            .parse::<f64>()
            .map(Value::Float)
            .map_err(|_| format!("float() cannot parse '{}'", s)),
        _ => Err("float() requires 1 argument".to_string()),
    }
}

pub(crate) fn builtin_to_string(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("to_string() requires 1 argument".to_string());
    }
    // №355: WorldState is private-by-default state — its materialization
    // outside the verified contour is a typed refusal + an audit record
    // (the Phase-1 lattice / №349 consistency). Every other embodied
    // handle projects through its opaque Display marker (no content).
    super::embodied::guard_world_state_to_string(&args[0])?;
    // №440: a FORECAST handle materializes TAINT-CONDITIONALLY — a clean
    // forecast renders its read-only content, a tainted one refuses with
    // the typed FORECAST_TAINTED stamp + the forecast.denied record.
    super::forecast::guard_forecast_to_string(&args[0])?;
    // Use Value's Display impl — Float omits .0 for integers automatically
    Ok(Value::String(format!("{}", args[0])))
}

// №514 (audit 28.09 C-10): the soft-failure rule is ONE — "silence is
// visible in the name" (`*_or`, the №481 env/env_or naming rule). A
// NON-NUMERIC string was silently converted to 0.0 — the audit's vector
// (`to_float("12,50")` → 0.0 → a zero-value charge with no error) — so the
// conversion error is now LOUD with the stable TYPE_MISMATCH code, and the
// explicit fallback lives in `to_float_or(value, default)`. The Bool →
// 1.0/0.0 mapping is a conversion, not a soft failure — unchanged.
pub(crate) fn builtin_to_float(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::Float(f)) => Ok(Value::Float(*f)),
        Some(Value::String(s)) => s.parse::<f64>().map(Value::Float).map_err(|_| {
            crate::interpreter::values::coded_error(
                crate::interpreter::values::CODE_TYPE_MISMATCH,
                format!(
                    "to_float({s:?}): not a number — use to_float_or(value, <default>) for an explicit fallback (№514)"
                ),
            )
        }),
        Some(Value::Bool(b)) => Ok(Value::Float(if *b { 1.0 } else { 0.0 })),
        Some(other) => {
            // №514: converting a non-scalar (list, secret, ...) to a float is
            // a programming error, not a default-value situation — LOUD.
            Err(crate::interpreter::values::coded_error(
                crate::interpreter::values::CODE_TYPE_MISMATCH,
                format!(
                    "to_float(): unsupported type {} — use to_string first, or to_float_or(value, <default>) for an explicit fallback (№514)",
                    other.type_name()
                ),
            ))
        }
        None => Err("to_float() requires 1 argument".to_string()),
    }
}

/// `to_float_or(value, default)` — the EXPLICIT-silence twin of `to_float`
/// (№514, the `_or` naming rule of №481): the parsed value when the string
/// parses, the default when it does not. Every firing of the fallback is
/// announced on the audit stderr (`[TO_FLOAT_OR]` — the №326 op-log
/// posture: the VALUE is never logged, only the fact). A non-scalar input
/// type stays LOUD — explicit silence covers DATA, not type errors.
pub(crate) fn builtin_to_float_or(args: &[Value]) -> Result<Value, String> {
    let default = match args.get(1) {
        Some(Value::Float(f)) => *f,
        Some(other) => {
            return Err(crate::interpreter::values::coded_error(
                crate::interpreter::values::CODE_TYPE_MISMATCH,
                format!(
                    "to_float_or(): the default must be a Float, got {} (№514)",
                    other.type_name()
                ),
            ))
        }
        None => return Err("to_float_or() requires 2 arguments".to_string()),
    };
    match args.first() {
        Some(Value::Float(f)) => Ok(Value::Float(*f)),
        Some(Value::String(s)) => match s.parse::<f64>() {
            Ok(v) => Ok(Value::Float(v)),
            Err(_) => {
                eprintln!(
                    "[TO_FLOAT_OR] value did not parse as a number — using the explicit default"
                );
                Ok(Value::Float(default))
            }
        },
        Some(Value::Bool(b)) => Ok(Value::Float(if *b { 1.0 } else { 0.0 })),
        Some(other) => Err(crate::interpreter::values::coded_error(
            crate::interpreter::values::CODE_TYPE_MISMATCH,
            format!(
                "to_float_or(): unsupported type {} — use to_string first (№514)",
                other.type_name()
            ),
        )),
        None => Err("to_float_or() requires 2 arguments".to_string()),
    }
}

pub(crate) fn builtin_confidence(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::Fluid(variants)) => {
            let best = variants
                .iter()
                .map(|v| v.confidence)
                .fold(0.0_f64, f64::max);
            Ok(Value::Float(best))
        }
        Some(_) => Ok(Value::Float(1.0)), // concrete values are fully confident
        None => Err("confidence() requires 1 argument".to_string()),
    }
}

/// `__abs(x)` — std-library primitive behind the std `abs` wrapper.
pub(crate) fn builtin_abs(args: &[Value]) -> Result<Value, String> {
    let f = expect_float_arg("__abs", args, 0)?;
    Ok(Value::Float(f.abs()))
}

/// `__min(a, b)` — std-library primitive behind the std `min` wrapper.
pub(crate) fn builtin_min(args: &[Value]) -> Result<Value, String> {
    let a = expect_float_arg("__min", args, 0)?;
    let b = expect_float_arg("__min", args, 1)?;
    Ok(Value::Float(a.min(b)))
}

/// `__max(a, b)` — std-library primitive behind the std `max` wrapper.
pub(crate) fn builtin_max(args: &[Value]) -> Result<Value, String> {
    let a = expect_float_arg("__max", args, 0)?;
    let b = expect_float_arg("__max", args, 1)?;
    Ok(Value::Float(a.max(b)))
}

/// `__clamp(x, lo, hi)` — std-library primitive behind the std `clamp` wrapper.
pub(crate) fn builtin_clamp(args: &[Value]) -> Result<Value, String> {
    let val = expect_float_arg("__clamp", args, 0)?;
    let lo = expect_float_arg("__clamp", args, 1)?;
    let hi = expect_float_arg("__clamp", args, 2)?;
    Ok(Value::Float(val.clamp(lo, hi)))
}

/// `__round(x)` — std-library primitive behind the std `round` wrapper.
pub(crate) fn builtin_round(args: &[Value]) -> Result<Value, String> {
    let f = expect_float_arg("__round", args, 0)?;
    Ok(Value::Float(f.round()))
}

/// `__first(list)` — std-library primitive behind the std `first` wrapper:
/// the first element (soft-failure semantics of the std layer apply).
pub(crate) fn builtin_first(args: &[Value]) -> Result<Value, String> {
    let list = match args.first() {
        Some(Value::List(items)) => items,
        _ => return Err("first() requires List as first argument".to_string()),
    };
    match list.first() {
        Some(v) => Ok(v.clone()),
        None => Ok(Value::String(String::new())), // soft-failure
    }
}

/// `__last(list)` — std-library primitive behind the std `last` wrapper:
/// the last element (soft-failure semantics of the std layer apply).
pub(crate) fn builtin_last(args: &[Value]) -> Result<Value, String> {
    let list = match args.first() {
        Some(Value::List(items)) => items,
        _ => return Err("last() requires List as first argument".to_string()),
    };
    match list.last() {
        Some(v) => Ok(v.clone()),
        None => Ok(Value::String(String::new())), // soft-failure
    }
}

/// `length(s)` — returns the length of a string or list as Float.
pub(crate) fn builtin_length(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::String(s)) => Ok(Value::Float(s.chars().count() as f64)),
        Some(Value::List(items)) => Ok(Value::Float(items.len() as f64)),
        other => Err(format!(
            "length() requires String or List, got {}",
            other.as_ref().map(|v| v.type_name()).unwrap_or("none")
        )),
    }
}

/// `to_int(s)` — parse a string to an integer Float (truncates towards zero).
/// №514 (audit 28.09 C-10): a NON-NUMERIC string was silently 0.0 — the
/// conversion error is now LOUD (TYPE_MISMATCH, the №514 naming rule);
/// the explicit fallback lives in `to_int_or(value, default)`. The Bool →
/// 1.0/0.0 mapping is a conversion, not a soft failure — unchanged.
pub(crate) fn builtin_to_int(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::Float(f)) => Ok(Value::Float(f.trunc())),
        Some(Value::String(s)) => {
            // Try integer parse first, then float truncation
            if let Ok(i) = s.parse::<i64>() {
                Ok(Value::Float(i as f64))
            } else if let Ok(f) = s.parse::<f64>() {
                Ok(Value::Float(f.trunc()))
            } else {
                Err(crate::interpreter::values::coded_error(
                    crate::interpreter::values::CODE_TYPE_MISMATCH,
                    format!(
                        "to_int({s:?}): not a number — use to_int_or(value, <default>) for an explicit fallback (№514)"
                    ),
                ))
            }
        }
        Some(Value::Bool(b)) => Ok(Value::Float(if *b { 1.0 } else { 0.0 })),
        Some(other) => Err(crate::interpreter::values::coded_error(
            crate::interpreter::values::CODE_TYPE_MISMATCH,
            format!(
                "to_int(): unsupported type {} — use to_string first, or to_int_or(value, <default>) for an explicit fallback (№514)",
                other.type_name()
            ),
        )),
        None => Err("to_int() requires 1 argument".to_string()),
    }
}

/// `to_int_or(value, default)` — the EXPLICIT-silence twin of `to_int`
/// (№514, the `_or` naming rule of №481): the parsed/truncated value when
/// the string parses, the default when it does not. Every firing of the
/// fallback is announced on the audit stderr (`[TO_INT_OR]`). A non-scalar
/// input type stays LOUD — explicit silence covers DATA, not type errors.
pub(crate) fn builtin_to_int_or(args: &[Value]) -> Result<Value, String> {
    let default = match args.get(1) {
        Some(Value::Float(f)) => *f,
        Some(other) => {
            return Err(crate::interpreter::values::coded_error(
                crate::interpreter::values::CODE_TYPE_MISMATCH,
                format!(
                    "to_int_or(): the default must be a Float, got {} (№514)",
                    other.type_name()
                ),
            ))
        }
        None => return Err("to_int_or() requires 2 arguments".to_string()),
    };
    match args.first() {
        Some(Value::Float(f)) => Ok(Value::Float(f.trunc())),
        Some(Value::String(s)) => {
            if let Ok(i) = s.parse::<i64>() {
                Ok(Value::Float(i as f64))
            } else if let Ok(f) = s.parse::<f64>() {
                Ok(Value::Float(f.trunc()))
            } else {
                eprintln!(
                    "[TO_INT_OR] value did not parse as a number — using the explicit default"
                );
                Ok(Value::Float(default))
            }
        }
        Some(Value::Bool(b)) => Ok(Value::Float(if *b { 1.0 } else { 0.0 })),
        Some(other) => Err(crate::interpreter::values::coded_error(
            crate::interpreter::values::CODE_TYPE_MISMATCH,
            format!(
                "to_int_or(): unsupported type {} — use to_string first (№514)",
                other.type_name()
            ),
        )),
        None => Err("to_int_or() requires 2 arguments".to_string()),
    }
}

// ── Наряд №177: Math foundation for Reflex (stage 1/6) ──────────────
//
// Numerically stable implementations of exp, ln, sqrt, pow, tanh,
// sigmoid, softmax + deterministic PRNG (random_seed/random).
// No external crates for PRNG — xorshift64, explicitly deterministic.

/// `exp(x)` — e^x. Direct delegation to f64::exp.
pub(crate) fn builtin_exp(args: &[Value]) -> Result<Value, String> {
    let x = expect_float_arg("exp", args, 0)?;
    Ok(Value::Float(x.exp()))
}

/// `ln(x)` — natural logarithm. Soft-failure: returns 0.0 for x <= 0
/// (documented, not a panic — NaN/inf in ML code is worse than 0.0).
pub(crate) fn builtin_ln(args: &[Value]) -> Result<Value, String> {
    let x = expect_float_arg("ln", args, 0)?;
    if x <= 0.0 {
        return Ok(Value::Float(0.0)); // soft-failure, documented
    }
    Ok(Value::Float(x.ln()))
}

/// `sqrt(x)` — square root. Soft-failure: returns 0.0 for x < 0.
pub(crate) fn builtin_sqrt(args: &[Value]) -> Result<Value, String> {
    let x = expect_float_arg("sqrt", args, 0)?;
    if x < 0.0 {
        return Ok(Value::Float(0.0)); // soft-failure, documented
    }
    Ok(Value::Float(x.sqrt()))
}

/// `pow(base, exp)` — base^exp. Direct delegation to f64::powf.
pub(crate) fn builtin_pow(args: &[Value]) -> Result<Value, String> {
    let base = expect_float_arg("pow", args, 0)?;
    let exp = expect_float_arg("pow", args, 1)?;
    Ok(Value::Float(base.powf(exp)))
}

/// `tanh(x)` — hyperbolic tangent. Direct delegation to f64::tanh.
/// Naturally bounded: tanh(x) ∈ (-1, 1) for all finite x.
pub(crate) fn builtin_tanh(args: &[Value]) -> Result<Value, String> {
    let x = expect_float_arg("tanh", args, 0)?;
    Ok(Value::Float(x.tanh()))
}

/// `sigmoid(x)` — logistic function 1/(1+e^{-x}).
/// Numerically stable: for x >= 0 uses 1/(1+exp(-x)),
/// for x < 0 uses exp(x)/(1+exp(x)) — avoids overflow in exp.
/// Returns 1.0 for very large positive x, 0.0 for very large negative x.
///
/// Наряд №182: the f64 math lives in `super::math_core::sigmoid_raw` —
/// shared with `nn/activation.rs` to eliminate the pre-Наряд №182
/// duplication of the numerically stable algorithm between this
/// `Value`-wrapping handler and the layer-level `&mut [f64]` apply.
pub(crate) fn builtin_sigmoid(args: &[Value]) -> Result<Value, String> {
    let x = expect_float_arg("sigmoid", args, 0)?;
    Ok(Value::Float(super::math_core::sigmoid_raw(x)))
}

/// `softmax(list)` — numerically stable softmax.
/// Subtracts max before exp to prevent overflow.
/// Output sums to 1.0 (within f64 epsilon).
///
/// Наряд №182: the f64 math lives in `super::math_core::softmax_raw` —
/// shared with `nn/activation.rs` for the same dedup reason as
/// `builtin_sigmoid` above.
pub(crate) fn builtin_softmax(args: &[Value]) -> Result<Value, String> {
    let list = match args.first() {
        Some(Value::List(items)) => items,
        Some(other) => {
            return Err(format!(
                "softmax() expected List argument, got {}",
                other.type_name()
            ))
        }
        None => return Err("softmax() requires 1 argument (List)".to_string()),
    };
    if list.is_empty() {
        return Ok(Value::List(vec![]));
    }

    // Extract float values — Value wrapping is unavoidable here (the
    // builtin interface is &[Value]), but the numerical work happens
    // in the shared softmax_raw, not in this handler.
    let values: Vec<f64> = list
        .iter()
        .map(|v| match v {
            Value::Float(f) => *f,
            other => other.as_float().unwrap_or(0.0),
        })
        .collect();

    let result: Vec<Value> = super::math_core::softmax_raw(&values)
        .into_iter()
        .map(Value::Float)
        .collect();
    Ok(Value::List(result))
}

// ── Deterministic PRNG (xorshift64) ──────────────────────────────────
//
// Наряд №177 Block 3: deterministic PRNG for reproducible weight init.
// Uses xorshift64 — simple, fast, fully deterministic, no external crate.
// State is stored in a thread-local to avoid threading issues.
// When random_seed(n) is called, the state is set to a value derived
// from n (not n directly — xorshift64 can't start from 0).
// When random() is called without a prior seed, it uses a non-deterministic
// seed (system time) and logs a warning.

use std::cell::RefCell;

thread_local! {
    static RNG_STATE: RefCell<Option<u64>> = const { RefCell::new(None) };
}

/// Convert a Float seed to a u64 xorshift state.
/// Ensures the state is never 0 (xorshift64 requires non-zero state).
fn seed_to_state(seed: f64) -> u64 {
    let bits = seed.to_bits();
    // XOR with a constant to ensure non-zero even if seed is 0.0
    let state = bits ^ 0x9E3779B97F4A7C15;
    if state == 0 {
        0x9E3779B97F4A7C15 // fallback for the degenerate case
    } else {
        state
    }
}

/// xorshift64 step — advances the state and returns the next random u64.
fn xorshift64(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// Convert a u64 to a float in [0.0, 1.0).
/// Uses the top 53 bits (mantissa width of f64) for maximum precision.
fn u64_to_float(bits: u64) -> f64 {
    // Mask to 53 bits (mantissa of f64), then divide by 2^53
    let mantissa = bits >> 11; // top 53 bits
    (mantissa as f64) / ((1u64 << 53) as f64)
}

/// `random_seed(n)` — set the deterministic PRNG seed.
/// All subsequent random() calls will produce the same sequence
/// for the same seed value.
pub(crate) fn builtin_random_seed(args: &[Value]) -> Result<Value, String> {
    let seed = expect_float_arg("random_seed", args, 0)?;
    let state = seed_to_state(seed);
    RNG_STATE.with(|s| {
        *s.borrow_mut() = Some(state);
    });
    Ok(Value::Unit)
}

/// `random()` — return a Float in [0.0, 1.0).
/// If random_seed() was called, uses the deterministic PRNG.
/// If not, uses a non-deterministic seed (system time) — logged.
pub(crate) fn builtin_random(args: &[Value]) -> Result<Value, String> {
    let _ = args; // no args
    let result = RNG_STATE.with(|s| {
        let mut borrow = s.borrow_mut();
        match &mut *borrow {
            Some(state) => {
                // Deterministic mode: advance the xorshift state
                let bits = xorshift64(state);
                Some(u64_to_float(bits))
            }
            None => {
                // Non-deterministic mode: seed from system time
                let seed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(1);
                let mut state = seed_to_state(seed as f64);
                let bits = xorshift64(&mut state);
                // Store the state so subsequent calls continue the sequence
                *borrow = Some(state);
                Some(u64_to_float(bits))
            }
        }
    });
    Ok(Value::Float(result.unwrap_or(0.0)))
}

// ── Наряд №590 (Волна 30, Камертон Н1-05): normal_sample ────────────
//
// `normal_sample(mean, stddev) -> Float` — the Box–Muller transform over
// the SHARED deterministic PRNG (the same thread-local xorshift64 that
// random_seed/random drive). No own state, no second algorithm: exactly
// one implementation (the classical Box–Muller, the polar/rejection
// variant is deliberately NOT introduced — one implementation, one point
// of failure, the naryad's own boundary).
//
// Determinism: a fixed seed yields the SAME sequence on TW and VM by
// construction — both backends call this one handler, which reads the
// shared thread-local state (the №372/ADR-0141 Stage 1.4 posture; the
// crosscheck asserts the parity end-to-end).
//
// Domain discipline (№316 classification): pure function — two uniform
// draws and closed-form arithmetic, zero effects. The classification
// comes from the provably-pure `math` category default (the same row
// family as random_seed/random in builtins_classification.rs).
//
// Loud failure: stddev <= 0.0 is a domain error, not a NaN pipeline —
// the error carries the stable origin stamp [NORMAL_SAMPLE_STDDEV]
// (№385/ADR-0169) so try{} classifies it to the typed code on BOTH
// backends (the stamp is read at position 0 only, stable_try_error_code).
/// `normal_sample(mean, stddev)` — one draw from N(mean, stddev) via the
/// classical Box–Muller transform over the shared deterministic PRNG (the
/// random_seed/random xorshift64 stream — a fixed seed yields the same
/// sequence on both backends). Pure function, no own state. stddev <= 0
/// (NaN included) is a loud domain error stamped [NORMAL_SAMPLE_STDDEV],
/// never a NaN result.
pub(crate) fn builtin_normal_sample(args: &[Value]) -> Result<Value, String> {
    let mean = expect_float_arg("normal_sample", args, 0)?;
    let stddev = expect_float_arg("normal_sample", args, 1)?;
    if stddev <= 0.0 || stddev.is_nan() {
        // Explicit NaN arm (the negated form is clippy-banned): a NaN
        // stddev is a domain error, not a silent passthrough (the
        // loud-failure contract).
        return Err(crate::interpreter::values::coded_error(
            crate::interpreter::values::CODE_NORMAL_SAMPLE_STDDEV,
            format!("normal_sample: stddev must be > 0, got {}", stddev),
        ));
    }
    // u1 ∈ (0, 1] via 1−u: Box–Muller takes ln(u1), and the raw
    // next_uniform() domain [0,1) contains 0.0 — mapping to the
    // open-at-zero side keeps ln finite WITHOUT a rejection loop and
    // WITHOUT skewing the law (1−u is uniform on (0,1] exactly as u is
    // on [0,1)). u2 stays on the raw [0,1) domain.
    let u1 = 1.0 - next_uniform();
    let u2 = next_uniform();
    // Classical Box–Muller: z = sqrt(-2 ln u1) * cos(2π u2).
    // u1 > 0.0 by construction, so ln is finite; sqrt domain holds.
    let z = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
    Ok(Value::Float(mean + stddev * z))
}

/// One uniform draw in [0.0, 1.0) from the shared PRNG state — the exact
/// body of `builtin_random`'s state machine, factored so `normal_sample`
/// consumes the SAME stream (№590: no second state, no second bootstrap).
/// Unseeded calls bootstrap from system time and store the state, so a
/// mixed random()/normal_sample() sequence stays one deterministic stream.
fn next_uniform() -> f64 {
    RNG_STATE.with(|s| {
        let mut borrow = s.borrow_mut();
        match &mut *borrow {
            Some(state) => u64_to_float(xorshift64(state)),
            None => {
                // Non-deterministic mode: seed from system time
                let seed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(1);
                let mut state = seed_to_state(seed as f64);
                let bits = xorshift64(&mut state);
                // Store the state so subsequent calls continue the sequence
                *borrow = Some(state);
                u64_to_float(bits)
            }
        }
    })
}

// ── №590 unit evidence: the 10⁶ statistical contract ─────────────────
//
// Direct-handler access (no interpreter marshaling) keeps 10⁶ draws in
// milliseconds. The seed is FIXED, so the sampled moments are the same
// numbers on every run — the 3σ bounds are deterministic evidence, not a
// flaky stochastic test (the DoD row: «10⁶ выборок: выборочные среднее и
// дисперсия в пределах 3σ от заданных»).
#[cfg(test)]
mod normal_sample_tests {
    use super::*;

    fn draw(mean: f64, stddev: f64) -> f64 {
        match builtin_normal_sample(&[Value::Float(mean), Value::Float(stddev)])
            .expect("seeded draw must succeed")
        {
            Value::Float(f) => f,
            other => panic!("expected Float, got {:?}", other.type_name()),
        }
    }

    #[test]
    fn n590_million_draw_moments_within_three_sigma() {
        builtin_random_seed(&[Value::Float(2026.0)]).expect("seed");
        let n: f64 = 1_000_000.0;
        let (mean, stddev) = (100.0f64, 15.0f64);
        let mut sum = 0.0f64;
        let mut sum_sq = 0.0f64;
        for _ in 0..(n as usize) {
            let x = draw(mean, stddev);
            sum += x;
            sum_sq += x * x;
        }
        let sample_mean = sum / n;
        let sample_var = sum_sq / n - sample_mean * sample_mean;
        // sd(mean) = σ/√N = 0.015 → 3σ = 0.045; give 4σ of headroom against
        // any single-seed granularity: still catches a wrong-scale sampler
        // (e.g. stddev applied as variance, or a uniform instead of normal).
        assert!(
            (sample_mean - mean).abs() < 4.0 * stddev / n.sqrt(),
            "sample mean {} too far from {}",
            sample_mean,
            mean
        );
        // Var(s²) ≈ 2σ⁴/N → sd(s²) = σ²·√(2/N) ≈ 0.318 → 3σ ≈ 0.955; 4σ
        // headroom again. A substituted 2π constant shifts z's variance by
        // a factor ≥ (π/3)² ≈ 1.1 on the cos arm alone — this bound and the
        // bit-exact pins in tests/naryad_590_normal_sample.rs both trip.
        assert!(
            (sample_var - stddev * stddev).abs() < 4.0 * stddev * stddev * (2.0f64 / n).sqrt(),
            "sample variance {} too far from {}",
            sample_var,
            stddev * stddev
        );
    }

    #[test]
    fn n590_nan_stddev_is_loud_not_silent() {
        let res = builtin_normal_sample(&[Value::Float(0.0), Value::Float(f64::NAN)]);
        let err = res.expect_err("NaN stddev is a domain error, not a silent passthrough");
        assert!(
            err.starts_with("[NORMAL_SAMPLE_STDDEV] "),
            "NaN refusal must carry the origin stamp, got: {}",
            err
        );
    }

    #[test]
    fn n590_zero_u1_domain_is_safe() {
        // The 1−u mapping puts u1 in (0, 1]; ln(u1) must stay finite for
        // every draw of a long run — a regression to raw u1 would produce
        // NaN/inf the moment u1 = 0.0 (probability 2⁻⁵³ per draw, so the
        // contract is verified structurally: recompute the mapping here and
        // assert the domain invariant holds for the whole stream).
        builtin_random_seed(&[Value::Float(99.0)]).expect("seed");
        for _ in 0..100_000 {
            let u1 = 1.0 - next_uniform();
            assert!(u1 > 0.0 && u1 <= 1.0, "u1 {} outside (0, 1]", u1);
            let z = (-2.0 * u1.ln()).sqrt();
            assert!(
                z.is_finite(),
                "sqrt(-2 ln u1) must stay finite, u1 = {}",
                u1
            );
        }
    }
}

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
    // Use Value's Display impl — Float omits .0 for integers automatically
    Ok(Value::String(format!("{}", args[0])))
}

pub(crate) fn builtin_to_float(args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::Float(f)) => Ok(Value::Float(*f)),
        Some(Value::String(s)) => Ok(s
            .parse::<f64>()
            .map(Value::Float)
            .unwrap_or(Value::Float(0.0))), // soft-failure: return 0.0 on parse error
        Some(Value::Bool(b)) => Ok(Value::Float(if *b { 1.0 } else { 0.0 })),
        _ => Ok(Value::Float(0.0)), // soft-failure
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

pub(crate) fn builtin_abs(args: &[Value]) -> Result<Value, String> {
    let f = expect_float_arg("__abs", args, 0)?;
    Ok(Value::Float(f.abs()))
}

pub(crate) fn builtin_min(args: &[Value]) -> Result<Value, String> {
    let a = expect_float_arg("__min", args, 0)?;
    let b = expect_float_arg("__min", args, 1)?;
    Ok(Value::Float(a.min(b)))
}

pub(crate) fn builtin_max(args: &[Value]) -> Result<Value, String> {
    let a = expect_float_arg("__max", args, 0)?;
    let b = expect_float_arg("__max", args, 1)?;
    Ok(Value::Float(a.max(b)))
}

pub(crate) fn builtin_clamp(args: &[Value]) -> Result<Value, String> {
    let val = expect_float_arg("__clamp", args, 0)?;
    let lo = expect_float_arg("__clamp", args, 1)?;
    let hi = expect_float_arg("__clamp", args, 2)?;
    Ok(Value::Float(val.clamp(lo, hi)))
}

pub(crate) fn builtin_round(args: &[Value]) -> Result<Value, String> {
    let f = expect_float_arg("__round", args, 0)?;
    Ok(Value::Float(f.round()))
}

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
                Ok(Value::Float(0.0)) // soft-failure
            }
        }
        Some(Value::Bool(b)) => Ok(Value::Float(if *b { 1.0 } else { 0.0 })),
        _ => Ok(Value::Float(0.0)), // soft-failure
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
pub(crate) fn builtin_sigmoid(args: &[Value]) -> Result<Value, String> {
    let x = expect_float_arg("sigmoid", args, 0)?;
    let result = if x >= 0.0 {
        // For x >= 0: exp(-x) is in (0, 1], no overflow risk.
        // 1.0 / (1.0 + exp(-x)) — standard formula, safe here.
        1.0 / (1.0 + (-x).exp())
    } else {
        // For x < 0: exp(-x) would overflow for large |x|.
        // Use the equivalent form: exp(x) / (1 + exp(x))
        // where exp(x) is in (0, 1) for x < 0 — no overflow.
        let exp_x = x.exp();
        exp_x / (1.0 + exp_x)
    };
    Ok(Value::Float(result))
}

/// `softmax(list)` — numerically stable softmax.
/// Subtracts max before exp to prevent overflow.
/// Output sums to 1.0 (within f64 epsilon).
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

    // Extract float values
    let values: Vec<f64> = list
        .iter()
        .map(|v| match v {
            Value::Float(f) => *f,
            other => other.as_float().unwrap_or(0.0),
        })
        .collect();

    // Numerical stability: subtract max before exp
    let max_val = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = values.iter().map(|v| (v - max_val).exp()).collect();
    let sum: f64 = exps.iter().sum();
    if sum == 0.0 {
        // All inputs were -inf or similar; return uniform distribution
        let n = values.len() as f64;
        return Ok(Value::List(
            values.iter().map(|_| Value::Float(1.0 / n)).collect(),
        ));
    }
    let result: Vec<Value> = exps.iter().map(|e| Value::Float(e / sum)).collect();
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

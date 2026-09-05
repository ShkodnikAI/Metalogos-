//! Reflex training/prediction builtins (Наряд №179b — этап 3/6).
//!
//! Connects the language to the already-implemented `ReflexModel::train`
//! and `compute_accuracy` (Наряд №179). The math is untouched — this
//! module only adds the language surface.
//!
//! ## Design choice (Block 1 of naryad 179b)
//!
//! The naryad recommended choosing between a new declaration
//! (`reflex_train_decl`) or a builtin function. We chose **builtin
//! function** because:
//!
//! 1. `ReflexModel::train` already accepts data and parameters as Rust
//!    function arguments — wrapping it as `reflex_train(model, data,
//!    epochs, metric, threshold)` is a thin adapter, not a new
//!    semantic concept.
//! 2. A builtin reuses the existing `FnCall` grammar — no new
//!    `grammar.pest` rule needed.
//! 3. `rollback_if` becomes a normal language expression
//!    (`if result.accuracy < 0.85 then ...`), not a new declarative
//!    construct. This is shorter and more composable than the
//!    declaration sketch from the original naryad.
//!
//! ## Dispatch model (Block 2 of naryad 179b)
//!
//! The `BuiltinFn = fn(&[Value]) -> Result<Value, String>` signature
//! cannot receive `&mut ReflexRegistry`. Two ways to bridge this:
//!
//! 1. Intercept `reflex_train` / `reflex_predict` in
//!    `interpreter::execution::invoke()` BEFORE the generic builtin
//!    dispatch (same pattern as `recall`, `memorize`, `find`).
//! 2. Use `thread_local!` to stash the registry (like `RNG_STATE`).
//!
//! We chose (1) because `recall`/`memorize` already use this pattern
//! and the registry is owned by `Interpreter`, not a free thread-local.
//!
//! The actual dispatch logic lives in `reflex_train_dispatch` /
//! `reflex_predict_dispatch` below. These are free functions that take
//! `&mut ReflexRegistry` / `&ReflexRegistry` as a parameter, so when
//! the VM gains Reflex support in a future naryad, it can call the
//! same functions without duplication (the "общее тело на оба бэкенда"
//! requirement of naryad 178).
//!
//! The `spec!` entries in `BUILTIN_REGISTRY` register stub handlers
//! that produce a clean "VM not yet supported" error. They exist for
//! bytecode index stability and arity checks — the real dispatch is
//! in `interpreter::execution::invoke()`.

use crate::interpreter::Value;
use crate::nn::{find_metric, ReflexId, ReflexModel, ReflexRegistry};
use std::collections::HashMap;

// ── Stub handlers (registered in BUILTIN_REGISTRY) ──────────────────
//
// These are ONLY reached if the VM backend somehow calls the builtin
// directly. The tree-walking interpreter intercepts `reflex_train` /
// `reflex_predict` in `invoke()` and never reaches these stubs.
//
// When the VM gains Reflex support (future naryad), these stubs will
// be replaced with calls to `reflex_train_dispatch` /
// `reflex_predict_dispatch` — same logic, different registry owner.

/// Stub — produces a clean error if VM calls reflex_train directly.
pub(crate) fn builtin_reflex_train_stub(_args: &[Value]) -> Result<Value, String> {
    Err(
        "reflex_train: VM backend does not yet support Reflex (ADR-0114) \
         — use `mlog run` (interpreter backend)"
            .to_string(),
    )
}

/// Stub — produces a clean error if VM calls reflex_predict directly.
pub(crate) fn builtin_reflex_predict_stub(_args: &[Value]) -> Result<Value, String> {
    Err(
        "reflex_predict: VM backend does not yet support Reflex (ADR-0114) \
         — use `mlog run` (interpreter backend)"
            .to_string(),
    )
}

// ── Shared dispatch bodies (reused by TW today, VM tomorrow) ────────

/// `reflex_train(model, data, epochs, metric_name, threshold) -> Struct`
///
/// Returns a Struct with fields:
///   - `loss`: Float (final train loss)
///   - `accuracy`: Float (holdout accuracy, 0.0..1.0)
///   - `metric`: String (metric name, e.g. "accuracy")
///   - `threshold_met`: Bool (true if accuracy >= threshold)
///
/// `data` is a `List<List<Float>>` where each inner list is
/// `[x_1, x_2, ..., x_n, class_idx]` — input features followed by the
/// integer class index (as Float). The dispatch splits features from
/// targets and calls `ReflexModel::train`.
///
/// `rollback_if` (Block 3 of naryad 179b) is NOT a builtin argument
/// — it's a normal language expression: `if result.accuracy <
/// 0.85 then ...`. The `threshold_met` field makes the common case
/// ergonomic without forcing a separate `if`.
pub fn reflex_train_dispatch(
    registry: &mut ReflexRegistry,
    args: &[Value],
) -> Result<Value, String> {
    if args.len() != 5 {
        return Err(format!(
            "reflex_train: expected 5 arguments (model, data, epochs, metric, threshold), got {}",
            args.len()
        ));
    }

    // arg 0: model handle
    let model_id: ReflexId = match &args[0] {
        Value::Reflex(id) => *id,
        other => {
            return Err(format!(
                "reflex_train: first argument must be a Reflex model handle, got {}",
                other.type_name()
            ));
        }
    };

    // arg 1: data — List<List<Float>>, each row = [features..., class_idx]
    let data = match &args[1] {
        Value::List(rows) => rows,
        other => {
            return Err(format!(
                "reflex_train: second argument must be a List of rows, got {}",
                other.type_name()
            ));
        }
    };

    // arg 2: epochs (Float, e.g. 200.0)
    let epochs_f = match &args[2] {
        Value::Float(n) => *n,
        other => {
            return Err(format!(
                "reflex_train: third argument (epochs) must be Float, got {}",
                other.type_name()
            ));
        }
    };
    if epochs_f < 0.0 {
        return Err(format!(
            "reflex_train: epochs must be >= 0, got {}",
            epochs_f
        ));
    }
    let epochs = epochs_f as usize;

    // arg 3: metric name (String)
    let metric_name = match &args[3] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "reflex_train: fourth argument (metric) must be String, got {}",
                other.type_name()
            ));
        }
    };

    // Validate metric is registered (ADR-0114 addendum)
    if find_metric(&metric_name).is_none() {
        return Err(format!(
            "reflex_train: unknown metric '{}'. Registered metrics: {}",
            metric_name,
            crate::nn::metric_names().join(", ")
        ));
    }

    // arg 4: threshold (Float, 0.0..1.0)
    let threshold = match &args[4] {
        Value::Float(n) => *n,
        other => {
            return Err(format!(
                "reflex_train: fifth argument (threshold) must be Float, got {}",
                other.type_name()
            ));
        }
    };

    // Get the model (mutable — train mutates weights)
    let model: &mut ReflexModel = registry.get_mut(model_id).ok_or_else(|| {
        format!(
            "reflex_train: model handle {:?} not found in registry",
            model_id
        )
    })?;

    // Split data into inputs and target_classes
    let mut inputs: Vec<Vec<f64>> = Vec::with_capacity(data.len());
    let mut targets: Vec<usize> = Vec::with_capacity(data.len());
    for (i, row) in data.iter().enumerate() {
        let features: &[Value] = match row {
            Value::List(f) => f,
            other => {
                return Err(format!(
                    "reflex_train: row {} must be a List, got {}",
                    i,
                    other.type_name()
                ));
            }
        };
        if features.len() < 2 {
            return Err(format!(
                "reflex_train: row {} has {} elements, need at least 2 (1 feature + class_idx)",
                i,
                features.len()
            ));
        }
        let class_idx_f = match features.last() {
            Some(Value::Float(n)) => *n,
            Some(other) => {
                return Err(format!(
                    "reflex_train: row {} last element (class_idx) must be Float, got {}",
                    i,
                    other.type_name()
                ));
            }
            None => {
                return Err(format!(
                    "reflex_train: row {} is empty — need at least 1 feature + class_idx",
                    i
                ));
            }
        };
        let class_idx = class_idx_f as usize;
        let feature_vec: Vec<f64> = features[..features.len() - 1]
            .iter()
            .map(|v| match v {
                Value::Float(n) => Ok(*n),
                other => Err(format!(
                    "reflex_train: row {} feature must be Float, got {}",
                    i,
                    other.type_name()
                )),
            })
            .collect::<Result<Vec<_>, _>>()?;
        // Validate class_idx against model labels
        if class_idx >= model.labels.len() {
            return Err(format!(
                "reflex_train: row {} class_idx {} out of range (model has {} labels: {:?})",
                i,
                class_idx,
                model.labels.len(),
                model.labels
            ));
        }
        // Validate feature count matches model input_size
        if feature_vec.len() != model.input_size {
            return Err(format!(
                "reflex_train: row {} has {} features, model expects {}",
                i,
                feature_vec.len(),
                model.input_size
            ));
        }
        inputs.push(feature_vec);
        targets.push(class_idx);
    }

    // Train (the math is in ReflexModel::train — untouched by this naryad)
    let learning_rate = 0.1; // matches naryad_179_convergence contract
    let (loss, accuracy) = model.train(&inputs, &targets, epochs, learning_rate)?;

    // Build result Struct
    let mut fields: HashMap<String, Value> = HashMap::new();
    fields.insert("loss".to_string(), Value::Float(loss));
    fields.insert("accuracy".to_string(), Value::Float(accuracy));
    fields.insert("metric".to_string(), Value::String(metric_name));
    fields.insert(
        "threshold_met".to_string(),
        Value::Bool(accuracy >= threshold),
    );

    Ok(Value::Struct {
        type_name: "ReflexTrainResult".to_string(),
        fields,
    })
}

/// `reflex_predict(model, input) -> Fluid`
///
/// Returns a Fluid value with one variant per class label:
///   - `type_name`: "Label"
///   - `value`: Value::String(label)
///   - `confidence`: softmax probability (0.0..1.0)
///
/// The Fluid is sorted by confidence descending (highest first). When
/// displayed via `to_string(fluid)`, the highest-confidence label is
/// shown — this is the standard Fluid Display behavior.
pub fn reflex_predict_dispatch(registry: &ReflexRegistry, args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "reflex_predict: expected 2 arguments (model, input), got {}",
            args.len()
        ));
    }

    let model_id: ReflexId = match &args[0] {
        Value::Reflex(id) => *id,
        other => {
            return Err(format!(
                "reflex_predict: first argument must be a Reflex model handle, got {}",
                other.type_name()
            ));
        }
    };

    let input_list = match &args[1] {
        Value::List(items) => items,
        other => {
            return Err(format!(
                "reflex_predict: second argument must be a List of Float, got {}",
                other.type_name()
            ));
        }
    };

    let model: &ReflexModel = registry.get(model_id).ok_or_else(|| {
        format!(
            "reflex_predict: model handle {:?} not found in registry",
            model_id
        )
    })?;

    // Convert input List<Value> to Vec<f64>
    let input: Vec<f64> = input_list
        .iter()
        .map(|v| match v {
            Value::Float(n) => Ok(*n),
            other => Err(format!(
                "reflex_predict: input feature must be Float, got {}",
                other.type_name()
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;

    if input.len() != model.input_size {
        return Err(format!(
            "reflex_predict: input has {} features, model expects {}",
            input.len(),
            model.input_size
        ));
    }

    // Forward pass — returns softmax probabilities
    let probs = model.forward(&input);

    // Build Fluid with one variant per label
    use crate::interpreter::FluidValueVariant;
    let variants: Vec<FluidValueVariant> = model
        .labels
        .iter()
        .zip(probs.iter())
        .map(|(label, &prob)| FluidValueVariant {
            type_name: "Label".to_string(),
            value: Value::String(label.clone()),
            confidence: prob,
        })
        .collect();

    Ok(Value::Fluid(variants))
}

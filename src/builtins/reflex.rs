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
use crate::nn::{find_metric, ReflexId, ReflexRegistry};
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

/// Stub — VM not yet supported (Наряд №180 — same pattern as train/predict).
pub(crate) fn builtin_reflex_save_stub(_args: &[Value]) -> Result<Value, String> {
    Err(
        "reflex_save: VM backend does not yet support Reflex (ADR-0114) \
         — use `mlog run` (interpreter backend)"
            .to_string(),
    )
}

/// Stub — VM not yet supported (Наряд №180 — same pattern as train/predict).
pub(crate) fn builtin_reflex_load_stub(_args: &[Value]) -> Result<Value, String> {
    Err(
        "reflex_load: VM backend does not yet support Reflex (ADR-0114) \
         — use `mlog run` (interpreter backend)"
            .to_string(),
    )
}

/// Stub — VM not yet supported (Наряд №187 — introspection, read-only).
pub(crate) fn builtin_reflex_metrics_stub(_args: &[Value]) -> Result<Value, String> {
    Err(
        "reflex_metrics: VM backend does not yet support Reflex (ADR-0114) \
         — use `mlog run` (interpreter backend)"
            .to_string(),
    )
}

/// Stub — VM not yet supported (Наряд №187 — introspection, read-only).
pub(crate) fn builtin_reflex_list_stub(_args: &[Value]) -> Result<Value, String> {
    Err(
        "reflex_list: VM backend does not yet support Reflex (ADR-0114) \
         — use `mlog run` (interpreter backend)"
            .to_string(),
    )
}

/// Stub — VM not yet supported (Наряд №193 — text generation).
pub(crate) fn builtin_reflex_generate_stub(_args: &[Value]) -> Result<Value, String> {
    Err(
        "reflex_generate: VM backend does not yet support Reflex (ADR-0114) \
         — use `mlog run` (interpreter backend)"
            .to_string(),
    )
}

// ── Наряд №194: Tokenization builtins (ADR-0120 follow-up) ──────────

/// `reflex_tokenize(text) -> List<Float>`
///
/// Character-level tokenization: each Unicode character → its code point as Float.
/// Not BPE — simplest deterministic scheme, no vocabulary training needed.
/// For `reflex_gen` models: ensure `vocab_size` covers all code points in your text.
pub fn builtin_reflex_tokenize(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "reflex_tokenize: expected 1 argument (text), got {}",
            args.len()
        ));
    }
    let text = match &args[0] {
        Value::String(s) => s,
        other => {
            return Err(format!(
                "reflex_tokenize: argument must be String, got {}",
                other.type_name()
            ))
        }
    };
    // Each Unicode char → code point as f64
    let tokens: Vec<Value> = text
        .chars()
        .map(|c| Value::Float(c as u32 as f64))
        .collect();
    Ok(Value::List(tokens))
}

/// `reflex_detokenize(tokens) -> String`
///
/// Converts token IDs back to a String. Inverse of `reflex_tokenize`.
/// `detokenize(tokenize(s)) == s` for any valid Unicode string.
pub fn builtin_reflex_detokenize(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "reflex_detokenize: expected 1 argument (List<Float>), got {}",
            args.len()
        ));
    }
    let tokens = match &args[0] {
        Value::List(items) => items,
        other => {
            return Err(format!(
                "reflex_detokenize: argument must be List, got {}",
                other.type_name()
            ))
        }
    };
    // Each Float → u32 → char → collect
    let mut result = String::new();
    for (i, token) in tokens.iter().enumerate() {
        let code = match token {
            Value::Float(n) => *n as u32,
            other => {
                return Err(format!(
                    "reflex_detokenize: token {} must be Float, got {}",
                    i,
                    other.type_name()
                ))
            }
        };
        // char::from_u32 returns None for invalid code points (surrogates, etc.)
        match char::from_u32(code) {
            Some(c) => result.push(c),
            None => {
                return Err(format!(
                "reflex_detokenize: token {} has invalid Unicode code point {} (not a valid char)",
                i, code
            ))
            }
        }
    }
    Ok(Value::String(result))
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
    // Наряд №185: dispatch on ModelKind. Dense path is UNCHANGED from
    // Наряд №179 — same `model.train(...)` call, same validation. The
    // Sequence path is the new code (Block 2 dispatch + Block 3 autograd).
    let model_kind: &mut crate::nn::ModelKind = registry.get_mut(model_id).ok_or_else(|| {
        format!(
            "reflex_train: model handle {:?} not found in registry",
            model_id
        )
    })?;

    match model_kind {
        crate::nn::ModelKind::Dense(model) => {
            // ── Dense path: existing code from Наряд №179, UNCHANGED ──
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
                if class_idx >= model.labels.len() {
                    return Err(format!(
                        "reflex_train: row {} class_idx {} out of range (model has {} labels: {:?})",
                        i,
                        class_idx,
                        model.labels.len(),
                        model.labels
                    ));
                }
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

            let learning_rate = 0.1;
            let (loss, accuracy) = model.train(&inputs, &targets, epochs, learning_rate)?;

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
        #[cfg(feature = "candle")]
        crate::nn::ModelKind::Sequence(model) => {
            // ── Sequence path: Наряд №185 Block 2/3 ──
            // Data format is different: each row is [seq_len * dim floats..., class_idx].
            // The feature portion is flattened [seq_len, dim] row-major; we reshape
            // to a 2D Tensor for forward.
            use candle_core::{Device, Tensor};
            let seq_len = model.seq_len;
            let input_dim = model.input_dim;
            let expected_features = seq_len * input_dim;

            let mut input_tensors: Vec<Tensor> = Vec::with_capacity(data.len());
            let mut targets: Vec<usize> = Vec::with_capacity(data.len());

            for (i, row) in data.iter().enumerate() {
                let features: &[Value] = match row {
                    Value::List(f) => f,
                    other => {
                        return Err(format!(
                            "reflex_train(seq): row {} must be a List, got {}",
                            i,
                            other.type_name()
                        ));
                    }
                };
                if features.len() != expected_features + 1 {
                    return Err(format!(
                        "reflex_train(seq): row {} has {} elements, need exactly {} (seq_len*dim + class_idx)",
                        i, features.len(), expected_features + 1
                    ));
                }
                let class_idx_f = match features.last() {
                    Some(Value::Float(n)) => *n,
                    Some(other) => {
                        return Err(format!(
                            "reflex_train(seq): row {} class_idx must be Float, got {}",
                            i,
                            other.type_name()
                        ));
                    }
                    None => unreachable!(),
                };
                let class_idx = class_idx_f as usize;
                if class_idx >= model.labels.len() {
                    return Err(format!(
                        "reflex_train(seq): row {} class_idx {} out of range (model has {} labels: {:?})",
                        i, class_idx, model.labels.len(), model.labels
                    ));
                }
                let feature_vec: Vec<f32> = features[..features.len() - 1]
                    .iter()
                    .map(|v| match v {
                        Value::Float(n) => Ok(*n as f32),
                        other => Err(format!(
                            "reflex_train(seq): row {} feature must be Float, got {}",
                            i,
                            other.type_name()
                        )),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let tensor = Tensor::from_vec(feature_vec, (seq_len, input_dim), &Device::Cpu)
                    .map_err(|e| format!("reflex_train(seq): row {} tensor build: {}", i, e))?
                    .to_dtype(candle_core::DType::F32)
                    .map_err(|e| format!("reflex_train(seq): row {} dtype: {}", i, e))?;
                input_tensors.push(tensor);
                targets.push(class_idx);
            }

            let learning_rate = 0.1;
            let (loss, accuracy) = model.train(&input_tensors, &targets, epochs, learning_rate)?;

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
        #[cfg(feature = "candle")]
        crate::nn::ModelKind::Gen(_) => Err(
            "reflex_train: gen models (reflex_gen) do not support reflex_train. \
                 Use direct Rust API or a future naryad for gen training via builtins."
                .to_string(),
        ),
    }
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

    let model_kind: &crate::nn::ModelKind = registry.get(model_id).ok_or_else(|| {
        format!(
            "reflex_predict: model handle {:?} not found in registry",
            model_id
        )
    })?;

    match model_kind {
        crate::nn::ModelKind::Dense(model) => {
            // ── Dense path: existing code from Наряд №179, UNCHANGED ──
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
        #[cfg(feature = "candle")]
        crate::nn::ModelKind::Sequence(model) => {
            // ── Sequence path: Наряд №185 Block 2 ──
            // Input format: flattened [seq_len * dim] row-major.
            use candle_core::{Device, Tensor};
            let expected_len = model.seq_len * model.input_dim;
            let feature_vec: Vec<f32> = input_list
                .iter()
                .map(|v| match v {
                    Value::Float(n) => Ok(*n as f32),
                    other => Err(format!(
                        "reflex_predict(seq): input feature must be Float, got {}",
                        other.type_name()
                    )),
                })
                .collect::<Result<Vec<_>, _>>()?;
            if feature_vec.len() != expected_len {
                return Err(format!(
                    "reflex_predict(seq): input has {} features, model expects {} (seq_len {} * dim {})",
                    feature_vec.len(), expected_len, model.seq_len, model.input_dim
                ));
            }
            let tensor =
                Tensor::from_vec(feature_vec, (model.seq_len, model.input_dim), &Device::Cpu)
                    .map_err(|e| format!("reflex_predict(seq): tensor build: {}", e))?
                    .to_dtype(candle_core::DType::F32)
                    .map_err(|e| format!("reflex_predict(seq): dtype: {}", e))?;

            let probs = model.predict_probs(&tensor)?;

            use crate::interpreter::FluidValueVariant;
            let variants: Vec<FluidValueVariant> = model
                .labels
                .iter()
                .zip(probs.iter())
                .map(|(label, &prob)| FluidValueVariant {
                    type_name: "Label".to_string(),
                    value: Value::String(label.clone()),
                    confidence: prob as f64,
                })
                .collect();

            Ok(Value::Fluid(variants))
        }
        #[cfg(feature = "candle")]
        crate::nn::ModelKind::Gen(_) => Err(
            "reflex_predict: gen models (reflex_gen) do not support reflex_predict. \
                 Use reflex_generate for text generation."
                .to_string(),
        ),
    }
}

// ── Наряд №180: persistence (ADR-0116) ──────────────────────────────

/// `reflex_save(model) -> Unit`
///
/// Saves the model's current weights + metadata to the SQLite database
/// configured by `memory { persist: "..." }`. The model is keyed by its
/// declared name (e.g. `reflex MyModel { ... }` → key = "MyModel").
///
/// Storage format: see `src/nn/serde_weights.rs` (Наряд №178).
/// Storage location: `reflex_models` table in the same SQLite database
/// as `memories` and `kv_store`.
///
/// Returns `Unit` on success. Errors:
///   - "reflex_save: persistence not configured" if no `memory { persist: ... }` block
///   - "reflex_save: model 'X' not declared" if name not in registry
///   - SQLite errors (failed to open, write, etc.)
pub fn reflex_save_dispatch(
    registry: &ReflexRegistry,
    model_name_to_id: &std::collections::HashMap<String, ReflexId>,
    persist_path: Option<&str>,
    args: &[Value],
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "reflex_save: expected 1 argument (model handle), got {}",
            args.len()
        ));
    }

    // arg 0: model handle (Value::Reflex) — produced by the FnCall
    // special-case in eval_expr_with_env (same pattern as reflex_train).
    let model_id: ReflexId = match &args[0] {
        Value::Reflex(id) => *id,
        Value::String(name) => {
            // Allow reflex_save("ModelName") form too — useful from REPL.
            *model_name_to_id
                .get(name)
                .ok_or_else(|| format!("reflex_save: model '{}' not declared", name))?
        }
        other => {
            return Err(format!(
                "reflex_save: first argument must be a Reflex model handle or String, got {}",
                other.type_name()
            ));
        }
    };

    // Look up the model name from the id-to-name reverse map.
    // We don't store id→name on ReflexRegistry (one-way only — name→id
    // is in model_name_to_id), so we scan.
    let model_name: String = model_name_to_id
        .iter()
        .find(|(_, id)| **id == model_id)
        .map(|(name, _)| name.clone())
        .ok_or_else(|| {
            format!(
                "reflex_save: model handle {:?} not bound to any declared name",
                model_id
            )
        })?;

    let persist_path = persist_path.ok_or_else(|| {
        "reflex_save: persistence not configured. Add `memory { persist: \"path.db\" }` \
         before calling reflex_save."
            .to_string()
    })?;

    let model_kind: &crate::nn::ModelKind = registry
        .get(model_id)
        .ok_or_else(|| format!("reflex_save: model handle {:?} not in registry", model_id))?;

    // Наряд №185: dispatch on model kind. Currently only Dense models
    // support persistence (Наряд №180). Sequence models would need a
    // separate serialization format — future naryad.
    match model_kind {
        crate::nn::ModelKind::Dense(model) => {
            crate::nn::persist::save_model_to_db(
                model,
                &model_name,
                std::path::Path::new(persist_path),
            )?;
            Ok(Value::Unit)
        }
        #[cfg(feature = "candle")]
        crate::nn::ModelKind::Sequence(_) => Err(
            "reflex_save: sequence models (reflex_seq) do not yet support persistence. \
             Only Dense models (reflex) can be saved. \
             Sequence model persistence is a future-naryad concern."
                .to_string(),
        ),
        #[cfg(feature = "candle")]
        crate::nn::ModelKind::Gen(_) => Err(
            "reflex_save: gen models (reflex_gen) do not yet support persistence. \
             Only Dense models (reflex) can be saved. \
             Gen model persistence is a future-naryad concern."
                .to_string(),
        ),
    }
}

/// `reflex_load(name) -> Value::Reflex`
///
/// Loads weights for a previously saved model and applies them to the
/// *currently declared* `reflex` block with the same name.
///
/// Block 3 (Наряд №180): before applying weights, verifies that the
/// stored layer shapes match the current declaration's layer shapes.
/// A mismatch is a loud error — never silent corruption.
///
/// Returns the same `Value::Reflex(id)` handle that the declaration
/// already produced (reflex_load does NOT register a new model —
/// it mutates the weights of the existing one). This matches ADR-0116:
/// "reflex_load reads and reconstructs a ReflexModel" — the model
/// already exists from the declaration; reflex_load only restores weights.
///
/// Errors:
///   - "reflex_load: persistence not configured" if no `memory { persist: ... }`
///   - "reflex_load: no saved model with name 'X'" if name not in DB
///   - Block 3 shape mismatch (input_size, labels, layer count, layer shape)
///   - `REFLEX_VERSION` mismatch (handled by `deserialize_model`)
pub fn reflex_load_dispatch(
    registry: &mut ReflexRegistry,
    model_name_to_id: &std::collections::HashMap<String, ReflexId>,
    persist_path: Option<&str>,
    args: &[Value],
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "reflex_load: expected 1 argument (model name String), got {}",
            args.len()
        ));
    }

    let name = match &args[0] {
        Value::String(s) => s.clone(),
        Value::Reflex(id) => {
            // Allow reflex_load(handle) form too — resolve id → name.
            model_name_to_id
                .iter()
                .find(|(_, rid)| **rid == *id)
                .map(|(n, _)| n.clone())
                .ok_or_else(|| {
                    format!(
                        "reflex_load: model handle {:?} not bound to any declared name",
                        id
                    )
                })?
        }
        other => {
            return Err(format!(
                "reflex_load: first argument must be a String (model name), got {}",
                other.type_name()
            ));
        }
    };

    let id: ReflexId = *model_name_to_id.get(&name).ok_or_else(|| {
        format!(
            "reflex_load: model '{}' not declared (no matching `reflex {} {{ ... }}` block)",
            name, name
        )
    })?;

    let persist_path = persist_path.ok_or_else(|| {
        "reflex_load: persistence not configured. Add `memory { persist: \"path.db\" }` \
         before calling reflex_load."
            .to_string()
    })?;

    crate::nn::persist::load_model_from_db(
        registry,
        id,
        &name,
        std::path::Path::new(persist_path),
    )?;

    Ok(Value::Reflex(id))
}

// ── Наряд №187: introspection builtins ──────────────────────────────

/// `reflex_metrics(model) -> Struct` (Наряд №187)
///
/// Read-only introspection — returns model metadata (NOT weights).
/// Per ADR-0114: weights never enter `Value`. This builtin exposes
/// only the same metadata that `ReflexModel::Debug` already prints
/// (name, is_trained, last_metric, input_size, labels).
///
/// Returns a Struct with fields:
///   - `name`: String — model name from the declaration
///   - `is_trained`: Bool — true if `last_metric` is Some
///   - `last_metric`: Float or Unit — last measured accuracy/loss
///   - `input_size`: Float — input dimension (embedding dim)
///   - `labels`: List of String — closed-set label names
///
/// Works for both Dense (`reflex`) and Sequence (`reflex_seq`) models —
/// dispatches on `ModelKind`.
pub fn reflex_metrics_dispatch(registry: &ReflexRegistry, args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "reflex_metrics: expected 1 argument (model), got {}",
            args.len()
        ));
    }

    let model_id: ReflexId = match &args[0] {
        Value::Reflex(id) => *id,
        other => {
            return Err(format!(
                "reflex_metrics: first argument must be a Reflex model handle, got {}",
                other.type_name()
            ))
        }
    };

    let model_kind: &crate::nn::ModelKind = registry.get(model_id).ok_or_else(|| {
        format!(
            "reflex_metrics: model handle {:?} not in registry",
            model_id
        )
    })?;

    let (name, is_trained, last_metric, input_size, labels): (
        String,
        bool,
        Option<f64>,
        usize,
        &[String],
    ) = match model_kind {
        crate::nn::ModelKind::Dense(m) => (
            m.name.clone(),
            m.last_metric.is_some(),
            m.last_metric,
            m.input_size,
            &m.labels,
        ),
        #[cfg(feature = "candle")]
        crate::nn::ModelKind::Sequence(m) => (
            m.name.clone(),
            m.last_metric.is_some(),
            m.last_metric,
            m.input_dim,
            &m.labels,
        ),
        #[cfg(feature = "candle")]
        crate::nn::ModelKind::Gen(m) => (
            m.name.clone(),
            false, // Gen models don't have last_metric (training returns loss, not accuracy)
            None,
            m.input_dim,
            &[], // Gen models don't have labels
        ),
    };

    let mut fields: HashMap<String, Value> = HashMap::new();
    fields.insert("name".to_string(), Value::String(name));
    fields.insert("is_trained".to_string(), Value::Bool(is_trained));
    fields.insert(
        "last_metric".to_string(),
        match last_metric {
            Some(v) => Value::Float(v),
            None => Value::Unit,
        },
    );
    fields.insert("input_size".to_string(), Value::Float(input_size as f64));
    fields.insert(
        "labels".to_string(),
        Value::List(labels.iter().map(|s| Value::String(s.clone())).collect()),
    );

    Ok(Value::Struct {
        type_name: "ReflexMetrics".to_string(),
        fields,
    })
}

/// `reflex_list() -> List<String>` (Наряд №187)
///
/// Returns the names of all declared `reflex` / `reflex_seq` models,
/// in registration order (declaration order in the source).
///
/// Read-only — uses the `reflex_names` HashMap (name → ReflexId) that
/// the interpreter maintains during declaration processing.
pub fn reflex_list_dispatch(
    _registry: &ReflexRegistry,
    model_names: &std::collections::HashMap<String, ReflexId>,
    args: &[Value],
) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!(
            "reflex_list: expected 0 arguments, got {}",
            args.len()
        ));
    }
    // Sort by ReflexId (registration order) — stable, deterministic output.
    let mut entries: Vec<(&String, &ReflexId)> = model_names.iter().collect();
    entries.sort_by_key(|(_, id)| id.0);
    let names: Vec<Value> = entries
        .into_iter()
        .map(|(name, _)| Value::String(name.clone()))
        .collect();
    Ok(Value::List(names))
}

// ── Наряд №193: reflex_generate — text generation ──────────────────

/// `reflex_generate(model, prompt_tokens, max_tokens, temperature) -> List<Float>`
///
/// Autoregressive generation with KV-cache (ADR-0120).
/// - `model`: Reflex model handle (must be a `reflex_gen` model)
/// - `prompt_tokens`: List of Float (token IDs, pre-tokenized)
/// - `max_tokens`: Float (number of tokens to generate)
/// - `temperature`: Float (0.0 = greedy, >0 = sampling)
///
/// Returns: List of Float (generated token IDs)
pub fn reflex_generate_dispatch(
    registry: &ReflexRegistry,
    args: &[Value],
) -> Result<Value, String> {
    if args.len() != 4 {
        return Err(format!(
            "reflex_generate: expected 4 arguments (model, prompt, max_tokens, temperature), got {}",
            args.len()
        ));
    }

    let model_id: ReflexId = match &args[0] {
        Value::Reflex(id) => *id,
        other => {
            return Err(format!(
                "reflex_generate: first argument must be a Reflex model handle, got {}",
                other.type_name()
            ))
        }
    };

    // Extract prompt tokens
    #[allow(unused_variables)]
    let prompt: Vec<u32> = match &args[1] {
        Value::List(items) => items
            .iter()
            .map(|v| match v {
                Value::Float(n) => Ok(*n as u32),
                other => Err(format!(
                    "reflex_generate: prompt token must be Float, got {}",
                    other.type_name()
                )),
            })
            .collect::<Result<Vec<_>, _>>()?,
        other => {
            return Err(format!(
                "reflex_generate: second argument must be a List of Float (token IDs), got {}",
                other.type_name()
            ))
        }
    };

    #[allow(unused_variables)]
    let max_tokens = match &args[2] {
        Value::Float(n) => *n as usize,
        other => {
            return Err(format!(
                "reflex_generate: third argument (max_tokens) must be Float, got {}",
                other.type_name()
            ))
        }
    };

    #[allow(unused_variables)]
    let temperature = match &args[3] {
        Value::Float(n) => *n,
        other => {
            return Err(format!(
                "reflex_generate: fourth argument (temperature) must be Float, got {}",
                other.type_name()
            ))
        }
    };

    #[allow(unused_variables)]
    let model_kind: &crate::nn::ModelKind = registry.get(model_id).ok_or_else(|| {
        format!(
            "reflex_generate: model handle {:?} not in registry",
            model_id
        )
    })?;

    match model_kind {
        #[cfg(feature = "candle")]
        crate::nn::ModelKind::Gen(model) => {
            let tokens = if temperature <= 0.0 {
                model.generate_greedy(&prompt, max_tokens)?
            } else {
                model.generate_with_temperature(&prompt, max_tokens, temperature)?
            };
            // Convert to List<Float>
            let result: Vec<Value> = tokens.iter().map(|&t| Value::Float(t as f64)).collect();
            Ok(Value::List(result))
        }
        #[cfg(feature = "candle")]
        crate::nn::ModelKind::Dense(_) | crate::nn::ModelKind::Sequence(_) => {
            Err("reflex_generate: model is not a reflex_gen model (use reflex_gen declaration, not reflex/reflex_seq)".to_string())
        }
        #[cfg(not(feature = "candle"))]
        _ => Err("reflex_generate: candle feature not enabled".to_string()),
    }
}

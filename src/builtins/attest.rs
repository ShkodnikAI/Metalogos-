//! `attest_verify` builtin (Наряд №825, ADR-0200 — W1).
//!
//! `attest_verify(model) -> Struct { ok, level, spec_hash, reason }`
//!
//! Read-only provenance verification of a declared Reflex model against
//! its stored certificate: re-serializes the LIVE weights (the hash must
//! match the certificate's `weights_sha256`), recomputes the spec hash
//! from the CURRENT declarations and compares the three hashes. `ok: true`
//! means provenance consistency — NEVER quality (decision 5; the
//! honest-limits register says so verbatim).
//!
//! Tier `experimental` (поправка (г), the №809 C3-a T1 gate): the row in
//! `builtins/registry.rs` carries the trailing tier literal and
//! `builtins_classification.rs` pins Source × Internal × Pure.
//!
//! The live dispatch is intercepted by BOTH backends before the registry
//! stub (the reflex-group shape): the TW expression path calls
//! `invoke_attest_verify` (needs the Mutex'd registry, the persist path
//! and the interpreter's learnable patterns); the VM calls the shared
//! `attest_verify_dispatch` from `call_reflex_builtin`. The stub stays
//! byte-for-byte as the shadowed fail-closed fallback (the №748 posture).

use std::collections::HashMap;

use crate::interpreter::Value;
use crate::nn::attest::{self, AttestInput, AttestPolicy};

/// The builtin name — spelled here and nowhere else outside the registry.
pub const NAME_ATTEST_VERIFY: &str = "attest_verify";

/// The shadowed fail-closed fallback — unreachable on both lanes (the
/// intercepts precede the registry row), the №748 stub posture.
pub(crate) fn builtin_attest_verify_stub(_args: &[Value]) -> Result<Value, String> {
    Err(
        "attest_verify: backend dispatch unavailable (the intercepts precede this stub on \
         both lanes — this is the fail-closed shadow, the №748 posture)"
            .to_string(),
    )
}

/// The shared verification dispatch. `attest` carries the caller's
/// pattern-side facts (the spec input + the live teaching examples); the
/// caller extracted them from its own compiled learnable pattern — TW and
/// VM build them with the same field mapping, the canonical form lives in
/// `nn::attest` so both lanes attest identically.
pub fn attest_verify_dispatch(
    registry: &crate::nn::ReflexRegistry,
    model_name_to_id: &std::collections::HashMap<String, crate::nn::ReflexId>,
    persist_path: Option<&str>,
    attest_input: Option<&AttestInput>,
    args: &[Value],
) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "attest_verify: expected 1 argument (model handle or name), got {}",
            args.len()
        ));
    }

    // The model name: handle (Value::Reflex) resolved through the reverse
    // map, or the plain String form (REPL convenience, the reflex_save
    // pattern).
    let model_name: String = match &args[0] {
        crate::interpreter::Value::Reflex(id) => model_name_to_id
            .iter()
            .find(|(_, rid)| **rid == *id)
            .map(|(name, _)| name.clone())
            .ok_or_else(|| {
                format!(
                    "attest_verify: model handle {:?} not bound to any declared name",
                    id
                )
            })?,
        crate::interpreter::Value::String(name) => {
            if !model_name_to_id.contains_key(name) {
                return Err(format!(
                    "attest_verify: model '{}' not declared (no matching `reflex {} {{ ... }}` block)",
                    name, name
                ));
            }
            name.clone()
        }
        other => {
            return Err(format!(
                "attest_verify: first argument must be a Reflex model handle or String, got {}",
                other.type_name()
            ));
        }
    };

    let persist_path = persist_path.ok_or_else(|| {
        "attest_verify: persistence not configured. Add `memory { persist: \"path.db\" }` \
         — the certificate lives in the same SQLite file as the weights."
            .to_string()
    })?;

    // Resolve the Dense model (W1: only Dense models attest — the same
    // boundary as persistence itself).
    let model_id = model_name_to_id
        .get(&model_name)
        .copied()
        .ok_or_else(|| format!("attest_verify: model '{}' not declared", model_name))?;
    let model_kind = registry
        .get(model_id)
        .ok_or_else(|| format!("attest_verify: model handle {:?} not in registry", model_id))?;
    // Resolve the Dense model (W1: only Dense models attest — the same
    // boundary as persistence itself). Without `candle` ModelKind carries
    // only the Dense variant (the infallible if-let form, the persist.rs
    // shape); with it, the other kinds refuse loud.
    #[cfg(feature = "candle")]
    let model = match model_kind {
        crate::nn::ModelKind::Dense(m) => m,
        _ => {
            return Err(format!(
                "attest_verify: model '{}' is not a Dense reflex model (only `reflex` models attest in W1)",
                model_name
            ));
        }
    };
    #[cfg(not(feature = "candle"))]
    let model = {
        let crate::nn::ModelKind::Dense(m) = model_kind;
        m
    };

    #[allow(clippy::disallowed_methods)]
    let conn = rusqlite::Connection::open(persist_path)
        .map_err(|e| format!("attest_verify: failed to open db '{}': {}", persist_path, e))?;

    let empty: Vec<(String, String)> = Vec::new();
    let default_spec = attest::AttestSpecInput::default();
    let (spec, examples) = match attest_input {
        Some(a) => (a.spec.clone(), a.examples.clone()),
        None => (default_spec, empty),
    };
    let current_spec = attest::canonical_spec_json(model, &spec);
    let s_hash = crate::vision::provenance::sha256_hex(current_spec.as_bytes());
    let blob = crate::nn::persist::serialize_for_storage(model);
    let verdict = attest::verify_attestation(
        &conn,
        &model_name,
        &blob,
        &model.feature_signature,
        &current_spec,
        &s_hash,
        &examples,
        spec.pattern_name.as_deref(),
        AttestPolicy::from_env(),
    )?;
    Ok(attest::verdict_to_struct(&verdict))
}

/// The TW helper: build the `AttestInput` for a model from the
/// interpreter's own compiled learnable patterns + teaching states. The
/// canonical field mapping — the VM builds the identical struct from
/// `CompiledLearnableInfo`.
#[allow(clippy::too_many_arguments)] // the field mapping IS the contract — a params struct would hide the per-field parity both lanes must keep identical
pub fn attest_input_from_learnable(
    pattern_name: &str,
    prompt: &str,
    reflex_name: &str,
    distill_after: usize,
    fallback_if: Option<(&str, f64)>,
    min_accuracy: f64,
    margin: f64,
    examples: Vec<(String, String)>,
) -> AttestInput {
    let _ = reflex_name;
    AttestInput {
        spec: attest::AttestSpecInput {
            pattern_name: Some(pattern_name.to_string()),
            prompt: Some(prompt.to_string()),
            distill_after: Some(distill_after),
            fallback_if: fallback_if.map(|(op, v)| (op.to_string(), v)),
            min_accuracy: Some(min_accuracy),
            margin: Some(margin),
        },
        examples,
    }
}

/// The operator word for the canonical spec — the TW side (AST enum).
pub fn compare_op_word(op: &crate::ast::CompareOp) -> &'static str {
    match op {
        crate::ast::CompareOp::Gt => "gt",
        crate::ast::CompareOp::Lt => "lt",
        crate::ast::CompareOp::Ge => "ge",
        crate::ast::CompareOp::Le => "le",
        crate::ast::CompareOp::Eq => "eq",
        crate::ast::CompareOp::Ne => "ne",
    }
}

/// The operator word for the canonical spec — the VM side (bytecode
/// enum, the same six operators spelled per enum).
pub fn condition_op_word(op: &crate::bytecode::ConditionOp) -> &'static str {
    match op {
        crate::bytecode::ConditionOp::Gt => "gt",
        crate::bytecode::ConditionOp::Lt => "lt",
        crate::bytecode::ConditionOp::Ge => "ge",
        crate::bytecode::ConditionOp::Le => "le",
        crate::bytecode::ConditionOp::Eq => "eq",
        crate::bytecode::ConditionOp::Ne => "ne",
    }
}

/// The Struct field helper used by tests and the CLI printer (no new
/// Value variants — ADR-0200 §6.4).
pub fn verdict_fields(verdict: &attest::AttestVerdict) -> HashMap<String, Value> {
    let mut fields: HashMap<String, Value> = HashMap::new();
    fields.insert("ok".to_string(), Value::Bool(verdict.ok));
    fields.insert("level".to_string(), Value::String(verdict.level.clone()));
    fields.insert(
        "spec_hash".to_string(),
        Value::String(verdict.spec_hash.clone()),
    );
    fields.insert("reason".to_string(), Value::String(verdict.reason.clone()));
    fields
}

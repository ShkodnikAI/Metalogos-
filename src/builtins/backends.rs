//! Backend registry builtins (Наряд №333, ADR-0163).
//!
//! `backend_list()` — read-only metadata over the static backend SSOT
//! (`src/backends.rs`): name, class, weights id, pin state, license
//! class + note. No weights bytes exist behind these entries (PARKED
//! №294); the listing is for tooling, docs, and in-program governance
//! checks. Stateless — no interception needed.

use crate::interpreter::Value;

/// `backend_list()` — the static backend registry as
/// `List[Struct { name, class, weights_id, pin, license, license_note }]`.
pub fn builtin_backend_list(args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!(
            "backend_list: expects 0 arguments, got {}",
            args.len()
        ));
    }
    let items = crate::backends::BACKEND_REGISTRY
        .iter()
        .map(|e| {
            let mut fields = std::collections::HashMap::new();
            fields.insert("name".to_string(), Value::String(e.name.to_string()));
            fields.insert(
                "class".to_string(),
                Value::String(e.class.as_str().to_string()),
            );
            fields.insert(
                "weights_id".to_string(),
                Value::String(e.weights_id.to_string()),
            );
            fields.insert("pin".to_string(), Value::String(e.pin.as_str().to_string()));
            fields.insert(
                "license".to_string(),
                Value::String(e.license.as_str().to_string()),
            );
            fields.insert(
                "license_note".to_string(),
                Value::String(e.license_note.to_string()),
            );
            Value::Struct {
                type_name: "BackendEntry".to_string(),
                fields,
            }
        })
        .collect();
    Ok(Value::List(items))
}

/// `backend_select(class, ladder)` — the backend try-chain (Наряд №336,
/// ADR-0165). Walks the ladder in priority order over the №333 registry
/// SSOT; every rung attempt is an audit event (stderr line + the
/// program-visible `attempts` list, №326 posture).
///
/// - Success → `Struct { type_name: "BackendSelected", ok: true, backend,
///   weights_id, mode, attempts }`.
/// - Exhaustion → `Degraded(t)`: `Struct { type_name: "Degraded",
///   ok: false, class: <t>, attempts, error: Struct{ code:
///   "BACKEND_DEGRADED", message } }` — a typed result, never a panic,
///   never a silent mock substitution (ADR-0165 §2.2).
/// - Shape errors (unknown class word, empty ladder, duplicate rungs,
///   non-String rungs) are loud Errs — catchable by `try` (ADR-0142).
pub(crate) fn builtin_backend_select(args: &[Value]) -> Result<Value, String> {
    const FN: &str = "backend_select";
    if args.len() != 2 {
        return Err(format!(
            "{}: expects 2 arguments (class, ladder), got {}",
            FN,
            args.len()
        ));
    }
    let class_word = match &args[0] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "{}: class must be String, got {}",
                FN,
                other.type_name()
            ))
        }
    };
    let class = crate::backends::BackendClass::parse(&class_word).ok_or_else(|| {
        format!(
            "{}: unknown backend class '{}' (available: stt, tts, omni, \
             vision-understanding, llm, ocr)",
            FN, class_word
        )
    })?;
    let rungs = match &args[1] {
        Value::List(items) => items,
        other => {
            return Err(format!(
                "{}: ladder must be List[String], got {}",
                FN,
                other.type_name()
            ))
        }
    };
    if rungs.is_empty() {
        return Err(format!(
            "{}: ladder is EMPTY — a ladder without rungs cannot fall back \
             (silent fallback forbidden, ADR-0165 §2.1)",
            FN
        ));
    }
    let mut names: Vec<String> = Vec::with_capacity(rungs.len());
    for r in rungs {
        match r {
            Value::String(s) => {
                if names.iter().any(|n| n == s) {
                    return Err(format!(
                        "{}: duplicate ladder rung '{}' — a rung tried twice \
                         is a contract bug, not a fallback",
                        FN, s
                    ));
                }
                names.push(s.clone());
            }
            other => {
                return Err(format!(
                    "{}: ladder rungs must be String, got {}",
                    FN,
                    other.type_name()
                ))
            }
        }
    }
    let real = !crate::voice::backend::mock_mode();
    // The attempt record: one audit surface per rung (the attempts list
    // is program-visible data; the stderr line is the op-log half).
    let mut attempts: Vec<Value> = Vec::with_capacity(names.len());
    let push_attempt = |backend: &str, status: &str, reason: &str, acc: &mut Vec<Value>| {
        eprintln!(
            "[BACKEND_SELECT] rung '{}' {} ({})",
            backend, status, reason
        );
        let mut f = std::collections::HashMap::new();
        f.insert("backend".to_string(), Value::String(backend.to_string()));
        f.insert("status".to_string(), Value::String(status.to_string()));
        f.insert("reason".to_string(), Value::String(reason.to_string()));
        acc.push(Value::Struct {
            type_name: "LadderAttempt".to_string(),
            fields: f,
        });
    };
    for name in &names {
        let entry = match crate::backends::find_by_name(name) {
            Some(e) => e,
            None => {
                push_attempt(
                    name,
                    "unavailable",
                    "no registry record (the №333 registry is the SSOT)",
                    &mut attempts,
                );
                continue;
            }
        };
        if entry.class != class {
            push_attempt(
                name,
                "unavailable",
                &format!(
                    "class mismatch: '{}' is '{}', ladder serves '{}'",
                    name,
                    entry.class.as_str(),
                    class.as_str()
                ),
                &mut attempts,
            );
            continue;
        }
        if real {
            // №334 contract: real mode requires the weights fetched and
            // SHA-verified. A PendingNo334 rung has NO manifest — it can
            // never be verified, and the refusal says so; a pinned rung
            // is unavailable until `backends::fetch_weights` verifies it
            // on disk (PARKED №294 in this environment — honest
            // exhaustion, never a mock substitution, ADR-0165 §2.3).
            let unavailable_reason = match entry.pin {
                crate::backends::ShaPin::PendingNo334 => {
                    "real mode: weights manifest pending (№334 sha-pin path) — \
                     rung cannot be verified"
                        .to_string()
                }
                crate::backends::ShaPin::Pinned(_) => {
                    let artifact = crate::backends_weights::first_manifest_file(entry.weights_id)
                        .map(|f| f.path.to_string())
                        .unwrap_or_else(|| "<no manifest>".to_string());
                    format!(
                        "real mode: weights ({}) not fetched and SHA-verified \
                         first (MLOG_BACKEND_WEIGHTS_ALLOWLIST + \
                         backends::fetch_weights); real inference is PARKED \
                         by hardware (№294) in this environment",
                        artifact
                    )
                }
            };
            push_attempt(name, "unavailable", &unavailable_reason, &mut attempts);
            continue;
        }
        // Mock mode: the deterministic mock-first contract (№334) makes
        // every registry rung of the requested class available; the mode
        // is visible in the result — never hidden (ADR-0165 §2.3).
        push_attempt(
            name,
            "selected",
            "mock mode — deterministic (№334 contract)",
            &mut attempts,
        );
        let mut f = std::collections::HashMap::new();
        f.insert("ok".to_string(), Value::Bool(true));
        f.insert("backend".to_string(), Value::String(entry.name.to_string()));
        f.insert(
            "weights_id".to_string(),
            Value::String(entry.weights_id.to_string()),
        );
        f.insert("mode".to_string(), Value::String("mock".to_string()));
        f.insert(
            "attempts".to_string(),
            Value::List(std::mem::take(&mut attempts)),
        );
        return Ok(Value::Struct {
            type_name: "BackendSelected".to_string(),
            fields: f,
        });
    }
    // Exhaustion → Degraded(t): typed, loud, inspectable. Not a panic,
    // not a downgrade (ADR-0165 §2.2). №385 (ADR-0169): the code is the
    // SAME frozen constant the try-classifier whitelists — one source of
    // truth, the typed result and the String-error contract cannot diverge.
    let mut err = std::collections::HashMap::new();
    err.insert(
        "code".to_string(),
        Value::String(crate::interpreter::values::CODE_BACKEND_DEGRADED.to_string()),
    );
    err.insert(
        "message".to_string(),
        Value::String(format!(
            "backend ladder exhausted: every rung for class '{}' is \
             unavailable ({} attempt(s) audited)",
            class.as_str(),
            attempts.len()
        )),
    );
    let mut f = std::collections::HashMap::new();
    f.insert("ok".to_string(), Value::Bool(false));
    f.insert(
        "class".to_string(),
        Value::String(class.as_str().to_string()),
    );
    f.insert("attempts".to_string(), Value::List(attempts));
    f.insert(
        "error".to_string(),
        Value::Struct {
            type_name: "TryError".to_string(),
            fields: err,
        },
    );
    Ok(Value::Struct {
        type_name: "Degraded".to_string(),
        fields: f,
    })
}

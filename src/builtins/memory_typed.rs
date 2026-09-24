// ── Naryad #350 (P1, feature/memory): typed-memory builtins ──────────
//
// The Language surface of the Memory<K> layer (state: src/memory_typed.rs).
// Arity/typing contract (registry SSOT — keep in sync with spec! rows):
//
//   memory_open(subject, label)            -> Memory   K ∈ {public, private};
//     private is consent-gated (active consent for "memory:<subject>",
//     №335) — MEMORY_CONSENT_REQUIRED otherwise
//   memory_put(handle, key, value, parents?) -> Unit    value: String|Secret|
//     Float|Bool (text form); parents: List<String> of EXISTING keys
//     (the №351 graph raw material) — MEMORY_UNKNOWN_PARENT otherwise
//   memory_read(handle, key)               -> String | Secret   THE AUDITED
//     READ SINK: public returns String; private returns Secret — the
//     lattice refuses its print/egress and redact() (№326) is the only
//     legal egress path; missing key — MEMORY_UNKNOWN_KEY
//   memory_keys(handle)                    -> List<String>   (audited)
//   memory_provenance(handle, key)         -> List<String>   (audited)
//   memory_export(handle, key, path)       -> String   THE FILE-EGRESS SINK:
//     public exports through the io sandbox; private REFUSES with
//     MEMORY_REDACT_REQUIRED — export the redact() output instead

use super::core::expect_string_arg;
use super::io::{open_sandbox_write, sandbox_path_ex, sandbox_violation, SandboxMode};
use crate::interpreter::values::Value;
use crate::memory_typed::{self, MemLabel};
use std::io::Write;

/// `memory_open(subject, label) -> Memory`
pub(crate) fn builtin_memory_open(args: &[Value]) -> Result<Value, String> {
    let fn_name = "memory_open";
    if args.len() != 2 {
        return Err(format!(
            "{}: expects 2 arguments (subject, label), got {}",
            fn_name,
            args.len()
        ));
    }
    let subject = expect_string_arg(fn_name, args, 0)?;
    let label_word = expect_string_arg(fn_name, args, 1)?;
    let label = MemLabel::parse(&label_word)?;
    memory_typed::open(&subject, label)
}

/// `memory_put(handle, key, value, parents?, opts?) -> Unit`
/// №445: the optional opts Struct `{priority?, decay_rate?, ttl_secs?}`
/// sets the activation attributes (the additive-arity precedent of
/// №280's include_forgotten — indices stay stable).
pub(crate) fn builtin_memory_put(args: &[Value]) -> Result<Value, String> {
    let fn_name = "memory_put";
    if args.len() < 3 || args.len() > 5 {
        return Err(format!(
            "{}: expects 3..5 arguments (handle, key, value, parents?, opts?), got {}",
            fn_name,
            args.len()
        ));
    }
    let handle_id = memory_typed::container_id_arg(fn_name, args, 0)?;
    let key = expect_string_arg(fn_name, args, 1)?;
    let text = memory_typed::value_to_text(fn_name, &args[2])?;
    let derived_from = match args.get(3) {
        None => Vec::new(),
        Some(Value::Unit) => Vec::new(),
        Some(Value::List(items)) => {
            let mut parents = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    Value::String(s) => parents.push(s.clone()),
                    other => {
                        return Err(format!(
                            "{}: derived-from parents must be List<String> of keys, got {} element",
                            fn_name,
                            other.type_name()
                        ))
                    }
                }
            }
            parents
        }
        Some(other) => {
            return Err(format!(
                "{}: expected List or Unit as parents, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    // №445: the optional activation attributes (a Struct with the
    // keys priority/decay_rate/ttl_secs; unknown keys are a loud
    // contract violation — silent typos would fake a TTL/priority).
    let opts = match args.get(4) {
        None => memory_typed::PutOpts::default(),
        Some(Value::Unit) => memory_typed::PutOpts::default(),
        Some(Value::Struct { fields, .. }) => {
            let mut o = memory_typed::PutOpts::default();
            for (k, v) in fields {
                match k.as_str() {
                    "priority" => {
                        let p = v
                            .as_float()
                            .map_err(|_| "memory_put: opts.priority must be a number".to_string())?;
                        o.priority = Some(p as f32);
                    }
                    "decay_rate" => {
                        let d = v
                            .as_float()
                            .map_err(|_| "memory_put: opts.decay_rate must be a number".to_string())?;
                        o.decay_rate = Some(d as f32);
                    }
                    "ttl_secs" => {
                        let t = v
                            .as_float()
                            .map_err(|_| "memory_put: opts.ttl_secs must be a number".to_string())?;
                        o.ttl_secs = Some(t);
                    }
                    other => {
                        return Err(format!(
                            "memory_put: unknown opts key '{}' (available: priority, decay_rate, ttl_secs)",
                            other
                        ))
                    }
                }
            }
            o
        }
        Some(other) => {
            return Err(format!(
                "{}: opts must be a Struct {{priority, decay_rate, ttl_secs}}, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    memory_typed::put(&handle_id, &key, &text, derived_from, opts)?;
    Ok(Value::Unit)
}

/// `memory_read(handle, key) -> String | Secret`
pub(crate) fn builtin_memory_read(args: &[Value]) -> Result<Value, String> {
    let fn_name = "memory_read";
    if args.len() != 2 {
        return Err(format!(
            "{}: expects 2 arguments (handle, key), got {}",
            fn_name,
            args.len()
        ));
    }
    let handle_id = memory_typed::container_id_arg(fn_name, args, 0)?;
    let key = expect_string_arg(fn_name, args, 1)?;
    memory_typed::read(&handle_id, &key)
}

/// `memory_keys(handle) -> List<String>`
pub(crate) fn builtin_memory_keys(args: &[Value]) -> Result<Value, String> {
    let fn_name = "memory_keys";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (handle), got {}",
            fn_name,
            args.len()
        ));
    }
    let handle_id = memory_typed::container_id_arg(fn_name, args, 0)?;
    let keys = memory_typed::keys(&handle_id)?;
    Ok(Value::List(keys.into_iter().map(Value::String).collect()))
}

/// `memory_provenance(handle, key) -> List<String>`
pub(crate) fn builtin_memory_provenance(args: &[Value]) -> Result<Value, String> {
    let fn_name = "memory_provenance";
    if args.len() != 2 {
        return Err(format!(
            "{}: expects 2 arguments (handle, key), got {}",
            fn_name,
            args.len()
        ));
    }
    let handle_id = memory_typed::container_id_arg(fn_name, args, 0)?;
    let key = expect_string_arg(fn_name, args, 1)?;
    let parents = memory_typed::provenance(&handle_id, &key)?;
    Ok(Value::List(
        parents.into_iter().map(Value::String).collect(),
    ))
}

/// `memory_export(handle, key, path) -> String`
pub(crate) fn builtin_memory_export(args: &[Value]) -> Result<Value, String> {
    let fn_name = "memory_export";
    if args.len() != 3 {
        return Err(format!(
            "{}: expects 3 arguments (handle, key, path), got {}",
            fn_name,
            args.len()
        ));
    }
    let handle_id = memory_typed::container_id_arg(fn_name, args, 0)?;
    let key = expect_string_arg(fn_name, args, 1)?;
    let path = expect_string_arg(fn_name, args, 2)?;
    // №445: the poison gate BEFORE the redact gate — a quarantined
    // entry never reaches the file egress and the refusal names the
    // quarantine (the stronger fact).
    if memory_typed::is_poisoned(&handle_id, &key)? {
        return Err(crate::interpreter::values::coded_error(
            crate::interpreter::values::CODE_MEMORY_POISONED,
            format!(
                "memory_export: entry '{}' is POISONED (a derived-from survivor of the forget cascade) — its content cannot materialize into any sink",
                key
            ),
        ));
    }
    // The redact gate BEFORE reading the plaintext: a private entry
    // never reaches the file egress in the clear (№326/ADR-0136).
    if memory_typed::label_of(&handle_id)? == MemLabel::Private {
        return Err(format!(
            "memory_export: entry '{}' is private — export the redact() output instead (MEMORY_REDACT_REQUIRED; №326/ADR-0136 is the only egress path for private content)",
            key
        ));
    }
    // The entry must exist (the core call also validates and journals).
    memory_typed::export(&handle_id, &key)?;
    let text = match memory_typed::read(&handle_id, &key)? {
        Value::String(s) => s,
        // Unreachable for public containers (read returns String), but
        // never leak: any non-String here aborts the export loudly.
        other => {
            return Err(format!(
                "memory_export: unexpected non-String read-out ({}) — aborted",
                other.type_name()
            ))
        }
    };
    if super::smfs::is_virtual(&path) {
        return Err(super::smfs::read_only_reject("memory_export", &path));
    }
    let safe_path = sandbox_path_ex(&path, SandboxMode::ForWrite).map_err(sandbox_violation)?;
    if let Some(parent) = safe_path.parent() {
        let _ = std::fs::create_dir_all(parent); // best-effort
    }
    let mut file = open_sandbox_write(&safe_path, false)?;
    file.write_all(text.as_bytes())
        .map_err(|e| format!("memory_export: write failed: {}", e))?;
    Ok(Value::String(path))
}

// ── Naryad #351 (ADR-0173 §3.6): the derived-graph surfaces ─────────
//
//   memory_cascade_preview(handle, key) -> Struct{closure, blocked_by}
//       the №280 dry-run discipline — read-only, no grant touched;
//       when blocked_by is empty the would-delete set IS the closure
//   memory_retain(handle, key)   -> Unit    pin the descendant closure
//       of the key (the CASCADE retain; reversible via release)
//   memory_release(handle, key)  -> Unit    unpin the descendant closure
//   memory_retained(handle)      -> List<String>   pinned keys
//   memory_forget_cascade(handle, key, grant) -> Struct{root, deleted,
//       batch_id}   the ADR-0155 linear action: GRANT_MISSING without a
//       grant; scope `memory:forget:<container_id>`; the retained VETO
//       (MEMORY_RETAIN_PROTECTED) refuses before anything is deleted;
//       the post-success ledger record `irreversible.memory_forget`

use crate::memory_typed::ForgetOutcome;

fn forget_outcome_value(o: &ForgetOutcome) -> Value {
    let fields: Vec<(&str, Value)> = vec![
        ("root", Value::String(o.root.clone())),
        (
            "deleted",
            Value::List(o.deleted.iter().cloned().map(Value::String).collect()),
        ),
        ("batch_id", Value::String(o.batch_id.clone())),
    ];
    super::core::make_struct("MemoryForgetResult", fields)
}

/// `memory_cascade_preview(handle, key) -> Struct`
pub(crate) fn builtin_memory_cascade_preview(args: &[Value]) -> Result<Value, String> {
    let fn_name = "memory_cascade_preview";
    if args.len() != 2 {
        return Err(format!(
            "{}: expects 2 arguments (handle, key), got {}",
            fn_name,
            args.len()
        ));
    }
    let handle_id = memory_typed::container_id_arg(fn_name, args, 0)?;
    let key = expect_string_arg(fn_name, args, 1)?;
    let (closure, blocked_by, _ops) = memory_typed::cascade_preview(&handle_id, &key)?;
    let fields: Vec<(&str, Value)> = vec![
        (
            "closure",
            Value::List(closure.into_iter().map(Value::String).collect()),
        ),
        (
            "blocked_by",
            Value::List(blocked_by.into_iter().map(Value::String).collect()),
        ),
    ];
    Ok(super::core::make_struct("MemoryCascadePlan", fields))
}

/// `memory_retain(handle, key) -> Unit`
pub(crate) fn builtin_memory_retain(args: &[Value]) -> Result<Value, String> {
    let fn_name = "memory_retain";
    if args.len() != 2 {
        return Err(format!(
            "{}: expects 2 arguments (handle, key), got {}",
            fn_name,
            args.len()
        ));
    }
    let handle_id = memory_typed::container_id_arg(fn_name, args, 0)?;
    let key = expect_string_arg(fn_name, args, 1)?;
    memory_typed::retain(&handle_id, &key)?;
    Ok(Value::Unit)
}

/// `memory_release(handle, key) -> Unit`
pub(crate) fn builtin_memory_release(args: &[Value]) -> Result<Value, String> {
    let fn_name = "memory_release";
    if args.len() != 2 {
        return Err(format!(
            "{}: expects 2 arguments (handle, key), got {}",
            fn_name,
            args.len()
        ));
    }
    let handle_id = memory_typed::container_id_arg(fn_name, args, 0)?;
    let key = expect_string_arg(fn_name, args, 1)?;
    memory_typed::release(&handle_id, &key)?;
    Ok(Value::Unit)
}

/// `memory_retained(handle) -> List<String>`
pub(crate) fn builtin_memory_retained(args: &[Value]) -> Result<Value, String> {
    let fn_name = "memory_retained";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (handle), got {}",
            fn_name,
            args.len()
        ));
    }
    let handle_id = memory_typed::container_id_arg(fn_name, args, 0)?;
    let keys = memory_typed::retained_keys(&handle_id)?;
    Ok(Value::List(keys.into_iter().map(Value::String).collect()))
}

/// `memory_forget_cascade(handle, key, grant) -> Struct`
pub(crate) fn builtin_memory_forget_cascade(args: &[Value]) -> Result<Value, String> {
    let fn_name = "memory_forget_cascade";
    if args.len() != 3 {
        return Err(format!(
            "{}: expects 3 arguments (handle, key, grant), got {}",
            fn_name,
            args.len()
        ));
    }
    let handle_id = memory_typed::container_id_arg(fn_name, args, 0)?;
    let key = expect_string_arg(fn_name, args, 1)?;
    let grant = match &args[2] {
        Value::Grant(h) => h.clone(),
        other => {
            return Err(format!(
                "GRANT_MISSING: {} requires a Grant as argument 3 (issue it with grant_issue(scope, ttl, class) — scope \"memory:forget:<container>\"), got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    let outcome = memory_typed::forget_cascade(&handle_id, &key, &grant)?;
    Ok(forget_outcome_value(&outcome))
}

/// `recall(query, min_confidence?) -> String` — the registry-level
/// recall (№442): the TYPED lane. This is the only state a bare
/// registry fn can reach; the TW/VM state-carrying blocks intercept by
/// name first and add their store lanes (the bug #530 twin pattern).
/// Contract (identical on every path):
///   - a query that names gated private memory (a private container's
///     key, no active grant) refuses fail-closed with the typed
///     MEMORY_RECALL_CONSENT_REQUIRED stamp and records
///     `memory.recall.denied`;
///   - otherwise the best typed hit is returned as its text plus the
///     `[MEM]` provenance suffix (container/subject/label/time/taint);
///   - every call leaves a `memory.recall` ledger record {query hash,
///     containers, hits, consent fact}.
pub(crate) fn builtin_recall(args: &[Value]) -> Result<Value, String> {
    let fn_name = "recall";
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "{}: expects 1..2 arguments (query, min_confidence?), got {}",
            fn_name,
            args.len()
        ));
    }
    let query = expect_string_arg(fn_name, args, 0)?;
    // The typed lane's deterministic scores (1.0/0.8/0.6) are
    // confidence-ordered; the gate keeps the same 0..=1 semantics as
    // the store lanes' threshold (mirrored by RECALL_CONFIDENCE_INVALID
    // at check time).
    let min_confidence = match args.get(1) {
        None => 0.0,
        Some(v) => {
            let mc = v
                .as_float()
                .map_err(|_| "recall: min_confidence must be a number".to_string())?;
            if !(0.0..=1.0).contains(&mc) {
                return Err(format!(
                    "recall: min_confidence {} is outside 0.0..=1.0",
                    mc
                ));
            }
            mc
        }
    };
    let lane = memory_typed::recall_lane(&query);
    if let Some((container_id, subject)) = lane.gated_key_matches.first() {
        memory_typed::ledger_recall_denied(&query, container_id);
        return Err(memory_typed::recall_consent_refusal(
            &query,
            container_id,
            subject,
        ));
    }
    let disclosed = lane
        .hits
        .iter()
        .filter(|h| h.score >= min_confidence as f32)
        .count();
    memory_typed::ledger_recall(&query, &lane, disclosed);
    match lane.hits.iter().find(|h| h.score >= min_confidence as f32) {
        Some(hit) => {
            let mut result = hit.text.clone();
            result.push_str(&memory_typed::recall_hit_provenance(hit));
            Ok(Value::String(result))
        }
        None => Ok(Value::String(String::new())),
    }
}

// ── Naryad #445: the forgetting memory — the language surface ────────
//
//   forget(handle, key, grant, dry_run?) -> Struct
//       the canon §10.3 front door: the ADR-0155 linear action with the
//       cascade refusal ladder (see memory_typed::forget_front — the
//       enforcement order, the poison semantics and the ledger family).
//       The TW/VM legacy `forget(query, days?)` intercepts (№72) keep
//       the 1..2-argument surface; 3..4 arguments fall through to THIS
//       handler on every backend (the parity by construction).
//   memory_retain_ttl(handle, key, ttl_secs) -> Unit
//       the canon retain(memory, ttl): the entry's lifetime; past the
//       deadline the sweep auto-forgets it (the №280 "v2" lifted).

/// `forget(handle, key, grant, dry_run?) -> Struct{dry_run, container,
/// root, deleted, poisoned, batch_id}`
pub(crate) fn builtin_forget(args: &[Value]) -> Result<Value, String> {
    let fn_name = "forget";
    if args.len() < 3 || args.len() > 4 {
        return Err(format!(
            "{}: expects 3..4 arguments (handle, key, grant, dry_run?), got {} — the legacy 1..2-argument form forget(query, days?) is unchanged",
            fn_name,
            args.len()
        ));
    }
    let handle_id = memory_typed::container_id_arg(fn_name, args, 0)?;
    let key = expect_string_arg(fn_name, args, 1)?;
    let grant = match &args[2] {
        Value::Grant(h) => h.clone(),
        other => {
            return Err(format!(
                "GRANT_MISSING: {} requires a Grant as argument 3 (issue it with grant_issue(scope, ttl, class) — scope \"memory:forget:<container>\"), got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    let dry_run = match args.get(3) {
        None => false,
        Some(Value::Bool(b)) => *b,
        Some(other) => {
            return Err(format!(
                "{}: dry_run must be a Bool, got {}",
                fn_name,
                other.type_name()
            ))
        }
    };
    let outcome = memory_typed::forget_front(&handle_id, &key, &grant, dry_run)?;
    let fields: Vec<(&str, Value)> = vec![
        ("dry_run", Value::Bool(outcome.dry_run)),
        ("container", Value::String(outcome.container)),
        ("root", Value::String(outcome.root)),
        (
            "deleted",
            Value::List(outcome.deleted.into_iter().map(Value::String).collect()),
        ),
        (
            "poisoned",
            Value::List(outcome.poisoned.into_iter().map(Value::String).collect()),
        ),
        ("batch_id", Value::String(outcome.batch_id)),
    ];
    Ok(super::core::make_struct("MemoryForgetResult", fields))
}

/// `memory_retain_ttl(handle, key, ttl_secs) -> Unit`
pub(crate) fn builtin_memory_retain_ttl(args: &[Value]) -> Result<Value, String> {
    let fn_name = "memory_retain_ttl";
    if args.len() != 3 {
        return Err(format!(
            "{}: expects 3 arguments (handle, key, ttl_secs), got {}",
            fn_name,
            args.len()
        ));
    }
    let handle_id = memory_typed::container_id_arg(fn_name, args, 0)?;
    let key = expect_string_arg(fn_name, args, 1)?;
    let ttl_secs = args
        .get(2)
        .and_then(|v| v.as_float().ok())
        .ok_or_else(|| format!("{}: ttl_secs must be a number", fn_name))?;
    if !ttl_secs.is_finite() || ttl_secs <= 0.0 {
        return Err(format!(
            "{}: ttl_secs must be a finite value > 0.0, got {}",
            fn_name, ttl_secs
        ));
    }
    memory_typed::retain_ttl(&handle_id, &key, ttl_secs as u64)?;
    Ok(Value::Unit)
}

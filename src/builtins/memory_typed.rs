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

/// `memory_put(handle, key, value, parents?) -> Unit`
pub(crate) fn builtin_memory_put(args: &[Value]) -> Result<Value, String> {
    let fn_name = "memory_put";
    if args.len() < 3 || args.len() > 4 {
        return Err(format!(
            "{}: expects 3..4 arguments (handle, key, value, parents?), got {}",
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
    memory_typed::put(&handle_id, &key, &text, derived_from)?;
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

// ── Naryad #393 (ADR-0167 §3.5): Action Ledger language surface ───────
//
// Builtins, NOT new AST nodes — the consent-component precedent (№335):
// the flow layer needs no new syntax, the record writes live in the
// action paths themselves (src/ledger.rs §3.4 hooks), and the surface
// below is introspection + egress only.
//
//   - `ledger_count()`   — in-process record count (a READ, not egress);
//   - `ledger_head()`    — the current head hash (designed to be
//     published out-of-band — the external anchor of ADR-0167 §7);
//   - `ledger_export(path)` — FILE EGRESS: the verifiable JSONL chain to
//     a sandboxed path, classified Sink (the `consent_ledger_export`
//     precedent — the №326 "policy as value" audit posture);
//   - `ledger_export_intoto(path)` — FILE EGRESS: the in-toto Statement
//     profile (ADR-0157), same Sink classification;
//   - `ledger_rotate()`  — append a key-rotation record (signed by the
//     still-active key); returns the new key id;
//   - `ledger_snapshot()` — append a snapshot record pinning the head;
//     returns the snapshot record hash (the archive anchor).

use crate::interpreter::values::Value;

/// `ledger_count()` — number of records in the process-local journal.
pub(crate) fn builtin_ledger_count(args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!(
            "ledger_count: expects 0 arguments, got {}",
            args.len()
        ));
    }
    let n = crate::ledger::count()?;
    Ok(Value::Float(n as f64))
}

/// `ledger_head()` — the current head hash ("" for an empty journal).
pub(crate) fn builtin_ledger_head(args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!(
            "ledger_head: expects 0 arguments, got {}",
            args.len()
        ));
    }
    Ok(Value::String(crate::ledger::head_hash()?))
}

/// Shared egress path for the two export profiles: sandboxed write (the
/// consent_ledger_export posture — FILE EGRESS, classified Sink).
fn export_to(path: &str, content: String, fn_name: &str) -> Result<Value, String> {
    let safe_path =
        crate::builtins::io::sandbox_path_ex(path, crate::builtins::io::SandboxMode::ForWrite)
            .map_err(crate::builtins::io::sandbox_violation)?;
    std::fs::write(&safe_path, content.as_bytes())
        .map_err(|e| format!("{}: cannot write {}: {}", fn_name, safe_path.display(), e))?;
    Ok(Value::String(safe_path.display().to_string()))
}

/// `ledger_export(path)` — dump the verifiable JSONL chain to a sandboxed
/// path. FILE EGRESS — classified Sink, audited. Returns the written path.
pub(crate) fn builtin_ledger_export(args: &[Value]) -> Result<Value, String> {
    const FN_NAME: &str = "ledger_export";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (path), got {}",
            FN_NAME,
            args.len()
        ));
    }
    let path = match &args[0] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "{}: path must be String, got {}",
                FN_NAME,
                other.type_name()
            ))
        }
    };
    let jsonl = crate::ledger::export_jsonl()?;
    export_to(&path, jsonl, FN_NAME)
}

/// `ledger_export_intoto(path)` — dump the in-toto Statement profile
/// (ADR-0157) to a sandboxed path. FILE EGRESS — classified Sink.
pub(crate) fn builtin_ledger_export_intoto(args: &[Value]) -> Result<Value, String> {
    const FN_NAME: &str = "ledger_export_intoto";
    if args.len() != 1 {
        return Err(format!(
            "{}: expects 1 argument (path), got {}",
            FN_NAME,
            args.len()
        ));
    }
    let path = match &args[0] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "{}: path must be String, got {}",
                FN_NAME,
                other.type_name()
            ))
        }
    };
    let stream = crate::ledger::export_intoto()?;
    export_to(&path, stream, FN_NAME)
}

/// `ledger_rotate()` — append a key-rotation record (signed by the
/// still-active key); returns the NEW key id. Subsequent records are
/// signed by the fresh key (signer continuity, ADR-0167 §3.2).
pub(crate) fn builtin_ledger_rotate(args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!(
            "ledger_rotate: expects 0 arguments, got {}",
            args.len()
        ));
    }
    let key_id = crate::ledger::rotate()?;
    Ok(Value::String(key_id))
}

/// `ledger_snapshot()` — append a snapshot record pinning the head;
/// returns the snapshot record hash (the `mlog ledger archive` anchor).
pub(crate) fn builtin_ledger_snapshot(args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!(
            "ledger_snapshot: expects 0 arguments, got {}",
            args.len()
        ));
    }
    let hash = crate::ledger::snapshot()?;
    Ok(Value::String(hash))
}

//! №466 (gh#687) — the db transfer group: the shared live module.
//!
//! The second group of the TW/VM dedup (gate gh#680, decision 4-A, step 3;
//! the CI threshold gate is №462/gh#683, the diff-fuzzer is №465/gh#686).
//! The seven db builtin names — `db_execute`, `db_execute_with_grant`,
//! `db_insert`, `query`, `query_param`, `query_row`, `query_scalar` —
//! moved OUT of both backends: `src/vm.rs` and `src/interpreter/` keep
//! their exact per-site marshaling hooks (const-name checks in the SAME
//! dispatch order as before) and the bodies live here. After the move
//! the name literals appear only in this module, so the №462 counter
//! drops 56 → 49.
//!
//! The module is the shared HOME, not a unification: the TW and the VM
//! db lanes stay deliberately separate where their behavior differs —
//! the TW returns the affected-row STRING from `db_execute` while the VM
//! returns `Unit`; the VM stringifies `query_row` params while the TW
//! binds them typed (the №381 contract); the second-argument error text
//! of `db_insert` differs by one suffix; the VM lazily opens the
//! connection on first use (№409) while the TW opens at declaration
//! time. Every per-backend form below is a verbatim transplant; the
//! genuinely shared pieces factored here once are the `query_param`
//! parse/return shape (injected context lookup) and the grant-use note
//! format. The divergences the №465 fuzzer pinned stay pinned — fixes
//! land as separate owner-gated naryads, never silently inside a
//! transfer.
//!
//! The live-contract requirement of the naryad (the owner's
//! "revive-or-delete" strengthening for the dead `RuntimeContext`) is
//! satisfied the same way as group 1: the dead stub is already gone
//! (№465) and the VM state is reached through the `VmDbAccess` trait
//! below — a live, mock-testable contract; `Vm` is one implementor.

use crate::interpreter::db::{convert_params, sql_err};
use crate::interpreter::Value;
use std::collections::HashMap;
use std::sync::Mutex;

/// The db-group names this module owns. The backends compare their
/// dispatch names against the constants below — the name strings are
/// spelled here and nowhere else outside the registry.
pub const NAME_QUERY: &str = "query";
pub const NAME_QUERY_PARAM: &str = "query_param";
pub const NAME_QUERY_ROW: &str = "query_row";
pub const NAME_QUERY_SCALAR: &str = "query_scalar";
pub const NAME_DB_EXECUTE: &str = "db_execute";
pub const NAME_DB_EXECUTE_WITH_GRANT: &str = "db_execute_with_grant";
pub const NAME_DB_INSERT: &str = "db_insert";

/// The deny-surface sink label for the granted destructive-SQL action
/// (`on_deny` arguments and the hooks write-builtin list): the same
/// string as the builtin name, named separately so the semantic role
/// survives future renames.
pub const SINK_DB_EXECUTE_WITH_GRANT: &str = NAME_DB_EXECUTE_WITH_GRANT;

/// The №393 irreversible-action ledger event (the success-path journal
/// entry of the granted destructive SQL).
pub const LEDGER_IRREVERSIBLE_DB_EXECUTE: &str = "irreversible.db_execute";

/// The grant-use audit note prefix (both backends).
pub fn grant_use_note(sql: &str) -> String {
    format!("{}: {}", NAME_DB_EXECUTE_WITH_GRANT, sql)
}

/// The seven db-group names this module owns, as a single hook.
pub fn handles(name: &str) -> bool {
    matches!(
        name,
        NAME_QUERY
            | NAME_QUERY_PARAM
            | NAME_QUERY_ROW
            | NAME_QUERY_SCALAR
            | NAME_DB_EXECUTE
            | NAME_DB_EXECUTE_WITH_GRANT
            | NAME_DB_INSERT
    )
}

// ─────────────────────────────────────────────────────────────────────
// The shared query_param shape.
//
// Both backends parse the first argument (non-String → empty name) and
// return the looked-up value or the empty string. The context access is
// injected: the TW passes its `get_server_query_param` accessor, the VM
// reads its own `server_query_params` map — the parse/return shape is
// the genuinely shared piece (the behavior is identical on both sides).
// ─────────────────────────────────────────────────────────────────────

/// `query_param(name)` — the server-context query parameter.
/// Missing context or a missed key yields the empty string (parity
/// contract since №40); a non-String first argument degrades to the
/// empty name (the historical shape on both backends).
pub fn query_param(
    args: &[Value],
    lookup: impl FnOnce(&str) -> Option<String>,
) -> Result<Value, String> {
    let param_name = args
        .first()
        .and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_default();
    if let Some(val) = lookup(&param_name) {
        return Ok(Value::String(val));
    }
    Ok(Value::String(String::new()))
}

// ─────────────────────────────────────────────────────────────────────
// The TW lane (tree-walking backend).
//
// Verbatim transplants of the former `Interpreter::invoke_*` methods
// (src/interpreter/db.rs) and the inline `db_insert` body
// (src/interpreter/execution.rs). The db connection is the
// interpreter's `Arc<Mutex<Option<Connection>>>` state passed by
// reference — no interpreter borrow escapes the call sites.
// ─────────────────────────────────────────────────────────────────────

/// `query(sql, params?)` — TW. SELECT/PRAGMA → List of Struct ("Row"),
/// everything else → the affected-row count as a String (Наряд №7).
pub fn query_tw(db: &Mutex<Option<rusqlite::Connection>>, args: &[Value]) -> Result<Value, String> {
    let sql = match args.first() {
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(format!(
                "query() expected String SQL, got {}",
                other.type_name()
            ))
        }
        None => return Err("query() requires at least 1 argument (SQL string)".to_string()),
    };
    // Наряд №99: convert_params — type-safe, no silent shift/degradation
    let params: Vec<rusqlite::types::Value> = if args.len() > 1 {
        match &args[1] {
            Value::List(items) => convert_params(items)?,
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    };

    let guard = db.lock().map_err(|e| format!("db lock error: {}", e))?;
    let conn = guard.as_ref().ok_or_else(|| {
        "query() error: no database connection. Declare db { url: \"sqlite::memory:\" } first."
            .to_string()
    })?;

    let sql_upper = sql.trim().to_uppercase();
    if sql_upper.starts_with("SELECT") || sql_upper.starts_with("PRAGMA") {
        // SELECT/PRAGMA → List of Struct
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| sql_err("query() SQL error", e))?;
        let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
        let rows: Vec<Value> = stmt
            .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                let mut fields = std::collections::HashMap::new();
                for (i, col) in col_names.iter().enumerate() {
                    let val: Value = match row.get_ref(i) {
                        Ok(rusqlite::types::ValueRef::Null) => Value::Unit,
                        Ok(rusqlite::types::ValueRef::Integer(n)) => {
                            // Heuristic: if the column name suggests it's an ID or count, keep as Float
                            Value::Float(n as f64)
                        }
                        Ok(rusqlite::types::ValueRef::Real(f)) => Value::Float(f),
                        Ok(rusqlite::types::ValueRef::Text(s)) => {
                            Value::String(String::from_utf8_lossy(s).to_string())
                        }
                        Ok(rusqlite::types::ValueRef::Blob(b)) => {
                            // Encode blobs as hex strings
                            Value::String(b.iter().map(|byte| format!("{:02x}", byte)).collect())
                        }
                        Err(_) => Value::Unit,
                    };
                    fields.insert(col.clone(), val);
                }
                Ok(Value::Struct {
                    type_name: "Row".to_string(),
                    fields,
                })
            })
            .map_err(|e| sql_err("query() execution error", e))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(Value::List(rows))
    } else {
        // INSERT/UPDATE/DELETE/CREATE/ALTER/etc. → affected row count as String
        let affected = conn
            .execute(&sql, rusqlite::params_from_iter(params.iter()))
            .map_err(|e| sql_err("query() SQL error", e))?;
        Ok(Value::String(affected.to_string()))
    }
}

/// `db_execute(sql, params?)` — TW. ADR-0068: optional second argument
/// (List) for parameterised statements; returns the affected-row count
/// as a String (Наряд №7).
pub fn db_execute_tw(
    db: &Mutex<Option<rusqlite::Connection>>,
    args: &[Value],
) -> Result<Value, String> {
    let sql = match args.first() {
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(format!(
                "db_execute() expected String SQL, got {}",
                other.type_name()
            ))
        }
        None => return Err("db_execute() requires at least 1 argument (SQL string)".to_string()),
    };
    // Наряд №99: convert_params — type-safe, no silent empty-string degradation
    let params: Vec<rusqlite::types::Value> = match args.get(1) {
        Some(Value::List(items)) => convert_params(items)?,
        Some(other) => {
            return Err(format!(
                "db_execute() second argument must be List, got {}",
                other.type_name()
            ))
        }
        None => Vec::new(),
    };
    let guard = db.lock().map_err(|e| format!("db lock error: {}", e))?;
    let conn = guard.as_ref().ok_or_else(|| {
        "db_execute() error: no database connection. Declare db { url: \"sqlite::memory:\" } first."
            .to_string()
    })?;
    let affected = conn
        .execute(&sql, rusqlite::params_from_iter(params.iter()))
        .map_err(|e| sql_err("db_execute() SQL error", e))?;
    Ok(Value::String(affected.to_string()))
}

/// `db_execute_with_grant(g, sql, params?)` — TW. Naryad #390
/// (ADR-0155 §3.3 rule 6): the granted destructive-SQL action. Gates at
/// runtime, in order: ledger state (active / not-consumed /
/// not-revoked), TTL, SCOPE coverage of the SQL's destructive ops
/// (GRANT_SCOPE_MISMATCH), then execute, then consume (Once ->
/// consumed, N(n) -> decrement, Unlimited -> audited event).
/// Non-destructive SQL under a grant executes WITHOUT consumption
/// (nothing irreversible happened) and still writes the event.
pub fn db_execute_with_grant_tw(
    db: &Mutex<Option<rusqlite::Connection>>,
    args: &[Value],
) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err(format!(
            "{}: expects 2..3 arguments (grant, sql, params?), got {}",
            NAME_DB_EXECUTE_WITH_GRANT,
            args.len()
        ));
    }
    let handle = match &args[0] {
        Value::Grant(h) => h.clone(),
        other => {
            return Err(format!(
                "{}: first argument must be a Grant, got {}",
                NAME_DB_EXECUTE_WITH_GRANT,
                other.type_name()
            ))
        }
    };
    let sql = match &args[1] {
        Value::String(s) => s.clone(),
        other => {
            return Err(format!(
                "{}: second argument must be String SQL, got {}",
                NAME_DB_EXECUTE_WITH_GRANT,
                other.type_name()
            ))
        }
    };
    let params: Vec<rusqlite::types::Value> = match args.get(2) {
        Some(Value::List(items)) => convert_params(items)?,
        Some(other) => {
            return Err(format!(
                "{}: third argument must be List, got {}",
                NAME_DB_EXECUTE_WITH_GRANT,
                other.type_name()
            ))
        }
        None => Vec::new(),
    };
    // Gate BEFORE execution: state/TTL (check_active) + scope coverage
    // of every destructive op in the statement.
    crate::grants::check_active(&handle)?;
    let ops = crate::grants::extract_destructive_ops(&sql);
    let destructive = !ops.is_empty();
    for (op, table) in &ops {
        if !crate::grants::scope_covers(&handle.scope, op, table) {
            return Err(format!(
                "GRANT_SCOPE_MISMATCH: grant {} ({}, scope '{}') does not cover {} {}",
                handle.grant_id, handle.class, handle.scope, op, table
            ));
        }
    }
    let guard = db.lock().map_err(|e| format!("db lock error: {}", e))?;
    let conn = guard.as_ref().ok_or_else(|| {
        "db_execute_with_grant() error: no database connection. Declare db { url: \"sqlite::memory:\" } first."
            .to_string()
    })?;
    let affected = conn
        .execute(&sql, rusqlite::params_from_iter(params.iter()))
        .map_err(|e| sql_err("db_execute_with_grant() SQL error", e))?;
    drop(guard);
    // Post-success consumption/audit (never on SQL failure).
    if destructive {
        crate::grants::grant_use(&handle, &grant_use_note(&sql))?;
        // ── Naryad #393 (ADR-0167 §3.4): the irreversible action
        // SUCCEEDED — the journal entry is a side effect of the
        // success path itself (not a separate call the caller could
        // forget). Best-effort, loud on failure; the SQL preimage
        // never enters the journal — only its SHA-256.
        crate::ledger::record(
            LEDGER_IRREVERSIBLE_DB_EXECUTE,
            &handle.issuer,
            &handle.scope,
            &format!("{}|{}|{}", handle.grant_id, handle.scope, sql),
        );
        eprintln!(
            "[GRANT_USE] grant (scope '{}', class {}) executed {} (affected {}) — remaining {}",
            handle.scope,
            handle.class,
            sql.trim(),
            affected,
            crate::grants::state_of(&handle.grant_id)
                .map(|(_, r)| r)
                .unwrap_or(-1)
        );
    } else {
        eprintln!(
            "[GRANT_USE] grant (scope '{}') ran non-destructive SQL — no consumption",
            handle.scope
        );
    }
    Ok(Value::String(affected.to_string()))
}

/// `query_scalar(sql, params?)` — TW. Наряда-26 P1-7: executes a SELECT
/// that returns exactly one row with one column; the scalar value
/// directly (String, Float, or Unit for NULL).
pub fn query_scalar_tw(
    db: &Mutex<Option<rusqlite::Connection>>,
    args: &[Value],
) -> Result<Value, String> {
    let sql = match args.first() {
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(format!(
                "query_scalar() expected String SQL, got {}",
                other.type_name()
            ))
        }
        None => return Err("query_scalar() requires at least 1 argument (SQL string)".to_string()),
    };
    // Наряд №99: convert_params — type-safe, no silent shift
    let params: Vec<rusqlite::types::Value> = if args.len() > 1 {
        match &args[1] {
            Value::List(items) => convert_params(items)?,
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    };

    let guard = db.lock().map_err(|e| format!("db lock error: {}", e))?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "query_scalar() error: no database connection.".to_string())?;

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| sql_err("query_scalar() SQL error", e))?;
    let mut rows = stmt
        .query_map(rusqlite::params_from_iter(params.iter()), |row| {
            row.get_ref(0).map(|v| match v {
                rusqlite::types::ValueRef::Null => Value::Unit,
                rusqlite::types::ValueRef::Integer(n) => Value::Float(n as f64),
                rusqlite::types::ValueRef::Real(f) => Value::Float(f),
                rusqlite::types::ValueRef::Text(s) => {
                    Value::String(String::from_utf8_lossy(s).to_string())
                }
                rusqlite::types::ValueRef::Blob(b) => {
                    Value::String(b.iter().map(|byte| format!("{:02x}", byte)).collect())
                }
            })
        })
        .map_err(|e| sql_err("query_scalar() execution error", e))?;

    match rows.next() {
        Some(Ok(val)) => Ok(val),
        Some(Err(e)) => Err(sql_err("query_scalar() row error", e)),
        None => Ok(Value::Unit),
    }
}

/// `query_row(sql, params?)` — TW. Наряда-26 P1-7: executes a SELECT
/// that returns exactly one row; a List of column values (preserving
/// column order). Params bind TYPED (the №381 convert_params contract).
pub fn query_row_tw(
    db: &Mutex<Option<rusqlite::Connection>>,
    args: &[Value],
) -> Result<Value, String> {
    let sql = match args.first() {
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(format!(
                "query_row() expected String SQL, got {}",
                other.type_name()
            ))
        }
        None => return Err("query_row() requires at least 1 argument (SQL string)".to_string()),
    };
    // Наряд №99: convert_params — type-safe, no silent shift
    let params: Vec<rusqlite::types::Value> = if args.len() > 1 {
        match &args[1] {
            Value::List(items) => convert_params(items)?,
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    };

    let guard = db.lock().map_err(|e| format!("db lock error: {}", e))?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| "query_row() error: no database connection.".to_string())?;

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| sql_err("query_row() SQL error", e))?;
    let col_count = stmt.column_count();
    let mut rows = stmt
        .query_map(rusqlite::params_from_iter(params.iter()), |row| {
            let mut vals = Vec::with_capacity(col_count);
            for i in 0..col_count {
                let val = match row.get_ref(i) {
                    Ok(rusqlite::types::ValueRef::Null) => Value::Unit,
                    Ok(rusqlite::types::ValueRef::Integer(n)) => Value::Float(n as f64),
                    Ok(rusqlite::types::ValueRef::Real(f)) => Value::Float(f),
                    Ok(rusqlite::types::ValueRef::Text(s)) => {
                        Value::String(String::from_utf8_lossy(s).to_string())
                    }
                    Ok(rusqlite::types::ValueRef::Blob(b)) => {
                        Value::String(b.iter().map(|byte| format!("{:02x}", byte)).collect())
                    }
                    Err(_) => Value::Unit,
                };
                vals.push(val);
            }
            Ok(vals)
        })
        .map_err(|e| sql_err("query_row() execution error", e))?;

    match rows.next() {
        Some(Ok(vals)) => Ok(Value::List(vals)),
        Some(Err(e)) => Err(sql_err("query_row() row error", e)),
        None => Ok(Value::List(vec![])),
    }
}

/// `db_insert(table, struct)` — TW (the inline execution.rs body).
/// Inserts the struct fields as one row, returns the last-inserted
/// rowid as a Float. The TW second-argument error text carries the
/// literal-shape suffix — verbatim (the VM text differs; each side
/// keeps its own).
pub fn db_insert_tw(
    db: &Mutex<Option<rusqlite::Connection>>,
    args: &[Value],
) -> Result<Value, String> {
    let table = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Err(
                "db_insert() expects first argument to be a table name (String)".to_string(),
            )
        }
    };
    let fields = match args.get(1) {
        Some(Value::Struct { fields, .. }) => fields.clone(),
        _ => {
            return Err(
                "db_insert() expects second argument to be a Struct { field: value, ... }"
                    .to_string(),
            )
        }
    };
    let guard = db.lock().map_err(|e| format!("db lock error: {}", e))?;
    let conn = guard.as_ref().ok_or_else(|| {
        "db_insert() error: no database connection. Declare db { url: \"sqlite::memory:\" } first."
            .to_string()
    })?;
    let col_names: Vec<String> = fields.keys().cloned().collect();
    let placeholders: Vec<String> = col_names.iter().map(|_| "?".to_string()).collect();
    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        table,
        col_names.join(", "),
        placeholders.join(", ")
    );
    let params: Vec<Box<dyn rusqlite::types::ToSql>> = fields
        .values()
        .map(|v| match v {
            Value::String(s) => Box::new(s.clone()) as Box<dyn rusqlite::types::ToSql>,
            Value::Float(f) => Box::new(*f) as Box<dyn rusqlite::types::ToSql>,
            Value::Bool(b) => Box::new(*b) as Box<dyn rusqlite::types::ToSql>,
            Value::Unit => Box::new(Option::<String>::None) as Box<dyn rusqlite::types::ToSql>,
            other => Box::new(format!("{}", other)) as Box<dyn rusqlite::types::ToSql>,
        })
        .collect();
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    conn.execute(&sql, param_refs.as_slice())
        .map_err(|e| sql_err("db_insert() SQL error", e))?;
    // Return last inserted rowid
    let rowid: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |row| row.get(0))
        .unwrap_or(0);
    Ok(Value::Float(rowid as f64))
}

// ─────────────────────────────────────────────────────────────────────
// The VM lane (register/bytecode backend).
//
// Verbatim transplants of the former inline vm.rs bodies. The VM state
// (the lazily-opened connection, the server query params) is reached
// through the `VmDbAccess` trait — the live contract that replaces the
// deleted dead `RuntimeContext` stub (№465); `Vm` implements it below
// in src/vm.rs. The №409 lazy open fires exactly where the inline
// bodies called it (every statement builtin, NOT query_param).
// ─────────────────────────────────────────────────────────────────────

/// The VM-side db contract: the lazy-open hook (№409), the live
/// connection, and the per-request server query params. Mock-testable
/// (the unit tests below drive every VM function through a mock).
pub trait VmDbAccess {
    /// №409: materialize the connection on first use (no-op when the
    /// connection is already open or a previous open attempt failed).
    fn ensure_db_open(&mut self);
    /// The live connection, if materialized.
    fn vm_db_conn(&mut self) -> &mut Option<rusqlite::Connection>;
    /// The per-request server query params (`query_param`).
    fn vm_server_query_params(&self) -> Option<&HashMap<String, String>>;
}

/// `query_param(name)` — VM. Same parse/return shape as the TW; the
/// lookup goes through the VM's own `server_query_params` map.
pub fn query_param_vm(vm: &impl VmDbAccess, args: &[Value]) -> Result<Value, String> {
    query_param(args, |name| {
        vm.vm_server_query_params()
            .and_then(|params| params.get(name))
            .cloned()
    })
}

/// `db_insert(table, struct)` — VM. The VM second-argument error text
/// has NO shape suffix (the TW text differs — each side keeps its own);
/// the connection is lazily opened (№409).
pub fn db_insert_vm(vm: &mut impl VmDbAccess, args: &[Value]) -> Result<Value, String> {
    let table = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Err(
                "db_insert() expects first argument to be a table name (String)".to_string(),
            )
        }
    };
    let fields = match args.get(1) {
        Some(Value::Struct { fields, .. }) => fields.clone(),
        _ => return Err("db_insert() expects second argument to be a Struct".to_string()),
    };
    // №409: LAZY db open — the connection (in-memory sqlite +
    // schema DDL) materializes here, on first use.
    vm.ensure_db_open();
    let conn = vm
        .vm_db_conn()
        .as_mut()
        .ok_or_else(|| {
            "db_insert() error: no database connection. Declare db { url: \"sqlite::memory:\" } first.".to_string()
        })?;
    let col_names: Vec<String> = fields.keys().cloned().collect();
    let placeholders: Vec<String> = col_names.iter().map(|_| "?".to_string()).collect();
    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        table,
        col_names.join(", "),
        placeholders.join(", ")
    );
    let params: Vec<Box<dyn rusqlite::types::ToSql>> = fields
        .values()
        .map(|v| match v {
            Value::String(s) => Box::new(s.clone()) as Box<dyn rusqlite::types::ToSql>,
            Value::Float(f) => Box::new(*f) as Box<dyn rusqlite::types::ToSql>,
            Value::Bool(b) => Box::new(*b) as Box<dyn rusqlite::types::ToSql>,
            Value::Unit => Box::new(Option::<String>::None) as Box<dyn rusqlite::types::ToSql>,
            other => Box::new(format!("{}", other)) as Box<dyn rusqlite::types::ToSql>,
        })
        .collect();
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    conn.execute(&sql, param_refs.as_slice())
        .map_err(|e| sql_err("db_insert() SQL error", e))?;
    let rowid: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |row| row.get(0))
        .unwrap_or(0);
    Ok(Value::Float(rowid as f64))
}

/// `query_scalar(sql, params?)` — VM. TYPED param binding (the №381
/// parity fix); the connection is lazily opened (№409).
pub fn query_scalar_vm(vm: &mut impl VmDbAccess, args: &[Value]) -> Result<Value, String> {
    let sql = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("query_scalar() expected String SQL".to_string()),
    };
    // Naryad #381 parity fix: bind parameters TYPED (the shared
    // convert_params SSOT) instead of stringifying them — the old
    // Float→"3"/Bool→"true" string binds degraded types behind
    // sqlite affinity (tree-walking binds them typed).
    let params: Vec<rusqlite::types::Value> = if args.len() > 1 {
        match &args[1] {
            Value::List(items) => convert_params(items)?,
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    };
    // №409: lazy db open on first use.
    vm.ensure_db_open();
    let conn = vm
        .vm_db_conn()
        .as_ref()
        .ok_or_else(|| "query_scalar() error: no database connection.".to_string())?;
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| sql_err("query_scalar() SQL error", e))?;
    let mut rows = stmt
        .query_map(rusqlite::params_from_iter(params.iter()), |row| {
            row.get_ref(0).map(|v| match v {
                rusqlite::types::ValueRef::Null => Value::Unit,
                rusqlite::types::ValueRef::Integer(n) => Value::Float(n as f64),
                rusqlite::types::ValueRef::Real(f) => Value::Float(f),
                rusqlite::types::ValueRef::Text(s) => {
                    Value::String(String::from_utf8_lossy(s).to_string())
                }
                rusqlite::types::ValueRef::Blob(b) => {
                    Value::String(b.iter().map(|byte| format!("{:02x}", byte)).collect())
                }
            })
        })
        .map_err(|e| sql_err("query_scalar() execution error", e))?;
    match rows.next() {
        Some(Ok(val)) => Ok(val),
        Some(Err(e)) => Err(sql_err("query_scalar() row error", e)),
        None => Ok(Value::Unit),
    }
}

/// `query(sql, params?)` — VM. SELECT → List of Struct ("Row"); the
/// optional params list binds TYPED (the №381 parity fix — the VM
/// dropped it entirely before that naryad).
pub fn query_vm(vm: &mut impl VmDbAccess, args: &[Value]) -> Result<Value, String> {
    let sql = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("query() expected String SQL".to_string()),
    };
    // Naryad #381 parity fix: the VM dropped the optional params list
    // entirely (stmt.query([])) — any parameterized query failed with
    // "Wrong number of parameters passed to query. Got 0, needed N",
    // while the tree-walking backend binds them. The Stage 4
    // benchmark corpus (naryad #381, ADR-0141 §D5) caught the
    // divergence; both backends now share the typed convert_params.
    let params: Vec<rusqlite::types::Value> = if args.len() > 1 {
        match &args[1] {
            Value::List(items) => convert_params(items)?,
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    };
    // №409: lazy db open on first use.
    vm.ensure_db_open();
    let conn = vm
        .vm_db_conn()
        .as_ref()
        .ok_or_else(|| "query() error: no database connection.".to_string())?;
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| sql_err("query() SQL error", e))?;
    let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
    let mut rows = stmt
        .query(rusqlite::params_from_iter(params.iter()))
        .map_err(|e| sql_err("query() execution error", e))?;
    let mut results = Vec::new();
    while let Some(row) = rows.next().map_err(|e| sql_err("query() row error", e))? {
        let mut fields = std::collections::HashMap::new();
        for (i, col) in col_names.iter().enumerate() {
            let val: rusqlite::types::ValueRef = row
                .get_ref(i)
                .map_err(|e| format!("query() column {} error: {}", col, e))?;
            fields.insert(
                col.clone(),
                match val {
                    rusqlite::types::ValueRef::Null => Value::Unit,
                    rusqlite::types::ValueRef::Integer(n) => Value::Float(n as f64),
                    rusqlite::types::ValueRef::Real(f) => Value::Float(f),
                    rusqlite::types::ValueRef::Text(s) => {
                        Value::String(String::from_utf8_lossy(s).to_string())
                    }
                    rusqlite::types::ValueRef::Blob(b) => {
                        Value::String(b.iter().map(|byte| format!("{:02x}", byte)).collect())
                    }
                },
            );
        }
        results.push(Value::Struct {
            type_name: "Row".to_string(),
            fields,
        });
    }
    Ok(Value::List(results))
}

/// `db_execute(sql, params?)` — VM. TYPED binding (№381); returns
/// `Unit` (the TW returns the affected count as a String — the known
/// divergence class stays as-is); lazy open (№409).
pub fn db_execute_vm(vm: &mut impl VmDbAccess, args: &[Value]) -> Result<Value, String> {
    let sql = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("db_execute() expected String SQL".to_string()),
    };
    // Naryad #381 parity fix: typed param binding (convert_params
    // SSOT) instead of stringification — same contract as the
    // tree-walking backend (Bool→0/1, Float→REAL, no affinity hacks).
    let params: Vec<rusqlite::types::Value> = if args.len() > 1 {
        match &args[1] {
            Value::List(items) => convert_params(items)?,
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    };
    // №409: lazy db open on first use.
    vm.ensure_db_open();
    let conn = vm
        .vm_db_conn()
        .as_ref()
        .ok_or_else(|| "db_execute() error: no database connection.".to_string())?;
    conn.execute(&sql, rusqlite::params_from_iter(params.iter()))
        .map_err(|e| sql_err("db_execute() SQL error", e))?;
    Ok(Value::Unit)
}

/// `db_execute_with_grant(g, sql, params?)` — VM. Naryad #390
/// (ADR-0155): the granted destructive-SQL action. Same gates as the
/// tree-walking backend (ledger state/TTL/scope via src/grants.rs,
/// typed binding via convert_params — the №381 contract); consumption
/// happens only after the statement succeeded. Lazy open (№409).
pub fn db_execute_with_grant_vm(vm: &mut impl VmDbAccess, args: &[Value]) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::Grant(h)) => h.clone(),
        Some(other) => {
            return Err(format!(
                "{}() first argument must be a Grant, got {}",
                NAME_DB_EXECUTE_WITH_GRANT,
                other.type_name()
            ))
        }
        None => {
            return Err(format!(
                "{}() missing grant argument",
                NAME_DB_EXECUTE_WITH_GRANT
            ))
        }
    };
    let sql = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(format!(
                "{}() second argument must be String SQL, got {}",
                NAME_DB_EXECUTE_WITH_GRANT,
                other.type_name()
            ))
        }
        None => {
            return Err(format!(
                "{}() missing sql argument",
                NAME_DB_EXECUTE_WITH_GRANT
            ))
        }
    };
    let params: Vec<rusqlite::types::Value> = if args.len() > 2 {
        match &args[2] {
            Value::List(items) => convert_params(items)?,
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    };
    crate::grants::check_active(&handle)?;
    let ops = crate::grants::extract_destructive_ops(&sql);
    let destructive = !ops.is_empty();
    for (op, table) in &ops {
        if !crate::grants::scope_covers(&handle.scope, op, table) {
            return Err(format!(
                "GRANT_SCOPE_MISMATCH: grant {} ({}, scope '{}') does not cover {} {}",
                handle.grant_id, handle.class, handle.scope, op, table
            ));
        }
    }
    // №409: lazy db open on first use.
    vm.ensure_db_open();
    let conn = vm.vm_db_conn().as_ref().ok_or_else(|| {
        format!(
            "{}() error: no database connection.",
            NAME_DB_EXECUTE_WITH_GRANT
        )
    })?;
    let affected = conn
        .execute(&sql, rusqlite::params_from_iter(params.iter()))
        .map_err(|e| sql_err("db_execute_with_grant() SQL error", e))?;
    if destructive {
        crate::grants::grant_use(&handle, &grant_use_note(&sql))?;
        // ── Naryad #393 (ADR-0167 §3.4), runtime-twin parity with
        // src/interpreter/db.rs (the naryad-397 follow-up caught
        // the VM side missing this record — the wave-3 e2e could
        // not see it because the TW+VM records share one process
        // ledger and the content assertions were not per-run): the
        // irreversible action SUCCEEDED — the journal entry is a
        // side effect of the success path itself. The SQL preimage
        // never enters the journal — only its SHA-256.
        crate::ledger::record(
            LEDGER_IRREVERSIBLE_DB_EXECUTE,
            &handle.issuer,
            &handle.scope,
            &format!("{}|{}|{}", handle.grant_id, handle.scope, sql),
        );
        eprintln!(
            "[GRANT_USE] grant (scope '{}', class {}) executed {} (affected {}) — remaining {}",
            handle.scope,
            handle.class,
            sql.trim(),
            affected,
            crate::grants::state_of(&handle.grant_id)
                .map(|(_, r)| r)
                .unwrap_or(-1)
        );
    } else {
        eprintln!(
            "[GRANT_USE] grant (scope '{}') ran non-destructive SQL — no consumption",
            handle.scope
        );
    }
    Ok(Value::String(affected.to_string()))
}

/// `query_row(sql, params?)` — VM. The VM binds params STRINGIFIED
/// (Vec<String> — String verbatim, Float/Bool via Display) — the
/// KNOWN TW/VM divergence class pinned by the №465 fuzzer; the typed
/// TW lane stays the №381 contract. Lazy open (№409).
pub fn query_row_vm(vm: &mut impl VmDbAccess, args: &[Value]) -> Result<Value, String> {
    let sql = match args.first() {
        Some(Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(format!(
                "query_row() expected String SQL, got {}",
                other.type_name()
            ))
        }
        None => return Err("query_row() requires at least 1 argument (SQL string)".to_string()),
    };
    let params: Vec<String> = if args.len() > 1 {
        match &args[1] {
            Value::List(items) => items
                .iter()
                .filter_map(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    Value::Float(n) => Some(format!("{}", n)),
                    Value::Bool(b) => Some(format!("{}", b)),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    };

    // №409: lazy db open on first use.
    vm.ensure_db_open();
    let conn = vm
        .vm_db_conn()
        .as_mut()
        .ok_or_else(|| "query_row() error: no database connection.".to_string())?;
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| sql_err("query_row() SQL error", e))?;
    let col_count = stmt.column_count();
    let mut rows = stmt
        .query_map(rusqlite::params_from_iter(params.iter()), |row| {
            let mut vals = Vec::with_capacity(col_count);
            for i in 0..col_count {
                let val = match row.get_ref(i) {
                    Ok(rusqlite::types::ValueRef::Null) => Value::Unit,
                    Ok(rusqlite::types::ValueRef::Integer(n)) => Value::Float(n as f64),
                    Ok(rusqlite::types::ValueRef::Real(f)) => Value::Float(f),
                    Ok(rusqlite::types::ValueRef::Text(s)) => {
                        Value::String(String::from_utf8_lossy(s).to_string())
                    }
                    Ok(rusqlite::types::ValueRef::Blob(b)) => {
                        Value::String(b.iter().map(|byte| format!("{:02x}", byte)).collect())
                    }
                    Err(_) => Value::Unit,
                };
                vals.push(val);
            }
            Ok(vals)
        })
        .map_err(|e| sql_err("query_row() execution error", e))?;

    match rows.next() {
        Some(Ok(vals)) => Ok(Value::List(vals)),
        Some(Err(e)) => Err(sql_err("query_row() row error", e)),
        None => Ok(Value::List(vec![])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::sync::Arc;

    // ── the mock VM: every VM function is driven through the trait ──

    struct MockVm {
        conn: Option<rusqlite::Connection>,
        opens: Cell<u32>,
        params: Option<HashMap<String, String>>,
    }

    impl MockVm {
        fn new() -> Self {
            MockVm {
                conn: None,
                opens: Cell::new(0),
                params: None,
            }
        }
        fn with_params(mut self, params: &[(&str, &str)]) -> Self {
            self.params = Some(
                params
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            );
            self
        }
    }

    impl VmDbAccess for MockVm {
        fn ensure_db_open(&mut self) {
            // The №409 lazy-open contract: materialize once, on first use —
            // repeated calls on an open connection are no-ops (the mock
            // mirrors the real Vm::ensure_db_open shape).
            if self.conn.is_none() {
                self.opens.set(self.opens.get() + 1);
                self.conn = Some(rusqlite::Connection::open_in_memory().unwrap());
            }
        }
        fn vm_db_conn(&mut self) -> &mut Option<rusqlite::Connection> {
            &mut self.conn
        }
        fn vm_server_query_params(&self) -> Option<&HashMap<String, String>> {
            self.params.as_ref()
        }
    }

    // ── the TW harness: a real Arc<Mutex<Option<Connection>>> ──

    fn tw_db() -> Arc<Mutex<Option<rusqlite::Connection>>> {
        Arc::new(Mutex::new(Some(
            rusqlite::Connection::open_in_memory().unwrap(),
        )))
    }

    fn tw_db_empty() -> Arc<Mutex<Option<rusqlite::Connection>>> {
        Arc::new(Mutex::new(None))
    }

    fn s(v: &str) -> Value {
        Value::String(v.to_string())
    }

    /// `Value` is Debug/Clone, deliberately not PartialEq — the contract
    /// assertions compare the Debug shapes (the same discipline the
    /// memory-group tests use).
    macro_rules! val_eq {
        ($a:expr, $b:expr) => {
            assert_eq!(format!("{:?}", $a), format!("{:?}", $b))
        };
        ($a:expr, $b:expr, $msg:literal) => {
            assert_eq!(format!("{:?}", $a), format!("{:?}", $b), $msg)
        };
    }

    fn table(db: &Arc<Mutex<Option<rusqlite::Connection>>>) {
        db_execute_tw(db, &[s("CREATE TABLE t (a TEXT, b REAL)")]).unwrap();
    }

    // ── handles(): the name set is spelled exactly once ──

    #[test]
    fn handles_covers_exactly_the_seven_db_names() {
        for name in [
            NAME_QUERY,
            NAME_QUERY_PARAM,
            NAME_QUERY_ROW,
            NAME_QUERY_SCALAR,
            NAME_DB_EXECUTE,
            NAME_DB_EXECUTE_WITH_GRANT,
            NAME_DB_INSERT,
        ] {
            assert!(handles(name), "{} must be owned by the module", name);
        }
        assert!(!handles("memorize"));
        assert!(!handles("exec"));
        assert!(!handles(""));
        assert!(!handles("db_execute_malformed"));
    }

    // ── the TW lane: exact texts and shapes, transplanted verbatim ──

    #[test]
    fn tw_query_select_returns_rows_of_structs_and_dml_returns_count() {
        let db = tw_db();
        table(&db);
        db_execute_tw(&db, &[s("INSERT INTO t VALUES ('x', 1.5)")]).unwrap();
        let _ = db_insert_tw(
            &db,
            &[
                s("t"),
                Value::Struct {
                    type_name: "T".to_string(),
                    fields: [
                        ("a".to_string(), s("y")),
                        ("b".to_string(), Value::Float(2.0)),
                    ]
                    .into_iter()
                    .collect(),
                },
            ],
        )
        .unwrap();
        let out = query_tw(&db, &[s("SELECT a, b FROM t ORDER BY a")]).unwrap();
        match out {
            Value::List(rows) => {
                assert_eq!(rows.len(), 2);
                match &rows[0] {
                    Value::Struct { type_name, fields } => {
                        assert_eq!(type_name, "Row");
                        val_eq!(fields.get("a"), Some(&s("x")));
                        val_eq!(fields.get("b"), Some(&Value::Float(1.5)));
                    }
                    other => panic!("expected Row struct, got {}", other.type_name()),
                }
            }
            other => panic!("expected List, got {}", other.type_name()),
        }
        let cnt = query_tw(&db, &[s("DELETE FROM t WHERE a = 'y'")]).unwrap();
        val_eq!(cnt, s("1"));
    }

    #[test]
    fn tw_query_error_texts_are_exact() {
        let db = tw_db();
        assert_eq!(
            query_tw(&db, &[]).unwrap_err(),
            "query() requires at least 1 argument (SQL string)"
        );
        assert_eq!(
            query_tw(&db, &[Value::Float(1.0)]).unwrap_err(),
            "query() expected String SQL, got Float"
        );
    }

    #[test]
    fn tw_db_execute_params_and_connection_texts_are_exact() {
        let db = tw_db_empty();
        assert_eq!(
            db_execute_tw(&db, &[s("DELETE FROM t")]).unwrap_err(),
            "db_execute() error: no database connection. Declare db { url: \"sqlite::memory:\" } first."
        );
        let db = tw_db();
        assert_eq!(
            db_execute_tw(&db, &[]).unwrap_err(),
            "db_execute() requires at least 1 argument (SQL string)"
        );
        assert_eq!(
            db_execute_tw(&db, &[Value::Bool(true)]).unwrap_err(),
            "db_execute() expected String SQL, got Bool"
        );
        assert_eq!(
            db_execute_tw(&db, &[s("DELETE FROM t"), Value::Float(2.0)]).unwrap_err(),
            "db_execute() second argument must be List, got Float"
        );
        table(&db);
        let out = db_execute_tw(&db, &[s("INSERT INTO t VALUES ('x', 1.5)")]).unwrap();
        val_eq!(out, s("1"));
    }

    #[test]
    fn tw_query_scalar_and_query_row_shapes() {
        let db = tw_db();
        table(&db);
        db_execute_tw(&db, &[s("INSERT INTO t VALUES ('a', 7.0)")]).unwrap();
        val_eq!(
            query_scalar_tw(&db, &[s("SELECT b FROM t LIMIT 1")]).unwrap(),
            Value::Float(7.0)
        );
        val_eq!(
            query_scalar_tw(&db, &[s("SELECT b FROM t WHERE a='zz'")]).unwrap(),
            Value::Unit
        );
        let row = query_row_tw(&db, &[s("SELECT a, b FROM t LIMIT 1")]).unwrap();
        match row {
            Value::List(vals) => val_eq!(vals, vec![s("a"), Value::Float(7.0)]),
            other => panic!("expected List row, got {}", other.type_name()),
        }
        val_eq!(
            query_row_tw(&db, &[s("SELECT a FROM t WHERE a='zz'")]).unwrap(),
            Value::List(vec![])
        );
        // Typed bind (the №381 contract): Float → REAL, not text.
        let bind = query_scalar_tw(
            &db,
            &[s("SELECT typeof(?)"), Value::List(vec![Value::Float(3.0)])],
        )
        .unwrap();
        val_eq!(bind, s("real"));
    }

    #[test]
    fn tw_db_insert_rowid_and_error_texts() {
        let db = tw_db_empty();
        assert_eq!(
            db_insert_tw(&db, &[s("t"), Value::Float(1.0)]).unwrap_err(),
            "db_insert() expects second argument to be a Struct { field: value, ... }"
        );
        let db = tw_db();
        table(&db);
        let id = db_insert_tw(
            &db,
            &[
                s("t"),
                Value::Struct {
                    type_name: "T".to_string(),
                    fields: [
                        ("a".to_string(), s("v")),
                        ("b".to_string(), Value::Float(1.0)),
                    ]
                    .into_iter()
                    .collect(),
                },
            ],
        )
        .unwrap();
        val_eq!(id, Value::Float(1.0));
        assert_eq!(
            db_insert_tw(&db, &[Value::Float(1.0)]).unwrap_err(),
            "db_insert() expects first argument to be a table name (String)"
        );
    }

    #[test]
    fn tw_db_execute_with_grant_arity_type_and_scope_texts() {
        let db = tw_db();
        assert_eq!(
            db_execute_with_grant_tw(&db, &[s("db:delete:users")]).unwrap_err(),
            "db_execute_with_grant: expects 2..3 arguments (grant, sql, params?), got 1"
        );
        assert_eq!(
            db_execute_with_grant_tw(&db, &[Value::Float(1.0), s("DELETE FROM t")]).unwrap_err(),
            "db_execute_with_grant: first argument must be a Grant, got Float"
        );
        let class = crate::grants::GrantClass::parse("unlimited").unwrap();
        let handle =
            crate::grants::issue("n466-tw-unrelated:table", 3600, &class, "n466-test").unwrap();
        assert!(
            db_execute_with_grant_tw(&db, &[Value::Grant(handle), s("DROP TABLE t")])
                .unwrap_err()
                .starts_with("GRANT_SCOPE_MISMATCH:"),
            "an out-of-scope destructive statement must refuse with the stable code"
        );
    }

    #[test]
    fn tw_db_execute_with_grant_consumes_only_destructive_sql() {
        let db = tw_db();
        table(&db);
        let class = crate::grants::GrantClass::parse("once").unwrap();
        let handle = crate::grants::issue("db:delete:t:n466", 3600, &class, "n466-test").unwrap();
        // Non-destructive SQL under the grant: executes, does NOT consume.
        let out = db_execute_with_grant_tw(
            &db,
            &[
                Value::Grant(handle.clone()),
                s("INSERT INTO t VALUES ('a', 1.0)"),
            ],
        )
        .unwrap();
        val_eq!(out, s("1"));
        // The same Once grant still covers the destructive statement…
        let out =
            db_execute_with_grant_tw(&db, &[Value::Grant(handle.clone()), s("DELETE FROM t")])
                .unwrap();
        val_eq!(out, s("1"));
        // …and consumption happens exactly on the destructive success.
        let (_, remaining) = crate::grants::state_of(&handle.grant_id).unwrap();
        assert_eq!(
            remaining, 0,
            "a Once grant is consumed by the destructive use"
        );
    }

    // ── the VM lane: the trait contract and the divergent forms ──

    #[test]
    fn vm_db_execute_returns_unit_and_lazily_opens_once() {
        let mut vm = MockVm::new();
        let out = db_execute_vm(&mut vm, &[s("CREATE TABLE t (a TEXT)")]).unwrap();
        val_eq!(
            out,
            Value::Unit,
            "the VM db_execute returns Unit (the TW returns a String count)"
        );
        assert_eq!(vm.opens.get(), 1, "the lazy open fires exactly once");
        db_execute_vm(&mut vm, &[s("INSERT INTO t VALUES ('x')")]).unwrap();
        assert_eq!(vm.opens.get(), 1, "an open connection is not re-opened");
        assert_eq!(
            db_execute_vm(&mut vm, &[Value::Unit]).unwrap_err(),
            "db_execute() expected String SQL"
        );
    }

    #[test]
    fn vm_missing_connection_text_is_exact() {
        struct FailedVm {
            none: Option<rusqlite::Connection>,
        }
        impl VmDbAccess for FailedVm {
            fn ensure_db_open(&mut self) {}
            fn vm_db_conn(&mut self) -> &mut Option<rusqlite::Connection> {
                &mut self.none
            }
            fn vm_server_query_params(&self) -> Option<&HashMap<String, String>> {
                None
            }
        }
        let mut failed = FailedVm { none: None };
        assert_eq!(
            query_row_vm(&mut failed, &[s("SELECT 1")]).unwrap_err(),
            "query_row() error: no database connection."
        );
        assert_eq!(
            query_scalar_vm(&mut failed, &[s("SELECT 1")]).unwrap_err(),
            "query_scalar() error: no database connection."
        );
        assert_eq!(
            query_vm(&mut failed, &[s("SELECT 1")]).unwrap_err(),
            "query() error: no database connection."
        );
        assert_eq!(
            db_execute_vm(&mut failed, &[s("SELECT 1")]).unwrap_err(),
            "db_execute() error: no database connection."
        );
    }

    #[test]
    fn vm_binds_params_typed_but_query_row_stringifies_the_known_divergence() {
        let mut vm = MockVm::new();
        db_execute_vm(&mut vm, &[s("CREATE TABLE t (a TEXT)")]).unwrap();
        // The typed lanes (the №381 contract): Float → REAL.
        let bind = query_scalar_vm(
            &mut vm,
            &[s("SELECT typeof(?)"), Value::List(vec![Value::Float(3.0)])],
        )
        .unwrap();
        val_eq!(bind, s("real"));
        // The query_row stringify lane (the №465-pinned divergence):
        // Float 3.0 → "3" (Display), so the TEXT comparison hits.
        db_execute_vm(&mut vm, &[s("INSERT INTO t VALUES ('3')")]).unwrap();
        let row = query_row_vm(
            &mut vm,
            &[
                s("SELECT a FROM t WHERE a = ?"),
                Value::List(vec![Value::Float(3.0)]),
            ],
        )
        .unwrap();
        val_eq!(row, Value::List(vec![s("3")]));
    }

    #[test]
    fn vm_query_shapes_and_error_texts() {
        let mut vm = MockVm::new();
        assert_eq!(
            query_vm(&mut vm, &[]).unwrap_err(),
            "query() expected String SQL"
        );
        db_execute_vm(&mut vm, &[s("CREATE TABLE t (a TEXT, b REAL)")]).unwrap();
        db_execute_vm(&mut vm, &[s("INSERT INTO t VALUES ('x', 1.5)")]).unwrap();
        let out = query_vm(&mut vm, &[s("SELECT a, b FROM t")]).unwrap();
        match out {
            Value::List(rows) => {
                assert_eq!(rows.len(), 1);
                match &rows[0] {
                    Value::Struct { type_name, fields } => {
                        assert_eq!(type_name, "Row");
                        val_eq!(fields.get("a"), Some(&s("x")));
                        val_eq!(fields.get("b"), Some(&Value::Float(1.5)));
                    }
                    other => panic!("expected Row struct, got {}", other.type_name()),
                }
            }
            other => panic!("expected List, got {}", other.type_name()),
        }
        val_eq!(
            query_scalar_vm(&mut vm, &[s("SELECT b FROM t LIMIT 1")]).unwrap(),
            Value::Float(1.5)
        );
        assert_eq!(
            query_scalar_vm(&mut vm, &[Value::Float(9.0)]).unwrap_err(),
            "query_scalar() expected String SQL"
        );
    }

    #[test]
    fn vm_db_insert_rowid_and_error_texts() {
        let mut vm = MockVm::new();
        db_execute_vm(&mut vm, &[s("CREATE TABLE t (a TEXT)")]).unwrap();
        let id = db_insert_vm(
            &mut vm,
            &[
                s("t"),
                Value::Struct {
                    type_name: "T".to_string(),
                    fields: [("a".to_string(), s("v"))].into_iter().collect(),
                },
            ],
        )
        .unwrap();
        val_eq!(id, Value::Float(1.0));
        assert_eq!(
            db_insert_vm(&mut vm, &[s("t"), Value::Float(1.0)]).unwrap_err(),
            "db_insert() expects second argument to be a Struct",
            "the VM text has no shape suffix — the TW text differs (each side keeps its own)"
        );
        assert_eq!(
            db_insert_vm(&mut vm, &[Value::Float(1.0)]).unwrap_err(),
            "db_insert() expects first argument to be a table name (String)"
        );
    }

    #[test]
    fn vm_db_execute_with_grant_texts_and_scope_gate() {
        let mut vm = MockVm::new();
        assert_eq!(
            db_execute_with_grant_vm(&mut vm, &[]).unwrap_err(),
            "db_execute_with_grant() missing grant argument"
        );
        assert_eq!(
            db_execute_with_grant_vm(&mut vm, &[Value::Float(1.0)]).unwrap_err(),
            "db_execute_with_grant() first argument must be a Grant, got Float"
        );
        let class = crate::grants::GrantClass::parse("unlimited").unwrap();
        let handle =
            crate::grants::issue("n466-vm-unrelated:table", 3600, &class, "n466-test").unwrap();
        assert!(
            db_execute_with_grant_vm(&mut vm, &[Value::Grant(handle), s("DROP TABLE t")])
                .unwrap_err()
                .starts_with("GRANT_SCOPE_MISMATCH:"),
            "the scope gate fires before the connection is even opened"
        );
        assert_eq!(
            vm.opens.get(),
            0,
            "the lazy open does not fire on a gate refusal"
        );
    }

    // ── the shared query_param shape ──

    #[test]
    fn query_param_shared_shape_hit_miss_and_non_string() {
        let hit = query_param(&[s("lang")], |k| {
            if k == "lang" {
                Some("ru".to_string())
            } else {
                None
            }
        })
        .unwrap();
        val_eq!(hit, s("ru"));
        let miss = query_param(&[s("nope")], |_| None).unwrap();
        val_eq!(miss, s(""));
        let non_string = query_param(&[Value::Float(1.0)], |k| {
            // the degraded EMPTY name reaches the lookup — a real map misses it
            if k.is_empty() {
                None
            } else {
                Some("x".to_string())
            }
        })
        .unwrap();
        val_eq!(
            non_string,
            s(""),
            "a non-String argument degrades to the empty name"
        );
    }

    #[test]
    fn vm_query_param_uses_its_own_map() {
        let vm = MockVm::new().with_params(&[("lang", "en")]);
        let out = query_param_vm(&vm, &[s("lang")]).unwrap();
        val_eq!(out, s("en"));
        let miss = query_param_vm(&vm, &[s("zzz")]).unwrap();
        val_eq!(miss, s(""));
    }
}

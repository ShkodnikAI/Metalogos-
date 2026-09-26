use super::*;
use crate::ast::*;

/// №385 (ADR-0169): stamp a `rusqlite::Error` with the stable `SQL_ERROR`
/// code AT THE ORIGIN — the only errors allowed to carry that code (the
/// SQL layer itself; lock-poisoning and API-validation refusals in this
/// file stay unstamped and classify as the honest `RUNTIME_ERROR` fallback).
/// The message text after the stamp is unchanged.
pub(crate) fn sql_err(ctx: &str, e: rusqlite::Error) -> String {
    crate::interpreter::values::coded_error(
        crate::interpreter::values::CODE_SQL_ERROR,
        format!("{}: {}", ctx, e),
    )
}

/// Наряд №99: Unified SQL parameter conversion — one function, four call sites.
/// Returns typed `rusqlite::types::Value` for each parameter, rejecting
/// unsupported types with an error instead of silently dropping or
/// degrading them.
///
/// Supported types:
///   - String → Text
///   - Float  → Real   (covers Int, since Metalogos represents ints as f64)
///   - Bool   → Integer(0 or 1) — idiomatic SQLite boolean representation
///   - Unit   → Null
///
/// Unsupported types (Secret, Struct, List, Html, etc.) produce an error.
pub(crate) fn convert_params(items: &[Value]) -> Result<Vec<rusqlite::types::Value>, String> {
    items
        .iter()
        .enumerate()
        .map(|(i, v)| match v {
            Value::String(s) => Ok(rusqlite::types::Value::Text(s.clone())),
            Value::Float(n) => Ok(rusqlite::types::Value::Real(*n)),
            Value::Bool(b) => Ok(rusqlite::types::Value::Integer(if *b { 1 } else { 0 })),
            Value::Unit => Ok(rusqlite::types::Value::Null),
            other => Err(format!(
                "SQL parameter ${} must be String, Float, Bool, or Unit — got {}",
                i + 1,
                other.type_name()
            )),
        })
        .collect()
}

impl Interpreter {
    /// Map Metalogos type names to SQLite column types (Problem C).
    pub(super) fn mlog_type_to_sql(t: &str) -> &'static str {
        match t {
            "Int" => "INTEGER",
            "Float" => "REAL",
            "String" | "Text" => "TEXT",
            "Bool" => "INTEGER",
            "DateTime" => "TEXT",
            _ => "TEXT",
        }
    }

    /// Problem C: Apply schema declaration — CREATE TABLE IF NOT EXISTS for each table.
    /// №426 (ADR-0175 §3.4): store a schema-as-code declaration for
    /// later replay (order-independent DDL — a `schema {}` decl may
    /// precede the `db {}` block or land on a conn-less context).
    pub(super) fn store_schema(&mut self, schema: &SchemaDecl) {
        if !self.schemas.iter().any(|sk| sk.name == schema.name) {
            self.schemas.push(schema.clone());
        }
    }

    /// №426 (ADR-0175 §3.4): replay EVERY stored schema against the
    /// live connection — additive-only, idempotent (CREATE TABLE IF NOT
    /// EXISTS, the ADR-0060 discipline). Failures are LOUD on stderr
    /// and never abort the context (the same best-effort posture as the
    /// ledger side effects). Called whenever a fresh connection becomes
    /// available (init_db_connection / reconnect_db) and once after the
    /// serve startup merge (build_state) — so routes AND cron ticks see
    /// the same schema as the program itself.
    pub fn replay_schemas(&self) {
        let has_conn = self
            .db_conn
            .lock()
            .map(|guard| guard.is_some())
            .unwrap_or(false);
        if !has_conn || self.schemas.is_empty() {
            return;
        }
        for schema in &self.schemas {
            if let Err(e) = self.apply_schema(schema) {
                eprintln!(
                    "[db] schema replay '{}' failed (loud, best-effort): {}",
                    schema.name, e
                );
            }
        }
    }

    pub(super) fn apply_schema(&self, schema: &SchemaDecl) -> Result<(), String> {
        let guard = self
            .db_conn
            .lock()
            .map_err(|e| format!("db lock error: {}", e))?;
        let conn = guard.as_ref().ok_or_else(|| {
            "schema declaration requires a db connection. Declare db { url: \"...\" } first."
                .to_string()
        })?;

        for table in &schema.tables {
            let mut col_defs = Vec::new();
            for col in &table.columns {
                let mut def = format!("{} {}", col.name, Self::mlog_type_to_sql(&col.col_type));
                for modi in &col.modifiers {
                    match modi {
                        ColumnModifier::PrimaryKey => def.push_str(" PRIMARY KEY"),
                        ColumnModifier::AutoIncrement => def.push_str(" AUTOINCREMENT"),
                        ColumnModifier::Nullable => def.push_str(" NULL"),
                        ColumnModifier::References(ref_table, ref_field) => {
                            def.push_str(&format!(" REFERENCES {}({})", ref_table, ref_field));
                        }
                    }
                }
                if let Some(ref default_val) = col.default {
                    if default_val == "now()" {
                        def.push_str(" DEFAULT (datetime('now'))");
                    } else {
                        // Strip quotes if present
                        let val = default_val.trim_matches('\"');
                        def.push_str(&format!(" DEFAULT '{}'", val));
                    }
                }
                col_defs.push(def);
            }
            let sql = format!(
                "CREATE TABLE IF NOT EXISTS {} ({})",
                table.name,
                col_defs.join(", ")
            );
            conn.execute(&sql, []).map_err(|e| {
                sql_err(
                    &format!("schema migration error for table '{}'", table.name),
                    e,
                )
            })?;
        }

        Ok(())
    }

    /// Initialize SQLite connection for db { url: "..." } block (Наряд №7).
    /// Supports "sqlite::memory:" for in-memory databases and file paths.
    pub(super) fn init_db_connection(&mut self, db: &DbDecl) {
        let url_expr = match &db.url {
            Some(expr) => expr,
            None => {
                eprintln!("[db] No url specified in db {{}} block — query() will be unavailable");
                return;
            }
        };
        // Evaluate the url expression (must be a string literal or variable)
        let url = match self.eval_expr(url_expr) {
            Ok(Value::String(s)) => s,
            Ok(other) => {
                eprintln!("[db] url must be a String, got {}", other.type_name());
                return;
            }
            Err(e) => {
                eprintln!("[db] Failed to evaluate url: {}", e);
                return;
            }
        };
        // Parse the URL: "sqlite::memory:" → in-memory, "sqlite:path.db" → file
        let conn = if url == "sqlite::memory:" {
            rusqlite::Connection::open_in_memory()
        } else if url.starts_with("sqlite:") {
            let path = url.trim_start_matches("sqlite:");
            rusqlite::Connection::open(path)
        } else {
            eprintln!(
                "[db] Unsupported URL scheme: '{}'. Use 'sqlite::memory:' or 'sqlite:path.db'",
                url
            );
            return;
        };
        match conn {
            Ok(c) => {
                // Enable WAL mode for better concurrent read performance
                let _ = c.execute_batch("PRAGMA journal_mode=WAL;");
                let mut guard = self.db_conn.lock().unwrap_or_else(|e| e.into_inner());
                *guard = Some(c);
                // Store resolved URL for per-request interpreter reconnection
                self.db_url = Some(url.clone());
                drop(guard); // the replay re-locks — release first (no self-deadlock)
                eprintln!("[db] Connected: {}", url);
                // №426 (ADR-0175 §3.4): the connection is live — the
                // stored schema DDL replays NOW (order-independent:
                // schema-before-db and fresh files both work).
                self.replay_schemas();
            }
            Err(e) => {
                eprintln!("[db] Failed to connect to '{}': {}", url, e);
            }
        }
    }

    // №466 (gh#687) group 2 (db): the five db invoke methods moved to the
    // shared live module src/db_ops.rs (query_tw, db_execute_tw,
    // db_execute_with_grant_tw, query_scalar_tw, query_row_tw) and the
    // inline db_insert body of execution.rs joined them as db_insert_tw.
    // The name literals now live only in that module — the №462 counter
    // drops 56 → 49.

    /// Open a new DB connection using stored db_url (Наряд №8).
    /// Called by per-request interpreters to get their own SQLite connection.
    /// For in-memory DBs, the Arc-shared connection is already set via clone_definitions_into.
    /// For file-based DBs, opens a new connection (safe for concurrent access via WAL).
    pub fn reconnect_db(&mut self) {
        if let Some(ref url) = self.db_url {
            if url == "sqlite::memory:" {
                // In-memory DB: Arc-shared connection from main interpreter
                // No need to reconnect — clone_definitions_into already shared it
            } else if url.starts_with("sqlite:") {
                // File DB: open a new connection for this request (WAL handles concurrency)
                let path = url.trim_start_matches("sqlite:");
                match rusqlite::Connection::open(path) {
                    Ok(c) => {
                        let _ = c.execute_batch("PRAGMA journal_mode=WAL;");
                        // For file DBs, each request gets its own connection
                        // (don't overwrite the shared Arc for in-memory)
                        let mut guard = self.db_conn.lock().unwrap_or_else(|e| e.into_inner());
                        // Only set if no connection yet (in-memory may have set it)
                        if guard.is_none() {
                            *guard = Some(c);
                            // №426 (ADR-0175 §3.4): a fresh file connection
                            // replays the program's schema DDL (additive,
                            // idempotent) — the route/tick context is
                            // schema-ready without any startup handshake.
                            drop(guard);
                            self.replay_schemas();
                        }
                    }
                    Err(e) => {
                        eprintln!("[db] Per-request reconnect failed: {}", e);
                    }
                }
            }
        }
    }
}

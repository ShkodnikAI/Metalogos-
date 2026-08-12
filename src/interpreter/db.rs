use super::*;
use crate::ast::*;

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
            conn.execute(&sql, [])
                .map_err(|e| format!("schema migration error for table '{}': {}", table.name, e))?;
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
                eprintln!("[db] Connected: {}", url);
            }
            Err(e) => {
                eprintln!("[db] Failed to connect to '{}': {}", url, e);
            }
        }
    }

    /// Execute a SQL query and return readable results (Наряд №7).
    /// - SELECT → List of Struct (each row = struct with column names as fields)
    /// - INSERT/UPDATE/DELETE → String with affected row count
    /// - PRAGMA/CREATE/etc. → String "ok"
    pub(super) fn invoke_query(&self, args: &[Value]) -> Result<Value, String> {
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

        let guard = self
            .db_conn
            .lock()
            .map_err(|e| format!("db lock error: {}", e))?;
        let conn = guard.as_ref().ok_or_else(|| {
            "query() error: no database connection. Declare db { url: \"sqlite::memory:\" } first."
                .to_string()
        })?;

        let sql_upper = sql.trim().to_uppercase();
        if sql_upper.starts_with("SELECT") || sql_upper.starts_with("PRAGMA") {
            // SELECT/PRAGMA → List of Struct
            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| format!("query() SQL error: {}", e))?;
            let col_names: Vec<String> =
                stmt.column_names().iter().map(|s| s.to_string()).collect();
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
                                Value::String(
                                    b.iter().map(|byte| format!("{:02x}", byte)).collect(),
                                )
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
                .map_err(|e| format!("query() execution error: {}", e))?
                .filter_map(|r| r.ok())
                .collect();
            Ok(Value::List(rows))
        } else {
            // INSERT/UPDATE/DELETE/CREATE/ALTER/etc. → affected row count as String
            let affected = conn
                .execute(&sql, rusqlite::params_from_iter(params.iter()))
                .map_err(|e| format!("query() SQL error: {}", e))?;
            Ok(Value::String(affected.to_string()))
        }
    }

    /// Execute a SQL statement via db_execute() — returns affected row count (Наряд №7).
    /// ADR-0068: optional second argument (List) for parameterised queries.
    /// Single-argument form (SQL only) remains backward-compatible.
    pub(super) fn invoke_db_execute(&self, args: &[Value]) -> Result<Value, String> {
        let sql = match args.first() {
            Some(Value::String(s)) => s.clone(),
            Some(other) => {
                return Err(format!(
                    "db_execute() expected String SQL, got {}",
                    other.type_name()
                ))
            }
            None => {
                return Err("db_execute() requires at least 1 argument (SQL string)".to_string())
            }
        };
        // Optional second argument: List of parameter values (par with query())
        // Types: String→Text, Float→Real, Unit→NULL. ADR-0068.
        let params: Vec<rusqlite::types::Value> = match args.get(1) {
            Some(Value::List(items)) => items
                .iter()
                .map(|v| match v {
                    Value::String(s) => rusqlite::types::Value::Text(s.clone()),
                    Value::Float(n) => rusqlite::types::Value::Real(*n),
                    Value::Unit => rusqlite::types::Value::Null,
                    _ => rusqlite::types::Value::Text(String::new()),
                })
                .collect(),
            Some(other) => {
                return Err(format!(
                    "db_execute() second argument must be List, got {}",
                    other.type_name()
                ))
            }
            None => Vec::new(),
        };
        let guard = self
            .db_conn
            .lock()
            .map_err(|e| format!("db lock error: {}", e))?;
        let conn = guard.as_ref().ok_or_else(|| {
            "db_execute() error: no database connection. Declare db { url: \"sqlite::memory:\" } first."
                .to_string()
        })?;
        let affected = conn
            .execute(&sql, rusqlite::params_from_iter(params.iter()))
            .map_err(|e| format!("db_execute() SQL error: {}", e))?;
        Ok(Value::String(affected.to_string()))
    }

    /// Наряда-26 P1-7: query_scalar(sql, params) -> Value
    /// Executes a SELECT that returns exactly one row with one column.
    /// Returns the scalar value directly (String, Float, or Unit for NULL).
    pub(super) fn invoke_query_scalar(&self, args: &[Value]) -> Result<Value, String> {
        let sql = match args.first() {
            Some(Value::String(s)) => s.clone(),
            Some(other) => {
                return Err(format!(
                    "query_scalar() expected String SQL, got {}",
                    other.type_name()
                ))
            }
            None => {
                return Err("query_scalar() requires at least 1 argument (SQL string)".to_string())
            }
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

        let guard = self
            .db_conn
            .lock()
            .map_err(|e| format!("db lock error: {}", e))?;
        let conn = guard
            .as_ref()
            .ok_or_else(|| "query_scalar() error: no database connection.".to_string())?;

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("query_scalar() SQL error: {}", e))?;
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
            .map_err(|e| format!("query_scalar() execution error: {}", e))?;

        match rows.next() {
            Some(Ok(val)) => Ok(val),
            Some(Err(e)) => Err(format!("query_scalar() row error: {}", e)),
            None => Ok(Value::Unit),
        }
    }

    /// Наряда-26 P1-7: query_row(sql, params) -> List
    /// Executes a SELECT that returns exactly one row.
    /// Returns a List of column values (preserving column order).
    pub(super) fn invoke_query_row(&self, args: &[Value]) -> Result<Value, String> {
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

        let guard = self
            .db_conn
            .lock()
            .map_err(|e| format!("db lock error: {}", e))?;
        let conn = guard
            .as_ref()
            .ok_or_else(|| "query_row() error: no database connection.".to_string())?;

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("query_row() SQL error: {}", e))?;
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
            .map_err(|e| format!("query_row() execution error: {}", e))?;

        match rows.next() {
            Some(Ok(vals)) => Ok(Value::List(vals)),
            Some(Err(e)) => Err(format!("query_row() row error: {}", e)),
            None => Ok(Value::List(vec![])),
        }
    }

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

// ── НАРЯД #152 Contract Tests: db_execute() covered by SQL_DYNAMIC ───
//
// Before this fix, check_sql_dynamic only walked "query" calls.
// db_execute(format(...)) passed audit/check completely unnoticed —
// same injection surface, zero protection.
//
// Design decision: same strictness as query(). db_execute() supports
// optional params (ADR-0068), so parameterized db_execute(literal, [params])
// is safe. Only the first arg (SQL string) must be a literal.

/// C1: db_execute(variable) → SQL_DYNAMIC error.
#[test]
fn n152_db_execute_variable_rejected() {
    let source = r#"
        pattern DeleteUser(id: String) -> String {
            db_execute(id)
            return "ok"
        }
    "#;
    let result = metalogos::check_program(source).unwrap();
    assert!(!result.is_ok(), "db_execute(variable) should fail check");
    assert!(
        result.errors.iter().any(|e| e.contains("SQL_DYNAMIC")),
        "error should mention SQL_DYNAMIC, got: {:?}",
        result.errors
    );
}

/// C2: db_execute(format(...)) → SQL_DYNAMIC error (the motivating case).
#[test]
fn n152_db_execute_format_rejected() {
    let source = r#"
        pattern DeleteById(id: String) -> String {
            db_execute(format("DELETE FROM t WHERE id={}", id))
            return "ok"
        }
    "#;
    let result = metalogos::check_program(source).unwrap();
    assert!(!result.is_ok(), "db_execute(format(...)) should fail check");
    assert!(
        result.errors.iter().any(|e| e.contains("SQL_DYNAMIC")),
        "error should mention SQL_DYNAMIC, got: {:?}",
        result.errors
    );
    assert!(
        result.errors.iter().any(|e| e.contains("db_execute")),
        "error should mention db_execute, got: {:?}",
        result.errors
    );
}

/// C3: db_execute(literal) → passes check. Safe usage.
#[test]
fn n152_db_execute_literal_passes() {
    let source = r#"
        pattern InitSchema() -> String {
            db_execute("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT)")
            return "ok"
        }
    "#;
    let result = metalogos::check_program(source).unwrap();
    assert!(
        result.is_ok(),
        "db_execute(literal) should pass check, got: {:?}",
        result.errors
    );
}

/// C4: db_execute(literal, [params]) → passes check. Parameterized form (ADR-0068).
#[test]
fn n152_db_execute_literal_with_params_passes() {
    let source = r#"
        pattern InsertUser(name: String) -> String {
            db_execute("INSERT INTO users (name) VALUES ($1)", [name])
            return "ok"
        }
    "#;
    let result = metalogos::check_program(source).unwrap();
    assert!(
        result.is_ok(),
        "db_execute(literal, params) should pass check, got: {:?}",
        result.errors
    );
}

/// C5: query() still works (regression guard — existing coverage intact).
#[test]
fn n152_query_still_rejects_non_literal() {
    let source = r#"
        pattern BadQuery(table: String) -> String {
            let result = query(table)
            return result
        }
    "#;
    let result = metalogos::check_program(source).unwrap();
    assert!(!result.is_ok(), "query(variable) should still fail check");
    assert!(
        result.errors.iter().any(|e| e.contains("SQL_DYNAMIC")),
        "error should mention SQL_DYNAMIC, got: {:?}",
        result.errors
    );
}

/// C6: mlog run blocks db_execute with non-literal SQL.
#[test]
fn n152_run_blocks_db_execute_dynamic() {
    let source = r#"
        db { url: "sqlite::memory:" }
        pattern Bad() -> String {
            let sql = "SELECT 1"
            db_execute(sql)
            return "ok"
        }
    "#;
    let result = metalogos::run_program(source);
    assert!(result.is_err(), "mlog run should block on SQL_DYNAMIC");
    assert!(
        result.unwrap_err().contains("SQL_DYNAMIC"),
        "error should mention SQL_DYNAMIC"
    );
}

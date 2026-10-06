// ── Naryad №611 (issue #1060): schema-as-code references — P0 bugfix ──
//
// The bug (the Камертон Н1-03 wave matrix, reproduced on main c694b6d):
//   - `references(parent.id)`   — the REFERENCES clause was dropped
//     SILENTLY (the parser's split kept the dot inside a token, so
//     `parent.id` stayed ONE identifier → `idents.len() == 1` → the
//     modifier was discarded without diagnostics);
//   - `references( parent.id )` — the same silent loss;
//   - `references( parent . id )` — three tokens →
//     `References("parent", ".")` → the broken clause
//     `REFERENCES parent(.)` → SQL_ERROR at apply, `mlog check` green.
//
// The fix (two halves, one contract):
//   1. the parser tokenizes the qualified name with the dot as the
//      TABLE/FIELD SEPARATOR — every spacing of `t.f` yields exactly
//      the pair (t, f); anything else is a LOUD parse error;
//   2. `mlog check` validates the generated DDL on an in-memory SQLite
//      dry-run (the SSOT renderer `ast::schema_table_ddl`, the same
//      function the interpreter apply executes) — a DDL SQLite rejects
//      fails the check, never the apply.
//
// Done-when criteria (§2.5 of the naryad):
//   R1    — the applied DDL contains `REFERENCES parent(id)`;
//   R2/R2c — both spaced forms behave identically to the compact one;
//   check — a DDL rejected by SQLite is a check failure (this file,
//           the reserved-word table), never an apply-time surprise.

use metalogos::ast::{schema_table_ddl, ColumnModifier, SchemaDecl};
use metalogos::parser::parse;

/// Extract the `schema` declaration named `schema_name` from parsed
/// declarations (№611 test helper).
fn find_schema(decls: &[metalogos::ast::Declaration], schema_name: &str) -> SchemaDecl {
    for d in decls {
        if let metalogos::ast::Declaration::Schema(s) = d {
            if s.name == schema_name {
                return s.clone();
            }
        }
    }
    panic!("schema {} not found", schema_name);
}

/// Extract the modifiers of one column of one table (№611 test helper).
fn column_modifiers(schema: &SchemaDecl, table: &str, column: &str) -> Vec<ColumnModifier> {
    schema
        .tables
        .iter()
        .find(|t| t.name == table)
        .unwrap_or_else(|| panic!("table {} not found", table))
        .columns
        .iter()
        .find(|c| c.name == column)
        .unwrap_or_else(|| panic!("column {} not found", column))
        .modifiers
        .clone()
}

const SCHEMA_PROGRAM_TEMPLATE: &str = r#"
db { url: "sqlite::memory:" }

schema app {
  table parent {
    id: Int primary_key auto_increment
  }
  table child {
    id: Int primary_key auto_increment,
    parent_id: Int REFERENCES_FORM
  }
}
"#;

/// №611 R1: the compact form `references(parent.id)` parses to the
/// real pair — the modifier is no longer dropped silently.
#[test]
fn n611_r1_compact_form_yields_the_reference_pair() {
    let src = SCHEMA_PROGRAM_TEMPLATE.replace("REFERENCES_FORM", "references(parent.id)");
    let decls = parse(&src).unwrap();
    let schema = find_schema(&decls, "app");
    let mods = column_modifiers(&schema, "child", "parent_id");
    assert!(
        mods.contains(&ColumnModifier::References("parent".into(), "id".into())),
        "R1: the compact form must yield References(parent, id), got {:?}",
        mods
    );
}

/// №611 R2: the spaced form `references( parent.id )` behaves
/// IDENTICALLY to the compact one (previously: the same silent loss).
#[test]
fn n611_r2_spaced_form_yields_the_same_pair() {
    let src = SCHEMA_PROGRAM_TEMPLATE.replace("REFERENCES_FORM", "references( parent.id )");
    let decls = parse(&src).unwrap();
    let schema = find_schema(&decls, "app");
    let mods = column_modifiers(&schema, "child", "parent_id");
    assert!(
        mods.contains(&ColumnModifier::References("parent".into(), "id".into())),
        "R2: the spaced form must yield References(parent, id), got {:?}",
        mods
    );
}

/// №611 R2c: the form with spaces around the dot
/// `references( parent . id )` behaves identically too (previously:
/// `References("parent", ".")` → the broken `REFERENCES parent(.)`).
#[test]
fn n611_r2c_spaced_dot_form_yields_the_same_pair() {
    let src = SCHEMA_PROGRAM_TEMPLATE.replace("REFERENCES_FORM", "references( parent . id )");
    let decls = parse(&src).unwrap();
    let schema = find_schema(&decls, "app");
    let mods = column_modifiers(&schema, "child", "parent_id");
    assert!(
        mods.contains(&ColumnModifier::References("parent".into(), "id".into())),
        "R2c: the spaced-dot form must yield References(parent, id), got {:?}",
        mods
    );
}

/// №611 R1 (end-to-end): the SSOT-rendered DDL — the same string the
/// interpreter apply executes — contains the real FOREIGN KEY clause
/// `REFERENCES parent(id)` and is accepted by SQLite.
#[test]
fn n611_r1_rendered_ddl_carries_the_references_clause() {
    let src = SCHEMA_PROGRAM_TEMPLATE.replace("REFERENCES_FORM", "references(parent.id)");
    let decls = parse(&src).unwrap();
    let schema = find_schema(&decls, "app");
    let table = schema
        .tables
        .iter()
        .find(|t| t.name == "child")
        .expect("child table");

    let ddl = schema_table_ddl(table);
    assert!(
        ddl.contains("REFERENCES parent(id)"),
        "the applied DDL must carry the FK clause, got: {}",
        ddl
    );

    // The apply-surface contract: SQLite accepts the rendered DDL.
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch(&ddl).unwrap();
    let stored: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name = 'child'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(
        stored.contains("REFERENCES parent(id)"),
        "the DDL stored by SQLite must carry the FK clause, got: {}",
        stored
    );
}

/// №611: table order does not matter — a schema where `child` is
/// declared BEFORE `parent` still yields the FK clause (SQLite
/// resolves references at CREATE time without the parent present).
#[test]
fn n611_reference_target_may_be_declared_later() {
    let src = r#"
db { url: "sqlite::memory:" }

schema app {
  table child {
    id: Int primary_key auto_increment,
    parent_id: Int references(parent.id)
  }
  table parent {
    id: Int primary_key auto_increment
  }
}
"#;
    let decls = parse(src).unwrap();
    let schema = find_schema(&decls, "app");
    let mods = column_modifiers(&schema, "child", "parent_id");
    assert!(mods.contains(&ColumnModifier::References("parent".into(), "id".into())));

    // And the check dry-run stays green (the FK target missing at
    // CREATE time is legal SQLite — resolution happens at DML).
    let result = metalogos::check_program(src).unwrap();
    assert!(
        result.is_ok(),
        "unexpected check errors: {:?}",
        result.errors
    );
}

/// №611 check-side: a schema WITH references — `mlog check` green
/// (the dry-run accepts the SSOT-rendered DDL).
#[test]
fn n611_check_valid_references_schema_stays_green() {
    let src = SCHEMA_PROGRAM_TEMPLATE.replace("REFERENCES_FORM", "references(parent.id)");
    let result = metalogos::check_program(&src).unwrap();
    assert!(
        result.is_ok(),
        "unexpected check errors: {:?}",
        result.errors
    );
}

/// №611 check-side (the loud half): a DDL SQLite rejects is a CHECK
/// failure with the SQLite message and the rendered DDL — never an
/// apply-time surprise. `order` is a reserved SQL word: the rendered
/// `CREATE TABLE IF NOT EXISTS order (…)` is a syntax error for
/// SQLite; the pre-№611 check did not validate DDL at all and came
/// back green.
#[test]
fn n611_check_sqlite_rejected_ddl_fails_loud() {
    let src = r#"
db { url: "sqlite::memory:" }

schema app {
  table order { id: Int primary_key }
}
"#;
    let result = metalogos::check_program(src).unwrap();
    assert!(
        !result.is_ok(),
        "a DDL rejected by SQLite must fail the check"
    );
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.message.contains("rejected by SQLite")),
        "the error must carry the dry-run diagnosis, got: {:?}",
        result.errors
    );
}

/// №611 armor: the fail-closed parse error — a references modifier
/// that does not decompose into exactly (table, field) is a LOUD
/// parse error, never a silent drop. (Today's grammar only admits
/// `IDENT DOT_CHAR IDENT`, so this guards the loosening case; the
/// defensive branch is the parser's contract, not the grammar's.)
#[test]
fn n611_malformed_references_is_a_loud_parse_error() {
    // Construct the malformed shape through a loosened form: any
    // references with != 2 identifiers must not parse. The grammar
    // currently rejects `references(parent)` before the parser branch
    // runs — the assert documents the DOUBLE gate (grammar + parser):
    // no path may produce a silent drop.
    let src = SCHEMA_PROGRAM_TEMPLATE.replace("REFERENCES_FORM", "references(parent)");
    assert!(
        parse(&src).is_err(),
        "references(parent) must not parse into a silent drop"
    );
}

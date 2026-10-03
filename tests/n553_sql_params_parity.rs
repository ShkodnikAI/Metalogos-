// ── tests/n553_sql_params_parity.rs ──────────────────────────────────
// №553 (Wave 25 P1; the audit 02.10 §3.3 — position А on the SQL
// pairs): a non-List params argument is a LOUD refusal on BOTH
// backends — the same text, the same outcome.
//
// THE MUTATION (red-before by construction): `db_execute_vm` used to
// end its params match with `_ => Vec::new()` — a non-List second
// argument was SILENTLY no-params on the serve-default backend. The
// audit's example — `db_execute("UPDATE accounts SET frozen = 1 WHERE
// id = ?1", user_id)` — then executed with the placeholder UNBOUND (a
// rusqlite parameter-count riddle) or, on a placeholder-free
// statement, silently IGNORED the argument; the TW lane refused loudly
// ("second argument must be List, got {}"). The same silent arm lived
// in `db_execute_with_grant_vm` for the third argument. The №553
// collapse onto the shared DbAccess bodies keeps the LOUD branch on
// both lanes; these tests pin the parity — on the pre-№553 revision
// the VM half of every assertion here FAILS (the refusal text
// differs from the TW's, or the statement executes).
//
// Parity = the IDENTICAL refusal text on both backends (the №474
// test-file posture: one contract, not two compatible behaviors).

fn tw_out(source: &str) -> Result<String, String> {
    metalogos::run_program(source).map(|o| o.unwrap_or_default().trim_end().to_string())
}

fn vm_out(source: &str) -> Result<String, String> {
    let program = metalogos::compile_program(source)?;
    metalogos::run_bytecode(program).map(|o| o.unwrap_or_default().trim_end().to_string())
}

fn assert_parity_error(source: &str, fragment: &str, what: &str) {
    for (backend, res) in [("TW", tw_out(source)), ("VM", vm_out(source))] {
        let err = res.expect_err(&format!(
            "{what}: {backend} must refuse loudly (the №553 contract)"
        ));
        assert!(
            err.contains(fragment),
            "{what}: {backend} refusal must name the contract ('{fragment}'), got: {err}"
        );
    }
}

/// The AUDIT EXAMPLE verbatim (the placeholder shape): a Float second
/// argument against `... WHERE id = ?1` — the loud List refusal, NOT a
/// silent no-params execute and NOT a parameter-count riddle.
#[test]
fn n553_audit_example_placeholder_sql_refused_identically() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern AuditRepro(uid: Float) -> String {
  db_execute("CREATE TABLE accounts (id REAL, frozen REAL)", [])
  db_execute("INSERT INTO accounts VALUES (42.0, 0.0)", [])
  db_execute("UPDATE accounts SET frozen = 1 WHERE id = ?1", uid)
  return "executed"
}
flow Main { input: Float = 42.0 -> AuditRepro -> output }
"#;
    assert_parity_error(
        src,
        "db_execute() second argument must be List, got Float",
        "the audit example (placeholder SQL)",
    );
}

/// The audit's second shape: a placeholder-FREE statement — the
/// non-List argument used to be SILENTLY IGNORED on the VM lane; now
/// both lanes refuse with the same loud text.
#[test]
fn n553_no_placeholder_sql_refused_identically() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern FreezeAll(note: String) -> String {
  db_execute("CREATE TABLE accounts (id REAL, frozen REAL)", [])
  db_execute("INSERT INTO accounts VALUES (1.0, 0.0)", [])
  db_execute("UPDATE accounts SET frozen = 1", note)
  return "executed"
}
flow Main { input: String = "not-a-list" -> FreezeAll -> output }
"#;
    assert_parity_error(
        src,
        "db_execute() second argument must be List, got String",
        "the audit example (placeholder-free SQL)",
    );
}

/// The grant twin: a non-List THIRD argument to
/// `db_execute_with_grant` — the same loud refusal on both backends
/// (the former silent `Vec::new()` arm in `db_execute_with_grant_vm`).
#[test]
fn n553_grant_twin_non_list_params_refused_identically() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern GrantedWipe(uid: Float) -> String {
  db_execute("CREATE TABLE t (id REAL)", [])
  let g = grant_issue("db:delete:t", 3600)
  db_execute_with_grant(g, "DELETE FROM t WHERE id = ?1", uid)
  return "executed"
}
flow Main { input: Float = 7.0 -> GrantedWipe -> output }
"#;
    assert_parity_error(
        src,
        "db_execute_with_grant() third argument must be List, got Float",
        "the grant twin",
    );
}

/// The positive control: a proper List still binds and executes on
/// both backends (the UPDATE hits exactly one row → "1" on TW and VM).
#[test]
fn n553_list_params_still_execute_in_parity() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern ThawOne(uid: Float) -> String {
  db_execute("CREATE TABLE accounts (id REAL, frozen REAL)", [])
  db_execute("INSERT INTO accounts VALUES (42.0, 0.0)", [])
  db_execute("INSERT INTO accounts VALUES (43.0, 0.0)", [])
  let n = db_execute("UPDATE accounts SET frozen = 1 WHERE id = ?", [uid])
  return n
}
flow Main { input: Float = 42.0 -> ThawOne -> output }
"#;
    let tw = tw_out(src).unwrap_or_else(|e| panic!("the List path must work on TW, got: {e}"));
    let vm = vm_out(src).unwrap_or_else(|e| panic!("the List path must work on VM, got: {e}"));
    assert_eq!(tw, "1", "TW: exactly one row affected");
    assert_eq!(vm, "1", "VM: exactly one row affected (parity)");
}

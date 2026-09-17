// ── Naryad #390 (P0, security/action): Grant algebra contract tests ────
//
// Verifies ADR-0155 as implemented by src/grants.rs + the five builtins:
//   - runtime algebra: issue/use/subgrant/revoke with typed errors
//     (GRANT_REUSED / GRANT_EXHAUSTED / GRANT_EXPIRED / GRANT_REVOKED /
//     GRANT_ESCALATION / GRANT_SCOPE_MISMATCH);
//   - static Once-linearity: [GRANT_REUSED] compile error, zero findings
//     on legitimate programs;
//   - fail-closed compatibility: the ungranted destructive-SQL deny keeps
//     its IRREVERSIBLE_NO_GRANT class (№325 leak-suite vocabulary);
//   - opacity: Grant is non-printable, serde emits a dead "[GRANT]" marker;
//   - the w2_grant_linear example passes on BOTH backends with identical
//     output, and the .error contract refuses the Once-reuse program.

use std::path::Path;

use metalogos::grants::{self, GrantClass, GrantHandle};

const MANIFEST: &str = env!("CARGO_MANIFEST_DIR");

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source.trim(), Path::new(MANIFEST).to_path_buf())
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source.trim()).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(Path::new(MANIFEST).to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

// ── Runtime algebra (ledger API) ───────────────────────────────────────

#[test]
fn once_grant_consumed_by_use_then_reused_is_typed_error() {
    let g = grants::issue("db:delete:users", 3600, &GrantClass::Once, "t").unwrap();
    assert_eq!(grants::state_of(&g.grant_id), Some(("active".into(), 1)));
    let remaining = grants::grant_use(&g, "first").unwrap();
    assert_eq!(remaining, 0);
    assert_eq!(grants::state_of(&g.grant_id), Some(("consumed".into(), 0)));
    let err = grants::grant_use(&g, "second").unwrap_err();
    assert!(err.starts_with("GRANT_REUSED"), "got: {}", err);
}

#[test]
fn n_quota_exhaustion_is_typed_error() {
    let g = grants::issue("db:delete:sessions", 3600, &GrantClass::N(2), "t").unwrap();
    assert_eq!(grants::grant_use(&g, "u1").unwrap(), 1);
    assert_eq!(grants::grant_use(&g, "u2").unwrap(), 0);
    let err = grants::grant_use(&g, "u3").unwrap_err();
    assert!(err.starts_with("GRANT_EXHAUSTED"), "got: {}", err);
}

#[test]
fn born_expired_grant_refuses_with_grant_expired() {
    // ttl 0 = already expired (deterministic; the DSL surface documents
    // ttl >= 1, this shape exists for the expiry contract).
    let g = grants::issue("db:delete:users", 0, &GrantClass::Once, "t").unwrap();
    let err = grants::grant_use(&g, "late").unwrap_err();
    assert!(err.starts_with("GRANT_EXPIRED"), "got: {}", err);
}

#[test]
fn revoke_is_cascading_and_refuses_later_uses() {
    let parent = grants::issue("db:*:*", 3600, &GrantClass::Unlimited, "t").unwrap();
    let child = grants::subgrant(&parent, "db:delete:users", 600, &GrantClass::Once).unwrap();
    let grandchild = grants::subgrant(&child, "db:delete:users", 60, &GrantClass::Once).unwrap();
    let n = grants::revoke(&parent, "test").unwrap();
    assert_eq!(n, 3, "parent + child + grandchild must all flip");
    for h in [&child, &grandchild] {
        let err = grants::grant_use(h, "after revoke").unwrap_err();
        assert!(err.starts_with("GRANT_REVOKED"), "got: {}", err);
    }
    // Revoking again is legal and revokes nothing new.
    assert_eq!(grants::revoke(&parent, "again").unwrap(), 0);
}

#[test]
fn subgrant_widening_attempts_are_escalation_errors() {
    let unlimited = grants::issue("db:delete:users", 3600, &GrantClass::Unlimited, "t").unwrap();

    // scope widening
    let err = grants::subgrant(&unlimited, "db:*:*", 60, &GrantClass::Once).unwrap_err();
    assert!(err.starts_with("GRANT_ESCALATION"), "scope: {}", err);

    // ttl extension beyond the parent horizon
    let err = grants::subgrant(&unlimited, "db:delete:users", 7200, &GrantClass::Once).unwrap_err();
    assert!(err.starts_with("GRANT_ESCALATION"), "ttl: {}", err);

    // class power increase (N parent -> Unlimited child)
    let n_parent = grants::issue("db:delete:users", 3600, &GrantClass::N(1), "t").unwrap();
    let err =
        grants::subgrant(&n_parent, "db:delete:users", 60, &GrantClass::Unlimited).unwrap_err();
    assert!(err.starts_with("GRANT_ESCALATION"), "power: {}", err);

    // quota overflow (N(1) parent -> N(2) child)
    let err = grants::subgrant(&n_parent, "db:delete:users", 60, &GrantClass::N(2)).unwrap_err();
    assert!(err.starts_with("GRANT_ESCALATION"), "quota: {}", err);
}

#[test]
fn subgranting_a_once_parent_consumes_it_linear_transfer() {
    let parent = grants::issue("db:delete:users", 3600, &GrantClass::Once, "t").unwrap();
    let child = grants::subgrant(&parent, "db:delete:users", 60, &GrantClass::Once).unwrap();
    assert_eq!(
        grants::state_of(&parent.grant_id),
        Some(("consumed".into(), 0))
    );
    let err = grants::grant_use(&parent, "after transfer").unwrap_err();
    assert!(err.starts_with("GRANT_REUSED"), "got: {}", err);
    // The child still holds the single use.
    assert!(grants::grant_use(&child, "via child").is_ok());
}

#[test]
fn n_parent_quota_is_debited_by_children_conservation() {
    let parent = grants::issue("db:delete:users", 3600, &GrantClass::N(3), "t").unwrap();
    let c1 = grants::subgrant(&parent, "db:delete:users", 60, &GrantClass::N(2)).unwrap();
    let c2 = grants::subgrant(&parent, "db:delete:users", 60, &GrantClass::Once).unwrap();
    // parent remaining: 3 - 2 - 1 = 0; a third child must refuse.
    assert_eq!(grants::state_of(&parent.grant_id).unwrap().1, 0);
    let err = grants::subgrant(&parent, "db:delete:users", 60, &GrantClass::Once).unwrap_err();
    // At zero remaining the exhaustion check fires before the debit math —
    // either typed refusal is fail-closed; both are correct here.
    assert!(
        err.starts_with("GRANT_ESCALATION") || err.starts_with("GRANT_EXHAUSTED"),
        "got: {}",
        err
    );
    // The carved-out children hold exactly the issued quota: 2 + 1 uses.
    assert_eq!(grants::grant_use(&c1, "u1").unwrap(), 1);
    assert_eq!(grants::grant_use(&c1, "u2").unwrap(), 0);
    assert!(grants::grant_use(&c2, "u3").is_ok());
}

// ── Opacity (ADR-0155 §3.3 rule 2) ─────────────────────────────────────

#[test]
fn grant_is_nonprintable_and_serde_is_a_dead_marker() {
    let g = grants::issue("db:delete:users", 3600, &GrantClass::Once, "t").unwrap();
    let v = g.to_value();
    assert!(metalogos::interpreter::values::is_nonprintable(&v));
    assert_eq!(v.type_name(), "Grant");
    // serde never persists the capability — only the dead marker.
    let json = serde_json::to_string(&g).unwrap();
    assert_eq!(json, "\"[GRANT]\"");
    // A deserialized grant is a tombstone: expired on arrival.
    let back: GrantHandle = serde_json::from_str(&json).unwrap();
    let err = grants::grant_use(&back, "revived").unwrap_err();
    assert!(
        err.starts_with("GRANT_"),
        "a deserialized grant must refuse every use, got: {}",
        err
    );
}

// ── The granted action surface (both backends) ──────────────────────────

const GRANTED_PROGRAM: &str = r#"
db { url: "sqlite::memory:" }
pattern G(_t: String) -> String {
  db_execute("CREATE TABLE IF NOT EXISTS t1 (id INTEGER PRIMARY KEY)")
  db_execute("INSERT INTO t1 (id) VALUES (1)")
  let g = grant_issue("db:delete:t1", 3600, "n", 3)
  let n1 = db_execute_with_grant(g, "DELETE FROM t1")
  let n2 = grant_use(g)
  return "ok:" + n1 + "|" + to_string(n2)
}
flow Main { input: String = "t" -> G -> output }
"#;

#[test]
fn granted_action_runs_on_both_backends_identically() {
    let tw = run_tw(GRANTED_PROGRAM).expect("TW run");
    let vm = run_vm(GRANTED_PROGRAM).expect("VM run");
    let tw = tw.expect("output");
    let vm = vm.expect("output");
    // N(3): the delete consumes use 1 (remaining 2, the [GRANT_USE] event),
    // grant_use consumes use 2 and returns the remaining 1.
    assert_eq!(
        tw.trim(),
        "ok:1|1",
        "n1=1 affected, n2=1 remaining — got: {}",
        tw
    );
    assert_eq!(tw, vm, "TW↔VM parity for the granted action");
}

#[test]
fn grant_scope_mismatch_refuses_at_runtime() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern M(_t: String) -> String {
  db_execute("CREATE TABLE IF NOT EXISTS t1 (id INTEGER PRIMARY KEY)")
  db_execute("CREATE TABLE IF NOT EXISTS t2 (id INTEGER PRIMARY KEY)")
  let g = grant_issue("db:delete:t1", 3600)
  return db_execute_with_grant(g, "DELETE FROM t2")
}
flow Main { input: String = "t" -> M -> output }
"#;
    let err = run_tw(src).expect_err("must refuse");
    assert!(err.contains("GRANT_SCOPE_MISMATCH"), "got: {}", err);
}

#[test]
fn non_destructive_sql_under_grant_does_not_consume() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern S(_t: String) -> String {
  db_execute("CREATE TABLE IF NOT EXISTS t1 (id INTEGER PRIMARY KEY)")
  let g = grant_issue("db:delete:t1", 3600, "n", 1)
  let _ = db_execute_with_grant(g, "INSERT INTO t1 (id) VALUES (7)")
  let remaining = grant_use(g)
  return "remaining:" + to_string(remaining)
}
flow Main { input: String = "t" -> S -> output }
"#;
    let out = run_tw(src).expect("run").expect("output");
    assert_eq!(
        out.trim(),
        "remaining:0",
        "quota intact before the manual use"
    );
}

// ── Fail-closed compatibility (№325 unchanged) ──────────────────────────

#[test]
fn ungranted_destructive_sql_still_denies_with_irreversible_no_grant() {
    // The №325 content gate covers the schema-destroying forms
    // (DROP/TRUNCATE — see semantic.rs sink_clearance_violations);
    // DELETE/ALTER stay in parameterized-CRUD/SQL_DYNAMIC territory.
    let src = r#"
pattern Bad(_t: String) -> String {
  let _ = db_execute("DROP TABLE users")
  return "x"
}
flow Main { input: String = "t" -> Bad -> output }
"#;
    let decls = metalogos::parser::parse(src.trim()).unwrap();
    let findings = metalogos::audit::audit_category_a(&decls, "");
    assert!(
        findings
            .iter()
            .any(|f| f.check_id == "IRREVERSIBLE_NO_GRANT"),
        "the ungranted deny must stay: {:?}",
        findings.iter().map(|f| f.check_id).collect::<Vec<_>>()
    );
}

// ── Static Once-linearity (GRANT_REUSED at compile time) ────────────────

#[test]
fn once_reuse_is_a_compile_error_and_clean_program_passes() {
    let reuse = r#"
pattern D(_t: String) -> String {
  let g = grant_issue("db:delete:users", 3600)
  let a = db_execute_with_grant(g, "DELETE FROM users WHERE id = 1")
  let b = db_execute_with_grant(g, "DELETE FROM users WHERE id = 2")
  return a
}
flow Main { input: String = "t" -> D -> output }
"#;
    let decls = metalogos::parser::parse(reuse.trim()).unwrap();
    let findings = metalogos::audit::audit_category_a(&decls, "");
    assert!(
        findings.iter().any(
            |f| f.check_id == "GRANT_REUSED" && f.severity == metalogos::audit::Severity::Error
        ),
        "the reuse must be a compile error: {:?}",
        findings.iter().map(|f| f.check_id).collect::<Vec<_>>()
    );

    // The same shape with an N(n) grant is runtime-managed, NOT a static error.
    let quota = reuse.replace(
        "grant_issue(\"db:delete:users\", 3600)",
        "grant_issue(\"db:delete:users\", 3600, \"n\", 5)",
    );
    let decls = metalogos::parser::parse(quota.trim()).unwrap();
    let findings = metalogos::audit::audit_category_a(&decls, "");
    assert!(
        findings.iter().all(|f| f.check_id != "GRANT_REUSED"),
        "N(n) holdings are not statically linear: {:?}",
        findings
    );
}

#[test]
fn move_then_use_is_flagged_but_branch_merge_allows_exclusive_paths() {
    // use after MOVE (let g2 = g) — flagged.
    let moved = r#"
pattern Mv(_t: String) -> String {
  let g = grant_issue("db:delete:users", 3600)
  let g2 = g
  let _ = db_execute_with_grant(g, "DELETE FROM users")
  return "x"
}
flow Main { input: String = "t" -> Mv -> output }
"#;
    let decls = metalogos::parser::parse(moved.trim()).unwrap();
    let findings = metalogos::audit::audit_category_a(&decls, "");
    assert!(
        findings.iter().any(|f| f.check_id == "GRANT_REUSED"),
        "use after move must be flagged: {:?}",
        findings
    );

    // Exclusive branches (if/else) each using the grant ONCE — legal, and
    // the flow intersection must NOT flag it.
    let exclusive = r#"
pattern Ex(_t: String, c: Bool) -> String {
  let g = grant_issue("db:delete:users", 3600)
  if c then {
    let _ = db_execute_with_grant(g, "DELETE FROM users WHERE id = 1")
  } else {
    let _ = db_execute_with_grant(g, "DELETE FROM users WHERE id = 2")
  }
  return "x"
}
flow Main { input: String = "t" -> Ex -> output }
"#;
    let decls = metalogos::parser::parse(exclusive.trim()).unwrap();
    let findings = metalogos::audit::audit_category_a(&decls, "");
    assert!(
        findings.iter().all(|f| f.check_id != "GRANT_REUSED"),
        "exclusive-branch single uses are legal: {:?}",
        findings
    );
}

// ── Wave examples (both backends + contracts) ───────────────────────────

#[test]
fn w2_grant_linear_example_passes_on_both_backends() {
    let src = std::fs::read_to_string(Path::new(MANIFEST).join("examples/w2_grant_linear.mlog"))
        .expect("example exists");
    let expected =
        std::fs::read_to_string(Path::new(MANIFEST).join("examples/w2_grant_linear.expected"))
            .expect("expected fixture exists");
    let tw = run_tw(&src).expect("TW run").expect("output");
    let vm = run_vm(&src).expect("VM run").expect("output");
    assert_eq!(tw.trim(), expected.trim(), "TW output matches the fixture");
    assert_eq!(vm.trim(), expected.trim(), "VM output matches the fixture");
}

#[test]
fn w2_grant_linear_reuse_error_contract() {
    let result = metalogos::check_program_with_root(
        &std::fs::read_to_string(Path::new(MANIFEST).join("examples/w2_grant_linear_reuse.mlog"))
            .unwrap(),
        None,
    )
    .expect("analysis");
    let text = result.format();
    assert!(
        text.contains("GRANT_REUSED"),
        "the .error contract must name GRANT_REUSED: {}",
        text
    );
}

// ── Builtin argument discipline ────────────────────────────────────────

#[test]
fn builtin_arg_validation_is_loud() {
    // unknown class word
    let err = run_tw(
        r#"
pattern A(_t: String) -> String {
  let g = grant_issue("db:delete:users", 3600, "forever")
  return "x"
}
flow Main { input: String = "t" -> A -> output }
"#,
    )
    .unwrap_err();
    assert!(err.contains("unknown class"), "got: {}", err);

    // class "n" without a uses count
    let err = run_tw(
        r#"
pattern B(_t: String) -> String {
  let g = grant_issue("db:delete:users", 3600, "n")
  return "x"
}
flow Main { input: String = "t" -> B -> output }
"#,
    )
    .unwrap_err();
    assert!(err.contains("uses count"), "got: {}", err);

    // grant argument must be a Grant
    let err = run_tw(
        r#"
pattern C(_t: String) -> String {
  let n = grant_use("not-a-grant")
  return "x"
}
flow Main { input: String = "t" -> C -> output }
"#,
    )
    .unwrap_err();
    assert!(err.contains("must be a Grant"), "got: {}", err);
}

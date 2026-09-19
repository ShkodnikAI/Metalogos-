// ── Naryad #397 follow-up: gate parity + VM journal parity ────────────
//
// The wave-3 acceptance e2e (examples/w2_kitchen_camera_capability.mlog,
// tests/wave3_kitchen_camera_e2e.rs) landed in main; an independent
// re-execution of the naryad surfaced two defects the merged coverage
// could not see:
//
//   1. STATIC GATE VOCABULARY: the irreversible-content matcher covered
//      only drop-table/drop-database/drop-index/truncate while the audit
//      doc comment, REFERENCE.md and the runtime twin
//      (grants.rs::extract_destructive_ops) claimed
//      DROP/DELETE/TRUNCATE/ALTER — a bare db_execute("DELETE FROM …")
//      compiled clean and executed UNGRANTED (the bare runtime path
//      trusts the static gate — there is no second runtime gate on it).
//      Fixed in src/semantic.rs; the №152 check realignment rides in
//      tests/check_integration.rs (n397_destructive_literal_needs_a_grant).
//
//   2. VM JOURNAL PARITY: the VM's granted destructive-SQL success path
//      wrote grant.used but skipped the irreversible.db_execute journal
//      record the TW path writes (src/interpreter/db.rs) — the signed
//      trail on the VM backend missed the action itself. The merged e2e
//      could not see it: TW and VM records share one process ledger and
//      the content assertions were not per-run — the TW record satisfied
//      the VM run's assertion. Fixed in src/vm.rs; the regression below
//      asserts the VM-side record with a per-run count delta.
//
// The runtime ledger is process-global — per-action count deltas are the
// house rule (№393); per-RUN deltas are this file's addition.

use metalogos::audit::{audit_category_a, Severity};
use metalogos::ledger::verify_file;
use metalogos::parser;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn audit_errors(src: &str) -> Vec<String> {
    let declarations = parser::parse(src).expect("parse");
    audit_category_a(&declarations, "")
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .map(|f| format!("[{}] {}", f.check_id, f.message))
        .collect()
}

fn action_snapshot() -> HashMap<String, u64> {
    let mut map: HashMap<String, u64> = HashMap::new();
    for r in metalogos::ledger::all_records().expect("readable ledger") {
        *map.entry(r.action.clone()).or_insert(0) += 1;
    }
    map
}

fn delta(action: &str, before: &HashMap<String, u64>) -> u64 {
    let now = action_snapshot();
    now.get(action).copied().unwrap_or(0) - before.get(action).copied().unwrap_or(0)
}

#[test]
fn vm_backend_journals_the_granted_irreversible_action() {
    // Run the merged kitchen-camera e2e on the VM backend with a per-run
    // baseline: the granted DELETE must leave its OWN irreversible record
    // on the VM path — not ride the TW run's record in a shared process
    // ledger.
    let src = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/w2_kitchen_camera_capability.mlog"
    ))
    .expect("the merged wave-3 e2e example exists");

    let before = action_snapshot();
    let program = metalogos::compile_program(&src).expect("the e2e compiles");
    let out = metalogos::run_bytecode(program)
        .expect("the e2e runs on the VM")
        .expect("flow output");
    assert!(
        out.contains("deleted:1"),
        "the granted DELETE executed: {out}"
    );
    assert_eq!(
        delta("irreversible.db_execute", &before),
        1,
        "the VM granted DELETE must journal its own irreversible.db_execute record (TW/VM parity, №393 §3.4)"
    );
    assert_eq!(
        delta("grant.issued", &before),
        1,
        "the VM grant lifecycle is journaled too"
    );
    assert_eq!(
        delta("deny.IRREVERSIBLE_NO_GRANT", &before),
        1,
        "the VM deny event is journaled (already covered in main — pinned here per-run)"
    );

    // The VM-side export verifies externally (the genesis-seq contract
    // from the ledger fix rides the same chain).
    verify_file(
        Path::new("target/w2_kitchen_camera_export.jsonl"),
        None,
        None,
    )
    .expect("the VM-era export verifies externally");
}

#[test]
fn static_gate_covers_the_full_destructive_vocabulary() {
    // The documented vocabulary (audit.rs comment + REFERENCE.md) and the
    // runtime twin (extract_destructive_ops) agree: DELETE and ALTER are
    // destructive — the static gate must refuse them exactly like DROP.
    for sql in [
        "DELETE FROM motion_events",
        "ALTER TABLE motion_events DROP COLUMN ts",
        "DROP TABLE motion_events",
        "TRUNCATE TABLE motion_events",
    ] {
        let src = format!(
            r#"
pattern Sweep(_tick: String) -> String {{
  let _ = db_execute("{sql}")
  return "done"
}}
flow Main {{ input: String = "tick" -> Sweep -> output }}
"#
        );
        let errs = audit_errors(&src);
        assert!(
            errs.iter().any(|m| m.contains("IRREVERSIBLE_NO_GRANT")),
            "bare db_execute({sql:?}) must be denied statically: {errs:?}"
        );
    }
    // The granted path is the legal route — the static gate does not
    // deny db_execute_with_grant (the runtime grant algebra meters it).
    let granted = r#"
db { url: "sqlite::memory:" }
pattern Sweep(_tick: String) -> String {
  let g = grant_issue("db:delete:motion_events", 60, "n", 1)
  return db_execute_with_grant(g, "DELETE FROM motion_events")
}
flow Main { input: String = "tick" -> Sweep -> output }
"#;
    assert!(
        metalogos::check_program(granted).unwrap().is_ok(),
        "the granted destructive path stays legal: {:?}",
        metalogos::check_program(granted).unwrap().errors
    );
}

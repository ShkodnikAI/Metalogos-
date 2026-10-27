// ── tests/naryad_820_db_insert_surface.rs ─────────────────────────────
// Наряд №820 (issue #1588, wave 67, P2, the #1528 stub-audit follow-up):
// the registry row said `spec!("db_insert", 0, "stub", "experimental")`
// — handler: None, arity 0 — while the REAL surface has existed on both
// lanes since №409 (the lazy-open VM lane) and carries the №783 params
// SSOT: `db_insert(table, struct)` is a real parameterized INSERT over
// src/db_ops.rs returning last_insert_rowid (the special-cased DbAccess
// dispatch: src/interpreter/execution.rs + src/vm.rs). The #1528 audit
// predates that landing — the remaining dishonesty was the registry
// metadata and the REFERENCE sink row ("registry-only stub"), not the
// missing handler.
//
// The choice recorded (the #1528 "выбор — в отчёте" form): NOT the
// [NOT_IMPLEMENTED] refusal (the №784 authenticate form) and NOT the
// removal — the surface WORKS and REFERENCE already promises exactly
// what the code does; the fix is the registry row + the sink line
// catching up to the shipped reality, plus a registered context-less
// fallback (the builtin_db_execute lane-contract form) so the generic
// dispatch never answers "no handler" for a name that owns a real lane
// path.
//
// Pins (the №772 RED-before form):
//   T1: the registry honesty — db_insert's spec is arity 2 (exact),
//       category "db", a registered handler, tier experimental
//       (RED on unpatched main: category "stub", handler None, arity 0).
//   T2: the two-lane behavior the row now claims — db { sqlite::memory: }
//       + db_insert lands the row and returns the rowid, identical on
//       the TW lane and the real `mlog run` VM lane (green on main too —
//       this is the surface the honest row describes).

use metalogos::builtins::BUILTIN_REGISTRY;

fn db_insert_spec() -> &'static metalogos::builtins::BuiltinSpec {
    BUILTIN_REGISTRY
        .iter()
        .find(|s| s.name == "db_insert")
        .expect("db_insert must stay registered")
}

// ── T1: the registry honesty (RED on unpatched main) ──────────────────

#[test]
fn n820_registry_row_describes_the_real_surface() {
    let spec = db_insert_spec();
    assert_eq!(
        spec.arity, 2,
        "db_insert(table, struct) requires BOTH arguments — arity 0 (the variadic skip) is the stub lie"
    );
    assert_eq!(
        spec.max_arity, None,
        "the (String, Struct) shape has no optional tail"
    );
    assert_eq!(
        spec.category, "db",
        "a working parameterized INSERT is not a \"stub\" — the category feeds the tier gates and the REFERENCE §6 row"
    );
    assert!(
        spec.handler.is_some(),
        "the generic dispatch must never answer \"no handler\" for a name that owns a real lane path"
    );
    assert_eq!(spec.tier, "experimental", "the tier floor is only-up (№809) — this naryad does not move it");
}

// ── T2: the two-lane behavior the honest row claims ───────────────────

fn two_lane_program() -> String {
    r#"
db { url: "sqlite::memory:" }
pattern P(t: String) -> Float {
  let _init = db_execute("CREATE TABLE IF NOT EXISTS probe (name TEXT, score REAL)")
  let rowid = db_insert("probe", {name: t, score: 2.5})
  return rowid
}
pattern Q(prev: Float) -> String {
  let rows = db_query("SELECT name FROM probe WHERE score = 2.5")
  return get(get(rows, 0), 0)
}
flow Main { input: String = "x" -> P -> Q -> output }
"#
    .to_string()
}

#[test]
fn n820_insert_lands_the_row_and_returns_the_rowid_tw_lane() {
    let out = metalogos::run_program_with_dir(two_lane_program().trim(), std::path::PathBuf::from("."))
        .expect("the TW lane must run the insert program");
    let out = out.expect("the flow returns the Q output");
    assert_eq!(out, "x", "the inserted row must be readable back (name = the inserted value)");
}

#[test]
fn n820_insert_lands_the_row_and_returns_the_rowid_vm_lane() {
    let out = metalogos::run_program_on_vm(two_lane_program().trim())
        .expect("the VM lane must run the insert program");
    let out = out.expect("the flow returns the Q output");
    assert_eq!(out, "x", "VM: the inserted row must be readable back (the lane parity, №762)");
}

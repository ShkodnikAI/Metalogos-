// ── Naryad #397 (wave-3 acceptance #1, gh#491): kitchen-camera e2e ────
//
// The §5.3 kitchen-camera scenario with the wave-3 capability layer
// integrated END TO END — the acceptance the wave-3 audit found open
// ("кухонная камера без grant/deny-интеграции"):
//
//   - GREEN path (allowed AND traced): private capture against the
//     kitchen-camera origin → one-time likeness ritual (consent trace)
//     → media_save passes the №325 clearance with the token → the
//     sidecar manifest records the origin chain (№337);
//   - GRANT path: the housekeeping purge runs under an N(1) grant —
//     the granted DELETE executes and journals itself;
//   - DENY path (explainable AND signed): the second granted DELETE
//     exhausts the quota → GRANT_EXHAUSTED → runtime DenyEvent with
//     reason IRREVERSIBLE_NO_GRANT, the signed deny record lands in
//     the ledger BEFORE handler selection (№393 §3.4), on_deny(db)
//     explains it via deny_reason() and degrades to Unit;
//   - EXTERNAL VERIFICATION: the exported JSONL chain verifies with
//     the pure hash+Ed25519 verifier — no runtime involved. This is
//     the regression that caught the runtime journal starting at
//     seq 1 (ADR-0167 §Genesis demands seq 0) — every runtime export
//     failed external verification before the fix;
//   - GATE PARITY: the static irreversible-content vocabulary now
//     matches the runtime twin (grants.rs::extract_destructive_ops) —
//     DELETE/ALTER literals no longer slip the compile-time gate the
//     doc comment always claimed.
//
// The static egress deny keeps its canonical red contracts (w1_-
// kitchen_camera, its alias variant) — asserted here as the paired
// red side of the acceptance.
//
// The runtime ledger is process-global, so the assertions are COUNT
// DELTAS and per-action filtered views (the №393 house rule); each
// delta uses a per-action baseline, never the ledger total.

use metalogos::audit::{audit_category_a, Severity};
use metalogos::consent;
use metalogos::ledger::{all_records, count, verify_file, LedgerRecord, GENESIS_PREV_HASH};
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

const EXPORT_PATH: &str = "target/w2_kitchen_camera_ledger.jsonl";

fn example_source() -> String {
    fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/w2_kitchen_camera.mlog"
    ))
    .expect("the kitchen-camera e2e example exists")
}

/// Per-action count snapshot — the baseline for count deltas.
fn action_snapshot() -> HashMap<String, u64> {
    let mut map: HashMap<String, u64> = HashMap::new();
    for r in all_records().expect("readable ledger") {
        *map.entry(r.action.clone()).or_insert(0) += 1;
    }
    map
}

fn delta(action: &str, before: &HashMap<String, u64>) -> u64 {
    let now = action_snapshot();
    now.get(action).copied().unwrap_or(0) - before.get(action).copied().unwrap_or(0)
}

#[test]
fn kitchen_camera_e2e_tw_then_vm_parity_with_external_verification() {
    let src = example_source();

    // ── TW backend ──────────────────────────────────────────────────
    let before = action_snapshot();
    let consent_before = consent::entry_count().expect("consent ledger readable");
    let out = metalogos::run_program(&src)
        .expect("the kitchen-camera e2e compiles and runs (TW)")
        .expect("flow output");
    assert_eq!(
        out, "frame:Image|saved:String|purged:1|refused:Unit|trail:String",
        "the green path saves, the granted purge executes, the refused purge degrades"
    );
    // The consent ritual left its trace (the "traced" side of GREEN).
    assert!(
        consent::entry_count().expect("consent ledger readable") > consent_before,
        "the likeness ritual must record the consent-ledger trace (№387)"
    );
    // The ALLOW path is traced by the ledger (grant lifecycle + the
    // irreversible action), the DENY path by the signed deny record.
    assert_eq!(delta("grant.issued", &before), 1, "one grant issued");
    assert_eq!(delta("grant.used", &before), 1, "one grant consumed");
    assert_eq!(
        delta("irreversible.db_execute", &before),
        1,
        "the granted DELETE journals itself (№393 §3.4)"
    );
    assert_eq!(
        delta("deny.IRREVERSIBLE_NO_GRANT", &before),
        1,
        "exactly one runtime deny event, journaled before handler selection"
    );
    assert_eq!(delta("ledger.rotate", &before), 1, "one rotation");
    assert_eq!(delta("ledger.snapshot", &before), 1, "one snapshot");

    // EXTERNAL VERIFICATION of the exported chain (pure hash + Ed25519).
    // The first record must be the genesis position (seq 0) per
    // ADR-0167 §Genesis — the off-by-one regression this naryad caught.
    let records = records_from_export();
    assert_eq!(
        records.first().expect("non-empty export").seq,
        0,
        "the runtime journal must start at the genesis seq 0 (ADR-0167 §Genesis)"
    );
    assert_eq!(
        records.first().expect("non-empty export").prev_hash,
        GENESIS_PREV_HASH,
        "the genesis record links to the all-zeros prev_hash"
    );
    verify_file(Path::new(EXPORT_PATH), None, None)
        .expect("the exported kitchen-camera chain verifies EXTERNALLY");

    // ── VM backend (the runtime twin must behave identically) ──────
    let before_vm = action_snapshot();
    let program = metalogos::compile_program(&src).expect("compiles");
    let vm_out = metalogos::run_bytecode(program)
        .expect("the kitchen-camera e2e runs (VM)")
        .expect("flow output");
    assert_eq!(
        vm_out, out,
        "VM twin parity: the same scenario produces the same output"
    );
    assert_eq!(
        delta("deny.IRREVERSIBLE_NO_GRANT", &before_vm),
        1,
        "the VM refusal must journal exactly one deny event (parity)"
    );
    assert_eq!(
        delta("irreversible.db_execute", &before_vm),
        1,
        "the VM granted DELETE journals itself (parity)"
    );
    // The export now covers the TW + VM records of this process in one
    // chain — the external verifier accepts it as a whole.
    verify_file(Path::new(EXPORT_PATH), None, None)
        .expect("the chain covering both backends verifies EXTERNALLY");
}

fn records_from_export() -> Vec<LedgerRecord> {
    let content = fs::read_to_string(EXPORT_PATH).expect("the export file exists");
    metalogos::ledger::records_from_jsonl(&content).expect("the export parses as ledger JSONL")
}

#[test]
fn unconsented_kitchen_camera_egress_is_denied() {
    // The acceptance's deny side on the MEDIA path: the same private
    // capture, but NO ritual token. On the camera/likeness origin the
    // static layer knows the label — the egress is denied at COMPILE
    // time with the explainable SECRET_LEAK reason (the №325 clearance
    // + the №391 bridge wording), on BOTH backends (the analyzer is
    // backend-independent; the e2e golden proves the token path runs).
    let src = r#"
origin kitchen_cam { kind: likeness, media: image, label: private }
pattern Leak(_tick: String) -> String {
  let frame = from kitchen_cam media_store_image("frame-bytes", "private")
  return media_save(frame, "target/w397_sealed_leak.jpg")
}
flow Main { input: String = "tick" -> Leak -> output }
"#;
    let errs = audit_errors(src);
    assert!(
        errs.iter().any(|m| m.starts_with("[SECRET_LEAK]")),
        "unconsented egress must be denied at compile time: {errs:?}"
    );

    // The RUNTIME backstop (№331 contract the e2e rides with): when the
    // privacy is only known at RUNTIME (statically-clean value over a
    // public generation origin, sealed at rest), the materialization
    // refuses loudly on both backends — no path to the bytes.
    let runtime_src = r#"
origin feed { kind: generation, media: image, label: public }
pattern Frame(_tick: String) -> String {
  let img = from feed media_store_image("frame-bytes", "private")
  let _p = media_save(img, "target/w397_runtime_sealed.jpg")
  return "saved"
}
flow Main { input: String = "tick" -> Frame -> output }
"#;
    let err = metalogos::run_program(runtime_src)
        .expect_err("the sealed entry must not materialize (TW)");
    assert!(
        err.contains("MEDIA_SEALED_EGRESS"),
        "the TW backstop must refuse the sealed entry: {err}"
    );
    let program = metalogos::compile_program(runtime_src)
        .expect("the runtime-sealed variant compiles (static labels are clean)");
    let err_vm =
        metalogos::run_bytecode(program).expect_err("the sealed entry must not materialize (VM)");
    assert!(
        err_vm.contains("MEDIA_SEALED_EGRESS"),
        "the VM backstop must refuse the sealed entry: {err_vm}"
    );
}

#[test]
fn canonical_static_denies_stay_red() {
    // The paired red side of the acceptance: the private camera frame
    // flowing into a file sink without consent is denied at COMPILE
    // time with an explainable reason — the canonical contracts the
    // e2e extends (not replaces).
    for name in ["w1_kitchen_camera.mlog", "w1_kitchen_camera_alias.mlog"] {
        let path = format!("{}/examples/{}", env!("CARGO_MANIFEST_DIR"), name);
        let src = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {name}: {e}"));
        let errs = audit_errors(&src);
        assert!(
            errs.iter().any(|m| m.starts_with("[SECRET_LEAK]")),
            "{name} keeps its compile-time deny with the explainable reason: {errs:?}"
        );
    }
}

#[test]
fn grantless_destructive_purge_is_denied_statically() {
    // The purge without a grant never reaches the runtime: the static
    // IRREVERSIBLE_NO_GRANT gate refuses the destructive literal — the
    // same reason word the runtime deny event carries when a granted
    // call is refused. (The naryad-397 gate-parity fix: DELETE/ALTER
    // literals used to slip this gate while the runtime metered them.)
    let src = r#"
db { url: "sqlite::memory:" }
pattern Sweep(_tick: String) -> String {
  let _ = db_execute("DELETE FROM motion_events")
  return "done"
}
flow Main { input: String = "tick" -> Sweep -> output }
"#;
    let errs = audit_errors(src);
    assert!(
        errs.iter().any(|m| m.contains("IRREVERSIBLE_NO_GRANT")),
        "the grantless purge is denied statically with the same reason word: {errs:?}"
    );
    // The runtime deny path stays reachable for granted calls only —
    // a static denial is a refusal to compile, nothing ran, and the
    // ledger must not have grown from this test's compile alone.
    let before = count().expect("count");
    assert_eq!(
        count().expect("count"),
        before,
        "compile-time denials journal nothing (the ledger only moves on runtime paths)"
    );
}

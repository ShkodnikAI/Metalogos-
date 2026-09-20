//! Naryad №397 — the kitchen-camera e2e (acceptance item 1 of the wave-3
//! dispatch): the canon §5.3 scenario over the FULL wave-3 runtime stack
//! (№390 grants + №391 bridge + №392 DenyEvent + №393 Ledger).
//!
//! Contract under test:
//! (а) a RUNTIME-sealed frame refuses materialization through the deny
//!     event layer — reason MEDIA_SEALED_EGRESS, class file; the on_deny
//!     handler degrades the call and the program continues; TW and VM
//!     agree on the degraded output (the №328 agreement principle);
//! (б) the exported ledger chain verifies EXTERNALLY (`ledger::verify_file`
//!     — pure hash + Ed25519, no Metalogos runtime state) and carries the
//!     sealed-egress deny record + the grant lifecycle records — every
//!     record a side effect of the action path itself (ADR-0167 §3.4);
//! (в) without a covering handler the sealed-egress refusal stays a loud
//!     coded error — deny behavior by default is unchanged (№392 posture);
//! (г) the DECLARED-private camera is still denied at compile time
//!     (SECRET_LEAK) — the static gate is unchanged; examples/
//!     w1_kitchen_camera.mlog remains the static half of the story.
//!
//! How to verify manually: `cargo test --test naryad_397_kitchen_camera_e2e`.

use std::path::Path;

const MANIFEST: &str = env!("CARGO_MANIFEST_DIR");

/// The canonical scenario (the golden example of the acceptance item).
const SCENARIO: &str = include_str!("../examples/w3_kitchen_camera.mlog");
/// The golden flow output.
const EXPECTED: &str = include_str!("../examples/w3_kitchen_camera.expected");

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

// ── (а) the sealed-egress refusal rides the deny-event layer ──────────

#[test]
fn n397_sealed_frame_refusal_degrades_and_continues_tw() {
    let out = run_tw(SCENARIO).expect("the handled refusal must not abort the program");
    assert_eq!(
        out.as_deref(),
        Some(EXPECTED.trim()),
        "the kitchen-camera e2e flow output (TW)"
    );
}

#[test]
fn n397_sealed_frame_refusal_degrades_and_continues_vm() {
    let out = run_vm(SCENARIO).expect("the handled refusal must not abort the program (VM)");
    assert_eq!(
        out.as_deref(),
        Some(EXPECTED.trim()),
        "TW and VM agree on the degraded continuation (№328)"
    );
}

// ── (б) the exported chain verifies externally and carries the trail ──

#[test]
fn n397_exported_chain_verifies_and_carries_the_deny_record() {
    // A per-test export path keeps this check race-free against the
    // golden suite (which runs the canonical export path).
    let unique = "target/w3_kitchen_camera_export_n397.jsonl";
    let program = SCENARIO.replace("target/w3_kitchen_camera_export.jsonl", unique);
    let out = run_tw(&program).expect("the scenario runs");
    assert_eq!(out.as_deref(), Some(EXPECTED.trim()));

    // EXTERNAL verification: pure hash + Ed25519 over the exported file
    // (the same contract `mlog ledger verify <file>` serves).
    let report = metalogos::ledger::verify_file(Path::new(unique), None, None)
        .expect("the exported chain must verify externally");
    assert!(
        report.records >= 6,
        "deny + grant + rotate + snapshot + … : {}",
        report.records
    );
    assert!(report.anchored_start || report.schema_version == 1);

    // The trail: the sealed-egress deny record is journaled with the
    // runtime/deny actor triple; the forensic detail rides the record's
    // args_hash (the export carries the hash, not the raw text — the
    // privacy posture of ADR-0167; whoever saw the event can re-verify).
    let content = std::fs::read_to_string(unique).expect("export exists");
    let records = metalogos::ledger::records_from_jsonl(&content).expect("parses");
    let seal = records
        .iter()
        .find(|r| r.action == "deny.MEDIA_SEALED_EGRESS")
        .expect("the sealed-egress deny is journaled");
    assert_eq!(seal.actor, "runtime");
    assert_eq!(seal.scope, "file");
    assert_eq!(seal.args_hash.len(), 64, "the SHA-256 detail anchor");
    // The office-shaped action: the grant lifecycle + the exhausted-quota
    // deny are journaled in the same chain.
    assert!(
        records.iter().any(|r| r.action == "grant.issued"),
        "grant.issued present"
    );
    assert!(
        records
            .iter()
            .any(|r| r.action == "deny.IRREVERSIBLE_NO_GRANT"),
        "the exhausted-quota deny is journaled"
    );
    assert!(
        records.iter().any(|r| r.action == "ledger.rotate"),
        "the key rotation is journaled"
    );
    let _ = std::fs::remove_file(unique);
}

// ── (в) no handler → the loud coded refusal, unchanged ────────────────

#[test]
fn n397_sealed_egress_without_handler_stays_loud() {
    let bare = SCENARIO.replace(
        r#"on_deny(*) {
  match deny_reason() {
    "MEDIA_SEALED_EGRESS" then { print("deny:MEDIA_SEALED_EGRESS") }
    "IRREVERSIBLE_NO_GRANT" then { print("deny:IRREVERSIBLE_NO_GRANT") }
    else { print("deny:other") }
  }
}"#,
        "",
    );
    let err = run_tw(&bare).expect_err("an uncovered sealed-egress refusal stays loud");
    assert!(
        err.contains("[MEDIA_SEALED_EGRESS]"),
        "the coded refusal surfaces unchanged: {err}"
    );
    assert!(
        err.contains("declared sensitivity 'private'"),
        "the refusal explains itself: {err}"
    );
}

// ── (г) the static half is untouched ──────────────────────────────────

#[test]
fn n397_declared_private_camera_is_still_denied_statically() {
    let w1 = std::fs::read_to_string(Path::new(MANIFEST).join("examples/w1_kitchen_camera.mlog"))
        .expect("the static-half fixture exists");
    let err = metalogos::compile_program(w1.trim())
        .expect_err("the declared-private camera must not compile");
    assert!(
        err.contains("SECRET_LEAK"),
        "the static class is unchanged: {err}"
    );
}

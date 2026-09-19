// ── Wave-3 acceptance №1 (dispatch gh#491): the kitchen camera e2e ───
//
// The composed end-to-end contract: origin → consent → egress of private
// media, grant-metered irreversible actions, an explainable runtime deny
// and the SIGNED, EXTERNALLY VERIFIABLE ledger trail — one scenario, both
// backends (the four building blocks w1_kitchen_camera + w2_grant_linear
// + w2_deny_exhaustive + w2_ledger, integrated).
//
// What the e2e integration caught (the reason this naryad exists):
//   1. The layers DISAGREED on consent: the static audit accepted a
//      consent scope as the media_save credential (№387 generalized
//      egress) while the runtime backstop knew only the likeness token —
//      a consent-clean compile was sealed at runtime
//      (MEDIA_SEALED_EGRESS) and the green path could not run. Fixed
//      (№397): the granted scope lands ON THE STORE ENTRY
//      (consent_grant_dispatch) and the backstop honors it;
//      consent_revoke withdraws it (the runtime twin of the flat
//      cascade).
//   2. The runtime ledger STARTED AT SEQ 1 with a genesis prev_hash —
//      the external verifier refuses any chain not starting at seq 0,
//      so EVERY fresh runtime export was unverifiable and the dispatch's
//      "external verification passes" could never hold. Fixed (№397):
//      the genesis record takes seq 0. The library tests never caught
//      it: they build synthetic chains via build_chain and never reach
//      the runtime append path.
//
// The ledger is process-global: assertions are monotone COUNT DELTAS and
// filtered views (the n393 template) — never absolute counts.

use std::fs;
use std::path::Path;
use std::process::Command;

const MANIFEST: &str = env!("CARGO_MANIFEST_DIR");

fn run_tw(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source.trim(), base_dir.to_path_buf())
}

fn run_vm(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source.trim()).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir.to_path_buf());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

fn audit_errors(src: &str) -> Vec<String> {
    use metalogos::audit::{audit_category_a, AuditFinding, Severity};
    let declarations = metalogos::parser::parse(src.trim()).expect("parse");
    audit_category_a(&declarations, "")
        .iter()
        .filter(|f: &&AuditFinding| f.severity == Severity::Error)
        .map(|f| format!("[{}] {}", f.check_id, f.message))
        .collect()
}

fn ledger_kind_delta(kind: &str, before: usize) -> usize {
    let after = metalogos::ledger::all_records()
        .expect("ledger reads")
        .iter()
        .filter(|r| r.action == kind || r.action.starts_with(&format!("{}.", kind)))
        .count();
    after.saturating_sub(before)
}

fn count_kind(kind: &str) -> usize {
    metalogos::ledger::all_records()
        .expect("ledger reads")
        .iter()
        .filter(|r| r.action.starts_with(kind))
        .count()
}

// ── 1. The canonical deny: static gate and the DenyEvent dictionary ──
//    share one vocabulary (the explainable deny, №392 SSOT).

#[test]
fn n397_layers_agree_on_the_canonical_deny() {
    let src = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/w1_kitchen_camera.mlog"
    ))
    .expect("the canonical example reads");
    let errs = audit_errors(&src);
    assert!(
        errs.iter().any(|m| m.starts_with("[SECRET_LEAK]")),
        "the kitchen camera keeps its compile-time deny: {errs:?}"
    );
    // The reason word the static diagnostic prints IS a DenyEvent reason
    // (the №392 dictionary is exhaustive over the gate's vocabulary).
    assert!(
        metalogos::deny::is_known_reason("SECRET_LEAK"),
        "SECRET_LEAK must be a known deny reason"
    );
    assert!(metalogos::deny::is_valid_class("file"), "file is a sink class");
}

// ── 2. GREEN (consent): the e2e runs on BOTH backends ─────────────────

#[test]
fn n397_consent_green_path_materializes_on_both_backends() {
    let src = r#"
origin cam { kind: camera, media: image, label: private }
pattern P(_tick: String) -> String {
  let frame = from cam media_store_image("frame-bytes", "private")
  let ok = consent_grant(frame, "household", "owner-1")
  return media_save(ok, "target/n397_consent_frame.jpg")
}
flow Main { input: String = "tick" -> P -> output }
"#;
    let _ = fs::remove_file("target/n397_consent_frame.jpg");
    let base = Path::new(MANIFEST);
    let out_tw = run_tw(src, base).expect("the consent e2e runs (TW)");
    let out_vm = run_vm(src, base).expect("the consent e2e runs (VM)");
    let expected = "target/n397_consent_frame.jpg";
    let text_tw = out_tw.unwrap_or_default();
    assert!(
        text_tw.contains(expected),
        "TW output names the saved frame: {text_tw:?}"
    );
    let text_vm = out_vm.unwrap_or_default();
    assert!(
        text_vm.contains(expected),
        "VM output names the saved frame: {text_vm:?}"
    );
    // The frame materialized and carries the C2PA-style sidecar manifest
    // built from the entry facts (origin chain №332 ↔ manifest fields).
    let manifest_path = "target/n397_consent_frame.jpg.manifest.json";
    assert!(Path::new("target/n397_consent_frame.jpg").exists(), "the frame is on disk");
    let manifest = fs::read_to_string(manifest_path).expect("the sidecar manifest is written");
    let compact: String = manifest.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        compact.contains("\"origin\":\"cam\""),
        "the manifest carries the origin: {manifest}"
    );
    assert!(
        compact.contains("\"conf\":\"private\""),
        "the manifest carries the declared conf: {manifest}"
    );
    let _ = fs::remove_file("target/n397_consent_frame.jpg");
    let _ = fs::remove_file(manifest_path);
}

// ── 3. The unconsented frame: the explainable deny holds ──────────────

#[test]
fn n397_unconsented_private_frame_is_refused() {
    let src = r#"
origin cam { kind: camera, media: image, label: private }
pattern P(_tick: String) -> String {
  let frame = from cam media_store_image("frame-bytes", "private")
  return media_save(frame, "target/n397_leak.jpg")
}
flow Main { input: String = "tick" -> P -> output }
"#;
    let errs = audit_errors(src);
    assert!(
        errs.iter().any(|m| m.starts_with("[SECRET_LEAK]")),
        "no credential, no egress: {errs:?}"
    );
    // The runtime twin stays fail-closed even if the static layer were
    // bypassed (the n387 seal contract, message updated by №397).
    let sealed = run_tw(src, Path::new(MANIFEST));
    // Compile-time refusal: the static gate IS the first runtime gate.
    assert!(sealed.is_err(), "the unconsented save never runs");
}

// ── 4. Revoke: the runtime credential is withdrawn ────────────────────

#[test]
fn n397_revoke_withdraws_the_runtime_credential() {
    // (a) Store-level: the scope lands on the entry and clears on revoke.
    let mut store = metalogos::media::MediaStore::new();
    let label = metalogos::labels::Label {
        conf: metalogos::labels::Conf::Private,
        integrity: metalogos::labels::Integrity::Trusted,
        consent: metalogos::labels::ConsentScope::new(),
    };
    let h = store
        .insert(metalogos::media::MediaKind::Image, vec![1, 2, 3], label)
        .expect("insert");
    assert!(
        store.entry(h).expect("entry").label.consent.is_empty(),
        "a fresh entry carries no consent"
    );
    store.extend_entry_consent(h, "household").expect("grant");
    assert!(
        !store.entry(h).expect("entry").label.consent.is_empty(),
        "the granted scope is ON THE ENTRY"
    );
    store.clear_entry_consent(h).expect("revoke");
    assert!(
        store.entry(h).expect("entry").label.consent.is_empty(),
        "the revoke withdraws the runtime credential"
    );
    // (b) Language-level: consent then revoke — the static cascade
    // poisons and every sink refuses (the compile-time twin).
    let src = r#"
origin cam { kind: camera, media: image, label: private }
pattern P(_tick: String) -> String {
  let frame = from cam media_store_image("frame-bytes", "private")
  let ok = consent_grant(frame, "household", "owner-1")
  let revoked = consent_revoke(ok)
  return media_save(revoked, "target/n397_after_revoke.jpg")
}
flow Main { input: String = "tick" -> P -> output }
"#;
    let errs = audit_errors(src);
    assert!(
        errs.iter().any(|m| m.starts_with("[SINK_CLEARANCE]")),
        "the revoked egress refuses (quarantine): {errs:?}"
    );
}

// ── 5. THE acceptance: grant metering + deny + the signed ledger ──────

#[test]
fn n397_kitchen_camera_e2e_signed_ledger_trail() {
    let src = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/w3_kitchen_camera_e2e.mlog"
    ))
    .expect("the composed example reads");
    let expected =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/w3_kitchen_camera_e2e.expected"))
            .expect("the expected output reads");

    // Ledger snapshots BEFORE (process-global: monotone deltas only).
    let before_grant_issued = count_kind("grant.issued");
    let before_grant_used = count_kind("grant.used");
    let before_irreversible = count_kind("irreversible.db_execute");
    let before_deny = count_kind("deny.IRREVERSIBLE_NO_GRANT");
    let before_consent =
        metalogos::consent::entry_count().unwrap_or(0);

    // TW: the composed story runs and its output is the golden contract.
    let out_tw = run_tw(&src, Path::new(MANIFEST)).expect("the composed e2e runs (TW)");
    assert_eq!(
        out_tw.unwrap_or_default().trim(),
        expected.trim(),
        "the composed story matches its golden"
    );

    // VM: backend parity for the SAME program (the VM's own store +
    // consent interception).
    let out_vm = run_vm(&src, Path::new(MANIFEST)).expect("the composed e2e runs (VM)");
    assert_eq!(
        out_vm.unwrap_or_default().trim(),
        expected.trim(),
        "the VM tells the same story"
    );

    // The signed trail: every event of the story landed in the ledger as
    // a side effect of the action path itself (ADR-0167 §3.4).
    assert!(ledger_kind_delta("grant.issued", before_grant_issued) >= 1, "grant.issued recorded");
    assert!(ledger_kind_delta("grant.used", before_grant_used) >= 1, "grant.used recorded");
    assert!(
        ledger_kind_delta("irreversible.db_execute", before_irreversible) >= 1,
        "the granted DELETE journaled itself"
    );
    assert!(
        ledger_kind_delta("deny.IRREVERSIBLE_NO_GRANT", before_deny) >= 1,
        "the runtime deny is a ledger record"
    );
    assert!(
        metalogos::consent::entry_count().unwrap_or(0) > before_consent,
        "the consent grant is in the consent ledger"
    );

    // The chain the example exported verifies IN-PROCESS...
    let export = "target/w3_kitchen_camera_e2e_export.jsonl";
    let report =
        metalogos::ledger::verify_file(Path::new(export), None, None).expect("the export verifies");
    assert!(report.records >= 4, "the chain carries the story: {}", report.records);

    // ...and EXTERNALLY: `mlog ledger verify` (a separate process, no
    // runtime state) accepts the same file — the dispatch acceptance
    // criterion that was structurally impossible before the seq-0 fix.
    let out = Command::new(env!("CARGO_BIN_EXE_mlog"))
        .args(["ledger", "verify", export])
        .output()
        .expect("the external verifier runs");
    assert!(
        out.status.success(),
        "external verification passes: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("VALID"), "loud verdict: {stdout}");
}

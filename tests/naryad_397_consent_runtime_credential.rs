// ── Naryad №397 (wave-3 acceptance №1 delta): the consent RUNTIME ─────
//    credential — the static and runtime gates AGREE.
//
// What the kitchen-camera e2e integration exposed (dispatch gh#491):
// the static audit accepted a consent scope as the media_save credential
// (№387 generalized egress: non-empty consent scope → no private-egress
// violation — `n387_consent_path_green_for_media_save`), while the
// runtime backstop knew only the №387 likeness token. A consent-clean
// compile was therefore SEALED at runtime (MEDIA_SEALED_EGRESS) — a
// statically-legal program that always dies at runtime. The layers
// violated the №328 agreement principle; the green consent path of the
// wave-3 acceptance (origin → consent → egress) could not run.
//
// Fix: the granted scope lands ON THE STORE ENTRY
// (`MediaStore::extend_entry_consent` — the ConsentScope meet, the same
// operation the static №335 rule applies to the inferred label) through
// store-aware `consent_grant`/`consent_revoke` interception (TW
// statement+expression paths + the VM's `call_media_builtin`; the
// consent-ledger record and the pass-through are unchanged), and the
// media_save backstop honors a consented entry. `consent_revoke`
// withdraws the runtime credential (the runtime twin of the flat
// cascade); quarantine materializes through NO sink (the runtime twin
// of ADR-0154 §2.1).
//
// The №387 likeness token remains a fully independent credential — the
// two credentials now agree with the static gate on BOTH paths.

use std::fs;
use std::path::Path;

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

// ── 1. THE green path: consent → egress runs on BOTH backends ─────────

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
    assert!(
        Path::new("target/n397_consent_frame.jpg").exists(),
        "the frame is on disk"
    );
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

// ── 2. The unconsented frame: the fail-closed refusal holds ───────────

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
    // The refusal is compile-time: the static gate IS the first gate.
    assert!(
        run_tw(src, Path::new(MANIFEST)).is_err(),
        "the unconsented save never runs"
    );
}

// ── 3. Revoke: the runtime credential is withdrawn ────────────────────

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
    // Scopes accumulate (the ConsentScope meet — capabilities add up).
    store.extend_entry_consent(h, "gdpr").expect("second grant");
    assert!(
        !store.entry(h).expect("entry").label.consent.is_empty(),
        "the accumulated scopes stay on the entry"
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

// ── 4. The canonical deny shares the DenyEvent dictionary ─────────────

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
    assert!(metalogos::deny::is_known_reason("SECRET_LEAK"));
    assert!(
        metalogos::deny::is_valid_class("file"),
        "file is a sink class"
    );
}

// ── Wave-3 acceptance item 1 (dispatch №397, gh#491): kitchen camera ──
//
// The kitchen camera end-to-end under the wave-3 capability layer, ONE
// story showing every side the dispatch requires:
//   * origin→capture of a private media frame (№331 origin chain);
//   * the GREEN path — the likeness ritual (№387) issues the one-time
//     credential, the consent ledger records the grant, media_save is
//     ALLOWED;
//   * the irreversible action under a linear grant (№390/№391);
//   * the DENY path — the exhausted quota refuses, on_deny(db) receives
//     the typed DenyEvent (№392), the call degrades, execution continues.
//
// The signed trail is then verified EXTERNALLY: the exported JSONL chain
// is re-verified with the pure `ledger::verify_file` (the same check the
// `mlog ledger verify` CLI performs — hash chain + Ed25519, no runtime
// state), and the consent export must carry the ritual's grant record.
//
// One serial test on purpose: the program writes fixed export paths and
// the ledger is process-global — the per-backend parallel-dir lesson
// (naryad_332) generalized: the two backends run in sequence inside one
// test so the exports are read only after the run that wrote them.

use std::path::Path;

const MANIFEST: &str = env!("CARGO_MANIFEST_DIR");
const EXAMPLE: &str = "examples/w2_kitchen_camera_capability.mlog";
const EXPECTED: &str =
    "saved:String|deleted:1|refused:Unit|snapshot:String|consent:String|exported:String";
const EXPORT: &str = "target/w2_kitchen_camera_export.jsonl";
const CONSENT: &str = "target/w2_kitchen_camera_consent.json";

fn example_source() -> String {
    std::fs::read_to_string(Path::new(MANIFEST).join(EXAMPLE))
        .expect("kitchen camera capability example present")
}

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

#[test]
fn kitchen_camera_capability_e2e_both_backends_and_signed_trail() {
    let src = example_source();

    // 1) TW: the whole story runs — the deny degrades, the rest proceeds.
    let out_tw = run_tw(&src).expect("kitchen camera capability runs on TW");
    assert_eq!(
        out_tw.as_deref().unwrap_or_default().trim_end(),
        EXPECTED,
        "TW flow output contract"
    );

    // 2) VM parity: the same story, the same observable contract.
    let out_vm = run_vm(&src).expect("kitchen camera capability runs on VM");
    assert_eq!(
        out_vm.as_deref().unwrap_or_default().trim_end(),
        EXPECTED,
        "TW/VM parity on the kitchen camera capability story"
    );

    // 3) EXTERNAL verification of the signed action-ledger chain: pure
    //    hash + Ed25519 over the re-parsed JSONL — no runtime state.
    let export_path = Path::new(MANIFEST).join(EXPORT);
    let report = metalogos::ledger::verify_file(&export_path, None, None)
        .expect("exported kitchen camera chain verifies externally");
    assert!(report.records >= 2, "chain carries the story's events");

    // 4) The chain contains the story's specific signed records: the
    //    grant lifecycle, the granted irreversible delete and the DENY
    //    event with the typed reason (the №392 refusal left a signed
    //    ledger record, not just a handler print).
    let content = std::fs::read_to_string(&export_path).expect("export read");
    assert!(
        content.contains("grant.issued"),
        "grant lifecycle journaled"
    );
    assert!(
        content.contains("irreversible.db_execute"),
        "granted delete journaled"
    );
    assert!(
        content.contains("deny.IRREVERSIBLE_NO_GRANT"),
        "deny event journaled with the typed reason"
    );

    // 5) The consent ledger traces the likeness ritual (the GREEN path's
    //    consent grant for the archive scope).
    let consent = std::fs::read_to_string(Path::new(MANIFEST).join(CONSENT))
        .expect("consent ledger export written");
    assert!(
        consent.contains("kitchen-archive") || consent.contains("resident"),
        "consent export carries the ritual's grant record, got: {}",
        consent
    );
}

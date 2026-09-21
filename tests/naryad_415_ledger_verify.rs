// ── Naryad #415 (P1, security/ledger): the structural runtime verify hook ─
//
// Red/green corpus:
//   (1) happy path — a valid signed chain verifies structurally (Jsonl AND
//       File sources) with the full report (records, head, keys, anchored);
//   (2) a bit-flip in a record BODY fails at THAT record (hash mismatch);
//   (3) a deleted record fails at the seq gap (chain break);
//   (4) a flipped signature byte fails the signature check at that record
//       (the M-SIG mutation killer — the sig is outside the hashed body);
//   (5) a RE-LINK attack (prev_hash swapped, hash recomputed, re-signed
//       with the correct key) is caught ONLY by the prev-hash linkage —
//       the M-LINK mutation killer;
//   (6) a rotation transition verifies (two keys, one seam);
//   (7) the archive junction verifies (anchored start);
//   (8) the EMPTY ledger: vacuously ok without anchors (the documented
//       debatable case), loud with the head anchor pinned;
//   (9) the wholesale-deletion attack (the last record dropped) is caught
//       by the out-of-band head anchor (ADR-0167 §7);
//  (10) the builtin `ledger_verify(path)` on BOTH backends: sandboxed read
//       → the verdict struct (VALID for a valid export, INVALID for a
//       tampered one, INVALID for a missing file);
//  (11) the JSON verdict shape (the `mlog ledger verify --json` contract).
//
// The crypto checks are the library's `verify_records_structural` — this
// corpus pins the STRUCTURAL verdict contract (position + reason fields,
// no string guessing) and the read-only surfaces around it.

use ed25519_dalek::Signer;
use metalogos::ledger::{
    archive_file, args_hash_of, build_chain, key_id_of, ledger_verify, record_hash,
    records_from_jsonl, records_to_jsonl, LedgerRecord, LedgerVerifySource, GENESIS_PREV_HASH,
};
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

/// A locally controlled chain builder (build_chain's key is unobservable —
/// the re-link attack in (5) must re-sign with the CORRECT key, so the
/// corpus builds its own chains with a key it holds).
struct Step<'a> {
    signer: &'a ed25519_dalek::SigningKey,
    kind: &'a str,
    action: &'a str,
    new_pub: Option<&'a str>,
}

fn signed_chain(steps: &[Step<'_>]) -> Vec<LedgerRecord> {
    let mut records = Vec::new();
    let mut prev = GENESIS_PREV_HASH.to_string();
    for (seq, s) in steps.iter().enumerate() {
        let seq = seq as u64;
        let pubkey = hex::encode(s.signer.verifying_key().as_bytes());
        let key_id = key_id_of(&pubkey).unwrap_or_default();
        let mut r = LedgerRecord {
            seq,
            ts: 1_700_000_000 + seq,
            kind: s.kind.to_string(),
            actor: "n415".to_string(),
            action: s.action.to_string(),
            scope: "s".to_string(),
            args_hash: args_hash_of(s.action),
            prev_hash: prev.clone(),
            new_pubkey: s.new_pub.unwrap_or("").to_string(),
            key_id,
            pubkey,
            hash: String::new(),
            sig: String::new(),
        };
        let hash = record_hash(&r);
        r.sig = hex::encode(s.signer.sign(hash.as_bytes()).to_bytes());
        r.hash = hash;
        prev = r.hash.clone();
        records.push(r);
    }
    records
}

fn three_step_key() -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(&{
        let a: [u8; 32] = rand::random();
        a
    })
}

// ── (1) Happy path: Jsonl and File sources ─────────────────────────────

#[test]
fn n415_ledger_verify_happy_path_jsonl_and_file() {
    let records = build_chain(&[
        ("grant.issued", "office", "db"),
        ("grant.used", "office", "db"),
        ("session.create", "office", ""),
    ]);
    let content = records_to_jsonl(&records);
    let v = ledger_verify(LedgerVerifySource::Jsonl(&content), None, None);
    assert!(v.ok, "valid chain must verify: {:?}", v.fault);
    assert_eq!(v.records, 3);
    assert_eq!(v.head_hash, records[2].hash);
    assert_eq!(v.distinct_keys, 1);
    assert!(!v.anchored_start);
    assert!(v.fault.is_none());

    // The File source — the same chain written to disk (read-only hook).
    let dir = std::env::temp_dir().join(format!("n415-happy-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmp dir");
    let path = dir.join("chain.jsonl");
    std::fs::write(&path, content.as_bytes()).expect("write chain");
    let vf = ledger_verify(LedgerVerifySource::File(&path), None, None);
    assert!(vf.ok, "file source verifies: {:?}", vf.fault);
    assert_eq!(vf, v, "File and Jsonl sources agree");
    let _ = std::fs::remove_dir_all(&dir);
}

// ── (2) Bit-flip in a body → hash mismatch AT that record ──────────────

#[test]
fn n415_ledger_verify_bit_flip_fails_at_flipped_record() {
    let mut records = build_chain(&[
        ("a.one", "n", "s"),
        ("b.two", "n", "s"),
        ("c.three", "n", "s"),
    ]);
    // Tamper with record 2's body (0-based index 1) WITHOUT touching hash.
    records[1].actor = "mallory".to_string();
    let v = ledger_verify(
        LedgerVerifySource::Jsonl(&records_to_jsonl(&records)),
        None,
        None,
    );
    assert!(!v.ok, "a modified body must fail");
    let fault = v.fault.as_ref().expect("structural fault present");
    assert_eq!(fault.record, Some(2), "the fault points AT record 2");
    assert!(
        fault.reason.contains("hash mismatch"),
        "reason: {}",
        fault.reason
    );
}

// ── (3) Deleted record → seq gap (chain break) ─────────────────────────

#[test]
fn n415_ledger_verify_deleted_record_fails_at_gap() {
    let records = build_chain(&[
        ("a.one", "n", "s"),
        ("b.two", "n", "s"),
        ("c.three", "n", "s"),
    ]);
    let mut broken = records.clone();
    broken.remove(1); // drop the middle record
    let v = ledger_verify(
        LedgerVerifySource::Jsonl(&records_to_jsonl(&broken)),
        None,
        None,
    );
    assert!(!v.ok, "a chain break must fail");
    let fault = v.fault.as_ref().expect("structural fault present");
    assert_eq!(fault.record, Some(2), "the gap surfaces at position 2");
    assert!(fault.reason.contains("seq gap"), "reason: {}", fault.reason);
}

// ── (4) Flipped signature byte → signature FAILED (M-SIG killer) ───────

#[test]
fn n415_ledger_verify_flipped_signature_fails_at_record() {
    let signing = three_step_key();
    let mut records = signed_chain(&[
        Step {
            signer: &signing,
            kind: "action",
            action: "a.one",
            new_pub: None,
        },
        Step {
            signer: &signing,
            kind: "action",
            action: "b.two",
            new_pub: None,
        },
        Step {
            signer: &signing,
            kind: "action",
            action: "c.three",
            new_pub: None,
        },
    ]);
    // Flip one hex char of record 3's signature (still 64 hex chars — the
    // sig bytes change; the hashed body does not cover the signature).
    let mut sig = records[2].sig.clone();
    let first = sig.chars().next().unwrap();
    let flipped = if first == 'a' { 'b' } else { 'a' };
    sig.replace_range(..1, &flipped.to_string());
    records[2].sig = sig;
    let v = ledger_verify(
        LedgerVerifySource::Jsonl(&records_to_jsonl(&records)),
        None,
        None,
    );
    assert!(!v.ok, "a forged signature must fail");
    let fault = v.fault.as_ref().expect("structural fault present");
    assert_eq!(fault.record, Some(3));
    assert!(
        fault.reason.contains("signature verification FAILED"),
        "reason: {}",
        fault.reason
    );
}

// ── (5) Re-link attack → caught ONLY by the linkage (M-LINK killer) ────

#[test]
fn n415_ledger_verify_relinked_prev_hash_fails_at_record() {
    let signing = three_step_key();
    let mut records = signed_chain(&[
        Step {
            signer: &signing,
            kind: "action",
            action: "a.one",
            new_pub: None,
        },
        Step {
            signer: &signing,
            kind: "action",
            action: "b.two",
            new_pub: None,
        },
        Step {
            signer: &signing,
            kind: "action",
            action: "c.three",
            new_pub: None,
        },
    ]);
    // Re-link record 2 to a FAKE parent, then recompute the hash and
    // re-sign with the correct key: seq ok, body-hash ok, key ok,
    // signature ok — ONLY the prev-hash linkage sees the forgery.
    records[1].prev_hash = GENESIS_PREV_HASH.to_string();
    let hash = record_hash(&records[1]);
    records[1].sig = hex::encode(signing.sign(hash.as_bytes()).to_bytes());
    records[1].hash = hash;
    let v = ledger_verify(
        LedgerVerifySource::Jsonl(&records_to_jsonl(&records)),
        None,
        None,
    );
    assert!(!v.ok, "a re-linked chain must fail");
    let fault = v.fault.as_ref().expect("structural fault present");
    assert_eq!(fault.record, Some(2));
    assert!(
        fault.reason.contains("prev_hash chain break"),
        "reason: {}",
        fault.reason
    );
}

// ── (6) Rotation transition verifies ───────────────────────────────────

#[test]
fn n415_ledger_verify_rotation_transition_ok() {
    let old_key = three_step_key();
    let new_key = three_step_key();
    let new_pub = hex::encode(new_key.verifying_key().as_bytes());
    let records = signed_chain(&[
        Step {
            signer: &old_key,
            kind: "action",
            action: "grant.issued",
            new_pub: None,
        },
        // The rotation record is signed by the STILL-ACTIVE (old) key.
        Step {
            signer: &old_key,
            kind: "key_rotation",
            action: "ledger.rotate",
            new_pub: Some(&new_pub),
        },
        Step {
            signer: &new_key,
            kind: "action",
            action: "grant.used",
            new_pub: None,
        },
        Step {
            signer: &new_key,
            kind: "action",
            action: "grant.revoked",
            new_pub: None,
        },
    ]);
    let v = ledger_verify(
        LedgerVerifySource::Jsonl(&records_to_jsonl(&records)),
        None,
        None,
    );
    assert!(v.ok, "rotation chain verifies: {:?}", v.fault);
    assert_eq!(v.distinct_keys, 2, "two keys across the seam");

    // A record after the rotation SIGNED by the OLD key = continuity break:
    // the declared key (old) differs from the active key (new) while the
    // signature itself is valid.
    let old_pub = hex::encode(old_key.verifying_key().as_bytes());
    let old_id = key_id_of(&old_pub).unwrap_or_default();
    let mut stale = records.clone();
    let last = stale.last_mut().unwrap();
    last.action = "session.create".to_string();
    last.args_hash = args_hash_of("session.create");
    last.pubkey = old_pub;
    last.key_id = old_id;
    let hash = record_hash(last);
    last.sig = hex::encode(old_key.sign(hash.as_bytes()).to_bytes());
    last.hash = hash;
    let v2 = ledger_verify(
        LedgerVerifySource::Jsonl(&records_to_jsonl(&stale)),
        None,
        None,
    );
    assert!(!v2.ok);
    assert!(
        v2.fault
            .as_ref()
            .unwrap()
            .reason
            .contains("signer continuity break"),
        "reason: {:?}",
        v2.fault
    );
}

// ── (7) Archive junction verifies (anchored start) ─────────────────────

#[test]
fn n415_ledger_verify_archive_junction_ok() {
    let signing = three_step_key();
    let records = signed_chain(&[
        Step {
            signer: &signing,
            kind: "action",
            action: "a.one",
            new_pub: None,
        },
        Step {
            signer: &signing,
            kind: "action",
            action: "b.two",
            new_pub: None,
        },
        Step {
            signer: &signing,
            kind: "snapshot",
            action: "ledger.snapshot",
            new_pub: None,
        },
        Step {
            signer: &signing,
            kind: "action",
            action: "c.three",
            new_pub: None,
        },
    ]);
    let dir = std::env::temp_dir().join(format!("n415-archive-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmp dir");
    let input = dir.join("full.jsonl");
    let output = dir.join("archived.jsonl");
    std::fs::write(&input, records_to_jsonl(&records).as_bytes()).expect("write full");
    // Archive at the snapshot (seq 2): the output starts at the anchor.
    let report = archive_file(&input, &output, 2).expect("archive truncates at the snapshot");
    assert_eq!(report.records, 2);
    // The junction verifies: anchored start, seqs non-zero, linkage holds.
    let v = ledger_verify(LedgerVerifySource::File(&output), None, None);
    assert!(v.ok, "archived slice verifies: {:?}", v.fault);
    assert!(v.anchored_start, "the junction is an anchored start");
    assert_eq!(v.records, 2);
    let _ = std::fs::remove_dir_all(&dir);
}

// ── (8) The EMPTY ledger: vacuous ok, loud under an anchor ─────────────

#[test]
fn n415_ledger_verify_empty_ledger_documented() {
    // The documented debatable case: an empty chain verifies vacuously —
    // nothing in it contradicts. The caveat is the anchor: without the
    // out-of-band head the verifier cannot tell "no actions" from "all
    // records deleted" (see (9)).
    let v = ledger_verify(LedgerVerifySource::Jsonl(""), None, None);
    assert!(v.ok, "empty ledger is vacuously ok (documented)");
    assert_eq!(v.records, 0);
    assert_eq!(v.head_hash, "");
    assert!(v.fault.is_none());

    // With the head anchor pinned, the empty ledger fails loudly.
    let va = ledger_verify(LedgerVerifySource::Jsonl(""), Some("deadbeef"), None);
    assert!(!va.ok, "an anchored empty ledger must fail");
    assert!(va.fault.as_ref().unwrap().record.is_none());
    assert!(
        va.fault.as_ref().unwrap().reason.contains("empty ledger"),
        "reason: {:?}",
        va.fault
    );

    // With the key anchor pinned — the same loud refusal.
    let vk = ledger_verify(LedgerVerifySource::Jsonl(""), None, Some("key"));
    assert!(!vk.ok);
    assert!(
        vk.fault.as_ref().unwrap().reason.contains("empty ledger"),
        "reason: {:?}",
        vk.fault
    );

    // Whitespace-only content parses to zero records — the same case.
    let vw = ledger_verify(LedgerVerifySource::Jsonl("  \n \n"), None, None);
    assert!(vw.ok);
    assert_eq!(vw.records, 0);
}

// ── (9) Wholesale deletion is caught by the head anchor ────────────────

#[test]
fn n415_ledger_verify_wholesale_deletion_caught_by_head_anchor() {
    let signing = three_step_key();
    let records = signed_chain(&[
        Step {
            signer: &signing,
            kind: "action",
            action: "a.one",
            new_pub: None,
        },
        Step {
            signer: &signing,
            kind: "action",
            action: "b.two",
            new_pub: None,
        },
    ]);
    let head = records[1].hash.clone();
    // The attacker drops the tail and keeps a valid prefix: the prefix
    // self-verifies, but the out-of-band head anchor disagrees.
    let truncated = vec![records[0].clone()];
    let ok_plain = ledger_verify(
        LedgerVerifySource::Jsonl(&records_to_jsonl(&truncated)),
        None,
        None,
    );
    assert!(ok_plain.ok, "the truncated prefix is self-consistent");
    let v = ledger_verify(
        LedgerVerifySource::Jsonl(&records_to_jsonl(&truncated)),
        Some(&head),
        None,
    );
    assert!(!v.ok, "the head anchor catches wholesale deletion");
    let fault = v.fault.as_ref().expect("structural fault present");
    assert!(fault.record.is_none(), "the head fault is chain-level");
    assert!(
        fault.reason.contains("head anchor mismatch"),
        "reason: {}",
        fault.reason
    );
    // The full chain against the same anchor passes.
    let vfull = ledger_verify(
        LedgerVerifySource::Jsonl(&records_to_jsonl(&records)),
        Some(&head),
        None,
    );
    assert!(vfull.ok);
}

// ── (10) The builtin on both backends (sandboxed read → verdict) ───────

const VERIFY_SCRIPT: &str = r#"
pattern V(_p: String) -> String {
  let v = ledger_verify("target/n415-ledger/chain.jsonl")
  if v.ok {
    return "VALID:" + v.head_hash
  }
  return "INVALID:" + v.error_reason
}
flow Main { input: String = "x" -> V -> output }
"#;

#[test]
fn n415_builtin_ledger_verify_reads_sandboxed_chain() {
    let signing = three_step_key();
    let records = signed_chain(&[
        Step {
            signer: &signing,
            kind: "action",
            action: "grant.issued",
            new_pub: None,
        },
        Step {
            signer: &signing,
            kind: "action",
            action: "grant.used",
            new_pub: None,
        },
    ]);
    let rel_dir = "target/n415-ledger";
    let abs_dir = Path::new(MANIFEST).join(rel_dir);
    let _ = std::fs::remove_dir_all(&abs_dir);
    std::fs::create_dir_all(&abs_dir).expect("sandbox dir");
    std::fs::write(
        abs_dir.join("chain.jsonl"),
        records_to_jsonl(&records).as_bytes(),
    )
    .expect("export written");

    for (name, run) in [
        (
            "tw",
            run_tw as fn(&str, &Path) -> Result<Option<String>, String>,
        ),
        ("vm", run_vm),
    ] {
        let out = run(VERIFY_SCRIPT, Path::new(MANIFEST))
            .unwrap_or_else(|e| panic!("ledger_verify runs on {}: {}", name, e));
        let out = out.as_deref().unwrap_or_default().trim_end();
        assert!(
            out.starts_with("VALID:"),
            "valid chain → VALID on {}, got: {:?}",
            name,
            out
        );
        assert!(
            out.contains(&records[1].hash),
            "the verdict carries the head hash, got: {:?}",
            out
        );
    }

    // Tamper the exported file: the builtin reports INVALID with the reason.
    let mut tampered = records.clone();
    tampered[0].action = "grant.stolen".to_string();
    std::fs::write(
        abs_dir.join("chain.jsonl"),
        records_to_jsonl(&tampered).as_bytes(),
    )
    .expect("tampered export written");
    let out = run_tw(VERIFY_SCRIPT, Path::new(MANIFEST)).expect("tampered run");
    let out = out.as_deref().unwrap_or_default().trim_end();
    assert!(
        out.starts_with("INVALID:") && out.contains("hash mismatch"),
        "tampered chain → INVALID with the reason, got: {:?}",
        out
    );

    // A missing file is a soft verdict (№254 read contract): INVALID with
    // "cannot read", not a sandbox breach.
    std::fs::remove_file(abs_dir.join("chain.jsonl")).expect("remove export");
    let out = run_vm(VERIFY_SCRIPT, Path::new(MANIFEST)).expect("missing run");
    let out = out.as_deref().unwrap_or_default().trim_end();
    assert!(
        out.starts_with("INVALID:") && out.contains("cannot read"),
        "missing file → soft INVALID, got: {:?}",
        out
    );
}

// ── (11) The JSON verdict shape (the `--json` CLI contract) ────────────

#[test]
fn n415_ledger_verdict_json_shape() {
    let records = build_chain(&[("a.one", "n", "s"), ("b.two", "n", "s")]);
    let content = records_to_jsonl(&records);
    let v = ledger_verify(LedgerVerifySource::Jsonl(&content), None, None);
    let j = serde_json::to_string(&v).expect("verdict serializes");
    for needle in [
        "\"ok\":true",
        "\"records\":2",
        "\"distinct_keys\":1",
        "\"anchored_start\":false",
        "\"fault\":null",
        &format!("\"head_hash\":\"{}\"", records[1].hash),
    ] {
        assert!(j.contains(needle), "JSON verdict misses {}: {}", needle, j);
    }

    let mut tampered = records.clone();
    tampered[1].actor = "mallory".to_string();
    let vt = ledger_verify(
        LedgerVerifySource::Jsonl(&records_to_jsonl(&tampered)),
        None,
        None,
    );
    let jt = serde_json::to_string(&vt).expect("fault verdict serializes");
    assert!(jt.contains("\"ok\":false"), "JSON: {}", jt);
    assert!(
        jt.contains("\"record\":2"),
        "the fault position is JSON: {}",
        jt
    );
    assert!(jt.contains("hash mismatch"), "the reason is JSON: {}", jt);

    // The parsed content matches what the verifier consumed (roundtrip).
    let parsed = records_from_jsonl(&content).expect("roundtrip parse");
    assert_eq!(parsed.len(), 2);
}

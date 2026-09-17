// ── Naryad #393 (P0, security/ledger, ADR-0167): Action Ledger v1 ─────
//
// Contract tests for the signed append-only action journal:
//   - chain build → JSONL export → external verification roundtrip;
//   - the 10k-record golden (release-only, CI step `ledger-golden`):
//     signs + verifies < 10 s;
//   - tamper evidence: single-byte flips (enumerated), deletion,
//     reordering, key substitution, the fresh-key full rewrite (caught
//     ONLY by the external anchor — the ADR §7 honest boundary);
//   - rotation and snapshot/archival;
//   - the in-toto/PROV profile shape (ADR-0157);
//   - integration: grant lifecycle, runtime deny events (TW + VM) and
//     successful irreversible actions land in the journal AUTOMATICALLY
//     (side effects of the action paths, ADR-0167 §3.4).
//
// The runtime ledger is process-global (the consent-ledger template), so
// the integration tests assert COUNT DELTAS and per-action filtered
// views — never absolute counts.

use ed25519_dalek::Signer;
use metalogos::ledger::{
    all_records, append_record, archive_file, args_hash_of, build_chain, canonical_body, count,
    head_hash, key_id_of, record_hash, records_from_jsonl, records_to_intoto, records_to_jsonl,
    verify_file, verify_records, LedgerRecord, GENESIS_PREV_HASH,
};
use std::fs;
use std::time::Instant;

fn tmp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "n393-{}-{}-{}",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    fs::create_dir_all(&dir).expect("create tmp dir");
    dir
}

// ── 1. Roundtrip: build → JSONL → external verification ──────────────

#[test]
fn chain_roundtrip_build_export_verify() {
    let specs: Vec<(&str, &str, &str)> = (0..7)
        .map(|i| {
            (
                match i {
                    0 => "grant.issued",
                    1 => "irreversible.db_execute",
                    2 => "deny.IRREVERSIBLE_NO_GRANT",
                    3 => "session.create",
                    4 => "ledger.rotate",
                    5 => "ledger.snapshot",
                    _ => "grant.revoked",
                },
                "n393-roundtrip",
                "db:delete:t",
            )
        })
        .collect();
    let records = build_chain(&specs);
    assert_eq!(records.len(), 7);
    assert_eq!(records[0].seq, 0);
    assert_eq!(records[0].prev_hash, GENESIS_PREV_HASH);

    let jsonl = records_to_jsonl(&records);
    let reparsed = records_from_jsonl(&jsonl).expect("jsonl reparses");
    assert_eq!(reparsed.len(), 7);

    let report = verify_records(&reparsed, None, None).expect("valid chain verifies");
    assert_eq!(report.records, 7);
    assert_eq!(report.head_hash, records[6].hash);
    assert!(!report.anchored_start);

    // The external anchors accept the correct head/key and reject wrong ones.
    let key_anchor = records[0].pubkey.clone();
    assert!(verify_records(&reparsed, Some(&report.head_hash), Some(&key_anchor)).is_ok());
    assert!(verify_records(&reparsed, Some("deadbeef"), None).is_err());
    assert!(verify_records(&reparsed, None, Some("deadbeef")).is_err());
}

#[test]
fn empty_ledger_file_refuses_and_verify_file_reads_from_disk() {
    let dir = tmp_dir("file");
    let path = dir.join("ledger.jsonl");
    let records = build_chain(&[("grant.issued", "n393-file", "s")]);
    fs::write(&path, records_to_jsonl(&records)).expect("write");
    let report = verify_file(&path, None, None).expect("file verifies");
    assert_eq!(report.records, 1);
    // An empty file has no records — verification refuses (loudly).
    let empty = dir.join("empty.jsonl");
    fs::write(&empty, "").expect("write empty");
    assert!(verify_file(&empty, None, None).is_err());
    let _ = fs::remove_dir_all(dir);
}

// ── 2. The 10k golden (release-only; run by the CI `ledger-golden` step)

#[test]
#[ignore = "release-only golden (10k Ed25519 signs+verifies); run by the CI ledger-golden step: cargo test --release --test naryad_393_ledger -- --ignored"]
fn golden_10k_chain_signs_and_verifies_under_10s() {
    let specs: Vec<(&str, &str, &str)> = (0..10_000)
        .map(|_| ("grant.used", "n393-golden", "db:delete:golden"))
        .collect();
    let start = Instant::now();
    let records = build_chain(&specs);
    let jsonl = records_to_jsonl(&records);
    let reparsed = records_from_jsonl(&jsonl).expect("10k jsonl reparses");
    let report = verify_records(&reparsed, None, None).expect("10k chain verifies");
    let elapsed = start.elapsed();
    assert_eq!(report.records, 10_000);
    assert!(
        elapsed.as_secs_f64() < 10.0,
        "10k chain must sign+verify under 10s, took {:?}",
        elapsed
    );
}

// ── 3. Tamper evidence ────────────────────────────────────────────────

#[test]
fn every_single_byte_flip_in_a_body_field_breaks_verification() {
    // Enumerated single-field mutations over a 6-record chain: every
    // covered field is protected by the hash (ADR-0167 §6).
    let base = build_chain(&[
        ("grant.issued", "n393-tamper", "s1"),
        ("grant.used", "n393-tamper", "s1"),
        ("deny.SINK_CLEARANCE", "runtime", "db"),
        ("irreversible.db_execute", "n393-tamper", "s1"),
        ("grant.revoked", "n393-tamper", "s1"),
        ("session.create", "user-1", "http"),
    ]);
    // One enumerated tamper: the field name and its single mutation.
    type Mutation = (&'static str, Box<dyn Fn(&mut LedgerRecord)>);
    let mutations: Vec<Mutation> = vec![
        ("ts", Box::new(|r: &mut LedgerRecord| r.ts += 1)),
        ("actor", Box::new(|r: &mut LedgerRecord| r.actor.push('X'))),
        (
            "action",
            Box::new(|r: &mut LedgerRecord| r.action.push('X')),
        ),
        ("scope", Box::new(|r: &mut LedgerRecord| r.scope.push('X'))),
        (
            "args_hash",
            Box::new(|r: &mut LedgerRecord| {
                if r.args_hash.starts_with('0') {
                    r.args_hash.replace_range(0..1, "1");
                } else {
                    r.args_hash.replace_range(0..1, "0");
                }
            }),
        ),
        ("seq", Box::new(|r: &mut LedgerRecord| r.seq += 7)),
        (
            "prev_hash",
            Box::new(|r: &mut LedgerRecord| {
                if r.prev_hash.starts_with('0') {
                    r.prev_hash.replace_range(0..1, "1");
                } else {
                    r.prev_hash.replace_range(0..1, "0");
                }
            }),
        ),
        (
            "hash",
            Box::new(|r: &mut LedgerRecord| {
                if r.hash.starts_with('0') {
                    r.hash.replace_range(0..1, "1");
                } else {
                    r.hash.replace_range(0..1, "0");
                }
            }),
        ),
        (
            "sig",
            Box::new(|r: &mut LedgerRecord| {
                if r.sig.starts_with('0') {
                    r.sig.replace_range(0..1, "1");
                } else {
                    r.sig.replace_range(0..1, "0");
                }
            }),
        ),
        (
            "key_id",
            Box::new(|r: &mut LedgerRecord| {
                if r.key_id.starts_with('0') {
                    r.key_id.replace_range(0..1, "1");
                } else {
                    r.key_id.replace_range(0..1, "0");
                }
            }),
        ),
        (
            "pubkey",
            Box::new(|r: &mut LedgerRecord| {
                if r.pubkey.starts_with('0') {
                    r.pubkey.replace_range(0..1, "1");
                } else {
                    r.pubkey.replace_range(0..1, "0");
                }
            }),
        ),
    ];
    for (field, mutate) in &mutations {
        for pos in 0..base.len() {
            let mut tampered = base.clone();
            mutate(&mut tampered[pos]);
            let err = verify_records(&tampered, None, None)
                .err()
                .unwrap_or_else(|| {
                    panic!(
                        "field '{}' at record {} must break verification",
                        field, pos
                    )
                });
            assert!(
                err.contains("record"),
                "field '{}' at record {}: error must be loud and located: {}",
                field,
                pos,
                err
            );
        }
    }
}

#[test]
fn deleting_or_reordering_records_breaks_verification() {
    let base = build_chain(&[
        ("grant.issued", "n393-del", "s"),
        ("grant.used", "n393-del", "s"),
        ("grant.revoked", "n393-del", "s"),
        ("session.create", "u", "http"),
    ]);
    // Delete a middle record → seq gap + chain break.
    let mut deleted = base.clone();
    deleted.remove(1);
    assert!(verify_records(&deleted, None, None).is_err());
    // Delete the genesis record → not a valid start.
    let mut no_genesis = base.clone();
    no_genesis.remove(0);
    assert!(verify_records(&no_genesis, None, None).is_err());
    // Reorder two records → prev_hash chain break.
    let mut reordered = base.clone();
    reordered.swap(1, 2);
    assert!(verify_records(&reordered, None, None).is_err());
    // Truncate the tail (repudiation) — still a *valid* chain prefix, but
    // the head moved: the external anchor catches it.
    let head_full = base[3].hash.clone();
    let truncated = &base[..3];
    assert!(verify_records(truncated, Some(&head_full), None).is_err());
    assert!(verify_records(truncated, None, None).is_ok());
}

#[test]
fn key_substitution_and_fresh_key_rewrite_are_caught() {
    // Substitute the pubkey of one record (key_id recomputed to match) —
    // the signature was made by the ORIGINAL key: verification fails.
    let base = build_chain(&[
        ("grant.issued", "n393-key", "s"),
        ("grant.used", "n393-key", "s"),
        ("grant.revoked", "n393-key", "s"),
    ]);
    let mut swapped = base.clone();
    let fresh: [u8; 32] = rand::random();
    let fresh_key = ed25519_dalek::SigningKey::from_bytes(&fresh);
    swapped[1].pubkey = hex::encode(fresh_key.verifying_key().as_bytes());
    swapped[1].key_id = key_id_of(&swapped[1].pubkey).unwrap_or_default();
    assert!(verify_records(&swapped, None, None).is_err());

    // The honest-boundary case (ADR-0167 §7): a FULL rewrite under a
    // fresh key (every record re-signed) is structurally self-consistent
    // — caught ONLY by the external key anchor.
    let rewriter = ed25519_dalek::SigningKey::from_bytes(&{
        let b: [u8; 32] = rand::random();
        b
    });
    let mut rewritten = base.clone();
    // A rewrite must RE-BUILD the chain too: the new key changes every
    // body, therefore every hash, therefore every prev_hash link.
    let mut prev = GENESIS_PREV_HASH.to_string();
    for r in rewritten.iter_mut() {
        r.prev_hash = prev.clone();
        r.pubkey = hex::encode(rewriter.verifying_key().as_bytes());
        r.key_id = key_id_of(&r.pubkey).unwrap_or_default();
        let body = canonical_body(r);
        let hash = metalogos::ledger::sha256_hex(body.as_bytes());
        let sig = rewriter.sign(hash.as_bytes());
        r.hash = hash;
        r.sig = hex::encode(sig.to_bytes());
        prev = r.hash.clone();
    }
    assert!(
        verify_records(&rewritten, None, None).is_ok(),
        "a full re-sign is structurally consistent — the anchor is what catches it"
    );
    let original_key = base[0].pubkey.clone();
    assert!(verify_records(&rewritten, None, Some(&original_key)).is_err());
    assert!(verify_records(&base, None, Some(&original_key)).is_ok());
}

// ── 4. Rotation and snapshot/archival ─────────────────────────────────

#[test]
fn rotation_record_switches_the_active_key_and_verifies() {
    // A chain where the second half is signed by a DIFFERENT key, with a
    // key_rotation record (signed by the OLD key) as the seam.
    let old_key = ed25519_dalek::SigningKey::from_bytes(&{
        let a: [u8; 32] = rand::random();
        a
    });
    let new_key = ed25519_dalek::SigningKey::from_bytes(&{
        let b: [u8; 32] = rand::random();
        b
    });
    let old_pub = hex::encode(old_key.verifying_key().as_bytes());
    let new_pub = hex::encode(new_key.verifying_key().as_bytes());
    let old_id = key_id_of(&old_pub).unwrap_or_default();

    let mut records = Vec::new();
    let mut prev = GENESIS_PREV_HASH.to_string();
    let mut seq = 0u64;
    let sign_with = |records: &mut Vec<LedgerRecord>,
                     signer: &ed25519_dalek::SigningKey,
                     signer_id: &str,
                     signer_pub: &str,
                     kind: &str,
                     action: &str,
                     new_pubkey: Option<&str>,
                     prev: &mut String,
                     seq: &mut u64| {
        let mut r = LedgerRecord {
            seq: *seq,
            ts: 1_700_000_000 + *seq,
            kind: kind.to_string(),
            actor: "n393-rotation".to_string(),
            action: action.to_string(),
            scope: "s".to_string(),
            args_hash: args_hash_of(action),
            prev_hash: prev.clone(),
            new_pubkey: new_pubkey.unwrap_or("").to_string(),
            key_id: signer_id.to_string(),
            pubkey: signer_pub.to_string(),
            hash: String::new(),
            sig: String::new(),
        };
        let hash = record_hash(&r);
        let sig = signer.sign(hash.as_bytes());
        r.hash = hash;
        r.sig = hex::encode(sig.to_bytes());
        *prev = r.hash.clone();
        *seq += 1;
        records.push(r);
    };
    sign_with(
        &mut records,
        &old_key,
        &old_id,
        &old_pub,
        "action",
        "grant.issued",
        None,
        &mut prev,
        &mut seq,
    );
    // The rotation record is signed by the OLD key (signer continuity).
    sign_with(
        &mut records,
        &old_key,
        &old_id,
        &old_pub,
        "key_rotation",
        "ledger.rotate",
        Some(&new_pub),
        &mut prev,
        &mut seq,
    );
    let new_pub_id = key_id_of(&new_pub).unwrap_or_default();
    sign_with(
        &mut records,
        &new_key,
        &new_pub_id,
        &new_pub,
        "action",
        "grant.used",
        None,
        &mut prev,
        &mut seq,
    );
    sign_with(
        &mut records,
        &new_key,
        &new_pub_id,
        &new_pub,
        "action",
        "grant.revoked",
        None,
        &mut prev,
        &mut seq,
    );

    let report = verify_records(&records, None, None).expect("rotation chain verifies");
    assert_eq!(report.distinct_keys, 2);

    // A record AFTER the rotation signed by the OLD key breaks continuity.
    let mut stale = records.clone();
    let r = stale.last_mut().unwrap();
    r.action = "session.create".to_string();
    r.args_hash = args_hash_of("session.create");
    r.hash = record_hash(r);
    let sig = old_key.sign(r.hash.as_bytes());
    r.sig = hex::encode(sig.to_bytes());
    // hash changed → prev_hash of nothing (it's the last record), but its
    // own body/signature re-signed under the old key = continuity break.
    assert!(verify_records(&stale, None, None).is_err());

    // A key_rotation record without new_pubkey is refused.
    let mut broken = records.clone();
    broken[1].new_pubkey = String::new();
    broken[1].hash = record_hash(&broken[1]);
    assert!(verify_records(&broken, None, None).is_err());
}

#[test]
fn snapshot_anchoring_and_archive_truncation_verify() {
    // 5 records + a snapshot at position 5 + 2 more after it.
    let mut specs: Vec<(&str, &str, &str)> =
        (0..5).map(|_| ("grant.used", "n393-snap", "s")).collect();
    specs.push(("ledger.snapshot", "runtime", ""));
    specs.push(("session.create", "u", "http"));
    specs.push(("session.destroy", "runtime", "http"));
    // Rebuild the chain manually so record 5 is a snapshot record.
    let mut records = Vec::new();
    let signing = ed25519_dalek::SigningKey::from_bytes(&{
        let a: [u8; 32] = rand::random();
        a
    });
    let pubkey = hex::encode(signing.verifying_key().as_bytes());
    let key_id = key_id_of(&pubkey).unwrap_or_default();
    let mut prev = GENESIS_PREV_HASH.to_string();
    for (i, (action, actor, scope)) in specs.iter().enumerate() {
        let mut r = LedgerRecord {
            seq: i as u64,
            ts: 1_700_000_000 + i as u64,
            kind: if action.starts_with("ledger.snapshot") {
                "snapshot".to_string()
            } else {
                "action".to_string()
            },
            actor: actor.to_string(),
            action: action.to_string(),
            scope: scope.to_string(),
            args_hash: args_hash_of(actor),
            prev_hash: prev.clone(),
            new_pubkey: String::new(),
            key_id: key_id.clone(),
            pubkey: pubkey.clone(),
            hash: String::new(),
            sig: String::new(),
        };
        r.hash = record_hash(&r);
        r.sig = hex::encode(signing.sign(r.hash.as_bytes()).to_bytes());
        prev = r.hash.clone();
        records.push(r);
    }
    let full = verify_records(&records, None, None).expect("full chain verifies");
    assert_eq!(full.records, 8);
    assert_eq!(records[5].kind, "snapshot");

    let dir = tmp_dir("archive");
    let src = dir.join("full.jsonl");
    let dst = dir.join("archived.jsonl");
    fs::write(&src, records_to_jsonl(&records)).expect("write full");

    // Archive at the snapshot (seq 5): the output starts at the anchor.
    let report = archive_file(&src, &dst, 5).expect("archive at snapshot");
    assert_eq!(report.records, 3);
    assert!(report.anchored_start);
    let archived = fs::read_to_string(&dst).expect("read archived");
    let reparsed = records_from_jsonl(&archived).expect("archived reparses");
    assert!(verify_records(&reparsed, None, None).is_ok());
    assert_eq!(reparsed[0].seq, 5);

    // Anchoring at a non-snapshot seq refuses.
    assert!(archive_file(&src, &dst, 2).is_err());
    // An anchored start NOT at a snapshot is refused by the verifier.
    let mut bad_anchor = reparsed.clone();
    bad_anchor[0].kind = "action".to_string();
    bad_anchor[0].hash = record_hash(&bad_anchor[0]);
    assert!(verify_records(&bad_anchor, None, None).is_err());
    let _ = fs::remove_dir_all(dir);
}

// ── 5. The in-toto/PROV profile (ADR-0157) ────────────────────────────

#[test]
fn intoto_profile_carries_the_full_contract() {
    let records = build_chain(&[
        ("grant.issued", "n393-intoto", "db:delete:t"),
        ("irreversible.db_execute", "n393-intoto", "db:delete:t"),
    ]);
    let stream = records_to_intoto(&records);
    let lines: Vec<&str> = stream.lines().collect();
    assert_eq!(lines.len(), 2);
    for (i, line) in lines.iter().enumerate() {
        let v: serde_json::Value = serde_json::from_str(line).expect("statement parses");
        assert_eq!(v["_type"], "https://in-toto.io/Statement/v0.1");
        assert_eq!(
            v["predicateType"],
            "https://metalogos.dev/attestations/action-ledger/v1"
        );
        assert_eq!(v["subject"][0]["digest"]["sha256"], records[i].args_hash);
        assert_eq!(v["predicate"]["seq"], records[i].seq);
        assert_eq!(v["predicate"]["action"], records[i].action);
        assert_eq!(v["predicate"]["prevHash"], records[i].prev_hash);
        assert_eq!(v["predicate"]["recordHash"], records[i].hash);
        assert_eq!(v["predicate"]["keyId"], records[i].key_id);
        assert_eq!(v["predicate"]["signature"], records[i].sig);
    }
    // Rotation records carry the taking-over key.
    let mut with_rotation = records.clone();
    with_rotation[1].kind = "key_rotation".to_string();
    with_rotation[1].new_pubkey = "aa".repeat(32);
    with_rotation[1].hash = record_hash(&with_rotation[1]);
    let v: serde_json::Value =
        serde_json::from_str(records_to_intoto(&with_rotation).lines().nth(1).unwrap())
            .expect("statement parses");
    assert_eq!(v["predicate"]["newPubkey"], "aa".repeat(32));
}

// ── 6. Runtime integration: actions land AUTOMATICALLY ───────────────

fn count_filtered(filter: impl Fn(&LedgerRecord) -> bool) -> u64 {
    all_records()
        .expect("ledger readable")
        .iter()
        .filter(|r| filter(r))
        .count() as u64
}

#[test]
fn grant_lifecycle_events_land_in_the_journal_automatically() {
    let marker = format!("n393-grant-{}", std::process::id());
    let filter = |r: &LedgerRecord| r.actor == marker && r.action.starts_with("grant.");
    let before = count_filtered(filter);
    let g = metalogos::grants::issue(
        "db:delete:n393",
        3600,
        &metalogos::grants::GrantClass::N(2),
        &marker,
    )
    .expect("issue");
    assert!(metalogos::grants::grant_use(&g, &marker).is_ok());
    assert!(metalogos::grants::revoke(&g, &marker).is_ok());
    let delta = count_filtered(filter) - before;
    assert_eq!(
        delta, 3,
        "issue + use + revoke must each journal exactly one record (side effect of the grant op)"
    );
}

#[test]
fn deny_events_land_in_the_journal_automatically_tw_and_vm() {
    // An N(1) grant, the second granted call refuses (GRANT_EXHAUSTED) —
    // a runtime deny event, handled by on_deny(db). The ledger record is
    // written BEFORE handler selection (ADR-0167 §3.4).
    let src = r#"
on_deny(db) {
  match deny_reason() {
    "IRREVERSIBLE_NO_GRANT" then { print("deny:handled") }
    else { print("deny:other") }
  }
}
db { url: "sqlite::memory:" }
pattern DenyFlow(_tick: String) -> String {
  db_execute("CREATE TABLE IF NOT EXISTS dn (id INTEGER PRIMARY KEY, user TEXT)")
  db_execute("INSERT INTO dn (user) VALUES ('alice')")
  let g = grant_issue("db:delete:dn", 60, "n", 1)
  let n1 = db_execute_with_grant(g, "DELETE FROM dn WHERE user = 'alice'")
  let n2 = db_execute_with_grant(g, "DELETE FROM dn")
  return "granted:" + n1 + "|refused:" + type_of(n2)
}
flow Main { input: String = "tick" -> DenyFlow -> output }
"#;
    let before = count().expect("count");
    let before_deny = all_records()
        .expect("readable")
        .iter()
        .filter(|r| r.action == "deny.IRREVERSIBLE_NO_GRANT")
        .count() as u64;

    // ── TW backend ──
    let out = metalogos::run_program(src)
        .expect("TW program runs (deny handled)")
        .expect("flow output");
    assert_eq!(out, "granted:1|refused:Unit");
    let deny_delta = all_records()
        .expect("readable")
        .iter()
        .filter(|r| r.action == "deny.IRREVERSIBLE_NO_GRANT")
        .count() as u64
        - before_deny;
    assert_eq!(
        deny_delta, 1,
        "the TW refusal must journal exactly one deny event"
    );

    // ── VM backend (the runtime twin must journal the same way) ──
    let before_deny_vm = all_records()
        .expect("readable")
        .iter()
        .filter(|r| r.action == "deny.IRREVERSIBLE_NO_GRANT")
        .count() as u64;
    let program = metalogos::compile_program(src).expect("compiles");
    let vm_out = metalogos::run_bytecode(program)
        .expect("VM program runs (deny handled)")
        .expect("flow output");
    assert_eq!(vm_out, "granted:1|refused:Unit");
    let deny_delta_vm = all_records()
        .expect("readable")
        .iter()
        .filter(|r| r.action == "deny.IRREVERSIBLE_NO_GRANT")
        .count() as u64
        - before_deny_vm;
    assert_eq!(
        deny_delta_vm, 1,
        "the VM refusal must journal exactly one deny event (runtime-twin parity)"
    );
    let _ = before; // total count is asserted via the filtered views above
}

#[test]
fn successful_irreversible_action_lands_in_the_journal_automatically() {
    let src = r#"
db { url: "sqlite::memory:" }
pattern IrrFlow(_tick: String) -> String {
  db_execute("CREATE TABLE IF NOT EXISTS irr (id INTEGER PRIMARY KEY, user TEXT)")
  db_execute("INSERT INTO irr (user) VALUES ('alice')")
  let g = grant_issue("db:delete:irr", 60, "n", 2)
  let n1 = db_execute_with_grant(g, "DELETE FROM irr WHERE user = 'alice'")
  return "deleted:" + n1
}
flow Main { input: String = "tick" -> IrrFlow -> output }
"#;
    let filter = |r: &LedgerRecord| r.action == "irreversible.db_execute";
    let before = count_filtered(filter);
    let out = metalogos::run_program(src)
        .expect("program runs")
        .expect("flow output");
    assert_eq!(out, "deleted:1");
    let delta = count_filtered(filter) - before;
    assert_eq!(
        delta, 1,
        "the successful destructive SQL must journal exactly one irreversible action"
    );
}

#[test]
fn runtime_record_and_count_and_head_move_together() {
    let before = count().expect("count");
    let head_before = head_hash().expect("head");
    append_record(
        "action",
        "n393-internal",
        "test.internal_probe",
        "s",
        &args_hash_of("probe"),
        None,
    )
    .expect("append");
    assert_eq!(count().expect("count"), before + 1);
    assert_ne!(head_hash().expect("head"), head_before);
    // The head is a valid 64-hex hash and equals the last record's hash.
    let head = head_hash().expect("head");
    assert_eq!(head.len(), 64);
    let records = all_records().expect("readable");
    assert_eq!(records.last().expect("non-empty").hash, head);
}

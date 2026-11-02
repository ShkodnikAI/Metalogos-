#![allow(clippy::disallowed_methods)]

// ── Naryad №873 (wave 77 — W-F, P1, [core][provenance]): the
// ProvenanceManifest carrier in code (ADR-0207 §2/§4) ───────────────────
//
// The integration gates:
//   1. the dedup guard — the THREE provenance domains stay apart:
//      this module (the shared I/O layer, 8C/C3), the vision/C2PA
//      domain (src/vision/provenance.rs + the №337 media_manifest
//      sidecar surface) and the compliance attestation contour
//      (ADR-0206, src/compliance.rs) share a root word, not a
//      contract;
//   2. the registry totality — every PROVENANCE_CARRIERS name is a
//      classified builtin with the pinned role (Source for ingress,
//      Sink for egress — the classification SSOT is the machine
//      source; a forged name reddens here);
//   3. the media-consistency guard — the C2PA contour (№337) hashes
//      the same bytes with the same SHA-256: the store's
//      `bytes_sha256` and the provenance `origin_digest` agree, two
//      records of the same bytes never contradict (ADR-0207 §2.3);
//   4. the e2e wiring trace on BOTH backends — read_file → process →
//      write_file / print / respond leaves an ingress row and egress
//      rows in the process map, the digests pin the exact bytes
//      (TW/VM parity — the №829 lesson);
//   5. the 1:1 migration gate — the happy path adds NO Ledger
//      records (the ledger-golden posture: the record-on-refusal-only
//      form keeps the golden traces byte-identical);
//   6. the serialization determinism — the canonical row digest is
//      byte-identical across two builds of the same crossing (the G5
//      form of №828).

use std::path::Path;

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

/// The lane-runner type — the TW/VM parity harness form (№748/№829).
type LaneRunner = fn(&str) -> Result<Option<String>, String>;

fn runners() -> [(&'static str, LaneRunner); 2] {
    [("tw", run_tw), ("vm", run_vm)]
}

// ── 1. The dedup guard: the three provenance domains stay apart ────────

#[test]
fn the_three_provenance_domains_share_a_root_word_not_a_contract() {
    // The shared I/O layer: the new type name is pinned, and it is
    // NOT the vision/C2PA provenance (a different module path with a
    // different contract) nor the compliance attestation.
    assert_eq!(metalogos::provenance::PROVENANCE_TYPE, "ProvenanceManifest");
    // The vision/C2PA domain keeps its own surface (the sha256 helper
    // the media store uses — the SAME hash vocabulary, a DIFFERENT
    // domain module; this module never re-exports or wraps it).
    let payload = b"the dedup-guard payload";
    let media_hash = metalogos::vision::provenance::sha256_hex(payload);
    let io_hash = metalogos::provenance::origin_digest(payload);
    assert_eq!(
        media_hash, io_hash,
        "one SHA-256 vocabulary across the domains"
    );
    // The name surfaces stay distinct: no carrier name may collide
    // with a media/vision surface (the media domain is consumed,
    // never rewritten — ADR-0207 §5).
    for spec in metalogos::provenance::PROVENANCE_CARRIERS {
        assert!(
            !spec.name.starts_with("media_") && !spec.name.starts_with("vision_"),
            "{}: the shared registry carries no media/vision surface names",
            spec.name
        );
    }
    // The compliance attestation contour is untouched: its type
    // surface stays `Attestation` (the ADR-0206 §5 boundary).
    assert_eq!(metalogos::compliance::ATTESTATION_TYPE, "Attestation");
    assert_ne!(
        metalogos::provenance::PROVENANCE_TYPE,
        metalogos::compliance::ATTESTATION_TYPE
    );
}

// ── 2. The registry totality: the classification SSOT is the machine source

#[test]
fn the_registry_names_are_classified_carriers_with_the_pinned_roles() {
    for spec in metalogos::provenance::PROVENANCE_CARRIERS {
        let entry = metalogos::builtins_classification::BUILTIN_CLASSES
            .iter()
            .find(|e| e.name == spec.name)
            .unwrap_or_else(|| {
                panic!(
                    "{}: the registered carrier is not a classified builtin — \
                     the registry transaction forged a name (ADR-0207 §4)",
                    spec.name
                )
            });
        match spec.direction {
            metalogos::provenance::Direction::Ingress => assert_eq!(
                entry.class.role,
                metalogos::builtins_classification::Role::Source,
                "{}: an ingress carrier must be classified Source",
                spec.name
            ),
            metalogos::provenance::Direction::Egress => assert_eq!(
                entry.class.role,
                metalogos::builtins_classification::Role::Sink,
                "{}: an egress carrier must be classified Sink",
                spec.name
            ),
        }
    }
}

// ── 3. The media-consistency guard (№337 ↔ the shared layer) ───────────

#[test]
fn the_media_manifest_and_the_provenance_record_agree_on_the_same_bytes() {
    use metalogos::media::MediaStore;

    let payload = "the c2pa-consistency payload: одинаковые байты — одинаковый digest";
    let mut store = MediaStore::new();
    let label = metalogos::labels::Label {
        conf: metalogos::labels::Conf::Public,
        integrity: metalogos::labels::Integrity::Trusted,
        consent: metalogos::labels::ConsentScope::new(),
    };
    let handle = store
        .insert(
            metalogos::media::MediaKind::Image,
            payload.as_bytes().to_vec(),
            label,
        )
        .expect("the media insert");
    let entry = store.entry(handle).expect("the media entry");
    // The C2PA contour hashed the plaintext at insert (the №337
    // insert-time fact); the shared layer hashes the same bytes with
    // the same SHA-256 — two record forms, one digest, no
    // contradiction (ADR-0207 §2.3).
    assert_eq!(
        entry.bytes_sha256,
        metalogos::provenance::origin_digest(payload.as_bytes()),
        "the media manifest digest and the provenance origin digest agree"
    );
    // The manifest built OVER those bytes pins the same origin.
    let m =
        metalogos::provenance::ProvenanceManifest::build("read_file", &entry.bytes_sha256, 0, 100)
            .expect("the manifest over the media digest");
    assert!(
        m.verify_bytes(payload.as_bytes()).is_ok(),
        "the record vouches for the media bytes"
    );
}

// ── 4. The e2e wiring trace on both backends ───────────────────────────

#[test]
#[serial_test::serial]
fn the_boundary_trace_leaves_ingress_and_egress_rows_on_both_lanes() {
    metalogos::provenance::store_reset();

    // The in-language paths are sandbox-relative (CWD = the crate root
    // under cargo test); the Rust side addresses the same files
    // through MANIFEST.
    let dir = std::path::Path::new(MANIFEST)
        .join("target")
        .join("n873_e2e");
    std::fs::create_dir_all(&dir).unwrap();
    let in_path = dir.join("in.txt");
    let out_path = dir.join("out.txt");
    let in_content = "the wave-77 ingress payload: строка для провенанса\n";
    std::fs::write(&in_path, in_content).unwrap();
    let _ = std::fs::remove_file(&out_path);
    let in_rel = "target/n873_e2e/in.txt";
    let out_rel = "target/n873_e2e/out.txt";

    let program = format!(
        r#"
pattern T(_t: String) -> String {{
  let c = read_file("{in_rel}")
  let n = c + "!"
  let w = write_file("{out_rel}", n)
  print("n873 egress marker")
  let r = respond("200", "n873 respond body")
  return w
}}
flow Main {{ input: String = "t" -> T -> output }}
"#
    );

    let before = metalogos::provenance::snapshot().len();
    for (lane, runner) in runners() {
        let out = runner(&program).unwrap().unwrap();
        assert!(
            out.contains("ok"),
            "{lane}: the read → write pipeline passes, got: {out}"
        );
    }

    let snap = metalogos::provenance::snapshot();
    let in_digest = metalogos::provenance::origin_digest(in_content.as_bytes());
    let out_bytes = std::fs::read(&out_path).unwrap();
    let out_digest = metalogos::provenance::origin_digest(&out_bytes);

    // the ingress row: read_file, file channel, the exact input bytes
    let ingress: Vec<_> = snap
        .rows()
        .iter()
        .filter(|r| r.carrier == "read_file" && r.origin_digest == in_digest)
        .collect();
    assert!(
        !ingress.is_empty(),
        "the read_file crossing is recorded with the input digest"
    );
    assert_eq!(ingress[0].channel, metalogos::provenance::Channel::File);
    assert_eq!(
        ingress[0].direction,
        metalogos::provenance::Direction::Ingress
    );

    // the egress rows: write_file pins the written bytes; print and
    // respond pin their own payloads
    let egress_file: Vec<_> = snap
        .rows()
        .iter()
        .filter(|r| r.carrier == "write_file" && r.origin_digest == out_digest)
        .collect();
    assert!(
        !egress_file.is_empty(),
        "the write_file crossing is recorded with the output digest"
    );
    assert_eq!(
        egress_file[0].direction,
        metalogos::provenance::Direction::Egress
    );
    let marker_digest = metalogos::provenance::origin_digest(b"n873 egress marker");
    assert!(
        snap.rows()
            .iter()
            .any(|r| r.carrier == "print" && r.origin_digest == marker_digest),
        "the print crossing is recorded"
    );
    let resp_digest = metalogos::provenance::origin_digest(b"n873 respond body");
    assert!(
        snap.rows()
            .iter()
            .any(|r| r.carrier == "respond" && r.origin_digest == resp_digest),
        "the respond crossing is recorded"
    );

    // the use-site rule: the recorded digests pass, a foreign digest
    // refuses («provenance missing»)
    assert!(metalogos::provenance::require(&in_digest).is_ok());
    assert!(metalogos::provenance::verify_origin(&out_digest, &out_bytes).is_ok());
    let foreign = metalogos::provenance::origin_digest(b"never crossed the boundary");
    let err = metalogos::provenance::require(&foreign).unwrap_err();
    assert!(
        err.starts_with("SINK_CLEARANCE"),
        "the frozen class, got: {err}"
    );
    assert!(err.contains("provenance missing"));

    // the append-only map grew, nothing shrank
    assert!(snap.len() >= before + 4, "the crossings appended rows");
    metalogos::provenance::store_reset();
}

// ── 5. The 1:1 migration gate: the happy path adds NO Ledger records ───

#[test]
#[serial_test::serial]
fn the_happy_path_adds_no_ledger_records() {
    metalogos::provenance::store_reset();

    let program = r#"
pattern T(_t: String) -> String {
  let w = write_file("target/n873_ledger_probe.txt", "the 1:1 probe")
  print("n873 ledger probe")
  return w
}
flow Main { input: String = "t" -> T -> output }
"#;
    let _ = run_tw(program).unwrap().unwrap();

    // The 1:1 gate, race-tolerant form (other test binaries may write
    // their own ledger rows in parallel): no record may carry the
    // provenance flavor — the boundary record never journals itself on
    // the happy path (the record-on-refusal-only posture — the golden
    // traces stay byte-identical). The Ledger record fields are the
    // ACTION-ribbon surface (action/scope; the detail rides the hashed
    // args_hash — the №393 form).
    let records = metalogos::ledger::all_records().unwrap_or_default();
    assert!(
        records
            .iter()
            .all(|r| !r.action.contains("provenance") && !r.scope.contains("provenance")),
        "the happy path must not journal provenance rows"
    );
    // the provenance records DID land (the record is not a no-op)
    let marker = metalogos::provenance::origin_digest(b"the 1:1 probe");
    assert!(
        metalogos::provenance::snapshot().has_origin(&marker),
        "the egress record exists without a Ledger row"
    );
    metalogos::provenance::store_reset();
}

// ── 6. The serialization determinism (the G5 double-run form) ──────────

#[test]
fn the_manifest_serialization_is_byte_identical_across_two_runs() {
    let build = || {
        let digest = metalogos::provenance::origin_digest(b"determinism payload");
        metalogos::provenance::ProvenanceManifest::build("http_get", &digest, 41, 1_700_000)
            .unwrap()
    };
    let a = build();
    let b = build();
    assert_eq!(
        a.record_digest, b.record_digest,
        "the double run is byte-identical"
    );
    assert_eq!(
        metalogos::provenance::canonical_row(&a),
        metalogos::provenance::canonical_row(&b)
    );
    // the record digest pins the FULL row (the G5 form of №828)
    assert_eq!(a.record_digest.len(), 64);
    assert!(
        a.record_digest.bytes().all(|c| c.is_ascii_hexdigit()),
        "the lowercase hex form"
    );
}

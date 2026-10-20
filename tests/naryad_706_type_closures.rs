// ── Naryad №706 (gh#1306, Wave 48): the 0.32 fixed-list movement №3 ──
//
// The next package over the ADR-0193 frozen list
// (scripts/ci/gate_032_untyped_sources.txt, 67 ids): EIGHT honest
// closures by the №644 criterion — every signature landed from the
// LIVE handler and verified HERE by a both-backend test:
//
//   recall_top_k        → String   (memory_ops.rs recall_top_k_tw /
//                                  recall_top_k_vm — BOTH backends end
//                                  in Value::String(serde_json::to_
//                                  string(hit array)); the registered
//                                  fallback IS the no-memory lane
//                                  contract — the zero-hit "[]", the
//                                  №702 fail-closed form)
//   video_understand    → String   (video/understand.rs — the success
//                                  path answers the deterministic mock
//                                  golden; the omni_ask №697 posture:
//                                  the real path without weights is a
//                                  LOUD Err, never a wrong type)
//   media_manifest_read → Struct<MediaManifest>{6 provenance fields,
//                                  all untrusted} (media.rs — the sidecar
//                                  fields read verbatim off the user
//                                  document; the xml_parse №N6
//                                  document-derivation form)
//   cron_list           → Struct<CronJob>{15 fields} (cron.rs — the
//                                  make_date_struct "CronJob" single
//                                  arm; prompt/payload are the
//                                  user-authored job content → private
//                                  (the №689 mail form), the scheduling
//                                  metadata reads off the in-tree store
//                                  → internal (the goal_get form);
//                                  last_run/last_window/next_run are the
//                                  OPTIONAL Float|Unit fields — the
//                                  memory_revise precedent)
//   ledger_verify       → Struct<LedgerVerdict>{7 fields, all internal}
//                                  (ledger.rs verdict_to_value — BOTH
//                                  arms, the soft-missing read and the
//                                  file verify, carry the same shape)
//   proof_verify        → Struct<EmbodiedProofReport>{8 fields}
//                                  (embodied.rs — make_struct
//                                  "EmbodiedProofReport", the SINGLE Ok
//                                  arm; proof_id echoes the caller's
//                                  handle → untrusted)
//   device_state        → Struct<EmbodiedDeviceState>{4 fields}
//                                  (embodied.rs — make_struct
//                                  "EmbodiedDeviceState", the SINGLE Ok
//                                  arm; id echoes the caller's handle →
//                                  untrusted; bounds_digest is the
//                                  OPTIONAL String|Unit field)
//   replay_snapshot     → Struct<ReplaySnapshot>{seq/count internal,
//                                  snapshot untrusted} (server.rs —
//                                  make_struct "ReplaySnapshot"; the
//                                  snapshot field serializes the
//                                  caller's argument — the echo form)
//
// The RE-VERIFIED refusals of the PDn core (the movement №3 obligation —
// the live contract state checked on fresh main 474ff7ae7, 2026-10-17;
// the №702 anchors all hold, the tests/naryad_702_type_closures.rs probes
// stay green and carry the proof):
//
//   query          the BACKEND-DIVERGENT contract is ALIVE — TW:
//                  SELECT → the row list, mutations → the affected-row
//                  count AS A STRING (db_ops.rs:133, 202-205); VM: the
//                  rows are built unconditionally (db_ops.rs:699+) — a
//                  mutation yields the EMPTY list. The "ONE contract"
//                  comments at db_ops.rs:223/1126 belong to db_execute
//                  (№474/#722), NOT to query. A unification is a
//                  language-design decision (the owner's gate).
//   query_row      the element type is data-dependent by column type —
//                  Null→Unit, Integer/Real→Float, Text/Blob→String
//                  (db_ops.rs:535-557, read live); the row is ALSO a
//                  planned-convenience stub in the registry.
//   query_scalar   the value type is data-dependent (numeric vs text
//                  columns); the row is a registry stub.
//   request_body   the dynamic-shape Struct<JsonBody> (http.rs — the
//                  fields parsed per request; the №661 field-meta armor
//                  refuses the static labeling).
//   secret         the opaque Value::Secret (crypto.rs — the SECRET_LEAK
//                  Category-A contract; typing String would breach it).
//   memory_open    the opaque Value::Memory handle behind the consent
//                  gate (registry.rs:388, the №539 posture).
//   memory_read    String|Secret by the container label (registry.rs:390
//                  — typing String would false-warn the private lane).
//   stdin          the true stub — no handler anywhere (registry.rs:180).
//   generate_key   re-refused this movement: the REAL 256-bit random key
//                  returns Value::Secret (crypto.rs:206-216) — the same
//                  opaque-Secret ground as secret().
//   backend_select re-refused this movement: the HETEROGENEOUS result —
//                  the success arm Struct "BackendSelected"
//                  {ok,backend,weights_id,mode,attempts} vs the
//                  exhaustion arm the typed degraded struct
//                  {ok,code,message,attempts} (backends.rs:166-249) —
//                  no single spelling exists (the graph_path №669 form).
//   config_load    re-refused this movement: the value SHAPE is parsed
//                  per file (config.rs — json_value_to_mlog_value; the
//                  type_name is the file stem) — the dynamic-shape
//                  ground, the request_body form.
//   The №748 re-verified anchors (gh#1392, wave 55): the deny pair and
//   the events trio are LIVE on both lanes (the ADR-0051/0052
//   intercepts — the №717 "planned wrapper, no handler" anchors were
//   factually STALE) and LEFT the ground — closed through
//   tests/naryad_748_wall_walk.rs (deny_event Struct<DenyEvent>{7
//   labeled} + deny_reason String + event_count/event_sum Float +
//   events_since List<Struct<Event>>); reflex_list closed List<String>
//   through the shared dispatch. The rest of the walk residue stays
//   stubs or opaque/heterogeneous as of main.
//   474ff7ae7 (the per-row anchors live in the PR table; the honest
//   refusals DO NOT close by silence — each was read, not skipped).
//
// The ratchet: the sensitive floor moved UP in the same PR
// (5793 → 6344 bp; scripts/ci/sensitive_surface_baseline.txt —
// 92*10000//145); the frozen list itself is NOT edited (the id closes
// once). goal032 stays report-only (ADR-0193 Proposed — the ≥20 figure
// is the OWNER's verdict, not self-captured).
#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;
#[cfg(feature = "video")]
use std::sync::Mutex;

/// The registries and the mock-mode env are process-global — the
/// env-touching test serializes here (the video test is feature-gated,
/// so the lock is too).
#[cfg(feature = "video")]
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run_tw(source: &str) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source.trim(), repo_root())
}

fn run_vm(source: &str) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source.trim()).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(repo_root());
    let program = comp.compile(declarations)?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

fn assert_both(src: &str, expected: &str) {
    let tw = run_tw(src)
        .unwrap_or_else(|e| panic!("TW run failed: {e}\n{src}"))
        .unwrap_or_default();
    assert_eq!(tw.trim_end(), expected, "TW output mismatch\n{src}");
    let vm = run_vm(src)
        .unwrap_or_else(|e| panic!("VM run failed: {e}\n{src}"))
        .unwrap_or_default();
    assert_eq!(vm.trim_end(), expected, "VM output mismatch\n{src}");
}

// ── 1. recall_top_k → String (both-backend intercept contract) ────────

#[test]
fn n706_recall_top_k_is_string_on_both_backends() {
    // memorize seeds the store; the recall answer concatenates with a
    // String literal — a non-String operand is a type error, so the
    // concat PROVES the String spelling on each backend; type_of pins
    // it loudly. The TW hybrid and the VM token-overlap scorer return
    // DIFFERENT JSON bodies (different scorers, one serialization
    // contract) — the type is asserted exactly, the payload by
    // containment.
    let src = r#"
pattern M(_t: String) -> String {
  let _m = memorize("n706 needle zebra unique", 1.0)
  let r = recall_top_k("needle")
  return type_of(r)
}
flow Main { input: String = "t" -> M -> output }
"#;
    assert_both(src, "String");
    // The payload: the serialized JSON hit array carries the stored
    // value (both backends). TW:
    let src2 = r#"
pattern M2(_t: String) -> String {
  let _m = memorize("n706 haystack pin value", 1.0)
  let r = recall_top_k("haystack")
  return r
}
flow Main { input: String = "t" -> M2 -> output }
"#;
    let tw = run_tw(src2).expect("tw recall runs").unwrap_or_default();
    assert!(
        tw.contains("haystack pin value"),
        "the TW JSON payload carries the stored value: {tw}"
    );
    let vm = run_vm(src2).expect("vm recall runs").unwrap_or_default();
    assert!(
        vm.contains("haystack pin value"),
        "the VM JSON payload carries the stored value: {vm}"
    );
}

// ── 2. video_understand → String (the mock golden, feature-gated) ─────

#[cfg(feature = "video")]
#[test]
fn n706_video_understand_is_string_mock_golden() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // №478: the SSOT mock predicate reads METALOGOS_MOCK_LLM; pin it,
    // run, unpin (the understand.rs in-file test form).
    std::env::set_var("METALOGOS_MOCK_LLM", "1");
    let src = r#"
pattern V(_t: String) -> String {
  let a = video_understand("video:seg706")
  return a + "!"
}
flow Main { input: String = "t" -> V -> output }
"#;
    let expected = "[MOCK: video_understand | qwen2.5-vl-7b-instruct | video:seg706 | ]!";
    let tw = run_tw(src);
    let vm = run_vm(src);
    std::env::remove_var("METALOGOS_MOCK_LLM");
    let tw = tw.expect("tw video_understand runs").unwrap_or_default();
    assert_eq!(tw.trim_end(), expected, "TW mock golden mismatch");
    let vm = vm.expect("vm video_understand runs").unwrap_or_default();
    assert_eq!(vm.trim_end(), expected, "VM mock golden mismatch");
}

// ── 3. media_manifest_read → Struct<MediaManifest> ────────────────────

#[test]
fn n706_media_manifest_read_fixed_provenance_shape() {
    let dir = repo_root().join("target/n706-media");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fresh dir");
    let sidecar = r#"{
  "kind": "image",
  "origin": "n706_origin",
  "conf": "public",
  "synthetic": true,
  "bytes_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "timestamp": "2026-10-17T00:00:00+00:00"
}"#;
    std::fs::write(dir.join("doc.png.manifest.json"), sidecar).expect("fixture");
    let src = r#"
pattern R(_t: String) -> String {
  let m = media_manifest_read("target/n706-media/doc.png.manifest.json")
  return type_of(m.kind) + "|" + type_of(m.origin) + "|" + type_of(m.conf)
    + "|" + type_of(m.synthetic) + "|" + type_of(m.bytes_sha256) + "|"
    + type_of(m.timestamp) + "|" + m.kind + "|" + m.origin + "|" + m.conf
    + "|" + to_string(m.synthetic == true) + "|"
    + to_string(len(m.bytes_sha256) == 64)
}
flow Main { input: String = "t" -> R -> output }
"#;
    assert_both(
        src,
        "String|String|String|Bool|String|String|image|n706_origin|public|true|true",
    );
}

// ── 4. cron_list → Struct<CronJob> (15 fields; optional Float|Unit) ───

#[test]
fn n706_cron_list_fixed_job_shape_with_private_content_fields() {
    // A FRESH job via the language surface: cron_add(expr, prompt, tz,
    // catch_up, payload) appends; jobs[-1] is the fresh row. The fresh
    // job pins the OPTIONAL fields deterministically: last_run /
    // last_window are Unit (never fired), next_run is a computed Float.
    // prompt/payload round-trip the user content (the private-marked
    // fields — the taint bridge seeds from the type-layer marks).
    let src = r#"
pattern C(_t: String) -> String {
  let _r = cron_add("*/7 * * * *", "n706 probe prompt", "Europe/Moscow", "skip", "pl-706")
  let jobs = cron_list()
  let j = jobs[-1]
  return type_of(j.id) + "|" + type_of(j.prompt) + "|" + type_of(j.cron_expr)
    + "|" + type_of(j.enabled) + "|" + type_of(j.run_count) + "|"
    + type_of(j.force_run) + "|" + type_of(j.created_at) + "|"
    + type_of(j.last_run) + "|" + type_of(j.last_run_tz) + "|"
    + type_of(j.last_window) + "|" + type_of(j.tz) + "|"
    + type_of(j.catch_up) + "|" + type_of(j.payload) + "|"
    + type_of(j.next_run) + "|" + type_of(j.next_run_tz) + "|"
    + j.cron_expr + "|" + j.tz + "|" + j.catch_up + "|" + j.payload
}
flow Main { input: String = "t" -> C -> output }
"#;
    assert_both(
        src,
        "String|String|String|Float|Float|Float|Float|Unit|String|Unit|String|String|String|Float|String|*/7 * * * *|Europe/Moscow|skip|pl-706",
    );
}

// ── 5. ledger_verify → Struct<LedgerVerdict> (the soft-missing arm) ───

#[test]
fn n706_ledger_verify_fixed_verdict_shape() {
    // The missing-file arm (ledger.rs) is deterministic: ok:false,
    // records 0, the empty head_hash, anchored_start false, the loud
    // error_reason; error_record stays Unit (no fault position). The
    // file-verify arm flows through the SAME verdict_to_value — the
    // shape is the row contract.
    let src = r#"
pattern L(_t: String) -> String {
  let v = ledger_verify("target/n706-ledger/absent.ledger")
  return type_of(v.ok) + "|" + type_of(v.records) + "|" + type_of(v.head_hash)
    + "|" + type_of(v.distinct_keys) + "|" + type_of(v.anchored_start)
    + "|" + type_of(v.error_record) + "|" + type_of(v.error_reason)
    + "|" + to_string(v.ok == false) + "|" + to_string(v.records == 0.0)
    + "|" + to_string(v.head_hash == "") + "|" + v.error_reason
}
flow Main { input: String = "t" -> L -> output }
"#;
    assert_both(
        src,
        "Bool|Float|String|Float|Bool|Unit|String|true|true|true|cannot read ledger file target/n706-ledger/absent.ledger: no such file",
    );
}

// ── 6. device_state → Struct<EmbodiedDeviceState> ─────────────────────

#[test]
fn n706_device_state_fixed_introspection_shape() {
    // device_open on the sanctioned sim record → device_state: the
    // FIXED 4-field introspection shape; bounds absent → the OPTIONAL
    // bounds_digest answers Unit (the memory_revise optional-field
    // form, verified live).
    let src = r#"
pattern D(_t: String) -> String {
  let d = device_open("embodied-mock-device")
  let s = device_state(d)
  return type_of(s.id) + "|" + type_of(s.backend) + "|"
    + type_of(s.bounds_present) + "|" + type_of(s.bounds_digest) + "|"
    + s.backend + "|" + to_string(s.bounds_present == false)
}
flow Main { input: String = "t" -> D -> output }
"#;
    assert_both(src, "String|String|Bool|Unit|embodied-mock-device|true");
}

// ── 7. proof_verify → Struct<EmbodiedProofReport> ──────────────────────

#[test]
fn n706_proof_verify_fixed_report_shape() {
    // A test-seam record (the tamper-corpus form): the signature does
    // NOT validate against the canonical serialization → the report
    // carries valid:false + the non-empty reasons list — the FIXED
    // 8-field shape, the SINGLE Ok arm, on both backends.
    let record = metalogos::embodied::ProofRecord {
        id: "n706-proof".to_string(),
        world_hash: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
        chunk_id: "n706-chunk".to_string(),
        bounds_id: "n706-bounds".to_string(),
        telemetry_digest: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            .to_string(),
        verdict: metalogos::embodied::Verdict::Pending,
        sim_hash: "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_string(),
        signature: "deadbeef".to_string(),
    };
    metalogos::embodied::insert_proof_record_for_tests(record);
    // The proof surface takes the OPAQUE Proof handle (proof_id_arg
    // refuses a bare String — the handle flows only from proof_seal), so
    // the shape contract is pinned at the fn level the registry row
    // BINDS (builtin_proof_verify → embodied::proof_verify — the same
    // code the TW/VM registry dispatch executes). The test-seam record
    // (the tamper-corpus form) does NOT validate against the canonical
    // serialization → the report carries valid:false + the non-empty
    // reasons list — the FIXED 8-field shape, the SINGLE Ok arm.
    let report =
        metalogos::embodied::proof_verify("proof_verify", "n706-proof").expect("the report arm");
    let metalogos::interpreter::values::Value::Struct { type_name, fields } = &report else {
        panic!("the report is a Struct: {report:?}");
    };
    assert_eq!(type_name, "EmbodiedProofReport");
    let ty = |k: &str| fields.get(k).map(|v| v.type_name()).unwrap_or("MISSING");
    assert_eq!(ty("valid"), "Bool");
    assert_eq!(ty("proof_id"), "String");
    assert_eq!(ty("verdict"), "String");
    assert_eq!(ty("world_hash"), "String");
    assert_eq!(ty("chunk_id"), "String");
    assert_eq!(ty("bounds_id"), "String");
    assert_eq!(ty("backend"), "String");
    assert_eq!(ty("reasons"), "List");
    assert_eq!(fields.len(), 8, "the fixed field set");
    let field = |k: &str| {
        fields
            .get(k)
            .cloned()
            .unwrap_or(metalogos::interpreter::values::Value::Unit)
    };
    use metalogos::interpreter::values::Value;
    let Value::Bool(valid) = field("valid") else {
        panic!("valid is Bool")
    };
    assert!(!valid, "the tampered record fails");
    let Value::String(verdict) = field("verdict") else {
        panic!("verdict is String")
    };
    assert_eq!(verdict, "pending");
    let Value::String(pid) = field("proof_id") else {
        panic!("proof_id is String")
    };
    assert_eq!(pid, "n706-proof");
    let Value::List(reasons) = field("reasons") else {
        panic!("reasons is List")
    };
    assert!(!reasons.is_empty(), "the tamper reasons surface");
}

// ── 8. replay_snapshot → Struct<ReplaySnapshot> ────────────────────────

#[test]
fn n706_replay_snapshot_fixed_shape() {
    // seq is the constant 0.0, count the argument length, snapshot the
    // serialized argument (the echo → untrusted). The shape is pinned
    // by types + value comparisons on both backends.
    let src = r#"
pattern S(_t: String) -> String {
  let s = replay_snapshot(["a", "b"])
  return type_of(s.seq) + "|" + type_of(s.count) + "|" + type_of(s.snapshot)
    + "|" + to_string(s.seq == 0.0) + "|" + to_string(s.count == 2.0) + "|"
    + to_string(len(s.snapshot) >= 2)
}
flow Main { input: String = "t" -> S -> output }
"#;
    assert_both(src, "Float|Float|String|true|true|true");
}

// ── 9. The registry spellings are pinned (no silent Unknown) ──────────

#[test]
fn n706_registry_type_spellings_pinned() {
    let row = |name: &str| {
        metalogos::builtins::BUILTIN_REGISTRY
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("registry row {name} missing"))
    };
    // The single precise closure of this package (the parameterized six
    // live in the field-meta block below):
    {
        let r = row("recall_top_k");
        assert!(
            matches!(r.return_type, metalogos::builtins::Type::String),
            "recall_top_k must carry the precise String spelling"
        );
        assert!(!r.parameterized && r.field_meta.is_empty());
        assert!(
            r.handler.is_some(),
            "recall_top_k must carry the fail-closed fallback"
        );
    }
    // video_understand is feature-gated — assert under the feature:
    #[cfg(feature = "video")]
    {
        let r = row("video_understand");
        assert!(matches!(r.return_type, metalogos::builtins::Type::String));
    }
    // The six parameterized closures — the Struct<Name>{labels} grammar:
    let expected_meta: &[(&str, &str)] = &[
        (
            "media_manifest_read",
            "kind:untrusted,origin:untrusted,conf:untrusted,synthetic:untrusted,bytes_sha256:untrusted,timestamp:untrusted",
        ),
        (
            "cron_list",
            "id:internal,cron_expr:internal,prompt:private,enabled:internal,run_count:internal,force_run:internal,created_at:internal,last_run:internal,last_run_tz:internal,last_window:internal,tz:internal,catch_up:internal,payload:private,next_run:internal,next_run_tz:internal",
        ),
        (
            "ledger_verify",
            "ok:internal,records:internal,head_hash:internal,distinct_keys:internal,anchored_start:internal,error_record:internal,error_reason:internal",
        ),
        (
            "proof_verify",
            "valid:internal,proof_id:untrusted,verdict:internal,world_hash:internal,chunk_id:internal,bounds_id:internal,backend:internal,reasons:internal",
        ),
        (
            "device_state",
            "id:untrusted,backend:internal,bounds_present:internal,bounds_digest:internal",
        ),
        ("replay_snapshot", "seq:internal,count:internal,snapshot:untrusted"),
    ];
    for (name, meta) in expected_meta {
        let r = row(name);
        assert!(r.parameterized, "{name} must be parameterized");
        assert_eq!(r.field_meta, *meta, "{name} field-meta drifted");
        assert!(r.handler.is_some());
    }
    // The refusals stay honestly Unknown (no silent typing):
    for n in [
        // query/query_row/query_scalar left the refusal ground: №740
        // (gh#1378, wave 53) closed them through the №740 UNION
        // vocabulary (lane α) with the live handler fact — the
        // spellings pinned in tests/naryad_740_union_spellings.rs.
        "stdin",
        "request_body",
        "secret",
        "memory_open",
        "memory_read",
        "generate_key",
        "backend_select",
        "config_load",
        // deny_event/deny_reason/event_count/event_sum/events_since left
        // the refusal ground: №748 (gh#1392, wave 55) closed them — the
        // ADR-0051/0052 intercepts are the live both-lane handlers (the
        // №736 intercept-walk lesson) — tests/naryad_748_wall_walk.rs.
    ] {
        let r = row(n);
        assert!(
            matches!(r.return_type, metalogos::builtins::Type::Unknown),
            "{n} must stay Unknown — the honest refusal"
        );
        assert!(!r.parameterized && r.field_meta.is_empty());
    }
}

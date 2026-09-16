// ── Наряд №334 (P0, feature/backends): real STT/omni/vision-understanding
//    backends — the SHA-pin path ─────────────────────────────────────────
//
// Red/green corpus:
//   (1) the three №334-scoped backends (STT / omni / vision-understanding)
//       carry REAL pins — HF LFS oid = sha256, provenance-dated — and
//       their per-file manifests validate (manifest mandatory, silent
//       fallback forbidden);
//   (2) the loader path: dry-run plans (URL → path → pinned sha → bytes),
//       manifest-without-SHA refusals, sha256 verification vectors,
//       byte-count and hash mismatch refusals (no writes);
//   (3) the network refusals: allowlist default-deny (unset / empty /
//       non-allowlisted host) fires BEFORE any network activity;
//   (4) the golden mock contract: stt_transcribe / omni_ask /
//       vision_understand are deterministic in mock mode (default) on
//       BOTH backends, and refuse LOUDLY in real mode (PARKED №294).

use std::path::Path;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

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

const MANIFEST: &str = env!("CARGO_MANIFEST_DIR");

// ── (1) The three scoped backends: real pins + manifest parity ───────

#[test]
fn scoped_backends_carry_real_pins() {
    // The №334 scope: one STT, one omni, one vision-understanding.
    let stt = metalogos::backends::find_by_weights_id("whisper-large-v3-turbo")
        .expect("STT backend in the registry");
    let omni = metalogos::backends::find_by_weights_id("nemotron-3-nano-omni-30b-a3b")
        .expect("omni backend in the registry");
    let vision = metalogos::backends::find_by_weights_id("molmoact2")
        .expect("vision-understanding backend in the registry");
    assert_eq!(stt.class, metalogos::backends::BackendClass::Stt);
    assert_eq!(omni.class, metalogos::backends::BackendClass::Omni);
    assert_eq!(
        vision.class,
        metalogos::backends::BackendClass::VisionUnderstanding
    );
    for e in [stt, omni, vision] {
        match e.pin {
            metalogos::backends::ShaPin::Pinned(h) => {
                assert_eq!(h.len(), 64, "{}: pin is sha256 hex", e.name);
            }
            metalogos::backends::ShaPin::PendingNo334 => {
                panic!("{}: a №334-scoped backend must carry a REAL pin", e.name)
            }
        }
    }
    // License classes (the report contract): whisper MIT → osi;
    // nemotron NVIDIA OML → non-osi; molmoact2 Apache-2.0 → osi.
    assert_eq!(stt.license, metalogos::backends::LicenseClass::Osi);
    assert_eq!(omni.license, metalogos::backends::LicenseClass::NonOsi);
    assert_eq!(vision.license, metalogos::backends::LicenseClass::Osi);
}

#[test]
fn scoped_manifests_validate() {
    for weights_id in [
        "whisper-large-v3-turbo",
        "nemotron-3-nano-omni-30b-a3b",
        "molmoact2",
    ] {
        let source = metalogos::backends::weights_source(weights_id)
            .unwrap_or_else(|| panic!("{}: manifest present", weights_id));
        metalogos::backends::validate_weights_source(source)
            .unwrap_or_else(|e| panic!("{}: manifest must validate: {}", weights_id, e));
        // Every file: 64-hex sha, positive bytes, repo-relative path.
        for f in source.files {
            assert_eq!(f.sha256.len(), 64, "{}: {}", weights_id, f.path);
            assert!(f.bytes > 0, "{}: {}", weights_id, f.path);
        }
    }
}

#[test]
fn registry_pin_matches_a_manifest_file() {
    // The registry pin and the manifest are two views of one truth.
    for weights_id in [
        "whisper-large-v3-turbo",
        "nemotron-3-nano-omni-30b-a3b",
        "molmoact2",
    ] {
        let entry = metalogos::backends::find_by_weights_id(weights_id).unwrap();
        let source = metalogos::backends::weights_source(weights_id).unwrap();
        match entry.pin {
            metalogos::backends::ShaPin::Pinned(h) => assert!(
                source.files.iter().any(|f| f.sha256 == h),
                "{}: pin must match a manifest file",
                weights_id
            ),
            metalogos::backends::ShaPin::PendingNo334 => panic!("scoped entries are pinned"),
        }
    }
}

// ── (2) The loader path: dry-run plan + verification refusals ────────

#[test]
fn dry_run_plan_whisper_is_exact() {
    let plan =
        metalogos::backends_weights::weights_plan("whisper-large-v3-turbo").expect("whisper plan");
    assert_eq!(plan.len(), 1, "whisper is a single-file artifact");
    let f = &plan[0];
    assert_eq!(
        f.url,
        "https://huggingface.co/openai/whisper-large-v3-turbo/resolve/main/model.safetensors"
    );
    assert_eq!(f.path, "model.safetensors");
    assert_eq!(
        f.sha256,
        "542566a422ae4f3fd23f1ba11add198fca01bbf82e66e6a2857b3f608b1eb9d1"
    );
    assert_eq!(f.bytes, 1_617_824_864);
}

#[test]
fn dry_run_plans_cover_every_shard() {
    let nemotron = metalogos::backends_weights::weights_plan("nemotron-3-nano-omni-30b-a3b")
        .expect("nemotron plan");
    assert_eq!(nemotron.len(), 17, "nemotron BF16 ships 17 shards");
    let molmo = metalogos::backends_weights::weights_plan("molmoact2").expect("molmoact2 plan");
    assert_eq!(molmo.len(), 5, "molmoact2 ships 5 shards");
    // All URLs https + host huggingface.co (the plan is the SSRF surface).
    for f in nemotron.iter().chain(molmo.iter()) {
        assert!(f.url.starts_with("https://huggingface.co/"), "{}", f.url);
    }
}

#[test]
fn plan_refuses_unknown_and_unpinned() {
    let err = metalogos::backends_weights::weights_plan("no-such-weights")
        .expect_err("unknown weights_id must be loud");
    assert!(err.contains("no registry record"), "got: {}", err);
    // The remaining PendingNo334 entries (chatterbox/kokoro/z-image-turbo/
    // wall-oss — out of the №334 three-backend scope; hashes for gated or
    // unfilled artifacts are NOT invented) refuse to plan loudly.
    for pending in ["chatterbox-multilingual-v3", "z-image-turbo"] {
        let err = metalogos::backends_weights::weights_plan(pending)
            .expect_err("PendingNo334 must refuse");
        assert!(err.contains("PendingNo334"), "{}: {}", pending, err);
    }
}

#[test]
fn manifest_without_sha_is_refused() {
    // A manifest entry without a valid pin — the loud-refusal contract.
    let bad = metalogos::backends::WeightsSource {
        repo: "example/repo",
        revision: "main",
        files: &[metalogos::backends::WeightsFile {
            path: "model.safetensors",
            sha256: "",
            bytes: 100,
        }],
    };
    let err = metalogos::backends::validate_weights_source(&bad)
        .expect_err("manifest without SHA must be refused");
    assert!(
        err.contains("no valid pinned SHA-256"),
        "expected the loud no-pin refusal, got: {}",
        err
    );
    // A truncated/garbage hash is equally refused.
    let bad2 = metalogos::backends::WeightsSource {
        repo: "example/repo",
        revision: "main",
        files: &[metalogos::backends::WeightsFile {
            path: "model.safetensors",
            sha256: "deadbeef",
            bytes: 100,
        }],
    };
    assert!(metalogos::backends::validate_weights_source(&bad2).is_err());
    // Zero bytes and duplicate paths too.
    let bad3 = metalogos::backends::WeightsSource {
        repo: "example/repo",
        revision: "main",
        files: &[metalogos::backends::WeightsFile {
            path: "model.safetensors",
            sha256: "542566a422ae4f3fd23f1ba11add198fca01bbf82e66e6a2857b3f608b1eb9d1",
            bytes: 0,
        }],
    };
    assert!(metalogos::backends::validate_weights_source(&bad3).is_err());
}

#[test]
fn sha256_verification_vector_and_mismatches() {
    // Known SHA-256 vector.
    assert_eq!(
        metalogos::backends_weights::sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    // Byte-count mismatch refuses.
    let err = metalogos::backends_weights::verify_pinned_bytes(
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        999,
        b"abc",
    )
    .expect_err("byte mismatch must refuse");
    assert!(err.contains("byte-count mismatch"), "got: {}", err);
    // Hash mismatch refuses.
    let err = metalogos::backends_weights::verify_pinned_bytes(
        "0000000000000000000000000000000000000000000000000000000000000000",
        3,
        b"abc",
    )
    .expect_err("hash mismatch must refuse");
    assert!(err.contains("SHA-256 mismatch"), "got: {}", err);
    // The match passes.
    metalogos::backends_weights::verify_pinned_bytes(
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        3,
        b"abc",
    )
    .expect("matching bytes verify");
}

// ── (3) Network refusals fire BEFORE any request ─────────────────────

#[test]
fn fetch_refuses_without_allowlist_env() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::remove_var("MLOG_BACKEND_WEIGHTS_ALLOWLIST");
    let err = metalogos::backends_weights::fetch_weights("whisper-large-v3-turbo", "target/n334-w")
        .expect_err("default-deny must refuse");
    assert!(
        err.contains("MLOG_BACKEND_WEIGHTS_ALLOWLIST is not set"),
        "got: {}",
        err
    );
}

#[test]
fn fetch_refuses_on_non_allowlisted_host_before_network() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var("MLOG_BACKEND_WEIGHTS_ALLOWLIST", "example.invalid");
    let err = metalogos::backends_weights::fetch_weights("whisper-large-v3-turbo", "target/n334-w")
        .expect_err("non-allowlisted host must refuse");
    assert!(
        err.contains("not in MLOG_BACKEND_WEIGHTS_ALLOWLIST"),
        "got: {}",
        err
    );
    // Nothing was written (the refusal is pre-network).
    assert!(
        !Path::new(MANIFEST).join("target/n334-w").exists(),
        "no filesystem side effects on refusal"
    );
    std::env::remove_var("MLOG_BACKEND_WEIGHTS_ALLOWLIST");
}

// ── (4) The golden mock contract (TW + VM) + real-mode loudness ──────

const STT_PROG: &str = r#"
pattern P(_x: String) -> String {
  return stt_transcribe("frame-001.wav")
}
flow Main { input: String = "x" -> P -> output }
"#;

const OMNI_PROG: &str = r#"
pattern P(_x: String) -> String {
  return omni_ask("what is in this frame?", "frame-001.jpg")
}
flow Main { input: String = "x" -> P -> output }
"#;

const VISION_PROG: &str = r#"
pattern P(_x: String) -> String {
  return vision_understand("frame-001.jpg", "describe the scene")
}
flow Main { input: String = "x" -> P -> output }
"#;

#[test]
fn mock_contract_is_deterministic_on_tw_and_vm() {
    // Mock mode is the DEFAULT (unset env) — deterministic outputs. The
    // env lock serializes against the real-mode test (the №376 lesson:
    // env is process-global, parallel tests must not race it).
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::remove_var("METALOGOS_LLM_MOCK");
    let stt = run_tw(STT_PROG, Path::new(MANIFEST)).expect("stt mock runs");
    assert_eq!(
        stt.as_deref().unwrap_or_default().trim_end(),
        "[MOCK: stt_transcribe | whisper-large-v3-turbo | frame-001.wav]"
    );
    let omni = run_tw(OMNI_PROG, Path::new(MANIFEST)).expect("omni mock runs");
    assert_eq!(
        omni.as_deref().unwrap_or_default().trim_end(),
        "[MOCK: omni_ask | nemotron-3-nano-omni-30b-a3b | what is in this frame? | frame-001.jpg]"
    );
    let vis = run_tw(VISION_PROG, Path::new(MANIFEST)).expect("vision mock runs");
    assert_eq!(
        vis.as_deref().unwrap_or_default().trim_end(),
        "[MOCK: vision_understand | molmoact2 | frame-001.jpg | describe the scene]"
    );
    // VM parity — the same strings on the compiled path.
    for (prog, expected) in [
        (
            STT_PROG,
            "[MOCK: stt_transcribe | whisper-large-v3-turbo | frame-001.wav]",
        ),
        (
            OMNI_PROG,
            "[MOCK: omni_ask | nemotron-3-nano-omni-30b-a3b | what is in this frame? | frame-001.jpg]",
        ),
        (
            VISION_PROG,
            "[MOCK: vision_understand | molmoact2 | frame-001.jpg | describe the scene]",
        ),
    ] {
        let out = run_vm(prog, Path::new(MANIFEST)).expect("VM mock parity");
        assert_eq!(out.as_deref().unwrap_or_default().trim_end(), expected);
    }
    std::env::remove_var("METALOGOS_LLM_MOCK");
}

#[test]
fn class_mismatch_and_unknown_model_are_loud() {
    // Env-locked: the class check is mode-independent, but the run must
    // not observe the real-mode flag from the parallel test.
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::remove_var("METALOGOS_LLM_MOCK");
    // Naming nemotron weights trips the №333 license gate FIRST (audit:
    // compile-blocking under the distribution profile) — the loud outer
    // layer. Assert it: the class check must not bypass licensing.
    let gated = r#"
pattern P(_x: String) -> String {
  return stt_transcribe("a.wav", "nemotron-3-nano-omni-30b-a3b")
}
flow Main { input: String = "x" -> P -> output }
"#;
    let lic_err = metalogos::compile_program(gated.trim())
        .expect_err("naming non-osi weights must refuse at audit");
    assert!(
        lic_err.contains("BACKEND_LICENSE_DISTRIBUTION"),
        "got: {}",
        lic_err
    );
    // Under the audited licensing bridge the call surface is reachable —
    // and the RUNTIME class check refuses an stt call with omni weights.
    let bridged = r#"
profile licensing { backends: permissive_with_audit }
pattern P(_x: String) -> String {
  return stt_transcribe("a.wav", "nemotron-3-nano-omni-30b-a3b")
}
flow Main { input: String = "x" -> P -> output }
"#;
    let err = run_tw(bridged, Path::new(MANIFEST)).expect_err("stt over omni weights refuses");
    assert!(err.contains("not stt"), "got: {}", err);
    let src2 = r#"
pattern P(_x: String) -> String {
  return vision_understand("img.jpg", "what?", "no-such-model")
}
flow Main { input: String = "x" -> P -> output }
"#;
    let err2 = run_tw(src2, Path::new(MANIFEST)).expect_err("unknown model refuses");
    assert!(err2.contains("no registry record"), "got: {}", err2);
    std::env::remove_var("METALOGOS_LLM_MOCK");
}

#[test]
fn real_mode_refuses_loudly_without_weights() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var("METALOGOS_LLM_MOCK", "false");
    let src = r#"
pattern P(_x: String) -> String {
  return stt_transcribe("a.wav")
}
flow Main { input: String = "x" -> P -> output }
"#;
    // The refusal is a RUNTIME loudness (the handler checks weights when
    // called) — run both pipelines, not just compile.
    let err = run_tw(src, Path::new(MANIFEST)).expect_err("real mode without weights refuses");
    assert!(
        err.contains("PARKED by hardware") && err.contains("model.safetensors"),
        "expected the loud PARKED refusal naming the artifact, got: {}",
        err
    );
    let err_vm = run_vm(src, Path::new(MANIFEST)).expect_err("real mode refuses on VM too");
    assert!(err_vm.contains("PARKED by hardware"), "got: {}", err_vm);
    std::env::remove_var("METALOGOS_LLM_MOCK");
}

// ── (5) limitations.md carries the PARKED line (loud, not silent) ────

#[test]
fn limitations_names_the_parked_boundary() {
    let doc = std::fs::read_to_string(Path::new(MANIFEST).join("docs/limitations.md"))
        .expect("limitations.md exists");
    assert!(
        doc.contains("№334") && doc.contains("PARKED"),
        "limitations.md must name the №334 real-weights PARKED boundary"
    );
}

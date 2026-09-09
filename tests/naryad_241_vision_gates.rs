//! Наряд №241 (Vision R5, ADR-0125): Category-A gate contract tests.
//!
//! The gate NAMES are the SSOT from ADR-0125 (Accepted 2026-09-07, before
//! R4.1): `VISION_UNSIGNED_EXPORT`, `MODEL_WEIGHTS_UNSAFE`,
//! `VISION_POLICY_MISSING` (+ taint `VISION_PROMPT_USER_INPUT` from
//! №240 — the 4th Category-A contract of plan §7.1; the plan's "4
//! контракта-теста категории A" close as 3 new + 1 from №240, stated
//! loudly in PR №239).
//!
//! NO network in this file. NO weights. Env-touching tests are #[serial]
//! (лекало naryad_130_ssrf_guard.rs). The LSB watermark roundtrip lives
//! in `src/vision/provenance.rs` unit tests behind
//! `#[cfg(all(test, feature = "vision"))]` (needs the `image` codec) and
//! runs in the vision-tests CI job.

use metalogos::audit::{audit_category_a, AuditFinding, Severity};
use metalogos::builtins::{vision_export_dispatch, vision_export_raw_dispatch};
use metalogos::interpreter::Value;
use metalogos::vision::provenance::VisionManifest;
use metalogos::vision::{VisionArtifact, VisionRegistry};
use serial_test::serial;

// ── Helpers ──────────────────────────────────────────────────────────

fn findings_of(source: &str) -> Vec<AuditFinding> {
    metalogos::audit_program(source)
        .expect("audit_program: parse must succeed")
        .findings
}

fn by_id<'a>(findings: &'a [AuditFinding], id: &str) -> Vec<&'a AuditFinding> {
    findings.iter().filter(|f| f.check_id == id).collect()
}

fn signed_manifest() -> VisionManifest {
    VisionManifest {
        model_id: "z-image-turbo".to_string(),
        model_sha256: "deadbeef".to_string(),
        seed: 42,
        prompt_sha256: "cafebabe".to_string(),
        policy: "safe".to_string(),
        timestamp: "2026-09-09T00:00:00+00:00".to_string(),
        png_sha256: "0123456789abcdef".to_string(),
    }
}

// ── Gate 1: VISION_UNSIGNED_EXPORT (audit Error + runtime backstop) ──

/// `vision_export` call site in a file with NO `vision { }` declaration
/// → Category-A compile error (ADR-0125: by construction, not by
/// procedure). `run_program` must refuse BEFORE runtime.
#[test]
fn vision_unsigned_export_is_category_a_compile_error() {
    let source = r#"
pattern Ship(name: String) -> String {
    vision_export("handle", "out.png")
    return name
}
flow Main { input: String = "x" -> Ship -> output }
"#;
    // Static layer: mlog audit reports the Error finding.
    let findings = findings_of(source);
    let hits = by_id(&findings, "VISION_UNSIGNED_EXPORT");
    assert_eq!(hits.len(), 1, "exactly one gate hit, got: {:?}", findings);
    assert_eq!(hits[0].severity, Severity::Error);
    assert!(
        hits[0].message.contains("no `vision { }` declaration"),
        "message must name the missing-declaration cause: {}",
        hits[0].message
    );

    // Compile layer: audit_category_a feeds semantic errors (№98).
    let decls = metalogos::parser::parse(source).expect("parse");
    let cat_a = audit_category_a(&decls, "");
    assert!(
        cat_a
            .iter()
            .any(|f| f.check_id == "VISION_UNSIGNED_EXPORT" && f.severity == Severity::Error),
        "compile path must carry the gate: {:?}",
        cat_a
    );

    // End-to-end: run_program refuses at compile time.
    let err = metalogos::run_program(source).expect_err("gate must refuse the program");
    assert!(
        err.contains("VISION_UNSIGNED_EXPORT"),
        "compile refusal must carry the check-id: {}",
        err
    );
}

/// Positive control: the same call WITH a `vision { }` declaration in the
/// file → no VISION_UNSIGNED_EXPORT finding (signed by construction).
#[test]
fn vision_export_with_declaration_passes_gate() {
    let source = r#"
vision "poster" { model: "z-image-turbo" steps: 8 width: 1024 height: 1024 seed: 42 policy: safe profile: fp16 }
pattern Ship(name: String) -> String {
    vision_export("handle", "out.png")
    return name
}
"#;
    let findings = findings_of(source);
    assert!(
        by_id(&findings, "VISION_UNSIGNED_EXPORT").is_empty(),
        "declaration present — gate must not fire: {:?}",
        by_id(&findings, "VISION_UNSIGNED_EXPORT")
    );
}

/// Runtime backstop (Block 2.2): a manifest-less artifact (hand-built
/// registry) cannot pass the SIGNED export — loud Err with the same
/// check-id.
#[test]
fn runtime_backstop_refuses_manifestless_export() {
    let mut reg = VisionRegistry::new();
    let id = reg.insert(VisionArtifact {
        png_bytes: vec![1, 2, 3],
        manifest: None,
    });
    let args = vec![Value::Vision(id), Value::String("out.png".to_string())];
    let err = vision_export_dispatch(&reg, &args).expect_err("backstop must refuse");
    assert!(
        err.contains("VISION_UNSIGNED_EXPORT"),
        "backstop Err must carry the check-id: {}",
        err
    );
}

/// Signed artifact → `vision_export` writes PNG + sidecar manifest
/// (Block 1.4 happy path; Block 4.2 manifest-fields presence).
#[test]
fn signed_export_writes_png_and_sidecar() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("img.png");
    let mut reg = VisionRegistry::new();
    let id = reg.insert(VisionArtifact {
        png_bytes: vec![1, 2, 3],
        manifest: Some(signed_manifest()),
    });
    let args = vec![Value::Vision(id), Value::String(path.display().to_string())];
    vision_export_dispatch(&reg, &args).expect("signed export must succeed");
    assert_eq!(std::fs::read(&path).expect("png bytes"), vec![1, 2, 3]);
    let sidecar = dir.path().join("img.png.manifest.json");
    let json = std::fs::read_to_string(&sidecar).expect("sidecar manifest");
    for field in [
        "\"model_id\"",
        "\"model_sha256\"",
        "\"seed\"",
        "\"prompt_sha256\"",
        "\"policy\"",
        "\"timestamp\"",
        "\"png_sha256\"",
    ] {
        assert!(json.contains(field), "sidecar missing {}", field);
    }
    assert!(json.contains("z-image-turbo"), "model id recorded");
    assert!(!json.contains("unspecified"), "declared policy is safe");
}

/// The raw opt-out (Block 2.1) works on the same manifest-less artifact —
/// that is its purpose: unsigned by EXPLICIT choice.
#[test]
fn raw_export_allows_manifestless_artifact() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("raw.png");
    let mut reg = VisionRegistry::new();
    let id = reg.insert(VisionArtifact {
        png_bytes: vec![7, 8, 9],
        manifest: None,
    });
    let args = vec![Value::Vision(id), Value::String(path.display().to_string())];
    vision_export_raw_dispatch(&reg, &args).expect("raw export must succeed");
    assert_eq!(std::fs::read(&path).expect("raw bytes"), vec![7, 8, 9]);
    // No sidecar for raw exports (Block 2.1).
    let sidecar = dir.path().join("raw.png.manifest.json");
    assert!(!sidecar.exists(), "raw export must not write a sidecar");
}

// ── Gate 2: MODEL_WEIGHTS_UNSAFE (audit Error) ───────────────────────

/// Bare `.safetensors` literal URL — no manifest, no pinned SHA source
/// ("конструкция загрузки без pin").
#[test]
fn model_weights_unsafe_bare_safetensors_url() {
    let source = r#"
pattern Fetch(dir: String) -> String {
    return vision_fetch_weights("https://huggingface.co/pkg/model.safetensors", dir)
}
"#;
    let findings = findings_of(source);
    let hits = by_id(&findings, "MODEL_WEIGHTS_UNSAFE");
    assert_eq!(hits.len(), 1, "got: {:?}", findings);
    assert_eq!(hits[0].severity, Severity::Error);
    assert!(
        hits[0].message.contains("no manifest"),
        "message must name the missing pin source: {}",
        hits[0].message
    );
}

/// Pickle-RCE-class extension — refused by construction.
#[test]
fn model_weights_unsafe_pickle_class_url() {
    let source = r#"
pattern Fetch(dir: String) -> String {
    return vision_fetch_weights("https://huggingface.co/pkg/weights.pkl", dir)
}
"#;
    let findings = findings_of(source);
    let hits = by_id(&findings, "MODEL_WEIGHTS_UNSAFE");
    assert_eq!(hits.len(), 1, "got: {:?}", findings);
    assert_eq!(hits[0].severity, Severity::Error);
    assert!(
        hits[0].message.contains("pickle-RCE"),
        "message must name the pickle class: {}",
        hits[0].message
    );
}

/// SSRF-blocked host class (loopback + cloud metadata) — unreachable
/// through the SSRF guard regardless of any runtime allowlist.
#[test]
fn model_weights_unsafe_ssrf_blocked_hosts() {
    for url in [
        "http://localhost:8080/manifest.json",
        "http://127.0.0.1/manifest.json",
        "http://10.0.0.5/manifest.json",
        "http://169.254.169.254/latest/manifest.json",
    ] {
        let source = format!(
            r#"
pattern Fetch(dir: String) -> String {{
    return vision_fetch_weights("{}", dir)
}}
"#,
            url
        );
        let findings = findings_of(&source);
        let hits = by_id(&findings, "MODEL_WEIGHTS_UNSAFE");
        assert_eq!(hits.len(), 1, "url {} — got: {:?}", url, findings);
        assert_eq!(hits[0].severity, Severity::Error, "url {}", url);
        assert!(
            hits[0].message.contains("SSRF-blocked"),
            "url {} — message: {}",
            url,
            hits[0].message
        );
    }
}

/// Positive control: a manifest.json-class URL on a public host is NOT a
/// static violation (runtime layers: allowlist + SSRF + SHA pins).
#[test]
fn model_weights_manifest_url_passes_static_gate() {
    let source = r#"
pattern Fetch(dir: String) -> String {
    return vision_fetch_weights("https://huggingface.co/pkg/manifest.json", dir)
}
"#;
    let findings = findings_of(source);
    assert!(
        by_id(&findings, "MODEL_WEIGHTS_UNSAFE").is_empty(),
        "manifest.json-class URL must not fire the static gate: {:?}",
        findings
    );
}

// ── Gate 3: VISION_POLICY_MISSING (audit Warning) ────────────────────

/// `vision { }` without `policy:` → audit WARNING (not error), with the
/// declaration's line. Also the end-to-end proof of the Block 3.1 parser
/// relax: the source parses cleanly (the six other fields stay required).
#[test]
fn vision_policy_missing_is_audit_warning() {
    let source = r#"vision "poster" {
  model: "z-image-turbo"
  steps: 8
  width: 1024
  height: 1024
  seed: 42
  profile: fp16
}
"#;
    let findings = findings_of(source);
    let hits = by_id(&findings, "VISION_POLICY_MISSING");
    assert_eq!(hits.len(), 1, "got: {:?}", findings);
    assert_eq!(hits[0].severity, Severity::Warning);
    assert_eq!(hits[0].line, 1, "declaration starts on line 1");
    assert!(
        hits[0].message.contains("unspecified"),
        "message must name the manifest marker: {}",
        hits[0].message
    );

    // Advisory, NOT compile-blocking: no Category-A error path carries it.
    let decls = metalogos::parser::parse(source).expect("parse");
    let cat_a = audit_category_a(&decls, "");
    assert!(
        by_id(&cat_a, "VISION_POLICY_MISSING").is_empty(),
        "policy-missing must stay advisory (ADR-0125 severity)"
    );
}

/// Positive control: with `policy: safe` → no finding.
#[test]
fn vision_with_policy_has_no_missing_warning() {
    let source = r#"vision "poster" { model: "z-image-turbo" steps: 8 width: 1024 height: 1024 seed: 42 policy: safe profile: fp16 }
"#;
    let findings = findings_of(source);
    assert!(
        by_id(&findings, "VISION_POLICY_MISSING").is_empty(),
        "got: {:?}",
        findings
    );
}

// ── Runtime layer: allowlist default-deny (no network, no files) ─────

/// Default-deny (Block 3.2в): no `MLOG_VISION_WEIGHTS_ALLOWLIST` → loud
/// refusal naming MODEL_WEIGHTS_UNSAFE, BEFORE any network activity —
/// the tempdir (passed as the flow input → dest_dir) stays empty.
#[test]
#[serial]
fn fetch_weights_default_deny_without_allowlist() {
    std::env::remove_var("MLOG_VISION_WEIGHTS_ALLOWLIST");
    let dir = tempfile::tempdir().expect("tempdir");
    let source = r#"
vision "poster" { model: "z-image-turbo" steps: 8 width: 1024 height: 1024 seed: 42 policy: safe profile: fp16 }
pattern Fetch(dest: String) -> String {
    return vision_fetch_weights("https://huggingface.co/pkg/manifest.json", dest)
}
flow Main { input: String = "DEST" -> Fetch -> output }
"#
    .replace("DEST", &dir.path().display().to_string());
    let err = metalogos::run_program(&source).expect_err("default-deny must refuse");
    assert!(
        err.contains("MODEL_WEIGHTS_UNSAFE") && err.contains("default-deny"),
        "refusal must carry check-id + default-deny: {}",
        err
    );
    assert!(
        std::fs::read_dir(dir.path()).expect("read dir").count() == 0,
        "no files may be created by a refused fetch"
    );
    std::env::remove_var("MLOG_VISION_WEIGHTS_ALLOWLIST");
}

/// Host NOT in the allowlist → loud refusal, still no network, no files.
#[test]
#[serial]
fn fetch_weights_refuses_host_outside_allowlist() {
    std::env::set_var("MLOG_VISION_WEIGHTS_ALLOWLIST", "huggingface.co");
    let dir = tempfile::tempdir().expect("tempdir");
    let source = r#"
vision "poster" { model: "z-image-turbo" steps: 8 width: 1024 height: 1024 seed: 42 policy: safe profile: fp16 }
pattern Fetch(dest: String) -> String {
    return vision_fetch_weights("https://evil.example.org/manifest.json", dest)
}
flow Main { input: String = "DEST" -> Fetch -> output }
"#
    .replace("DEST", &dir.path().display().to_string());
    let err = metalogos::run_program(&source).expect_err("non-allowlisted host must refuse");
    assert!(
        err.contains("not in MLOG_VISION_WEIGHTS_ALLOWLIST"),
        "got: {}",
        err
    );
    assert!(
        std::fs::read_dir(dir.path()).expect("read dir").count() == 0,
        "no files may be created by a refused fetch"
    );
    std::env::remove_var("MLOG_VISION_WEIGHTS_ALLOWLIST");
}

/// Empty allowlist string = default-deny too (no hosts named = no trust).
#[test]
#[serial]
fn fetch_weights_empty_allowlist_is_deny() {
    std::env::set_var("MLOG_VISION_WEIGHTS_ALLOWLIST", "  , ,");
    let dir = tempfile::tempdir().expect("tempdir");
    let source = r#"
vision "poster" { model: "z-image-turbo" steps: 8 width: 1024 height: 1024 seed: 42 policy: safe profile: fp16 }
pattern Fetch(dest: String) -> String {
    return vision_fetch_weights("https://huggingface.co/pkg/manifest.json", dest)
}
flow Main { input: String = "DEST" -> Fetch -> output }
"#
    .replace("DEST", &dir.path().display().to_string());
    let err = metalogos::run_program(&source).expect_err("empty allowlist must refuse");
    assert!(err.contains("default-deny"), "got: {}", err);
    assert!(
        std::fs::read_dir(dir.path()).expect("read dir").count() == 0,
        "no files may be created by a refused fetch"
    );
    std::env::remove_var("MLOG_VISION_WEIGHTS_ALLOWLIST");
}

// ── SHA pinning: mismatch refused on synthetic bytes (no network) ────

/// The pin contract (Block 4.2): synthetic bytes vs a pinned SHA —
/// mismatch is a loud Err naming expected/computed; match passes.
#[test]
fn sha_pin_mismatch_is_loud_and_match_passes() {
    use metalogos::vision::provenance::{sha256_hex, verify_sha_pin};

    let bytes = b"synthetic weights shard - not real weights";
    let real = sha256_hex(bytes);

    // Match → Ok.
    verify_sha_pin("shard-0.safetensors", &real, bytes).expect("matching pin passes");

    // Mismatch → loud Err with check-id + expected + computed.
    let err = verify_sha_pin(
        "shard-0.safetensors",
        "0000000000000000000000000000000000000000000000000000000000000000",
        bytes,
    )
    .expect_err("mismatch must refuse");
    assert!(
        err.contains("MODEL_WEIGHTS_UNSAFE") && err.contains("pin mismatch"),
        "got: {}",
        err
    );
    assert!(err.contains(&real), "computed SHA must be in the message");
}

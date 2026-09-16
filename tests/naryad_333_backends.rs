// ── Наряд №333 (ADR-0163): backend registry + license gate ───────────
//
// Contracts:
//   (1) the registry schema — every entry carries class / weights_id /
//       license / license_note, and a pin that is either a real SHA-256
//       hex pin or the typed PendingNo334 boundary (never a fake hash);
//   (2) the distribution gate — a program that NAMES non-osi/restrictive
//       weights does not compile, with the license and registry record
//       named (the w1_license_gate fixture, MDL-3 scenario);
//   (3) osi references compile clean;
//   (4) `profile licensing { backends: permissive_with_audit }` — the
//       loud bridge: usage compiles and produces Info audit events;
//   (5) the vision-decl model field is a gated reference position;
//   (6) `backend_list()` — the language surface over the registry.

use metalogos::audit::{audit_category_a, Severity};

fn audit_findings(source: &str) -> Vec<(Severity, String)> {
    // Severity + check_id pairs of the Category-A audit.
    let decls = metalogos::parser::parse(source.trim()).expect("parses");
    audit_category_a(&decls, "")
        .into_iter()
        .map(|f| (f.severity, f.check_id.to_string()))
        .collect()
}

// ── (1) Registry schema (integration-level invariants) ───────────────

#[test]
fn registry_has_both_license_gate_classes_represented() {
    // The gate must have something to refuse (non-osi + restrictive) and
    // something to allow (osi) — otherwise the check is decorative.
    let has = |l: metalogos::backends::LicenseClass| {
        metalogos::backends::BACKEND_REGISTRY
            .iter()
            .any(|e| e.license == l)
    };
    assert!(has(metalogos::backends::LicenseClass::Osi));
    assert!(has(metalogos::backends::LicenseClass::NonOsi));
    assert!(has(metalogos::backends::LicenseClass::Restrictive));
}

#[test]
fn registry_names_are_statically_visible() {
    // The №316 lesson: registry contracts must hold at the SOURCE level.
    // The table is a plain struct-literal const in src/backends.rs —
    // every entry line names its license class explicitly.
    let src = include_str!("../src/backends.rs");
    for lic in [
        "LicenseClass::Osi",
        "LicenseClass::NonOsi",
        "LicenseClass::Restrictive",
    ] {
        assert!(src.contains(lic), "table must name {} literally", lic);
    }
    // Every entry in the table carries a pin field (the schema contract).
    // Table entries are the 4-space-indented literals inside the const
    // (the struct DEFINITION has no indent and is not counted).
    let entries = src.matches("    BackendEntry {").count();
    let pins = src.matches("pin: ShaPin::").count();
    assert_eq!(entries, pins, "every entry carries a pin field");
}

// ── (2) Distribution gate: refusal with license named ────────────────

#[test]
fn license_gate_fixture_fails_to_compile() {
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/w1_license_gate.mlog"),
    )
    .expect("fixture exists");
    let err = metalogos::compile_program(src.trim()).expect_err("non-osi weights must not compile");
    assert!(
        err.contains("BACKEND_LICENSE_DISTRIBUTION"),
        "expected the license gate, got: {}",
        err
    );
    assert!(
        err.contains("non-osi") && err.contains("nemotron-3-nano-omni-30b-a3b"),
        "the license class and the registry record must be named, got: {}",
        err
    );
}

#[test]
fn restrictive_weights_are_denied_by_default_deny() {
    let src = r#"
pattern Serve() -> String {
  let omni_model = "WALL-OSS-0.5"
  return "ok"
}
flow Main { input: String = "x" -> Serve -> output }
"#;
    let err =
        metalogos::compile_program(src.trim()).expect_err("restrictive weights must not compile");
    assert!(
        err.contains("BACKEND_LICENSE_DISTRIBUTION") && err.contains("restrictive"),
        "expected restrictive default-deny, got: {}",
        err
    );
}

// ── (3) osi references compile clean ─────────────────────────────────

#[test]
fn osi_weights_reference_compiles() {
    let src = r#"
pattern Serve() -> String {
  let tts_model = "chatterbox-multilingual-v3"
  return "ok"
}
flow Main { input: String = "x" -> Serve -> output }
"#;
    metalogos::compile_program(src.trim())
        .unwrap_or_else(|e| panic!("osi weights must compile, got: {}", e));
}

#[test]
fn unknown_model_strings_are_not_gate_business() {
    // Unregistered names are NOT license-gate findings here — the
    // №334 SHA-pin path gates actual fetches; the license gate governs
    // the REGISTERED non-osi/restrictive surface.
    let src = r#"
pattern Serve() -> String {
  let m = "some-unregistered-model"
  return "ok"
}
flow Main { input: String = "x" -> Serve -> output }
"#;
    metalogos::compile_program(src.trim())
        .unwrap_or_else(|e| panic!("unregistered names must compile, got: {}", e));
    let sevs = audit_findings(src);
    assert!(
        !sevs
            .iter()
            .any(|(_, c)| c == "BACKEND_LICENSE_DISTRIBUTION"),
        "no license finding for unregistered names"
    );
}

// ── (4) The licensing bridge: allowed, audited, never silent ─────────

#[test]
fn licensing_profile_downgrades_to_audit_events() {
    let src = r#"
profile licensing { backends: permissive_with_audit }

pattern Serve() -> String {
  let omni_model = "nemotron-3-nano-omni-30b-a3b"
  return "ok"
}
flow Main { input: String = "x" -> Serve -> output }
"#;
    // The bridge compiles the program...
    metalogos::compile_program(src.trim())
        .unwrap_or_else(|e| panic!("licensing profile must unlock compilation, got: {}", e));
    // ...and the usage stays AUDITED (Info events, not silence).
    let sevs = audit_findings(src);
    let events = sevs
        .iter()
        .filter(|(s, c)| c == "BACKEND_LICENSE" && *s == Severity::Info)
        .count();
    assert!(
        events >= 1,
        "expected a BACKEND_LICENSE audit event, got {:?}",
        sevs
    );
}

#[test]
fn licensing_profile_does_not_weaken_the_sink_gate() {
    // Independent flags: licensing must NOT open the №325 egress gate.
    let src = r#"
profile licensing { backends: permissive_with_audit }

pattern Leak() -> String {
  let tok = env("MLOG_333_TOKEN")
  print(tok)
  return "sent"
}
flow Main { input: String = "x" -> Leak -> output }
"#;
    let err = metalogos::compile_program(src.trim()).expect_err("sink gate must stay strict");
    assert!(
        err.contains("SECRET_LEAK"),
        "expected the №325 gate to stay strict, got: {}",
        err
    );
}

// ── (5) The vision-decl model field is a gated position ──────────────

#[test]
fn vision_decl_model_field_is_gated() {
    // Audit runs BEFORE the semantic KNOWN-model check, so a non-osi id
    // in the vision declaration is a license finding even though the
    // model is not (yet) in KNOWN_VISION_MODELS.
    let src = r#"
vision "poster" { model: "nemotron-3-nano-omni-30b-a3b" steps: 8 width: 1024 height: 1024 seed: 42 policy: safe profile: fp16 }
flow Main { input: String = "x" -> output }
"#;
    let sevs = audit_findings(src);
    assert!(
        sevs.iter()
            .any(|(_, c)| c == "BACKEND_LICENSE_DISTRIBUTION"),
        "vision model field must be gated, got {:?}",
        sevs
    );
}

// ── (6) backend_list() — the language surface ────────────────────────

#[test]
fn backend_list_builtin_lists_the_registry() {
    let src = r#"
pattern Regs(_data: String) -> String {
  let entries = backend_list()
  let n = len(entries)
  if n >= 6.0 {
    return "registry-listed"
  }
  return "registry-empty"
}
flow Main { input: String = "x" -> Regs -> output }
"#;
    let out = metalogos::run_program(src.trim())
        .unwrap_or_else(|e| panic!("backend_list must run: {}", e));
    assert_eq!(
        out.as_deref().unwrap_or_default().trim_end(),
        "registry-listed"
    );
}

#[test]
fn backend_list_rejects_arguments() {
    let src = r#"
pattern Regs(_data: String) -> String {
  let _x = backend_list("arg")
  return "x"
}
flow Main { input: String = "x" -> Regs -> output }
"#;
    let err = metalogos::run_program(src.trim()).expect_err("backend_list takes no args");
    assert!(
        err.contains("backend_list: expects 0 arguments"),
        "got: {}",
        err
    );
}

// ── (7) No stubs (№16.0-D) ───────────────────────────────────────────

#[test]
fn naryad_333_no_stubs() {
    let markers = [
        concat!("todo", "!"),
        concat!("unimplemented", "!"),
        concat!("SKELE", "TON"),
    ];
    for path in [
        "src/backends.rs",
        "src/builtins/backends.rs",
        "src/profile.rs",
    ] {
        let src =
            std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
                .unwrap_or_default();
        for m in markers {
            assert!(!src.contains(m), "{} must not contain {}", path, m);
        }
    }
}

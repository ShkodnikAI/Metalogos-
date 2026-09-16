// ── Наряд №332 (ADR-0164 §7.4): perception AST + origin chain ─────────
//
// Red/green corpus:
//   (1) origin declaration — vocabulary/shape validation is loud on
//       every compile path (ORIGIN_DECL_INVALID);
//   (2) HandleSource (`source <origin>`) — unknown origin, illegal
//       positions, file capture end-to-end (TW + VM), camera PARKED;
//   (3) ProvBind / Lift (`from <origin> media_store_*(...)`) — the bare
//       Lift is refused ("a handle without origin is not constructed"),
//       bind attaches observable provenance, re-seals on a non-public
//       origin label;
//   (4) Sink — the §5.3 kitchen-camera scenario: the STATIC deny names
//       the sink, the node and the rule (SECRET_LEAK via №325), never a
//       runtime fall or a silent pass; public origins flow through;
//   (5) the w1_origin_chain example fixture passes on both backends.

use std::path::Path;

fn run_tw(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    // Grammar does not skip a leading blank line — trim the raw strings.
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

fn fresh_dir(rel: &str) {
    // The io sandbox canonicalizes the parent BEFORE the write — the
    // directory must exist up front (same contract as write_file).
    let p = Path::new(MANIFEST).join(rel);
    if p.exists() {
        let _ = std::fs::remove_dir_all(&p);
    }
    std::fs::create_dir_all(&p).expect("sandbox parent dir created");
}

// ── (1) Origin declaration: vocabulary/shape validation ──────────────

#[test]
fn origin_decl_parses_and_compiles() {
    let src = r#"
origin cam { kind: camera, media: image, label: private }
pattern P() -> String { return "ok" }
flow Main { input: String = "x" -> P -> output }
"#;
    let decls = metalogos::parser::parse(src.trim()).expect("origin decl parses");
    let origin = decls
        .iter()
        .find_map(|d| match d {
            metalogos::ast::Declaration::Origin(o) => Some(o),
            _ => None,
        })
        .expect("Origin declaration in AST");
    assert_eq!(origin.name, "cam");
    assert!(origin
        .fields
        .contains(&("kind".to_string(), "camera".to_string())));
    assert!(origin
        .fields
        .contains(&("label".to_string(), "private".to_string())));
    // The declared camera alone is inert: the program compiles (the
    // capture is what would be PARKED — the declaration is not).
    metalogos::compile_program(src.trim())
        .unwrap_or_else(|e| panic!("inert origin declaration must compile, got: {}", e));
}

#[test]
fn origin_decl_unknown_kind_is_loud() {
    let src = r#"
origin cam { kind: microwave, media: image, label: private }
pattern P() -> String { return "ok" }
flow Main { input: String = "x" -> P -> output }
"#;
    let err =
        metalogos::compile_program(src.trim()).expect_err("unknown origin kind must not compile");
    assert!(
        err.contains("ORIGIN_DECL_INVALID") && err.contains("unknown kind 'microwave'"),
        "expected loud kind vocabulary error, got: {}",
        err
    );
}

#[test]
fn origin_decl_unknown_media_is_loud() {
    let src = r#"
origin cam { kind: file, media: hologram, label: private, path: "x.bin" }
pattern P() -> String { return "ok" }
flow Main { input: String = "x" -> P -> output }
"#;
    let err =
        metalogos::compile_program(src.trim()).expect_err("unknown origin media must not compile");
    assert!(
        err.contains("ORIGIN_DECL_INVALID"),
        "expected loud media vocabulary error, got: {}",
        err
    );
}

#[test]
fn origin_decl_poisoned_label_is_not_constructible() {
    // Quarantine (poisoned) comes only from the taint machinery — a
    // declaration cannot mint it.
    let src = r#"
origin cam { kind: camera, media: image, label: poisoned }
pattern P() -> String { return "ok" }
flow Main { input: String = "x" -> P -> output }
"#;
    let err =
        metalogos::compile_program(src.trim()).expect_err("poisoned origin label must not compile");
    assert!(
        err.contains("ORIGIN_DECL_INVALID"),
        "expected loud label vocabulary error, got: {}",
        err
    );
}

#[test]
fn origin_decl_unknown_field_is_loud() {
    let src = r#"
origin cam { kind: camera, media: image, label: private, pat: "x" }
pattern P() -> String { return "ok" }
flow Main { input: String = "x" -> P -> output }
"#;
    let err =
        metalogos::compile_program(src.trim()).expect_err("a typo'd origin field must not compile");
    assert!(
        err.contains("ORIGIN_DECL_INVALID") && err.contains("unknown field 'pat'"),
        "expected loud unknown-field error, got: {}",
        err
    );
}

#[test]
fn origin_decl_missing_required_field_is_loud() {
    let src = r#"
origin cam { kind: camera, media: image }
pattern P() -> String { return "ok" }
flow Main { input: String = "x" -> P -> output }
"#;
    let err = metalogos::compile_program(src.trim()).expect_err("missing 'label' must not compile");
    assert!(
        err.contains("missing required field 'label'"),
        "expected loud missing-field error, got: {}",
        err
    );
}

#[test]
fn origin_decl_file_without_path_is_loud() {
    let src = r#"
origin cam { kind: file, media: image, label: public }
pattern P() -> String { return "ok" }
flow Main { input: String = "x" -> P -> output }
"#;
    let err = metalogos::compile_program(src.trim())
        .expect_err("file origin without path must not compile");
    assert!(
        err.contains("requires the 'path' field"),
        "expected loud path requirement, got: {}",
        err
    );
}

// ── (2) HandleSource: `source <origin>` ─────────────────────────────

#[test]
fn source_unknown_origin_is_loud() {
    let src = r#"
pattern P() -> String {
  let frame = source nosuch
  return "x"
}
flow Main { input: String = "x" -> P -> output }
"#;
    let err = metalogos::compile_program(src.trim())
        .expect_err("source of unknown origin must not compile");
    assert!(
        err.contains("ORIGIN_REQUIRED") && err.contains("names no declared origin"),
        "expected loud unknown-origin error, got: {}",
        err
    );
}

#[test]
fn source_outside_binding_is_loud() {
    // A source handle must be BOUND to a variable to be tracked — a
    // construction nested in an expression is refused.
    let src = r#"
origin cam { kind: camera, media: image, label: public }
pattern P(_x: String) -> String {
  let m = media_meta(source cam)
  return "unreachable"
}
flow Main { input: String = "x" -> P -> output }
"#;
    let err = metalogos::compile_program(src.trim())
        .expect_err("unbound source construction must not compile");
    assert!(
        err.contains("ORIGIN_REQUIRED") && err.contains("only legal as a binding initializer"),
        "expected loud position error, got: {}",
        err
    );
}

#[test]
fn source_file_capture_end_to_end_on_tw() {
    // Per-backend dirs — the two end-to-end tests run in PARALLEL threads
    // and must not race on one directory (fresh_dir removes + recreates;
    // the №331 OUTFILE lesson, generalized to the whole dir).
    fresh_dir("target/n332-cam-tw");
    std::fs::write(
        Path::new(MANIFEST).join("target/n332-cam-tw/frame.bin"),
        b"frame-332-payload",
    )
    .expect("capture source written");
    let src = r#"
origin cam { kind: file, media: image, label: public, path: "target/n332-cam-tw/frame.bin" }
pattern P(_tick: String) -> String {
  let frame = source cam
  let saved = media_save(frame, "target/n332-cam-tw/tw-out.bin")
  return saved
}
flow Main { input: String = "t" -> P -> output }
"#;
    let out = run_tw(src, Path::new(MANIFEST))
        .unwrap_or_else(|e| panic!("file capture must run on TW: {}", e));
    assert_eq!(
        out.as_deref().unwrap_or_default().trim_end(),
        "target/n332-cam-tw/tw-out.bin"
    );
    let written =
        std::fs::read(Path::new(MANIFEST).join("target/n332-cam-tw/tw-out.bin")).expect("written");
    assert_eq!(
        written, b"frame-332-payload",
        "media_save writes the captured bytes"
    );
}

#[test]
fn source_file_capture_end_to_end_on_vm() {
    fresh_dir("target/n332-cam-vm");
    std::fs::write(
        Path::new(MANIFEST).join("target/n332-cam-vm/frame.bin"),
        b"frame-332-payload",
    )
    .expect("capture source written");
    let src = r#"
origin cam { kind: file, media: image, label: public, path: "target/n332-cam-vm/frame.bin" }
pattern P(_tick: String) -> String {
  let frame = source cam
  let saved = media_save(frame, "target/n332-cam-vm/vm-out.bin")
  return saved
}
flow Main { input: String = "t" -> P -> output }
"#;
    let out = run_vm(src, Path::new(MANIFEST))
        .unwrap_or_else(|e| panic!("file capture must run on VM: {}", e));
    assert_eq!(
        out.as_deref().unwrap_or_default().trim_end(),
        "target/n332-cam-vm/vm-out.bin"
    );
    let written =
        std::fs::read(Path::new(MANIFEST).join("target/n332-cam-vm/vm-out.bin")).expect("written");
    assert_eq!(
        written, b"frame-332-payload",
        "VM media_save writes the captured bytes"
    );
}

#[test]
fn source_camera_runtime_is_parked_boundary() {
    // The STATIC chain is unaffected (compiles); the capture itself is a
    // loud PARKED boundary — no capture hardware in this environment.
    let src = r#"
origin cam { kind: camera, media: image, label: public }
pattern P(_x: String) -> String {
  let frame = source cam
  return "unreachable"
}
flow Main { input: String = "x" -> P -> output }
"#;
    metalogos::compile_program(src.trim()).unwrap_or_else(|e| {
        panic!(
            "camera origin must compile (static chain is kind-blind): {}",
            e
        )
    });
    let err = run_tw(src, Path::new(MANIFEST)).expect_err("camera capture is PARKED");
    assert!(
        err.contains("PARKED boundary"),
        "expected the loud PARKED boundary, got: {}",
        err
    );
}

// ── (3) ProvBind / Lift: `from <origin> media_store_*(...)` ─────────

#[test]
fn bare_lift_is_refused() {
    // THE rule: a handle without origin is not constructed.
    let src = r#"
pattern P() -> String {
  let img = media_store_image("abc", "public")
  return "x"
}
flow Main { input: String = "x" -> P -> output }
"#;
    let err = metalogos::compile_program(src.trim()).expect_err("bare Lift must not compile");
    assert!(
        err.contains("ORIGIN_REQUIRED")
            && err.contains("bare media_store_image")
            && err.contains("handle without origin is not constructed"),
        "expected the loud origin-chain rule, got: {}",
        err
    );
}

#[test]
fn bind_over_non_construction_is_loud() {
    let src = r#"
origin gen { kind: generation, media: image, label: public }
pattern P() -> String {
  let x = from gen print("not a construction")
  return "x"
}
flow Main { input: String = "x" -> P -> output }
"#;
    let err = metalogos::compile_program(src.trim())
        .expect_err("bind over a non-construction must not compile");
    assert!(
        err.contains("ORIGIN_REQUIRED") && err.contains("must wrap a handle construction"),
        "expected loud bind-shape error, got: {}",
        err
    );
}

#[test]
fn bind_unknown_origin_is_loud() {
    let src = r#"
pattern P() -> String {
  let img = from nosuch media_store_image("abc", "public")
  return "x"
}
flow Main { input: String = "x" -> P -> output }
"#;
    let err = metalogos::compile_program(src.trim())
        .expect_err("bind to unknown origin must not compile");
    assert!(
        err.contains("ORIGIN_REQUIRED") && err.contains("names no declared origin"),
        "expected loud unknown bind origin, got: {}",
        err
    );
}

#[test]
fn direct_state_builtin_calls_are_loud() {
    // The dispatch builtins are lowered forms only — a hand-written call
    // bypasses the AST node and is refused.
    let src = r#"
origin gen { kind: generation, media: image, label: public }
pattern P() -> String {
  let img = media_source_capture("gen")
  return "x"
}
flow Main { input: String = "x" -> P -> output }
"#;
    let err = metalogos::compile_program(src.trim())
        .expect_err("direct media_source_capture call must not compile");
    assert!(
        err.contains("ORIGIN_REQUIRED") && err.contains("direct media_source_capture call"),
        "expected loud direct-call refusal, got: {}",
        err
    );
}

#[test]
fn bind_attaches_provenance_and_reseals_on_vm_parity() {
    // A public store entry bound to a consented origin re-seals at rest;
    // media_meta exposes the origin WITHOUT materializing bytes.
    let src = r#"
origin gdpr_src { kind: generation, media: image, label: consented }
pattern P(_x: String) -> String {
  let img = from gdpr_src media_store_image("gdpr-bytes", "public")
  let m = media_meta(img)
  if m.origin == "gdpr_src" {
    if m.sealed == true {
      if m.conf == "consented" {
        return "provenance-ok"
      }
    }
  }
  return "provenance-broken"
}
flow Main { input: String = "x" -> P -> output }
"#;
    let out_tw = run_tw(src, Path::new(MANIFEST))
        .unwrap_or_else(|e| panic!("provenance bind must run on TW: {}", e));
    assert_eq!(
        out_tw.as_deref().unwrap_or_default().trim_end(),
        "provenance-ok"
    );
    let out_vm = run_vm(src, Path::new(MANIFEST))
        .unwrap_or_else(|e| panic!("provenance bind must run on VM: {}", e));
    assert_eq!(
        out_vm.as_deref().unwrap_or_default().trim_end(),
        "provenance-ok"
    );
}

// ── (4) Sink: the §5.3 kitchen-camera scenario, statically denied ────

#[test]
fn kitchen_camera_fixture_is_denied_statically() {
    let src = std::fs::read_to_string(Path::new(MANIFEST).join("examples/w1_kitchen_camera.mlog"))
        .expect("fixture exists");
    let err = metalogos::compile_program(src.trim())
        .expect_err("the kitchen camera leak must not compile");
    // The deny NAMES the sink, the carried label and the class — the
    // reason is static (compile time), not a runtime fall.
    assert!(
        err.contains("SECRET_LEAK"),
        "expected the SECRET_LEAK class, got: {}",
        err
    );
    assert!(
        err.contains("media_save") && err.contains("'private, trusted'"),
        "expected the sink and the origin-carried label in the reason, got: {}",
        err
    );
}

#[test]
fn private_origin_label_blocks_sink_through_alias() {
    // The origin label flows with the handle — through ALIASES too.
    let src = r#"
origin cam { kind: camera, media: image, label: private }
pattern P() -> String {
  let frame = source cam
  let alias = frame
  let leak = media_save(alias, "target/n332-cam/leak.bin")
  return leak
}
flow Main { input: String = "x" -> P -> output }
"#;
    let err = metalogos::compile_program(src.trim())
        .expect_err("private origin through alias must not compile");
    assert!(
        err.contains("SECRET_LEAK") && err.contains("media_save"),
        "expected the static sink deny, got: {}",
        err
    );
}

#[test]
fn public_origin_flows_through_sink_gate() {
    // SINK_CLEARANCE compatibility (№325): a public origin handle
    // materializes through the sanctioned sink — the origin rule
    // creates no new holes and no new friction.
    fresh_dir("target/n332-cam-pub");
    let src = r#"
origin gen { kind: generation, media: image, label: public }
pattern P(_x: String) -> String {
  let img = from gen media_store_image("pub-bytes", "public")
  let saved = media_save(img, "target/n332-cam-pub/public-out.bin")
  return saved
}
flow Main { input: String = "x" -> P -> output }
"#;
    let out = run_tw(src, Path::new(MANIFEST))
        .unwrap_or_else(|e| panic!("public origin must flow: {}", e));
    assert_eq!(
        out.as_deref().unwrap_or_default().trim_end(),
        "target/n332-cam-pub/public-out.bin"
    );
    let written = std::fs::read(Path::new(MANIFEST).join("target/n332-cam-pub/public-out.bin"))
        .expect("written");
    assert_eq!(written, b"pub-bytes");
}

// ── (5) The example fixture passes on both backends ──────────────────

#[test]
fn w1_origin_chain_example_passes_on_tw_and_vm() {
    let src = std::fs::read_to_string(Path::new(MANIFEST).join("examples/w1_origin_chain.mlog"))
        .expect("fixture exists");
    let out_tw = run_tw(&src, Path::new(MANIFEST))
        .unwrap_or_else(|e| panic!("w1_origin_chain must run on TW: {}", e));
    assert_eq!(
        out_tw.as_deref().unwrap_or_default().trim_end(),
        "origin-bound-ok"
    );
    let out_vm = run_vm(&src, Path::new(MANIFEST))
        .unwrap_or_else(|e| panic!("w1_origin_chain must run on VM: {}", e));
    assert_eq!(
        out_vm.as_deref().unwrap_or_default().trim_end(),
        "origin-bound-ok"
    );
}

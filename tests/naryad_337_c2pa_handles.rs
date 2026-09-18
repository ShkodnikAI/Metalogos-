// ── Наряд №337 (P1, feature/provenance): the C2PA contour of media
//    handles — read/write manifests + the generation guarantee
//    (ADR-0166) ────────────────────────────────────────────────────────
//
// Red/green corpus:
//   (1) WRITE: media_save emits the sidecar `<path>.manifest.json`
//       (№241 continuity) built from the ENTRY's manifest facts —
//       captured content reads synthetic: false, the origin chain (№332)
//       and the C2PA record cannot disagree;
//   (2) GENERATION GUARANTEE: a bind whose declared origin kind is
//       `generation` FORCES synthetic: true on the store entry (no API
//       to un-mark) — the egress sidecar is synthetic: true on BOTH
//       backends; a generation bind over a non-construction does NOT
//       compile (the refusal names the marking contract);
//   (3) READ: media_manifest(handle) — provenance WITHOUT byte
//       materialization; media_manifest_read(path) — loud refusals for
//       missing/empty/corrupt sidecars (№320 posture) and the
//       conservative `synthetic: true` read of pre-№337 manifests.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
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

fn fresh_dir(name: &str) -> PathBuf {
    let dir = repo_root().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fresh dir");
    dir
}

const CAPTURE_PROG: &str = r#"
origin src_dir { kind: file, media: image, label: public, path: "frames/frame-001.jpg" }
pattern Cap(_t: String) -> String {
  let frame = source src_dir
  let p = media_save(frame, "target/n337-tw/captured.png")
  return p
}
flow Main { input: String = "t" -> Cap -> output }
"#;

const GENERATE_PROG: &str = r#"
origin demo_gen { kind: generation, media: image, label: public }
pattern Gen(_t: String) -> String {
  let img = from demo_gen media_store_image("gen-bytes-337", "public")
  let p = media_save(img, "target/n337-tw/generated.png")
  return p
}
flow Main { input: String = "t" -> Gen -> output }
"#;

// ── (1) The egress sidecar: written by the sink, entry-fact-backed ──

#[test]
fn media_save_writes_sidecar_captured_reads_synthetic_false() {
    fresh_dir("target/n337-tw");
    let out = run_tw(CAPTURE_PROG).expect("capture runs");
    assert_eq!(
        out.as_deref().unwrap_or_default().trim_end(),
        "target/n337-tw/captured.png"
    );
    let sidecar =
        std::fs::read_to_string(repo_root().join("target/n337-tw/captured.png.manifest.json"))
            .expect("sidecar exists next to the bytes (№241 continuity)");
    let m: serde_json::Value = serde_json::from_str(&sidecar).expect("sidecar is valid JSON");
    assert_eq!(m["kind"], "image");
    assert_eq!(
        m["origin"], "src_dir",
        "the origin chain and the C2PA record agree (№332 ↔ №337)"
    );
    assert_eq!(m["conf"], "public");
    assert_eq!(m["synthetic"], false, "captured bytes are NOT synthetic");
    let bytes = std::fs::read(repo_root().join("target/n337-tw/captured.png")).expect("bytes");
    let expect_sha = sha256_of(&bytes);
    assert_eq!(
        m["bytes_sha256"], expect_sha,
        "the manifest describes exactly the shipped bytes"
    );
    assert!(
        m["timestamp"].as_str().is_some(),
        "RFC3339 timestamp present"
    );
}

// ── (2) The generation guarantee: forced marking, both backends ─────

#[test]
fn generation_bind_forces_synthetic_true_on_the_egress_sidecar() {
    fresh_dir("target/n337-tw");
    let out = run_tw(GENERATE_PROG).expect("generation runs");
    assert_eq!(
        out.as_deref().unwrap_or_default().trim_end(),
        "target/n337-tw/generated.png"
    );
    let sidecar =
        std::fs::read_to_string(repo_root().join("target/n337-tw/generated.png.manifest.json"))
            .expect("sidecar exists");
    let m: serde_json::Value = serde_json::from_str(&sidecar).expect("valid JSON");
    assert_eq!(m["synthetic"], true, "generation lift is marked (Art. 50)");
    assert_eq!(m["origin"], "demo_gen");
    // VM parity — the same sidecar on the compiled path (separate dir:
    // the two backends run in parallel threads and must not race).
    let prog = GENERATE_PROG.replace("n337-tw", "n337-vm");
    fresh_dir("target/n337-vm");
    let out_vm = run_vm(&prog).expect("vm runs");
    assert_eq!(
        out_vm.as_deref().unwrap_or_default().trim_end(),
        "target/n337-vm/generated.png"
    );
    let sidecar_vm =
        std::fs::read_to_string(repo_root().join("target/n337-vm/generated.png.manifest.json"))
            .expect("vm sidecar exists");
    let mv: serde_json::Value = serde_json::from_str(&sidecar_vm).expect("valid JSON");
    assert_eq!(mv["synthetic"], true, "vm generation lift is marked too");
}

#[test]
fn media_manifest_reads_provenance_without_materializing_bytes() {
    fresh_dir("target/n337-tw");
    // In-program: media_manifest shows the entry facts BEFORE egress —
    // the generation bind marks, the capture does not.
    let src = r#"
origin demo_gen { kind: generation, media: image, label: public }
origin src_dir { kind: file, media: image, label: public, path: "frames/frame-001.jpg" }
pattern Probe(_t: String) -> String {
  let img = from demo_gen media_store_image("probe-bytes", "public")
  let live = media_manifest(img)
  let frame = source src_dir
  let cap = media_manifest(frame)
  if live.synthetic == true {
    if live.origin == "demo_gen" {
      if cap.synthetic == false {
        if cap.origin == "src_dir" {
          if cap.kind == "image" {
            return "probe-ok"
          }
        }
      }
    }
  }
  return "probe-broken"
}
flow Main { input: String = "t" -> Probe -> output }
"#;
    let out = run_tw(src).expect("probe runs");
    assert_eq!(out.as_deref().unwrap_or_default().trim_end(), "probe-ok");
    let out_vm = run_vm(src).expect("vm parity for media_manifest");
    assert_eq!(out_vm.as_deref().unwrap_or_default().trim_end(), "probe-ok");
}

// ── (3) The read path: loud refusals + the conservative default ─────

#[test]
fn sidecar_read_is_conservative_true_for_pre_337_manifests() {
    // Distinct dir per test — parallel test threads must not race on
    // fresh_dir (the №332 lesson).
    fresh_dir("target/n337-read-conservative");
    // A pre-№337 sidecar has NO `synthetic` field — the №320 vocabulary
    // reads unknown as MARKED (true). Roundtrip through the real reader.
    let old_manifest = r#"{
  "kind": "image",
  "origin": "",
  "conf": "public",
  "bytes_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
  "timestamp": "2026-09-01T00:00:00+00:00"
}"#;
    std::fs::write(
        repo_root().join("target/n337-read-conservative/old.png.manifest.json"),
        old_manifest,
    )
    .expect("fixture");
    let src = r#"
pattern R(_t: String) -> String {
  let m = media_manifest_read("target/n337-read-conservative/old.png.manifest.json")
  if m.synthetic == true {
    return "conservative-true"
  }
  return "not-conservative"
}
flow Main { input: String = "t" -> R -> output }
"#;
    let out = run_tw(src).expect("conservative read runs");
    assert_eq!(
        out.as_deref().unwrap_or_default().trim_end(),
        "conservative-true"
    );
    let out_vm = run_vm(src).expect("vm parity for the conservative read");
    assert_eq!(
        out_vm.as_deref().unwrap_or_default().trim_end(),
        "conservative-true"
    );
}

#[test]
fn missing_empty_and_corrupt_sidecars_are_loud_refusals() {
    fresh_dir("target/n337-read-errors");
    std::fs::write(
        repo_root().join("target/n337-read-errors/empty.png.manifest.json"),
        "",
    )
    .expect("empty fixture");
    std::fs::write(
        repo_root().join("target/n337-read-errors/corrupt.png.manifest.json"),
        "{not json",
    )
    .expect("corrupt fixture");
    for (path, needle) in [
        (
            "target/n337-read-errors/absent.png.manifest.json",
            "cannot read sidecar",
        ),
        (
            "target/n337-read-errors/empty.png.manifest.json",
            "sidecar is EMPTY",
        ),
        (
            "target/n337-read-errors/corrupt.png.manifest.json",
            "corrupt sidecar JSON",
        ),
    ] {
        let src = format!(
            r#"
pattern R(_t: String) -> String {{
  let m = media_manifest_read("{}")
  return "x"
}}
flow Main {{ input: String = "t" -> R -> output }}
"#,
            path
        );
        let err = run_tw(&src).expect_err("the read path refuses loudly (№320 posture)");
        assert!(err.contains(needle), "expected '{}' got: {}", needle, err);
    }
}

// ── (4) Compile-time generation contract (red/green) ─────────────────

#[test]
fn generation_lift_over_non_construction_does_not_compile() {
    // A bind over an EXISTING handle would either skip the marking (a
    // generation lift without synthetic: true) or falsify it (captured
    // bytes marked synthetic) — BOTH are provenance lies, refused.
    let src = r#"
origin src_dir { kind: file, media: image, label: public, path: "frames/frame-001.jpg" }
origin demo_gen { kind: generation, media: image, label: public }
pattern P(_t: String) -> String {
  let h = source src_dir
  let g = from demo_gen h
  return "x"
}
flow Main { input: String = "t" -> P -> output }
"#;
    let err = metalogos::compile_program(src.trim())
        .expect_err("a generation lift without a fresh construction must not compile");
    assert!(
        err.contains("GENERATION lift sets synthetic: true"),
        "the refusal must name the marking contract (ADR-0166 §2.3), got: {}",
        err
    );
    // The sanctioned form compiles and marks — the green half.
    assert!(metalogos::compile_program(GENERATE_PROG.trim()).is_ok());
}

// ── helpers ──────────────────────────────────────────────────────────

fn sha256_of(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

// ── (5) No-stubs discipline (№16.0-D) ────────────────────────────────

#[test]
fn no_stubs_in_the_n337_surface() {
    for file in [
        "src/media/mod.rs",
        "src/builtins/media.rs",
        "src/semantic.rs",
    ] {
        let content = std::fs::read_to_string(repo_root().join(file)).expect(file);
        for bad in ["todo!", "unimplemented!", "SKELETON"] {
            assert!(
                !content.contains(bad),
                "{} contains {} — stubs are forbidden (№16.0-D)",
                file,
                bad
            );
        }
    }
}

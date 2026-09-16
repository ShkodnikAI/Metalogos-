// ── Наряд №331 (ADR-0162): unified media handles + store contract ─────
//
// Red/green corpus:
//   (1) opaque guarantee — byte extraction from a media handle is a
//       COMPILE error (the examples/w1_handle_opaque pair + direct
//       call-chained field access);
//   (2) materialization — the sanctioned sink writes exact bytes to the
//       sandboxed path (TW and VM backends);
//   (3) refcount — retain/release/meta contract incl. eviction loudness;
//   (4) at-rest — private/consented entries are sealed (runtime backstop
//       MEDIA_SEALED_EGRESS; the static №325 gate refuses private-LABELLED
//       handles at compile time — private-egress);
//   (5) non-media field access stays legal (positive control).

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

// ── (1) Opaque guarantee: compile-time refusals ──────────────────────

#[test]
fn opaque_example_fixture_fails_to_compile() {
    let src = std::fs::read_to_string(Path::new(MANIFEST).join("examples/w1_handle_opaque.mlog"))
        .expect("fixture exists");
    let err = metalogos::compile_program(&src).expect_err("byte extraction must not compile");
    assert!(
        err.contains("media handle is opaque (ADR-0114)"),
        "expected opacity error, got: {}",
        err
    );
}

#[test]
fn opaque_field_access_through_alias_fails_to_compile() {
    // Aliases carry the media binding too — extraction through them is
    // refused identically. (The grammar cannot express `.field` on a call
    // result: postfix ops attach to primaries only, so chaining a field
    // access on `media_store_image(...)` is a parse error by construction.)
    let src = r#"
pattern Frame(_data: String) -> String {
  let img = media_store_image("abc", "public")
  let alias = img
  let raw = alias.bytes
  return raw
}
flow Main { input: String = "x" -> Frame -> output }
"#;
    let err =
        metalogos::compile_program(src.trim()).expect_err("alias byte extraction must not compile");
    assert!(
        err.contains("media handle is opaque (ADR-0114)"),
        "expected opacity error, got: {}",
        err
    );
}

#[test]
fn non_media_field_access_still_compiles() {
    // Positive control: field access on STRUCT values is untouched.
    let src = r#"
pattern Meta() -> String {
  let img = media_store_image("abc", "public")
  let m = media_meta(img)
  let sealed = m.sealed
  if sealed == false {
    return "plain-public"
  }
  return "unexpected"
}
flow Main { input: String = "x" -> Meta -> output }
"#;
    metalogos::compile_program(src.trim())
        .unwrap_or_else(|e| panic!("struct field access must compile, got: {}", e));
}

// ── (2) Materialization through the sanctioned sink ──────────────────

const MATERIALIZATION_PROG: &str = r#"
pattern Frame(data: String) -> String {
  let img = media_store_image(data, "public")
  let _p = media_save(img, "target/n331-media/OUTFILE")
  return "saved"
}
flow Main { input: String = "media-331-payload" -> Frame -> output }
"#;

fn materialization_source(outfile: &str) -> String {
    // Separate output names per backend — the two backends run in
    // PARALLEL test threads and must not race on one file.
    MATERIALIZATION_PROG.replace("OUTFILE", outfile)
}

#[test]
fn materialization_writes_exact_bytes_on_tw() {
    fresh_dir("target/n331-media");
    let out = run_tw(&materialization_source("tw-out.bin"), Path::new(MANIFEST))
        .unwrap_or_else(|e| panic!("materialization must run: {}", e));
    assert_eq!(out.as_deref().unwrap_or_default().trim_end(), "saved");
    let written = std::fs::read(Path::new(MANIFEST).join("target/n331-media/tw-out.bin"))
        .expect("file written");
    assert_eq!(written, b"media-331-payload");
}

#[test]
fn materialization_writes_exact_bytes_on_vm() {
    fresh_dir("target/n331-media");
    let out = run_vm(&materialization_source("vm-out.bin"), Path::new(MANIFEST))
        .unwrap_or_else(|e| panic!("materialization must run on VM: {}", e));
    assert_eq!(out.as_deref().unwrap_or_default().trim_end(), "saved");
    let written = std::fs::read(Path::new(MANIFEST).join("target/n331-media/vm-out.bin"))
        .expect("file written");
    assert_eq!(written, b"media-331-payload");
}

// ── (3) Refcount contract (retain / release / meta) ──────────────────

const REFCOUNT_PROG: &str = r#"
pattern Frame(_data: String) -> String {
  let img = media_store_image("rc", "public")
  let _r = media_retain(img)
  let m2 = media_meta(img)
  let refs2 = m2.refs
  let _gone = media_release(img)
  let m1 = media_meta(img)
  let refs1 = m1.refs
  if refs2 == 2.0 {
    if refs1 == 1.0 {
      return "refcount-ok"
    }
  }
  return "refcount-broken"
}
flow Main { input: String = "x" -> Frame -> output }
"#;

#[test]
fn refcount_retain_release_observed_via_meta() {
    let out = run_tw(REFCOUNT_PROG, Path::new(MANIFEST))
        .unwrap_or_else(|e| panic!("refcount program must run: {}", e));
    assert_eq!(out.as_deref().unwrap_or_default().trim_end(), "refcount-ok");
}

#[test]
fn refcount_vm_matches_tw() {
    let out = run_vm(REFCOUNT_PROG, Path::new(MANIFEST))
        .unwrap_or_else(|e| panic!("refcount program must run on VM: {}", e));
    assert_eq!(out.as_deref().unwrap_or_default().trim_end(), "refcount-ok");
}

// ── (4) At-rest + gates ──────────────────────────────────────────────

#[test]
fn private_labelled_handle_refused_at_compile_time_by_sink_gate() {
    // The static №325 gate: the handle variable carries the private
    // label by DATA FLOW (env → store) — the materialization sink
    // refuses the call site at compile time (private-egress).
    let src = r#"
pattern Frame(_data: String) -> String {
  let tok = env("MLOG_331_TEST_TOKEN")
  let img = media_store_image(tok, "public")
  let _p = media_save(img, "target/n331-media/leak.bin")
  return "x"
}
flow Main { input: String = "x" -> Frame -> output }
"#;
    let err = metalogos::compile_program(src.trim())
        .expect_err("private flow into egress must not compile");
    // File-egress class for a private conf — the leak-suite vocabulary
    // (media_save maps to "file" in BOTH the semantic sink_kind table
    // and the audit sink_check_id table).
    assert!(
        err.contains("SECRET_LEAK") && err.contains("media_save"),
        "expected SECRET_LEAK on media_save, got: {}",
        err
    );
}

#[test]
fn runtime_backstop_refuses_materializing_sealed_entry() {
    // Static labels are bottom (literals) — the call site compiles; the
    // RUNTIME backstop (entry.label.conf = private, sealed at rest)
    // refuses the materialization loudly.
    let src = r#"
pattern Frame(_data: String) -> String {
  let img = media_store_image("secret-bytes", "private")
  let _p = media_save(img, "target/n331-media/sealed-out.bin")
  return "x"
}
flow Main { input: String = "x" -> Frame -> output }
"#;
    let err = run_tw(src, Path::new(MANIFEST)).expect_err("sealed entry must not materialize");
    assert!(
        err.contains("MEDIA_SEALED_EGRESS"),
        "expected runtime backstop, got: {}",
        err
    );
    // The backstop runs on the VM too.
    let err_vm =
        run_vm(src, Path::new(MANIFEST)).expect_err("sealed entry must not materialize on VM");
    assert!(
        err_vm.contains("MEDIA_SEALED_EGRESS"),
        "expected runtime backstop on VM, got: {}",
        err_vm
    );
}

#[test]
fn evicted_entry_is_loud_on_materialization() {
    let src = r#"
pattern Frame(_data: String) -> String {
  let img = media_store_image("evict-me", "public")
  let _r1 = media_release(img)
  let _r2 = media_release(img)
  let _p = media_save(img, "target/n331-media/gone.bin")
  return "x"
}
flow Main { input: String = "x" -> Frame -> output }
"#;
    let err = run_tw(src, Path::new(MANIFEST)).expect_err("evicted handle must be loud");
    assert!(
        err.contains("unknown handle"),
        "expected eviction loudness, got: {}",
        err
    );
}

#[test]
fn unknown_sensitivity_is_loud() {
    let src = r#"
pattern Frame(_data: String) -> String {
  let img = media_store_image("abc", "topsecret")
  return "x"
}
flow Main { input: String = "x" -> Frame -> output }
"#;
    let err = run_tw(src, Path::new(MANIFEST)).expect_err("unknown sensitivity must be loud");
    assert!(
        err.contains("unknown sensitivity 'topsecret'"),
        "got: {}",
        err
    );
}

#[test]
fn consented_is_sealed_like_private() {
    let src = r#"
pattern Frame(_data: String) -> String {
  let img = media_store_image("gdpr-frame", "consented")
  let _p = media_save(img, "target/n331-media/consented.bin")
  return "x"
}
flow Main { input: String = "x" -> Frame -> output }
"#;
    let err = run_tw(src, Path::new(MANIFEST)).expect_err("consented is sealed at rest");
    assert!(err.contains("MEDIA_SEALED_EGRESS"), "got: {}", err);
}

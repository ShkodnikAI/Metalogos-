// ── Naryad №697 (gh#1276, Wave 46): the 0.32 fixed-list package №1 ────
//
// The FIRST closure package over the ADR-0193 frozen list
// (scripts/ci/gate_032_untyped_sources.txt, 67 ids): NINE honest
// closures by the №644 criterion (typed ∧ (scalar ∨ parameterized) ∧
// (¬Struct ∨ all fields labeled)) — every signature landed from the
// LIVE handler and verified HERE by a both-backend test:
//
//   read_file_or    → String   (io.rs read_file_impl — the content and
//                               the default arms; the smfs virtual arm
//                               is String too)
//   list_dir        → List<String> (io.rs — the fs_gate branch and the
//                               smfs virtual branch both list names)
//   voice_list      → String   (voice/mod.rs — the single Ok site)
//   policy_check    → Struct<PolicyResult>{allowed:internal,
//                     reason:internal} (server.rs — all three
//                     make_struct sites, the fixed 2-field shape)
//   http_download   → Bool     (http.rs — all eight Ok sites Bool)
//   stt_transcribe  → String   (voice/backend.rs — the single Ok site,
//                               the METALOGOS_MOCK_LLM deterministic
//                               path)
//   omni_ask        → String   (voice/backend.rs — the single Ok site)
//   ocr_extract     → String   (vision/ocr.rs — the single Ok site)
//   vision_understand → String (vision/understand.rs — the single Ok
//                               site)
//
// The PDn core of the list stayed HONESTLY uncovered — the №650 refusal
// forms (row × reason × file fact) live in the naryad's PR: query (the
// opaque Query wrapper — the №538 honest Unknown), query_row /
// query_scalar / conv_context / conv_history / stdin (stub rows, no
// live handler), request_body (the dynamic-shape Struct), secret (the
// opaque Secret), memory_open (the opaque Memory handle), memory_read
// (the data-dependent stored value).
//
// The ratchet: the sensitive floor moved UP in the same PR
// (5034 → 5655 bp; scripts/ci/sensitive_surface_baseline.txt); the
// frozen list itself is NOT edited (the id closes once).
#![allow(clippy::disallowed_methods)]

use std::sync::Mutex;

/// The GLOBAL registries (VOICE_REGISTRY) and the mock-mode env are
/// process-global — the env-touching test serializes here.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn vm_run(src: &str) -> Result<Option<String>, String> {
    let declarations = metalogos::parser::parse(src).expect("parse");
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut comp = metalogos::compiler::Compiler::with_std_root(base.to_path_buf());
    let program = comp.compile(declarations).expect("compile");
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program)
}

fn assert_both(src: &str, expected: &str) {
    let tw = metalogos::run_program(src)
        .unwrap_or_else(|e| panic!("TW run failed: {e}\n{src}"))
        .unwrap_or_default();
    assert_eq!(tw.trim_end(), expected, "TW output mismatch\n{src}");
    let vm = vm_run(src)
        .unwrap_or_else(|e| panic!("VM run failed: {e}\n{src}"))
        .unwrap_or_default();
    assert_eq!(vm.trim_end(), expected, "VM output mismatch\n{src}");
}

// ── read_file_or → String ─────────────────────────────────────────────

#[test]
fn n697_read_file_or_is_string() {
    // The sandbox requires RELATIVE paths (the №455 containment) — the
    // cargo-test cwd is the crate root, and target/ is gitignored.
    let dir = std::path::Path::new("target/n697-read-file-or");
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).expect("temp dir");
    let file = dir.join("note.txt");
    std::fs::write(&file, "n697-content").expect("temp file");
    let fp = "target/n697-read-file-or/note.txt";
    let dp = "target/n697-read-file-or";

    // The existing-file arm: the content rides back verbatim (String).
    let src = format!(
        r#"
pattern T(i: String) -> String {{
  return read_file_or("{fp}", "fallback")
}}
flow Main {{ input: String = "x" -> T -> output }}
"#
    );
    assert_both(&src, "n697-content");

    // The missing-file arm: the explicit default (String).
    let src2 = format!(
        r#"
pattern T(i: String) -> String {{
  return read_file_or("{dp}/missing-n697.txt", "fallback")
}}
flow Main {{ input: String = "x" -> T -> output }}
"#
    );
    assert_both(&src2, "fallback");

    let _ = std::fs::remove_dir_all(dir);
}

// ── list_dir → List<String> ───────────────────────────────────────────

#[test]
fn n697_list_dir_is_list_of_strings() {
    let dir = std::path::Path::new("target/n697-list-dir");
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).expect("temp dir");
    std::fs::write(dir.join("a.txt"), "1").expect("entry");
    std::fs::write(dir.join("b.txt"), "2").expect("entry");
    let dp = "target/n697-list-dir";

    // len() accepts a List — the call proves the List shape; the count
    // pins the element population (the entries are Value::String — the
    // io.rs map builds Value::String per entry).
    let src = format!(
        r#"
pattern T(i: String) -> String {{
  return to_string(len(list_dir("{dp}")))
}}
flow Main {{ input: String = "x" -> T -> output }}
"#
    );
    assert_both(&src, "2");

    let _ = std::fs::remove_dir_all(dir);
}

// ── voice_list → String ───────────────────────────────────────────────

#[test]
// №799 (T2, gh#1488): voice_list rides the `voice` build with its parked
// row — the both-backend probe is the feature build's contract (the
// default parked refusal is pinned by naryad_799).
#[cfg(feature = "voice")]
fn n697_voice_list_is_string() {
    // The String contract: the result concatenates like a String (the
    // `+` on a non-String operand is a type error). The empty-registry
    // output is the empty string — deterministic within this binary.
    let src = r#"
pattern T(i: String) -> String {
  let s = voice_list()
  return s + "!"
}
flow Main { input: String = "x" -> T -> output }
"#;
    assert_both(src, "!");
}

// ── policy_check → Struct<PolicyResult>{allowed, reason} ─────────────

#[test]
fn n697_policy_check_is_policy_result_struct() {
    // The fixed 2-field shape (server.rs, all three make_struct sites):
    // the field access proves the Struct shape, the Bool/String fields
    // pin the well-formed meta (allowed:internal, reason:internal).
    let src = r#"
pattern T(i: String) -> String {
  let ok = policy_check("ls -la")
  let blocked = policy_check("vim file.txt")
  return to_string(ok.allowed) + " " + ok.reason + " " + to_string(blocked.allowed) + " " + blocked.reason
}
flow Main { input: String = "x" -> T -> output }
"#;
    assert_both(
        src,
        "true ok false blocked: interactive program 'vim' detected",
    );
}

// ── http_download → Bool ──────────────────────────────────────────────

#[test]
fn n697_http_download_is_bool() {
    // The refusal arm is Ok(Bool(false)) — a closed port answers fast
    // and deterministically offline (http.rs — all eight Ok sites are
    // Bool, the success arm included). The SSRF guard (№130/№150)
    // refuses loopback BEFORE the request — the documented test switch
    // METALOGOS_HTTP_ALLOW_PRIVATE=1 lets the loopback attempt through,
    // and the connection refusal lands on the Ok(Bool(false)) arm.
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let prev = std::env::var("METALOGOS_HTTP_ALLOW_PRIVATE").ok();
    std::env::set_var("METALOGOS_HTTP_ALLOW_PRIVATE", "1");
    let dst = "target/n697-download-target.bin";
    let src = format!(
        r#"
pattern T(i: String) -> String {{
  return to_string(http_download("http://127.0.0.1:1/n697-nope", "{dst}"))
}}
flow Main {{ input: String = "x" -> T -> output }}
"#,
        dst = dst
    );
    assert_both(&src, "false");
    match prev {
        Some(v) => std::env::set_var("METALOGOS_HTTP_ALLOW_PRIVATE", v),
        None => std::env::remove_var("METALOGOS_HTTP_ALLOW_PRIVATE"),
    }
}

// ── the mock-path String returns (stt/omni/ocr/vision) ────────────────

#[test]
fn n697_mock_ai_builtins_are_strings() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // The №454/№478 mock contract: ONLY the explicit env produces the
    // deterministic mock — set, probe, restore.
    let prev = std::env::var("METALOGOS_MOCK_LLM").ok();
    std::env::set_var("METALOGOS_MOCK_LLM", "1");

    // stt_transcribe → String
    let src = r#"
pattern T(i: String) -> String {
  return stt_transcribe("n697-audio")
}
flow Main { input: String = "x" -> T -> output }
"#;
    let tw = metalogos::run_program(src)
        .expect("stt TW")
        .unwrap_or_default();
    let vm = vm_run(src).expect("stt VM").unwrap_or_default();
    assert_eq!(tw.trim_end(), vm.trim_end(), "stt TW/VM parity");
    assert!(
        !tw.trim_end().is_empty(),
        "the mock transcript is non-empty"
    );

    // omni_ask → String
    let src = r#"
pattern T(i: String) -> String {
  return omni_ask("describe n697")
}
flow Main { input: String = "x" -> T -> output }
"#;
    let tw = metalogos::run_program(src)
        .expect("omni TW")
        .unwrap_or_default();
    let vm = vm_run(src).expect("omni VM").unwrap_or_default();
    assert_eq!(tw.trim_end(), vm.trim_end(), "omni TW/VM parity");
    assert!(
        !tw.trim_end().is_empty(),
        "the mock omni answer is non-empty"
    );

    // ocr_extract → String
    let src = r#"
pattern T(i: String) -> String {
  return ocr_extract("n697-image")
}
flow Main { input: String = "x" -> T -> output }
"#;
    let tw = metalogos::run_program(src)
        .expect("ocr TW")
        .unwrap_or_default();
    let vm = vm_run(src).expect("ocr VM").unwrap_or_default();
    assert_eq!(tw.trim_end(), vm.trim_end(), "ocr TW/VM parity");
    assert!(!tw.trim_end().is_empty(), "the mock ocr text is non-empty");

    // vision_understand → String
    let src = r#"
pattern T(i: String) -> String {
  return vision_understand("n697-image")
}
flow Main { input: String = "x" -> T -> output }
"#;
    let tw = metalogos::run_program(src)
        .expect("vision TW")
        .unwrap_or_default();
    let vm = vm_run(src).expect("vision VM").unwrap_or_default();
    assert_eq!(tw.trim_end(), vm.trim_end(), "vision TW/VM parity");
    assert!(
        !tw.trim_end().is_empty(),
        "the mock vision answer is non-empty"
    );

    match prev {
        Some(v) => std::env::set_var("METALOGOS_MOCK_LLM", v),
        None => std::env::remove_var("METALOGOS_MOCK_LLM"),
    }
}

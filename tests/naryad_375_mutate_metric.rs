//! Наряд №375 (ADR-0112 addendum): REAL golden-task battery for the mutate
//! keep/rollback decision — MOCK-MODE side.
//!
//! Mock mode (METALOGOS_MOCK_LLM unset or truthy — the default-on test-mode
//! convention) keeps the 0.95 stub, loudly documented: these tests pin the
//! mock behavior (message formats, keep/rollback threshold edges, the p2
//! golden contract, TW↔VM parity) so the real-mode change cannot silently
//! alter the test-mode surface.
//!
//! Real-mode tests live in `naryad_375_real_mode.rs` (separate process —
//! the env switch is process-global).

use std::path::{Path, PathBuf};

fn run_tw(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, base_dir.to_path_buf())
}

fn run_vm(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    let declarations =
        metalogos::parser::parse(source).map_err(|e| format!("parse error: {}", e))?;
    let mut comp = metalogos::compiler::Compiler::with_std_root(base_dir.to_path_buf());
    let program = comp
        .compile(declarations)
        .map_err(|e| format!("compile error: {}", e))?;
    let mut vm = metalogos::vm::Vm::new();
    vm.run(program).map_err(|e| e.to_string())
}

/// Mock mode: the stub 0.95 message format is byte-identical to the
/// pre-№375 contract (the battery note is appended ONLY in real mode).
#[test]
fn naryad_375_mock_mode_message_format_unchanged() {
    let src = "learnable pattern Sentiment(text: String) -> String {\n  prompt: \"Classify sentiment\"\n}\n\nmutate Sentiment { add_example(\"terrible experience\", \"negative\") rollback_if: accuracy < 0.9 }\n\nflow Main { input: String = \"s\" -> Sentiment -> output }";
    let base = PathBuf::from("examples");
    let tw = run_tw(src, &base).expect("TW runs");
    let vm = run_vm(src, &base).expect("VM runs");
    let tw_out = tw.as_deref().map(str::trim_end).unwrap_or_default();
    let vm_out = vm.as_deref().map(str::trim_end).unwrap_or_default();
    assert_eq!(tw_out, vm_out, "mock-mode TW/VM mutate messages diverge");
    assert!(
        tw_out.contains("[MUTATE] Sentiment: accuracy=0.95, kept (>= 0.9)"),
        "mock-mode message format changed — the 0.95 stub contract is pinned: {}",
        tw_out
    );
    assert!(
        !tw_out.contains("battery"),
        "mock mode must NOT append the battery note (real-mode only): {}",
        tw_out
    );
}

/// Mock-mode threshold edges: rollback fires when the rollback condition is
/// true (accuracy 0.95 < 1.0 → rollback), keep when false (0.95 < 0.9 is
/// false → keep). CompareOp mapping unchanged (№375 constraint).
#[test]
fn naryad_375_mock_mode_threshold_edges() {
    let base = PathBuf::from("examples");
    // rollback_if: accuracy < 1.0 → 0.95 < 1.0 true → ROLLBACK.
    let rollback_src = "learnable pattern Sentiment(text: String) -> String {\n  prompt: \"Classify sentiment\"\n}\n\nmutate Sentiment { add_example(\"x\", \"y\") rollback_if: accuracy < 1.0 }\n\nflow Main { input: String = \"s\" -> Sentiment -> output }";
    let out = run_tw(rollback_src, &base).expect("TW runs");
    let out = out.as_deref().map(str::trim_end).unwrap_or_default();
    assert!(
        out.contains("accuracy=0.95, rolled back (below 1.0)"),
        "mock-mode rollback edge broken: {}",
        out
    );
    // rollback_if: accuracy < 0.9 → false → KEEP.
    let keep_src = "learnable pattern Sentiment(text: String) -> String {\n  prompt: \"Classify sentiment\"\n}\n\nmutate Sentiment { add_example(\"x\", \"y\") rollback_if: accuracy < 0.9 }\n\nflow Main { input: String = \"s\" -> Sentiment -> output }";
    let out = run_tw(keep_src, &base).expect("TW runs");
    let out = out.as_deref().map(str::trim_end).unwrap_or_default();
    assert!(
        out.contains("accuracy=0.95, kept (>= 0.9)"),
        "mock-mode keep edge broken: {}",
        out
    );
}

/// The p2_full_adapt golden contract (mock mode) is untouched.
#[test]
fn naryad_375_p2_golden_unchanged() {
    let base = PathBuf::from("examples");
    let path = base.join("p2_full_adapt.mlog");
    let source = std::fs::read_to_string(&path).expect("p2 example exists");
    let expected =
        std::fs::read_to_string(path.with_extension("expected")).expect("p2 expected exists");
    let tw = run_tw(&source, &base).expect("p2 runs on TW");
    let vm = run_vm(&source, &base).expect("p2 runs on VM");
    let exp = expected.trim_end();
    assert_eq!(tw.as_deref().map(str::trim_end).unwrap_or_default(), exp);
    assert_eq!(vm.as_deref().map(str::trim_end).unwrap_or_default(), exp);
}

/// TW↔VM parity of the full mutate path in mock mode (both sides share the
/// same decision mapping and message format).
#[test]
fn naryad_375_mock_mode_tw_vm_parity() {
    let base = PathBuf::from("examples");
    let cases: [(&str, Option<&str>); 3] = [
        ("keep_edge", Some("0.9")),
        ("rollback_edge", Some("1.0")),
        ("no_condition", None),
    ];
    for (name, threshold) in cases {
        let mutate = match threshold {
            Some(t) => format!(
                "mutate Sentiment {{ add_example(\"x\", \"y\") rollback_if: accuracy < {} }}",
                t
            ),
            None => "mutate Sentiment { add_example(\"x\", \"y\") }".to_string(),
        };
        let src = format!(
            "learnable pattern Sentiment(text: String) -> String {{\n  prompt: \"Classify sentiment\"\n}}\n\n{}\n\nflow Main {{ input: String = \"s\" -> Sentiment -> output }}",
            mutate
        );
        let tw = run_tw(&src, &base).expect("TW runs");
        let vm = run_vm(&src, &base).expect("VM runs");
        assert_eq!(
            tw.as_deref().map(str::trim_end).unwrap_or_default(),
            vm.as_deref().map(str::trim_end).unwrap_or_default(),
            "{}: TW/VM diverge",
            name
        );
    }
}

/// No stubs (№16.0-D) — markers assembled from parts to avoid self-matching.
#[test]
fn naryad_375_no_stubs() {
    let markers = [
        concat!("todo", "!"),
        concat!("unimplemented", "!"),
        concat!("SKELE", "TON"),
    ];
    let src = std::fs::read_to_string(file!()).unwrap_or_default();
    for m in markers {
        assert!(
            !src.contains(m),
            "naryad_375 test contains stub marker {}",
            m
        );
    }
}

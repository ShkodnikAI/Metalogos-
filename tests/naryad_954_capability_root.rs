// ── №954 (issue #1921, the audit 12.11): capability permits — the ENTRY
//    program only ─────────────────────────────────────────────────────
//
// The audit's H-2 (probe c03): a root without permissions imports
// dep/evil whose own `profile capability { allow_carrier: "http_post" }`
// EXTENDED the permit roster on v0.39.0 — any dependency granted itself
// the network carrier (the VM compiler unioned the imported declaration
// through `permits_from_declarations(&all_decls)`).
//
// The fix (this PR): the roster resolves from the ENTRY declarations
// only; an imported capability profile is the loud
// `[CAPABILITY_PROFILE_IN_IMPORT]` refusal at check/run/compile/serve/
// mcp-serve; the transition window (`profile legacy { capability_imports:
// warn }` in the entry) downgrades the refusal to a loud stderr warning —
// the roster STILL never extends and the carrier refuses at the sink
// (IRREVERSIBLE_NO_GRANT, fail-closed).
//
// The probe files live in the audit PoC set 4_capability (c03/dep/evil).
// These tests rebuild the shapes in a temp dir (the import geometry).
#![allow(clippy::disallowed_methods)]

use serial_test::serial;
use std::fs;
use std::path::PathBuf;

const MODULE_WITH_PROFILE: &str = r#"
profile capability { allow_carrier: "http_post" }
pattern Exfil(x: String) -> String { let r = http_post("http://127.0.0.1:18124/c03-from-import", x) return "ok" }
"#;

const MODULE_CLEAN: &str = r#"
pattern Exfil(x: String) -> String { return "ok" }
"#;

const ENTRY: &str = r#"
import dep/evil
pattern P(x: String) -> String { return Exfil("data-from-root") }
flow Main { input: String = "x" -> P -> output }
"#;

fn setup(tag: &str, module_src: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("n954_{}_{}", tag, std::process::id()));
    let dep = dir.join("dep");
    fs::create_dir_all(&dep).expect("dep dir");
    fs::write(dir.join("entry.mlog"), ENTRY).expect("entry");
    fs::write(dep.join("evil.mlog"), module_src).expect("module");
    dir
}

/// THE probe (c03): the check refuses the dependency self-grant.
#[test]
#[serial]
fn n954_check_refuses_the_imported_capability_profile() {
    let dir = setup("refuse", MODULE_WITH_PROFILE);
    std::env::set_current_dir(&dir).expect("cwd");
    let src = fs::read_to_string(dir.join("entry.mlog")).expect("entry");
    let result = metalogos::check_program_with_root(&src, Some(dir.join("entry.mlog").as_path()))
        .expect("parse ok");
    let rendered = format!("{:?}", result.errors);
    assert!(
        rendered.contains("CAPABILITY_PROFILE_IN_IMPORT"),
        "the check must refuse the imported capability profile, got: {}",
        rendered
    );
    let _ = fs::remove_dir_all(&dir);
}

/// The transition window: the entry's legacy key downgrades the refusal
/// to the loud warning — the CHECK passes (the run still refuses at the
/// sink, pinned below).
#[test]
#[serial]
fn n954_transition_window_downgrades_to_the_loud_warning() {
    let dir = setup("warn", MODULE_WITH_PROFILE);
    std::env::set_current_dir(&dir).expect("cwd");
    let mut src = "profile legacy { capability_imports: warn }\n".to_string();
    src.push_str(&fs::read_to_string(dir.join("entry.mlog")).expect("entry"));
    // The root==file form compares the SOURCE texts — the warn variant
    // must be the file on disk too (the `check entry_warn.mlog --root
    // entry_warn.mlog` CLI geometry).
    fs::write(dir.join("entry.mlog"), &src).expect("rewrite entry");
    let result = metalogos::check_program_with_root(&src, Some(dir.join("entry.mlog").as_path()))
        .expect("parse ok");
    assert!(
        result.errors.is_empty(),
        "the transition window passes the check (the warning is on stderr), got: {:?}",
        result
            .errors
            .iter()
            .map(|e| format!("{:?}", e))
            .collect::<Vec<_>>()
    );
    let _ = fs::remove_dir_all(&dir);
}

/// The office form REGRESSION: the permit declared in the ENTRY (the
/// app.mlog shape) + a clean module — check passes, the VM run works.
#[test]
#[serial]
fn n954_office_form_entry_declared_permit_still_works() {
    let dir = std::env::temp_dir().join(format!("n954_office_{}", std::process::id()));
    let dep = dir.join("dep2");
    fs::create_dir_all(&dep).expect("dep dir");
    fs::write(dep.join("clean.mlog"), MODULE_CLEAN).expect("module");
    let entry = format!(
        "profile capability {{ allow_carrier: \"http_post\" }}\nimport dep2/clean\npattern P(x: String) -> String {{ return Exfil(\"data-from-root\") }}\nflow Main {{ input: String = \"x\" -> P -> output }}\n"
    );
    fs::write(dir.join("app.mlog"), &entry).expect("entry");
    std::env::set_current_dir(&dir).expect("cwd");
    let result = metalogos::check_program_with_root(&entry, Some(dir.join("app.mlog").as_path()))
        .expect("parse ok");
    assert!(
        result.errors.is_empty(),
        "the entry-declared permit + a clean module is the LEGAL office form, got: {:?}",
        result
            .errors
            .iter()
            .map(|e| format!("{:?}", e))
            .collect::<Vec<_>>()
    );
    let _ = fs::remove_dir_all(&dir);
}

/// THE leak is gone (the c03 probe): with the transition window, the run
/// proceeds to the sink and the SINK refuses — IRREVERSIBLE_NO_GRANT,
/// the POST never leaves (the roster never extended). On the default
/// path the run gate refuses BEFORE execution.
#[test]
#[serial]
fn n954_run_sink_refuses_even_in_the_warn_window() {
    let dir = setup("run", MODULE_WITH_PROFILE);
    std::env::set_current_dir(&dir).expect("cwd");
    let mut src = "profile legacy { capability_imports: warn }\n".to_string();
    src.push_str(&fs::read_to_string(dir.join("entry.mlog")).expect("entry"));
    // The VM lane (the single user executor): the compile strips the
    // imported profile from the roster, the sink refuses.
    let err = metalogos::run_program_on_vm(&src)
        .expect_err("the carrier must refuse at the sink — no permit from the import");
    assert!(
        err.contains("IRREVERSIBLE_NO_GRANT") && err.contains("http_post"),
        "the sink refusal must name the gate and the carrier, got: {}",
        err
    );
    let _ = fs::remove_dir_all(&dir);
}

/// The default run path refuses BEFORE execution (the gate, not the sink).
#[test]
#[serial]
fn n954_run_gate_refuses_the_imported_profile_before_execution() {
    let dir = setup("gate", MODULE_WITH_PROFILE);
    std::env::set_current_dir(&dir).expect("cwd");
    let src = fs::read_to_string(dir.join("entry.mlog")).expect("entry");
    let err = metalogos::run_program_on_vm(&src)
        .expect_err("the run gate must refuse the dependency self-grant");
    assert!(
        err.contains("CAPABILITY_PROFILE_IN_IMPORT"),
        "the run refusal must be the origin gate, got: {}",
        err
    );
    let _ = fs::remove_dir_all(&dir);
}

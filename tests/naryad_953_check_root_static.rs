// ── №953 (issue #1920, the audit 12.11): `check --root` is STATIC ──────
//
// The audit's C-2: `mlog check <file> --root <root>` EXECUTED the root
// program (an `Interpreter::run` since 2026-06-11) "to resolve imports" —
// the root's flows and effects ran BEFORE the analysis, without the
// Category-A gates (the probe printed the secret and wrote LEAKED.txt).
// The fix: the import graph resolves statically (the №523 mechanism,
// `semantic::resolve_import_groups`) — zero execution on the whole path.
//
// Pins (the audit PoC set 5_check_root_and_test, commit fd384d3):
//   - root.mlog/root2.mlog shapes: the side-effect file is NOT created;
//   - the static resolution still delivers the contract: the checked
//     file sees the root's (and the root's imports') declarations;
//   - the honest BONUS of the static form: the root's own leaks are now
//     REPORTED by check instead of executed (the C-2 probe refuses with
//     SECRET_LEAK).
//
// Mutation (scripts/mutation_verify_953.sh): restore `interp.run` in
// `check_program_with_root` → `check_root_does_not_execute_the_root`
// goes RED (the side-effect file reappears).
#![allow(clippy::disallowed_methods)]

use serial_test::serial;
use std::fs;
use std::path::PathBuf;

fn tmp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("n953_{}_{}", tag, std::process::id()));
    fs::create_dir_all(&dir).expect("tmp dir");
    dir
}

fn cwd() -> PathBuf {
    std::env::current_dir().expect("cwd")
}

/// The audit probe root2 shape: the root writes a file and prints.
const LEAK_ROOT: &str = r#"
pattern Leak(x: String) -> String {
  let s = env("HOME_SECRET_PROBE")
  let _ = write_file("LEAKED_953.txt", "secret=" + s)
  print("leak: " + s)
  return "done"
}
flow Main { input: String = "x" -> Leak -> output }
"#;

const CLEAN_LEAF: &str = "pattern Leaf(a: String) -> String { return a }";

/// THE pin: check --root on the leak-root creates NO file, prints
/// NOTHING — and honestly REFUSES the root's own leaks instead.
/// The CWD is aligned with the root's directory for the duration (the
/// fs_gate resolves relative writes against the process CWD — the same
/// geometry the CLI probe runs under); the test is `serial` (the CWD is
/// process-global).
#[test]
#[serial]
fn n953_check_root_does_not_execute_the_root() {
    let dir = tmp_dir("noexec");
    let root_path = dir.join("root.mlog");
    fs::write(&root_path, LEAK_ROOT).expect("write root");

    // The probe geometry: HOME_SECRET_PROBE set, CWD == the root's dir.
    std::env::set_var("HOME_SECRET_PROBE", "TOPSECRET123");
    let original_cwd = cwd();
    std::env::set_current_dir(&dir).expect("set cwd");

    let result = metalogos::check_program_with_root(CLEAN_LEAF, Some(root_path.as_path()));

    let _ = std::env::set_current_dir(original_cwd);
    let result = result.expect("parse ok");

    // Clean around both candidate write sites (the executed root wrote
    // relative to the checker's CWD on v0.39.0).
    let cwd_side = dir.join("LEAKED_953.txt");
    let executed = cwd_side.exists();
    let _ = fs::remove_file(&cwd_side);
    // The honest bonus: the static form REPORTS the root's leaks instead
    // of executing them.
    let rendered = format!("{:?}", result.errors);
    assert!(
        rendered.contains("SECRET_LEAK"),
        "the static check --root must refuse the leaking root with SECRET_LEAK findings, got: {}",
        rendered
    );
    assert!(
        !executed,
        "check --root EXECUTED the root (the side-effect file appeared) — the C-2 bug is back"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// The contract check --root exists for: the checked file sees the
/// ROOT's declarations — and the root's IMPORTS (the №523 static
/// resolution replaces the executed-interpreter merge).
#[test]
fn n953_check_root_resolves_imports_statically() {
    let dir = tmp_dir("imports");
    // The root imports a module; the module defines the pattern the
    // checked file uses.
    fs::write(dir.join("root.mlog"), "import deps/util\n").expect("write root");
    fs::create_dir_all(dir.join("deps")).expect("deps dir");
    fs::write(
        dir.join("deps").join("util.mlog"),
        "pattern Util(a: String) -> String { return upper(a) }\n",
    )
    .expect("write module");
    fs::write(
        dir.join("leaf.mlog"),
        "pattern UseIt(x: String) -> String { return Util(x) }\n",
    )
    .expect("write leaf");

    let leaf_src = fs::read_to_string(dir.join("leaf.mlog")).expect("read leaf");
    let result =
        metalogos::check_program_with_root(&leaf_src, Some(dir.join("root.mlog").as_path()))
            .expect("parse ok");
    let rendered = format!("{:?}", result.errors);
    assert!(
        !rendered.contains("UndefinedFunction"),
        "the static import resolution must deliver the root's imported symbols to the checked file, got: {}",
        rendered
    );
    let _ = fs::remove_dir_all(&dir);
}

/// The test/resume gates: the library entry points refuse the leak-class
/// programs BEFORE any execution (the H-1 probe shape).
#[test]
#[serial]
fn n953_test_program_refuses_the_leak_before_execution() {
    let dir = tmp_dir("testgate");
    let src = format!(
        "{}\ntest \"leak in test\" {{\n  assert_eq(Leak(\"x\"), \"done\")\n}}",
        LEAK_ROOT
    );
    std::env::set_var("HOME_SECRET_PROBE", "TOPSECRET123");
    let original_cwd = cwd();
    std::env::set_current_dir(&dir).expect("set cwd");
    let ran = metalogos::test_program_with_dir(&src, dir.clone());
    let _ = std::env::set_current_dir(original_cwd);
    let cwd_side = dir.join("LEAKED_953.txt");
    let executed = cwd_side.exists();
    let _ = fs::remove_file(&cwd_side);
    let err = ran.expect_err("the gated mlog test must refuse the leak program");
    assert!(
        err.contains("Category A security invariant violated") && err.contains("SECRET_LEAK"),
        "the refusal must name the Category-A gate and the class, got: {}",
        err
    );
    assert!(
        !executed,
        "the gated mlog test executed the leak (the side-effect file exists)"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
#[serial]
fn n953_resume_refuses_the_leak_before_execution() {
    let dir = tmp_dir("resumegate");
    std::env::set_var("HOME_SECRET_PROBE", "TOPSECRET123");
    let original_cwd = cwd();
    std::env::set_current_dir(&dir).expect("set cwd");
    let ran = metalogos::resume_program_with_dir(LEAK_ROOT, "Main", "cp", dir.clone());
    let _ = std::env::set_current_dir(original_cwd);
    let cwd_side = dir.join("LEAKED_953.txt");
    let executed = cwd_side.exists();
    let _ = fs::remove_file(&cwd_side);
    let err = ran.expect_err("the gated resume must refuse the leak program");
    assert!(
        err.contains("Category A security invariant violated"),
        "the resume refusal must carry the Category-A gate, got: {}",
        err
    );
    assert!(
        !executed,
        "the gated resume executed the leak (the side-effect file exists)"
    );
    let _ = fs::remove_dir_all(&dir);
}

//! Issue #602 — string projection of a `try` result (0.19 contract).
//!
//! 0.21.0 rendered `to_string(try X)` as the full `TryResult {ok, value,
//! error}` struct dump on BOTH paths — the 0.19 idiom
//! `let x = try f(); if to_string(x) == "()" { fallback }` was broken in
//! both branches (success gave the dump, failure never fired the fallback).
//! The fix: the Display projection of a TryResult renders ONLY the `value`
//! field — success → the inner value (0.19 was transparent on success),
//! failure → "()" (the `value` field is Unit on the error path). This is
//! exactly the office migration workaround contract (`TryVal(x) = x.value`,
//! verified on the vendored 0.19/0.21 binaries in the issue).
//!
//! The structural contract (№374/ADR-0142) is untouched: `.ok`, `.value`,
//! `.error.code`/`.error.message` keep working; stable codes (№385/ADR-0169)
//! live in the `error` field. Both backends share the builder and the
//! Display impl, so parity is by construction — asserted anyway.

use std::path::{Path, PathBuf};

/// Execute via tree-walking interpreter.
fn run_tw(source: &str, base_dir: &Path) -> Result<Option<String>, String> {
    metalogos::run_program_with_dir(source, base_dir.to_path_buf())
}

/// Execute via bytecode VM.
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

/// Assert both backends agree AND return the trimmed TW output.
fn assert_parity(name: &str, source: &str) -> String {
    let base_dir = PathBuf::from("examples");
    let tw =
        run_tw(source, &base_dir).unwrap_or_else(|e| panic!("{}: TW must succeed: {}", name, e));
    let vm =
        run_vm(source, &base_dir).unwrap_or_else(|e| panic!("{}: VM must succeed: {}", name, e));
    let tw_out = tw.as_deref().map(str::trim_end).unwrap_or_default();
    let vm_out = vm.as_deref().map(str::trim_end).unwrap_or_default();
    assert_eq!(tw_out, vm_out, "{}: TW and VM outputs diverge", name);
    tw_out.to_string()
}

fn program(body: &str) -> String {
    format!(
        "pattern T(_input: String) -> String {{\n{}\n}}\nflow Main {{ input: String = \"s\" -> T -> output }}",
        body
    )
}

/// Success: `to_string(try X)` renders the inner value (0.19 transparency).
#[test]
fn naryad_602_success_renders_value() {
    let out = assert_parity(
        "602 success value",
        &program("  let a = to_string(try trim(\"hello\"))\n  return \"v:\" + a"),
    );
    assert_eq!(
        out, "v:hello",
        "success must render the inner value, not the struct dump"
    );
}

/// Failure: `to_string(try X)` renders "()" — the `value` field is Unit on
/// the error path (office TryVal contract).
#[test]
fn naryad_602_failure_renders_unit() {
    let out = assert_parity(
        "602 failure unit",
        &program("  let a = to_string(try float(\"not-a-number\"))\n  return \"v:\" + a"),
    );
    assert_eq!(
        out, "v:()",
        "failure must render the Unit value field, not the struct dump"
    );
}

/// The 0.19 office idiom works in BOTH branches again:
/// `let x = try f(); if to_string(x) == "()" { fallback }`.
#[test]
fn naryad_602_office_idiom_success_path() {
    let out = assert_parity(
        "602 idiom success",
        &program(
            "  let x = try trim(\"payload\")\n  if to_string(x) == \"()\" { return \"fallback\" }\n  return \"use:\" + to_string(x)",
        ),
    );
    assert_eq!(
        out, "use:payload",
        "the success branch must use the real value"
    );
}

#[test]
fn naryad_602_office_idiom_failure_path() {
    let out = assert_parity(
        "602 idiom failure",
        &program(
            "  let x = try float(\"nope\")\n  if to_string(x) == \"()\" { return \"fallback\" }\n  return \"use:\" + to_string(x)",
        ),
    );
    assert_eq!(out, "fallback", "the failure branch must fire the fallback");
}

/// The structural contract (№374/ADR-0142) is untouched: `.ok`, `.value`,
/// `.error` fields keep working; `type_of` still reports the Struct.
#[test]
fn naryad_602_structural_access_untouched() {
    let out = assert_parity(
        "602 structural success",
        &program(
            "  let r = try trim(\"hello\")\n  return to_string(r.ok) + \"/\" + type_of(r) + \"/\" + r.value + \"/\" + to_string(r.error)",
        ),
    );
    assert_eq!(out, "true/Struct/hello/()");

    let out = assert_parity(
        "602 structural failure",
        &program(
            "  let r = try float(\"nope\")\n  if r.ok == false { return \"code:\" + r.error.code }\n  return \"unreachable\"",
        ),
    );
    assert_eq!(out, "code:RUNTIME_ERROR");
}

/// Interpolation via to_string + concat matches the issue's migration shape.
#[test]
fn naryad_602_concat_shape() {
    let out = assert_parity(
        "602 concat",
        &program("  return \"v=\" + to_string(try trim(\"hello\"))"),
    );
    assert_eq!(out, "v=hello");
}

/// №16.0-D: no stubs in this test file (markers built by parts so the
/// self-scan does not match its own literals).
#[test]
fn naryad_602_no_stubs() {
    let src = fs_self();
    let bang = String::from("!");
    let markers = ["todo", "unimplemented"];
    for m in markers {
        let marker = format!("{}{}", m, bang);
        assert!(
            !src.contains(&marker),
            "stub marker {} found in test file",
            marker
        );
    }
    let skeleton = ["SKELE", "TON"].concat();
    assert!(
        !src.contains(&skeleton),
        "stub marker (assembled) found in test file"
    );
}

fn fs_self() -> String {
    std::fs::read_to_string(file!()).unwrap_or_default()
}
